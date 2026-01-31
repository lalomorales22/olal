//! Error types for Olal.

use thiserror::Error;

/// Core error type for Olal operations.
#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Item not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Processing error: {0}")]
    Processing(String),

    #[error("Ollama error: {0}")]
    Ollama(String),

    #[error("{0}")]
    Other(String),
}

/// Result type alias using Olal's Error.
pub type Result<T> = std::result::Result<T, Error>;

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}
