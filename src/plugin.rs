// src/plugin.rs
use std::fs;
use std::path::{Path, PathBuf};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub commands: Vec<String>,
}

pub trait Plugin {
    fn manifest(&self) -> &PluginManifest;
    fn execute(&self, command: &str, args: &[String]) -> Result<String, String>;
}

/// Scan the `.openmerc/plugins/` directory for `*.toml` manifest files and return the parsed
/// `PluginManifest`s. Errors are logged and ignored – a malformed manifest does not stop the whole
/// loading process.
pub fn load_plugins() -> Vec<PluginManifest> {
    let plugins_dir = Path::new(".openmerc/plugins");
    let mut manifests = Vec::new();
    if !plugins_dir.is_dir() {
        eprintln!("Plugins directory {:?} does not exist", plugins_dir);
        return manifests;
    }
    let entries = match fs::read_dir(plugins_dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to read plugins directory: {}", e);
            return manifests;
        }
    };
    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                match fs::read_to_string(&path) {
                    Ok(contents) => {
                        match toml::from_str::<PluginManifest>(&contents) {
                            Ok(manifest) => manifests.push(manifest),
                            Err(e) => eprintln!("Failed to parse manifest {:?}: {}", path, e),
                        }
                    }
                    Err(e) => eprintln!("Failed to read manifest {:?}: {}", path, e),
                }
            }
        }
    }
    manifests
}
