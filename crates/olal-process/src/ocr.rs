//! OCR processing using Tesseract.

use crate::error::{ProcessError, ProcessResult};
use std::path::Path;
use std::process::Command;
use tracing::debug;

/// Result of OCR processing.
#[derive(Debug, Clone)]
pub struct OcrResult {
    /// The extracted text.
    pub text: String,
    /// Confidence score (0-100), if available.
    pub confidence: Option<f32>,
}

/// Perform OCR on an image file.
pub fn ocr_image(image_path: &Path) -> ProcessResult<OcrResult> {
    if !image_path.exists() {
        return Err(ProcessError::FileNotFound(image_path.to_path_buf()));
    }

    // Check if tesseract is available
    if which::which("tesseract").is_err() {
        return Err(ProcessError::ToolNotFound {
            tool: "tesseract".to_string(),
        });
    }

    debug!("Running OCR on {:?}", image_path);

    // Run tesseract
    let output = Command::new("tesseract")
        .arg(image_path)
        .arg("stdout")  // Output to stdout instead of file
        .args(["--oem", "3"])  // LSTM + legacy engine
        .args(["--psm", "1"])  // Automatic page segmentation with OSD
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Tesseract sometimes outputs warnings to stderr but still works
        if !output.stdout.is_empty() {
            debug!("Tesseract warning: {}", stderr);
        } else {
            return Err(ProcessError::OcrError(stderr.to_string()));
        }
    }

    let text = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();

    Ok(OcrResult {
        text,
        confidence: None,
    })
}

/// Perform OCR optimized for code/terminal screenshots.
#[allow(dead_code)]
pub fn ocr_code_image(image_path: &Path) -> ProcessResult<OcrResult> {
    if !image_path.exists() {
        return Err(ProcessError::FileNotFound(image_path.to_path_buf()));
    }

    if which::which("tesseract").is_err() {
        return Err(ProcessError::ToolNotFound {
            tool: "tesseract".to_string(),
        });
    }

    debug!("Running code-optimized OCR on {:?}", image_path);

    // Use PSM 6 for uniform block of text (good for code)
    let output = Command::new("tesseract")
        .arg(image_path)
        .arg("stdout")
        .args(["--oem", "3"])
        .args(["--psm", "6"])  // Assume uniform block of text
        .output()?;

    if !output.status.success() && output.stdout.is_empty() {
        return Err(ProcessError::OcrError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();

    Ok(OcrResult {
        text,
        confidence: None,
    })
}

/// Perform OCR on multiple images and deduplicate similar text.
#[allow(dead_code)]
pub fn ocr_images_deduplicated(image_paths: &[impl AsRef<Path>]) -> ProcessResult<Vec<(usize, OcrResult)>> {
    let mut results: Vec<(usize, OcrResult)> = Vec::new();
    let mut seen_texts: Vec<String> = Vec::new();

    for (idx, path) in image_paths.iter().enumerate() {
        match ocr_image(path.as_ref()) {
            Ok(result) => {
                // Check if this text is similar to any we've seen
                if !is_similar_to_any(&result.text, &seen_texts) {
                    seen_texts.push(result.text.clone());
                    results.push((idx, result));
                }
            }
            Err(e) => {
                debug!("OCR failed for image {}: {}", idx, e);
            }
        }
    }

    Ok(results)
}

/// Check if text is similar to any in the list (basic deduplication).
#[allow(dead_code)]
fn is_similar_to_any(text: &str, others: &[String]) -> bool {
    if text.is_empty() {
        return true;  // Skip empty text
    }

    for other in others {
        if is_similar(text, other) {
            return true;
        }
    }
    false
}

/// Check if two texts are similar (simple heuristic).
#[allow(dead_code)]
fn is_similar(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }

    // Normalize whitespace
    let a_normalized: String = a.split_whitespace().collect();
    let b_normalized: String = b.split_whitespace().collect();

    if a_normalized == b_normalized {
        return true;
    }

    // Check if one is a subset of the other (for partial matches)
    let a_words: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let b_words: std::collections::HashSet<&str> = b.split_whitespace().collect();

    if a_words.is_empty() || b_words.is_empty() {
        return false;
    }

    let intersection = a_words.intersection(&b_words).count();
    let min_len = a_words.len().min(b_words.len());

    // Consider similar if 80% of words match
    intersection as f64 / min_len as f64 >= 0.8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_similarity() {
        assert!(is_similar("hello world", "hello world"));
        assert!(is_similar("hello  world", "hello world"));
        // 2/3 = 66% overlap, below 80% threshold
        assert!(!is_similar("hello world foo", "hello world bar"));
        // 4/5 = 80% overlap, meets threshold
        assert!(is_similar("hello world foo bar baz", "hello world foo bar qux"));
        assert!(!is_similar("hello", "goodbye"));
    }

    #[test]
    fn test_tool_check() {
        let _ = which::which("tesseract");
    }
}
