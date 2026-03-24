// src/test_runner.rs
use std::process::{Command, Stdio};
use std::io::{self, BufRead};

#[derive(Debug, Default)]
pub struct TestResult {
    pub passed: Vec<String>,
    pub failed: Vec<String>,
    pub ignored: Vec<String>,
    pub failures: Vec<(String, String)>, // (test name, output snippet)
}

pub fn run_tests() -> io::Result<TestResult> {
    let mut cmd = Command::new("cargo");
    cmd.arg("test")
        .arg("--no-run") // compile only to get output quickly
        .arg("--")
        .arg("--nocapture")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = cmd.spawn()?;
    let stdout = child.stdout.ok_or_else(|| io::Error::new(io::ErrorKind::Other, "no stdout"))?;
    let reader = io::BufReader::new(stdout);
    let mut result = TestResult::default();
    for line in reader.lines() {
        let line = line?;
        // Simple parsing based on cargo test output patterns
        if line.contains("test") && line.contains("ok") {
            // e.g. test test_module::test_name ... ok
            if let Some(name) = line.split_whitespace().nth(1) {
                result.passed.push(name.to_string());
            }
        } else if line.contains("test") && line.contains("FAILED") {
            if let Some(name) = line.split_whitespace().nth(1) {
                result.failed.push(name.to_string());
                // capture following lines until empty line as failure detail
                // (simplified: just store the line itself)
                result.failures.push((name.to_string(), line.clone()));
            }
        } else if line.contains("test") && line.contains("ignored") {
            if let Some(name) = line.split_whitespace().nth(1) {
                result.ignored.push(name.to_string());
            }
        }
    }
    Ok(result)
}

pub fn format_test_results(res: &TestResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("Passed: {}\n", res.passed.len()));
    out.push_str(&format!("Failed: {}\n", res.failed.len()));
    out.push_str(&format!("Ignored: {}\n", res.ignored.len()));
    if !res.failures.is_empty() {
        out.push_str("\nFailure details:\n");
        for (name, detail) in &res.failures {
            out.push_str(&format!("- {}: {}\n", name, detail));
        }
    }
    out
}
