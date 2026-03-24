# Audit Report

## Summary

- **Highest Cognitive Complexity**: `src/main.rs` (complexity = 179, nesting depth = 7) – contains the bulk of the application logic, UI handling, and async orchestration.
- **Second Highest**: `src/context/honcho.rs` (complexity = 76, nesting depth = 4).
- **Third Highest**: `src/api/mercury.rs` (complexity = 65, nesting depth = 4).

All files examined have **behavioral_risk: high** except a few simple modules (`src/api/mod.rs`, `src/context/mod.rs`). The high‑risk files are the ones with the greatest complexity.

## Recommended Refactors

### 1. `src/main.rs`
- **Extract UI loop** into its own module (`ui_loop.rs`).
- **Separate async orchestration** (Mercury calls, Honcho sessions, task handling) into service modules (`services/mercury.rs`, `services/honcho.rs`).
- **Reduce nesting** by early‑returning on errors and using `?` propagation.
- **Split large match/if chains** into smaller helper functions.
- **Limit state changes** – group related state updates into structs or use builder patterns.

### 2. `src/context/honcho.rs`
- **Isolate networking logic** into a `network.rs` helper.
- **Break large functions** (e.g., `search`, `fetch_context`) into smaller, single‑responsibility methods.
- **Reduce duplicated state updates** (e.g., `self.reachable` assignments) by consolidating them.

### 3. `src/api/mercury.rs`
- **Wrap API request/response handling** in a dedicated `client.rs` module.
- **Deduplicate code** – the request flow appears twice; extract to a reusable function.
- **Simplify state changes** – consider a builder for request payloads.

### 4. General Recommendations
- **Introduce a linting/formatting rule** to cap cognitive complexity (e.g., max = 30) and nesting depth (max = 3).
- **Add unit tests** for each extracted function to ensure behavior stays unchanged.
- **Document public APIs** with `///` comments to aid future audits.

---

*Audit performed using Semfora semantic analysis on each source file in `src/`. No source files were modified.*
