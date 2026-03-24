use chrono::Utc;
use uuid::Uuid;

use crate::app::{Task, TaskStatus};

/// Create a new task, returns a confirmation string
pub fn create_task(tasks: &mut Vec<Task>, title: &str, description: Option<&str>) -> String {
    let task = Task {
        id: Uuid::new_v4().to_string()[..8].to_string(), // short ID
        title: title.to_string(),
        status: TaskStatus::Pending,
        description: description.map(|s| s.to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let id = task.id.clone();
    tasks.push(task);
    format!("Created task {id}: {title}")
}

/// Update a task's status/title/description
pub fn update_task(
    tasks: &mut Vec<Task>,
    id: &str,
    status: Option<&str>,
    title: Option<&str>,
    description: Option<&str>,
) -> String {
    let Some(task) = tasks.iter_mut().find(|t| t.id == id) else {
        return format!("Task {id} not found");
    };

    let mut changes = Vec::new();

    if let Some(s) = status {
        let new_status = match s {
            "pending" => TaskStatus::Pending,
            "in_progress" => TaskStatus::InProgress,
            "completed" => TaskStatus::Completed,
            "blocked" => TaskStatus::Blocked,
            _ => return format!("Invalid status: {s}. Use: pending, in_progress, completed, blocked"),
        };
        task.status = new_status;
        changes.push(format!("status → {s}"));
    }

    if let Some(t) = title {
        task.title = t.to_string();
        changes.push(format!("title → {t}"));
    }

    if let Some(d) = description {
        task.description = Some(d.to_string());
        changes.push("description updated".to_string());
    }

    task.updated_at = Utc::now();

    if changes.is_empty() {
        format!("Task {id}: no changes")
    } else {
        format!("Task {id} updated: {}", changes.join(", "))
    }
}

/// List all tasks as a formatted string
pub fn list_tasks(tasks: &[Task]) -> String {
    if tasks.is_empty() {
        return "No tasks.".to_string();
    }

    let mut lines = Vec::new();
    for task in tasks {
        let icon = match task.status {
            TaskStatus::Completed => "✓",
            TaskStatus::InProgress => "→",
            TaskStatus::Blocked => "✗",
            TaskStatus::Pending => "○",
        };
        let desc = task.description.as_deref().unwrap_or("");
        let desc_part = if desc.is_empty() { String::new() } else { format!(" — {desc}") };
        lines.push(format!("{icon} [{}] {} ({}){desc_part}", task.id, task.title, task.status));
    }
    lines.join("\n")
}
