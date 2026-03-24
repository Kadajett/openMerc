use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

/// Validates that a path is within the workspace root.
/// Returns the canonicalized path if valid.
pub fn resolve_sandboxed(workspace: &Path, requested: &str) -> Result<PathBuf> {
    let requested_path = if Path::new(requested).is_absolute() {
        PathBuf::from(requested)
    } else {
        workspace.join(requested)
    };

    let canonical = requested_path.canonicalize().unwrap_or(requested_path.clone());
    let workspace_canonical = workspace.canonicalize()?;

    if !canonical.starts_with(&workspace_canonical) {
        bail!(
            "Path escape denied: {} is outside workspace {}",
            canonical.display(),
            workspace_canonical.display()
        );
    }

    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn allows_paths_within_workspace() {
        let ws = env::current_dir().unwrap();
        let result = resolve_sandboxed(&ws, "Cargo.toml");
        assert!(result.is_ok());
    }

    #[test]
    fn blocks_path_escape() {
        let ws = PathBuf::from("/tmp/fake_workspace");
        let result = resolve_sandboxed(&ws, "/etc/passwd");
        assert!(result.is_err());
    }
}
