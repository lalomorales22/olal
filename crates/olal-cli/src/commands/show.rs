//! Show command - display item details.

use super::get_database;
use anyhow::Result;
use olal_core::ItemType;
use colored::Colorize;
use serde_json;

pub fn run(id: &str) -> Result<()> {
    let db = get_database()?;
    run_with_db(&db, id)
}

/// Run show with an existing database connection.
pub fn run_with_db(db: &olal_db::Database, id: &str) -> Result<()> {

    let item = db.get_item(id)?;

    let type_icon = match item.item_type {
        ItemType::Video => "🎬",
        ItemType::Audio => "🎵",
        ItemType::Document => "📄",
        ItemType::Note => "📝",
        ItemType::Code => "💻",
        ItemType::Image => "🖼️",
        ItemType::Bookmark => "🔖",
    };

    println!("{} {}", type_icon, item.title.white().bold());
    println!("{}", "─".repeat(70));

    println!("  {}: {}", "ID".cyan(), item.id);
    println!("  {}: {}", "Type".cyan(), item.item_type);
    println!(
        "  {}: {}",
        "Created".cyan(),
        item.created_at.format("%Y-%m-%d %H:%M:%S")
    );

    if let Some(processed) = item.processed_at {
        println!(
            "  {}: {}",
            "Processed".cyan(),
            processed.format("%Y-%m-%d %H:%M:%S")
        );
    }

    if let Some(ref path) = item.source_path {
        println!("  {}: {}", "Source".cyan(), path);
    }

    if let Some(ref hash) = item.content_hash {
        println!("  {}: {}", "Hash".cyan(), hash);
    }

    // Tags
    let tags = db.get_item_tags(&item.id)?;
    if !tags.is_empty() {
        let tag_names: Vec<String> = tags.iter().map(|t| t.name.clone()).collect();
        println!("  {}: {}", "Tags".cyan(), tag_names.join(", ").yellow());
    }

    // Summary
    if let Some(ref summary) = item.summary {
        println!();
        println!("{}", "Summary".white().bold());
        println!("{}", "─".repeat(70));
        println!("{}", summary);
    }

    // Chunks preview
    let chunks = db.get_chunks_by_item(&item.id)?;
    if !chunks.is_empty() {
        println!();
        println!(
            "{} ({} chunks)",
            "Content Preview".white().bold(),
            chunks.len()
        );
        println!("{}", "─".repeat(70));

        // Show first few chunks
        for chunk in chunks.iter().take(3) {
            let preview = if chunk.content.len() > 200 {
                format!("{}...", &chunk.content[..197])
            } else {
                chunk.content.clone()
            };

            if let (Some(start), Some(end)) = (chunk.start_time, chunk.end_time) {
                println!(
                    "[{:.1}s - {:.1}s]",
                    start,
                    end
                );
            }
            println!("{}", preview.dimmed());
            println!();
        }

        if chunks.len() > 3 {
            println!(
                "{}",
                format!("... and {} more chunks", chunks.len() - 3).dimmed()
            );
        }
    }

    // Metadata
    if !item.metadata.is_null() && item.metadata != serde_json::json!({}) {
        println!();
        println!("{}", "Metadata".white().bold());
        println!("{}", "─".repeat(70));
        println!(
            "{}",
            serde_json::to_string_pretty(&item.metadata)?.dimmed()
        );
    }

    Ok(())
}
