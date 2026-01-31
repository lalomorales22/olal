//! Recent command - list recent items.

use super::get_database;
use anyhow::Result;
use olal_core::ItemType;
use colored::Colorize;

pub fn run(limit: i64, item_type: Option<String>) -> Result<()> {
    let db = get_database()?;
    run_with_db(&db, limit, item_type)
}

/// Run recent with an existing database connection.
pub fn run_with_db(db: &olal_db::Database, limit: i64, item_type: Option<String>) -> Result<()> {

    let item_type_filter = item_type
        .as_ref()
        .and_then(|t| ItemType::from_str(t));

    if item_type.is_some() && item_type_filter.is_none() {
        anyhow::bail!(
            "Invalid item type. Valid types: video, audio, document, note, code, image, bookmark"
        );
    }

    let items = db.list_items(item_type_filter, Some(limit))?;

    if items.is_empty() {
        println!(
            "{}",
            "No items found. Use 'olal ingest <path>' to add content.".dimmed()
        );
        return Ok(());
    }

    println!("{}", "Recent Items".cyan().bold());
    println!("{}", "─".repeat(70));

    for item in items {
        let type_icon = match item.item_type {
            ItemType::Video => "🎬",
            ItemType::Audio => "🎵",
            ItemType::Document => "📄",
            ItemType::Note => "📝",
            ItemType::Code => "💻",
            ItemType::Image => "🖼️",
            ItemType::Bookmark => "🔖",
        };

        let date = item.created_at.format("%Y-%m-%d %H:%M").to_string();

        println!(
            "{} {} {} {}",
            type_icon,
            item.title.white().bold(),
            format!("[{}]", item.id.chars().take(8).collect::<String>()).dimmed(),
            date.dimmed()
        );

        if let Some(ref path) = item.source_path {
            let short_path = if path.len() > 60 {
                format!("...{}", &path[path.len() - 57..])
            } else {
                path.clone()
            };
            println!("  {}", short_path.dimmed());
        }

        if let Some(ref summary) = item.summary {
            let short_summary = if summary.len() > 80 {
                format!("{}...", &summary[..77])
            } else {
                summary.clone()
            };
            println!("  {}", short_summary.dimmed());
        }
    }

    Ok(())
}
