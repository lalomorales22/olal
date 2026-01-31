//! Audio file parser with transcription support.

use super::ParsedDocument;
use crate::error::{IngestError, IngestResult};
use olal_process::{transcribe_audio, TranscriptSegment};
use std::path::Path;
use tempfile::tempdir;
use tracing::info;

/// Parser for audio files.
/// Transcribes directly using Whisper (no audio extraction needed).
pub struct AudioParser {
    /// Whisper model to use (tiny, base, small, medium, large)
    whisper_model: String,
}

impl AudioParser {
    /// Create a new audio parser with the specified Whisper model.
    pub fn new(whisper_model: impl Into<String>) -> Self {
        Self {
            whisper_model: whisper_model.into(),
        }
    }

    /// Create an audio parser with the default model (base).
    pub fn with_default_model() -> Self {
        Self::new("base")
    }

    /// Parse an audio file by transcribing it.
    pub fn parse(&self, path: &Path) -> IngestResult<AudioParseResult> {
        if !path.exists() {
            return Err(IngestError::FileNotFound(path.to_path_buf()));
        }

        info!("Processing audio: {:?}", path);

        // Create temp directory for processing
        let temp_dir = tempdir().map_err(|e| {
            IngestError::ProcessingError(format!("Failed to create temp directory: {}", e))
        })?;

        // Transcribe directly (file is already audio)
        info!("Transcribing with Whisper ({})...", self.whisper_model);
        let segments = transcribe_audio(path, &self.whisper_model, temp_dir.path()).map_err(|e| {
            IngestError::ProcessingError(format!("Failed to transcribe: {}", e))
        })?;

        info!("Transcribed {} segments", segments.len());

        // Build content from segments
        let content = segments
            .iter()
            .map(|s| s.text.clone())
            .collect::<Vec<_>>()
            .join(" ");

        // Calculate duration from segments
        let duration = segments
            .last()
            .map(|s| s.end)
            .unwrap_or(0.0);

        // Use filename as title
        let title = path
            .file_stem()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());

        let metadata = serde_json::json!({
            "format": "audio",
            "duration": duration,
            "segment_count": segments.len(),
            "whisper_model": self.whisper_model,
        });

        let mut doc = ParsedDocument::new(&content).with_metadata(metadata);

        if let Some(t) = title {
            doc = doc.with_title(t);
        }

        Ok(AudioParseResult {
            document: doc,
            segments,
        })
    }

    /// Check if audio processing tools are available.
    pub fn tools_available() -> ToolAvailability {
        let whisper = which::which("whisper").is_ok();

        ToolAvailability { whisper }
    }
}

/// Result of parsing an audio file.
pub struct AudioParseResult {
    /// The parsed document (content + metadata).
    pub document: ParsedDocument,
    /// Transcript segments with timestamps.
    pub segments: Vec<TranscriptSegment>,
}

/// Availability of required audio processing tools.
#[derive(Debug)]
pub struct ToolAvailability {
    pub whisper: bool,
}

impl ToolAvailability {
    /// Check if all required tools are available.
    pub fn all_available(&self) -> bool {
        self.whisper
    }

    /// Get a message describing missing tools.
    pub fn missing_message(&self) -> Option<String> {
        if self.whisper {
            None
        } else {
            Some(
                "Missing tool: whisper. Install with:\n  pip install openai-whisper".to_string(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_availability() {
        let avail = AudioParser::tools_available();
        // Just verify it doesn't panic
        let _ = avail.all_available();
        let _ = avail.missing_message();
    }
}
