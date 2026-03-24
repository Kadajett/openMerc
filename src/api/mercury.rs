// Updated build_messages: keep last 20 user/assistant messages and inject a placeholder for Honcho summary.
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::logger;
use crate::app::Message;
use crate::config::MercuryConfig;
use crate::event::AppEvent;
use crate::tools::registry::{self, ToolCall, ToolDef};

/// Mercury API client — OpenAI‑compatible chat completions with tool calling + diffusion
pub struct MercuryClient {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
    max_tokens: u32,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ToolDef>>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    diffusing: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ChatMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ToolCallRaw>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ToolCallRaw {
    id: Option<String>,
    #[serde(rename = "type")]
    call_type: Option<String>,
    function: ToolCallFunctionRaw,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ToolCallFunctionRaw {
    name: String,
    arguments: String,
}

// --- Non‑streaming response types ---

#[derive(Deserialize, Debug)]
struct ChatResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<UsageInfo>,
}

#[derive(Deserialize, Debug)]
struct UsageInfo {
    #[serde(default)]
    total_tokens: u32,
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
}

#[derive(Deserialize, Debug)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize, Debug)]
struct ResponseMessage {
    content: Option<String>,
    tool_calls: Option<Vec<ToolCallRaw>>,
}

// --- Streaming / diffusion response types ---

#[derive(Deserialize, Debug)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize, Debug)]
struct StreamChoice {
    delta: StreamDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
struct StreamDelta {
    content: Option<String>,
}

impl MercuryClient {
    pub fn from_config(config: &MercuryConfig) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            base_url: config.base_url.clone(),
            api_key: config.api_key.clone(),
            model: config.model.clone(),
            max_tokens: config.max_tokens,
        }
    }

    fn build_messages(&self, system_context: Option<&str>, summary: Option<&str>, messages: &[Message]) -> Vec<ChatMessage> {
        let mut out = Vec::new();
        if let Some(ctx) = system_context {
            out.push(ChatMessage {
                role: "system".to_string(),
                content: Some(ctx.to_string()),
                tool_calls: None,
                tool_call_id: None,
            });
        }
        if let Some(s) = summary {
            out.push(ChatMessage {
                role: "system".to_string(),
                content: Some(s.to_string()),
                tool_calls: None,
                tool_call_id: None,
            });
        }
        // Filter to only user/assistant messages
        let relevant: Vec<&Message> = messages.iter()
            .filter(|m| !matches!(m.role, crate::app::Role::Tool | crate::app::Role::System))
            .collect();
        // Keep last 20 user/assistant messages
        let max_history = 20;
        let start = if relevant.len() > max_history { relevant.len() - max_history } else { 0 };
        if start > 0 {
            // Placeholder: in production this would call Honcho get_session_context to obtain a compressed summary.
            out.push(ChatMessage {
                role: "system".to_string(),
                content: Some(format!("(Earlier messages summarized by Honcho – {} messages omitted)", start)),
                tool_calls: None,
                tool_call_id: None,
            });
        }
        for msg in &relevant[start..] {
            out.push(ChatMessage {
                role: msg.role.to_string(),
                content: Some(msg.content.clone()),
                tool_calls: None,
                tool_call_id: None,
            });
        }
        out
    }

    async fn call_api(&self, req: &ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = serde_json::to_string(req).unwrap_or_default();
        logger::log_api_request(&url, &body);
        let resp = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(req)
            .send()
            .await?;
        let status = resp.status();
        let txt = resp.text().await?;
        logger::log_api_response(status.as_u16(), &txt);
        if !status.is_success() {
            anyhow::bail!("Mercury API error ({}): {}", status, txt);
        }
        Ok(serde_json::from_str(&txt)?)
    }

    async fn call_api_diffusing(&self, req: &ChatRequest, tx: &mpsc::UnboundedSender<AppEvent>) -> Result<String> {
        let url = format!("{}/chat/completions", self.base_url);
        let resp = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(req)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let txt = resp.text().await?;
            anyhow::bail!("Mercury API error ({}): {}", status, txt);
        }
        let mut final_content = String::new();
        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();
        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer = buffer[pos+1..].to_string();
                if line.is_empty() || !line.starts_with("data: ") { continue; }
                let data = &line[6..];
                if data == "[DONE]" { continue; }
                if !data.starts_with('{') { continue; }
                if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                    for choice in chunk.choices {
                        if let Some(content) = choice.delta.content {
                            final_content = content.clone();
                            let _ = tx.send(AppEvent::DiffusionUpdate(content.clone()));
                        }
                    }
                }
            }
        }
        Ok(final_content)
    }

    /// Runs the tool‑calling loop. Returns `Ok(Some(content))` when the loop ends early
    /// (cancellation or max rounds reached) and `Ok(None)` when we should continue to diffusion.
    async fn run_tool_loop(
        &self,
        msgs: &mut Vec<ChatMessage>,
        tools: &[ToolDef],
        max_rounds: u32,
        tool_ctx: registry::ToolContext,
        tx: &mpsc::UnboundedSender<AppEvent>,
        cancel: &CancellationToken,
    ) -> Result<Option<String>> {
        let mut failed: std::collections::HashSet<String> = std::collections::HashSet::new();
        for round in 0..max_rounds {
            if cancel.is_cancelled() {
                return Ok(Some("(operation cancelled)".to_string()));
            }
            // Compact tool history: if msgs is getting large, summarize old tool results
            // Keep system prompt + last 40 messages, summarize the rest
            if msgs.len() > 30 {
                let system_msgs: Vec<ChatMessage> = msgs.iter()
                    .take_while(|m| m.role == "system")
                    .cloned()
                    .collect();
                let sys_count = system_msgs.len();
                let tail: Vec<ChatMessage> = msgs[msgs.len().saturating_sub(20)..].to_vec();
                let trimmed_count = msgs.len() - sys_count - tail.len();
                let mut compacted = system_msgs;
                if trimmed_count > 0 {
                    compacted.push(ChatMessage {
                        role: "system".to_string(),
                        content: Some(format!("({trimmed_count} earlier tool call messages were summarized to save context. Focus on the recent messages below.)")),
                        tool_calls: None,
                        tool_call_id: None,
                    });
                }
                compacted.extend(tail);
                *msgs = compacted;
                logger::log("MERCURY", &format!("Compacted tool history: trimmed {trimmed_count} messages"));
            }
            let req = ChatRequest {
                model: self.model.clone(),
                messages: msgs.clone(),
                stream: false,
                max_tokens: Some(self.max_tokens),
                tools: Some(tools.to_vec()),
                diffusing: false,
            };
            let resp = self.call_api(&req).await?;
            // Track token usage
            if let Some(usage) = &resp.usage {
                let _ = tx.send(AppEvent::TokensUsed(usage.total_tokens, usage.prompt_tokens, usage.completion_tokens));
            }
            let choice = resp.choices.first()
                .ok_or_else(|| anyhow::anyhow!("No choices in response"))?;
            if let Some(tool_calls) = &choice.message.tool_calls {
                if tool_calls.is_empty() { continue; }
                msgs.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: choice.message.content.clone(),
                    tool_calls: Some(tool_calls.clone()),
                    tool_call_id: None,
                });
                for tc in tool_calls {
                    let key = format!("{}:{}", tc.function.name, tc.function.arguments);
                    let tool_call = ToolCall {
                        id: tc.id.clone(),
                        function: crate::tools::registry::ToolCallFunction {
                            name: tc.function.name.clone(),
                            arguments: tc.function.arguments.clone(),
                        },
                    };
                    let _ = tx.send(AppEvent::ToolUse(tc.function.name.clone(), tc.function.arguments.clone()));
                    let _ = tx.send(AppEvent::AgentProgress(round+1, max_rounds, tc.function.name.clone()));
                    let result = if failed.contains(&key) {
                        "(skipped — this exact call already failed)".to_string()
                    } else {
                        let r = registry::execute_tool(&tool_ctx, &tool_call).await;
                        if r.contains("Error") || r.contains("Not Found") || r.contains("not exist") || r.contains("No such file") {
                            failed.insert(key);
                        }
                        r
                    };
                    logger::log_tool(&tc.function.name, &tc.function.arguments, &result);
                    let _ = tx.send(AppEvent::ToolResult(tc.function.name.clone(), result.clone()));
                    if tc.function.name.starts_with("create_task") || tc.function.name.starts_with("update_task") {
                        let tasks = tool_ctx.tasks.lock().await.clone();
                        let _ = tx.send(AppEvent::TaskUpdated(tasks));
                    }
                    // Truncate large tool results to avoid blowing 128K context
                    let truncated_result = if result.len() > 4000 {
                        format!("{}...\n(truncated from {} bytes)", logger::safe_truncate(&result, 4000), result.len())
                    } else {
                        result
                    };
                    msgs.push(ChatMessage {
                        role: "tool".to_string(),
                        content: Some(truncated_result),
                        tool_calls: None,
                        tool_call_id: tc.id.clone(),
                    });
                }
                continue;
            }
            return Ok(None);
        }
        Ok(Some("(max tool rounds reached)".to_string()))
    }

    /// Public entry point – builds messages, runs tool loop, then diffusion streaming.
    pub async fn chat(
        &self,
        system_context: Option<&str>,
        messages: &[Message],
        tool_ctx: registry::ToolContext,
        event_tx: mpsc::UnboundedSender<AppEvent>,
        cancel: CancellationToken,
    ) {
        // Retrieve optional Honcho session summary
        let session_summary = {
            let mut honcho = tool_ctx.honcho.lock().await;
            honcho.get_session_context().await
        };
        let mut chat_messages = self.build_messages(system_context, session_summary.as_deref(), messages);

        // Router: use Mercury structured output to decide chat vs tools
        let last_user_msg = messages.iter().rev()
            .find(|m| m.role == crate::app::Role::User)
            .map(|m| m.content.clone())
            .unwrap_or_default();

        let router_messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: Some("You are a router. Respond ONLY with JSON. If the user wants to READ files, WRITE files, RUN commands, SEARCH code, CREATE tasks, or do ANY coding work: {\"needs_tools\": true, \"response\": \"\"}. If the user is just chatting, asking questions, or greeting: {\"needs_tools\": false, \"response\": \"your reply\"}".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: Some(last_user_msg),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let router_req = ChatRequest {
            model: self.model.clone(),
            messages: router_messages,
            stream: false,
            max_tokens: Some(300),
            tools: None,
            diffusing: false,
        };

        let is_chat = match self.call_api(&router_req).await {
            Ok(resp) => {
                if let Some(usage) = &resp.usage {
                    let _ = event_tx.send(AppEvent::TokensUsed(usage.total_tokens, usage.prompt_tokens, usage.completion_tokens));
                }
                if let Some(content) = resp.choices.first().and_then(|c| c.message.content.as_ref()) {
                    // Try to parse the JSON response
                    if let Some(start) = content.find('{') {
                        if let Some(end) = content.rfind('}') {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content[start..=end]) {
                                let needs_tools = v["needs_tools"].as_bool().unwrap_or(true);
                                if !needs_tools {
                                    let response = v["response"].as_str().unwrap_or("").to_string();
                                    if !response.is_empty() {
                                        let _ = event_tx.send(AppEvent::DiffusionUpdate(response));
                                        let _ = event_tx.send(AppEvent::StreamDone);
                                        return;
                                    }
                                }
                                !needs_tools
                            } else { false }
                        } else { false }
                    } else { false }
                } else { false }
            }
            Err(_) => false,
        };

        if is_chat { return; } // Already sent the response above

        // Tool-calling mode — full tool loop
        let tools = registry::tool_definitions();
        let max_rounds = 100_u32;
        match self.run_tool_loop(&mut chat_messages, &tools, max_rounds, tool_ctx, &event_tx, &cancel).await {
            Ok(Some(early)) => {
                let _ = event_tx.send(AppEvent::DiffusionUpdate(early.clone()));
                let _ = event_tx.send(AppEvent::StreamDone);
                return;
            }
            Ok(None) => {
                let diff_req = ChatRequest {
                    model: self.model.clone(),
                    messages: chat_messages.clone(),
                    stream: true,
                    max_tokens: Some(self.max_tokens),
                    tools: None,
                    diffusing: true,
                };
                match self.call_api_diffusing(&diff_req, &event_tx).await {
                    Ok(content) if !content.is_empty() => {
                        let _ = event_tx.send(AppEvent::DiffusionUpdate(content.clone()));
                        let _ = event_tx.send(AppEvent::StreamDone);
                    }
                    Ok(_) | Err(_) => {
                        logger::log("MERCURY", "Diffusion failed, falling back to non-streaming");
                        let fallback_req = ChatRequest {
                            model: self.model.clone(),
                            messages: chat_messages.clone(),
                            stream: false,
                            max_tokens: Some(self.max_tokens),
                            tools: None,
                            diffusing: false,
                        };
                        match self.call_api(&fallback_req).await {
                            Ok(resp) => {
                                let content = resp.choices.first()
                                    .and_then(|c| c.message.content.as_ref())
                                    .cloned()
                                    .unwrap_or_else(|| "(empty response)".to_string());
                                let _ = event_tx.send(AppEvent::DiffusionUpdate(content));
                                let _ = event_tx.send(AppEvent::StreamDone);
                            }
                            Err(e) => {
                                let _ = event_tx.send(AppEvent::Error(format!("Both diffusion and fallback failed: {e}")));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let _ = event_tx.send(AppEvent::Error(format!("{e}")));
            }
        }
    }
}
