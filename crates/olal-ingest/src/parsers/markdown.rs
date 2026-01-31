//! Markdown document parser.

use super::{DocumentParser, ParsedDocument};
use crate::error::{IngestError, IngestResult};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag};
use std::path::Path;

/// Parser for Markdown files.
pub struct MarkdownParser {
    /// Whether to preserve code blocks.
    preserve_code_blocks: bool,
}

impl MarkdownParser {
    /// Create a new markdown parser.
    pub fn new() -> Self {
        Self {
            preserve_code_blocks: true,
        }
    }

    /// Extract text content from markdown.
    fn extract_text(&self, markdown: &str) -> (String, Option<String>, Vec<String>) {
        let parser = Parser::new(markdown);
        let mut text = String::new();
        let mut title: Option<String> = None;
        let mut links = Vec::new();
        let mut _in_code_block = false;
        let mut in_heading = false;
        let mut heading_level: Option<HeadingLevel> = None;
        let mut current_heading = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::Heading(level, _, _)) => {
                    in_heading = true;
                    heading_level = Some(level);
                    current_heading.clear();
                }
                Event::End(Tag::Heading(_, _, _)) => {
                    in_heading = false;
                    if heading_level == Some(HeadingLevel::H1) && title.is_none() {
                        title = Some(current_heading.trim().to_string());
                    }
                    // Add heading to text with some formatting
                    text.push_str(&current_heading);
                    text.push_str("\n\n");
                    heading_level = None;
                }
                Event::Start(Tag::CodeBlock(_)) => {
                    _in_code_block = true;
                    if self.preserve_code_blocks {
                        text.push_str("\n```\n");
                    }
                }
                Event::End(Tag::CodeBlock(_)) => {
                    _in_code_block = false;
                    if self.preserve_code_blocks {
                        text.push_str("```\n\n");
                    }
                }
                Event::Start(Tag::Link(_, dest, _)) => {
                    links.push(dest.to_string());
                }
                Event::Start(Tag::Paragraph) => {}
                Event::End(Tag::Paragraph) => {
                    text.push_str("\n\n");
                }
                Event::Start(Tag::List(_)) => {}
                Event::End(Tag::List(_)) => {
                    text.push('\n');
                }
                Event::Start(Tag::Item) => {
                    text.push_str("- ");
                }
                Event::End(Tag::Item) => {
                    text.push('\n');
                }
                Event::Text(t) => {
                    if in_heading {
                        current_heading.push_str(&t);
                    } else {
                        text.push_str(&t);
                    }
                }
                Event::Code(code) => {
                    text.push('`');
                    text.push_str(&code);
                    text.push('`');
                }
                Event::SoftBreak | Event::HardBreak => {
                    text.push('\n');
                }
                _ => {}
            }
        }

        (text.trim().to_string(), title, links)
    }
}

impl Default for MarkdownParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentParser for MarkdownParser {
    fn parse(&self, path: &Path) -> IngestResult<ParsedDocument> {
        if !path.exists() {
            return Err(IngestError::FileNotFound(path.to_path_buf()));
        }

        let content = std::fs::read_to_string(path)?;
        let (text, title, links) = self.extract_text(&content);

        let metadata = serde_json::json!({
            "format": "markdown",
            "links": links,
            "original_length": content.len(),
        });

        let mut doc = ParsedDocument::new(text).with_metadata(metadata);

        if let Some(t) = title {
            doc = doc.with_title(t);
        } else {
            // Use filename as title if no h1 found
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                doc = doc.with_title(stem);
            }
        }

        Ok(doc)
    }

    fn extensions(&self) -> &[&str] {
        &["md", "markdown", "mdown", "mkd"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_markdown() {
        let mut file = NamedTempFile::with_suffix(".md").unwrap();
        writeln!(
            file,
            r#"# My Document

This is a paragraph with some text.

## Section One

More content here with a [link](https://example.com).

```rust
fn main() {{
    println!("Hello");
}}
```

- Item one
- Item two
"#
        )
        .unwrap();

        let parser = MarkdownParser::new();
        let doc = parser.parse(file.path()).unwrap();

        assert_eq!(doc.title, Some("My Document".to_string()));
        assert!(doc.content.contains("This is a paragraph"));
        assert!(doc.content.contains("Section One"));
        assert!(doc.content.contains("fn main()"));

        // Check links were extracted
        let links = doc.metadata["links"].as_array().unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0], "https://example.com");
    }

    #[test]
    fn test_no_title() {
        let mut file = NamedTempFile::with_suffix(".md").unwrap();
        writeln!(file, "Just some text without a heading.").unwrap();

        let parser = MarkdownParser::new();
        let doc = parser.parse(file.path()).unwrap();

        // Should use filename as title
        assert!(doc.title.is_some());
    }
}
