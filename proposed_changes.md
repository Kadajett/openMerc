# Proposed Changes

## Issue
`tmux_kill_pane` currently determines the pane to protect by calling:

```rust
let our_pane = run_tmux(&["display-message", "-p", "#{pane_id}"]);
```

After a `split-window` (as used in `spawn_agent`), tmux automatically switches focus to the newly created pane. Consequently the *active* pane is no longer the pane that the Merc process is attached to, so the safety check can be bypassed and the function may unintentionally kill the pane that owns the test harness.

## Fix
Use the `TMUX_PANE` environment variable, which tmux sets for the client pane and does **not** change when the user switches panes or when new panes are created. Fall back to the original `display-message` approach only when the variable is missing (e.g., when running outside of tmux).

### Code Change (src/tools/tmux.rs)
```diff
@@
 pub fn tmux_kill_pane(pane_id: &str) -> String {
-    // Safety: don't kill our own pane
-    let our_pane = run_tmux(&["display-message", "-p", "#{pane_id}"]);
-    if our_pane.trim() == pane_id {
-        return format!("Refused to kill own pane {pane_id}. Use tmux_kill_pane on other panes only.");
-    }
+    // Safety: don't kill the pane this process is attached to.
+    // TMUX_PANE is set by tmux for the client pane and remains constant
+    // regardless of which pane is currently active.
+    let our_pane = std::env::var("TMUX_PANE")
+        .unwrap_or_else(|_| run_tmux(&["display-message", "-p", "#{pane_id}"]).trim().to_string());
+    if our_pane == pane_id {
+        return format!("Refused to kill own pane {pane_id}. Use tmux_kill_pane on other panes only.");
+    }
*** End of File
```

## Rationale
* `TMUX_PANE` reliably identifies the pane that launched the Merc process.
* The fallback maintains compatibility when the environment variable is not set.
* No other parts of the code need modification; the safety guard now works correctly after any `split-window` operation.

## Impact
* The function will no longer mistakenly kill the pane that owns the test harness.
* Existing behavior when `TMUX_PANE` is unset remains unchanged because of the fallback.
