//! Error types for media processing.

use std::path::PathBuf;
use thiserror::Error;

/// Result type for processing operations.
pub type ProcessResult<T> = Result<T, ProcessError>;

/// Errors that can occur during media processing.
#[derive(Error, Debug)]
pub enum ProcessError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Tool not found: {tool}. Please install it.")]
    ToolNotFound { tool: String },

    #[error("FFmpeg error: {0}")]
    FfmpegError(String),

    #[error("Transcription error: {0}")]
    TranscriptionError(String),

    #[error("OCR error: {0}")]
    OcrError(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Process failed with exit code {code}: {stderr}")]
    ProcessFailed { code: i32, stderr: String },

    #[error("Parse error: {0}")]
    ParseError(String),
}
