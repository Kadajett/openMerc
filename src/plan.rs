use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::app::{Task, TaskStatus};
use crate::progress::ProgressTracker;

/// The lifecycle phase of a plan
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanPhase {
    Planning,
    Executing,
    Reviewing,
    Generating,
    Paused,
    Completed,
}

impl std::fmt::Display for PlanPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanPhase::Planning => write!(f, "PLANNING"),
            PlanPhase::Executing => write!(f, "EXECUTING"),
            PlanPhase::Reviewing => write!(f, "REVIEWING"),
            PlanPhase::Generating => write!(f, "GENERATING"),
            PlanPhase::Paused => write!(f, "PAUSED"),
            PlanPhase::Completed => write!(f, "COMPLETED"),
        }
    }
}

/// A plan wrapping a task graph with lifecycle management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub title: String,
    pub original_prompt: String,
    pub phase: PlanPhase,
    pub tasks: Vec<Task>,
    pub cycle_count: u32,
    pub total_rounds_used: u32,
    pub total_rounds_budget: Option<u32>,
    pub generation: u32,
    pub max_generations: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub progress: ProgressTracker,
}

impl Plan {
    pub fn new(title: &str, original_prompt: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.to_string(),
            original_prompt: original_prompt.to_string(),
            phase: PlanPhase::Planning,
            tasks: Vec::new(),
            cycle_count: 0,
            total_rounds_used: 0,
            total_rounds_budget: Some(1000),
            generation: 0,
            max_generations: 10,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            progress: ProgressTracker::new(),
        }
    }

    /// Get tasks that are ready to execute (dependencies satisfied)
    pub fn ready_tasks(&self) -> Vec<&Task> {
        self.tasks.iter()
            .filter(|t| t.status == TaskStatus::Pending || t.status == TaskStatus::InProgress)
            .filter(|t| {
                t.depends_on.iter().all(|dep_id| {
                    self.tasks.iter().any(|d| d.id == *dep_id && d.status == TaskStatus::Completed)
                })
            })
            .collect()
    }

    /// Select the next batch of tasks, sorted by priority
    pub fn select_batch(&self, max_tasks: usize) -> Vec<&Task> {
        let mut ready = self.ready_tasks();
        ready.sort_by(|a, b| {
            let ip_order = matches!(b.status, TaskStatus::InProgress)
                .cmp(&matches!(a.status, TaskStatus::InProgress));
            ip_order
                .then(a.priority.cmp(&b.priority))
                .then(a.estimated_rounds.cmp(&b.estimated_rounds))
        });
        ready.into_iter().take(max_tasks).collect()
    }

    pub fn all_done(&self) -> bool {
        self.tasks.iter().all(|t| t.status == TaskStatus::Completed || t.status == TaskStatus::Blocked)
    }

    pub fn completed_count(&self) -> usize {
        self.tasks.iter().filter(|t| t.status == TaskStatus::Completed).count()
    }

    pub fn over_budget(&self) -> bool {
        self.total_rounds_budget.map(|b| self.total_rounds_used >= b).unwrap_or(false)
    }

    pub fn can_generate(&self) -> bool {
        self.generation < self.max_generations && !self.over_budget()
    }

    pub fn pause(&mut self) {
        self.phase = PlanPhase::Paused;
        self.updated_at = Utc::now();
    }

    pub fn resume(&mut self) {
        if self.phase == PlanPhase::Paused {
            self.phase = PlanPhase::Executing;
            self.updated_at = Utc::now();
        }
    }
}
