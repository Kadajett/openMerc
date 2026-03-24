use std::error::Error;
use std::path::Path;
use std::process::Command;
use serde_json::Value;

/// Generates a guided tour of the codebase.
///
/// * `workspace_path` – Path to the root of the workspace.
///
/// Returns a formatted string where each module is listed with a one‑line description.
pub fn generate_tour(workspace_path: &Path) -> Result<String, Box<dyn Error>> {
    // Run semfora-engine to get a JSON overview of the workspace.
    let output = Command::new("semfora-engine")
        .arg("query")
        .arg("overview")
        .arg("--format")
        .arg("json")
        .arg(workspace_path)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "semfora-engine failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
_to.into());
    }
    let json: Value = serde_json::from_slice(&output.stdout)?;
    // Expect the JSON to contain a "modules" array with objects that have "path" and "description".
    let modules = json["modules"].as_array().ok_or("Missing modules array")?;
    let mut tour = String::new();
    tour.push_str("Guided Tour of the Codebase:\n\n");
    for module in modules {
        let path = module["path"].as_str().unwrap_or("<unknown>");
        let desc = module["description"].as_str().unwrap_or("No description");
        tour.push_str(&format!("- {}: {}\n", path, desc));
    }
    Ok(tour)
}
