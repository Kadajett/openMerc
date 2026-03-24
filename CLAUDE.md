# CLAUDE.md

## Project Overview
Rust ratatui TUI application (`openmerc`).
- Uses `ratatui` for UI, `crossterm` for terminal handling.
- Async runtime via `tokio`.
- Provides a set of tools (file I/O, shell commands, task management, memory search, Semfora integration) that the Mercury agent can invoke.

## Build & Run
```bash
cargo run
```
The binary starts the TUI and connects to the Mercury API as configured in `.openmerc.toml`.

## Configuration (`.openmerc.toml`)
```toml
[mercury]
base_url = "https://api.inceptionlabs.ai/v1"
api_key = ""  # Set via MERCURY_API_KEY or INCEPTION_API_KEY env var
model = "mercury-2"
max_tokens = 16384

[honcho]
enabled = true
base_url = "https://your-honcho-api.example.com"
app_id = "your-app-id"
user_id = "your-user-id"
assistant_name = "merc"
workspace_id = "your-workspace-id"

[agent]
name = "Merc"
system_prompt = "..."
```
- `mercury` section defines API endpoint, model, and token limits.
- `honcho` enables the Honcho memory service.
- `agent` holds the agent name and system prompt.

## Tool List (from `src/tools/registry.rs`)
| Tool | Description |
|------|-------------|
| `read_file` | Read a file from the workspace. |
| `write_file` | Write content to a file (creates dirs). |
| `list_dir` | List files/directories at a path. |
| `glob_search` | Find files matching a glob pattern. |
| `grep_search` | Search text pattern in files matching a glob. |
| `run_command` | Execute a shell command (build, test, git, …). |
| `create_task` | Create a tracked task. |
| `update_task` | Update task status, title, or description. |
| `list_tasks` | List all tasks for the session. |
| `search_memory` | Query Honcho memory for user/project info. |
| `semfora_analyze` | Run `semfora-engine analyze` on a path. |
| `semfora_search` | Run `semfora-engine search` with a query. |
| `semantic_read_file` | Return a concise semantic summary of a file via Semfora. |

## Known Bugs / Limitations
- **Crash on `src/` write** – writing files under `src/` can trigger a panic due to path handling edge‑cases.
- **Context blow‑up after ~50 tool rounds** – the internal prompt grows large, causing the LLM to exceed context limits and crash.
- These issues are documented in the project README and may require refactoring of the tool execution pipeline.

---
*Generated from the actual repository contents (Cargo.toml, .openmerc.toml, source tree).*
