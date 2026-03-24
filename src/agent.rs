use std::process::Command;
use std::io::{self, Write};

pub struct SubAgent {
    pub name: String,
    pub role_description: String,
    pub allowed_tools: Vec<String>,
    pub pane_id: Option<String>,
    pub prompt: String,
}

impl SubAgent {
    pub fn new(name: &str, role_description: &str, allowed_tools: Vec<String>, prompt: &str) -> Self {
        Self {
            name: name.to_string(),
            role_description: role_description.to_string(),
            allowed_tools,
            pane_id: None,
            prompt: prompt.to_string(),
        }
    }

    pub fn spawn(&mut self) -> io::Result<()> {
        // Use tmux split-window to create a new pane and run openmerc in headless mode
        let cmd = format!(
            "tmux split-window -P -F '#{{pane_id}}' \"openmerc --headless --prompt '{}'\"",
            self.prompt.replace('"', "\\\"")
        );
        let output = Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .output()?;
        if output.status.success() {
            let pane = String::from_utf8_lossy(&output.stdout).trim().to_string();
            self.pane_id = Some(pane);
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("tmux error: {}", String::from_utf8_lossy(&output.stderr)),
            ))
        }
    }

    pub fn collect_result(&self) -> io::Result<String> {
        let pane = match &self.pane_id {
            Some(p) => p,
            None => return Err(io::Error::new(io::ErrorKind::Other, "Pane not spawned")),
        };
        let cmd = format!("tmux capture-pane -p -t {}", pane);
        let output = Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .output()?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("capture error: {}", String::from_utf8_lossy(&output.stderr)),
            ))
        }
    }
}
