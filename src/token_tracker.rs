//! Token tracking utilities.

/// Tracks token usage for a request.
#[derive(Debug, Default)]
pub struct TokenTracker {
    pub total_tokens: u32,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

impl TokenTracker {
    /// Creates a new, empty tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds token counts to the tracker.
    pub fn add(&mut self, total: u32, prompt: u32, completion: u32) {
        self.total_tokens = self.total_tokens.saturating_add(total);
        self.prompt_tokens = self.prompt_tokens.saturating_add(prompt);
        self.completion_tokens = self.completion_tokens.saturating_add(completion);
    }
}
