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
        // --- New semantic read tool (most impactful missing feature) ---
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "semantic_read_file".to_string(),
                description: "Read a file and return a concise semantic summary using Semfora. Includes cognitive complexity, risk, and key state changes. Helpful for quick code insight without dumping the whole file.".to_string(),
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
                Ok(content) => content,
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
        // --- New semantic read implementation ---
        "semantic_read_file" => {
            let path = args["path"].as_str().unwrap_or("");
            // Run semfora analyze on the file
            let mut cmd = std::process::Command::new("semfora-engine");
            cmd.arg("analyze").arg(path).current_dir(workspace);
            match cmd.output() {
                Ok(output) => {
                    if output.status.success() {
                        let raw = String::from_utf8_lossy(&output.stdout);
                        // Extract key fields for a concise summary
                        let mut summary = String::new();
                        for line in raw.lines() {
                            if line.contains("cognitive_complexity") || line.contains("behavioral_risk") || line.contains("max_nesting_depth") || line.contains("state_changes") {
                                summary.push_str(line);
                                summary.push('\n');
                            }
                        }
                        if summary.is_empty() {
                            // Fallback: first 10 lines
                            summary = raw.lines().take(10).collect::<Vec<_>>().join("\n");
                        }
                        summary
                    } else {
                        let err = String::from_utf8_lossy(&output.stderr);
                        format!("semfora analyze failed: {}", err)
                    }
                }
                Err(e) => format!("Failed to execute semfora-engine analyze: {}", e),
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
