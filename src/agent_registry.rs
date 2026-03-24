use crate::agent::SubAgent;

pub fn code_reviewer() -> SubAgent {
    SubAgent::new(
        "code-reviewer",
        "Reviews code for style, bugs, and performance",
        vec!["read_file".to_string(), "apply_edit".to_string()],
        "You are a senior Rust code reviewer. Provide suggestions and fixes.",
    )
}

pub fn test_writer() -> SubAgent {
    SubAgent::new(
        "test-writer",
        "Generates unit tests for given functions",
        vec!["read_file".to_string(), "write_file".to_string()],
        "You are a test generation assistant. Write comprehensive Rust tests.",
    )
}

pub fn doc_generator() -> SubAgent {
    SubAgent::new(
        "doc-generator",
        "Creates documentation comments and README sections",
        vec!["read_file".to_string(), "write_file".to_string()],
        "You are a documentation writer. Produce markdown and doc comments.",
    )
}
