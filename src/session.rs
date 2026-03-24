use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::app::{Message, Task};

/// Metadata for a single session (stored in index)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub honcho_session_id: Option<String>,
    pub workspace: String,
    pub git_branch: Option<String>,
    pub message_count: usize,
}

/// Index of all sessions for this workspace
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionIndex {
    pub sessions: Vec<SessionMeta>,
}

/// A full persisted session (stored as individual JSON file)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSession {
    pub meta: SessionMeta,
    pub messages: Vec<Message>,
    pub tasks: Vec<Task>,
    pub summary: Option<String>,
}

/// Get the session storage directory for a workspace
pub fn session_dir(workspace: &Path) -> PathBuf {
    workspace.join(".openmerc").join("sessions")
}

/// Ensure the session directory exists
pub fn ensure_session_dir(workspace: &Path) -> Result<()> {
    let dir = session_dir(workspace);
    std::fs::create_dir_all(&dir)?;
    Ok(())
}

/// Load the session index (returns empty if doesn't exist)
pub fn load_index(workspace: &Path) -> SessionIndex {
    let path = session_dir(workspace).join("index.json");
    if !path.exists() {
        return SessionIndex::default();
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => SessionIndex::default(),
    }
}

/// Save the session index
pub fn save_index(workspace: &Path, index: &SessionIndex) -> Result<()> {
    ensure_session_dir(workspace)?;
    let path = session_dir(workspace).join("index.json");
    let content = serde_json::to_string_pretty(index)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Load a specific session by ID
pub fn load_session(workspace: &Path, id: &str) -> Result<PersistedSession> {
    let path = session_dir(workspace).join(format!("{id}.json"));
    let content = std::fs::read_to_string(&path)?;
    let session: PersistedSession = serde_json::from_str(&content)?;
    Ok(session)
}

/// Save a session (both the session file and update the index)
pub fn save_session(workspace: &Path, session: &PersistedSession) -> Result<()> {
    ensure_session_dir(workspace)?;

    // Write session file
    let path = session_dir(workspace).join(format!("{}.json", session.meta.id));
    let content = serde_json::to_string_pretty(session)?;
    std::fs::write(path, content)?;

    // Update index
    let mut index = load_index(workspace);
    if let Some(existing) = index.sessions.iter_mut().find(|s| s.id == session.meta.id) {
        *existing = session.meta.clone();
    } else {
        index.sessions.push(session.meta.clone());
    }

    // Sort by updated_at descending (most recent first)
    index.sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    save_index(workspace, &index)?;
    Ok(())
}

/// Delete a session
pub fn delete_session(workspace: &Path, id: &str) -> Result<()> {
    let path = session_dir(workspace).join(format!("{id}.json"));
    if path.exists() {
        std::fs::remove_file(path)?;
    }

    let mut index = load_index(workspace);
    index.sessions.retain(|s| s.id != id);
    save_index(workspace, &index)?;
    Ok(())
}

/// Get the current git branch name (if in a git repo)
pub fn git_branch(workspace: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .current_dir(workspace)
        .output()
        .ok()?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch.is_empty() { None } else { Some(branch) }
    } else {
        None
    }
}

/// Build a PersistedSession from current App state
pub fn snapshot_from_app(app: &crate::app::App) -> PersistedSession {
    PersistedSession {
        meta: SessionMeta {
            id: app.session_id.clone(),
            title: app.conversation.title.clone(),
            created_at: app.conversation.created_at,
            updated_at: Utc::now(),
            honcho_session_id: app.honcho_session_id.clone(),
            workspace: app.workspace.display().to_string(),
            git_branch: git_branch(&app.workspace),
            message_count: app.conversation.messages.len(),
        },
        messages: app.conversation.messages.clone(),
        tasks: app.tasks.clone(),
        summary: None,
    }
}

/// Restore App state from a PersistedSession
pub fn restore_to_app(app: &mut crate::app::App, session: PersistedSession) {
    app.session_id = session.meta.id;
    app.honcho_session_id = session.meta.honcho_session_id;
    app.conversation.messages = session.messages;
    app.conversation.title = session.meta.title;
    app.conversation.created_at = session.meta.created_at;
    app.tasks = session.tasks;
}
