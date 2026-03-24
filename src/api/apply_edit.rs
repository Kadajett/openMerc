use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ApplyEditRequest<'a> {
    model: &'a str,
    original_code: &'a str,
    update_snippet: &'a str,
}

#[derive(Deserialize)]
struct ApplyEditResponse {
    merged: String,
}

/// Calls the Mercury Edit API and returns the merged code.
///
/// # Arguments
/// * `base_url` - Base URL of the Mercury service (e.g., "https://api.mercury.ai").
/// * `api_key` - API key for authentication.
/// * `original_code` - The original source code.
/// * `update_snippet` - The snippet to apply.
///
/// # Returns
/// `Result<String, reqwest::Error>` containing the merged code.
pub async fn apply_edit(
    base_url: &str,
    api_key: &str,
    original_code: &str,
    update_snippet: &str,
) -> Result<String, reqwest::Error> {
    let client = Client::new();
    let url = format!("{}/v1/apply/completions", base_url.trim_end_matches('/'));
    let req = ApplyEditRequest {
        model: "mercury-edit",
        original_code,
        update_snippet,
    };
    let resp = client
        .post(&url)
        .bearer_auth(api_key)
        .json(&req)
        .send()
        .await?
        .error_for_status()?;
    let parsed: ApplyEditResponse = resp.json().await?;
    Ok(parsed.merged)
}
