//! Plain text document parser.

use super::{DocumentParser, ParsedDocument};
use crate::error::{IngestError, IngestResult};
use std::path::Path;

/// Parser for plain text files (including code).
pub struct TextParser;

impl TextParser {
    /// Create a new text parser.
    pub fn new() -> Self {
        Self
    }

    /// Detect if file is likely code based on extension.
    fn is_code_file(extension: &str) -> bool {
        matches!(
            extension.to_lowercase().as_str(),
            "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "go" | "c" | "cpp" | "h" | "hpp"
                | "java" | "rb" | "sh" | "bash" | "zsh" | "json" | "yaml" | "yml" | "toml"
                | "html" | "css" | "scss" | "sql" | "swift" | "kt" | "scala" | "php"
                | "lua" | "r" | "pl" | "ex" | "exs" | "clj" | "hs" | "ml" | "fs"
        )
    }

    /// Detect programming language from extension.
    fn detect_language(extension: &str) -> Option<&'static str> {
        match extension.to_lowercase().as_str() {
            "rs" => Some("rust"),
            "py" => Some("python"),
            "js" => Some("javascript"),
            "ts" => Some("typescript"),
            "jsx" => Some("javascript"),
            "tsx" => Some("typescript"),
            "go" => Some("go"),
            "c" => Some("c"),
            "cpp" | "cc" | "cxx" => Some("cpp"),
            "h" | "hpp" => Some("cpp"),
            "java" => Some("java"),
            "rb" => Some("ruby"),
            "sh" | "bash" | "zsh" => Some("shell"),
            "json" => Some("json"),
            "yaml" | "yml" => Some("yaml"),
            "toml" => Some("toml"),
            "html" | "htm" => Some("html"),
            "css" => Some("css"),
            "scss" | "sass" => Some("scss"),
            "sql" => Some("sql"),
            "swift" => Some("swift"),
            "kt" => Some("kotlin"),
            "scala" => Some("scala"),
            "php" => Some("php"),
            "lua" => Some("lua"),
            "r" => Some("r"),
            "pl" => Some("perl"),
            "ex" | "exs" => Some("elixir"),
            "clj" => Some("clojure"),
            "hs" => Some("haskell"),
            "ml" | "mli" => Some("ocaml"),
            "fs" | "fsx" => Some("fsharp"),
            _ => None,
        }
    }
}

impl Default for TextParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentParser for TextParser {
    fn parse(&self, path: &Path) -> IngestResult<ParsedDocument> {
        if !path.exists() {
            return Err(IngestError::FileNotFound(path.to_path_buf()));
        }

        let content = std::fs::read_to_string(path)?;
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let is_code = Self::is_code_file(extension);
        let language = Self::detect_language(extension);

        let mut metadata = serde_json::json!({
            "format": if is_code { "code" } else { "text" },
            "length": content.len(),
            "lines": content.lines().count(),
        });

        if let Some(lang) = language {
            metadata["language"] = serde_json::json!(lang);
        }

        // Use filename as title
        let title = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());

        let mut doc = ParsedDocument::new(&content).with_metadata(metadata);

        if let Some(t) = title {
            doc = doc.with_title(t);
        }

        Ok(doc)
    }

    fn extensions(&self) -> &[&str] {
        &[
            "txt", "text", "log", // Plain text
            "rs", "py", "js", "ts", "jsx", "tsx", "go", "c", "cpp", "h", "hpp", // Code
            "java", "rb", "sh", "bash", "zsh", "json", "yaml", "yml", "toml", "html", "css",
            "scss", "sql", "swift", "kt", "scala", "php", "lua", "r", "pl", "ex", "exs", "clj",
            "hs", "ml", "fs", "org", "rst",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_text() {
        let mut file = NamedTempFile::with_suffix(".txt").unwrap();
        writeln!(file, "This is a plain text file.\nWith multiple lines.").unwrap();

        let parser = TextParser::new();
        let doc = parser.parse(file.path()).unwrap();

        assert!(doc.content.contains("plain text file"));
        assert_eq!(doc.metadata["format"], "text");
    }

    #[test]
    fn test_parse_code() {
        let mut file = NamedTempFile::with_suffix(".rs").unwrap();
        writeln!(
            file,
            r#"fn main() {{
    println!("Hello, world!");
}}"#
        )
        .unwrap();

        let parser = TextParser::new();
        let doc = parser.parse(file.path()).unwrap();

        assert!(doc.content.contains("fn main()"));
        assert_eq!(doc.metadata["format"], "code");
        assert_eq!(doc.metadata["language"], "rust");
    }

    #[test]
    fn test_language_detection() {
        assert_eq!(TextParser::detect_language("rs"), Some("rust"));
        assert_eq!(TextParser::detect_language("py"), Some("python"));
        assert_eq!(TextParser::detect_language("JS"), Some("javascript")); // Case insensitive
        assert_eq!(TextParser::detect_language("xyz"), None);
    }
}
