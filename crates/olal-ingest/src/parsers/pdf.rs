//! PDF document parser.

use super::{DocumentParser, ParsedDocument};
use crate::error::{IngestError, IngestResult};
use std::path::Path;
use tracing::debug;

/// Parser for PDF files.
pub struct PdfParser;

impl PdfParser {
    /// Create a new PDF parser.
    pub fn new() -> Self {
        Self
    }
}

impl Default for PdfParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentParser for PdfParser {
    fn parse(&self, path: &Path) -> IngestResult<ParsedDocument> {
        if !path.exists() {
            return Err(IngestError::FileNotFound(path.to_path_buf()));
        }

        debug!("Parsing PDF: {:?}", path);

        // Extract text from PDF
        let content = pdf_extract::extract_text(path).map_err(|e| {
            IngestError::ParseError {
                path: path.to_path_buf(),
                message: format!("Failed to extract text from PDF: {}", e),
            }
        })?;

        // Clean up the extracted text
        let content = clean_pdf_text(&content);

        // Count pages (rough estimate based on form feeds or content length)
        let page_count = content.matches('\x0C').count().max(1);

        let metadata = serde_json::json!({
            "format": "pdf",
            "length": content.len(),
            "pages": page_count,
        });

        // Use filename as title
        let title = path
            .file_stem()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());

        let mut doc = ParsedDocument::new(&content).with_metadata(metadata);

        if let Some(t) = title {
            doc = doc.with_title(t);
        }

        debug!("Extracted {} characters from PDF", content.len());

        Ok(doc)
    }

    fn extensions(&self) -> &[&str] {
        &["pdf"]
    }
}

/// Clean up extracted PDF text.
fn clean_pdf_text(text: &str) -> String {
    text.lines()
        // Remove excessive whitespace
        .map(|line| line.trim())
        // Remove empty lines that occur multiple times in a row
        .fold(Vec::new(), |mut acc, line| {
            let last_was_empty = acc.last().map(|s: &String| s.is_empty()).unwrap_or(false);
            if !(line.is_empty() && last_was_empty) {
                acc.push(line.to_string());
            }
            acc
        })
        .join("\n")
        // Remove form feed characters used as page breaks
        .replace('\x0C', "\n\n---\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_pdf_text() {
        let messy = "  Hello  \n\n\n\nWorld  \n\nTest";
        let cleaned = clean_pdf_text(messy);
        assert!(!cleaned.contains("\n\n\n")); // No triple newlines
    }

    #[test]
    fn test_pdf_parser_extensions() {
        let parser = PdfParser::new();
        assert!(parser.supports("pdf"));
        assert!(parser.supports("PDF"));
        assert!(!parser.supports("txt"));
    }
}
