// src/session_branch.rs
use crate::session::Session;
use uuid::Uuid;

/// Forks the given session, creating a new session with a fresh ID and a title suffixed with "(branch)".
/// The cloned session contains a deep copy of the original messages.
pub fn fork_session(original: &Session) -> Session {
    // Clone messages (assuming they implement Clone)
    let mut new_messages = original.messages.clone();
    // Generate a new ID
    let new_id = Uuid::new_v4().to_string();
    // Append branch suffix to title
    let new_title = format!("{} (branch)", original.title);
    // Build the new session
    Session {
        id: new_id,
        title: new_title,
        messages: new_messages,
        ..original.clone()
    }
}
