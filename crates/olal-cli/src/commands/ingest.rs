//! Ingest command implementation.

use anyhow::Result;
use olal_config::Config;
use olal_core::ItemType;
use olal_db::Database;
use olal_ingest::{ChunkConfig, Ingestor};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

/// Ingest a single file or directory.
pub fn run(
    path: &str,
    item_type_filter: Option<String>,
    dry_run: bool,
    queue: bool,
) -> Result<()> {
    let config = Config::load().unwrap_or_default();
    let paths = olal_config::AppPaths::new().ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
    let db = Database::open(&paths.database_file)?;

    let path = Path::new(path);
    if !path.exists() {
        return Err(anyhow::anyhow!("Path does not exist: {}", path.display()));
    }

    // Parse type filter
    let type_filter = item_type_filter
        .as_ref()
        .and_then(|t| ItemType::from_str(t));

    if let Some(ref filter_str) = item_type_filter {
        if type_filter.is_none() {
            return Err(anyhow::anyhow!("Unknown item type: {}", filter_str));
        }
    }

    // Create ingestor with config-based chunking settings
    let chunk_config = ChunkConfig::from_processing_config(&config.processing);
    let ingestor = Ingestor::new(db, chunk_config);

    if path.is_file() {
        // Single file
        if dry_run {
            println!("{} {}", "Would ingest:".cyan(), path.display());
            if let Some(it) = path.extension().and_then(|e| e.to_str()).and_then(ItemType::from_extension) {
                println!("  Type: {}", it);
            }
            return Ok(());
        }

        if queue {
            // Add to queue for background processing
            let item = ingestor.queue_file(path, 0)?;
            println!(
                "{} {} (queue id: {})",
                "Queued:".green().bold(),
                path.display(),
                &item.id[..8]
            );
        } else {
            // Process immediately
            let pb = ProgressBar::new_spinner();
            pb.set_style(ProgressStyle::default_spinner().template("{spinner:.green} {msg}")?);
            pb.set_message(format!("Ingesting {}", path.display()));
            pb.enable_steady_tick(std::time::Duration::from_millis(100));

            let result = ingestor.ingest_file(path)?;

            pb.finish_with_message(format!(
                "{} {} ({} chunks)",
                if result.was_update { "Updated:" } else { "Ingested:" }.green().bold(),
                result.item.title,
                result.chunks.len()
            ));

            println!("  ID: {}", result.item.id);
            println!("  Type: {}", result.item.item_type);
        }
    } else {
        // Directory
        println!("{} {}", "Scanning:".cyan(), path.display());

        // Collect files first
        let files: Vec<_> = walkdir::WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .filter(|e| {
                // Skip hidden files
                !e.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with('.'))
                    .unwrap_or(false)
            })
            .filter(|e| {
                // Filter by extension
                let ext = e.path().extension().and_then(|e| e.to_str());
                let item_type = ext.and_then(ItemType::from_extension);

                match (item_type, type_filter) {
                    (Some(it), Some(filter)) => it == filter,
                    (Some(_), None) => true,
                    (None, _) => false,
                }
            })
            .collect();

        if files.is_empty() {
            println!("{}", "No supported files found.".yellow());
            return Ok(());
        }

        println!("Found {} files", files.len());

        if dry_run {
            for entry in &files {
                let item_type = entry.path()
                    .extension()
                    .and_then(|e| e.to_str())
                    .and_then(ItemType::from_extension)
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                println!("  {} [{}]", entry.path().display(), item_type);
            }
            println!("\n{}", "Dry run - no files were ingested.".cyan());
            return Ok(());
        }

        // Create progress bar
        let pb = ProgressBar::new(files.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")?
                .progress_chars("#>-"),
        );

        let mut success = 0;
        let mut failed = 0;
        let mut skipped = 0;

        for entry in &files {
            let filename = entry.path()
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file");
            pb.set_message(format!("{}", filename));

            if queue {
                match ingestor.queue_file(entry.path(), 0) {
                    Ok(_) => success += 1,
                    Err(olal_ingest::IngestError::AlreadyProcessed(_)) => skipped += 1,
                    Err(_) => failed += 1,
                }
            } else {
                match ingestor.ingest_file(entry.path()) {
                    Ok(_) => success += 1,
                    Err(_) => failed += 1,
                }
            }

            pb.inc(1);
        }

        pb.finish_and_clear();

        let action = if queue { "Queued" } else { "Ingested" };
        println!(
            "\n{} {} files",
            format!("{}:", action).green().bold(),
            success
        );
        if skipped > 0 {
            println!("{} {} files (already processed)", "Skipped:".yellow().bold(), skipped);
        }
        if failed > 0 {
            println!("{} {} files", "Failed:".red().bold(), failed);
        }
    }

    Ok(())
}

/// Process all pending items in the queue.
#[allow(dead_code)]
pub fn process_queue() -> Result<()> {
    let config = Config::load().unwrap_or_default();
    let paths = olal_config::AppPaths::new().ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
    let db = Database::open(&paths.database_file)?;

    let chunk_config = ChunkConfig::from_processing_config(&config.processing);
    let ingestor = Ingestor::new(db, chunk_config);

    println!("{}", "Processing queue...".cyan());

    let results = ingestor.process_all()?;

    if results.is_empty() {
        println!("{}", "Queue is empty.".yellow());
    } else {
        println!("{} {} items", "Processed:".green().bold(), results.len());
    }

    Ok(())
}
