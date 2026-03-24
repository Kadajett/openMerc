// src/model_router.rs
// Routing logic to select appropriate Mercury model based on task description.

/// Returns the model name to use for a given task description.
/// - "mercury-2" for chat or reasoning tasks.
/// - "mercury-edit" for code editing tasks (apply_edit).
/// - "mercury-coder" for code generation tasks if available.
pub fn route_model(task_desc: &str) -> &'static str {
    let lower = task_desc.to_ascii_lowercase();
    if lower.contains("chat") || lower.contains("reason") || lower.contains("explain") {
        "mercury-2"
    } else if lower.contains("edit") || lower.contains("apply_edit") || lower.contains("refactor") {
        "mercury-edit"
    } else if lower.contains("generate") || lower.contains("code") || lower.contains("write") {
        "mercury-coder"
    } else {
        // Default to general model
        "mercury-2"
    }
}
