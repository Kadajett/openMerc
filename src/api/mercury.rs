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

    fn build_messages(&self, system_context: Option<&str>, messages: &[Message]) -> Vec<ChatMessage> {
        let mut out = Vec::new();
        if let Some(ctx) = system_context {
            out.push(ChatMessage {
                role: "system".to_string(),
                content: Some(ctx.to_string()),
                tool_calls: None,
                tool_call_id: None,
            });
        }
        for msg in messages {
            match msg.role {
                crate::app::Role::Tool | crate::app::Role::System => continue,
                _ => {}
            }
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
            let req = ChatRequest {
                model: self.model.clone(),
                messages: msgs.clone(),
                stream: false,
                max_tokens: Some(self.max_tokens),
                tools: Some(tools.to_vec()),
                diffusing: false,
            };
            let resp = self.call_api(&req).await?;
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
                    msgs.push(ChatMessage {
                        role: "tool".to_string(),
                        content: Some(result),
                        tool_calls: None,
                        tool_call_id: tc.id.clone(),
                    });
                }
                continue; // next round
            }
            // No tool calls – exit loop
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
        let mut chat_messages = self.build_messages(system_context, messages);
        let tools = registry::tool_definitions();
        let max_rounds = 50_u32;
        match self.run_tool_loop(&mut chat_messages, &tools, max_rounds, tool_ctx, &event_tx, &cancel).await {
            Ok(Some(early)) => {
                let _ = event_tx.send(AppEvent::DiffusionUpdate(early.clone()));
                let _ = event_tx.send(AppEvent::StreamDone);
                return;
            }
            Ok(None) => {
                // Diffusion phase
                let diff_req = ChatRequest {
                    model: self.model.clone(),
                    messages: chat_messages.clone(),
                    stream: true,
                    max_tokens: Some(self.max_tokens),
                    tools: None,
                    diffusing: true,
                };
                match self.call_api_diffusing(&diff_req, &event_tx).await {
                    Ok(content) => {
                        let _ = event_tx.send(AppEvent::DiffusionUpdate(content.clone()));
                        let _ = event_tx.send(AppEvent::StreamDone);
                    }
                    Err(e) => {
                        let _ = event_tx.send(AppEvent::Error(format!("{e}")));
                    }
                }
            }
            Err(e) => {
                let _ = event_tx.send(AppEvent::Error(format!("{e}")));
            }
        }
    }
}
