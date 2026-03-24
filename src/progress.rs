use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Tracks progress across multiple auto-continue cycles
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProgressTracker {
    pub cycles: Vec<CycleRecord>,
}

/// Record of what happened in one auto-continue cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleRecord {
    pub cycle_number: u32,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub rounds_used: u32,
    pub tasks_started: Vec<String>,
    pub tasks_completed: Vec<String>,
    pub tasks_blocked: Vec<String>,
    pub files_modified: Vec<String>,
    pub summary: Option<String>,
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self { cycles: Vec::new() }
    }

    pub fn start_cycle(&mut self) -> u32 {
        let num = self.cycles.len() as u32 + 1;
        self.cycles.push(CycleRecord {
            cycle_number: num,
            started_at: Utc::now(),
            ended_at: None,
            rounds_used: 0,
            tasks_started: Vec::new(),
            tasks_completed: Vec::new(),
            tasks_blocked: Vec::new(),
            files_modified: Vec::new(),
            summary: None,
        });
        num
    }

    pub fn end_cycle(&mut self, rounds: u32, summary: Option<String>) {
        if let Some(cycle) = self.cycles.last_mut() {
            cycle.ended_at = Some(Utc::now());
            cycle.rounds_used = rounds;
            cycle.summary = summary;
        }
    }

    pub fn record_task_started(&mut self, task_id: &str) {
        if let Some(cycle) = self.cycles.last_mut() {
            cycle.tasks_started.push(task_id.to_string());
        }
    }

    pub fn record_task_completed(&mut self, task_id: &str) {
        if let Some(cycle) = self.cycles.last_mut() {
            cycle.tasks_completed.push(task_id.to_string());
        }
    }

    pub fn record_file_modified(&mut self, path: &str) {
        if let Some(cycle) = self.cycles.last_mut() {
            if !cycle.files_modified.contains(&path.to_string()) {
                cycle.files_modified.push(path.to_string());
            }
        }
    }

    pub fn total_rounds(&self) -> u32 {
        self.cycles.iter().map(|c| c.rounds_used).sum()
    }

    pub fn total_tasks_completed(&self) -> usize {
        self.cycles.iter().flat_map(|c| &c.tasks_completed).count()
    }

    /// Build a summary of recent cycles for context injection
    pub fn recent_summary(&self, last_n: usize) -> String {
        let recent: Vec<&CycleRecord> = self.cycles.iter().rev().take(last_n).collect();
        if recent.is_empty() {
            return String::new();
        }
        let mut lines = vec!["## Recent Progress".to_string()];
        for cycle in recent.iter().rev() {
            let completed = if cycle.tasks_completed.is_empty() {
                "none".to_string()
            } else {
                cycle.tasks_completed.join(", ")
            };
            let files = if cycle.files_modified.is_empty() {
                "none".to_string()
            } else {
                cycle.files_modified.join(", ")
            };
            let summary = cycle.summary.as_deref().unwrap_or("no summary");
            lines.push(format!(
                "Cycle {}: {} rounds, completed [{}], files [{}] — {}",
                cycle.cycle_number, cycle.rounds_used, completed, files, summary
            ));
        }
        lines.join("\n")
    }
}
