//! Embed command - generate embeddings for chunks.

use super::get_database;
use anyhow::{Context, Result};
use olal_config::Config;
use olal_ollama::OllamaClient;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::runtime::Runtime;

/// Run the embed command.
pub fn run(all: bool, item_id: Option<String>, batch_size: usize) -> Result<()> {
    let db = get_database()?;
    let config = Config::load().context("Failed to load configuration")?;

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

    // Check if embedding model is available
    let has_model = rt
        .block_on(client.has_model(&config.ollama.embedding_model))
        .unwrap_or(false);

    if !has_model {
        println!(
            "{} Model '{}' not found. Downloading...",
            "Note:".yellow(),
            config.ollama.embedding_model
        );
        println!(
            "Run: {}",
            format!("ollama pull {}", config.ollama.embedding_model).cyan()
        );
        anyhow::bail!(
            "Model '{}' is not available. Run 'ollama pull {}' first.",
            config.ollama.embedding_model,
            config.ollama.embedding_model
        );
    }

    if let Some(ref id) = item_id {
        // Embed chunks for a specific item
        embed_item(&db, &client, &config.ollama.embedding_model, id, &rt)?;
    } else if all {
        // Embed all unembedded chunks
        embed_all(&db, &client, &config.ollama.embedding_model, batch_size, &rt)?;
    } else {
        // Show stats and usage
        let (embedded, total) = db.embedding_stats()?;
        println!("{}", "Embedding Statistics".cyan().bold());
        println!("{}", "─".repeat(40));
        println!(
            "Embedded chunks: {} / {}",
            embedded.to_string().green(),
            total
        );

        if total > embedded {
            let remaining = total - embedded;
            println!(
                "\n{} {} chunks need embeddings.",
                "→".yellow(),
                remaining
            );
            println!("\nUsage:");
            println!("  {} Embed all unembedded chunks", "olal embed --all".cyan());
            println!(
                "  {} Embed chunks for a specific item",
                "olal embed --item <ID>".cyan()
            );
        } else if total == 0 {
            println!(
                "\n{} No chunks to embed. Ingest some content first.",
                "Note:".yellow()
            );
            println!("  {}", "olal ingest <path>".cyan());
        } else {
            println!("\n{} All chunks have embeddings!", "✓".green());
        }
    }

    Ok(())
}

/// Embed chunks for a specific item.
fn embed_item(
    db: &olal_db::Database,
    client: &OllamaClient,
    model: &str,
    item_id: &str,
    rt: &Runtime,
) -> Result<()> {
    // Try to find the item (support partial ID)
    let item = db
        .get_item_by_prefix(item_id)
        .context("Item not found")?;

    println!(
        "{} {} [{}]",
        "Embedding:".cyan().bold(),
        item.title,
        &item.id[..8]
    );

    let chunks = db.get_chunks_by_item(&item.id)?;

    if chunks.is_empty() {
        println!("{} No chunks found for this item.", "Note:".yellow());
        return Ok(());
    }

    let pb = ProgressBar::new(chunks.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("█▓░"),
    );

    let mut embedded = 0;
    let mut skipped = 0;

    for chunk in &chunks {
        // Check if already embedded
        if db.get_embedding(&chunk.id)?.is_some() {
            skipped += 1;
            pb.inc(1);
            continue;
        }

        // Generate embedding
        match rt.block_on(client.embed(model, &chunk.content)) {
            Ok(embedding) => {
                db.store_embedding(&chunk.id, &embedding, model)?;
                embedded += 1;
            }
            Err(e) => {
                pb.println(format!(
                    "{} Failed to embed chunk {}: {}",
                    "Warning:".yellow(),
                    &chunk.id[..8],
                    e
                ));
            }
        }

        pb.inc(1);
    }

    pb.finish_and_clear();

    println!(
        "{} {} new embeddings, {} already embedded",
        "Done:".green().bold(),
        embedded.to_string().green(),
        skipped
    );

    Ok(())
}

/// Embed all unembedded chunks.
fn embed_all(
    db: &olal_db::Database,
    client: &OllamaClient,
    model: &str,
    batch_size: usize,
    rt: &Runtime,
) -> Result<()> {
    let (embedded_count, total_count) = db.embedding_stats()?;
    let remaining = total_count - embedded_count;

    if remaining == 0 {
        println!("{} All chunks already have embeddings!", "✓".green());
        return Ok(());
    }

    println!(
        "{} Generating embeddings for {} chunks using '{}'",
        "→".cyan(),
        remaining.to_string().yellow(),
        model.cyan()
    );
    println!("{}", "─".repeat(60));

    let pb = ProgressBar::new(remaining as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("█▓░"),
    );

    let mut total_embedded = 0;
    let mut errors = 0;

    loop {
        let chunks = db.get_unembedded_chunks(batch_size)?;

        if chunks.is_empty() {
            break;
        }

        for chunk in &chunks {
            match rt.block_on(client.embed(model, &chunk.content)) {
                Ok(embedding) => {
                    db.store_embedding(&chunk.id, &embedding, model)?;
                    total_embedded += 1;
                }
                Err(e) => {
                    errors += 1;
                    pb.println(format!(
                        "{} Chunk {}: {}",
                        "Error:".red(),
                        &chunk.id[..8],
                        e
                    ));
                }
            }

            pb.inc(1);
        }
    }

    pb.finish_and_clear();

    println!();
    println!("{}", "─".repeat(60));
    println!(
        "{} Generated {} embeddings",
        "✓".green(),
        total_embedded.to_string().green()
    );

    if errors > 0 {
        println!(
            "{} {} chunks failed",
            "⚠".yellow(),
            errors.to_string().yellow()
        );
    }

    Ok(())
}
