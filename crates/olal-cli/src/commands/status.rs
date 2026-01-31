//! Status command - show processing queue status.

use super::get_database;
use anyhow::Result;
use colored::Colorize;

pub fn run() -> Result<()> {
    let db = get_database()?;

    println!("{}", "Olal Status".cyan().bold());
    println!("{}", "─".repeat(50));

    // Queue status
    let (pending, processing, done, failed) = db.queue_counts()?;

    println!();
    println!("{}", "Processing Queue".white().bold());
    println!(
        "  {} Pending: {}",
        "○".yellow(),
        pending
    );
    println!(
        "  {} Processing: {}",
        "◐".blue(),
        processing
    );
    println!(
        "  {} Completed: {}",
        "●".green(),
        done
    );
    if failed > 0 {
        println!(
            "  {} Failed: {}",
            "✗".red(),
            failed
        );
    }

    // Show pending items
    let pending_items = db.list_queue(Some(olal_core::QueueStatus::Pending))?;
    if !pending_items.is_empty() {
        println!();
        println!("{}", "Pending Items".white().bold());
        for item in pending_items.iter().take(5) {
            let path = std::path::Path::new(&item.source_path);
            let filename = path.file_name()
                .map(|s| s.to_string_lossy())
                .unwrap_or_else(|| item.source_path.as_str().into());
            println!(
                "  {} {} ({})",
                "•".dimmed(),
                filename,
                item.item_type
            );
        }
        if pending_items.len() > 5 {
            println!(
                "  {} ...and {} more",
                "".dimmed(),
                pending_items.len() - 5
            );
        }
    }

    // Show processing items
    let processing_items = db.list_queue(Some(olal_core::QueueStatus::Processing))?;
    if !processing_items.is_empty() {
        println!();
        println!("{}", "Currently Processing".white().bold());
        for item in &processing_items {
            let path = std::path::Path::new(&item.source_path);
            let filename = path.file_name()
                .map(|s| s.to_string_lossy())
                .unwrap_or_else(|| item.source_path.as_str().into());
            println!(
                "  {} {} (attempt {})",
                "▶".blue(),
                filename,
                item.attempts
            );
        }
    }

    // Show failed items
    let failed_items = db.list_queue(Some(olal_core::QueueStatus::Failed))?;
    if !failed_items.is_empty() {
        println!();
        println!("{}", "Failed Items".red().bold());
        for item in failed_items.iter().take(3) {
            let path = std::path::Path::new(&item.source_path);
            let filename = path.file_name()
                .map(|s| s.to_string_lossy())
                .unwrap_or_else(|| item.source_path.as_str().into());
            println!(
                "  {} {}",
                "✗".red(),
                filename
            );
            if let Some(ref err) = item.error {
                println!(
                    "    {}",
                    err.dimmed()
                );
            }
        }
    }

    if pending == 0 && processing == 0 {
        println!();
        println!(
            "{}",
            "No items in queue. Use 'olal ingest <path>' to add content.".dimmed()
        );
    }

    Ok(())
}
