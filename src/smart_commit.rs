use std::path::Path;
use std::process::Command;

/// Generate a semantic commit message from staged/unstaged changes
pub fn generate_commit_message(workspace: &Path) -> String {
    let mut parts = Vec::new();

    // Get git diff (staged first, fall back to unstaged)
    let diff = Command::new("git")
        .args(["diff", "--cached", "--stat"])
        .current_dir(workspace)
        .output()
        .ok()
        .filter(|o| o.status.success() && !o.stdout.is_empty())
        .or_else(|| Command::new("git")
            .args(["diff", "--stat"])
            .current_dir(workspace)
            .output()
            .ok())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    if diff.is_empty() {
        return "No changes detected".to_string();
    }

    // Count files changed
    let file_count = diff.lines().count().saturating_sub(1); // last line is summary
    parts.push(format!("{file_count} files changed"));

    // Try semfora analyze-diff for semantic summary
    let semfora = Command::new("semfora-engine")
        .args(["analyze-diff", "--format", "json"])
        .current_dir(workspace)
        .output();

    if let Ok(output) = semfora {
        if output.status.success() {
            let raw = String::from_utf8_lossy(&output.stdout);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                // Extract changed modules
                if let Some(modules) = v["changed_modules"].as_array() {
                    let names: Vec<&str> = modules.iter()
                        .filter_map(|m| m["name"].as_str())
                        .take(5)
                        .collect();
                    if !names.is_empty() {
                        parts.push(format!("modules: {}", names.join(", ")));
                    }
                }
                // Extract risk summary
                if let Some(risk) = v["risk_summary"].as_str() {
                    parts.push(format!("risk: {risk}"));
                }
            }
        }
    }

    // Format: summary line from diff stat + semantic details
    let summary = diff.lines().last().unwrap_or("changes").trim().to_string();
    format!("{summary}\n\n{}", parts.join("\n"))
}
