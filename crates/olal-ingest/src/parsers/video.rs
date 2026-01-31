//! Video file parser with transcription support.

use super::ParsedDocument;
use crate::error::{IngestError, IngestResult};
use olal_process::{extract_audio, get_video_info, transcribe_audio, TranscriptSegment};
use std::path::Path;
use tempfile::tempdir;
use tracing::{debug, info};

/// Parser for video files.
/// Extracts audio and transcribes using Whisper.
pub struct VideoParser {
    /// Whisper model to use (tiny, base, small, medium, large)
    whisper_model: String,
}

impl VideoParser {
    /// Create a new video parser with the specified Whisper model.
    pub fn new(whisper_model: impl Into<String>) -> Self {
        Self {
            whisper_model: whisper_model.into(),
        }
    }

    /// Create a video parser with the default model (base).
    pub fn with_default_model() -> Self {
        Self::new("base")
    }

    /// Parse a video file by extracting audio and transcribing.
    pub fn parse(&self, path: &Path) -> IngestResult<VideoParseResult> {
        if !path.exists() {
            return Err(IngestError::FileNotFound(path.to_path_buf()));
        }

        info!("Processing video: {:?}", path);

        // Get video info
        let video_info = get_video_info(path).map_err(|e| {
            IngestError::ProcessingError(format!("Failed to get video info: {}", e))
        })?;

        debug!(
            "Video info: {}x{}, {:.1}s duration",
            video_info.width, video_info.height, video_info.duration
        );

        // Create temp directory for processing
        let temp_dir = tempdir().map_err(|e| {
            IngestError::ProcessingError(format!("Failed to create temp directory: {}", e))
        })?;

        // Extract audio
        info!("Extracting audio...");
        let audio_path = extract_audio(path, temp_dir.path()).map_err(|e| {
            IngestError::ProcessingError(format!("Failed to extract audio: {}", e))
        })?;

        // Transcribe
        info!("Transcribing with Whisper ({})...", self.whisper_model);
        let segments = transcribe_audio(&audio_path, &self.whisper_model, temp_dir.path())
            .map_err(|e| {
                IngestError::ProcessingError(format!("Failed to transcribe: {}", e))
            })?;

        info!("Transcribed {} segments", segments.len());

        // Build content from segments
        let content = segments
            .iter()
            .map(|s| s.text.clone())
            .collect::<Vec<_>>()
            .join(" ");

        // Use filename as title
        let title = path
            .file_stem()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());

        let metadata = serde_json::json!({
            "format": "video",
            "duration": video_info.duration,
            "width": video_info.width,
            "height": video_info.height,
            "video_codec": video_info.video_codec,
            "audio_codec": video_info.audio_codec,
            "fps": video_info.fps,
            "segment_count": segments.len(),
            "whisper_model": self.whisper_model,
        });

        let mut doc = ParsedDocument::new(&content).with_metadata(metadata);

        if let Some(t) = title {
            doc = doc.with_title(t);
        }

        Ok(VideoParseResult {
            document: doc,
            segments,
        })
    }

    /// Check if video processing tools are available.
    pub fn tools_available() -> ToolAvailability {
        let ffmpeg = which::which("ffmpeg").is_ok();
        let ffprobe = which::which("ffprobe").is_ok();
        let whisper = which::which("whisper").is_ok();

        ToolAvailability {
            ffmpeg,
            ffprobe,
            whisper,
        }
    }
}

/// Result of parsing a video file.
pub struct VideoParseResult {
    /// The parsed document (content + metadata).
    pub document: ParsedDocument,
    /// Transcript segments with timestamps.
    pub segments: Vec<TranscriptSegment>,
}

/// Availability of required video processing tools.
#[derive(Debug)]
pub struct ToolAvailability {
    pub ffmpeg: bool,
    pub ffprobe: bool,
    pub whisper: bool,
}

impl ToolAvailability {
    /// Check if all required tools are available.
    pub fn all_available(&self) -> bool {
        self.ffmpeg && self.ffprobe && self.whisper
    }

    /// Get a message describing missing tools.
    pub fn missing_message(&self) -> Option<String> {
        let mut missing = Vec::new();
        if !self.ffmpeg {
            missing.push("ffmpeg");
        }
        if !self.ffprobe {
            missing.push("ffprobe");
        }
        if !self.whisper {
            missing.push("whisper");
        }

        if missing.is_empty() {
            None
        } else {
            Some(format!(
                "Missing tools: {}. Install with:\n  brew install ffmpeg\n  pip install openai-whisper",
                missing.join(", ")
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_availability() {
        let avail = VideoParser::tools_available();
        // Just verify it doesn't panic
        let _ = avail.all_available();
        let _ = avail.missing_message();
    }
}
