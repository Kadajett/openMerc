// src/metrics.rs
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use serde::{Serialize, Deserialize};
use serde_json::json;
use std::time::{Instant, Duration};

#[derive(Serialize, Deserialize, Default)]
pub struct Metrics {
    pub token_usage: u64,
    pub tool_calls: u64,
    pub total_response_time_ms: u128,
    pub reviews: u64,
}

impl Metrics {
    pub fn new() -> Self { Metrics::default() }
    pub fn record(&mut self, tokens: u64, tool_calls: u64, resp: Duration) {
        self.token_usage += tokens;
        self.tool_calls += tool_calls;
        self.total_response_time_ms += resp.as_millis();
        self.reviews += 1;
    }
    pub fn avg_response_time(&self) -> f64 {
        if self.reviews == 0 { 0.0 } else { self.total_response_time_ms as f64 / self.reviews as f64 }
    }
    pub fn write_to_file(&self) -> io::Result<()> {
        let dir = Path::new(".openmerc");
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }
        let path = dir.join("metrics.json");
        let file = OpenOptions::new().write(true).create(true).truncate(true).open(path)?;
        serde_json::to_writer_pretty(file, &self)?;
        Ok(())
    }
}
