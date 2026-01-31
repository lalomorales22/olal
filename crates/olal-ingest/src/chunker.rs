//! Content chunking for RAG retrieval.
//!
//! Provides strategies for splitting text into chunks suitable for embedding
//! and retrieval.

use olal_core::{Chunk, ItemId};

/// Configuration for chunking.
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Target size of each chunk in characters.
    pub chunk_size: usize,
    /// Number of characters to overlap between chunks.
    pub chunk_overlap: usize,
    /// Minimum chunk size (won't create chunks smaller than this).
    pub min_chunk_size: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            chunk_size: 1000,
            chunk_overlap: 100,
            min_chunk_size: 100,
        }
    }
}

impl ChunkConfig {
    /// Create config from processing settings.
    pub fn from_processing_config(config: &olal_config::ProcessingConfig) -> Self {
        Self {
            // Convert token-based config to character-based (rough estimate: 4 chars per token)
            chunk_size: config.chunk_size * 4,
            chunk_overlap: config.chunk_overlap * 4,
            min_chunk_size: 100,
        }
    }
}

/// Content chunker for splitting text.
pub struct Chunker {
    config: ChunkConfig,
}

impl Chunker {
    /// Create a new chunker with the given configuration.
    pub fn new(config: ChunkConfig) -> Self {
        Self { config }
    }

    /// Create a chunker with default configuration.
    pub fn default_chunker() -> Self {
        Self::new(ChunkConfig::default())
    }

    /// Split text into chunks.
    /// Works by splitting on paragraph/sentence boundaries where possible.
    pub fn chunk_text(&self, item_id: &ItemId, text: &str) -> Vec<Chunk> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return vec![];
        }

        // If it fits in one chunk, return it
        if trimmed.chars().count() <= self.config.chunk_size {
            return vec![Chunk::new(item_id.clone(), 0, trimmed)];
        }

        // Split into paragraphs first
        let paragraphs: Vec<&str> = trimmed.split("\n\n").collect();

        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut chunk_index = 0;

        for para in paragraphs {
            let para = para.trim();
            if para.is_empty() {
                continue;
            }

            let current_len = current_chunk.chars().count();
            let para_len = para.chars().count();

            // If adding this paragraph would exceed chunk size
            if current_len > 0 && current_len + para_len + 2 > self.config.chunk_size {
                // Save current chunk
                let chunk_text = current_chunk.trim();
                if chunk_text.chars().count() >= self.config.min_chunk_size || chunk_index == 0 {
                    chunks.push(Chunk::new(item_id.clone(), chunk_index, chunk_text));
                    chunk_index += 1;
                }

                // Start new chunk with overlap
                if self.config.chunk_overlap > 0 {
                    let chars: Vec<char> = current_chunk.chars().collect();
                    let skip = chars.len().saturating_sub(self.config.chunk_overlap);
                    current_chunk = chars[skip..].iter().collect();
                } else {
                    current_chunk.clear();
                }
            }

            // Add paragraph to current chunk
            if !current_chunk.is_empty() {
                current_chunk.push_str("\n\n");
            }

            // If paragraph itself is too long, we need to split it
            if para_len > self.config.chunk_size {
                // Split long paragraph into sentences
                let sentences = self.split_sentences(para);

                // If no sentences found (e.g., JSON without periods), force character-based split
                if sentences.len() <= 1 && para_len > self.config.chunk_size {
                    // Force split by character limit
                    for chunk_str in self.force_split_by_chars(para) {
                        let chunk_str_len = chunk_str.chars().count();
                        let current_len = current_chunk.chars().count();

                        if current_len > 0 && current_len + chunk_str_len + 1 > self.config.chunk_size {
                            let chunk_text = current_chunk.trim();
                            if chunk_text.chars().count() >= self.config.min_chunk_size || chunk_index == 0 {
                                chunks.push(Chunk::new(item_id.clone(), chunk_index, chunk_text));
                                chunk_index += 1;
                            }
                            current_chunk.clear();
                        }

                        if !current_chunk.is_empty() {
                            current_chunk.push(' ');
                        }
                        current_chunk.push_str(&chunk_str);
                    }
                } else {
                    for sentence in sentences {
                        let sentence_len = sentence.chars().count();
                        let current_len = current_chunk.chars().count();

                        if current_len > 0 && current_len + sentence_len + 1 > self.config.chunk_size {
                            // Save current chunk
                            let chunk_text = current_chunk.trim();
                            if chunk_text.chars().count() >= self.config.min_chunk_size || chunk_index == 0 {
                                chunks.push(Chunk::new(item_id.clone(), chunk_index, chunk_text));
                                chunk_index += 1;
                            }

                            // Start new chunk with overlap
                            if self.config.chunk_overlap > 0 {
                                let chars: Vec<char> = current_chunk.chars().collect();
                                let skip = chars.len().saturating_sub(self.config.chunk_overlap);
                                current_chunk = chars[skip..].iter().collect();
                            } else {
                                current_chunk.clear();
                            }
                        }

                        if !current_chunk.is_empty() {
                            current_chunk.push(' ');
                        }
                        current_chunk.push_str(sentence);
                    }
                }
            } else {
                current_chunk.push_str(para);
            }
        }

        // Don't forget the last chunk
        let chunk_text = current_chunk.trim();
        if !chunk_text.is_empty() {
            chunks.push(Chunk::new(item_id.clone(), chunk_index, chunk_text));
        }

        chunks
    }

    /// Force split text by character limit (for content without natural breaks like JSON).
    fn force_split_by_chars(&self, text: &str) -> Vec<String> {
        let chars: Vec<char> = text.chars().collect();
        let mut result = Vec::new();
        let mut start = 0;

        while start < chars.len() {
            let end = std::cmp::min(start + self.config.chunk_size, chars.len());
            let chunk: String = chars[start..end].iter().collect();
            result.push(chunk);
            // Move forward with overlap
            start = end.saturating_sub(self.config.chunk_overlap);
            if start >= end {
                start = end; // Prevent infinite loop
            }
        }

        result
    }

    /// Split text into sentences.
    fn split_sentences<'a>(&self, text: &'a str) -> Vec<&'a str> {
        let mut sentences = Vec::new();
        let mut start = 0;

        for (i, c) in text.char_indices() {
            if c == '.' || c == '!' || c == '?' {
                // Check if next char is space or end
                let next_idx = i + c.len_utf8();
                if next_idx >= text.len() || text[next_idx..].starts_with(' ') || text[next_idx..].starts_with('\n') {
                    sentences.push(&text[start..next_idx]);
                    start = next_idx;
                    // Skip the space if present
                    if start < text.len() && text[start..].starts_with(' ') {
                        start += 1;
                    }
                }
            }
        }

        // Add remaining text
        if start < text.len() {
            let remaining = text[start..].trim();
            if !remaining.is_empty() {
                sentences.push(remaining);
            }
        }

        if sentences.is_empty() && !text.trim().is_empty() {
            sentences.push(text.trim());
        }

        sentences
    }

    /// Split transcript segments into chunks with timestamps.
    pub fn chunk_transcript(
        &self,
        item_id: &ItemId,
        segments: &[(String, f64, f64)], // (text, start_time, end_time)
    ) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let mut current_text = String::new();
        let mut current_start: Option<f64> = None;
        let mut current_end: f64 = 0.0;
        let mut chunk_index = 0;

        for (text, start, end) in segments {
            let current_char_count = current_text.chars().count();
            let text_char_count = text.chars().count();

            // Check if adding this segment would exceed chunk size
            if !current_text.is_empty()
                && current_char_count + text_char_count + 1 > self.config.chunk_size
            {
                // Finalize current chunk
                if current_char_count >= self.config.min_chunk_size {
                    let chunk = Chunk::new(item_id.clone(), chunk_index, current_text.trim())
                        .with_timestamps(current_start.unwrap_or(0.0), current_end);
                    chunks.push(chunk);
                    chunk_index += 1;
                }

                // Start new chunk with overlap (in characters)
                let overlap_chars = self.config.chunk_overlap;
                let skip_chars = current_char_count.saturating_sub(overlap_chars);
                current_text = current_text.chars().skip(skip_chars).collect();
                current_start = Some(*start);
            }

            if current_start.is_none() {
                current_start = Some(*start);
            }

            if !current_text.is_empty() {
                current_text.push(' ');
            }
            current_text.push_str(text);
            current_end = *end;
        }

        // Don't forget the last chunk
        if !current_text.trim().is_empty() {
            let chunk = Chunk::new(item_id.clone(), chunk_index, current_text.trim())
                .with_timestamps(current_start.unwrap_or(0.0), current_end);
            chunks.push(chunk);
        }

        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_text_single_chunk() {
        let chunker = Chunker::default_chunker();
        let chunks = chunker.chunk_text(&"item1".to_string(), "This is a small piece of text.");

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "This is a small piece of text.");
        assert_eq!(chunks[0].chunk_index, 0);
    }

    #[test]
    fn test_large_text_multiple_chunks() {
        let config = ChunkConfig {
            chunk_size: 100,
            chunk_overlap: 20,
            min_chunk_size: 20,
        };
        let chunker = Chunker::new(config);

        let text = "This is sentence one. This is sentence two. This is sentence three. \
                    This is sentence four. This is sentence five. This is sentence six. \
                    This is sentence seven. This is sentence eight. This is sentence nine.";

        let chunks = chunker.chunk_text(&"item1".to_string(), text);

        assert!(chunks.len() > 1, "Should create multiple chunks, got {}", chunks.len());

        // Verify all chunks have content
        for chunk in &chunks {
            assert!(!chunk.content.is_empty());
        }
    }

    #[test]
    fn test_utf8_text() {
        let config = ChunkConfig {
            chunk_size: 50,
            chunk_overlap: 10,
            min_chunk_size: 10,
        };
        let chunker = Chunker::new(config);

        // Text with multi-byte characters (box drawing characters, emojis)
        let text = "Hello ─── World! This has unicode: 日本語 and more ─ content here.";

        let chunks = chunker.chunk_text(&"item1".to_string(), text);

        // Should not panic
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            // Verify the content is valid UTF-8 (would panic if not)
            assert!(!chunk.content.is_empty());
        }
    }

    #[test]
    fn test_chunk_transcript_with_timestamps() {
        let config = ChunkConfig {
            chunk_size: 50,
            chunk_overlap: 10,
            min_chunk_size: 10,
        };
        let chunker = Chunker::new(config);

        let segments = vec![
            ("Hello world".to_string(), 0.0, 1.0),
            ("This is a test".to_string(), 1.0, 2.0),
            ("More content here".to_string(), 2.0, 3.0),
        ];

        let chunks = chunker.chunk_transcript(&"item1".to_string(), &segments);

        assert!(!chunks.is_empty());
        // Check timestamps are set
        for chunk in &chunks {
            assert!(chunk.start_time.is_some());
            assert!(chunk.end_time.is_some());
        }
    }

    #[test]
    fn test_empty_text() {
        let chunker = Chunker::default_chunker();
        let chunks = chunker.chunk_text(&"item1".to_string(), "");
        assert!(chunks.is_empty());

        let chunks = chunker.chunk_text(&"item1".to_string(), "   ");
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_paragraph_based_chunking() {
        let config = ChunkConfig {
            chunk_size: 100,
            chunk_overlap: 10,
            min_chunk_size: 10,
        };
        let chunker = Chunker::new(config);

        let text = "First paragraph here.\n\nSecond paragraph with more content.\n\nThird paragraph.";

        let chunks = chunker.chunk_text(&"item1".to_string(), text);

        // Should handle paragraphs properly
        assert!(!chunks.is_empty());
    }
}
