use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Scope level for a context file
#[derive(Debug, Clone)]
pub enum ContextScope {
    Global,   // ~/.merc/
    Project,  // .merc/ at project root
    Branch,   // .merc/git/{branch}.md
}

/// A single loaded context file
#[derive(Debug, Clone)]
pub struct ContextFile {
    pub path: PathBuf,
    pub content: String,
    pub scope: ContextScope,
}

/// Project context loaded from cascading .merc/ folders
#[derive(Debug, Clone)]
pub struct ProjectContext {
    pub project_id: String,
    pub git_remote: Option<String>,
    pub git_branch: Option<String>,
    pub project_name: String,
    pub context_files: Vec<ContextFile>,
    pub merged_context: String,
}

/// Load project context from cascading .merc/ folders
pub fn load_project_context(workspace: &Path) -> ProjectContext {
    let git_remote = git_remote_url(workspace);
    let git_branch = git_branch_name(workspace);
    let project_name = detect_project_name(workspace);
    let project_id = make_project_id(&git_remote, workspace);

    let mut files: Vec<ContextFile> = Vec::new();

    // 1. Global: ~/.merc/CONTEXT.md
    if let Some(home) = dirs::home_dir() {
        let global_ctx = home.join(".merc").join("CONTEXT.md");
        if global_ctx.exists() {
            if let Ok(content) = std::fs::read_to_string(&global_ctx) {
                files.push(ContextFile {
                    path: global_ctx,
                    content,
                    scope: ContextScope::Global,
                });
            }
        }
    }

    // 2. Project: .merc/CONTEXT.md
    let project_merc = workspace.join(".merc");
    if project_merc.exists() {
        for name in &["CONTEXT.md", "RULES.md", "PATTERNS.md"] {
            let path = project_merc.join(name);
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    files.push(ContextFile {
                        path,
                        content,
                        scope: ContextScope::Project,
                    });
                }
            }
        }
    }

    // 3. Branch: .merc/git/{branch}.md
    if let Some(branch) = &git_branch {
        let branch_file = project_merc.join("git").join(format!("{branch}.md"));
        if branch_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&branch_file) {
                files.push(ContextFile {
                    path: branch_file,
                    content,
                    scope: ContextScope::Branch,
                });
            }
        }
    }

    // 4. Also load CLAUDE.md if it exists (compatibility)
    let claude_md = workspace.join("CLAUDE.md");
    if claude_md.exists() {
        if let Ok(content) = std::fs::read_to_string(&claude_md) {
            files.push(ContextFile {
                path: claude_md,
                content,
                scope: ContextScope::Project,
            });
        }
    }

    // Merge: branch > project > global (highest priority last so it's appended last)
    let merged = files.iter()
        .map(|f| {
            let label = match f.scope {
                ContextScope::Global => "Global",
                ContextScope::Project => "Project",
                ContextScope::Branch => "Branch",
            };
            format!("### {} ({})\n{}", f.path.file_name().unwrap_or_default().to_string_lossy(), label, f.content)
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    crate::logger::log("CONTEXT", &format!(
        "Loaded {} context files for project '{}' (id={})",
        files.len(), project_name, project_id
    ));

    ProjectContext {
        project_id,
        git_remote,
        git_branch,
        project_name,
        context_files: files,
        merged_context: merged,
    }
}

fn git_remote_url(workspace: &Path) -> Option<String> {
    Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(workspace)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn git_branch_name(workspace: &Path) -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(workspace)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn detect_project_name(workspace: &Path) -> String {
    // Try Cargo.toml
    let cargo = workspace.join("Cargo.toml");
    if cargo.exists() {
        if let Ok(content) = std::fs::read_to_string(&cargo) {
            for line in content.lines() {
                if line.starts_with("name") {
                    if let Some(name) = line.split('=').nth(1) {
                        return name.trim().trim_matches('"').to_string();
                    }
                }
            }
        }
    }
    // Try package.json
    let pkg = workspace.join("package.json");
    if pkg.exists() {
        if let Ok(content) = std::fs::read_to_string(&pkg) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(name) = v["name"].as_str() {
                    return name.to_string();
                }
            }
        }
    }
    // Fallback to directory name
    workspace.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn make_project_id(git_remote: &Option<String>, workspace: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    if let Some(remote) = git_remote {
        remote.hash(&mut hasher);
    } else {
        workspace.display().to_string().hash(&mut hasher);
    }
    format!("{:x}", hasher.finish())[..12].to_string()
}
