# Proposed Changes

## Overview
Implement a **Plan Mode** that allows users to define a multi‑step plan, generate tasks, and execute them with optional approval. The mode persists across sessions and can be paused/resumed.

## Tasks
1. **Add AppMode variants** – Extend `AppMode` with `Plan` and `Paused` variants.
2. **Add PlanState struct** – Define a serializable `PlanState` (mode, tasks, last_updated) and store it in `App` as `Option<PlanState>`.
3. **Initialize plan_state** – Set `plan_state: None` in `App::new`.
4. **Create plan module** – `src/plan.rs` containing:
   - `should_enter_plan_mode(&Message) -> bool`
   - `init_plan(&mut App, description: &str)`
   - `generate_plan(&mut App, description: &str)`
   - `run_next_task(&mut App)`
   - `handle_approve(&mut App, ids: &[usize])`
   - `handle_resume(&mut App)`
   - Persistence helpers (`save_plan_state`, `load_plan_state`).
5. **Register slash commands** – In `src/main.rs` add handling for `/plan`, `/approve`, `/resume` (and `/plan` without args to show status). These will call the functions from the plan module.
6. **Update UI** – Extend UI rendering to show plan tasks when `app_mode == AppMode::Plan` (optional for now, can be a placeholder).
7. **Persist plan state** – Store JSON under `<workspace>/.plan/<session_id>.json` and load on startup if present.
8. **Tests** – Add unit tests for `PlanState` serialization and the plan module logic.

## Implementation Steps
- Modify `src/app.rs` to add the enum variants, `PlanState` struct, and `plan_state` field.
- Create `src/plan.rs` with the above functions, using `serde_json` for persistence.
- Update `src/main.rs` to import `plan` and handle the new slash commands.
- Ensure any new dependencies (`serde_json`, `once_cell` if needed) are added to `Cargo.toml`.
- Run `cargo test` and `cargo build` to verify.

## Risks & Mitigations
- **State consistency** – Persist after each task change; load on app start.
- **User interruption** – `Paused` mode stops task execution; `resume` clears the flag.
- **Backward compatibility** – Existing sessions without a `.plan` file simply start with `plan_state = None`.

## Next Steps
- Implement tasks 1‑3 in `src/app.rs`.
- Add `src/plan.rs` (task 4).
- Modify `src/main.rs` for command registration (task 5).
- Incrementally test each change.
