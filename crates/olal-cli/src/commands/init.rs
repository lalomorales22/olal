//! Initialize Olal.

use super::get_paths;
use anyhow::{Context, Result};
use olal_config::Config;
use olal_db::Database;
use colored::Colorize;

pub fn run() -> Result<()> {
    let paths = get_paths()?;

    // Check if already initialized
    if paths.is_initialized() {
        println!(
            "{} Olal is already initialized.",
            "Note:".yellow().bold()
        );
        println!("  Config: {}", paths.config_file.display());
        println!("  Database: {}", paths.database_file.display());
        return Ok(());
    }

    println!("{}", "Initializing Olal...".cyan().bold());

    // Create directories
    paths
        .ensure_dirs()
        .context("Failed to create directories")?;
    println!("  {} Created directories", "✓".green());

    // Create config file
    Config::create_default_file(&paths.config_file)
        .context("Failed to create config file")?;
    println!(
        "  {} Created config: {}",
        "✓".green(),
        paths.config_file.display()
    );

    // Initialize database
    let _db = Database::open(&paths.database_file).context("Failed to initialize database")?;
    println!(
        "  {} Created database: {}",
        "✓".green(),
        paths.database_file.display()
    );

    println!();
    println!("{}", "Olal initialized successfully!".green().bold());
    println!();
    println!("Next steps:");
    println!(
        "  1. Edit config: {}",
        "olal config edit".cyan()
    );
    println!(
        "  2. Add watch directories: {}",
        "olal config add-watch ~/Movies/ScreenRecordings".cyan()
    );
    println!(
        "  3. Check status: {}",
        "olal status".cyan()
    );

    Ok(())
}
