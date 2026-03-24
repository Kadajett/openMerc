// src/knowledge_graph.rs
// Query all Honcho conclusions and sessions to build a simple keyword map of project knowledge.

use std::collections::HashMap;

// Placeholder types – replace with actual Honcho data structures.
#[derive(Debug)]
struct Conclusion {
    text: String,
    tags: Vec<String>,
}

#[derive(Debug)]
struct Session {
    id: String,
    messages: Vec<String>,
}

// Stub functions to retrieve all conclusions and sessions.
fn get_all_conclusions() -> Vec<Conclusion> {
    vec![
        Conclusion { text: "Mercury diff LLMs output tokens in parallel".to_string(), tags: vec!["diffusion".to_string(), "LLM".to_string()] },
        Conclusion { text: "Project uses Kubernetes for home services".to_string(), tags: vec!["k8s".to_string(), "home".to_string()] },
    ]
}

fn get_all_sessions() -> Vec<Session> {
    vec![
        Session { id: "s1".to_string(), messages: vec!["Discussed token benchmarks".to_string()] },
        Session { id: "s2".to_string(), messages: vec!["Planned knowledge graph".to_string()] },
    ]
}

pub fn build_keyword_map() -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();

    // Index conclusions by tags.
    for c in get_all_conclusions() {
        for tag in c.tags {
            map.entry(tag.clone()).or_default().push(c.text.clone());
        }
    }

    // Index sessions by simple keyword extraction from messages.
    for s in get_all_sessions() {
        for msg in s.messages {
            // Very naive keyword split on spaces.
            for word in msg.split_whitespace() {
                let kw = word.to_ascii_lowercase();
                map.entry(kw).or_default().push(format!("session {}: {}", s.id, msg));
            }
        }
    }

    map
}
