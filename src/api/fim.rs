use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct FimRequest<'a> {
    model: &'a str,
    prompt_prefix: &'a str,
    prompt_suffix: &'a str,
}

#[derive(Deserialize)]
struct FimResponse {
    middle: String,
}

/// Calls the Mercury FIM (fill-in-the-middle) API and returns the generated middle text.
///
/// # Arguments
/// * `base_url` - Base URL of the Mercury service (e.g., "https://api.mercury.ai").
/// * `api_key` - API key for authentication.
/// * `prompt_prefix` - Text before the missing middle.
/// * `prompt_suffix` - Text after the missing middle.
///
/// # Returns
/// `Result<String, reqwest::Error>` containing the generated middle.
pub async fn fim(
    base_url: &str,
    api_key: &str,
    prompt_prefix: &str,
    prompt_suffix: &str,
) -> Result<String, reqwest::Error> {
    let client = Client::new();
    let url = format!("{}/v1/fim/completions", base_url.trim_end_matches('/'));
    let req = FimRequest {
        model: "mercury-edit",
        prompt_prefix,
        prompt_suffix,
    };
    let resp = client
        .post(&url)
        .bearer_auth(api_key)
        .json(&req)
        .send()
        .await?
        .error_for_status()?;
    let parsed: FimResponse = resp.json().await?;
    Ok(parsed.middle)
}
