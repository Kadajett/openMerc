use crate::app::Task;
use crate::plan::{Plan, PlanPhase};

/// What the engine wants the main loop to do next
pub enum EngineAction {
    /// Send this prompt to Mercury and continue
    Continue(String),
    /// Plan phase transition — notify user
    PhaseTransition(PlanPhase, String),
    /// Plan is complete
    Complete(String),
    /// Plan is paused, waiting for user
    Paused,
    /// Over budget, stop
    BudgetExhausted(String),
}

/// Configuration for the execution engine
pub struct EngineConfig {
    pub max_tasks_per_cycle: usize,
    pub auto_generate: bool,
    pub auto_review: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_tasks_per_cycle: 3,
            auto_generate: false,
            auto_review: true,
        }
    }
}

/// Decides what to do next based on plan state
pub fn next_action(plan: &mut Plan, config: &EngineConfig) -> EngineAction {
    if plan.phase == PlanPhase::Paused {
        return EngineAction::Paused;
    }

    if plan.over_budget() {
        return EngineAction::BudgetExhausted(format!(
            "Budget exhausted: {} rounds used of {} budget",
            plan.total_rounds_used,
            plan.total_rounds_budget.unwrap_or(0)
        ));
    }

    // Stall detection: if we've run 3+ cycles without completing any new tasks, stop
    if plan.cycle_count >= 3 {
        let recent_completed: usize = plan.progress.cycles.iter()
            .rev().take(3)
            .flat_map(|c| &c.tasks_completed)
            .count();
        if recent_completed == 0 && plan.completed_count() < plan.tasks.len() {
            plan.phase = PlanPhase::Paused;
            return EngineAction::Complete(format!(
                "Stalled: no tasks completed in last 3 cycles. {} tasks remain. Pausing for user input.",
                plan.tasks.len() - plan.completed_count()
            ));
        }
    }

    plan.cycle_count += 1;

    match &plan.phase {
        PlanPhase::Planning => {
            // Check if any tasks were created yet
            if plan.tasks.is_empty() {
                EngineAction::Continue(build_planning_prompt(plan))
            } else {
                // Tasks exist — transition to executing
                plan.phase = PlanPhase::Executing;
                EngineAction::PhaseTransition(
                    PlanPhase::Executing,
                    format!("{} tasks created. Starting execution.", plan.tasks.len()),
                )
            }
        }
        PlanPhase::Executing => {
            if plan.all_done() {
                if config.auto_review {
                    plan.phase = PlanPhase::Reviewing;
                    EngineAction::PhaseTransition(
                        PlanPhase::Reviewing,
                        format!("All {} tasks complete. Entering review.", plan.completed_count()),
                    )
                } else if config.auto_generate && plan.can_generate() {
                    plan.phase = PlanPhase::Generating;
                    EngineAction::PhaseTransition(
                        PlanPhase::Generating,
                        "All tasks complete. Generating new improvement tasks.".to_string(),
                    )
                } else {
                    plan.phase = PlanPhase::Completed;
                    EngineAction::Complete(build_completion_summary(plan))
                }
            } else {
                EngineAction::Continue(build_execution_prompt(plan, config))
            }
        }
        PlanPhase::Reviewing => {
            // After review, either generate or complete
            if config.auto_generate && plan.can_generate() {
                plan.phase = PlanPhase::Generating;
                EngineAction::Continue(build_review_prompt(plan))
            } else {
                plan.phase = PlanPhase::Completed;
                EngineAction::Complete(build_completion_summary(plan))
            }
        }
        PlanPhase::Generating => {
            let prev_count = plan.tasks.len();
            // If no new tasks were added in this generation cycle, stop
            // (checked by the caller after Mercury responds)
            plan.generation += 1;
            plan.phase = PlanPhase::Executing;
            EngineAction::Continue(build_generation_prompt(plan))
        }
        PlanPhase::Completed => {
            EngineAction::Complete(build_completion_summary(plan))
        }
        _ => EngineAction::Paused,
    }
}

fn build_planning_prompt(plan: &Plan) -> String {
    format!(
        r#"You are in PLANNING mode. The user asked: "{}"

Break this work into discrete tasks using create_task. For each task:
- Set a clear, specific title
- Include a description with acceptance criteria
- Set priority (1=highest, 5=lowest)
- Declare dependencies with depends_on if task B requires task A
- Estimate the tool rounds needed with estimated_rounds

When you have fully decomposed the work, the system will automatically move to execution.
Do NOT start executing tasks yet — only create them."#,
        plan.original_prompt
    )
}

fn build_execution_prompt(plan: &Plan, config: &EngineConfig) -> String {
    let batch = plan.select_batch(config.max_tasks_per_cycle);
    let focus_tasks: String = batch.iter()
        .map(|t| {
            let deps = if t.depends_on.is_empty() { String::new() } else {
                format!(" (deps: {})", t.depends_on.join(", "))
            };
            let last_note = t.notes.last().map(|n| format!("\n    Last note: {n}")).unwrap_or_default();
            format!("- [{}] {} (P{}, est {} rounds){}{}\n    {}",
                t.id, t.title, t.priority,
                t.estimated_rounds.unwrap_or(10),
                deps, last_note,
                t.description.as_deref().unwrap_or(""))
        })
        .collect::<Vec<_>>()
        .join("\n");

    let recent = plan.progress.recent_summary(2);
    let done = plan.completed_count();
    let total = plan.tasks.len();
    let existing_ids = plan.tasks.iter()
        .map(|t| format!("{}({})", t.id, t.title))
        .collect::<Vec<_>>()
        .join(", ");

    let mut prompt = String::new();
    prompt.push_str(&format!("You are in EXECUTION mode. Progress: {done}/{total} tasks complete. Cycle {}.\n\n", plan.cycle_count));
    prompt.push_str(&recent);
    prompt.push_str("\n\n## Focus tasks this cycle (work on these):\n");
    prompt.push_str(&focus_tasks);
    prompt.push_str(&format!("\n\nRules:\n\
        - DO NOT create duplicate tasks. Existing: {existing_ids}\n\
        - Mark each task in_progress when you start, completed when done.\n\
        - Use add_task_note to record what you did.\n\
        - If a write keeps failing cargo check, mark the task BLOCKED. Do not retry more than twice.\n\
        - After completing a task, move to the next.\n\
        - Auto-commit with git when you finish a batch."));
    prompt
}

fn build_review_prompt(plan: &Plan) -> String {
    let completed: String = plan.tasks.iter()
        .filter(|t| t.status == crate::app::TaskStatus::Completed)
        .map(|t| format!("- {} {}", t.id, t.title))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"You are in REVIEW mode. All tasks are complete.

## Completed tasks:
{completed}

Review steps:
1. Run cargo check to verify all code compiles.
2. Run cargo test if tests exist.
3. Review for obvious issues (dead code, missing error handling).
4. If you find problems, create new tasks with priority 1.
5. If everything looks good, confirm the review is complete."#
    )
}

fn build_generation_prompt(plan: &Plan) -> String {
    let summary = plan.progress.recent_summary(5);

    format!(
        r#"You are in GENERATION mode. Original request: "{}"

All tasks from generation {} are complete. {summary}

Analyze the codebase and create 3-5 new improvement tasks using create_task.
Consider: code quality, missing tests, documentation, performance, error handling.
Set priorities and dependencies. These will be generation {} tasks."#,
        plan.original_prompt,
        plan.generation,
        plan.generation + 1
    )
}

fn build_completion_summary(plan: &Plan) -> String {
    format!(
        "Plan '{}' completed.\n- Tasks: {}/{}\n- Cycles: {}\n- Total rounds: {}\n- Generations: {}",
        plan.title,
        plan.completed_count(),
        plan.tasks.len(),
        plan.cycle_count,
        plan.total_rounds_used,
        plan.generation
    )
}
