use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::app::Task;
use crate::context::honcho::HonchoContext;
use super::files;
use super::search;
use super::tasks;

/// Context passed to tool execution (workspace + shared state)
pub struct ToolContext {
    pub workspace: PathBuf,
    pub tasks: Arc<Mutex<Vec<Task>>>,
    pub honcho: Arc<Mutex<HonchoContext>>,
}

/// Tool definition sent to Mercury API
#[derive(Debug, Clone, Serialize)]
pub struct ToolDef {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDef,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionDef {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// A tool call returned by Mercury
#[derive(Debug, Clone, Deserialize)]
pub struct ToolCall {
    pub id: Option<String>,
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}

/// Build the list of tools to send to Mercury
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "read_file".to_string(),
                description: "Read the contents of a file in the workspace. Returns the file content as a string.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path to the file within the workspace"
                        }
                    },
                    "required": ["path"]
                }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "write_file".to_string(),
                description: "Write content to a file in the workspace. Creates parent directories if needed.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path to the file within the workspace"
                        },
                        "content": {
                            "type": "string",
                            "description": "The full content to write to the file"
                        }
                    },
                    "required": ["path", "content"]
                }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "list_dir".to_string(),
                description: "List files and directories at a path in the workspace. Directories have a trailing slash.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path to the directory (use '.' for workspace root)"
                        }
                    },
                    "required": ["path"]
                }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "glob_search".to_string(),
                description: "Find files matching a glob pattern in the workspace. Example: '**/*.rs' finds all Rust files.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Glob pattern to match files (e.g., '**/*.rs', 'src/**/*.toml')"
                        }
                    },
                    "required": ["pattern"]
                }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "grep_search".to_string(),
                description: "Search for a text pattern in files matching a glob. Returns matching lines with file path and line number.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Text pattern to search for in file contents"
                        },
                        "file_glob": {
                            "type": "string",
                            "description": "Glob pattern for which files to search (e.g., '**/*.rs')"
                        }
                    },
                    "required": ["pattern", "file_glob"]
                }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "run_command".to_string(),
                description: "Run a shell command in the workspace directory. Use for build, test, git, etc. Returns stdout and stderr.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The shell command to execute"
                        }
                    },
                    "required": ["command"]
                }),
            },
        },
        // --- Task management tools ---
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "create_task".to_string(),
                description: "Create a task to track work. Use this to break down multi-step changes into trackable items.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "title": {
                            "type": "string",
                            "description": "Short title for the task"
                        },
                        "description": {
                            "type": "string",
                            "description": "Optional detailed description"
                        }
                    },
                    "required": ["title"]
                }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "update_task".to_string(),
                description: "Update a task's status, title, or description. Use to mark progress.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Task ID (short hash)"
                        },
                        "status": {
                            "type": "string",
                            "enum": ["pending", "in_progress", "completed", "blocked"],
                            "description": "New status"
                        },
                        "title": {
                            "type": "string",
                            "description": "New title (optional)"
                        },
                        "description": {
                            "type": "string",
                            "description": "New description (optional)"
                        }
                    },
                    "required": ["id"]
                }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "list_tasks".to_string(),
                description: "List all tasks and their statuses for the current session.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
        // --- Memory search tool ---
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "search_memory".to_string(),
                description: "Search across all Honcho memory spaces for information about the user, project, past conversations, decisions, and context. Use this when you need to recall something from past sessions or find information the user has shared before.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "What to search for in memory (e.g., 'semfora integration', 'deployment setup', 'user preferences')"
                        }
                    },
                    "required": ["query"]
                }),
            },
        },
        // --- Semfora integration tools ---
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "semfora_analyze".to_string(),
                description: "Run `semfora-engine analyze` on a given path and return the JSON/TOON output. Useful for getting a semantic overview of code.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path to the file or directory to analyze"
                        }
                    },
                    "required": ["path"]
                }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "semfora_search".to_string(),
                description: "Run `semfora-engine search` for a query string and return the results in JSON format.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query to pass to semfora"
                        }
                    },
                    "required": ["query"]
                }),
            },
        },
        // --- Semantic code reading (semfora-powered) ---
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "semantic_read_file".to_string(),
                description: "Get a semantic overview of a file using Semfora: lists all functions/symbols with their line ranges, complexity, and risk. Use this FIRST before read_file on any file >50 lines. Then use read_symbol to read specific functions.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Relative path to the file"
                        }
                    },
                    "required": ["path"]
                }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "read_symbol".to_string(),
                description: "Read the source code of a specific function/symbol by its semfora hash, or read a line range from a file. Use after semantic_read_file to drill into specific functions without reading the whole file.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "hash": {
                            "type": "string",
                            "description": "Semfora symbol hash (from semantic_read_file output). E.g. 'e45e6708:26df3fcae0ff7978'"
                        },
                        "file": {
                            "type": "string",
                            "description": "File path (alternative to hash — use with start/end lines)"
                        },
                        "start_line": {
                            "type": "integer",
                            "description": "Start line number (1-indexed, use with file)"
                        },
                        "end_line": {
                            "type": "integer",
                            "description": "End line number (inclusive, use with file)"
                        }
                    },
                    "required": []
                }),
            },
        },
        // --- Tmux environment tools ---
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "tmux_info".to_string(),
                description: "Get current tmux environment info: session, window, pane IDs. You ARE running inside tmux.".to_string(),
                parameters: json!({ "type": "object", "properties": {}, "required": [] }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "tmux_run".to_string(),
                description: "Run a shell command in a tmux pane. If pane_id is empty, creates a new split pane. Returns the output.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "pane_id": { "type": "string", "description": "Target pane ID (e.g. %1). Empty string to create new pane." },
                        "command": { "type": "string", "description": "Shell command to run" }
                    },
                    "required": ["command"]
                }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "tmux_capture".to_string(),
                description: "Capture the last N lines of output from a tmux pane. Use to read what another process is displaying.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "pane_id": { "type": "string", "description": "Pane ID to capture from (e.g. %1)" },
                        "lines": { "type": "integer", "description": "Number of lines to capture (default 20)" }
                    },
                    "required": ["pane_id"]
                }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "tmux_send_keys".to_string(),
                description: "Send keystrokes to a tmux pane. For text, it types character by character. Supports special keys: Enter, Escape, C-c, Up, Down. Use to interact with TUI apps in other panes.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "pane_id": { "type": "string", "description": "Target pane ID" },
                        "keys": { "type": "string", "description": "Keys to send. Text is typed literally. Special: Enter, Escape, C-c, Up, Down" }
                    },
                    "required": ["pane_id", "keys"]
                }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "spawn_agent".to_string(),
                description: "Build and spawn a new openMerc instance in a split tmux pane. Use to test changes: build, spawn, interact via tmux_send_keys, capture output, then kill with tmux_kill_pane. Returns the new pane ID.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "extra_env": { "type": "string", "description": "Extra environment variables (e.g. INCEPTION_API_KEY=xxx). Empty for defaults." }
                    },
                    "required": []
                }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "tmux_kill_pane".to_string(),
                description: "Kill a tmux pane by ID. Use to clean up after testing.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "pane_id": { "type": "string", "description": "Pane ID to kill" }
                    },
                    "required": ["pane_id"]
                }),
            },
        },
        // --- Mercury Edit tools ---
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "apply_edit".to_string(),
                description: "Surgically merge a code change using Mercury Edit. Takes original code and an update snippet with '// ... existing code ...' markers. Returns the merged result. Much more efficient than rewriting entire files.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "original_code": { "type": "string", "description": "The original source code" },
                        "update_snippet": { "type": "string", "description": "The update with // ... existing code ... markers" }
                    },
                    "required": ["original_code", "update_snippet"]
                }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "fim_complete".to_string(),
                description: "Fill-in-the-middle autocomplete using Mercury Edit. Given a prefix and suffix, generates the code that goes between them.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "prompt": { "type": "string", "description": "Code before the cursor (prefix)" },
                        "suffix": { "type": "string", "description": "Code after the cursor (suffix)" }
                    },
                    "required": ["prompt", "suffix"]
                }),
            },
        },
    ]
}

/// Execute a tool call and return the result as a string
pub async fn execute_tool(ctx: &ToolContext, tool_call: &ToolCall) -> String {
    let workspace = &ctx.workspace;
    let args: Value = serde_json::from_str(&tool_call.function.arguments)
        .unwrap_or_else(|e| json!({"error": format!("Failed to parse arguments: {e}")}));

    match tool_call.function.name.as_str() {
        "read_file" => {
            let path = args["path"].as_str().unwrap_or("");
            match files::read_file(workspace, path) {
                Ok(content) => {
                    // For large files, auto-redirect to semantic summary + hint
                    if content.lines().count() > 100 {
                        let preview: String = content.lines().take(30).collect::<Vec<_>>().join("\n");
                        format!("(File is {} lines — showing first 30. Use semantic_read_file for the symbol map, then read_symbol for specific functions.)\n\n{preview}\n...", content.lines().count())
                    } else {
                        content
                    }
                }
                Err(e) => format!("Error reading file: {e}"),
            }
        }
        "write_file" => {
            let path = args["path"].as_str().unwrap_or("");
            let content = args["content"].as_str().unwrap_or("");
            match files::write_file(workspace, path, content) {
                Ok(diff) => diff,
                Err(e) => format!("Error writing file: {e}"),
            }
        }
        "list_dir" => {
            let path = args["path"].as_str().unwrap_or(".");
            match files::list_dir(workspace, path) {
                Ok(entries) => entries.join("\n"),
                Err(e) => format!("Error listing directory: {e}"),
            }
        }
        "glob_search" => {
            let pattern = args["pattern"].as_str().unwrap_or("");
            match search::glob_search(workspace, pattern) {
                Ok(results) => {
                    if results.is_empty() {
                        "No files found matching pattern".to_string()
                    } else {
                        results.join("\n")
                    }
                }
                Err(e) => format!("Error searching: {e}"),
            }
        }
        "grep_search" => {
            let pattern = args["pattern"].as_str().unwrap_or("");
            let file_glob = args["file_glob"].as_str().unwrap_or("**/*");
            match search::grep_search(workspace, pattern, file_glob) {
                Ok(matches) => {
                    if matches.is_empty() {
                        "No matches found".to_string()
                    } else {
                        matches.iter().map(|m| m.to_string()).collect::<Vec<_>>().join("\n")
                    }
                }
                Err(e) => format!("Error searching: {e}"),
            }
        }
        "run_command" => {
            let command = args["command"].as_str().unwrap_or("");
            execute_command(workspace, command)
        }
        "create_task" => {
            let title = args["title"].as_str().unwrap_or("Untitled");
            let description = args["description"].as_str();
            let mut task_list = ctx.tasks.lock().await;
            tasks::create_task(&mut task_list, title, description)
        }
        "update_task" => {
            let id = args["id"].as_str().unwrap_or("");
            let status = args["status"].as_str();
            let title = args["title"].as_str();
            let description = args["description"].as_str();
            let mut task_list = ctx.tasks.lock().await;
            tasks::update_task(&mut task_list, id, status, title, description)
        }
        "list_tasks" => {
            let task_list = ctx.tasks.lock().await;
            tasks::list_tasks(&task_list)
        }
        "search_memory" => {
            let query = args["query"].as_str().unwrap_or("");
            let mut honcho = ctx.honcho.lock().await;
            match honcho.search_workspace(query).await {
                Some(results) => results,
                None => "No memories found. Honcho memory search returned empty — try a different query or the information may not exist in memory.".to_string(),
            }
        }
        "semfora_analyze" => {
            let path = args["path"].as_str().unwrap_or("");
            // Run semfora-engine analyze and capture output
            let mut cmd = std::process::Command::new("semfora-engine");
            cmd.arg("analyze").arg(path).current_dir(workspace);
            match cmd.output() {
                Ok(output) => {
                    if output.status.success() {
                        String::from_utf8_lossy(&output.stdout).to_string()
                    } else {
                        let err = String::from_utf8_lossy(&output.stderr);
                        format!("semfora analyze failed: {}", err)
                    }
                }
                Err(e) => format!("Failed to execute semfora-engine analyze: {}", e),
            }
        }
        "semfora_search" => {
            let query = args["query"].as_str().unwrap_or("");
            let mut cmd = std::process::Command::new("semfora-engine");
            cmd.arg("search").arg(query).current_dir(workspace);
            match cmd.output() {
                Ok(output) => {
                    if output.status.success() {
                        String::from_utf8_lossy(&output.stdout).to_string()
                    } else {
                        let err = String::from_utf8_lossy(&output.stderr);
                        format!("semfora search failed: {}", err)
                    }
                }
                Err(e) => format!("Failed to execute semfora-engine search: {}", e),
            }
        }
        // --- Semantic file reading (semfora-powered, chunk-based) ---
        "semantic_read_file" => {
            let path = args["path"].as_str().unwrap_or("");
            let mut result = String::new();

            // Step 1: Get file analysis summary
            let analyze = std::process::Command::new("semfora-engine")
                .args(["analyze", path])
                .current_dir(workspace)
                .output();
            if let Ok(o) = &analyze {
                if o.status.success() {
                    let raw = String::from_utf8_lossy(&o.stdout);
                    for line in raw.lines() {
                        if line.contains("cognitive_complexity") || line.contains("behavioral_risk")
                            || line.contains("max_nesting_depth") || line.contains("total_lines")
                            || line.contains("purpose") {
                            result.push_str(line.trim());
                            result.push('\n');
                        }
                    }
                }
            }

            // Step 2: Get symbol map with line ranges
            let symbols = std::process::Command::new("semfora-engine")
                .args(["query", "file", path, "--format", "json"])
                .current_dir(workspace)
                .output();
            if let Ok(o) = &symbols {
                if o.status.success() {
                    let raw = String::from_utf8_lossy(&o.stdout);
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                        if let Some(syms) = v["symbols"].as_array() {
                            result.push_str("\nSymbols (use read_symbol with hash to read source):\n");
                            for s in syms {
                                let name = s["name"].as_str().unwrap_or("?");
                                let hash = s["hash"].as_str().unwrap_or("?");
                                let kind = s["kind"].as_str().unwrap_or("?");
                                let lines = s["lines"].as_str().unwrap_or("?");
                                let risk = s["risk"].as_str().unwrap_or("?");
                                result.push_str(&format!("  {kind} {name} [{lines}] risk={risk} hash={hash}\n"));
                            }
                        }
                    }
                }
            }

            if result.is_empty() {
                // Fallback: just read first 20 lines
                match files::read_file(workspace, path) {
                    Ok(content) => {
                        let preview: String = content.lines().take(20).collect::<Vec<_>>().join("\n");
                        format!("(semfora unavailable, showing first 20 lines)\n{preview}")
                    }
                    Err(e) => format!("Error: {e}"),
                }
            } else {
                result
            }
        }
        "read_symbol" => {
            let hash = args["hash"].as_str().unwrap_or("");
            let file = args["file"].as_str().unwrap_or("");
            let start = args["start_line"].as_u64().unwrap_or(0);
            let end = args["end_line"].as_u64().unwrap_or(0);

            if !hash.is_empty() {
                // Read by symbol hash
                let output = std::process::Command::new("semfora-engine")
                    .args(["query", "source", "--hash", hash])
                    .current_dir(workspace)
                    .output();
                match output {
                    Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
                    Ok(o) => format!("semfora source query failed: {}", String::from_utf8_lossy(&o.stderr)),
                    Err(e) => format!("Failed to run semfora: {e}"),
                }
            } else if !file.is_empty() && start > 0 && end > 0 {
                // Read by file + line range
                let output = std::process::Command::new("semfora-engine")
                    .args(["query", "source", file, "--start", &start.to_string(), "--end", &end.to_string()])
                    .current_dir(workspace)
                    .output();
                match output {
                    Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
                    Ok(o) => format!("semfora source query failed: {}", String::from_utf8_lossy(&o.stderr)),
                    Err(e) => format!("Failed to run semfora: {e}"),
                }
            } else {
                "Provide either hash (from semantic_read_file) or file + start_line + end_line.".to_string()
            }
        }
        // --- Tmux tools ---
        "tmux_info" => {
            super::tmux::tmux_info()
        }
        "tmux_run" => {
            let pane_id = args["pane_id"].as_str().unwrap_or("");
            let command = args["command"].as_str().unwrap_or("");
            super::tmux::tmux_run(pane_id, command)
        }
        "tmux_capture" => {
            let pane_id = args["pane_id"].as_str().unwrap_or("");
            let lines = args["lines"].as_u64().unwrap_or(20) as u32;
            super::tmux::tmux_capture(pane_id, lines)
        }
        "tmux_send_keys" => {
            let pane_id = args["pane_id"].as_str().unwrap_or("");
            let keys = args["keys"].as_str().unwrap_or("");
            super::tmux::tmux_send_keys(pane_id, keys)
        }
        "spawn_agent" => {
            let extra_env = args["extra_env"].as_str().unwrap_or("");
            super::tmux::spawn_agent(workspace, extra_env)
        }
        "tmux_kill_pane" => {
            let pane_id = args["pane_id"].as_str().unwrap_or("");
            super::tmux::tmux_kill_pane(pane_id)
        }
        // --- Mercury Edit tools ---
        "apply_edit" => {
            let original = args["original_code"].as_str().unwrap_or("");
            let snippet = args["update_snippet"].as_str().unwrap_or("");
            let config = crate::config::Config::load(workspace).unwrap_or_default();
            match crate::api::apply_edit::apply_edit(&config.mercury.base_url, &config.mercury.api_key, original, snippet).await {
                Ok(result) => result,
                Err(e) => format!("apply_edit failed: {e}"),
            }
        }
        "fim_complete" => {
            let prompt = args["prompt"].as_str().unwrap_or("");
            let suffix = args["suffix"].as_str().unwrap_or("");
            let config = crate::config::Config::load(workspace).unwrap_or_default();
            match crate::api::fim::fim(&config.mercury.base_url, &config.mercury.api_key, prompt, suffix).await {
                Ok(result) => result,
                Err(e) => format!("fim_complete failed: {e}"),
            }
        }
        other => format!("Unknown tool: {other}"),
    }
}

fn execute_command(workspace: &Path, command: &str) -> String {
    use std::process::Command;

    match Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(workspace)
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut result = String::new();
            if !stdout.is_empty() {
                result.push_str(&stdout);
            }
            if !stderr.is_empty() {
                if !result.is_empty() {
                    result.push_str("\n--- stderr ---\n");
                }
                result.push_str(&stderr);
            }
            if result.is_empty() {
                format!("Command completed (exit code: {})", output.status.code().unwrap_or(-1))
            } else {
                // Truncate if too long to avoid blowing context
                if result.len() > 8000 {
                    result.truncate(8000);
                    result.push_str("\n... (truncated)");
                }
                result
            }
        }
        Err(e) => format!("Failed to execute command: {e}"),
    }
}
