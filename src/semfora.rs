use anyhow::{Result, Context};
use std::path::Path;
use std::process::Command;
use serde_json::Value;

/// Run a `semfora-engine query` subcommand and return its stdout as a string.
/// `subcmd` is one of the query sub‑commands (e.g. "overview", "module", "file").
/// `args` are the arguments passed to that sub‑command.
pub fn query(subcmd: &str, args: &[&str]) -> Result<String> {
    let mut cmd = Command::new("semfora-engine");
    cmd.arg("query").arg(subcmd);
    for a in args {
        cmd.arg(a);
    }
    cmd.arg("--format").arg("json");
    let output = cmd.output().with_context(|| "Failed to execute semfora-engine query")?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("semfora query failed: {}", err);
    }
    let out = String::from_utf8(output.stdout).with_context(|| "Non‑UTF8 output from semfora query")?;
    Ok(out)
}

/// Provide a simple advisory for a file path based on the repository overview.
/// Returns `None` if no matching module is found.
pub fn advise_path(_workspace: &Path, path: &str) -> Result<Option<String>> {
    // The overview contains a "top_risk_modules" array with objects {name, files, risk}.
    let overview = query("overview", &[])?;
    let v: Value = serde_json::from_str(&overview)?;
    if let Some(modules) = v.get("top_risk_modules") {
        if let Some(arr) = modules.as_array() {
            for module in arr {
                if let (Some(name), Some(risk)) = (module.get("name"), module.get("risk")) {
                    if let Some(name_str) = name.as_str() {
                        // Direct match on module name in path.
                        if path.contains(name_str) {
                            return Ok(Some(format!("Semfora warns: module '{}' has risk level '{}'.", name_str, risk)));
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}

/// Progressive analysis: try to identify the module associated with a target path.
/// Returns the module name if found.
pub fn progressive_analyze(_workspace: &Path, target_path: &str) -> Result<Option<String>> {
    // First, quick advisory which already extracts the module name.
    if let Some(advice) = advise_path(Path::new("."), target_path)? {
        // Advisory format: "Semfora warns: module '<name>' has risk level '<risk>'."
        if let Some(start) = advice.find('\'') {
            if let Some(end) = advice[start + 1..].find('\'') {
                let name = &advice[start + 1..start + 1 + end];
                return Ok(Some(name.to_string()));
            }
        }
    }
    // Fallback: re‑query overview and search manually.
    let overview = query("overview", &[])?;
    let v: Value = serde_json::from_str(&overview)?;
    if let Some(modules) = v.get("top_risk_modules") {
        if let Some(arr) = modules.as_array() {
            for module in arr {
                if let Some(name) = module.get("name") {
                    if let Some(name_str) = name.as_str() {
                        if target_path.contains(name_str) {
                            return Ok(Some(name_str.to_string()));
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}

/// After a change, run `semfora-engine validate` to perform a quality audit.
/// Returns the raw JSON output.
pub fn check_dead_code(_workspace: &Path) -> Result<String> {
    let mut cmd = Command::new("semfora-engine");
    cmd.arg("validate").arg("--format").arg("json");
    let output = cmd.output().with_context(|| "Failed to run semfora validate")?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("semfora validate failed: {}", err);
    }
    let out = String::from_utf8(output.stdout).with_context(|| "Non‑UTF8 output from semfora validate")?;
    Ok(out)
}
