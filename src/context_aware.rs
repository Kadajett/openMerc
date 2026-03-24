// src/context_aware.rs
// Utility to check Honcho memory for past mistakes on a file before editing.

use crate::memory::search_memory; // Assuming a memory module exists.

/// Searches Honcho memory for entries related to the given filename.
/// Returns a vector of warning strings if past mistakes are found.
pub fn check_file_history(filename: &str) -> Vec<String> {
    // Query memory for any records mentioning the filename.
    // The memory system is expected to return a JSON string; we parse it.
    let query = format!("file:{}", filename);
    let result = search_memory(&query);
    // If no result or empty, return empty warnings.
    if result.is_empty() {
        return Vec::new();
    }
    // For simplicity, treat each line as a warning.
    result
        .lines()
        .map(|line| format!("Past issue: {}", line.trim()))
        .collect()
}
