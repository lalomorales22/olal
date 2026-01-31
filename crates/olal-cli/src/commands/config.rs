//! Configuration commands.

use super::get_paths;
use anyhow::{Context, Result};
use olal_config::Config;
use colored::Colorize;
use std::process::Command;

pub fn show() -> Result<()> {
    let paths = get_paths()?;

    if !paths.config_file.exists() {
        anyhow::bail!("Config file not found. Run 'olal init' first.");
    }

    let contents = std::fs::read_to_string(&paths.config_file)
        .context("Failed to read config file")?;

    println!("{}", "Current Configuration".cyan().bold());
    println!("{}", "─".repeat(50));
    println!("{}", contents);

    Ok(())
}

pub fn edit() -> Result<()> {
    let paths = get_paths()?;

    if !paths.config_file.exists() {
        anyhow::bail!("Config file not found. Run 'olal init' first.");
    }

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| {
        if cfg!(target_os = "macos") {
            "open -t".to_string()
        } else {
            "nano".to_string()
        }
    });

    let parts: Vec<&str> = editor.split_whitespace().collect();
    let (cmd, args) = parts.split_first().context("Invalid editor command")?;

    let status = Command::new(cmd)
        .args(args)
        .arg(&paths.config_file)
        .status()
        .context("Failed to open editor")?;

    if !status.success() {
        anyhow::bail!("Editor exited with error");
    }

    println!(
        "{} Configuration saved.",
        "✓".green()
    );

    Ok(())
}

pub fn add_watch(path: &str) -> Result<()> {
    let paths = get_paths()?;

    // Expand ~ to home directory
    let expanded_path = if path.starts_with('~') {
        let home = std::env::var("HOME").context("HOME not set")?;
        path.replacen('~', &home, 1)
    } else {
        path.to_string()
    };

    // Check if directory exists
    if !std::path::Path::new(&expanded_path).is_dir() {
        anyhow::bail!("Directory does not exist: {}", expanded_path);
    }

    let mut config = Config::load_from(&paths.config_file)
        .context("Failed to load config")?;

    // Check if already added
    if config.watch.directories.contains(&path.to_string()) {
        println!(
            "{} Directory already in watch list: {}",
            "Note:".yellow(),
            path
        );
        return Ok(());
    }

    config.add_watch_directory(path.to_string());
    config.save_to(&paths.config_file)
        .context("Failed to save config")?;

    println!(
        "{} Added watch directory: {}",
        "✓".green(),
        path
    );

    Ok(())
}

pub fn set(key: &str, value: &str) -> Result<()> {
    let paths = get_paths()?;

    let mut config = Config::load_from(&paths.config_file)
        .context("Failed to load config")?;

    // Parse key path (e.g., "ollama.model")
    let parts: Vec<&str> = key.split('.').collect();

    match parts.as_slice() {
        ["ollama", "model"] => config.ollama.model = value.to_string(),
        ["ollama", "host"] => config.ollama.host = value.to_string(),
        ["ollama", "embedding_model"] => config.ollama.embedding_model = value.to_string(),
        ["ollama", "timeout_seconds"] => {
            config.ollama.timeout_seconds = value.parse()
                .context("Invalid timeout value")?;
        }
        ["processing", "whisper_model"] => config.processing.whisper_model = value.to_string(),
        ["processing", "chunk_size"] => {
            config.processing.chunk_size = value.parse()
                .context("Invalid chunk_size value")?;
        }
        ["processing", "max_concurrent_jobs"] => {
            config.processing.max_concurrent_jobs = value.parse()
                .context("Invalid max_concurrent_jobs value")?;
        }
        ["youtube", "default_style"] => config.youtube.default_style = value.to_string(),
        ["ui", "color"] => {
            config.ui.color = value.parse()
                .context("Invalid boolean value")?;
        }
        ["ui", "pager"] => config.ui.pager = value.to_string(),
        ["ui", "date_format"] => config.ui.date_format = value.to_string(),
        _ => {
            anyhow::bail!("Unknown config key: {}", key);
        }
    }

    config.save_to(&paths.config_file)
        .context("Failed to save config")?;

    println!(
        "{} Set {} = {}",
        "✓".green(),
        key.cyan(),
        value
    );

    Ok(())
}
