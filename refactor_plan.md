# Refactor Plan

## 1. `src/main.rs`

### Functions that need splitting
| Original Function | Reason (high cognitive complexity) |
|-------------------|-----------------------------------|
| `async fn main() -> Result<()>` (cognitive complexity 179, nesting depth 7) | Monolithic setup, session picker, UI loop, and shutdown logic all in one function. |
| `fn handle_key(...)` | Handles many unrelated concerns (global shortcuts, focus handling). |
| `fn handle_input_key(...)` | Very long match with many UI actions and async task spawning. |
| `fn handle_chat_key(...)` | Mixes navigation and focus changes. |
| `fn handle_mouse(...)` | Contains UI‑specific logic and terminal‑size handling. |
| `fn summarize_tool_args(name: &str, args_json: &str) -> String` | Large match with many branches; could be split per tool. |

### Proposed split signatures
```rust
// Init & teardown
fn init_terminal() -> Result<()>;
fn restore_terminal();
fn init_contexts(workspace: &Path) -> Result<(Config, Arc<MercuryClient>, Arc<Mutex<HonchoContext>>)>;

// Session picker
async fn run_session_picker(
    app: &mut App,
    index: &SessionIndex,
    honcho: Arc<Mutex<HonchoContext>>,
    tx: mpsc::UnboundedSender<AppEvent>,
) -> Result<()>;

// Main UI loop
async fn run_ui_loop(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,\n    mercury: Arc<MercuryClient>,
    honcho: Arc<Mutex<HonchoContext>>,
    tx: mpsc::UnboundedSender<AppEvent>,
) -> Result<()>;

// Key handling – split by focus
fn handle_global_shortcuts(app: &mut App, key: KeyEvent) -> bool; // true if handled
fn handle_input_key(
    app: &mut App,
    key: KeyEvent,
    tx: &mpsc::UnboundedSender<AppEvent>,
    mercury: &Arc<MercuryClient>,
    honcho: &Arc<Mutex<HonchoContext>>,
    system_prompt: &str,
) -> (); 
fn handle_chat_key(app: &mut App, key: KeyEvent) -> (;
fn handle_mouse(app: &mut App, mouse: MouseEvent) -> (;

// Tool‑argument summariser – per‑tool helpers
fn summarize_read_file(args: &serde_json::Value) -> String;
fn summarize_write_file(args: &serde_json::Value) -> String;
fn summarize_glob_search(args: &serde_json::Value) -> String;
fn summarize_grep_search(args: &serde_json::Value) -> String;
fn summarize_run_command(args: &serde_json::Value) -> String;
fn summarize_generic(args_json: &str) -> String;
```

## 2. `src/context/honcho.rs`

### Functions that need splitting
| Original Function | Reason |
|-------------------|--------|
| `pub async fn search_workspace(&mut self, query: &str) -> Option<String>` (cognitive complexity 76) | Handles deduplication, logging, request building, response parsing, and truncation. |
| `pub async fn enrich_system_prompt(&self, base_prompt: &str, user_query: &str) -> String` | Performs multiple async calls, concatenates strings, and contains branching logic. |
| `pub async fn query_conclusions(&self, query: &str) -> Option<String>` | Similar pattern to `search_workspace`.

### Proposed split signatures
```rust
// Low‑level request helpers
async fn honcho_post<T: Serialize>(&self, path: &str, payload: &T) -> Result<serde_json::Value>;
async fn honcho_get(&self, path: &str, query: &[(&str, &str)]) -> Result<serde_json::Value>;

// High‑level API calls (thin wrappers)
pub async fn search_workspace(&mut self, query: &str) -> Option<String> {
    self._search_workspace(query).await
}
async fn _search_workspace(&mut self, query: &str) -> Option<String>;

pub async fn enrich_system_prompt(&self, base_prompt: &str, user_query: &str) -> String {
    self._enrich_system_prompt(base_prompt, user_query).await
}
async fn _enrich_system_prompt(&self, base_prompt: &str, user_query: &str) -> String;

pub async fn query_conclusions(&self, query: &str) -> Option<String> {
    self._query_conclusions(query).await
}
async fn _query_conclusions(&self, query: &str) -> Option<String>;
```

## 3. `src/api/mercury.rs`

### Functions that need splitting
| Original Function | Reason |
|-------------------|--------|
| `pub async fn run_tool_loop(... ) -> Result<Option<String>>` (cognitive complexity 65) | Contains loop, cancellation handling, request building, tool execution, logging, and state updates. |
| `pub async fn chat(... )` | Orchestrates message building, tool loop, diffusion streaming, and error handling – too big for a single function. |

### Proposed split signatures
```rust
// Build the chat request payload
fn build_chat_request(
    &self,
    system_context: Option<&str>,
    messages: &[Message],
    tools: &[ToolDef],
    stream: bool,
    diffusing: bool,
) -> ChatRequest;

// Execute the tool‑calling phase
pub async fn tool_phase(
    &self,
    msgs: &mut Vec<ChatMessage>,
    tools: &[ToolDef],
    max_rounds: u32,
    tool_ctx: registry::ToolContext,
    tx: &mpsc::UnboundedSender<AppEvent>,
    cancel: &CancellationToken,
) -> Result<Option<String>>;

// Execute the diffusion (streaming) phase
pub async fn diffusion_phase(
    &self,
    msgs: Vec<ChatMessage>,
    tx: &mpsc::UnboundedSender<AppEvent>,
) -> Result<()>;

// Public entry point – thin orchestrator
pub async fn chat(
    &self,
    system_context: Option<&str>,
    messages: &[Message],
    tool_ctx: registry::ToolContext,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    cancel: CancellationToken,
) {
    // build request, call tool_phase, then diffusion_phase
}
```

---

**Next steps**
1. Create new modules/files for the split functions (e.g., `src/ui/input.rs`, `src/ui/chat.rs`, `src/context/honcho_impl.rs`, `src/api/mercury_tool.rs`).
2. Update `mod` declarations accordingly.
3. Adjust all call sites to use the new signatures.
4. Run `cargo test` / `cargo run` to ensure behaviour is unchanged.
