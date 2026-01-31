//! Main ingestion logic.

use crate::chunker::{ChunkConfig, Chunker};
use crate::error::{IngestError, IngestResult};
use crate::parsers::{self, AudioParser, ParsedDocument, VideoParser};
use olal_core::{Chunk, Item, ItemType, QueueItem};
use olal_db::Database;
use olal_process::TranscriptSegment;
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::path::Path;
use tracing::{debug, info, warn};

/// Result of processing a file.
#[derive(Debug)]
pub struct IngestResult2 {
    /// The created item.
    pub item: Item,
    /// The created chunks.
    pub chunks: Vec<Chunk>,
    /// Whether this was a re-process of an existing item.
    pub was_update: bool,
}

/// Main ingestor for processing files.
pub struct Ingestor {
    db: Database,
    chunker: Chunker,
}

impl Ingestor {
    /// Create a new ingestor.
    pub fn new(db: Database, chunk_config: ChunkConfig) -> Self {
        Self {
            db,
            chunker: Chunker::new(chunk_config),
        }
    }

    /// Create an ingestor with default chunking config.
    pub fn with_defaults(db: Database) -> Self {
        Self::new(db, ChunkConfig::default())
    }

    /// Ingest a single file.
    pub fn ingest_file(&self, path: &Path) -> IngestResult<IngestResult2> {
        let path = path.canonicalize()?;
        let path_str = path.to_string_lossy().to_string();

        info!("Ingesting file: {}", path_str);

        // Detect file type
        let item_type = path
            .extension()
            .and_then(|e| e.to_str())
            .and_then(ItemType::from_extension)
            .ok_or_else(|| {
                IngestError::UnsupportedFileType(
                    path.extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("unknown")
                        .to_string(),
                )
            })?;

        // Calculate content hash
        let content_hash = self.hash_file(&path)?;

        // Check if already processed with same hash
        if let Ok(Some(existing)) = self.db.find_item_by_hash(&content_hash) {
            debug!("File already processed with same hash: {}", path_str);
            let chunks = self.db.get_chunks_by_item(&existing.id)?;
            return Ok(IngestResult2 {
                item: existing,
                chunks,
                was_update: false,
            });
        }

        // Check if path was previously ingested
        let existing_item = self.db.find_item_by_path(&path_str)?;
        let was_update = existing_item.is_some();

        // If updating, delete old chunks
        if let Some(ref old_item) = existing_item {
            debug!("Updating existing item: {}", old_item.id);
            self.db.delete_chunks_by_item(&old_item.id)?;
        }

        // Parse the document (special handling for videos)
        let (parsed, video_segments) = self.parse_file(&path, item_type)?;

        // Create or update item
        let item = if let Some(old_item) = existing_item {
            let mut item = old_item;
            item.title = parsed.title.unwrap_or_else(|| item.title.clone());
            item.content_hash = Some(content_hash);
            item.processed_at = Some(Utc::now());
            item.metadata = parsed.metadata;
            self.db.update_item(&item)?;
            item
        } else {
            let title = parsed.title.unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled")
                    .to_string()
            });

            let item = Item::new(item_type, title)
                .with_source_path(&path_str)
                .with_content_hash(&content_hash);

            let mut item = item;
            item.processed_at = Some(Utc::now());
            item.metadata = parsed.metadata;

            self.db.create_item(&item)?;
            item
        };

        // Create chunks (use transcript segments for videos if available)
        let chunks = if let Some(segments) = video_segments {
            // Convert TranscriptSegment to tuple format for chunker
            let segment_tuples: Vec<(String, f64, f64)> = segments
                .iter()
                .map(|s| (s.text.clone(), s.start, s.end))
                .collect();
            self.chunker.chunk_transcript(&item.id, &segment_tuples)
        } else {
            self.chunker.chunk_text(&item.id, &parsed.content)
        };
        debug!("Created {} chunks for item {}", chunks.len(), item.id);

        // Store chunks
        self.db.create_chunks(&chunks)?;

        // AI enrichment (summary + auto-tagging)
        if let Ok(config) = olal_config::Config::load() {
            let combined: String = chunks.iter().map(|c| c.content.as_str()).collect::<Vec<_>>().join(" ");
            let mut item = item.clone();
            if let Err(e) = crate::ai_enrich::enrich_item(&self.db, &mut item, &combined, &config) {
                warn!("AI enrichment failed: {}", e);
            }
            // Use the enriched item
            info!(
                "Successfully ingested: {} ({} chunks)",
                path_str,
                chunks.len()
            );

            return Ok(IngestResult2 {
                item,
                chunks,
                was_update,
            });
        }

        info!(
            "Successfully ingested: {} ({} chunks)",
            path_str,
            chunks.len()
        );

        Ok(IngestResult2 {
            item,
            chunks,
            was_update,
        })
    }

    /// Queue a file for processing.
    pub fn queue_file(&self, path: &Path, priority: i32) -> IngestResult<QueueItem> {
        let path = path.canonicalize()?;
        let path_str = path.to_string_lossy().to_string();

        // Check if already queued
        if self.db.is_queued(&path_str)? {
            return Err(IngestError::AlreadyProcessed(path.clone()));
        }

        // Detect file type
        let item_type = path
            .extension()
            .and_then(|e| e.to_str())
            .and_then(ItemType::from_extension)
            .ok_or_else(|| {
                IngestError::UnsupportedFileType(
                    path.extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("unknown")
                        .to_string(),
                )
            })?;

        let queue_item = QueueItem::new(&path_str, item_type).with_priority(priority);
        self.db.enqueue(&queue_item)?;

        info!("Queued file for processing: {}", path_str);
        Ok(queue_item)
    }

    /// Process the next item in the queue.
    pub fn process_next(&self) -> IngestResult<Option<IngestResult2>> {
        let queue_item = match self.db.dequeue()? {
            Some(item) => item,
            None => return Ok(None),
        };

        let path = Path::new(&queue_item.source_path);

        match self.ingest_file(path) {
            Ok(result) => {
                self.db.mark_completed(&queue_item.id)?;
                Ok(Some(result))
            }
            Err(e) => {
                warn!("Failed to process {}: {}", queue_item.source_path, e);
                self.db.mark_failed(&queue_item.id, &e.to_string())?;
                Err(e)
            }
        }
    }

    /// Process all pending items in the queue.
    pub fn process_all(&self) -> IngestResult<Vec<IngestResult2>> {
        let mut results = Vec::new();

        loop {
            match self.process_next()? {
                Some(result) => results.push(result),
                None => break,
            }
        }

        Ok(results)
    }

    /// Ingest all files in a directory.
    pub fn ingest_directory(
        &self,
        dir: &Path,
        item_type_filter: Option<ItemType>,
    ) -> IngestResult<Vec<IngestResult2>> {
        let mut results = Vec::new();

        for entry in walkdir::WalkDir::new(dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Skip directories
            if path.is_dir() {
                continue;
            }

            // Skip hidden files
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with('.'))
                .unwrap_or(false)
            {
                continue;
            }

            // Check file type
            let item_type = path
                .extension()
                .and_then(|e| e.to_str())
                .and_then(ItemType::from_extension);

            match item_type {
                Some(it) => {
                    // Apply filter if specified
                    if let Some(filter) = item_type_filter {
                        if it != filter {
                            continue;
                        }
                    }

                    match self.ingest_file(path) {
                        Ok(result) => results.push(result),
                        Err(e) => {
                            warn!("Failed to ingest {:?}: {}", path, e);
                        }
                    }
                }
                None => {
                    debug!("Skipping unsupported file: {:?}", path);
                }
            }
        }

        Ok(results)
    }

    /// Parse a file into a document.
    /// Returns (ParsedDocument, Option<Vec<TranscriptSegment>>) - segments are present for videos/audio.
    fn parse_file(
        &self,
        path: &Path,
        item_type: ItemType,
    ) -> IngestResult<(ParsedDocument, Option<Vec<TranscriptSegment>>)> {
        match item_type {
            ItemType::Video => {
                // Check if video processing tools are available
                let tools = VideoParser::tools_available();
                if !tools.all_available() {
                    if let Some(msg) = tools.missing_message() {
                        warn!("{}", msg);
                    }
                    // Fall back to placeholder if tools aren't available
                    let title = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Video")
                        .to_string();

                    return Ok((
                        ParsedDocument::new(format!("Video file: {}", path.display()))
                            .with_title(title)
                            .with_metadata(serde_json::json!({
                                "format": "video",
                                "needs_processing": true,
                                "error": "Video processing tools not installed",
                            })),
                        None,
                    ));
                }

                // Process the video
                let parser = VideoParser::with_default_model();
                let result = parser.parse(path)?;

                Ok((result.document, Some(result.segments)))
            }
            ItemType::Audio => {
                // Check if audio processing tools are available
                let tools = AudioParser::tools_available();
                if !tools.all_available() {
                    if let Some(msg) = tools.missing_message() {
                        warn!("{}", msg);
                    }
                    // Fall back to placeholder if tools aren't available
                    let title = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Audio")
                        .to_string();

                    return Ok((
                        ParsedDocument::new(format!("Audio file: {}", path.display()))
                            .with_title(title)
                            .with_metadata(serde_json::json!({
                                "format": "audio",
                                "needs_processing": true,
                                "error": "Audio processing tools not installed",
                            })),
                        None,
                    ));
                }

                // Process the audio (transcribe directly)
                let parser = AudioParser::with_default_model();
                let result = parser.parse(path)?;

                Ok((result.document, Some(result.segments)))
            }
            ItemType::Image => {
                // Images need OCR processing
                let title = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Image")
                    .to_string();

                Ok((
                    ParsedDocument::new(format!("Image file: {}", path.display()))
                        .with_title(title)
                        .with_metadata(serde_json::json!({
                            "format": "image",
                            "needs_ocr": true,
                        })),
                    None,
                ))
            }
            _ => {
                // Use text-based parsers
                Ok((parsers::parse_file(path)?, None))
            }
        }
    }

    /// Calculate SHA256 hash of a file.
    fn hash_file(&self, path: &Path) -> IngestResult<String> {
        let content = std::fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let result = hasher.finalize();
        Ok(hex::encode(result))
    }
}

// Add hex encoding utility
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{tempdir, NamedTempFile};

    #[test]
    fn test_ingest_markdown_file() {
        let db = Database::open_in_memory().unwrap();
        let ingestor = Ingestor::with_defaults(db);

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.md");
        std::fs::write(
            &file_path,
            "# Test Document\n\nThis is some content for testing.",
        )
        .unwrap();

        let result = ingestor.ingest_file(&file_path).unwrap();

        assert_eq!(result.item.item_type, ItemType::Note);
        assert_eq!(result.item.title, "Test Document");
        assert!(!result.chunks.is_empty());
        assert!(!result.was_update);
    }

    #[test]
    fn test_ingest_code_file() {
        let db = Database::open_in_memory().unwrap();
        let ingestor = Ingestor::with_defaults(db);

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.rs");
        std::fs::write(
            &file_path,
            r#"fn main() {
    println!("Hello, world!");
}
"#,
        )
        .unwrap();

        let result = ingestor.ingest_file(&file_path).unwrap();

        assert_eq!(result.item.item_type, ItemType::Code);
        assert!(result.chunks[0].content.contains("fn main()"));
    }

    #[test]
    fn test_detect_duplicate_by_hash() {
        let db = Database::open_in_memory().unwrap();
        let ingestor = Ingestor::with_defaults(db);

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "Same content").unwrap();

        // First ingest
        let result1 = ingestor.ingest_file(&file_path).unwrap();
        assert!(!result1.was_update);

        // Second ingest of same content (should detect duplicate)
        let result2 = ingestor.ingest_file(&file_path).unwrap();
        assert!(!result2.was_update);
        assert_eq!(result1.item.id, result2.item.id);
    }

    #[test]
    fn test_update_on_content_change() {
        let db = Database::open_in_memory().unwrap();
        let ingestor = Ingestor::with_defaults(db);

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");

        // First version
        std::fs::write(&file_path, "Original content").unwrap();
        let result1 = ingestor.ingest_file(&file_path).unwrap();

        // Modified version
        std::fs::write(&file_path, "Modified content").unwrap();
        let result2 = ingestor.ingest_file(&file_path).unwrap();

        assert!(result2.was_update);
        assert_eq!(result1.item.id, result2.item.id);
    }
}
