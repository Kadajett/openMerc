use chrono::Utc;
use uuid::Uuid;

use crate::app::{Task, TaskStatus};

/// Create a new task with optional priority, dependencies, parent, and estimate
pub fn create_task(
    tasks: &mut Vec<Task>,
    title: &str,
    description: Option<&str>,
    priority: Option<u8>,
    depends_on: Option<Vec<String>>,
    parent_id: Option<&str>,
    estimated_rounds: Option<u16>,
) -> String {
    let id = Uuid::new_v4().to_string()[..8].to_string();
    let task = Task {
        id: id.clone(),
        title: title.to_string(),
        status: TaskStatus::Pending,
        description: description.map(|s| s.to_string()),
        priority: priority.unwrap_or(3),
        depends_on: depends_on.unwrap_or_default(),
        parent_id: parent_id.map(|s| s.to_string()),
        estimated_rounds,
        actual_rounds: 0,
        notes: Vec::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        completed_at: None,
    };
    tasks.push(task);
    format!("Created task {id}: {title} (P{})", priority.unwrap_or(3))
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
        if new_status == TaskStatus::Completed {
            task.completed_at = Some(Utc::now());
        }
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
        // Auto-complete parent if all siblings done
        let parent_check = task.parent_id.clone();
        let result = format!("Task {id} updated: {}", changes.join(", "));

        if let Some(parent_id) = parent_check {
            let all_siblings_done = tasks.iter()
                .filter(|t| t.parent_id.as_deref() == Some(&parent_id))
                .all(|t| t.status == TaskStatus::Completed);
            if all_siblings_done {
                if let Some(parent) = tasks.iter_mut().find(|t| t.id == parent_id) {
                    parent.status = TaskStatus::Completed;
                    parent.completed_at = Some(Utc::now());
                    return format!("{result}\nParent task {parent_id} auto-completed (all subtasks done)");
                }
            }
        }

        result
    }
}

/// Append a note to a task (survives context compaction)
pub fn add_task_note(tasks: &mut Vec<Task>, id: &str, note: &str) -> String {
    let Some(task) = tasks.iter_mut().find(|t| t.id == id) else {
        return format!("Task {id} not found");
    };
    task.notes.push(note.to_string());
    task.updated_at = Utc::now();
    format!("Note added to task {id}")
}

/// List all tasks as a formatted string
pub fn list_tasks(tasks: &[Task]) -> String {
    if tasks.is_empty() {
        return "No tasks.".to_string();
    }

    let done = tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();
    let mut lines = vec![format!("Progress: {done}/{} tasks completed\n", tasks.len())];

    for task in tasks {
        let icon = match task.status {
            TaskStatus::Completed => "✓",
            TaskStatus::InProgress => "→",
            TaskStatus::Blocked => "✗",
            TaskStatus::Pending => "○",
        };
        let desc = task.description.as_deref().unwrap_or("");
        let desc_part = if desc.is_empty() { String::new() } else { format!(" — {desc}") };
        let deps = if task.depends_on.is_empty() { String::new() } else {
            format!(" [deps: {}]", task.depends_on.join(","))
        };
        let priority = format!("P{}", task.priority);
        lines.push(format!("{icon} [{}] {} ({}) {priority}{deps}{desc_part}", task.id, task.title, task.status));
    }
    lines.join("\n")
}
