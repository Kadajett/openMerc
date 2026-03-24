// Smoke tests for openMerc core functionality
use std::fs;
use std::path::Path;
// Import library modules
use openmerc::config;
use openmerc::session;

#[test]
fn test_config_load() {
    // Setup a temporary workspace with a minimal .openmerc.toml
    let workspace = Path::new("tmp_test_workspace");
    fs::create_dir_all(workspace).unwrap();
    let config_path = workspace.join(".openmerc.toml");
    let toml_content = r#"
        [mercury]
        base_url = "http://localhost"
        api_key = "test"
        model = "test-model"
        max_tokens = 100
        
        [honcho]
        # empty for now
        
        [agent]
        # empty for now
    "#;
    fs::write(&config_path, toml_content).unwrap();

    // Load config using the library
    let cfg = config::Config::load(workspace).expect("config should load");
    assert_eq!(cfg.mercury.base_url, "http://localhost");
    // Cleanup
    let _ = fs::remove_dir_all(workspace);
}

#[test]
fn test_session_dir_creation() {
    let workspace = Path::new("tmp_test_workspace2");
    fs::create_dir_all(workspace).unwrap();

    // Ensure session directory is created
    session::ensure_session_dir(workspace).expect("should create session dir");
    let session_dir = workspace.join(".openmerc").join("sessions");
    assert!(session_dir.is_dir(), "session dir must exist");

    // Cleanup
    let _ = fs::remove_dir_all(workspace);
}
