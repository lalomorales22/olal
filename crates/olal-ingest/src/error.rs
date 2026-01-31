//! Error types for the ingestion pipeline.

use std::path::PathBuf;
use thiserror::Error;

/// Result type for ingestion operations.
pub type IngestResult<T> = Result<T, IngestError>;

/// Errors that can occur during ingestion.
#[derive(Error, Debug)]
pub enum IngestError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] olal_db::DbError),

    #[error("Config error: {0}")]
    Config(#[from] olal_config::ConfigError),

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),

    #[error("Parse error for {path}: {message}")]
    ParseError { path: PathBuf, message: String },

    #[error("Watch error: {0}")]
    WatchError(String),

    #[error("File already processed: {0}")]
    AlreadyProcessed(PathBuf),

    #[error("Processing error: {0}")]
    ProcessingError(String),
}
