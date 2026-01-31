//! Watch command implementation.

use anyhow::Result;
use olal_config::Config;
use olal_db::Database;
use olal_ingest::{ChunkConfig, FileWatcher, Ingestor, WatchEvent, WatcherConfig};
use colored::Colorize;
use std::time::Duration;
use tracing::{error, info};

/// Start the file watcher.
pub fn run(daemon: bool) -> Result<()> {
    let config = Config::load().unwrap_or_default();
    let paths = olal_config::AppPaths::new()
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;

    if config.watch.directories.is_empty() {
        println!("{}", "No watch directories configured.".yellow());
        println!("Add directories with: olal config add-watch <path>");
        return Ok(());
    }

    if daemon {
        // For daemon mode, we'd typically fork the process
        // For now, just run in foreground with a message
        println!("{}", "Daemon mode not yet implemented. Running in foreground.".yellow());
    }

    // Check external tools
    let tools = olal_process::check_dependencies();
    let missing: Vec<_> = tools.iter().filter(|(_, available)| !available).collect();
    if !missing.is_empty() {
        println!("{}", "Warning: Some processing tools are not available:".yellow());
        for (tool, _) in &missing {
            println!("  - {}", tool);
        }
        println!("Video processing features will be limited.\n");
    }

    println!("{}", "Starting file watcher...".cyan());
    println!("Watching directories:");
    for dir in &config.watch.directories {
        let expanded = shellexpand::tilde(dir);
        let path = std::path::Path::new(expanded.as_ref());
        if path.exists() {
            println!("  {} {}", "+".green(), dir);
        } else {
            println!("  {} {} (not found)", "-".red(), dir);
        }
    }
    println!("\nPress Ctrl+C to stop.\n");

    // Set up the watcher
    let watcher_config = WatcherConfig::from_config(&config.watch);
    let mut watcher = FileWatcher::new(watcher_config)?;
    watcher.start()?;

    // Set up the ingestor
    let db = Database::open(&paths.database_file)?;
    let chunk_config = ChunkConfig::from_processing_config(&config.processing);
    let ingestor = Ingestor::new(db, chunk_config);

    // Main watch loop
    loop {
        // Poll for events (with timeout to allow ctrl+c)
        std::thread::sleep(Duration::from_millis(100));

        for event in watcher.poll() {
            match event {
                WatchEvent::FileChanged { path, item_type } => {
                    info!("File changed: {:?}", path);
                    println!(
                        "{} {} [{}]",
                        "New file:".green(),
                        path.display(),
                        item_type
                    );

                    // Queue the file for processing
                    match ingestor.queue_file(&path, 0) {
                        Ok(item) => {
                            println!(
                                "  {} ({})",
                                "Queued".cyan(),
                                &item.id[..8]
                            );
                        }
                        Err(olal_ingest::IngestError::AlreadyProcessed(_)) => {
                            println!("  {}", "Already in queue".yellow());
                        }
                        Err(e) => {
                            error!("Failed to queue file: {}", e);
                            println!("  {} {}", "Error:".red(), e);
                        }
                    }
                }
                WatchEvent::FileDeleted { path } => {
                    info!("File deleted: {:?}", path);
                    println!(
                        "{} {}",
                        "Deleted:".yellow(),
                        path.display()
                    );
                    // Note: We don't remove items when files are deleted
                    // as the user might want to keep the indexed content
                }
                WatchEvent::Error(msg) => {
                    error!("Watch error: {}", msg);
                    println!("{} {}", "Watch error:".red(), msg);
                }
            }
        }
    }
}

/// Stop the daemon watcher.
pub fn stop() -> Result<()> {
    // For now, daemon mode isn't fully implemented
    // This would look for a PID file and send SIGTERM
    println!("{}", "Daemon mode not yet implemented.".yellow());
    Ok(())
}

/// Show watch status.
pub fn status() -> Result<()> {
    let config = Config::load().unwrap_or_default();

    println!("{}", "Watch Configuration".cyan().bold());
    println!();

    if config.watch.directories.is_empty() {
        println!("{}", "No directories configured.".yellow());
    } else {
        println!("Directories:");
        for dir in &config.watch.directories {
            let expanded = shellexpand::tilde(dir);
            let path = std::path::Path::new(expanded.as_ref());
            if path.exists() {
                println!("  {} {} (exists)", "+".green(), dir);
            } else {
                println!("  {} {} (not found)", "-".red(), dir);
            }
        }
    }

    println!();
    println!("Ignore patterns:");
    for pattern in &config.watch.ignore_patterns {
        println!("  - {}", pattern);
    }

    println!();
    println!("Poll interval: {}s", config.watch.poll_interval_seconds);

    // Check tools
    println!();
    println!("Processing tools:");
    for (tool, available) in olal_process::check_dependencies() {
        if available {
            println!("  {} {} (installed)", "+".green(), tool);
        } else {
            println!("  {} {} (not found)", "-".red(), tool);
        }
    }

    Ok(())
}
