//! Olal Process - Media processing for video, audio, and images.
//!
//! This crate provides:
//! - Video processing (via FFmpeg CLI)
//! - Audio transcription (via Whisper CLI)
//! - OCR for images (via Tesseract CLI)
//!
//! These rely on external tools being installed on the system.

mod error;
mod ffmpeg;
mod ocr;
mod transcribe;

pub use error::{ProcessError, ProcessResult};
pub use ffmpeg::{extract_audio, extract_frames, get_video_info, VideoInfo};
pub use ocr::{ocr_image, OcrResult};
pub use transcribe::{transcribe_audio, TranscriptSegment};

/// Check if required external tools are available.
pub fn check_dependencies() -> Vec<(&'static str, bool)> {
    vec![
        ("ffmpeg", which::which("ffmpeg").is_ok()),
        ("ffprobe", which::which("ffprobe").is_ok()),
        ("whisper", which::which("whisper").is_ok()),
        ("tesseract", which::which("tesseract").is_ok()),
    ]
}

/// Check if all required tools are installed.
pub fn all_tools_available() -> bool {
    check_dependencies().iter().all(|(_, available)| *available)
}
