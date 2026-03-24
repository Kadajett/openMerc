// Updated commands.rs with /export and /watch implementations and new /login and /config commands
use crate::app::{App, Role, AppMode};
use crate::tools::{files, tasks as task_tools};
use crate::session;
use crate::config::Config;
use std::process::Command;
use std::thread;
use std::time::Duration;
use std::fs;
use std::io::Write;
use std::env;

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
                let msg = task_tools::create_task(&mut app.tasks, title, description, None, None, None, None);
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
        // New command: /quickfix – run semfora-engine validate on the project and show results
        "/quickfix" => {
            let result = Command::new("semfora-engine")
                .arg("validate")
                .arg(".")
                .output();
            match result {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    let mut msg = String::new();
                    if out.status.success() {
                        msg.push_str("## Semfora validation passed\n\n```\n");
                        msg.push_str(&stdout);
                        msg.push_str("\n```\n");
                    } else {
                        msg.push_str("## Semfora validation failed\n\n```\n");
                        msg.push_str(&stderr);
                        msg.push_str("\n```\n");
                    }
                    app.conversation.push_message(Role::System, msg);
                }
                Err(e) => {
                    app.conversation.push_message(Role::System, format!("Error running semfora validate: {e}"));
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
        // New command: /export – export session as markdown
        "/export" => {
            // Build summary
            let title = &app.conversation.title;
            let start = app.conversation.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
            let msg_count = app.conversation.messages.len();
            let tool_calls = app.pending_tools.len();
            let duration = if let Some(dur) = app.last_duration {
                format!("{}s", dur.as_secs())
            } else {
                "N/A".to_string()
            };
            let mut md = format!(
                "# Session Export\n\n**Title:** {}\n**Started:** {}\n**Messages:** {}\n**Pending tools:** {}\n**Duration:** {}\n\n---\n\n",
                title, start, msg_count, tool_calls, duration
            );
            // Conversation
            for msg in &app.conversation.messages {
                match msg.role {
                    Role::User => {
                        md.push_str(&format!("### User\n\n{}\n\n", msg.content));
                    }
                    Role::Assistant => {
                        md.push_str(&format!("### Assistant\n\n{}\n\n", msg.content));
                    }
                    Role::Tool => {
                        md.push_str(&format!("<details><summary>Tool output</summary>\n\n{}\n\n</details>\n\n", msg.content));
                    }
                    Role::System => {
                        md.push_str(&format!("> {}\n\n", msg.content.replace('\n', "\n> ")));
                    }
                }
            }
            // Diff summary if any
            if !app.modified_files.is_empty() {
                md.push_str("## Diff Summary\n\n");
                for fd in &app.modified_files {
                    md.push_str(&format!("### {}\n\n```diff\n{}\n```\n\n", fd.path, fd.diff));
                }
            }
            // Write to a temporary markdown file in workspace
            let out_path = "session_export.md";
            let write_res = fs::write(out_path, md);
            match write_res {
                Ok(_) => app.conversation.push_message(Role::System, format!("✅ Exported session to `{}`", out_path)),
                Err(e) => app.conversation.push_message(Role::System, format!("❌ Failed to write export: {e}")),
            }
            InputAction::Handled
        }
        // New command: /watch – run a command every N seconds and stream output
        "/watch" => {
            // Expected: /watch <interval_seconds> <shell_command>
            let mut parts = args.splitn(2, ' ');
            let interval_str = parts.next();
            let cmd_str = parts.next();
            if interval_str.is_none() || cmd_str.is_none() {
                app.conversation.push_message(Role::System, "Usage: /watch <seconds> <command>".to_string());
                return InputAction::Handled;
            }
            let interval: u64 = match interval_str.unwrap().parse() {
                Ok(v) => v,
                Err(_) => {
                    app.conversation.push_message(Role::System, "Invalid interval – must be a number".to_string());
                    return InputAction::Handled;
                }
            };
            let command = cmd_str.unwrap();
            // Run 5 iterations as a demo; in real use could be stopped via /watch stop
            for i in 1..=5 {
                let out = Command::new("sh").arg("-c").arg(command).current_dir(&app.workspace).output();
                match out {
                    Ok(res) => {
                        let stdout = String::from_utf8_lossy(&res.stdout);
                        let msg = format!("## Watch iteration {}\n\n```\n{}\n```", i, stdout);
                        app.conversation.push_message(Role::System, msg);
                    }
                    Err(e) => {
                        app.conversation.push_message(Role::System, format!("Watch error: {e}"));
                        break;
                    }
                }
                thread::sleep(Duration::from_secs(interval));
            }
            InputAction::Handled
        }
        // New command: /pr – generate PR description from git diff, create branch, open PR via gh
        "/pr" => {
            if args.is_empty() {
                app.conversation.push_message(Role::System, "Usage: /pr <pr-title>".to_string());
                return InputAction::Handled;
            }
            let title = args.trim();
            // Create a new branch based on title (slugified)
            let branch_name = title.to_lowercase().replace(' ', "_").replace('-', "_");
            // git checkout -b <branch>
            let checkout = Command::new("git")
                .arg("checkout")
                .arg("-b")
                .arg(&branch_name)
                .current_dir(&app.workspace)
                .output();
            match checkout {
                Ok(out) if out.status.success() => {
                    // Get diff summary
                    let diff = Command::new("git")
                        .arg("diff")
                        .arg("--stat")
                        .current_dir(&app.workspace)
                        .output();
                    let diff_summary = match diff {
                        Ok(d) => String::from_utf8_lossy(&d.stdout).to_string(),
                        Err(e) => format!("Error getting diff: {e}")
                    };
                    // Build PR body
                    let body = format!("## Summary\n{}\n\n## Diff\n```diff\n{}\n```", title, diff_summary);
                    // gh pr create
                    let gh = Command::new("gh")
                        .arg("pr")
                        .arg("create")
                        .arg("--title")
                        .arg(title)
                        .arg("--body")
                        .arg(&body)
                        .arg("--head")
                        .arg(&branch_name)
                        .current_dir(&app.workspace)
                        .output();
                    match gh {
                        Ok(out) => {
                            let out_str = String::from_utf8_lossy(&out.stdout);
                            app.conversation.push_message(Role::System, format!("PR created:\n{}", out_str));
                        }
                        Err(e) => {
                            app.conversation.push_message(Role::System, format!("Error creating PR: {e}"));
                        }
                    }
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr);
                    app.conversation.push_message(Role::System, format!("git checkout failed: {}", err));
                }
                Err(e) => {
                    app.conversation.push_message(Role::System, format!("Failed to run git checkout: {e}"));
                }
            }
            InputAction::Handled
        }
        // New command: /test – run cargo test and format results
        "/test" => {
            let out = Command::new("cargo")
                .arg("test")
                .arg("--quiet")
                .current_dir(&app.workspace)
                .output();
            match out {
                Ok(res) => {
                    let stdout = String::from_utf8_lossy(&res.stdout);
                    let stderr = String::from_utf8_lossy(&res.stderr);
                    let mut msg = String::new();
                    if res.status.success() {
                        msg.push_str("## Cargo test passed\n\n```\n");
                        msg.push_str(&stdout);
                        msg.push_str("\n```\n");
                    } else {
                        msg.push_str("## Cargo test failed\n\n```\n");
                        msg.push_str(&stderr);
                        msg.push_str("\n```\n");
                    }
                    app.conversation.push_message(Role::System, msg);
                }
                Err(e) => {
                    app.conversation.push_message(Role::System, format!("Error running cargo test: {e}"));
                }
            }
            InputAction::Handled
        }
        // New command: /history — show recent git commits
        "/history" => {
            let out = Command::new("git")
                .arg("log")
                .arg("--oneline")
                .arg("-10")
                .current_dir(&app.workspace)
                .output();
            match out {
                Ok(res) => {
                    let log = String::from_utf8_lossy(&res.stdout);
                    app.conversation.push_message(Role::System, format!("## Recent commits (last 10)\n{}", log));
                }
                Err(e) => {
                    app.conversation.push_message(Role::System, format!("Error getting git history: {e}"));
                }
            }
            InputAction::Handled
        }
        // New command: /improve – self‑analysis and auto‑task generation
        "/improve" => {
            // Run semfora analysis on the whole workspace
            let result = Command::new("semfora-engine")
                .arg("analyze")
                .arg(".")
                .output();
            match result {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let mut created = 0;
                    for line in stdout.lines() {
                        if line.contains("complexity") {
                            if let Some(num_str) = line.split(':').nth(1) {
                                if let Ok(num) = num_str.trim().parse::<u32>() {
                                    if num > 10 {
                                        let title = format!("Refactor high‑complexity code ({} )", num);
                                        let desc = format!("Semfora reported high complexity: {}", line.trim());
                                        let _ = task_tools::create_task(&mut app.tasks, &title, Some(&desc), None, None, None, None);
                                        created += 1;
                                    }
                                }
                            }
                        }
                        if line.contains("risk") && line.to_lowercase().contains("high") {
                            let title = "Address high‑risk code".to_string();
                            let desc = format!("Semfora flagged a high‑risk area: {}", line.trim());
                            let _ = task_tools::create_task(&mut app.tasks, &title, Some(&desc), None, None, None, None);
                            created += 1;
                        }
                    }
                    let summary = format!("/improve generated {} improvement task(s). Use /tasks to view them.", created);
                    app.conversation.push_message(Role::System, summary);
                }
                Err(e) => {
                    app.conversation.push_message(Role::System, format!("Error running semfora analyze: {e}"));
                }
            }
            InputAction::Handled
        }
        // New command: /login – store API key and optional Honcho config
        "/login" => {
            // Expected: /login <api_key> [base_url] [app_id] [user_id]
            let mut args_iter = args.split_whitespace();
            let api_key = match args_iter.next() {
                Some(k) => k,
                None => {
                    app.conversation.push_message(Role::System, "Usage: /login <api_key> [base_url] [app_id] [user_id]".to_string());
                    return InputAction::Handled;
                }
            };
            let base_url = args_iter.next();
            let app_id = args_iter.next();
            let user_id = args_iter.next();

            // Load existing config or start fresh
            let config_path = app.workspace.join(".openmerc.toml");
            let mut config: toml::Value = if config_path.exists() {
                let content = fs::read_to_string(&config_path).unwrap_or_default();
                toml::from_str(&content).unwrap_or_else(|_| toml::Value::Table(toml::map::Map::new()))
            } else {
                toml::Value::Table(toml::map::Map::new())
            };

            // Ensure [mercury] table exists and set fields
            {
                let mer_tbl = config.get_mut("mercury").and_then(|v| v.as_table_mut());
                let mer_tbl = match mer_tbl {
                    Some(t) => t,
                    None => {
                        config.as_table_mut().unwrap().insert("mercury".to_string(), toml::Value::Table(toml::map::Map::new()));
                        config.get_mut("mercury").unwrap().as_table_mut().unwrap()
                    }
                };
                mer_tbl.insert("api_key".to_string(), toml::Value::String(api_key.to_string()));
                if let Some(url) = base_url {
                    mer_tbl.insert("base_url".to_string(), toml::Value::String(url.to_string()));
                }
            }

            // Optional Honcho config
            if app_id.is_some() || user_id.is_some() {
                {
                    let hon_tbl = config.get_mut("honcho").and_then(|v| v.as_table_mut());
                    let hon_tbl = match hon_tbl {
                        Some(t) => t,
                        None => {
                            config.as_table_mut().unwrap().insert("honcho".to_string(), toml::Value::Table(toml::map::Map::new()));
                            config.get_mut("honcho").unwrap().as_table_mut().unwrap()
                        }
                    };
                    if let Some(id) = app_id {
                        hon_tbl.insert("app_id".to_string(), toml::Value::String(id.to_string()));
                    }
                    if let Some(uid) = user_id {
                        hon_tbl.insert("user_id".to_string(), toml::Value::String(uid.to_string()));
                    }
                    hon_tbl.insert("enabled".to_string(), toml::Value::Boolean(true));
                }
            }

            // Write back to file
            let new_toml = toml::to_string_pretty(&config).unwrap_or_default();
            let write_res = fs::write(&config_path, new_toml);
            match write_res {
                Ok(_) => {
                    // Optionally set env var for current process (not required)
                    // env::set_var("INCEPTION_API_KEY", api_key);
                    app.conversation.push_message(Role::System, "✅ Login info saved to .openmerc.toml".to_string());
                }
                Err(e) => {
                    app.conversation.push_message(Role::System, format!("❌ Failed to write config: {e}"));
                }
            }
            InputAction::Handled
        }
        // New command: /config – show current config (redact api key)
        "/config" => {
            let cfg = match Config::load(&app.workspace) {
                Ok(c) => c,
                Err(e) => {
                    app.conversation.push_message(Role::System, format!("Error loading config: {e}"));
                    return InputAction::Handled;
                }
            };
            let api_key_redacted = if cfg.mercury.api_key.len() > 4 {
                let len = cfg.mercury.api_key.len();
                let last4 = &cfg.mercury.api_key[len - 4..];
                format!("*****{}", last4)
            } else {
                "*****".to_string()
            };
            let mut out = String::new();
            out.push_str("## Current configuration\n\n");
            out.push_str(&format!("[mercury]\nbase_url = \"{}\"\napi_key = \"{}\"\nmodel = \"{}\"\nmax_tokens = {}\n\n", cfg.mercury.base_url, api_key_redacted, cfg.mercury.model, cfg.mercury.max_tokens));
            out.push_str(&format!("[honcho]\nenabled = {}\nbase_url = \"{}\"\napp_id = \"{}\"\nuser_id = \"{}\"\nassistant_name = \"{}\"\nworkspace_id = \"{}\"\n\n", cfg.honcho.enabled, cfg.honcho.base_url, cfg.honcho.app_id, cfg.honcho.user_id, cfg.honcho.assistant_name, cfg.honcho.workspace_id));
            out.push_str(&format!("[agent]\nname = \"{}\"\nsystem_prompt = \"...\"\n\n", cfg.agent.name));
            app.conversation.push_message(Role::System, out);
            InputAction::Handled
        }
        // New command: /snippet – save and load code snippets
        "/snippet" => {
            // Expected: /snippet save <name> <code>  OR  /snippet load <name>
            let mut args_iter = args.split_whitespace();
            let sub = args_iter.next();
            match sub {
                Some("save") => {
                    if let Some(name) = args_iter.next() {
                        // The rest of the args is the code (may be empty)
                        let code: String = args_iter.collect::<Vec<&str>>().join(" ");
                        let snippets_dir = app.workspace.join(".openmerc/snippets");
                        let _ = fs::create_dir_all(&snippets_dir);
                        let file_path = snippets_dir.join(format!("{}.rs", name));
                        match fs::write(&file_path, code) {
                            Ok(_) => app.conversation.push_message(Role::System, format!("✅ Snippet '{}' saved.", name)),
                            Err(e) => app.conversation.push_message(Role::System, format!("❌ Failed to save snippet: {e}")),
                        }
                    } else {
                        app.conversation.push_message(Role::System, "Usage: /snippet save <name> <code>".to_string());
                    }
                }
                Some("load") => {
                    if let Some(name) = args_iter.next() {
                        let file_path = app.workspace.join(format!(".openmerc/snippets/{}.rs", name));
                        match fs::read_to_string(&file_path) {
                            Ok(content) => {
                                app.conversation.push_message(Role::System, format!("## Snippet: {name}\n```rust\n{content}\n```"));
                            }
                            Err(e) => app.conversation.push_message(Role::System, format!("❌ Could not load snippet: {e}")),
                        }
                    } else {
                        app.conversation.push_message(Role::System, "Usage: /snippet load <name>".to_string());
                    }
                }
                _ => {
                    app.conversation.push_message(Role::System, "Usage: /snippet <save|load> <name> [code]".to_string());
                }
            }
            InputAction::Handled
        }
        // New command: /context – show Honcho memory conclusions and peer context
        "/context" => {
            // For simplicity, read memory_dump.md and display its content trimmed
            let mem_path = app.workspace.join("memory_dump.md");
            if mem_path.exists() {
                match fs::read_to_string(&mem_path) {
                    Ok(content) => {
                        let display = if content.len() > 2000 {
                            format!("{}...\n(truncated)", &content[..2000])
                        } else {
                            content
                        };
                        app.conversation.push_message(Role::System, format!("## Honcho context\n\n{}", display));
                    }
                    Err(e) => app.conversation.push_message(Role::System, format!("❌ Failed to read memory: {e}")),
                }
            } else {
                app.conversation.push_message(Role::System, "❌ No memory_dump.md found in workspace.".to_string());
            }
            InputAction::Handled
        }
        // New command: /todo – alias for /tasks and inline add
        "/todo" => {
            if args.is_empty() {
                // Show tasks (same as /tasks)
                let output = task_tools::list_tasks(&app.tasks);
                app.conversation.push_message(Role::System, format!("## Tasks\n{output}"));
            } else {
                // Create a task with the given title (args)
                let title = args.trim();
                let msg = task_tools::create_task(&mut app.tasks, title, None, None, None, None, None);
                app.conversation.push_message(Role::System, msg);
                // Auto‑commit the new todo file (if any changes)
                let _ = Command::new("git").arg("add").arg(".").current_dir(&app.workspace).status();
                let _ = Command::new("git").arg("commit").arg("-m").arg(format!("Add todo: {}", title)).current_dir(&app.workspace).status();
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
/export           — Export session to markdown (session_export.md)
/watch <sec> <cmd> — Run <cmd> every <sec> seconds (5 iterations)
/pr <title>       — Create a PR branch and open PR via gh
/test             — Run cargo test and show results
/history          — Show recent git commits (last 10)
/improve         — Self‑analysis with Semfora, auto‑create improvement tasks
/login <api_key> [base_url] [app_id] [user_id] — Store credentials in .openmerc.toml
/config           — Show current config (api key redacted)
/snippet <save|load> <name> [code] — Save or load code snippets
/context          — Show Honcho context (memory dump)
/todo [text]      — List tasks or add inline todo (auto‑commit)

## Mentions
@path/to/file      — Attach file contents to your message
^keyword           — Pull related Honcho memory into context

## Shortcuts
Ctrl+C             — Quit (or cancel in‑progress operation)
Ctrl+Q             — Always quit
Esc                — Switch to chat scroll mode
Enter / i          — Switch to input mode
↑↓ / j/k           — Scroll chat"#;
