// src/prompt_templates.rs
use std::collections::HashMap;

lazy_static::lazy_static! {
    static ref TEMPLATES: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("code_review", "You are a senior engineer reviewing the following code. Identify bugs, style issues, and suggest improvements.");
        m.insert("refactor", "Refactor the given code to improve readability, performance, and maintainability while preserving behavior.");
        m.insert("explain", "Explain the purpose and logic of the following code snippet in clear, concise language.");
        m.insert("test_generation", "Generate unit tests for the given Rust function, covering edge cases and typical usage.");
        m
    };
}

/// Returns the prompt template for the given name, or None if not found.
pub fn get_template(name: &str) -> Option<&'static str> {
    TEMPLATES.get(name).copied()
}
