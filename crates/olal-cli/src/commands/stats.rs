//! Stats command - show database statistics.

use super::{format_size, get_database};
use anyhow::Result;
use colored::Colorize;

pub fn run() -> Result<()> {
    let db = get_database()?;
    run_with_db(&db)
}

/// Run stats with an existing database connection.
pub fn run_with_db(db: &olal_db::Database) -> Result<()> {
    let stats = db.get_stats()?;

    println!("{}", "Olal Statistics".cyan().bold());
    println!("{}", "─".repeat(50));

    // Knowledge Base
    println!();
    println!("{}", "Knowledge Base".white().bold());
    println!("  Total items: {}", stats.total_items.to_string().green());

    if !stats.items_by_type.is_empty() {
        for (item_type, count) in &stats.items_by_type {
            let icon = match item_type.as_str() {
                "video" => "🎬",
                "audio" => "🎵",
                "document" => "📄",
                "note" => "📝",
                "code" => "💻",
                "image" => "🖼️",
                "bookmark" => "🔖",
                _ => "📁",
            };
            println!("    {} {}: {}", icon, item_type, count);
        }
    }

    println!("  Total chunks: {}", stats.total_chunks);

    // Organization
    println!();
    println!("{}", "Organization".white().bold());
    println!("  Projects: {}", stats.total_projects);
    println!("  Tags: {}", stats.total_tags);

    // Tasks
    println!();
    println!("{}", "Tasks".white().bold());
    println!("  Total: {}", stats.total_tasks);
    println!("  Pending: {}", stats.pending_tasks.to_string().yellow());

    // Processing Queue
    println!();
    println!("{}", "Processing Queue".white().bold());
    println!("  Pending: {}", stats.queue_pending);
    println!("  Processing: {}", stats.queue_processing);
    if stats.queue_failed > 0 {
        println!("  Failed: {}", stats.queue_failed.to_string().red());
    }

    // Storage
    println!();
    println!("{}", "Storage".white().bold());
    println!("  Database size: {}", format_size(stats.database_size_bytes));

    Ok(())
}
