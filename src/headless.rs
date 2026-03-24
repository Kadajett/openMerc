use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use crate::api::mercury::MercuryClient;
use crate::context::honcho::HonchoContext;
use crate::app::{Message, Role};
use crate::tools::registry::{ToolContext};
use crate::event::AppEvent;

/// Run openMerc in headless mode.
/// `prompt` is the user query to send to Mercury.
pub async fn run_headless(prompt: &str) -> Result<()> {
    // Load configuration (same as interactive mode)
    let workspace = std::env::current_dir()?;
    let config = Config::load(&workspace)?;

    // Initialise Mercury client and Honcho context
    let mercury = Arc::new(MercuryClient::from_config(&config.mercury));
    let honcho = Arc::new(Mutex::new(HonchoContext::from_config(&config.honcho)));

    // Build the initial message list (system prompt + user prompt)
    let system_prompt = config.agent.system_prompt.clone();
    let mut messages = vec![];
    messages.push(Message {
        id: uuid::Uuid::new_v4().to_string(),
        role: Role::User,
        content: prompt.to_string(),
        timestamp: chrono::Utc::now(),
    });

    // Event channel to receive Mercury output
    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();
    let cancel = CancellationToken::new();

    // Tool context – empty task list for headless runs
    let tool_ctx = ToolContext {
        workspace: workspace.clone(),
        tasks: Arc::new(Mutex::new(Vec::new())),
        honcho: honcho.clone(),
    };

    // Spawn the chat operation (it will push DiffusionUpdate and StreamDone events)
    let mercury_clone = mercury.clone();
    let tx_clone = tx.clone();
    let cancel_clone = cancel.clone();
    tokio::spawn(async move {
        mercury_clone.chat(Some(&system_prompt), &messages, tool_ctx, tx_clone, cancel_clone).await;
    });

    // Collect the final output from the stream
    let mut output = String::new();
    while let Some(event) = rx.recv().await {
        match event {
            AppEvent::DiffusionUpdate(chunk) => output.push_str(&chunk),
            AppEvent::StreamDone => break,
            AppEvent::Error(e) => {
                eprintln!("Error: {}", e);
                break;
            }
            _ => {}
        }
    }

    // Print the result to stdout
    println!("{}", output);
    Ok(())
}
