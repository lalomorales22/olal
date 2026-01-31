//! Capture command - quick thought/note capture.

use super::get_database;
use anyhow::Result;
use olal_core::{Chunk, Item, ItemType};
use chrono::Utc;
use colored::Colorize;

/// Run the capture command.
pub fn run(thought: &str, title: Option<String>, tags: Vec<String>) -> Result<()> {
    let db = get_database()?;

    // Generate a title if not provided
    let title = title.unwrap_or_else(|| {
        // Use first 50 chars of thought or timestamp
        let preview: String = thought.chars().take(50).collect();
        if preview.len() < thought.len() {
            format!("{}...", preview)
        } else if preview.is_empty() {
            format!("Note {}", Utc::now().format("%Y-%m-%d %H:%M"))
        } else {
            preview
        }
    });

    // Create the item
    let mut item = Item::new(ItemType::Note, &title);
    item.processed_at = Some(Utc::now());
    item.metadata = serde_json::json!({
        "source": "capture",
        "captured_at": Utc::now().to_rfc3339(),
    });

    db.create_item(&item)?;

    // Create a single chunk with the content
    let chunk = Chunk::new(item.id.clone(), 0, thought);
    db.create_chunks(&[chunk])?;

    // Add tags if provided
    for tag_name in &tags {
        db.tag_item(&item.id, tag_name)?;
    }

    // Display confirmation
    println!("{} Captured thought", "✓".green());
    println!();
    println!(
        "  {} {}",
        "ID:".cyan(),
        item.id.chars().take(8).collect::<String>()
    );
    println!("  {}: {}", "Title".cyan(), title);

    if !tags.is_empty() {
        println!(
            "  {}: {}",
            "Tags".cyan(),
            tags.join(", ").yellow()
        );
    }

    println!();
    println!(
        "{}",
        "Use 'olal show <id>' to view or 'olal search' to find it.".dimmed()
    );

    Ok(())
}
