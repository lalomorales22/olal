//! Document parsers for various file types.

mod audio;
mod markdown;
mod pdf;
mod text;
mod video;

pub use audio::AudioParser;
pub use markdown::MarkdownParser;
pub use pdf::PdfParser;
pub use text::TextParser;
pub use video::VideoParser;

use crate::error::IngestResult;
use olal_core::ItemType;
use std::path::Path;

/// Parsed document content.
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    /// The main text content.
    pub content: String,
    /// Document title (if extracted).
    pub title: Option<String>,
    /// Extracted metadata.
    pub metadata: serde_json::Value,
}

impl ParsedDocument {
    /// Create a new parsed document.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            title: None,
            metadata: serde_json::json!({}),
        }
    }

    /// Set the title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Trait for document parsers.
pub trait DocumentParser: Send + Sync {
    /// Parse a file at the given path.
    fn parse(&self, path: &Path) -> IngestResult<ParsedDocument>;

    /// Get the supported file extensions.
    fn extensions(&self) -> &[&str];

    /// Check if this parser supports the given extension.
    fn supports(&self, extension: &str) -> bool {
        self.extensions()
            .iter()
            .any(|ext| ext.eq_ignore_ascii_case(extension))
    }
}

/// Get the appropriate parser for a file type.
#[allow(dead_code)]
pub fn get_parser(item_type: ItemType) -> Option<Box<dyn DocumentParser>> {
    match item_type {
        ItemType::Note => Some(Box::new(MarkdownParser::new())),
        ItemType::Document => Some(Box::new(PdfParser::new())),
        ItemType::Code => Some(Box::new(TextParser::new())),
        _ => None,
    }
}

/// Parse a file based on its extension.
pub fn parse_file(path: &Path) -> IngestResult<ParsedDocument> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    // Try PDF parser for PDF files
    let pdf_parser = PdfParser::new();
    if pdf_parser.supports(extension) {
        return pdf_parser.parse(path);
    }

    // Try markdown parser
    let md_parser = MarkdownParser::new();
    if md_parser.supports(extension) {
        return md_parser.parse(path);
    }

    // Fall back to text parser
    let text_parser = TextParser::new();
    text_parser.parse(path)
}
