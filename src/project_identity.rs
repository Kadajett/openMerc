//! Module to detect project identity information.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use serde_json;

#[derive(Debug, Clone)]
pub struct ProjectIdentity {
    pub git_remote: Option<String>,
    pub git_branch: Option<String>,
    pub workspace_path: String,
    pub project_name: Option<String>,
    pub module_count: usize,
}

impl ProjectIdentity {
    /// Detects the project identity based on the current workspace.
    pub fn detect() -> Self {
        let workspace_path = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .to_string_lossy()
            .into_owned();

        let git_remote = Self::git_remote_url();
        let git_branch = Self::git_branch_name();
        let project_name = Self::read_project_name();
        let module_count = Self::semfora_module_count();

        ProjectIdentity {
            git_remote,
            git_branch,
            workspace_path,
            project_name,
            module_count,
        }
    }

    fn git_remote_url() -> Option<String> {
        let output = Command::new("git")
            .args(&["remote", "-v"]) 
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Take the first line, second column is the URL.
        stdout.lines().next().and_then(|line| {
            line.split_whitespace().nth(1).map(|s| s.to_string())
        })
    }

    fn git_branch_name() -> Option<String> {
        let output = Command::new("git")
            .args(&["rev-parse", "--abbrev-ref", "HEAD"]) 
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn read_project_name() -> Option<String> {
        // Cargo.toml
        if let Ok(content) = fs::read_to_string("Cargo.toml") {
            for line in content.lines() {
                if line.trim_start().starts_with("name") {
                    if let Some(eq) = line.find('=') {
                        let name_part = line[(eq + 1)..].trim();
                        let name = name_part.trim_matches('"');
                        return Some(name.to_string());
                    }
                }
            }
        }
        // package.json
        if let Ok(content) = fs::read_to_string("package.json") {
            if let Some(start) = content.find("\"name\"") {
                let after = &content[start + 6..];
                if let Some(colon) = after.find(':') {
                    let val = &after[colon + 1..];
                    let trimmed = val.trim();
                    if trimmed.starts_with('"') {
                        if let Some(end) = trimmed[1..].find('"') {
                            return Some(trimmed[1..=end].to_string());
                        }
                    }
                }
            }
        }
        None
    }

    fn semfora_module_count() -> usize {
        // Attempt to run `semfora-engine analyze .` and count modules.
        let output = Command::new("semfora-engine")
            .args(&["analyze", "."]) 
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    if let Some(mods) = json.get("modules") {
                        if let Some(arr) = mods.as_array() {
                            return arr.len();
                        }
                    }
                }
            }
        }
        0
    }
}
