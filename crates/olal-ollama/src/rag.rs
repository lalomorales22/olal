//! RAG (Retrieval-Augmented Generation) query engine.

use crate::client::OllamaClient;
use crate::error::{OllamaError, OllamaResult};
use crate::types::{GenerateOptions, GenerateRequest};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Configuration for RAG queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    /// Model to use for generation.
    pub model: String,
    /// Model to use for embeddings.
    pub embedding_model: String,
    /// Maximum number of context chunks to include.
    pub max_context_chunks: usize,
    /// Minimum similarity score for context (0.0 to 1.0).
    pub min_similarity: f32,
    /// Temperature for generation (0.0 to 2.0).
    pub temperature: f32,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            model: "gpt-oss:20b".to_string(),
            embedding_model: "nomic-embed-text".to_string(),
            max_context_chunks: 5,
            min_similarity: 0.3,
            temperature: 0.7,
        }
    }
}

/// A reference to a source used in the answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceReference {
    /// ID of the source item.
    pub item_id: String,
    /// Title of the source item.
    pub item_title: String,
    /// The chunk content that was used.
    pub chunk_content: String,
    /// Similarity score (0.0 to 1.0).
    pub similarity: f32,
}

/// Response from a RAG query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagResponse {
    /// The generated answer.
    pub answer: String,
    /// Sources used to generate the answer.
    pub sources: Vec<SourceReference>,
}

/// Context item for RAG queries (from vector search results).
#[derive(Debug, Clone)]
pub struct ContextItem {
    /// The text content of the chunk.
    pub content: String,
    /// Similarity score.
    pub similarity: f32,
    /// ID of the parent item.
    pub item_id: String,
    /// Title of the parent item.
    pub item_title: String,
}

/// Build the RAG prompt with context.
pub fn build_rag_prompt(question: &str, context: &[ContextItem]) -> String {
    let mut prompt = String::new();

    // Add context section
    prompt.push_str("Use the following context to answer the question. If the context doesn't contain relevant information, say so.\n\n");
    prompt.push_str("Context:\n");
    prompt.push_str("─────────────────────────────────────\n");

    for (i, item) in context.iter().enumerate() {
        prompt.push_str(&format!("\n[{}] From: {}\n", i + 1, item.item_title));
        prompt.push_str(&item.content);
        prompt.push_str("\n");
    }

    prompt.push_str("\n─────────────────────────────────────\n\n");
    prompt.push_str(&format!("Question: {}\n\n", question));
    prompt.push_str("Answer:");

    prompt
}

/// Build the system prompt for RAG.
pub fn build_system_prompt() -> String {
    r#"You are a helpful assistant that answers questions based on the provided context from a personal knowledge base.

Guidelines:
- Base your answers on the context provided
- If the context doesn't contain enough information, acknowledge that
- Be concise but thorough
- When relevant, mention which source(s) your answer is based on
- Do not make up information not present in the context"#
        .to_string()
}

impl OllamaClient {
    /// Perform a RAG query with the given context.
    pub async fn rag_query(
        &self,
        question: &str,
        context: &[ContextItem],
        config: &RagConfig,
    ) -> OllamaResult<RagResponse> {
        if context.is_empty() {
            return Err(OllamaError::NoContext);
        }

        // Build the prompt
        let prompt = build_rag_prompt(question, context);
        let system = build_system_prompt();

        // Create the request
        let request = GenerateRequest::new(&config.model, prompt)
            .with_system(system)
            .with_options(GenerateOptions::new().with_temperature(config.temperature));

        // Generate the response
        let response = self.generate(request).await?;

        // Build source references
        let sources: Vec<SourceReference> = context
            .iter()
            .map(|c| SourceReference {
                item_id: c.item_id.clone(),
                item_title: c.item_title.clone(),
                chunk_content: truncate_content(&c.content, 200),
                similarity: c.similarity,
            })
            .collect();

        Ok(RagResponse {
            answer: response.response,
            sources,
        })
    }

    /// Perform a RAG query with streaming response.
    /// Returns a channel receiver for response chunks and the sources.
    pub async fn rag_query_stream(
        &self,
        question: &str,
        context: &[ContextItem],
        config: &RagConfig,
    ) -> OllamaResult<(mpsc::Receiver<String>, Vec<SourceReference>)> {
        if context.is_empty() {
            return Err(OllamaError::NoContext);
        }

        // Build the prompt
        let prompt = build_rag_prompt(question, context);
        let system = build_system_prompt();

        // Create the request
        let request = GenerateRequest::new(&config.model, prompt)
            .with_system(system)
            .with_stream(true)
            .with_options(GenerateOptions::new().with_temperature(config.temperature));

        // Start streaming
        let rx = self.generate_stream(request).await?;

        // Build source references
        let sources: Vec<SourceReference> = context
            .iter()
            .map(|c| SourceReference {
                item_id: c.item_id.clone(),
                item_title: c.item_title.clone(),
                chunk_content: truncate_content(&c.content, 200),
                similarity: c.similarity,
            })
            .collect();

        Ok((rx, sources))
    }
}

/// Truncate content to a maximum length, adding ellipsis if needed.
fn truncate_content(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        content.to_string()
    } else {
        let truncated: String = content.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_rag_prompt() {
        let context = vec![
            ContextItem {
                content: "Olal is a knowledge management system.".to_string(),
                similarity: 0.9,
                item_id: "id1".to_string(),
                item_title: "README".to_string(),
            },
            ContextItem {
                content: "It uses SQLite for storage.".to_string(),
                similarity: 0.8,
                item_id: "id2".to_string(),
                item_title: "Architecture".to_string(),
            },
        ];

        let prompt = build_rag_prompt("What is Olal?", &context);

        assert!(prompt.contains("What is Olal?"));
        assert!(prompt.contains("README"));
        assert!(prompt.contains("Olal is a knowledge management system"));
        assert!(prompt.contains("Architecture"));
    }

    #[test]
    fn test_truncate_content() {
        let short = "Hello";
        assert_eq!(truncate_content(short, 10), "Hello");

        let long = "This is a very long string that should be truncated";
        let truncated = truncate_content(long, 20);
        assert!(truncated.len() <= 20);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_rag_config_default() {
        let config = RagConfig::default();
        assert_eq!(config.max_context_chunks, 5);
        assert_eq!(config.min_similarity, 0.3);
        assert_eq!(config.temperature, 0.7);
    }
}
