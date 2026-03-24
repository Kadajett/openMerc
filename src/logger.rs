use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use chrono::Utc;

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Initialize the logger with the workspace path
pub fn init(workspace: &Path) {
    let path = workspace.join(".openmerc").join("debug.log");
    // Ensure dir exists
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // Truncate on startup
    let _ = std::fs::write(&path, format!("=== openMerc session started {} ===\n", Utc::now()));
    let _ = LOG_PATH.set(path);
}

/// Log a message to the debug file
pub fn log(category: &str, message: &str) {
    let Some(path) = LOG_PATH.get() else { return };
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else { return };
    let ts = Utc::now().format("%H:%M:%S%.3f");
    let _ = writeln!(file, "[{ts}] [{category}] {message}");
}

/// Log an API request
pub fn log_api_request(url: &str, body: &str) {
    let truncated = if body.len() > 2000 {
        format!("{}... ({} bytes total)", &body[..2000], body.len())
    } else {
        body.to_string()
    };
    log("API_REQ", &format!("POST {url}\n{truncated}"));
}

/// Log an API response
pub fn log_api_response(status: u16, body: &str) {
    let truncated = if body.len() > 2000 {
        format!("{}... ({} bytes total)", &body[..2000], body.len())
    } else {
        body.to_string()
    };
    log("API_RES", &format!("status={status}\n{truncated}"));
}

/// Log an event
pub fn log_event(event: &str) {
    log("EVENT", event);
}

/// Log a tool call
pub fn log_tool(name: &str, args: &str, result: &str) {
    let truncated_result = if result.len() > 500 {
        format!("{}... ({} bytes)", &result[..500], result.len())
    } else {
        result.to_string()
    };
    log("TOOL", &format!("{name}({args}) → {truncated_result}"));
}
