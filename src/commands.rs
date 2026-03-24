use crate::app::{App, Role};
use crate::tools::{files, tasks as task_tools};
use crate::session;

/// Result of processing user input
pub enum InputAction {
    /// Send to Mercury as a chat message (possibly with injected context)
    Chat {
        /// The user's message text (cleaned of special syntax)
        message: String,
        /// Extra context to prepend to the system prompt for this turn
        injected_context: Vec<String>,
    },
    /// Command was handled locally, don't send to Mercury
    Handled,
}

/// Process user input — detect slash commands, @file mentions, ^thought queries
pub fn process_input(app: &mut App, raw_input: &str) -> InputAction {
    let trimmed = raw_input.trim();

    // Slash commands
    if trimmed.starts_with('/') {
        return handle_slash_command(app, trimmed);
    }

    // Parse @file mentions and ^thought queries from the message
    let mut injected = Vec::new();
    let mut cleaned_message = String::new();
    let mut thoughts_to_query = Vec::new();

    for word in trimmed.split_whitespace() {
        if word.starts_with('@') && word.len() > 1 {
            let file_path = &word[1..];
            match files::read_file(&app.workspace, file_path) {
                Ok(content) => {
                    let lines = content.lines().count();
                    let truncated = if content.len() > 6000 {
                        format!("{}...\n(truncated, {} total lines)", crate::logger::safe_truncate(&content, 6000), lines)
                    } else {
                        content
                    };
                    injected.push(format!("## File: {file_path}\n```\n{truncated}\n```"));
                    app.conversation.push_message(
                        Role::System,
                        format!("📎 attached {file_path} ({lines} lines)"),
                    );
                }
                Err(e) => {
                    app.conversation.push_message(
                        Role::System,
                        format!("⚠ @{file_path}: {e}"),
                    );
                }
            }
            // Keep the @mention in the message so Merc knows about it
            cleaned_message.push_str(word);
            cleaned_message.push(' ');
        } else if word.starts_with('^') && word.len() > 1 {
            let thought = &word[1..];
            thoughts_to_query.push(thought.to_string());
            cleaned_message.push_str(word);
            cleaned_message.push(' ');
        } else {
            cleaned_message.push_str(word);
            cleaned_message.push(' ');
        }
    }

    // ^thought queries get resolved async in main.rs (need honcho access)
    // Store them as a hint in injected context — main.rs will resolve them
    for thought in &thoughts_to_query {
        injected.push(format!("__THOUGHT_QUERY__:{thought}"));
    }

    InputAction::Chat {
        message: cleaned_message.trim().to_string(),
        injected_context: injected,
    }
}

/// Handle a slash command, return Handled if consumed locally
fn handle_slash_command(app: &mut App, input: &str) -> InputAction {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    let cmd = parts[0];
    let args = parts.get(1).copied().unwrap_or("");

    match cmd {
        "/help" | "/h" => {
            app.conversation.push_message(Role::System, HELP_TEXT.to_string());
            InputAction::Handled
        }
        "/tasks" | "/t" => {
            let output = task_tools::list_tasks(&app.tasks);
            app.conversation.push_message(Role::System, format!("## Tasks\n{output}"));
            InputAction::Handled
        }
        "/sessions" | "/s" => {
            let index = session::load_index(&app.workspace);
            if index.sessions.is_empty() {
                app.conversation.push_message(Role::System, "No saved sessions.".to_string());
            } else {
                let mut lines = vec!["## Sessions".to_string()];
                for (i, s) in index.sessions.iter().enumerate() {
                    let active = if s.id == app.session_id { " (active)" } else { "" };
                    lines.push(format!("{}. {} — {} msgs{active}", i + 1, s.title, s.message_count));
                }
                app.conversation.push_message(Role::System, lines.join("\n"));
            }
            InputAction::Handled
        }
        "/title" => {
            if args.is_empty() {
                app.conversation.push_message(Role::System, format!("Session title: {}", app.conversation.title));
            } else {
                app.conversation.title = args.to_string();
                app.conversation.push_message(Role::System, format!("Title set: {args}"));
            }
            InputAction::Handled
        }
        "/clear" => {
            app.conversation.messages.clear();
            app.conversation.push_message(Role::System, "Chat cleared. Session preserved.".to_string());
            InputAction::Handled
        }
        "/model" => {
            if args.is_empty() {
                app.conversation.push_message(Role::System, "Usage: /model <model-name>\nAvailable: mercury-2, mercury-edit".to_string());
            } else {
                app.conversation.push_message(Role::System, format!("Model switching not yet implemented. Current: mercury-2"));
            }
            InputAction::Handled
        }
        "/files" | "/ls" => {
            let path = if args.is_empty() { "." } else { args };
            match files::list_dir(&app.workspace, path) {
                Ok(entries) => {
                    app.conversation.push_message(Role::System, format!("## {path}\n{}", entries.join("\n")));
                }
                Err(e) => {
                    app.conversation.push_message(Role::System, format!("Error: {e}"));
                }
            }
            InputAction::Handled
        }
        "/cat" | "/read" => {
            if args.is_empty() {
                app.conversation.push_message(Role::System, "Usage: /cat <file-path>".to_string());
            } else {
                match files::read_file(&app.workspace, args) {
                    Ok(content) => {
                        let lines = content.lines().count();
                        let display = if content.len() > 4000 {
                            format!("{}...\n({lines} total lines)", crate::logger::safe_truncate(&content, 4000))
                        } else {
                            content
                        };
                        app.conversation.push_message(Role::System, format!("## {args}\n```\n{display}\n```"));
                    }
                    Err(e) => {
                        app.conversation.push_message(Role::System, format!("Error: {e}"));
                    }
                }
            }
            InputAction::Handled
        }
        "/think" | "/thought" => {
            if args.is_empty() {
                app.conversation.push_message(Role::System, "Usage: /think <query> — search project memory in Honcho".to_string());
            } else {
                // Return as a thought query to be resolved async
                return InputAction::Chat {
                    message: String::new(),
                    injected_context: vec![format!("__THOUGHT_QUERY__:{args}")],
                };
            }
            InputAction::Handled
        }
        _ => {
            app.conversation.push_message(
                Role::System,
                format!("Unknown command: {cmd}. Type /help for available commands."),
            );
            InputAction::Handled
        }
    }
}

const HELP_TEXT: &str = r#"## Commands
/help, /h          — Show this help
/tasks, /t         — List current tasks
/sessions, /s      — List saved sessions
/title <name>      — Set session title
/clear             — Clear chat (session preserved)
/files, /ls [path] — List directory
/cat, /read <file> — Read a file
/think <query>     — Search project memory (Honcho)
/model <name>      — Switch model (TODO)

## Mentions
@path/to/file      — Attach file contents to your message
^keyword           — Pull related Honcho memory into context

## Shortcuts
Ctrl+C             — Quit (or cancel in-progress operation)
Ctrl+Q             — Always quit
Esc                — Switch to chat scroll mode
i / Enter          — Switch to input mode
↑↓ / j/k           — Scroll chat"#;
