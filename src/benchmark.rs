// src/benchmark.rs
// Compare token usage across models by running the same prompt through
// `mercury-2` and `mercury-edit` and logging token counts.

use std::process::Command;
use std::fs::File;
use std::io::{Write, BufWriter};

fn run_model(cmd: &str, prompt: &str) -> usize {
    // Execute the model binary with the prompt and capture token count from stdout.
    // This is a placeholder: actual models should output token count in a parseable format.
    let output = Command::new(cmd)
        .arg("--prompt")
        .arg(prompt)
        .output()
        .expect("failed to execute model");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Expect a line like "tokens: <number>"
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("tokens:") {
            if let Ok(cnt) = rest.trim().parse::<usize>() {
                return cnt;
            }
        }
    }
    0
}

pub fn benchmark(prompt: &str) {
    let mercury2_tokens = run_model("mercury-2", prompt);
    let mercury_edit_tokens = run_model("mercury-edit", prompt);

    let file = File::create("benchmark.log").expect("cannot create log file");
    let mut writer = BufWriter::new(file);
    writeln!(writer, "Prompt: {}", prompt).unwrap();
    writeln!(writer, "mercury-2 tokens: {}", mercury2_tokens).unwrap();
    writeln!(writer, "mercury-edit tokens: {}", mercury_edit_tokens).unwrap();
    writeln!(writer, "Difference: {}", (mercury2_tokens as is64 - mercury_edit_tokens as i64).abs()).unwrap();
}
