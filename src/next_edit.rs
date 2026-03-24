// src/next_edit.rs
use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct EditRequest {
    pub recently_viewed_snippets: Vec<String>,
    pub current_file_content: String,
    pub code_to_edit: String, // region identifier or raw code
    pub edit_diff_history: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct EditResponse {
    pub diff: String,
    pub auto_commit: bool,
    pub message: Option<String>,
}

/// Very simple diff generator – in a real system you would use a proper diff library.
fn generate_diff(old: &str, new: &str) -> String {
    // Placeholder: just show new content for now.
    format!("--- old\n+++ new\n{}", new)
}

#[post("/v1/edit/completions")]
async fn edit_completions(req: web::Json<EditRequest>) -> impl Responder {
    // In a real implementation we would feed the request to the Mercury model.
    // Here we mock a transformation: replace the region with a comment.
    let transformed = format!("{}\n// edited region\n{}", 
        &req.current_file_content[..req.current_file_content.find(&req.code_to_edit).unwrap_or(0)],
        &req.current_file_content[req.current_file_content.find(&req.code_to_edit).unwrap_or(0) + req.code_to_edit.len()..]
    );
    let diff = generate_diff(&req.current_file_content, &transformed);
    let resp = EditResponse {
        diff,
        auto_commit: true,
        message: Some("Edit applied and auto‑committed".to_string()),
    };
    HttpResponse::Ok().json(resp)
}

pub async fn run_server() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(edit_completions)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
