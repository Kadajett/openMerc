use std::fmt;
use std::io;
use std::error::Error;

/// Central error type for the application.
#[derive(Debug)]
pub enum AppError {
    Io(io::Error),
    Api(String),
    Config(String),
    Tool(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "IO error: {}", e),
            AppError::Api(msg) => write!(f, "API error: {}", msg),
            AppError::Config(msg) => write!(f, "Config error: {}", msg),
            AppError::Tool(msg) => write!(f, "Tool error: {}", msg),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            AppError::Io(e) => Some(e),
            _ => None,
        }
    }
}

// Optional From implementations for convenience
impl From<io::Error> for AppError {
    fn from(e: io::Error) -> Self {
        AppError::Io(e)
    }
}
