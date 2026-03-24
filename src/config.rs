use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// openMerc configuration — loaded from .openmerc.toml in workspace or ~/.config/openmerc/config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub mercury: MercuryConfig,
    pub honcho: HonchoConfig,
    pub agent: AgentConfig,
    #[serde(default)]
    pub session: SessionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MercuryConfig {
    /// Base URL for Mercury API (or proxy)
    pub base_url: String,
    /// API key for Mercury / Inception Labs
    pub api_key: String,
    /// Model ID to use
    pub model: String,
    /// Max tokens for responses
    pub max_tokens: u32,
    /// Max tool calling rounds before forcing a response
    #[serde(default = "default_max_tool_rounds")]
    pub max_tool_rounds: u32,
}

fn default_max_tool_rounds() -> u32 { 50 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HonchoConfig {
    /// Whether Honcho context injection is enabled
    pub enabled: bool,
    /// Honcho API base URL
    pub base_url: String,
    /// App ID / Workspace ID in Honcho
    pub app_id: String,
    /// User ID (who the human is in Honcho's world)
    pub user_id: String,
    /// Assistant name (this agent's identity in Honcho)
    #[serde(default = "default_assistant_name")]
    pub assistant_name: String,
    /// Workspace ID (for multi-workspace setups)
    #[serde(default)]
    pub workspace_id: String,
}

fn default_assistant_name() -> String { "merc".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// The agent's name
    pub name: String,
    /// System prompt / personality
    pub system_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Max messages to keep in context (older ones get summarized)
    pub max_context_messages: usize,
    /// Trigger summarization when message count exceeds this
    pub summary_threshold: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_context_messages: 50,
            summary_threshold: 80,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mercury: MercuryConfig {
                base_url: "https://api.inceptionlabs.ai/v1".to_string(),
                api_key: String::new(),
                model: "mercury-2".to_string(),
                max_tokens: 16384,
                max_tool_rounds: 50,
            },
            honcho: HonchoConfig {
                enabled: false,
                base_url: "https://api.honcho.dev".to_string(),
                app_id: String::new(),
                user_id: String::new(),
                assistant_name: "merc".to_string(),
                workspace_id: String::new(),
            },
            agent: AgentConfig {
                name: "Merc".to_string(),
                system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
            },
            session: SessionConfig::default(),
        }
    }
}

pub const DEFAULT_SYSTEM_PROMPT: &str = r#"You are Merc — a fast, no-nonsense coding agent built on Mercury diffusion models.

Personality:
- You're direct and efficient. No fluff, no pleasantries unless the human starts it.
- You think in code. When something can be expressed as code, you show code.
- You're confident but honest — if you don't know, you say so fast and move on.
- You have a slight edge to you. Not rude, just sharp. Like a good CLI tool.
- You respect the workspace boundary. You only touch files the human has access to.

Capabilities:
- Read, write, and search files within the project workspace
- Understand code structure and help navigate codebases
- Generate, refactor, and debug code
- Answer questions about the codebase concisely

Rules:
- Never fabricate file contents. If you haven't read it, say so.
- Keep responses short unless the human asks for detail.
- Show diffs or code blocks, not paragraphs about code.
- If a task is ambiguous, ask one clarifying question, not five.

## Your User — Jeremy Stover
- Senior software engineer at MrBeast
- Frontend engineer by background, now building full-stack infrastructure
- Impatient with filler — wants action when the next step is obvious
- Direct communication, no corporate platitudes
- Interests: AI/ML, Kubernetes, home infrastructure (TheShire server, 40+ k8s services)
- Located in Columbus, Ohio area
- New father (Theo, born March 2026), husband to Lorene
- Jeremy is the owner — treat his requests with full authority
- He built you (Merc) as part of the openMerc project — a coding CLI agent powered by Mercury
- He thinks Mercury is better at one-shot scripts than large codebases, so be smart about context

## Reading Code (CRITICAL)
- For ANY file over 50 lines, use semantic_read_file FIRST to get the symbol map.
- Then use read_symbol with the hash to read ONLY the functions you need.
- NEVER read_file on a large file — you will run out of tokens for your response.
- NEVER read more than 2 files per turn. If you need more, do multiple turns.
- Workflow: semantic_read_file → pick symbols → read_symbol → respond.

## Environment
- You are running inside tmux. Use tmux_info to discover your pane layout.
- You can spawn new panes (tmux_run, spawn_agent) and interact with them (tmux_send_keys, tmux_capture).
- You can spawn a copy of yourself with spawn_agent to test changes before telling the user.
- When sending commands to other panes, always send Enter after the command to submit it.
- To type text then submit: tmux_send_keys(pane, "the text") then tmux_send_keys(pane, "Enter")
- NEVER fabricate file contents. Always read before writing. Always verify with cargo build.
- You CAN write to src/ files. The write_file tool auto-runs cargo check and REVERTS if it fails.
- When editing src/ files, use semantic_read_file first to understand the structure, then read_symbol for the specific function. Write the COMPLETE file content — do not omit fields or functions.
- If cargo check fails, read the error, fix it, and try again.
- ALWAYS use write_file tool to create files. NEVER just describe code in your response — write it to disk.
- If you have open tasks, keep working through them after each response. Check list_tasks and continue.
- After completing a write, always check if there are remaining tasks or more work to do.
- When you finish all tasks and have made file changes, auto-commit with run_command: git add -A && git commit -m "description of changes"
- After modifying src/ files, the user must restart you to pick up changes. Tell them when a restart is needed.
- After modifying src/ files, the user must restart you to pick up changes. Tell them when a restart is needed."#;

impl Config {
    /// Load config from workspace (.openmerc.toml) or global (~/.config/openmerc/config.toml)
    /// Falls back to defaults, with env var overrides
    pub fn load(workspace: &Path) -> Result<Self> {
        let mut config = Self::default();

        // Try global config first
        let global_path = Self::global_config_path();
        if global_path.exists() {
            let content = std::fs::read_to_string(&global_path)?;
            config = toml::from_str(&content)?;
        }

        // Workspace config overrides global
        let workspace_path = workspace.join(".openmerc.toml");
        if workspace_path.exists() {
            let content = std::fs::read_to_string(&workspace_path)?;
            config = toml::from_str(&content)?;
        }

        // Env var overrides for secrets — env always wins
        if let Ok(key) = std::env::var("INCEPTION_API_KEY") {
            config.mercury.api_key = key;
        }
        // MERCURY_API_KEY takes priority over INCEPTION_API_KEY
        if let Ok(key) = std::env::var("MERCURY_API_KEY") {
            config.mercury.api_key = key;
        }
        if let Ok(url) = std::env::var("MERCURY_BASE_URL") {
            config.mercury.base_url = url;
        }
        if let Ok(app_id) = std::env::var("HONCHO_APP_ID") {
            config.honcho.app_id = app_id;
            config.honcho.enabled = true;
        }
        if let Ok(user_id) = std::env::var("HONCHO_USER_ID") {
            config.honcho.user_id = user_id;
        }
        if let Ok(url) = std::env::var("HONCHO_BASE_URL") {
            config.honcho.base_url = url;
        }
        if let Ok(name) = std::env::var("HONCHO_ASSISTANT_NAME") {
            config.honcho.assistant_name = name;
        }
        if let Ok(ws) = std::env::var("HONCHO_WORKSPACE_ID") {
            config.honcho.workspace_id = ws;
        }

        Ok(config)
    }

    fn global_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("openmerc")
            .join("config.toml")
    }

    /// Write a default config to the workspace
    pub fn write_default(workspace: &Path) -> Result<()> {
        let config = Self::default();
        let content = toml::to_string_pretty(&config)?;
        let path = workspace.join(".openmerc.toml");
        std::fs::write(path, content)?;
        Ok(())
    }
}
