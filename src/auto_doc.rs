// src/auto_doc.rs
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::io::{self, Write};

/// Walks all .rs files under src/, runs semfora-engine analyze on each,
/// extracts a brief purpose description, and writes a module-level doc map
/// to docs/architecture.md.
pub fn generate_docs() -> io::Result<()> {
    let src_dir = Path::new("src");
    let mut doc_map = Vec::new();
    for entry in walk_dir(src_dir)? {
        let path = entry?;
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            // Run semfora-engine analyze and capture JSON output
            let output = Command::new("semfora-engine")
                .arg("analyze")
                .arg(&path)
                .output()?;
            if !output.status.success() {
                eprintln!("semfora failed on {}: {}", path.display(), String::from_utf8_lossy(&output.stderr));
                continue;
            }
            let json = String::from_utf8_lossy(&output.stdout);
            // Very naive extraction: look for "purpose" field in JSON
            let purpose = json.lines()
                .find(|l| l.contains("\"purpose\":"))
                .and_then(|l| l.split(\":\").nth(1))
                .map(|s| s.trim().trim_matches(',').trim_matches('"'))
                .unwrap_or("<unknown>");
            doc_map.push((path.strip_prefix(src_dir).unwrap().to_path_buf(), purpose.to_string()));
        }
    }
    // Ensure docs/ exists
    fs::create_dir_all("docs")?;
    let mut file = fs::File::create("docs/architecture.md")?;
    writeln!(file, "# Architecture Documentation\n")?;
    for (mod_path, purpose) in doc_map {
        writeln!(file, "- `{}`: {}", mod_path.display(), purpose)?;
    }
    Ok(())
}

fn walk_dir(dir: &Path) -> io::Result<impl Iterator<Item = io::Result<PathBuf>>> {
    let entries = fs::read_dir(dir)?;
    let mut stack = vec![entries];
    let mut out = Vec::new();
    while let Some(mut iter) = stack.pop() {
        for entry in iter {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(fs::read_dir(path)?);
            } else {
                out.push(Ok(path));
            }
        }
    }
    Ok(out.into_iter())
}
