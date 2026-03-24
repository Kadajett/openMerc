#![allow(dead_code, unused_imports, unused_variables)]
use anyhow::Result;
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config::HonchoConfig;
use crate::logger;

/// Honcho v3 API client.
/// Uses /v3/workspaces/{workspace_id}/... endpoints.
pub struct HonchoContext {
    client: Client,
    base_url: String,
    workspace_id: String,
    user_id: String,
    assistant_name: String,
    session_id: Option<String>,
    enabled: bool,
    reachable: bool,
    last_search_query: Option<String>,
}

#[derive(Deserialize)]
struct SessionResponse {
    id: String,
}

impl HonchoContext {
    pub fn from_config(config: &HonchoConfig) -> Self {
        let workspace_id = if config.workspace_id.is_empty() {
            config.app_id.clone()
        } else {
            config.workspace_id.clone()
        };

        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(8))
                .build()
                .unwrap_or_default(),
            base_url: config.base_url.clone(),
            workspace_id,
            user_id: config.user_id.clone(),
            assistant_name: config.assistant_name.clone(),
            session_id: None,
            enabled: config.enabled && !config.base_url.is_empty(),
            reachable: true,
            last_search_query: None,
        }
    }

    /// Base URL for workspace-scoped endpoints
    fn ws_url(&self) -> String {
        format!("{}/v3/workspaces/{}", self.base_url, self.workspace_id)
    }

    pub fn is_enabled(&self) -> bool { self.enabled }
    pub fn session_id(&self) -> Option<&str> { self.session_id.as_deref() }
    pub fn set_session_id(&mut self, id: String) { self.session_id = Some(id); }

    // ---- Session Lifecycle ----

    pub async fn start_session(&mut self) -> Result<()> {
        if !self.enabled || !self.reachable { return Ok(()); }

        // Create session under merc's peer so messages are peer-scoped
        let url = format!("{}/peers/{}/sessions", self.ws_url(), self.assistant_name);
        logger::log("HONCHO", &format!("POST {url}"));

        let session_id = uuid::Uuid::new_v4().to_string();
        match self.client.post(&url)
            .json(&serde_json::json!({ "id": session_id }))
            .send().await
        {
            Ok(resp) => {
                let status = resp.status();
                logger::log("HONCHO", &format!("start_session status={status}"));
                if status.is_success() {
                    if let Ok(session) = resp.json::<SessionResponse>().await {
                        self.session_id = Some(session.id.clone());
                        logger::log("HONCHO", &format!("session_id={}", session.id));
                    }
                } else {
                    let body = resp.text().await.unwrap_or_default();
                    logger::log("HONCHO", &format!("start_session error: {body}"));
                }
            }
            Err(e) => {
                logger::log("HONCHO", &format!("start_session error: {e}"));
                self.reachable = false;
            }
        }
        Ok(())
    }

    // ---- Workspace Search (the big one) ----

    /// Search across ALL Honcho conversations/spaces.
    /// Deduplicates repeated queries within the same tool loop.
    pub async fn search_workspace(&mut self, query: &str) -> Option<String> {
        if !self.enabled || !self.reachable { return None; }

        if self.last_search_query.as_deref() == Some(query) {
            logger::log("HONCHO", &format!("search dedup skip: {query}"));
            return Some("(already searched — no additional results)".to_string());
        }
        self.last_search_query = Some(query.to_string());

        let url = format!("{}/search", self.ws_url());
        logger::log("HONCHO", &format!("search query={query}"));

        let resp = self.client.post(&url)
            .json(&serde_json::json!({ "query": query }))
            .send().await;

        match resp {
            Ok(r) => {
                let status = r.status();
                logger::log("HONCHO", &format!("search status={status}"));
                if status.is_success() {
                    let body: serde_json::Value = r.json().await.ok()?;
                    let results = format_search_results(&body);
                    logger::log("HONCHO", &format!("search found {} chars", results.len()));
                    if results.is_empty() { None } else { Some(results) }
                } else {
                    None
                }
            }
            Err(e) => {
                logger::log("HONCHO", &format!("search error: {e}"));
                self.reachable = false;
                None
            }
        }
    }

    // ---- Peer Context ----

    /// Get the peer card for a user (compact biographical facts)
    pub async fn get_peer_context(&self, peer_id: &str, query: &str) -> Option<String> {
        if !self.enabled || !self.reachable { return None; }

        let url = format!("{}/peers/{}/context", self.ws_url(), peer_id);

        let resp = self.client.get(&url)
            .query(&[("search_query", query), ("max_conclusions", "10")])
            .send().await.ok()?;

        if resp.status().is_success() {
            let body: serde_json::Value = resp.json().await.ok()?;
            let card = body["peer_card"].as_array()
                .map(|items| items.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
                    .join("\n"))
                .unwrap_or_default();
            let rep = body["representation"].as_str().unwrap_or("").to_string();

            let mut result = String::new();
            if !card.is_empty() { result.push_str(&card); }
            if !rep.is_empty() {
                if !result.is_empty() { result.push_str("\n\n"); }
                result.push_str(&rep);
            }
            if result.is_empty() { None } else { Some(result) }
        } else {
            None
        }
    }

    // ---- Messages & Turns ----

    pub async fn add_turn(&self, user_message: &str, assistant_message: &str) {
        if !self.enabled || !self.reachable { return; }
        let Some(session_id) = &self.session_id else { return };

        let url = format!("{}/sessions/{}/messages", self.ws_url(), session_id);

        let _ = self.client.post(&url)
            .json(&serde_json::json!([
                { "role": "user", "content": user_message },
                { "role": "assistant", "content": assistant_message }
            ]))
            .send().await;
    }

    // ---- Conclusions ----

    pub async fn create_conclusion(&self, content: &str) {
        if !self.enabled || !self.reachable { return; }

        let url = format!("{}/conclusions", self.ws_url());
        let _ = self.client.post(&url)
            .json(&serde_json::json!({
                "peer_id": self.assistant_name,
                "target_peer_id": self.user_id,
                "conclusions": [content]
            }))
            .send().await;
    }

    pub async fn query_conclusions(&self, query: &str) -> Option<String> {
        if !self.enabled || !self.reachable { return None; }

        let url = format!("{}/conclusions/query", self.ws_url());
        let resp = self.client.post(&url)
            .json(&serde_json::json!({
                "peer_id": self.assistant_name,
                "query": query,
                "top_k": 5
            }))
            .send().await.ok()?;

        if resp.status().is_success() {
            let body: serde_json::Value = resp.json().await.ok()?;
            let results = format_search_results(&body);
            if results.is_empty() { None } else { Some(results) }
        } else {
            None
        }
    }

    /// Retrieve a brief summary of the current session, if available.
    /// Currently implemented by querying conclusions with a generic "session summary" query.
    pub async fn get_session_context(&self) -> Option<String> {
        if !self.enabled || !self.reachable { return None; }

        // Search scoped to merc's peer — only returns Merc's own conversations
        let url = format!("{}/peers/{}/search", self.ws_url(), self.assistant_name);
        logger::log("HONCHO", &format!("peer search: {url}"));

        let resp = self.client.post(&url)
            .json(&serde_json::json!({ "query": "openMerc coding session progress" }))
            .send().await.ok()?;

        if resp.status().is_success() {
            let body: serde_json::Value = resp.json().await.ok()?;
            let results = format_search_results(&body);
            logger::log("HONCHO", &format!("peer search got {} chars", results.len()));
            if results.is_empty() { None } else { Some(results) }
        } else {
            // Fallback to workspace search filtered by peer
            let url = format!("{}/search", self.ws_url());
            let resp = self.client.post(&url)
                .json(&serde_json::json!({ "query": "openMerc merc coding agent" }))
                .send().await.ok()?;
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await.ok()?;
                // Filter results to only merc's peer
                if let Some(arr) = body.as_array() {
                    let filtered: Vec<&serde_json::Value> = arr.iter()
                        .filter(|r| r["peer_id"].as_str() == Some(&self.assistant_name))
                        .collect();
                    if filtered.is_empty() { return None; }
                    let results: Vec<String> = filtered.iter().take(5)
                        .filter_map(|r| r["content"].as_str().map(|c| {
                            let t = crate::logger::safe_truncate(c, 200);
                            t.to_string()
                        }))
                        .collect();
                    if results.is_empty() { None } else { Some(results.join("\n\n")) }
                } else { None }
            } else { None }
        }
    }

    // ---- Context Enrichment ----

    pub async fn enrich_system_prompt(&self, base_prompt: &str, user_query: &str) -> String {
        if !self.enabled || !self.reachable {
            return base_prompt.to_string();
        }

        let mut additions = Vec::new();

        if let Some(conclusions) = self.query_conclusions(user_query).await {
            additions.push(format!("## Project Memory\n{conclusions}"));
        }

        if let Some(peer_ctx) = self.get_peer_context(&self.user_id, user_query).await {
            additions.push(format!("## User Context\n{peer_ctx}"));
        }

        if additions.is_empty() {
            base_prompt.to_string()
        } else {
            format!("{base_prompt}\n\n{}", additions.join("\n\n"))
        }
    }

    // ---- Memory Consolidation ----

    pub async fn schedule_dream(&self) {
        if !self.enabled || !self.reachable { return; }
        let url = format!("{}/schedule_dream", self.ws_url());
        let _ = self.client.post(&url)
            .json(&serde_json::json!({ "peer_id": self.assistant_name }))
            .send().await;
    }
}

/// Format search results from Honcho into readable text
fn format_search_results(body: &serde_json::Value) -> String {
    let mut lines = Vec::new();

    // Handle array of results (workspace search returns this)
    if let Some(items) = body.as_array() {
        for item in items.iter().take(8) {
            if let Some(content) = item["content"].as_str() {
                let peer = item["peer_id"].as_str().unwrap_or("?");
                let channel = item["metadata"]["channel_name"].as_str().unwrap_or("");
                let source = if !channel.is_empty() {
                    format!("{peer} in {channel}")
                } else {
                    peer.to_string()
                };
                let truncated = if content.len() > 400 {
                    format!("{}...", crate::logger::safe_truncate(content, 400))
                } else {
                    content.to_string()
                };
                lines.push(format!("[{source}] {truncated}"));
            }
        }
    }

    // Handle object with items/conclusions field
    for key in &["items", "conclusions"] {
        if let Some(items) = body[key].as_array() {
            for item in items.iter().take(8) {
                if let Some(content) = item["content"].as_str() {
                    let truncated = if content.len() > 400 {
                        format!("{}...", crate::logger::safe_truncate(content, 400))
                    } else {
                        content.to_string()
                    };
                    lines.push(truncated);
                }
            }
        }
    }

    lines.join("\n\n")
}
