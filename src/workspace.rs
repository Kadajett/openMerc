// src/workspace.rs
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;
use serde::Deserialize;

#[derive(Debug)]
pub struct WorkspaceInfo {
    pub git_root: Option<PathBuf>,
    pub project_name: Option<String>,
    pub language: Option<String>,
    pub dependency_count: usize,
}

/// Detect the git repository root if any.
fn detect_git_root() -> Option<PathBuf> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Some(PathBuf::from(path_str))
}

/// Infer project name from git root or current directory.
fn infer_project_name(git_root: &Option<PathBuf>) -> Option<String> {
    git_root.as_ref().and_then(|p| p.file_name()).and_then(|os| os.to_str()).map(|s| s.to_owned())
}

/// Detect primary language based on file extensions present in the workspace.
fn detect_language(root: &Path) -> Option<String> {
    let mut extensions = std::collections::HashSet::new();
    let walk = walkdir::WalkDir::new(root).max_depth(3);
    for entry in walk {
        if let Ok(entry) = entry {
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension().and_then(|s| s.to_str()) {
                    extensions.insert(ext.to_ascii_lowercase());
                }
            }
        }
    }
    // Simple heuristic
    if extensions.contains("rs") {
        Some("Rust".to_string())
    } else if extensions.contains("ts") || extensions.contains("js") {
        Some("JavaScript/TypeScript".to_string())
    } else if extensions.contains("py") {
        Some("Python".to_string())
    } else if extensions.contains("go") {
        Some("Go".to_string())
    } else {
        None
    }
}

/// Count dependencies from Cargo.toml or package.json if present.
fn count_dependencies(root: &Path) -> usize {
    // Cargo
    let cargo_path = root.join("Cargo.toml");
    if cargo_path.is_file() {
        #[derive(Deserialize)]
        struct CargoToml { dependencies: Option<std::collections::HashMap<String, toml::Value>> }
        if let Ok(content) = fs::read_to_string(&cargo_path) {
            if let Ok(toml) = toml::from_str::<CargoToml>(&content) {
                return toml.dependencies.map_or(0, |d| d.len());
            }
        }
    }
    // package.json
    let pkg_path = root.join("package.json");
    if pkg_path.is_file() {
        #[derive(Deserialize)]
        struct PackageJson { dependencies: Option<std::collections::HashMap<String, String>> }
        if let Ok(content) = fs::read_to_string(&pkg_path) {
            if let Ok(json) = serde_json::from_str::<PackageJson>(&content) {
                return json.dependencies.map_or(0, |d| d.len());
            }
        }
    }
    0
}

/// Consolidated workspace detection.
pub fn detect_workspace() -> WorkspaceInfo {
    let git_root = detect_git_root();
    let project_name = infer_project_name(&git_root);
    let root = git_root.as_ref().map(|p| p.as_path()).unwrap_or_else(|| Path::new("."));
    let language = detect_language(root);
    let dependency_count = count_dependencies(root);
    WorkspaceInfo {
        git_root,
        project_name,
        language,
        dependency_count,
    }
}
