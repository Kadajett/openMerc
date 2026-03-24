use std::path::Path;
use std::process::Command;

/// Get info about the current tmux environment
pub fn tmux_info() -> String {
    let session = run_tmux(&["display-message", "-p", "#{session_name}"]);
    let window = run_tmux(&["display-message", "-p", "#{window_index}:#{window_name}"]);
    let pane = run_tmux(&["display-message", "-p", "#{pane_id}"]);
    let pane_count = run_tmux(&["list-panes", "-F", "#{pane_id}"]);
    let all_panes: Vec<&str> = pane_count.trim().lines().collect();

    format!(
        "tmux session: {}\nwindow: {}\ncurrent pane: {}\ntotal panes: {}\npane ids: {}",
        session.trim(),
        window.trim(),
        pane.trim(),
        all_panes.len(),
        all_panes.join(", ")
    )
}

/// Run a command in a specific tmux pane (or current pane if pane_id is empty)
pub fn tmux_run(pane_id: &str, command: &str) -> String {
    let target = if pane_id.is_empty() {
        // Run in a new split pane
        let output = run_tmux(&[
            "split-window", "-h", "-P", "-F", "#{pane_id}", command
        ]);
        return format!("Spawned new pane: {}", output.trim());
    } else {
        pane_id.to_string()
    };

    // Send command to existing pane
    run_tmux(&["send-keys", "-t", &target, command, "Enter"]);
    // Brief wait then capture
    std::thread::sleep(std::time::Duration::from_millis(500));
    tmux_capture(&target, 20)
}

/// Capture the last N lines from a tmux pane
pub fn tmux_capture(pane_id: &str, lines: u32) -> String {
    let output = Command::new("tmux")
        .args(["capture-pane", "-t", pane_id, "-p", "-S", &format!("-{lines}")])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout).trim().to_string()
        }
        Ok(o) => format!("capture failed: {}", String::from_utf8_lossy(&o.stderr)),
        Err(e) => format!("tmux error: {e}"),
    }
}

/// Send keystrokes to a tmux pane.
/// Special keys: Enter, Escape, C-c, Up, Down, Left, Right, Tab
/// Regular text is typed literally (use -l flag).
/// If text ends with \n, Enter is sent automatically after.
pub fn tmux_send_keys(pane_id: &str, keys: &str) -> String {
    let special = ["Enter", "Escape", "C-c", "C-d", "Up", "Down", "Left", "Right", "Tab",
                    "BSpace", "Space", "C-a", "C-e", "C-k", "C-u", "C-l"];

    if special.iter().any(|s| *s == keys) {
        // Send as a special key
        run_tmux(&["send-keys", "-t", pane_id, keys]);
    } else if keys.ends_with('\n') || keys.ends_with("\\n") {
        // Text + Enter: type the text then press Enter
        let text = keys.trim_end_matches('\n').trim_end_matches("\\n");
        if !text.is_empty() {
            run_tmux(&["send-keys", "-t", pane_id, "-l", text]);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        run_tmux(&["send-keys", "-t", pane_id, "Enter"]);
    } else {
        // Just type the text
        run_tmux(&["send-keys", "-t", pane_id, "-l", keys]);
    }

    std::thread::sleep(std::time::Duration::from_millis(300));
    format!("Sent to {pane_id}: {}", if keys.len() > 50 { &keys[..50] } else { keys })
}

/// Split the current window and return the new pane ID
pub fn tmux_split(direction: &str, command: Option<&str>) -> String {
    let dir_flag = if direction == "vertical" { "-v" } else { "-h" };

    let mut args = vec!["split-window", dir_flag, "-P", "-F", "#{pane_id}"];
    if let Some(cmd) = command {
        args.push(cmd);
    }

    let output = run_tmux(&args);
    let pane_id = output.trim().to_string();

    if pane_id.is_empty() {
        "Failed to split pane".to_string()
    } else {
        format!("New pane: {pane_id}")
    }
}

/// Kill a tmux pane
pub fn tmux_kill_pane(pane_id: &str) -> String {
    run_tmux(&["kill-pane", "-t", pane_id]);
    format!("Killed pane {pane_id}")
}

/// Spawn a new instance of openMerc in a split pane for testing.
/// Builds first, then runs in the new pane. Returns the pane ID.
pub fn spawn_agent(workspace: &Path, extra_env: &str) -> String {
    // Build first
    let build = Command::new("cargo")
        .arg("build")
        .current_dir(workspace)
        .output();

    match build {
        Ok(o) if !o.status.success() => {
            let err = String::from_utf8_lossy(&o.stderr);
            return format!("Build failed:\n{err}");
        }
        Err(e) => return format!("Failed to run cargo build: {e}"),
        _ => {}
    }

    // Spawn in a new horizontal split
    let cmd = if extra_env.is_empty() {
        format!(
            "cd {} && INCEPTION_API_KEY=$INCEPTION_API_KEY cargo run 2>/dev/null",
            workspace.display()
        )
    } else {
        format!(
            "cd {} && {} cargo run 2>/dev/null",
            workspace.display(),
            extra_env
        )
    };

    let output = run_tmux(&[
        "split-window", "-h", "-P", "-F", "#{pane_id}",
        "-c", &workspace.display().to_string(),
        &cmd,
    ]);

    let pane_id = output.trim().to_string();
    if pane_id.is_empty() {
        "Failed to spawn agent pane".to_string()
    } else {
        format!("Agent spawned in pane {pane_id}. Use tmux_send_keys to interact, tmux_capture to read output.")
    }
}

fn run_tmux(args: &[&str]) -> String {
    match Command::new("tmux").args(args).output() {
        Ok(o) => {
            if o.status.success() {
                String::from_utf8_lossy(&o.stdout).to_string()
            } else {
                let err = String::from_utf8_lossy(&o.stderr);
                format!("tmux error: {err}")
            }
        }
        Err(e) => format!("Failed to run tmux: {e}"),
    }
}
