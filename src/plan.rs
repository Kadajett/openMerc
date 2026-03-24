use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::app::{AppMode, Task};

/// Persistent state for a running plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanState {
    pub mode: AppMode,
    pub tasks: Vec<Task>,
    pub last_updated: DateTime<Utc>,
}

impl PlanState {
    pub fn new() -> Self {
        Self {
            mode: AppMode::Plan,
            tasks: Vec::new(),
            last_updated: Utc::now(),
        }
    }

    pub fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
        self.last_updated = Utc::now();
    }

    pub fn pause(&mut self) {
        self.mode = AppMode::Paused;
        self.last_updated = Utc::now();
    }

    pub fn resume(&mut self) {
        self.mode = AppMode::Plan;
        self.last_updated = Utc::now();
    }
}
