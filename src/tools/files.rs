use anyhow::Result;
use std::path::Path;

use super::sandbox::resolve_sandboxed;

/// Read a file's contents, sandboxed to workspace
pub fn read_file(workspace: &Path, path: &str) -> Result<String> {
    let resolved = resolve_sandboxed(workspace, path)?;

    if !resolved.exists() {
        anyhow::bail!("File does not exist: {path}");
    }
    if resolved.is_dir() {
        anyhow::bail!("{path} is a directory, not a file. Use list_dir instead.");
    }

    // Check if file is likely binary
    let metadata = std::fs::metadata(&resolved)?;
    if metadata.len() > 1_000_000 {
        anyhow::bail!("{path} is too large ({} bytes). Read a specific section or use grep_search.", metadata.len());
    }

    match std::fs::read_to_string(&resolved) {
        Ok(content) => Ok(content),
        Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
            anyhow::bail!("{path} is a binary file (not valid UTF-8). Cannot read as text.");
        }
        Err(e) => Err(e.into()),
    }
}

/// Write content to a file, sandboxed to workspace.
/// For src/ files: writes, runs `cargo check`, reverts on failure.
/// Returns a unified diff of changes.
pub fn write_file(workspace: &Path, path: &str, content: &str) -> Result<String> {
    if let Ok(Some(advice)) = crate::semfora::advise_path(workspace, path) {
        crate::logger::log("SEMFORA", &advice);
    }

    let resolved = resolve_sandboxed(workspace, path)?;

    // Save old content for diff and potential revert
    let old_content = std::fs::read_to_string(&resolved).unwrap_or_default();
    let is_new = !resolved.exists();
    let is_source = path.starts_with("src/") || path.ends_with(".rs");

    // Ensure parent directory exists
    if let Some(parent) = resolved.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write the file
    std::fs::write(&resolved, content)?;

    // For source files: verify with cargo check, revert on failure
    if is_source {
        crate::logger::log("WRITE", &format!("Source file {path} written, running cargo check..."));
        let check = std::process::Command::new("cargo")
            .arg("check")
            .current_dir(workspace)
            .output();

        match check {
            Ok(output) if !output.status.success() => {
                // Revert the write
                if is_new {
                    let _ = std::fs::remove_file(&resolved);
                } else {
                    let _ = std::fs::write(&resolved, &old_content);
                }
                let stderr = String::from_utf8_lossy(&output.stderr);
                // Extract just the error lines
                let errors: String = stderr.lines()
                    .filter(|l| l.contains("error"))
                    .take(5)
                    .collect::<Vec<_>>()
                    .join("\n");
                crate::logger::log("WRITE", &format!("REVERTED {path} — cargo check failed"));
                anyhow::bail!("cargo check FAILED — file reverted. Errors:\n{errors}\n\nFix the errors and try again.");
            }
            Ok(_) => {
                crate::logger::log("WRITE", &format!("{path} passed cargo check"));
            }
            Err(e) => {
                crate::logger::log("WRITE", &format!("cargo check could not run: {e}"));
            }
        }
    }

    // Generate diff
    if is_new {
        let mut diff = format!("--- /dev/null\n+++ b/{path}\n@@ -0,0 +1,{} @@\n", content.lines().count());
        for line in content.lines() {
            diff.push_str(&format!("+{line}\n"));
        }
        if diff.len() > 4000 {
            diff.truncate(4000);
            diff.push_str("\n... (truncated)");
        }
        Ok(diff)
    } else {
        let diff = generate_unified_diff(path, &old_content, content);
        if diff.is_empty() {
            Ok(format!("Wrote {path} (no changes)"))
        } else {
            Ok(diff)
        }
    }
}

/// List files in a directory, sandboxed to workspace
pub fn list_dir(workspace: &Path, path: &str) -> Result<Vec<String>> {
    let resolved = resolve_sandboxed(workspace, path)?;
    let mut entries = Vec::new();

    for entry in std::fs::read_dir(&resolved)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if entry.file_type()?.is_dir() {
            entries.push(format!("{name}/"));
        } else {
            entries.push(name);
        }
    }

    entries.sort();
    Ok(entries)
}

/// Generate a unified diff between old and new content
fn generate_unified_diff(path: &str, old: &str, new: &str) -> String {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let mut diff = String::new();
    diff.push_str(&format!("--- a/{path}\n"));
    diff.push_str(&format!("+++ b/{path}\n"));

    // Simple line-by-line diff with context
    let max_len = old_lines.len().max(new_lines.len());
    let mut in_hunk = false;
    let mut hunk_start_old = 0;
    let mut hunk_start_new = 0;
    let mut hunk_lines: Vec<String> = Vec::new();
    let mut changes_found = false;

    let context = 3;
    let mut last_change = 0_usize;

    for i in 0..max_len {
        let old_line = old_lines.get(i).copied();
        let new_line = new_lines.get(i).copied();

        let changed = old_line != new_line;

        if changed {
            changes_found = true;
            if !in_hunk {
                // Start a new hunk with context
                in_hunk = true;
                hunk_start_old = i.saturating_sub(context);
                hunk_start_new = i.saturating_sub(context);
                hunk_lines.clear();
                // Add leading context
                for j in i.saturating_sub(context)..i {
                    if let Some(line) = old_lines.get(j) {
                        hunk_lines.push(format!(" {line}"));
                    }
                }
            }
            last_change = i;

            if let Some(line) = old_line {
                hunk_lines.push(format!("-{line}"));
            }
            if let Some(line) = new_line {
                hunk_lines.push(format!("+{line}"));
            }
        } else if in_hunk {
            if i > last_change + context {
                // End hunk
                let old_count = i - hunk_start_old;
                let new_count = i - hunk_start_new;
                diff.push_str(&format!(
                    "@@ -{},{} +{},{} @@\n",
                    hunk_start_old + 1, old_count,
                    hunk_start_new + 1, new_count
                ));
                for line in &hunk_lines {
                    diff.push_str(line);
                    diff.push('\n');
                }
                in_hunk = false;
                hunk_lines.clear();
            } else if let Some(line) = old_line {
                hunk_lines.push(format!(" {line}"));
            }
        }
    }

    // Flush remaining hunk
    if in_hunk && !hunk_lines.is_empty() {
        let end = max_len.min(last_change + context + 1);
        let old_count = end.min(old_lines.len()) - hunk_start_old;
        let new_count = end.min(new_lines.len()) - hunk_start_new;
        diff.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            hunk_start_old + 1, old_count,
            hunk_start_new + 1, new_count
        ));
        for line in &hunk_lines {
            diff.push_str(line);
            diff.push('\n');
        }
    }

    if !changes_found {
        return String::new();
    }

    // Truncate very long diffs
    if diff.len() > 4000 {
        diff.truncate(4000);
        diff.push_str("\n... (diff truncated)");
    }

    diff
}

/// Apply a change to a file using progressive Semfora analysis and dead‑code validation.
/// Returns the diff of the write operation.
pub fn apply_change(workspace: &Path, path: &str, new_content: &str) -> Result<String> {
    // 1️⃣ Progressive analysis – find module risk and advise.
    if let Some(module) = crate::semfora::progressive_analyze(workspace, path)? {
        crate::logger::log("SEMFORA_ANALYSIS", &format!("Target file '{}' belongs to module '{}'.", path, module));
    }

    // 2️⃣ Write the file (advisory already logged inside write_file).
    let diff = write_file(workspace, path, new_content)?;

    // 3️⃣ Dead‑code detection after change.
    match crate::semfora::check_dead_code(workspace) {
        Ok(report) => {
            crate::logger::log("DEADCODE", "Dead‑code check completed");
            crate::logger::log("DEADCODE_REPORT", &report);
        }
        Err(e) => {
            crate::logger::log("DEADCODE_ERROR", &format!("Dead‑code check failed: {}", e));
        }
    }

    Ok(diff)
}
