use crate::session::Session;
use crate::context::Context;
use anyhow::Result;

/// Generate a concise recap of the session, summarizing the last few messages and the
/// current context. This is used by the UI to display a short overview of the session.
pub fn get_session_recap(session: &Session, ctx: &Context) -> Result<String> {
    // Gather the last N messages (e.g., 5) from the session.
    let recent = session.messages().iter().rev().take(5).collect::<Vec<_>>();
    // Build a simple markdown summary.
    let mut recap = String::new();
    recap.push_str("**Session Recap**\n\n");
    for msg in recent.iter().rev() {
        let role = match msg.role {
            crate::session::MessageRole::User => "User",
            crate::session::MessageRole::Assistant => "Assistant",
            crate::session::MessageRole::System => "System",
        };
        recap.push_str(&format!("- {}: {}\n", role, msg.content.trim()));
    }
    // Append a short context summary if available.
    if let Some(summary) = ctx.summary() {
        recap.push_str("\n**Context Summary**\n");
        recap.push_str(&format!("{}\n", summary));
    }
    Ok(recap)
}
