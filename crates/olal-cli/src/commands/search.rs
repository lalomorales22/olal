//! Search command - full-text and semantic search.

use super::get_database;
use anyhow::{Context, Result};
use olal_config::Config;
use olal_core::ItemType;
use olal_ollama::OllamaClient;
use colored::Colorize;
use tokio::runtime::Runtime;

pub fn run(query: &str, limit: i64, semantic: bool) -> Result<()> {
    let db = get_database()?;
    run_with_db(&db, query, limit, semantic)
}

/// Run search with an existing database connection.
pub fn run_with_db(db: &olal_db::Database, query: &str, limit: i64, semantic: bool) -> Result<()> {
    if semantic {
        run_semantic_search(db, query, limit as usize)
    } else {
        run_fts_search(db, query, limit)
    }
}

/// Run full-text search (original behavior).
fn run_fts_search(db: &olal_db::Database, query: &str, limit: i64) -> Result<()> {
    println!(
        "{} \"{}\"",
        "Searching for:".cyan().bold(),
        query
    );
    println!("{}", "─".repeat(70));

    let items = db.search_items(query, Some(limit))?;

    if items.is_empty() {
        println!();
        println!("{}", "No results found.".dimmed());
        println!();
        println!("Tips:");
        println!("  • Try different keywords");
        println!("  • Use 'olal recent' to browse items");
        println!("  • Make sure content has been processed");
        println!("  • Try {} for meaning-based search", "--semantic".cyan());
        return Ok(());
    }

    println!();
    println!(
        "Found {} result{}",
        items.len().to_string().green(),
        if items.len() == 1 { "" } else { "s" }
    );
    println!();

    for item in items {
        print_item(&item.item_type, &item.title, &item.id, item.summary.as_deref(), None);
    }

    Ok(())
}

/// Run semantic (vector) search.
fn run_semantic_search(db: &olal_db::Database, query: &str, limit: usize) -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;

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

    println!(
        "{} \"{}\" {}",
        "Semantic search for:".cyan().bold(),
        query,
        "(meaning-based)".dimmed()
    );
    println!("{}", "─".repeat(70));

    // Generate embedding for the query
    let query_embedding = rt
        .block_on(client.embed(&config.ollama.embedding_model, query))
        .context("Failed to embed query")?;

    // Search for similar chunks
    let results = db.vector_search(&query_embedding, limit, Some(0.2))?;

    if results.is_empty() {
        println!();
        println!("{}", "No similar content found.".dimmed());
        println!();
        println!("Tips:");
        println!("  • Try rephrasing your query");
        println!("  • Run 'olal embed --all' to generate more embeddings");
        println!("  • Try regular search without {}", "--semantic".cyan());
        return Ok(());
    }

    println!();
    println!(
        "Found {} similar chunk{}",
        results.len().to_string().green(),
        if results.len() == 1 { "" } else { "s" }
    );
    println!();

    // Group by item to avoid duplicates
    use std::collections::HashMap;
    let mut items_seen: HashMap<String, (String, String, f32, String)> = HashMap::new();

    for result in &results {
        let item_id = &result.item_id;

        items_seen
            .entry(item_id.clone())
            .and_modify(|(_, _, sim, content)| {
                // Keep the highest similarity and best content snippet
                if result.similarity > *sim {
                    *sim = result.similarity;
                    *content = truncate(&result.chunk.content, 150);
                }
            })
            .or_insert_with(|| {
                (
                    result.item_title.clone(),
                    item_id.clone(),
                    result.similarity,
                    truncate(&result.chunk.content, 150),
                )
            });
    }

    // Sort by similarity
    let mut items: Vec<_> = items_seen.into_values().collect();
    items.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

    for (title, id, similarity, snippet) in items {
        println!(
            "{} {} {}",
            "•".cyan(),
            title.white().bold(),
            format!("[{}]", &id[..8]).dimmed()
        );
        println!(
            "  {} {:.0}%",
            "Similarity:".dimmed(),
            similarity * 100.0
        );
        println!("  {}", snippet.dimmed());
        println!();
    }

    Ok(())
}

/// Print an item result.
fn print_item(
    item_type: &ItemType,
    title: &str,
    id: &str,
    summary: Option<&str>,
    similarity: Option<f32>,
) {
    let type_icon = match item_type {
        ItemType::Video => "🎬",
        ItemType::Audio => "🎵",
        ItemType::Document => "📄",
        ItemType::Note => "📝",
        ItemType::Code => "💻",
        ItemType::Image => "🖼️",
        ItemType::Bookmark => "🔖",
    };

    println!(
        "{} {} {}",
        type_icon,
        title.white().bold(),
        format!("[{}]", id.chars().take(8).collect::<String>()).dimmed()
    );

    if let Some(sim) = similarity {
        println!("  {} {:.0}%", "Similarity:".dimmed(), sim * 100.0);
    }

    if let Some(summary) = summary {
        let short_summary = truncate(summary, 100);
        println!("  {}", short_summary.dimmed());
    }

    println!();
}

/// Truncate a string to a maximum length.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }
}
