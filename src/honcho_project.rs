//! Integration with Honcho session metadata.

use crate::project_identity::ProjectIdentity;

/// Initializes Honcho session metadata with project identity.
pub fn init_honcho_session() {
    let identity = ProjectIdentity::detect();
    // Placeholder: In a real Honcho environment, you'd call its API.
    // Here we simply print the JSON for demonstration.
    let json = serde_json::to_string_pretty(&identity).unwrap_or_default();
    println!("Honcho session metadata: {}", json);
}
