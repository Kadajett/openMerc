use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::time::Duration;
use tokio::sync::mpsc;

/// Application events
#[derive(Debug)]
pub enum AppEvent {
    /// A key was pressed
    Key(KeyEvent),
    /// Mouse event
    Mouse(MouseEvent),
    /// Terminal was resized
    Resize(u16, u16),
    /// A chunk of streamed text arrived
    StreamChunk(String),
    /// Stream completed
    StreamDone,
    /// An error occurred
    Error(String),
    /// A tool is being called (name, arguments)
    ToolUse(String, String),
    /// A tool returned a result (name, result)
    ToolResult(String, String),
    /// Diffusion update — full content replacement (text crystallizing from noise)
    DiffusionUpdate(String),
    /// Agent progress during multi-step work (round, max_rounds, action description)
    AgentProgress(u32, u32, String),
    /// Tasks were updated (synced from async tool context)
    TaskUpdated(Vec<crate::app::Task>),
    /// A file was modified (path, diff)
    FileModified(String, String),
    /// Tick for UI refresh
    Tick,
}

/// Spawns a background task that reads terminal events and forwards them
pub fn spawn_event_reader(tx: mpsc::UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        loop {
            // Poll every 50ms so we get responsive updates
            if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                match event::read() {
                    Ok(CrosstermEvent::Key(key)) => {
                        if tx.send(AppEvent::Key(key)).is_err() {
                            break;
                        }
                    }
                    Ok(CrosstermEvent::Mouse(mouse)) => {
                        let _ = tx.send(AppEvent::Mouse(mouse));
                    }
                    Ok(CrosstermEvent::Resize(w, h)) => {
                        let _ = tx.send(AppEvent::Resize(w, h));
                    }
                    _ => {}
                }
            } else {
                // Send tick for UI refresh
                if tx.send(AppEvent::Tick).is_err() {
                    break;
                }
            }
        }
    });
}
