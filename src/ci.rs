// src/ci.rs
use std::process::Command;
use std::io::{self, Write};

/// Runs a CI review for the given PR number.
///
/// * Fetches the PR diff via `gh pr diff <pr>`.
/// * Analyzes the diff with `semfora analyze-diff`.
/// * Searches Honcho memory for relevant project context.
/// * Emits a review to stdout.
pub fn run_ci_review(pr_number: u32) -> io::Result<()> {
    // 1. Get diff
    let diff_output = Command::new("gh")
        .args(&["pr", "diff", &pr_number.to_string()])
        .output()?;
    if !diff_output.status.success() {
        eprintln!("Failed to fetch diff for PR {}", pr_number);
        return Ok(());
    }
    let diff = String::from_utf8_lossy(&diff_output.stdout);

    // 2. Run semfora analyze-diff
    let analyze = Command::new("semfora")
        .arg("analyze-diff")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;
    // write diff to stdin
    if let Some(mut stdin) = analyze.stdin {
        stdin.write_all(diff.as_bytes())?;
    }
    let output = analyze.wait_with_output()?;
    let analysis = String::from_utf8_lossy(&output.stdout);

    // 3. Search Honcho memory (stubbed – real implementation would call search_memory)
    // For now we just note that we would perform a search.
    let memory_context = "[Honcho memory search placeholder]";

    // 4. Emit review
    println!("--- CI Review for PR #{} ---", pr_number);
    println!("Diff summary:\n{}", diff.lines().take(10).collect::<Vec<_>>().join("\n"));
    println!("Semfora analysis:\n{}", analysis);
    println!("Relevant context: {}", memory_context);
    println!("--- End of Review ---");
    Ok(())
}
