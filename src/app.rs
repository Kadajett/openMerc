// Updated App struct to include focus_dir for /focus command
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: Role,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::Tool => write!(f, "tool"),
        }
    }
}

/// A task tracked within a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub description: Option<String>,
    #[serde(default = "default_priority")]
    pub priority: u8,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub estimated_rounds: Option<u16>,
    #[serde(default)]
    pub actual_rounds: u16,
    #[serde(default)]
    pub notes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
}

fn default_priority() -> u8 { 3 }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Blocked,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::InProgress => write!(f, "in_progress"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Blocked => write!(f, "blocked"),
        }
    }
}

/// A conversation (chat session)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub messages: Vec<Message>,
    pub created_at: DateTime<Utc>,
    pub honcho_session_id: Option<String>,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title: "New Chat".to_string(),
            messages: Vec::new(),
            created_at: Utc::now(),
            honcho_session_id: None,
        }
    }

    pub fn push_message(&mut self, role: Role, content: String) {
        self.messages.push(Message {
            id: Uuid::new_v4().to_string(),
            role,
            content,
            timestamp: Utc::now(),
        });
    }
}

/// Which panel is focused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPanel {
    Chat,
    Input,
    Files,
}

/// Application mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    /// Normal chat mode
    Normal,
    /// Session picker on launch
    SessionPicker,
    /// Plan mode – executing a plan
    Plan,
    /// Paused mode – plan execution paused
    Paused,
}

/// Agent progress during multi-step work
#[derive(Debug, Clone)]
pub struct AgentProgressInfo {
    pub round: u32,
    pub max_rounds: u32,
    pub current_action: String,
}

/// Top-level application state
pub struct App {
    /// The workspace root — all file ops sandboxed here
    pub workspace: PathBuf,

    /// Current session ID (persisted)
    pub session_id: String,

    /// Honcho session ID (for resume)
    pub honcho_session_id: Option<String>,

    /// Active conversation
    pub conversation: Conversation,

    /// Task list for current session
    pub tasks: Vec<Task>,

    /// Current app mode
    pub app_mode: AppMode,

    /// Session picker state
    pub picker_selected: usize,

    /// Current input buffer
    pub input: String,

    /// Cursor position in input
    pub cursor_pos: usize,

    /// Which panel has focus
    pub focus: FocusPanel,

    /// Focus directory for quick navigation (optional)
    pub focus_dir: Option<PathBuf>,

    /// Chat scroll offset (from bottom)
    pub chat_scroll: u16,

    /// Whether the app should quit
    pub should_quit: bool,

    /// Whether we're waiting for an API response
    pub loading: bool,

    /// Streaming response buffer (partial assistant message)
    pub stream_buffer: String,

    /// Agent progress during multi-step operations
    pub agent_progress: Option<AgentProgressInfo>,

    /// Accumulated tool calls during current request (for grouped display)
    pub pending_tools: Vec<ToolLogEntry>,

    /// When the current request started (for duration display)
    pub request_started: Option<std::time::Instant>,

    /// Duration of the last completed request
    pub last_duration: Option<std::time::Duration>,

    /// Cancellation token for the current in-flight request
    pub cancel_token: Option<tokio_util::sync::CancellationToken>,

    /// Active execution plan (if in plan mode)
    pub active_plan: Option<crate::plan::Plan>,

    /// Files modified this session (path → diff string)
    pub modified_files: Vec<FileDiff>,

    /// Which modified file is selected in the diff panel
    pub diff_selected: usize,

    /// Whether the side panel is visible (always open by default)
    pub show_diff_panel: bool,

    /// Which tab is active in the side panel
    pub side_tab: SideTab,

    /// Per-tab scroll offsets
    pub diff_scroll: u16,
    pub log_scroll: u16,
    pub tasks_scroll: u16,

    /// Running log of change summaries (from Honcho or auto-generated)
    pub change_log: Vec<ChangeLogEntry>,
}

/// Which tab is active in the side panel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideTab {
    Diff,
    Log,
    Tasks,
}

/// A summary entry in the change log
#[derive(Debug, Clone)]
pub struct ChangeLogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub summary: String,
}

/// A tracked file modification
#[derive(Debug, Clone)]
pub struct FileDiff {
    pub path: String,
    pub diff: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// A single tool call log entry
#[derive(Debug, Clone)]
pub struct ToolLogEntry {
    pub name: String,
    pub args_summary: String,
    pub result: Option<String>,
    /// Whether this is a "visible" tool (read/write) vs a "thinking" tool
    pub visible: bool,
}

impl App {
    pub fn new(workspace: PathBuf) -> Self {
        let conversation = Conversation::new();
        let session_id = conversation.id.clone();
        Self {
            workspace,
            session_id,
            honcho_session_id: None,
            conversation,
            tasks: Vec::new(),
            app_mode: AppMode::Normal,
            picker_selected: 0,
            input: String::new(),
            cursor_pos: 0,
            focus: FocusPanel::Input,
            focus_dir: None,
            chat_scroll: 0,
            should_quit: false,
            loading: false,
            stream_buffer: String::new(),
            agent_progress: None,
            pending_tools: Vec::new(),
            request_started: None,
            last_duration: None,
            cancel_token: None,
            active_plan: None,
            modified_files: Vec::new(),
            diff_selected: 0,
            show_diff_panel: true,
            side_tab: SideTab::Diff,
            diff_scroll: 0,
            log_scroll: 0,
            tasks_scroll: 0,
            change_log: Vec::new(),
        }
    }

    /// Submit the current input as a user message
    pub fn submit_input(&mut self) -> Option<String> {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return None;
        }
        self.conversation.push_message(Role::User, text.clone());
        self.input.clear();
        self.cursor_pos = 0;
        self.chat_scroll = 0;
        Some(text)
    }

    /// Append streamed text to the buffer
    pub fn append_stream(&mut self, chunk: &str) {
        self.stream_buffer.push_str(chunk);
    }

    /// Finalize the streamed response into a message
    pub fn finalize_stream(&mut self) {
        // Flush pending tools into conversation
        self.flush_tool_log();

        if !self.stream_buffer.is_empty() {
            let content = std::mem::take(&mut self.stream_buffer);
            self.conversation.push_message(Role::Assistant, content);
        }

        // Compute duration
        if let Some(started) = self.request_started.take() {
            self.last_duration = Some(started.elapsed());
        }

        self.loading = false;
        self.agent_progress = None;
    }

    /// Flush accumulated tool calls into the conversation as grouped messages
    pub fn flush_tool_log(&mut self) {
        if self.pending_tools.is_empty() {
            return;
        }

        let tools = std::mem::take(&mut self.pending_tools);

        // Separate visible (read/write) from thinking tools
        let mut thinking: Vec<&ToolLogEntry> = Vec::new();
        let mut visible: Vec<&ToolLogEntry> = Vec::new();

        for t in &tools {
            if t.visible {
                visible.push(t);
            } else {
                thinking.push(t);
            }
        }

        // If there are thinking tools, group them into one block
        if !thinking.is_empty() {
            let mut lines = Vec::new();
            for t in &thinking {
                let result_preview = t.result.as_deref()
                    .map(|r| if r.len() > 80 { format!("{}...", crate::logger::safe_truncate(r, 80)) } else { r.to_string() })
                    .unwrap_or_default();
                if result_preview.is_empty() {
                    lines.push(format!("  {} {}", t.name, t.args_summary));
                } else {
                    lines.push(format!("  {} {} → {}", t.name, t.args_summary, result_preview));
                }
            }
            self.conversation.push_message(
                Role::Tool,
                format!("thinking ({} calls)\n{}", thinking.len(), lines.join("\n")),
            );
        }

        // Show visible tools (read/write) individually
        for t in &visible {
            let result = t.result.as_deref().unwrap_or("");
            self.conversation.push_message(
                Role::Tool,
                format!("⚡ {}({})\n{}", t.name, t.args_summary, result),
            );
        }
    }

    /// Format tasks as a markdown checklist for system prompt injection
    pub fn tasks_as_context(&self) -> Option<String> {
        if self.tasks.is_empty() {
            return None;
        }
        let mut lines = vec!["## Current Tasks".to_string()];
        let done = self.tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();
        let total = self.tasks.len();
        lines.push(format!("Progress: {done}/{total} completed\n"));

        for task in &self.tasks {
            let icon = match task.status {
                TaskStatus::Completed => "[x]",
                TaskStatus::InProgress => "[~]",
                TaskStatus::Blocked => "[!]",
                TaskStatus::Pending => "[ ]",
            };
            let status_label = match task.status {
                TaskStatus::InProgress => " IN PROGRESS:",
                TaskStatus::Blocked => " BLOCKED:",
                _ => "",
            };
            let priority = format!("P{}", task.priority);
            let deps = if task.depends_on.is_empty() { String::new() } else {
                format!(" (depends: {})", task.depends_on.join(", "))
            };
            let desc = task.description.as_deref().unwrap_or("");
            let desc_part = if desc.is_empty() { String::new() } else { format!(" — {desc}") };
            let last_note = task.notes.last().map(|n| format!(" [note: {}]", crate::logger::safe_truncate(n, 60))).unwrap_or_default();
            lines.push(format!("- {icon} [{priority}]{status_label} {}{desc_part}{deps}{last_note}", task.title));
        }
        lines.push(String::new());
        lines.push("Use create_task, update_task, list_tasks, add_task_note tools to manage tasks.".to_string());
        Some(lines.join("\n"))
    }
}
