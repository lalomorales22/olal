//! CLI command implementations.

pub mod ask;
pub mod capture;
pub mod clips;
pub mod config;
pub mod digest;
pub mod embed;
pub mod ingest;
pub mod init;
pub mod project;
pub mod recent;
pub mod search;
pub mod shell;
pub mod show;
pub mod stats;
pub mod status;
pub mod tag;
pub mod task;
pub mod watch;
pub mod youtube;

use olal_config::AppPaths;
use olal_db::Database;
use anyhow::{Context, Result};

/// Get the application paths.
pub fn get_paths() -> Result<AppPaths> {
    AppPaths::new().context("Failed to determine application directories")
}

/// Get a database connection, ensuring olal is initialized.
pub fn get_database() -> Result<Database> {
    let paths = get_paths()?;

    if !paths.is_initialized() {
        anyhow::bail!(
            "Olal is not initialized. Run 'olal init' first."
        );
    }

    Database::open(&paths.database_file).context("Failed to open database")
}

/// Format a file size in human-readable form.
pub fn format_size(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;
    const GB: i64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
