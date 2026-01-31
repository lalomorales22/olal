//! Ask command - RAG-based question answering.

use super::get_database;
use anyhow::{Context, Result};
use olal_config::Config;
use olal_ollama::{rag::ContextItem, OllamaClient, RagConfig};
use colored::Colorize;
use std::io::{self, Write};
use tokio::runtime::Runtime;

/// Run the ask command.
pub fn run(
    question: &str,
    model: Option<String>,
    show_sources: bool,
    max_context: usize,
    stream: bool,
) -> Result<()> {
    let db = get_database()?;
    let config = Config::load().context("Failed to load configuration")?;
    run_with_db(&db, &config, question, model, show_sources, max_context, stream)
}

/// Run ask with an existing database connection and config.
pub fn run_with_db(
    db: &olal_db::Database,
    config: &Config,
    question: &str,
    model: Option<String>,
    show_sources: bool,
    max_context: usize,
    stream: bool,
) -> Result<()> {

    // Create Ollama client
    let client = OllamaClient::from_config(&config.ollama)
        .context("Failed to create Ollama client")?;

    // Create async runtime
    let rt = Runtime::new().context("Failed to create async runtime")?;

    // Check if Ollama is available
    let is_available = rt.block_on(client.is_available());
    if !is_available {
        anyhow::bail!(
            "Ollama is not running at {}. Start it with 'ollama serve'.",
            config.ollama.host
        );
    }

    // Check embedding stats
    let (embedded, total) = db.embedding_stats()?;
    if embedded == 0 {
        if total == 0 {
            anyhow::bail!(
                "No content in the knowledge base. Run 'olal ingest <path>' first."
            );
        } else {
            anyhow::bail!(
                "No embeddings found. Run 'olal embed --all' first to enable semantic search."
            );
        }
    }

    // First, embed the question
    let model_name = model.as_deref().unwrap_or(&config.ollama.model);
    let embedding_model = &config.ollama.embedding_model;

    println!(
        "{} {}",
        "Question:".cyan().bold(),
        question
    );
    println!("{}", "─".repeat(70));
    println!();

    // Generate embedding for the question
    let query_embedding = rt
        .block_on(client.embed(embedding_model, question))
        .context("Failed to embed question")?;

    // Search for similar chunks
    let min_similarity = 0.3;
    let results = db.vector_search(&query_embedding, max_context, Some(min_similarity))?;

    if results.is_empty() {
        println!(
            "{} No relevant content found in your knowledge base for this question.",
            "Note:".yellow()
        );
        println!();
        println!("Suggestions:");
        println!("  • Try rephrasing your question");
        println!("  • Check if relevant content has been ingested");
        println!("  • Lower the similarity threshold");
        return Ok(());
    }

    // Convert to context items
    let context: Vec<ContextItem> = results
        .iter()
        .map(|r| ContextItem {
            content: r.chunk.content.clone(),
            similarity: r.similarity,
            item_id: r.item_id.clone(),
            item_title: r.item_title.clone(),
        })
        .collect();

    // Build RAG config
    let rag_config = RagConfig {
        model: model_name.to_string(),
        embedding_model: embedding_model.to_string(),
        max_context_chunks: max_context,
        min_similarity,
        temperature: 0.7,
    };

    // Generate answer
    if stream {
        // Streaming response
        let (mut rx, sources) = rt
            .block_on(client.rag_query_stream(question, &context, &rag_config))
            .context("Failed to generate answer")?;

        print!("{} ", "Answer:".green().bold());
        io::stdout().flush()?;

        rt.block_on(async {
            while let Some(chunk) = rx.recv().await {
                print!("{}", chunk);
                io::stdout().flush().ok();
            }
        });

        println!();
        println!();

        // Show sources
        if show_sources && !sources.is_empty() {
            println!("{}", "─".repeat(70));
            println!("{}", "Sources:".cyan().bold());
            for (i, source) in sources.iter().enumerate() {
                println!(
                    "  {}. {} {} (similarity: {:.0}%)",
                    i + 1,
                    source.item_title.white(),
                    format!("[{}]", &source.item_id[..8]).dimmed(),
                    source.similarity * 100.0
                );
            }
        }
    } else {
        // Non-streaming response
        let response = rt
            .block_on(client.rag_query(question, &context, &rag_config))
            .context("Failed to generate answer")?;

        println!("{}", "Answer:".green().bold());
        println!();
        println!("{}", response.answer);
        println!();

        // Show sources
        if show_sources && !response.sources.is_empty() {
            println!("{}", "─".repeat(70));
            println!("{}", "Sources:".cyan().bold());
            for (i, source) in response.sources.iter().enumerate() {
                println!(
                    "  {}. {} {} (similarity: {:.0}%)",
                    i + 1,
                    source.item_title.white(),
                    format!("[{}]", &source.item_id[..8]).dimmed(),
                    source.similarity * 100.0
                );
            }
        }
    }

    Ok(())
}
