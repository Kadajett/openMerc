use anyhow::Result;
use std::path::Path;

use super::sandbox::resolve_sandboxed;

/// Glob search within the workspace
pub fn glob_search(workspace: &Path, pattern: &str) -> Result<Vec<String>> {
    // Prepend workspace to pattern
    let full_pattern = workspace.join(pattern).to_string_lossy().to_string();
    let mut results = Vec::new();

    for entry in glob::glob(&full_pattern)? {
        match entry {
            Ok(path) => {
                // Return relative to workspace
                if let Ok(rel) = path.strip_prefix(workspace) {
                    results.push(rel.to_string_lossy().to_string());
                }
            }
            Err(_) => continue,
        }
    }

    Ok(results)
}

/// Simple text search within workspace files matching a glob
pub fn grep_search(workspace: &Path, pattern: &str, file_glob: &str) -> Result<Vec<GrepMatch>> {
    let files = glob_search(workspace, file_glob)?;
    let mut matches = Vec::new();

    for file_path in files {
        let resolved = resolve_sandboxed(workspace, &file_path)?;
        if let Ok(content) = std::fs::read_to_string(&resolved) {
            for (line_num, line) in content.lines().enumerate() {
                if line.contains(pattern) {
                    matches.push(GrepMatch {
                        file: file_path.clone(),
                        line_number: line_num + 1,
                        line: line.to_string(),
                    });
                }
            }
        }
    }

    Ok(matches)
}

#[derive(Debug, Clone)]
pub struct GrepMatch {
    pub file: String,
    pub line_number: usize,
    pub line: String,
}

impl std::fmt::Display for GrepMatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.file, self.line_number, self.line)
    }
}
