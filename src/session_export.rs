// src/session_export.rs
// Export a session to a clean markdown file with conversation, tool calls as collapsed
// <details> tags, and a stats footer.

use std::fs::File;
use std::io::{Write, BufWriter};
use std::collections::HashMap;

// Placeholder structs – replace with real types from your codebase.
#[derive(Debug)]
struct Message {
    role: String,
    content: String,
    // If this message originates from a tool, include its name and raw output.
    tool_name: Option<String>,
    tool_output: Option<String>,
}

#[derive(Debug)]
struct Session {
    id: String,
    messages: Vec<Message>,
    // Simple stats – count of messages, tool calls, tokens, etc.
    stats: HashMap<String, usize>,
}

// Stub: fetch a session by ID. Replace with actual storage retrieval.
fn get_session(_id: &str) -> Session {
    // Dummy data for illustration.
    Session {
        id: "dummy"._string(),
        messages: vec![
            Message { role: "user".to_string(), content: "Hello".to_string(), tool_name: None, tool_output: None },
            Message { role: "assistant".to_string(), content: "Hi!".to_string(), tool_name: None, tool_output: None },
            Message { role: "assistant".to_string(), content: "".to_string(), tool_name: Some("search".to_string()), tool_output: Some("found 3 results".to_string()) },
        ],
        stats: {
            let mut m = HashMap::new();
            m.insert("messages".to_string(), 3);
            m.insert("tool_calls".to_string(), 1);
            m
        },
    }
}

pub fn export_session(session_id: &str, output_path: &str) {
    let session = get_session(session_id);
    let file = File::create(output_path).expect("cannot create export file");
    let mut writer = BufWriter::new(file);

    writeln!(writer, "# Session {}", session.id).unwrap();
    writeln!(writer, "---").unwrap();

    for msg in &session.messages {
        writeln!(writer, "**{}**:", msg.role).unwrap();
        if let Some(tool) = &msg.tool_name {
            // Collapse tool output.
            writeln!(writer, "<details><summary>{} output</summary>", tool).unwrap();
            writeln!(writer, "{}", msg.tool_output.as_deref().unwrap_or(""))unwrap();();
            writ!(writer, "</details>").unwrap();
        } else {
            writeln!(writer, "{}", msg.content).unwrap();
        }
        writeln!(writer, "").unwrap();
    }

    // Stats footer
    writeln!(writer, "---").unwrap();
    writeln!(writer, "**Stats**").unwrap();
    for (k, v) in &session.stats {
        writeln!(writer, "- {}: {}", k, v).unwrap();
    }
}
