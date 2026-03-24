# ROADMAP for Merc (OpenMerc)

## 1. What We Need to Become a Real Competitor

| Area | Gap vs Claude Code / Codex / Aider | Why It Matters |
|------|-----------------------------------|----------------|
| **Multi‑file, context‑aware editing** | Currently only single‑file operations are safe; we lack a robust project‑wide AST and dependency graph. | Competitors can understand and refactor across many files without user‑driven gluing. |
| **Interactive REPL / Prompt‑driven execution** | No live REPL that can execute snippets, inspect state, or run tests on‑the‑fly. | Users expect instant feedback and the ability to iterate quickly. |
| **Diff‑based code generation** | We output whole files; no fine‑grained diff or patch mode. | Reduces noise, integrates cleanly with version control, and matches Claude Code’s “apply‑patch” workflow. |
| **Test‑first / property‑based generation** | No built‑in test scaffolding or automatic test generation. | Codex and Aider can generate tests and run them to validate suggestions. |
| **IDE‑style integration (LSP, diagnostics)** | No Language Server Protocol support, no inline diagnostics. | Enables editors to surface errors and suggestions in real time. |
| **Secure sandboxed execution** | Current commands run in the same process; no isolation. | Prevents accidental damage to the host system and matches Aider’s sandboxed runner. |
| **Extensible plugin system** | No first‑class plugin API for custom commands or language back‑ends. | Allows community to add support for new languages, frameworks, or CI pipelines. |
| **Rich UI for task management** | UI only shows chat; task list is a work‑in‑progress feature. | Competitors provide persistent task panels, status badges, and drag‑and‑drop editing. |
| **Documentation & onboarding** | Minimal docs; users must read source to understand commands. | Good docs lower friction and increase adoption. |

## 2. Priority Order of Features

1. **Project‑wide Context & Diff Engine** (high impact, medium effort)
   - Build a lightweight AST + file graph.
   - Expose `apply_patch` command that accepts unified diffs.
2. **REPL / Live Execution Environment** (high impact, high effort)
   - Spawn a sandboxed subprocess for Rust/JS/Python snippets.
   - Provide `run`, `inspect`, `test` commands.
3. **Task Panel UI + Real‑time Updates** (medium impact, low effort)
   - Finish the `TaskPanel` React component and backend API.
   - Add WebSocket push for status changes.
4. **Test Generation & Execution** (medium impact, medium effort)
   - Auto‑generate unit tests for extracted functions.
   - Run `cargo test` / `npm test` and report results.
5. **LSP Integration** (high impact, high effort)
   - Implement a minimal LSP server exposing diagnostics, completions, and code actions.
6. **Secure Sandbox for File Ops** (medium impact, low effort)
   - Wrap all file‑write commands in a temporary directory; commit on graceful exit.
7. **Plugin Architecture** (low impact, high effort)
   - Define a plugin manifest and runtime loader.
8. **Comprehensive Docs & Quick‑start Guides** (low impact, low effort)
   - Write markdown tutorials, example scripts, and cheat‑sheet.

## 3. What We Can Do **TODAY** vs What Needs User Implementation

### ✅ Ready to Implement (we can code now)
- **Finish Task Panel UI** – write the missing React component, hook up polling, and add the backend `/tasks` endpoints.n- **Queue Source‑file Writes** – integrate the `pending_changes` logic from *proposed_changes.md* into `src/commands.rs` and ensure graceful shutdown writes them.
- **Terminal Guard** – add `TerminalGuard` in `src/terminal.rs` and register it in `main.rs` to fix raw‑mode exit bugs.
- **Basic Diff Apply** – implement a helper `apply_patch(content: &str, diff: &str) -> String` using the `diffy` crate; expose it via a new CLI flag.
- **Simple REPL Stub** – spawn a child process that runs `cargo run` in a sandbox and pipe stdin/stdout; enough for quick prototyping.

### ⏳ Requires User Action or External Resources
- **Project‑wide AST & Dependency Graph** – needs a full‑scale parser (e.g., `syn` for Rust) and a build‑graph; user must allocate time for design and testing.
- **LSP Server** – would benefit from community contribution; we can scaffold the crate but need user to publish and integrate with editors.
- **Plugin System** – architectural decisions (dynamic loading, safety) are non‑trivial; user should define the plugin API contract.
- **Test Generation** – depends on language‑specific heuristics; user may need to provide language templates or integrate with existing test frameworks.
- **Sandboxed Execution** – would require Docker or `firejail` setup; user must provision container runtime on target machines.

### 📌 Limitations to Be Honest About
- **No AI model inference** – Merc delegates to external LLM APIs; we cannot guarantee latency or quality without a provider.
- **Rust‑only focus** – current codebase is Rust‑centric; adding first‑class support for Python/JS will need additional parsers.
- **Limited error handling** – many commands still `unwrap()` on I/O; we need systematic error propagation.
- **No built‑in multi‑modal support** – unlike diffusion LLMs, we cannot handle images/audio yet.
- **Scalability of task queue** – pending changes are stored in memory; large batches may overflow.

---

*Prepared by Merc, based on audit_report.md, proposed_changes.md, and knowledge of Claude Code, Codex, and Aider.*
