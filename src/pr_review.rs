// src/pr_review.rs
use std::process::Command;
use std::path::Path;
use semfora_engine::analyze_diff; // placeholder for actual semfora engine call
use git2::Repository;

/// Generates an automated PR review summary for the given branch.
/// It runs `git diff main..branch`, analyzes the diff with semfora-engine,
/// and returns a textual summary.
pub fn review_pr(branch: &str) -> Result<String, String> {
    // Ensure we are in a git repo
    let repo = Repository::discover(".")
        .map_err(|e| format!("Failed to locate repo: {}", e))?;
    // Get diff between main and branch
    let output = Command::new("git")
        .args(&["diff", "main..", branch])
        .output()
        .map_err(|e| format!("git diff failed: {}", e))?;
    if !output.status.success() {
        return Err(format!("git diff error: {}", String::from_utf8_lossy(&output.stderr)));
    }
    let diff_text = String::from_utf8_lossy(&output.stdout);
    // Analyze diff with semfora-engine (placeholder command)
    let analysis = Command::new("semfora-engine")
        .args(&["analyze-diff", "-"])
        .stdin(diff_text.as_bytes().to_owned())
        .output()
        .map_err(|e| format!("semfora analyze failed: {}", e))?;
    if !analysis.status.success() {
        return Err(format!("semfora error: {}", String::from_utf8_lossy(&analysis.stderr)));
    }
    let summary = String::from_utf8_lossy(&analysis.stdout).to_string();
    Ok(summary)
}
