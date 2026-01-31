//! FFmpeg integration for video/audio processing.

use crate::error::{ProcessError, ProcessResult};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;
use tracing::{debug, info};

/// Information about a video file.
#[derive(Debug, Clone)]
pub struct VideoInfo {
    /// Duration in seconds.
    pub duration: f64,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Video codec.
    pub video_codec: Option<String>,
    /// Audio codec.
    pub audio_codec: Option<String>,
    /// Frame rate.
    pub fps: Option<f64>,
    /// Bitrate in bits per second.
    pub bitrate: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    format: FfprobeFormat,
    streams: Vec<FfprobeStream>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
    bit_rate: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    codec_type: String,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
}

/// Get information about a video file.
pub fn get_video_info(path: &Path) -> ProcessResult<VideoInfo> {
    if !path.exists() {
        return Err(ProcessError::FileNotFound(path.to_path_buf()));
    }

    // Check ffprobe is available
    if which::which("ffprobe").is_err() {
        return Err(ProcessError::ToolNotFound {
            tool: "ffprobe".to_string(),
        });
    }

    let output = Command::new("ffprobe")
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()?;

    if !output.status.success() {
        return Err(ProcessError::FfmpegError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let probe: FfprobeOutput = serde_json::from_str(&json_str)
        .map_err(|e| ProcessError::ParseError(format!("Failed to parse ffprobe output: {}", e)))?;

    // Extract video stream info
    let video_stream = probe.streams.iter().find(|s| s.codec_type == "video");
    let audio_stream = probe.streams.iter().find(|s| s.codec_type == "audio");

    let duration = probe
        .format
        .duration
        .as_ref()
        .and_then(|d| d.parse::<f64>().ok())
        .unwrap_or(0.0);

    let (width, height) = video_stream
        .map(|s| (s.width.unwrap_or(0), s.height.unwrap_or(0)))
        .unwrap_or((0, 0));

    let fps = video_stream
        .and_then(|s| s.r_frame_rate.as_ref())
        .and_then(|r| {
            let parts: Vec<&str> = r.split('/').collect();
            if parts.len() == 2 {
                let num: f64 = parts[0].parse().ok()?;
                let den: f64 = parts[1].parse().ok()?;
                if den > 0.0 {
                    Some(num / den)
                } else {
                    None
                }
            } else {
                r.parse().ok()
            }
        });

    Ok(VideoInfo {
        duration,
        width,
        height,
        video_codec: video_stream.and_then(|s| s.codec_name.clone()),
        audio_codec: audio_stream.and_then(|s| s.codec_name.clone()),
        fps,
        bitrate: probe.format.bit_rate.as_ref().and_then(|b| b.parse().ok()),
    })
}

/// Extract audio from a video file.
///
/// Returns the path to the extracted audio file (WAV format).
pub fn extract_audio(video_path: &Path, output_dir: &Path) -> ProcessResult<std::path::PathBuf> {
    if !video_path.exists() {
        return Err(ProcessError::FileNotFound(video_path.to_path_buf()));
    }

    if which::which("ffmpeg").is_err() {
        return Err(ProcessError::ToolNotFound {
            tool: "ffmpeg".to_string(),
        });
    }

    // Create output path
    let stem = video_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("audio");
    let audio_path = output_dir.join(format!("{}.wav", stem));

    info!("Extracting audio from {:?} to {:?}", video_path, audio_path);

    let output = Command::new("ffmpeg")
        .args([
            "-i",
        ])
        .arg(video_path)
        .args([
            "-vn",           // No video
            "-acodec", "pcm_s16le",  // PCM audio
            "-ar", "16000",  // 16kHz sample rate (good for Whisper)
            "-ac", "1",      // Mono
            "-y",            // Overwrite output
        ])
        .arg(&audio_path)
        .output()?;

    if !output.status.success() {
        return Err(ProcessError::FfmpegError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    debug!("Audio extracted successfully");
    Ok(audio_path)
}

/// Extract frames from a video at regular intervals.
///
/// Returns the paths to the extracted frame images.
pub fn extract_frames(
    video_path: &Path,
    output_dir: &Path,
    interval_seconds: u64,
) -> ProcessResult<Vec<std::path::PathBuf>> {
    if !video_path.exists() {
        return Err(ProcessError::FileNotFound(video_path.to_path_buf()));
    }

    if which::which("ffmpeg").is_err() {
        return Err(ProcessError::ToolNotFound {
            tool: "ffmpeg".to_string(),
        });
    }

    // Get video duration
    let info = get_video_info(video_path)?;

    // Create output pattern
    let stem = video_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("frame");
    let pattern = output_dir.join(format!("{}_frame_%04d.png", stem));

    info!(
        "Extracting frames from {:?} every {}s",
        video_path, interval_seconds
    );

    // Extract frames at interval
    let fps = 1.0 / interval_seconds as f64;
    let output = Command::new("ffmpeg")
        .args(["-i"])
        .arg(video_path)
        .args([
            "-vf",
            &format!("fps={}", fps),
            "-q:v", "2",  // High quality
            "-y",
        ])
        .arg(&pattern)
        .output()?;

    if !output.status.success() {
        return Err(ProcessError::FfmpegError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    // Find all extracted frames
    let mut frames = Vec::new();
    let expected_frames = (info.duration / interval_seconds as f64).ceil() as u32;

    for i in 1..=expected_frames + 1 {
        let frame_path = output_dir.join(format!("{}_frame_{:04}.png", stem, i));
        if frame_path.exists() {
            frames.push(frame_path);
        }
    }

    debug!("Extracted {} frames", frames.len());
    Ok(frames)
}

/// Extract a single frame at a specific timestamp.
#[allow(dead_code)]
pub fn extract_frame_at(
    video_path: &Path,
    output_path: &Path,
    timestamp_seconds: f64,
) -> ProcessResult<()> {
    if !video_path.exists() {
        return Err(ProcessError::FileNotFound(video_path.to_path_buf()));
    }

    if which::which("ffmpeg").is_err() {
        return Err(ProcessError::ToolNotFound {
            tool: "ffmpeg".to_string(),
        });
    }

    let output = Command::new("ffmpeg")
        .args(["-ss", &format!("{:.2}", timestamp_seconds)])
        .args(["-i"])
        .arg(video_path)
        .args([
            "-vframes", "1",
            "-q:v", "2",
            "-y",
        ])
        .arg(output_path)
        .output()?;

    if !output.status.success() {
        return Err(ProcessError::FfmpegError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_check() {
        // Just verify the tool check doesn't panic
        let _ = which::which("ffmpeg");
        let _ = which::which("ffprobe");
    }
}
