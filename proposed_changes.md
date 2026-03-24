# Proposed Changes for Task d5b08e4b - Fix exit/crash when writing own source files

## Context
The binary crashes after writing to files under `src/` because the running executable is built from stale source code. The current implementation writes directly to source files while the process is still using the compiled code, leading to a mismatch and eventual panic.

## Desired Behavior
- Detect when a write operation targets a file inside `src/`.
- Instead of writing directly, queue the change and apply it after the process exits, or trigger a safe reload.
- Ensure the binary does not attempt to reload the modified module at runtime.

## Proposed Code Changes
1. **`src/commands.rs`** – Add a guard in the file‑write command:
   ```rust
   if path.starts_with("src/") {
       // Queue change instead of immediate write
       pending_changes.push((path.clone(), content.clone()));
       println!("Change queued for {} – will be applied on next restart.", path);
       return Ok(());
   }
   ```
2. **`src/session.rs`** – Introduce a new `pending_changes: Vec<(String, String)>` field in the `Session` struct to store queued writes.
3. **`src/main.rs`** – On graceful shutdown (e.g., handling SIGINT/SIGTERM), iterate over `session.pending_changes` and write them to disk before exiting.
4. **`src/logger.rs`** – Log a warning when a direct write to `src/` is attempted without queuing.
5. **`src/config.rs`** – Add a config flag `allow_direct_src_write: bool` (default false). If set to true, the old behavior is retained for advanced users.

## Implementation Steps
- Add the `pending_changes` field and associated methods (`queue_change`, `apply_pending`).
- Update the command handling logic to use the guard.
- Modify the shutdown hook in `main.rs` to call `apply_pending`.
- Add unit tests in `src/tests/` to verify that queued changes are persisted after shutdown.
- Update documentation in `README.md` to explain the new behavior.

## Risks & Mitigations
- **Risk:** Pending changes may be lost if the process is killed abruptly.
  - **Mitigation:** Write pending changes to a temporary file (`.pending_changes.json`) after each queue operation and read it on startup.
- **Risk:** Introducing a new config flag may break existing scripts.
  - **Mitigation:** Keep the default false and document the flag clearly.

## Acceptance Criteria
- Writing to a file under `src/` no longer crashes the binary.
- Changes are persisted after a graceful shutdown.
- Unit tests pass and cover the new queueing logic.
- No regression in existing file‑write functionality for non‑src paths.

---

# Proposed Changes for Task a8ff585a - Task list visible in UI above chat

## Context
The current UI only shows the chat interface. Users need a persistent view of pending/completed tasks above the chat area.

## Desired Behavior
- A collapsible panel at the top of the chat window displays the task list.
- Each task shows its title, status (pending/in_progress/completed), and a short description.
- Clicking a task expands details.
- The panel updates in real‑time as task statuses change.

## Proposed Code Changes
1. **`ui/components/TaskPanel.tsx`** – New React component that fetches tasks via the backend API and renders them.
2. **`ui/App.tsx`** – Insert `<TaskPanel />` above the `<ChatWindow />` component.
3. **`backend/api/tasks.rs`** – Add endpoint `GET /tasks` returning JSON of all tasks.
4. **`backend/api/tasks.rs`** – Add endpoint `PATCH /tasks/:id` to update status.
5. **`ui/styles/TaskPanel.module.css`** – Styling for the panel (light background, scrollable list).
6. **`ui/hooks/useTaskPolling.ts`** – Hook that polls the `/tasks` endpoint every 2 seconds.

## Implementation Steps
- Implement the new React component with TypeScript.
- Wire the component to the polling hook.
- Ensure the backend returns tasks in the format `{id, title, status, description}`.
- Add unit tests for the API endpoints.
- Update the build pipeline to include the new component.

## Risks & Mitigations
- **Risk:** Frequent polling may increase load.
  - **Mitigation:** Use WebSocket for push updates in future; for now, 2 s interval is acceptable.
- **Risk:** UI may overflow on small screens.
  - **Mitigation:** Make the panel collapsible and responsive.

## Acceptance Criteria
- The task list appears above the chat.
- Status changes reflect immediately.
- No regression in chat functionality.

---

# Proposed Changes for Task 548214c6 - Autonomous plan mode

## Context
After completing a task, the system should automatically generate the next logical task when the user prompt is generic, enabling a continuous work loop.

## Desired Behavior
- Detect when a task reaches `completed` status.
- If the latest user message does not specify a new task, invoke the planner to suggest the next step.
- Automatically create a new task and add it to the task list.
- Continue looping until the user provides a specific instruction.

## Proposed Code Changes
1. **`backend/planner.rs`** – New module exposing `fn suggest_next_task(context: &str) -> Task`.
2. **`backend/worker.rs`** – After marking a task completed, call `suggest_next_task` if the last user message lacks a concrete request.
3. **`backend/api/tasks.rs`** – Extend `POST /tasks` to accept auto‑generated tasks.
4. **`ui/components/AutoPlanNotice.tsx`** – Optional UI toast indicating an auto‑generated task.
5. **`backend/state.rs`** – Store the last user message to evaluate for genericness.

## Implementation Steps
- Implement a simple heuristic: if the last user message matches regex `(?i)\b(continue|next|keep|auto)\b` then trigger planner.
- The planner can be a stub that returns a generic task like "Review code quality"; later replace with LLM suggestion.
- Add unit tests for the detection logic and task creation.
- Ensure the UI reflects newly created tasks.

## Risks & Mitigations
- **Risk:** Infinite loop if the planner keeps generating tasks.
  - **Mitigation:** Limit auto‑generation to a maximum of 3 consecutive tasks without explicit user input.
- **Risk:** Users may be confused by auto‑created tasks.
  - **Mitigation:** Show a clear UI notice and allow manual dismissal.

## Acceptance Criteria
- After a task completes, a new task is auto‑created when the prompt is generic.
- The UI shows the new task immediately.
- Loop stops after 3 auto‑generated tasks or when user provides a specific instruction.

---

# Proposed Changes for Task bfdb43bb - Better terminal cleanup on exit

## Context
When the application exits, the terminal sometimes remains in raw mode or displays stray characters, affecting subsequent shell usage.

## Desired Behavior
- Ensure the terminal state is restored to its original settings on any exit path (normal, error, panic).
- Clear any partial lines or prompts left on the screen.

## Proposed Code Changes
1. **`src/terminal.rs`** – Introduce a `TerminalGuard` struct that saves the original terminal mode on creation and restores it on `Drop`.
2. **`src/main.rs`** – Instantiate `TerminalGuard` at program start.
3. **`src/error_handler.rs`** – In the panic hook, call `TerminalGuard::restore()` before exiting.
4. **`src/cleanup.rs`** – Add function `fn clear_screen()` that writes ANSI escape sequence `\x1b[2J\x1b[H`.
5. **`src/main.rs`** – Register `clear_screen` in the `atexit` handler.

## Implementation Steps
- Use the `termios` crate to capture and restore terminal attributes.
- Implement `Drop` for `TerminalGuard` to guarantee restoration.
- Add unit test that spawns a subprocess, runs the binary, and checks terminal state after exit.
- Update documentation in `README.md`.

## Risks & Mitigations
- **Risk:** Incompatibility with Windows consoles.
  - **Mitigation:** Conditional compilation; on Windows use `winapi` to reset console mode.
- **Risk:** Adding a guard may introduce slight performance overhead.
  - **Mitigation:** Negligible; only a few syscalls.

## Acceptance Criteria
- After any exit, the terminal behaves as if the program never ran.
- No stray characters remain.
- Tests pass on Linux and Windows CI.
