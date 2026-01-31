//! Tag management commands.

use super::get_database;
use anyhow::Result;
use colored::Colorize;

pub fn add(item_id: &str, tag_name: &str) -> Result<()> {
    let db = get_database()?;

    // Verify item exists
    let item = db.get_item(item_id)?;

    // Add tag (creates if doesn't exist)
    let tag = db.tag_item(&item.id, tag_name)?;

    println!(
        "{} Tagged '{}' with '{}'",
        "✓".green(),
        item.title.white(),
        tag.name.yellow()
    );

    Ok(())
}

pub fn list() -> Result<()> {
    let db = get_database()?;

    let tag_counts = db.get_tag_counts()?;

    if tag_counts.is_empty() {
        println!(
            "{}",
            "No tags found. Use 'olal tag <item-id> <tag>' to create one.".dimmed()
        );
        return Ok(());
    }

    println!("{}", "Tags".cyan().bold());
    println!("{}", "─".repeat(50));

    for (tag, count) in tag_counts {
        let color_indicator = if let Some(ref color) = tag.color {
            format!(" ({})", color)
        } else {
            String::new()
        };

        println!(
            "  {} {}{} ({})",
            "•".yellow(),
            tag.name.white(),
            color_indicator.dimmed(),
            count
        );
    }

    Ok(())
}
