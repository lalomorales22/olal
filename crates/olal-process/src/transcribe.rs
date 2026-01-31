//! Audio transcription using Whisper.

use crate::error::{ProcessError, ProcessResult};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;
use tracing::{debug, info};

/// A segment of transcribed audio.
#[derive(Debug, Clone)]
pub struct TranscriptSegment {
    /// The transcribed text.
    pub text: String,
    /// Start time in seconds.
    pub start: f64,
    /// End time in seconds.
    pub end: f64,
}

#[derive(Debug, Deserialize)]
struct WhisperJsonOutput {
    #[allow(dead_code)]
    text: String,
    segments: Vec<WhisperSegment>,
}

#[derive(Debug, Deserialize)]
struct WhisperSegment {
    text: String,
    start: f64,
    end: f64,
}

/// Transcribe an audio file using Whisper.
///
/// Requires the `whisper` CLI to be installed (pip install openai-whisper).
pub fn transcribe_audio(
    audio_path: &Path,
    model: &str,
    output_dir: &Path,
) -> ProcessResult<Vec<TranscriptSegment>> {
    if !audio_path.exists() {
        return Err(ProcessError::FileNotFound(audio_path.to_path_buf()));
    }

    // Check if whisper is available
    if which::which("whisper").is_err() {
        return Err(ProcessError::ToolNotFound {
            tool: "whisper".to_string(),
        });
    }

    info!("Transcribing {:?} with model '{}'", audio_path, model);

    // Run whisper
    let output = Command::new("whisper")
        .arg(audio_path)
        .args(["--model", model])
        .args(["--output_format", "json"])
        .args(["--output_dir"])
        .arg(output_dir)
        .args(["--language", "en"])  // Default to English
        .output()?;

    if !output.status.success() {
        return Err(ProcessError::TranscriptionError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    // Find the output JSON file
    let stem = audio_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("audio");
    let json_path = output_dir.join(format!("{}.json", stem));

    if !json_path.exists() {
        return Err(ProcessError::TranscriptionError(
            "Whisper output file not found".to_string(),
        ));
    }

    // Parse the JSON output
    let json_content = std::fs::read_to_string(&json_path)?;
    let whisper_output: WhisperJsonOutput = serde_json::from_str(&json_content)
        .map_err(|e| ProcessError::ParseError(format!("Failed to parse Whisper output: {}", e)))?;

    let segments: Vec<TranscriptSegment> = whisper_output
        .segments
        .into_iter()
        .map(|s| TranscriptSegment {
            text: s.text.trim().to_string(),
            start: s.start,
            end: s.end,
        })
        .collect();

    debug!("Transcribed {} segments", segments.len());
    Ok(segments)
}

/// Transcribe using insanely-fast-whisper for faster processing.
/// Falls back to regular whisper if not available.
#[allow(dead_code)]
pub fn transcribe_fast(
    audio_path: &Path,
    output_dir: &Path,
) -> ProcessResult<Vec<TranscriptSegment>> {
    // First try insanely-fast-whisper
    if which::which("insanely-fast-whisper").is_ok() {
        info!("Using insanely-fast-whisper for transcription");
        return transcribe_with_insanely_fast(audio_path, output_dir);
    }

    // Fall back to regular whisper with base model
    info!("Falling back to regular whisper");
    transcribe_audio(audio_path, "base", output_dir)
}

#[allow(dead_code)]
fn transcribe_with_insanely_fast(
    audio_path: &Path,
    output_dir: &Path,
) -> ProcessResult<Vec<TranscriptSegment>> {
    let stem = audio_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("audio");
    let output_path = output_dir.join(format!("{}_transcript.json", stem));

    let output = Command::new("insanely-fast-whisper")
        .args(["--file-name"])
        .arg(audio_path)
        .args(["--transcript-path"])
        .arg(&output_path)
        .output()?;

    if !output.status.success() {
        return Err(ProcessError::TranscriptionError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    // Parse output (format may differ slightly)
    let json_content = std::fs::read_to_string(&output_path)?;

    // Try parsing as whisper format first
    if let Ok(whisper_output) = serde_json::from_str::<WhisperJsonOutput>(&json_content) {
        return Ok(whisper_output
            .segments
            .into_iter()
            .map(|s| TranscriptSegment {
                text: s.text.trim().to_string(),
                start: s.start,
                end: s.end,
            })
            .collect());
    }

    // If that fails, try other formats or return error
    Err(ProcessError::ParseError(
        "Failed to parse transcription output".to_string(),
    ))
}

/// Get the full transcript text from segments.
#[allow(dead_code)]
pub fn segments_to_text(segments: &[TranscriptSegment]) -> String {
    segments
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Format segments for display with timestamps.
#[allow(dead_code)]
pub fn format_transcript(segments: &[TranscriptSegment]) -> String {
    segments
        .iter()
        .map(|s| {
            format!(
                "[{:02}:{:02}] {}",
                (s.start / 60.0) as u32,
                (s.start % 60.0) as u32,
                s.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segments_to_text() {
        let segments = vec![
            TranscriptSegment {
                text: "Hello".to_string(),
                start: 0.0,
                end: 1.0,
            },
            TranscriptSegment {
                text: "world".to_string(),
                start: 1.0,
                end: 2.0,
            },
        ];

        assert_eq!(segments_to_text(&segments), "Hello world");
    }

    #[test]
    fn test_format_transcript() {
        let segments = vec![
            TranscriptSegment {
                text: "Hello".to_string(),
                start: 0.0,
                end: 1.0,
            },
            TranscriptSegment {
                text: "world".to_string(),
                start: 65.0,
                end: 66.0,
            },
        ];

        let formatted = format_transcript(&segments);
        assert!(formatted.contains("[00:00] Hello"));
        assert!(formatted.contains("[01:05] world"));
    }
}
