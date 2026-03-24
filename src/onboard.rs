use std::error::Error;
use std::path::Path;
use crate::tour::generate_tour;

/// Generates an onboarding guide for a new developer.
///
/// It combines a guided tour of the codebase with relevant project context
/// retrieved from Honcho memory search.
pub fn generate_onboarding_guide(workspace_path: &Path) -> Result<String, Box<dyn Error>> {
    // Get the guided tour.
    let tour = generate_tour(workspace_path)?;

    // Retrieve project context from Honcho memory. This is a placeholder –
    // replace with the actual memory‑search API when available.
    // For now we simulate it with an empty string.
    let project_context = "";

    // Combine both parts.
    let guide = format!(
        "=== Onboarding Guide ===\n\n{}\n\nProject Context:\n{}\n",
        tour,
        project_context,
    );
    Ok(guide)
}
