use crate::agent::SubAgent;
use std::collections::HashMap;
use std::io;

pub struct Orchestrator {
    pub agents: HashMap<String, SubAgent>,
}

impl Orchestrator {
    pub fn new() -> Self {
        Self { agents: HashMap::new() }
    }

    pub fn register_agent(&mut self, id: &str, agent: SubAgent) {
        self.agents.insert(id.to_string(), agent);
    }

    pub fn dispatch(&mut self, task: &str) -> io::Result<String> {
        // Very naive split: split by "---" delimiter into subtasks
        let subtasks: Vec<&str> = task.split("---").map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        let mut results = Vec::new();
        for (i, sub) in subtasks.iter().enumerate() {
            let agent_id = format!("agent_{}", i);
            // Use a generic SubAgent; in real code you'd pick based on subtask type
            let mut agent = SubAgent::new(
                &agent_id,
                "General purpose subtask executor",
                vec!["tmux".to_string(), "openmerc".to_string()],
                sub,
            );
            agent.spawn()?;
            let out = agent.collect_result()?;
            results.push(out);
        }
        Ok(results.join("\n---\n"))
    }
}
