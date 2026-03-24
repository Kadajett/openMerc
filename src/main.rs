//! Crate-level warnings suppression
#![allow(dead_code, unused_imports, unused_variables, dead_code)]

mod app;
mod commands;
mod config;
mod logger;
mod event;
mod session;
mod ui;
mod tools;
mod api;
mod context;

mod semfora;
mod headless;
mod engine;
mod plan;
mod progress;

use anyhow::Result;
use crossterm::{
    event::{KeyCode, KeyModifiers, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::time;

use app::{App, AppMode, FocusPanel, Role};
use api::mercury::MercuryClient;
use config::Config;
use context::honcho::HonchoContext;

use event::{AppEvent, spawn_event_reader};

/// Restore terminal state — called on clean exit, panic, or signal
fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(
        io::stdout(),
        LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    );
}

#[tokio::main]
async fn main() -> Result<()> {
    // Check for headless mode before any UI setup
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--headless") {
        // Collect everything after the flag as the prompt
        let prompt = if let Some(idx) = args.iter().position(|a| a == "--headless") {
            args.iter().skip(idx + 1).cloned().collect::<Vec<String>>().join(" ")
        } else {
            "".to_string()
        };
        // Run headless and exit
        return headless::run_headless(&prompt).await;
    }

    // Set up panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        original_hook(info);
    }));

    // Workspace is CWD at launch
    let workspace = std::env::current_dir()?;

    // Load config
    let config = Config::load(&workspace)?;

    // Init logger
    logger::init(&workspace);
    logger::log_event("openMerc starting");

    // Ensure session directory exists
    session::ensure_session_dir(&workspace)?;

    // Load session index
    let index = session::load_index(&workspace);

    // Init Mercury client
    let mercury = Arc::new(MercuryClient::from_config(&config.mercury));

    // Init Honcho context
    let honcho = Arc::new(Mutex::new(HonchoContext::from_config(&config.honcho)));

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::event::EnableMouseCapture,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        crossterm::cursor::MoveTo(0, 0)
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // App state
    let mut app = App::new(workspace.clone());
    let system_prompt = config.agent.system_prompt.clone();
    let _agent_name = config.agent.name.clone();

    // Session selection
    if !index.sessions.is_empty() {
        app.app_mode = AppMode::SessionPicker;
        let session_titles: Vec<String> = index.sessions.iter().map(|s| {
            let age = chrono::Utc::now().signed_duration_since(s.updated_at);
            let age_str = if age.num_hours() < 1 {
                format!("{}m ago", age.num_minutes())
            } else if age.num_hours() < 24 {
                format!("{}h ago", age.num_hours())
            } else {
                format!("{}d ago", age.num_days())
            };
            format!("{} ({} msgs, {})", s.title, s.message_count, age_str)
        }).collect();
        let (picker_tx, mut picker_rx) = mpsc::unbounded_channel::<AppEvent>();
        spawn_event_reader(picker_tx);
        loop {
            terminal.draw(|f| {
                ui::draw_session_picker(f, &session_titles, app.picker_selected);
            })?;
            if let Some(event) = picker_rx.recv().await {
                match event {
                    AppEvent::Key(key) => {
                        match key.code {
                            KeyCode::Up => {
                                if app.picker_selected > 0 {
                                    app.picker_selected -= 1;
                                }
                            }
                            KeyCode::Down => {
                                if app.picker_selected < index.sessions.len() {
                                    app.picker_selected += 1;
                                }
                            }
                            KeyCode::Enter => {
                                if app.picker_selected < index.sessions.len() {
                                    let meta = &index.sessions[app.picker_selected];
                                    if let Ok(persisted) = session::load_session(&workspace, &meta.id) {
                                        session::restore_to_app(&mut app, persisted);
                                        if let Some(honcho_id) = &app.honcho_session_id {
                                            let mut h = honcho.lock().await;
                                            h.set_session_id(honcho_id.clone());
                                        }
                                        app.conversation.push_message(
                                            Role::System,
                                            format!("Session resumed: {} ({} messages)", app.conversation.title, app.conversation.messages.len() - 1),
                                        );
                                    }
                                } else {
                                    let mut h = honcho.lock().await;
                                    if h.is_enabled() {
                                        let _ = h.start_session().await;
                                        app.honcho_session_id = h.session_id().map(|s| s.to_string());
                                    }
                                }
                                break;
                            }
                            KeyCode::Char('n') => {
                                let mut h = honcho.lock().await;
                                if h.is_enabled() {
                                    let _ = h.start_session().await;
                                    app.honcho_session_id = h.session_id().map(|s| s.to_string());
                                }
                                break;
                            }
                            KeyCode::Char('q') => {
                                restore_terminal();
                                terminal.show_cursor()?;
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
        app.app_mode = AppMode::Normal;
    } else {
        let mut h = honcho.lock().await;
        if h.is_enabled() {
            let _ = h.start_session().await;
            app.honcho_session_id = h.session_id().map(|s| s.to_string());
        }
    }

    // Show welcome message if this is a new session (no resumed messages)
    if app.conversation.messages.is_empty() || app.conversation.messages.iter().all(|m| m.role == Role::System && m.content.contains("resumed")) {
        if app.conversation.messages.is_empty() {
            app.conversation.push_message(
                Role::System,
                format!(
                    "{}{}",
                    concat!(
                        "  __  __ _____ ____   ____\n",
                        " |  \\/  | ____|  _ \\ / ___|\n",
                        " | |\\/| |  _| | |_) | |    \n",
                        " | |  | | |___|  _ <| |___ \n",
                        " |_|  |_|_____|_| \\_\\\\____|\n",
                    ),
                    if config.mercury.api_key.is_empty() {
                        "\n⚠ No API key. Set MERCURY_API_KEY or INCEPTION_API_KEY".to_string()
                    } else {
                        format!("\n {} — /help for commands", app.workspace.display())
                    }
                ),
            );
        }
    }

    // Event channel for main loop
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AppEvent>();
    spawn_event_reader(event_tx.clone());

    // Tick interval for UI refresh
    let mut tick_interval = time::interval(Duration::from_millis(50));

    // Main loop
    loop {
        tokio::select! {
            _ = tick_interval.tick() => {
                terminal.draw(|f| ui::draw(f, &app))?;
            }
            Some(event) = event_rx.recv() => {
                match event {
                    AppEvent::Key(key) => handle_key(&mut app, key, &event_tx, &mercury, &honcho, &system_prompt),
                    AppEvent::StreamChunk(chunk) => app.append_stream(&chunk),
                    AppEvent::Mouse(mouse) => handle_mouse(&mut app, mouse),
                    AppEvent::DiffusionUpdate(content) => {
                        app.stream_buffer = content;
                        app.loading = true;
                    }
                    AppEvent::StreamDone => {
                        // Record the turn in Honcho + auto-conclusion for meaningful exchanges
                        let assistant_msg = app.stream_buffer.clone();
                        let had_tool_calls = app.conversation.messages.iter().rev().take(10).any(|m| m.role == Role::Tool);
                        let meaningful = assistant_msg.len() > 200 || had_tool_calls;
                        if !assistant_msg.is_empty() {
                            let user_msg = app.conversation.messages.iter().rev().find(|m| m.role == Role::User).map(|m| m.content.clone()).unwrap_or_default();
                            let honcho_ref = honcho.clone();
                            let user_msg_clone = user_msg.clone();
                            let assistant_clone = assistant_msg.clone();
                            tokio::spawn(async move {
                                let h = honcho_ref.lock().await;
                                h.add_turn(&user_msg_clone, &assistant_clone).await;
                                if meaningful {
                                    let summary = if assistant_clone.len() > 150 {
                                        format!("{}...", crate::logger::safe_truncate(&assistant_clone, 150))
                                    } else { assistant_clone };
                                    h.create_conclusion(&format!("User asked: {}\nMerc did: {}", if user_msg_clone.len() > 100 { crate::logger::safe_truncate(&user_msg_clone, 100) } else { &user_msg_clone }, summary)).await;
                                }
                            });
                        }
                        app.finalize_stream();
                        // Fetch Honcho summaries for the Log tab
                        {
                            let honcho_ref = honcho.clone();
                            let tx = event_tx.clone();
                            tokio::spawn(async move {
                                let h = honcho_ref.lock().await;
                                if let Some(ctx) = h.get_session_context().await {
                                    let _ = tx.send(AppEvent::HonchoSummary(ctx));
                                }
                            });
                        }
                        // Auto-save session
                        let snapshot = session::snapshot_from_app(&app);
                        let ws = workspace.clone();
                        tokio::spawn(async move { let _ = session::save_session(&ws, &snapshot); });
                        // Engine-driven auto-continue
                        if let Some(mut plan) = app.active_plan.take() {
                            // Sync tasks from app to plan
                            plan.tasks = app.tasks.clone();
                            let engine_config = engine::EngineConfig::default();
                            let action = engine::next_action(&mut plan, &engine_config);

                            match action {
                                engine::EngineAction::Continue(prompt) => {
                                    app.loading = true;
                                    app.request_started = Some(std::time::Instant::now());
                                    app.pending_tools.clear();
                                    app.last_duration = None;
                                    let mercury = mercury.clone();
                                    let honcho = honcho.clone();
                                    let base_prompt = system_prompt.to_string();
                                    let mut messages = app.conversation.messages.clone();
                                    messages.push(app::Message { id: uuid::Uuid::new_v4().to_string(), role: app::Role::User, content: prompt, timestamp: chrono::Utc::now() });
                                    let tx = event_tx.clone();
                                    let workspace_clone = app.workspace.clone();
                                    let tasks_arc = Arc::new(Mutex::new(app.tasks.clone()));
                                    let cancel = tokio_util::sync::CancellationToken::new();
                                    app.cancel_token = Some(cancel.clone());
                                    let task_context = app.tasks_as_context().unwrap_or_default();
                                    let tasks_for_sync = tasks_arc.clone();
                                    tokio::spawn(async move {
                                        let mut enriched_prompt = { let h = honcho.lock().await; h.enrich_system_prompt(&base_prompt, "continue tasks").await };
                                        if !task_context.is_empty() { enriched_prompt = format!("{enriched_prompt}\n\n{task_context}"); }
                                        let tool_ctx = tools::registry::ToolContext { workspace: workspace_clone, tasks: tasks_for_sync, honcho: honcho.clone() };
                                        mercury.chat(Some(&enriched_prompt), &messages, tool_ctx, tx, cancel).await;
                                    });
                                    logger::log_event(&format!("Engine continue: phase={}", plan.phase));
                                    app.active_plan = Some(plan);
                                }
                                engine::EngineAction::PhaseTransition(phase, msg) => {
                                    app.conversation.push_message(app::Role::System, format!("⚙ {msg}"));
                                    logger::log_event(&format!("Engine phase transition: {phase}"));
                                    app.active_plan = Some(plan);
                                }
                                engine::EngineAction::Complete(summary) => {
                                    app.conversation.push_message(app::Role::System, format!("✓ {summary}"));
                                    app.active_plan = None;
                                    logger::log_event("Engine: plan completed");
                                }
                                engine::EngineAction::BudgetExhausted(msg) => {
                                    app.conversation.push_message(app::Role::System, format!("⚠ {msg}"));
                                    app.active_plan = None;
                                }
                                engine::EngineAction::Paused => {
                                    app.active_plan = Some(plan);
                                }
                            }
                        } else {
                            // Legacy auto-continue for tasks without a plan
                            let has_pending = app.tasks.iter().any(|t| matches!(t.status, app::TaskStatus::Pending | app::TaskStatus::InProgress));
                            if has_pending && !app.loading {
                                app.loading = true;
                                app.request_started = Some(std::time::Instant::now());
                                app.pending_tools.clear();
                                let mercury = mercury.clone();
                                let honcho = honcho.clone();
                                let base_prompt = system_prompt.to_string();
                                let mut messages = app.conversation.messages.clone();
                                messages.push(app::Message { id: uuid::Uuid::new_v4().to_string(), role: app::Role::User, content: "Continue working on the remaining tasks.".to_string(), timestamp: chrono::Utc::now() });
                                let tx = event_tx.clone();
                                let workspace_clone = app.workspace.clone();
                                let tasks_arc = Arc::new(Mutex::new(app.tasks.clone()));
                                let cancel = tokio_util::sync::CancellationToken::new();
                                app.cancel_token = Some(cancel.clone());
                                let task_context = app.tasks_as_context().unwrap_or_default();
                                let tasks_for_sync = tasks_arc.clone();
                                tokio::spawn(async move {
                                    let mut enriched_prompt = { let h = honcho.lock().await; h.enrich_system_prompt(&base_prompt, "continue tasks").await };
                                    if !task_context.is_empty() { enriched_prompt = format!("{enriched_prompt}\n\n{task_context}"); }
                                    let tool_ctx = tools::registry::ToolContext { workspace: workspace_clone, tasks: tasks_for_sync, honcho: honcho.clone() };
                                    mercury.chat(Some(&enriched_prompt), &messages, tool_ctx, tx, cancel).await;
                                });
                            }
                        }
                    }
                    AppEvent::ToolUse(name, args) => {
                        let summary = summarize_tool_args(&name, &args);
                        let visible = matches!(name.as_str(), "read_file" | "write_file");
                        app.pending_tools.push(app::ToolLogEntry { name: name.clone(), args_summary: summary, result: None, visible });
                    }
                    AppEvent::ToolResult(name, result) => {
                        if name == "write_file" {
                            if let Some(entry) = app.pending_tools.iter().rev().find(|t| t.name == "write_file") {
                                let path = entry.args_summary.split(',').next().unwrap_or("?").trim().to_string();
                                if !result.contains("REVERTED") && !result.contains("Error") {
                                    let _ = event_tx.send(AppEvent::FileModified(path, result.clone()));
                                }
                            }
                        }
                        if let Some(entry) = app.pending_tools.iter_mut().rev().find(|t| t.name == name && t.result.is_none()) {
                            entry.result = Some(result);
                        }
                        if app.pending_tools.len() == 1 && app.pending_tools[0].visible {
                            app.flush_tool_log();
                        }
                    }
                    AppEvent::Error(e) => {
                        app.loading = false;
                        app.stream_buffer.clear();
                        app.conversation.push_message(Role::System, format!("✗ {e}"));
                    }
                    AppEvent::AgentProgress(round, max_rounds, action) => {
                        app.agent_progress = Some(app::AgentProgressInfo { round, max_rounds, current_action: action });
                    }
                    AppEvent::TaskUpdated(tasks) => {
                        app.tasks = tasks;
                    }
                    AppEvent::FileModified(path, diff) => {

                        if let Some(existing) = app.modified_files.iter_mut().find(|f| f.path == path) {
                            existing.diff = diff;
                            existing.timestamp = chrono::Utc::now();
                        } else {
                            app.modified_files.push(app::FileDiff { path, diff, timestamp: chrono::Utc::now() });
                        }
                        app.diff_selected = app.modified_files.len().saturating_sub(1);
                    }
                    AppEvent::HonchoSummary(summary) => {
                        app.change_log.push(app::ChangeLogEntry {
                            timestamp: chrono::Utc::now(),
                            summary,
                        });
                    }
                    AppEvent::Resize(_, _) | AppEvent::Tick => {}
                }
            }
        }
        if app.should_quit { break; }
    }

    // Save session on exit
    let snapshot = session::snapshot_from_app(&app);
    let _ = session::save_session(&workspace, &snapshot);

    // Restore terminal
    restore_terminal();
    terminal.show_cursor()?;

    Ok(())
}

fn handle_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    event_tx: &mpsc::UnboundedSender<AppEvent>,
    mercury: &Arc<MercuryClient>,
    honcho: &Arc<Mutex<HonchoContext>>,
    system_prompt: &str,
) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('q') {
        app.should_quit = true;
        return;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        if app.loading {
            if let Some(token) = app.cancel_token.take() { token.cancel(); }
            app.loading = false;
            app.stream_buffer.clear();
            app.agent_progress = None;
            app.pending_tools.clear();
            app.conversation.push_message(Role::System, "Cancelled.".to_string());
        } else {
            app.should_quit = true;
        }
        return;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('d') {
        app.show_diff_panel = !app.show_diff_panel;
        return;
    }
    // Tab cycling: Ctrl+1/2/3 or Tab to switch side panel tabs
    if key.code == KeyCode::Tab && !key.modifiers.contains(KeyModifiers::SHIFT) {
        app.side_tab = match app.side_tab {
            app::SideTab::Diff => app::SideTab::Log,
            app::SideTab::Log => app::SideTab::Tasks,
            app::SideTab::Tasks => app::SideTab::Diff,
        };
        return;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('1') => { app.side_tab = app::SideTab::Diff; return; }
            KeyCode::Char('2') => { app.side_tab = app::SideTab::Log; return; }
            KeyCode::Char('3') => { app.side_tab = app::SideTab::Tasks; return; }
            _ => {}
        }
    }
    if key.code == KeyCode::Esc {
        if app.loading {
            if let Some(token) = app.cancel_token.take() { token.cancel(); }
            app.loading = false;
            app.stream_buffer.clear();
            app.agent_progress = None;
            app.pending_tools.clear();
            app.conversation.push_message(app::Role::System, "Cancelled.".to_string());
        }
        app.focus = FocusPanel::Chat;
        return;
    }
    match app.focus {
        FocusPanel::Input => handle_input_key(app, key, event_tx, mercury, honcho, system_prompt),
        FocusPanel::Chat => handle_chat_key(app, key),
        FocusPanel::Files => {}
    }
}

fn handle_input_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    event_tx: &mpsc::UnboundedSender<AppEvent>,
    mercury: &Arc<MercuryClient>,
    honcho: &Arc<Mutex<HonchoContext>>,
    system_prompt: &str,
) {
    match key.code {
        KeyCode::Enter => {
            if !app.loading {
                let raw = app.input.clone();
                app.input.clear();
                app.cursor_pos = 0;
                let action = commands::process_input(app, &raw);
                match action {
                    commands::InputAction::Handled => {}
                    commands::InputAction::Chat { message, injected_context } => {
                        if !message.is_empty() || !injected_context.is_empty() {
                            if !message.is_empty() {
                                app.conversation.push_message(Role::User, message.clone());
                                // Auto title naming: set title based on first user message
                                if app.conversation.title == "New Chat" {
                                    let words: Vec<&str> = message.split_whitespace().take(4).collect();
                                    let mut title = words.join(" ");
                                    if title.len() > 25 {
                                        title = crate::logger::safe_truncate(&title, 25).to_string();
                                    }
                                    app.conversation.title = title.clone();
                                    app.conversation.push_message(Role::System, format!("Session title set to: {}", title));
                                }
                            }
                            app.chat_scroll = 0;
                            app.loading = true;
                            app.request_started = Some(std::time::Instant::now());
                            app.pending_tools.clear();
                            app.last_duration = None;
                            let mercury = mercury.clone();
                            let honcho = honcho.clone();
                            let base_prompt = system_prompt.to_string();
                            let mut messages = app.conversation.messages.clone();
                            messages.push(app::Message { id: uuid::Uuid::new_v4().to_string(), role: app::Role::User, content: "Continue working on the remaining tasks. Use list_tasks to see what is left, then work on the next one.".to_string(), timestamp: chrono::Utc::now() });
                            let tx = event_tx.clone();
                            let workspace_clone = app.workspace.clone();
                            let tasks_arc = Arc::new(Mutex::new(app.tasks.clone()));
                            let cancel = tokio_util::sync::CancellationToken::new();
                            app.cancel_token = Some(cancel.clone());
                            let task_context = app.tasks_as_context().unwrap_or_default();
                            let tasks_for_sync = tasks_arc.clone();
                            tokio::spawn(async move {
                                let mut enriched_prompt = { let h = honcho.lock().await; h.enrich_system_prompt(&base_prompt, "continue tasks").await };
                                if !task_context.is_empty() { enriched_prompt = format!("{enriched_prompt}\n\n{task_context}"); }
                                let tool_ctx = tools::registry::ToolContext { workspace: workspace_clone, tasks: tasks_for_sync, honcho: honcho.clone() };
                                mercury.chat(Some(&enriched_prompt), &messages, tool_ctx, tx, cancel).await;
                            });
                        }
                    }
                }
            }
        }
        KeyCode::Char(c) => { app.input.insert(app.cursor_pos, c); app.cursor_pos += 1; }
        KeyCode::Backspace => { if app.cursor_pos > 0 { app.cursor_pos -= 1; app.input.remove(app.cursor_pos); } }
        KeyCode::Delete => { if app.cursor_pos < app.input.len() { app.input.remove(app.cursor_pos); } }
        KeyCode::Left => { if app.cursor_pos > 0 { app.cursor_pos -= 1; } }
        KeyCode::Right => { if app.cursor_pos < app.input.len() { app.cursor_pos += 1; } }
        KeyCode::Home => app.cursor_pos = 0,
        KeyCode::End => app.cursor_pos = app.input.len(),
        KeyCode::Esc => app.focus = FocusPanel::Chat,
        KeyCode::Up => app.chat_scroll = app.chat_scroll.saturating_add(1),
        KeyCode::Down => app.chat_scroll = app.chat_scroll.saturating_sub(1),
        _ => {}
    }
}

fn handle_chat_key(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Char('i') | KeyCode::Enter => app.focus = FocusPanel::Input,
        KeyCode::Up | KeyCode::Char('k') => app.chat_scroll = app.chat_scroll.saturating_add(1),
        KeyCode::Down | KeyCode::Char('j') => app.chat_scroll = app.chat_scroll.saturating_sub(1),
        _ => {}
    }
}

fn handle_mouse(app: &mut App, mouse: crossterm::event::MouseEvent) {
    let (term_width, term_height) = crossterm::terminal::size().unwrap_or((80, 40));
    let panel_start = if app.show_diff_panel { (term_width as f32 * 0.55) as u16 } else { term_width };
    let in_side_panel = mouse.column >= panel_start;

    match mouse.kind {
        MouseEventKind::ScrollUp => {
            if in_side_panel {
                // Scroll the active side tab
                match app.side_tab {
                    app::SideTab::Diff => app.diff_scroll = app.diff_scroll.saturating_add(3),
                    app::SideTab::Log => app.log_scroll = app.log_scroll.saturating_add(3),
                    app::SideTab::Tasks => app.tasks_scroll = app.tasks_scroll.saturating_add(3),
                }
            } else {
                app.chat_scroll = app.chat_scroll.saturating_add(3);
            }
        }
        MouseEventKind::ScrollDown => {
            if in_side_panel {
                match app.side_tab {
                    app::SideTab::Diff => app.diff_scroll = app.diff_scroll.saturating_sub(3),
                    app::SideTab::Log => app.log_scroll = app.log_scroll.saturating_sub(3),
                    app::SideTab::Tasks => app.tasks_scroll = app.tasks_scroll.saturating_sub(3),
                }
            } else {
                app.chat_scroll = app.chat_scroll.saturating_sub(3);
            }
        }
        MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
            if mouse.row >= term_height.saturating_sub(4) {
                app.focus = FocusPanel::Input;
            } else if in_side_panel && mouse.row < 2 {
                // Click on tab bar — detect which tab
                let rel_x = mouse.column.saturating_sub(panel_start);
                if rel_x < 7 {
                    app.side_tab = app::SideTab::Diff;
                } else if rel_x < 14 {
                    app.side_tab = app::SideTab::Log;
                } else {
                    app.side_tab = app::SideTab::Tasks;
                }
            } else {
                app.focus = FocusPanel::Chat;
            }
        }
        _ => {}
    }
}

fn summarize_tool_args(name: &str, args_json: &str) -> String {
    let args: serde_json::Value = serde_json::from_str(args_json).unwrap_or_default();
    match name {
        "read_file" | "list_dir" => args["path"].as_str().unwrap_or("?").to_string(),
        "write_file" => {
            let path = args["path"].as_str().unwrap_or("?");
            let len = args["content"].as_str().map(|c| c.len()).unwrap_or(0);
            format!("{path}, {len} bytes")
        }
        "glob_search" => args["pattern"].as_str().unwrap_or("?").to_string(),
        "grep_search" => {
            let p = args["pattern"].as_str().unwrap_or("?");
            let g = args["file_glob"].as_str().unwrap_or("**/*");
            format!("\"{p}\" in {g}")
        }
        "run_command" => {
            let cmd = args["command"].as_str().unwrap_or("?");
            if cmd.len() > 60 { format!("{}...", crate::logger::safe_truncate(cmd, 60)) } else { cmd.to_string() }
        }
        _ => {
            if args_json.len() > 40 { format!("{}...", crate::logger::safe_truncate(args_json, 40)) } else { args_json.to_string() }
        }
    }
}
