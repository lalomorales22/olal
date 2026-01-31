//! Error types for Ollama operations.

use thiserror::Error;

/// Errors that can occur when interacting with Ollama.
#[derive(Error, Debug)]
pub enum OllamaError {
    /// Connection error - unable to reach Ollama server.
    #[error("Connection error: {0}")]
    Connection(String),

    /// Request timeout.
    #[error("Request timed out after {seconds} seconds")]
    Timeout { seconds: u64 },

    /// The requested model is not available.
    #[error("Model not found: {model}. Run 'ollama pull {model}' to download it.")]
    ModelNotFound { model: String },

    /// Ollama server is not running.
    #[error("Ollama server is not running at {host}. Start it with 'ollama serve'.")]
    ServerNotRunning { host: String },

    /// API returned an error response.
    #[error("API error (status {status}): {message}")]
    ApiError { status: u16, message: String },

    /// Failed to parse response.
    #[error("Failed to parse response: {0}")]
    ParseError(String),

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Embedding dimension mismatch.
    #[error("Embedding dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    /// No context available for RAG query.
    #[error("No relevant context found for the query")]
    NoContext,

    /// HTTP request error.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Result type for Ollama operations.
pub type OllamaResult<T> = Result<T, OllamaError>;
