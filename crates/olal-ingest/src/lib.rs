//! Olal Ingest - File ingestion and processing pipeline.
//!
//! This crate provides:
//! - File system watching for automatic ingestion
//! - Document parsing (markdown, text, PDF, audio)
//! - Content chunking for RAG
//! - Processing queue management
//! - AI-based enrichment (summarization, auto-tagging)

pub mod ai_enrich;
mod chunker;
mod error;
mod ingestor;
mod parsers;
mod watcher;

pub use chunker::{ChunkConfig, Chunker};
pub use error::{IngestError, IngestResult};
pub use ingestor::Ingestor;
pub use watcher::{FileWatcher, WatchEvent, WatcherConfig};
