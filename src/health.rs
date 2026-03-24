use std::path::Path;
use std::process::Command;

/// Generate a project health summary
pub fn health_report(workspace: &Path) -> String {
    let mut lines = vec!["## Project Health".to_string()];

    // Semfora overview
    if let Ok(output) = Command::new("semfora-engine")
        .args(["query", "overview", "--format", "json"])
        .current_dir(workspace)
        .output()
    {
        if output.status.success() {
            let raw = String::from_utf8_lossy(&output.stdout);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                let files = v["total_files"].as_u64().unwrap_or(0);
                let modules = v["total_modules"].as_u64().unwrap_or(0);
                lines.push(format!("Files: {files} | Modules: {modules}"));
                if let Some(risk) = v["top_risk_modules"].as_array() {
                    let high: Vec<String> = risk.iter()
                        .filter(|m| m["risk"].as_str() == Some("high"))
                        .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                        .collect();
                    if !high.is_empty() {
                        lines.push(format!("High risk: {}", high.join(", ")));
                    }
                }
            }
        }
    }

    // Session count
    let session_dir = workspace.join(".openmerc").join("sessions");
    if session_dir.exists() {
        let count = std::fs::read_dir(&session_dir)
            .map(|d| d.filter(|e| e.as_ref().map(|e| e.path().extension().map(|x| x == "json").unwrap_or(false)).unwrap_or(false)).count())
            .unwrap_or(0);
        lines.push(format!("Sessions: {count}"));
    }

    // Git status
    if let Ok(output) = Command::new("git").args(["status", "--short"]).current_dir(workspace).output() {
        if output.status.success() {
            let changes = String::from_utf8_lossy(&output.stdout).lines().count();
            lines.push(format!("Uncommitted changes: {changes}"));
        }
    }

    lines.join("\n")
}
