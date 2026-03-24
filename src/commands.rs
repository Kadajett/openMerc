use crate::app::{App, Role, AppMode};
use crate::tools::{files, tasks as task_tools};
use crate::session;
use std::process::Command;

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
    if trimmed.starts_with('/') {
        return handle_slash_command(app, trimmed);
    }

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
                        format!(
                            "{}...\n(truncated, {} total lines)",
                            crate::logger::safe_truncate(&content, 6000),
                            lines
                        )
                    } else {
                        content
                    };
                    injected.push(format!("## File: {file_path}\n```\n{truncated}\n```"));
                    app.conversation.push_message(
                        Role::System,
                        format!("📎 attached {file_path} ({lines} lines)")
                    );
                }
                Err(e) => {
                    app.conversation.push_message(
                        Role::System,
                        format!("⚠ @{file_path}: {e}")
                    );
                }
            }
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
        "/rename" => {
            if args.is_empty() {
                app.conversation.push_message(Role::System, "Usage: /rename <new-title>".to_string());
            } else {
                app.conversation.title = args.to_string();
                let persisted = session::snapshot_from_app(app);
                let _ = session::save_session(&app.workspace, &persisted);
                app.conversation.push_message(Role::System, format!("Session renamed to: {args}"));
            }
            InputAction::Handled
        }
        "/branch" => {
            match session::git_branch(&app.workspace) {
                Some(branch) => app.conversation.push_message(Role::System, format!("Current git branch: {branch}")),
                None => app.conversation.push_message(Role::System, "Not a git repository or unable to determine branch.".to_string()),
            }
            InputAction::Handled
        }
        "/commit" => {
            if args.is_empty() {
                app.conversation.push_message(Role::System, "Usage: /commit <message>".to_string());
            } else {
                let msg = args.trim();
                let add_status = Command::new("git")
                    .arg("add")
                    .arg(".")
                    .current_dir(&app.workspace)
                    .status();
                match add_status {
                    Ok(status) if status.success() => {
                        let commit_output = Command::new("git")
                            .arg("commit")
                            .arg("-m")
                            .arg(msg)
                            .current_dir(&app.workspace)
                            .output();
                        match commit_output {
                            Ok(out) => {
                                let stdout = String::from_utf8_lossy(&out.stdout);
                                let stderr = String::from_utf8_lossy(&out.stderr);
                                let response = if out.status.success() {
                                    format!("Commit successful:\n{}", stdout)
                                } else {
                                    format!("Commit failed:\n{}", stderr)
                                };
                                app.conversation.push_message(Role::System, response);
                            }
                            Err(e) => {
                                app.conversation.push_message(Role::System, format!("Error running git commit: {e}"));
                            }
                        }
                    }
                    _ => {
                        app.conversation.push_message(Role::System, "git add failed".to_string());
                    }
                }
            }
            InputAction::Handled
        }
        "/model" => {
            if args.is_empty() {
                app.conversation.push_message(Role::System, "Usage: /model <model-name>\nAvailable: mercury-2, mercury-edit".to_string());
            } else {
                app.conversation.push_message(Role::System, "Model switching not yet implemented. Current: mercury-2".to_string());
            }
            InputAction::Handled
        }
        "/files" | "/ls" => {
            let path = if args.is_empty() { "." } else { args };
            match files::list_dir(&app.workspace, path) {
                Ok(entries) => app.conversation.push_message(Role::System, format!("## {path}\n{}", entries.join("\n"))),
                Err(e) => app.conversation.push_message(Role::System, format!("Error: {e}")),
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
                    Err(e) => app.conversation.push_message(Role::System, format!("Error: {e}")),
                }
            }
            InputAction::Handled
        }
        "/think" | "/thought" => {
            if args.is_empty() {
                app.conversation.push_message(Role::System, "Usage: /think <query> — search project memory in Honcho".to_string());
            } else {
                return InputAction::Chat {
                    message: String::new(),
                    injected_context: vec![format!("__THOUGHT_QUERY__:{args}")],
                };
            }
            InputAction::Handled
        }
        "/plan" => {
            if args.is_empty() {
                app.conversation.push_message(Role::System, "Usage: /plan <title> [description]".to_string());
            } else {
                let mut split = args.splitn(2, ' ');
                let title = split.next().unwrap();
                let description = split.next();
                let msg = task_tools::create_task(&mut app.tasks, title, description);
                app.conversation.push_message(Role::System, msg);
                app.app_mode = AppMode::Plan;
            }
            InputAction::Handled
        }
        "/approve" => {
            if args.is_empty() {
                app.conversation.push_message(Role::System, "Usage: /approve <task-id>".to_string());
            } else {
                let id = args.trim();
                let msg = task_tools::update_task(&mut app.tasks, id, Some("completed"), None, None);
                app.conversation.push_message(Role::System, msg);
                app.app_mode = AppMode::Normal;
            }
            InputAction::Handled
        }
        // New command: /git – show git status, recent commits, diff summary
        "/git" => {
            let status = Command::new("git").arg("status").arg("--short").current_dir(&app.workspace).output();
            let log = Command::new("git").arg("log").arg("-n").arg("5").current_dir(&app.workspace).output();
            let diff = Command::new("git").arg("diff").arg("--stat").current_dir(&app.workspace).output();
            let mut out = String::new();
            match status {
                Ok(s) => out.push_str(&format!("## Git status\n{}\n", String::from_utf8_lossy(&s.stdout))),
                Err(e) => out.push_str(&format!("Error git status: {e}\n")),
            }
            match log {
                Ok(l) => out.push_str(&format!("## Recent commits (last 5)\n{}\n", String::from_utf8_lossy(&l.stdout))),
                Err(e) => out.push_str(&format!("Error git log: {e}\n")),
            }
            match diff {
                Ok(d) => out.push_str(&format!("## Diff summary\n{}\n", String::from_utf8_lossy(&d.stdout))),
                Err(e) => out.push_str(&format!("Error git diff: {e}\n")),
            }
            app.conversation.push_message(Role::System, out);
            InputAction::Handled
        }
        // New command: /stats – session statistics
        "/stats" => {
            let msg_count = app.conversation.messages.len();
            let tool_calls = app.pending_tools.len();
            let duration = if let Some(start) = app.request_started {
                std::time::Instant::now().duration_since(start).as_secs()
            } else {
                0
            };
            let stats = format!(
                "## Session stats\nMessages sent: {}\nPending tool calls: {}\nSession time (s): {}\n",
                msg_count, tool_calls, duration
            );
            app.conversation.push_message(Role::System, stats);
            InputAction::Handled
        }
        // New command: /about – easter egg introducing Mercury
        "/about" => {
            let intro = "I am Mercury, a fast‑no‑nonsense coding agent powered by diffusion LLMs.\n";
            let caps = "Capabilities: read/write/search files, generate/refactor/debug code, run shell commands, manage tasks, and integrate Semfora for deep code analysis.\n";
            let vibe = "Sharp, direct, and always ready to act – no fluff.\n";
            let msg = format!("## About Me\n{}\n{}\n{}", intro, caps, vibe);
            app.conversation.push_message(Role::System, msg);
            InputAction::Handled
        }
        // New command: /review – run semfora analysis on the whole workspace and summarize
        "/review" => {
            let result = Command::new("semfora-engine")
                .arg("analyze")
                .arg(".")
                .output();
            match result {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let display = if stdout.len() > 800 {
                        format!("{}...\n(truncated)", &stdout[..800])
                    } else {
                        stdout.to_string()
                    };
                    app.conversation.push_message(Role::System, format!("## Semfora analysis (summary)\n```json\n{display}\n```"));
                }
                Err(e) => {
                    app.conversation.push_message(Role::System, format!("Error running semfora analyze: {e}"));
                }
            }
            InputAction::Handled
        }
        // New command: /search – combine semfora code search and memory dump search
        "/search" => {
            if args.is_empty() {
                app.conversation.push_message(Role::System, "Usage: /search <query>".to_string());
            } else {
                let query = args.trim();
                let mut combined = String::new();
                let code_res = Command::new("semfora-engine")
                    .arg("search")
                    .arg(query)
                    .output();
                match code_res {
                    Ok(out) => {
                        let out_str = String::from_utf8_lossy(&out.stdout);
                        combined.push_str("## Code search results\n");
                        combined.push_str(&out_str);
                    }
                    Err(e) => {
                        combined.push_str(&format!("Error running semfora search: {e}\n"));
                    }
                }
                let mem_res = Command::new("grep")
                    .arg("-i")
                    .arg(query)
                    .arg("memory_dump.md")
                    .output();
                match mem_res {
                    Ok(out) => {
                        let out_str = String::from_utf8_lossy(&out.stdout);
                        combined.push_str("\n## Memory search results\n");
                        combined.push_str(&out_str);
                    }
                    Err(e) => {
                        combined.push_str(&format!("Error searching memory dump: {e}\n"));
                    }
                }
                app.conversation.push_message(Role::System, combined);
            }
            InputAction::Handled
        }
        _ => {
            app.conversation.push_message(Role::System, format!("Unknown command: {cmd}. Type /help for available commands."));
            InputAction::Handled
        }
    }
}

const HELP_TEXT: &str = r#"## Commands
/help, /h          — Show this help
/tasks, /t         — List current tasks
/sessions, /s      — List saved sessions
/title <name>      — Set session title
/rename <name>    — Rename session (persisted)
/branch           — Show current git branch
/commit <msg>     — Commit changes with message
/clear             — Clear chat (session preserved)
/files, /ls [path] — List directory
/cat, /read <file> — Read a file
/think <query>     — Search project memory (Honcho)
/model <name>      — Switch model (TODO)

/ Added commands
/git              — Show git status, recent commits, diff summary
/stats            — Show session statistics
/about            — About Mercury
/review           — Run Semfora analysis on workspace
/search <query>   — Search code via Semfora and memory dump

## Mentions
@path/to/file      — Attach file contents to your message
^keyword           — Pull related Honcho memory into context

## Shortcuts
Ctrl+C             — Quit (or cancel in-progress operation)
Ctrl+Q             — Always quit
Esc                — Switch to chat scroll mode
i / Enter          — Switch to input mode
↑↓ / j/k           — Scroll chat"#;
