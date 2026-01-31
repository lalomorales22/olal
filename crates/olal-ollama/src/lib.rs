//! Olal Ollama - Ollama integration for embeddings, semantic search, and RAG.
//!
//! This crate provides async clients for interacting with Ollama's API,
//! including embedding generation, text generation, and RAG-based queries.

mod client;
mod error;
pub mod rag;
mod types;

pub use client::OllamaClient;
pub use error::{OllamaError, OllamaResult};
pub use rag::{RagConfig, RagResponse, SourceReference};
pub use types::*;
