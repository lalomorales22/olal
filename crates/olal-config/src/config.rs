//! Configuration structures and loading.

use crate::error::{ConfigError, ConfigResult};
use crate::paths::AppPaths;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,

    #[serde(default)]
    pub ollama: OllamaConfig,

    #[serde(default)]
    pub watch: WatchConfig,

    #[serde(default)]
    pub processing: ProcessingConfig,

    #[serde(default)]
    pub youtube: YoutubeConfig,

    #[serde(default)]
    pub ui: UiConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            ollama: OllamaConfig::default(),
            watch: WatchConfig::default(),
            processing: ProcessingConfig::default(),
            youtube: YoutubeConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from the default location.
    pub fn load() -> ConfigResult<Self> {
        let paths = AppPaths::new().ok_or(ConfigError::NoConfigDir)?;
        Self::load_from(&paths.config_file)
    }

    /// Load configuration from a specific path.
    pub fn load_from(path: &PathBuf) -> ConfigResult<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Save configuration to the default location.
    pub fn save(&self) -> ConfigResult<()> {
        let paths = AppPaths::new().ok_or(ConfigError::NoConfigDir)?;
        self.save_to(&paths.config_file)
    }

    /// Save configuration to a specific path.
    pub fn save_to(&self, path: &PathBuf) -> ConfigResult<()> {
        let contents = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, contents)?;
        Ok(())
    }

    /// Create a default config file with comments.
    pub fn create_default_file(path: &PathBuf) -> ConfigResult<()> {
        let default_config = Self::default_config_string();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, default_config)?;
        Ok(())
    }

    /// Generate a default config file with helpful comments.
    pub fn default_config_string() -> String {
        r#"# Olal Configuration
# Your Personal Second Brain & Life Operating System

[general]
# Data directory for database and cache
# data_dir = "~/.local/share/olal"

[ollama]
# Ollama server address
host = "http://localhost:11434"

# Default model for chat/queries
model = "gpt-oss:20b"

# Model for generating embeddings
embedding_model = "nomic-embed-text"

# Request timeout in seconds
timeout_seconds = 120

[watch]
# Directories to watch for new files
# Add your screen recordings folder, notes folder, etc.
directories = [
    # "~/Movies/ScreenRecordings",
    # "~/Documents/Notes",
]

# File patterns to ignore
ignore_patterns = [
    "*.tmp",
    "*.temp",
    ".DS_Store",
    "._*",
    "*.part",
]

# How often to check for changes (seconds)
poll_interval_seconds = 5

[processing]
# Video processing options
extract_audio = true
transcribe = true
ocr_enabled = true
ocr_interval_seconds = 10      # Extract frame every N seconds for OCR
generate_summary = true        # AI-generated summaries for ingested content
auto_tag = true                # AI-suggested tags for ingested content
detect_chapters = true

# Text chunking for RAG
chunk_size = 512               # Tokens per chunk
chunk_overlap = 50             # Overlap between chunks

# Performance
max_concurrent_jobs = 2

# Whisper model size: tiny, base, small, medium, large
whisper_model = "base"

[youtube]
# Default style for YouTube metadata generation
# Options: tutorial, review, vlog, educational
default_style = "tutorial"

# Include timestamps in descriptions
include_timestamps = true

# Generate chapter markers
include_chapters = true

[ui]
# Enable colored output
color = true

# Pager for long output
pager = "less"

# Date format (strftime)
date_format = "%Y-%m-%d %H:%M"
"#
        .to_string()
    }

    /// Add a directory to the watch list.
    pub fn add_watch_directory(&mut self, path: String) {
        if !self.watch.directories.contains(&path) {
            self.watch.directories.push(path);
        }
    }
}

/// General application settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub data_dir: Option<String>,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self { data_dir: None }
    }
}

/// Ollama LLM settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OllamaConfig {
    pub host: String,
    pub model: String,
    pub embedding_model: String,
    pub timeout_seconds: u64,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            host: "http://localhost:11434".to_string(),
            model: "gpt-oss:20b".to_string(),
            embedding_model: "nomic-embed-text".to_string(),
            timeout_seconds: 120,
        }
    }
}

/// File watching settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WatchConfig {
    pub directories: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub poll_interval_seconds: u64,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            directories: vec![],
            ignore_patterns: vec![
                "*.tmp".to_string(),
                "*.temp".to_string(),
                ".DS_Store".to_string(),
                "._*".to_string(),
                "*.part".to_string(),
            ],
            poll_interval_seconds: 5,
        }
    }
}

/// Content processing settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProcessingConfig {
    pub extract_audio: bool,
    pub transcribe: bool,
    pub ocr_enabled: bool,
    pub ocr_interval_seconds: u64,
    pub generate_summary: bool,
    pub auto_tag: bool,
    pub detect_chapters: bool,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub max_concurrent_jobs: usize,
    pub whisper_model: String,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            extract_audio: true,
            transcribe: true,
            ocr_enabled: true,
            ocr_interval_seconds: 10,
            generate_summary: true,
            auto_tag: true,
            detect_chapters: true,
            chunk_size: 512,
            chunk_overlap: 50,
            max_concurrent_jobs: 2,
            whisper_model: "base".to_string(),
        }
    }
}

/// YouTube content generation settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct YoutubeConfig {
    pub default_style: String,
    pub include_timestamps: bool,
    pub include_chapters: bool,
}

impl Default for YoutubeConfig {
    fn default() -> Self {
        Self {
            default_style: "tutorial".to_string(),
            include_timestamps: true,
            include_chapters: true,
        }
    }
}

/// UI/Display settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub color: bool,
    pub pager: String,
    pub date_format: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            color: true,
            pager: "less".to_string(),
            date_format: "%Y-%m-%d %H:%M".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.ollama.host, "http://localhost:11434");
        assert_eq!(config.ollama.model, "gpt-oss:20b");
        assert!(config.processing.transcribe);
    }

    #[test]
    fn test_config_roundtrip() {
        let config = Config::default();
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();

        assert_eq!(config.ollama.host, deserialized.ollama.host);
        assert_eq!(config.ollama.model, deserialized.ollama.model);
    }

    #[test]
    fn test_load_from_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
            [ollama]
            model = "mistral"
            "#
        )
        .unwrap();

        let path = temp_file.path().to_path_buf();
        let config = Config::load_from(&path).unwrap();

        assert_eq!(config.ollama.model, "mistral");
        // Defaults should still work
        assert_eq!(config.ollama.host, "http://localhost:11434");
    }

    #[test]
    fn test_add_watch_directory() {
        let mut config = Config::default();
        config.add_watch_directory("/path/to/watch".to_string());
        config.add_watch_directory("/path/to/watch".to_string()); // Duplicate

        assert_eq!(config.watch.directories.len(), 1);
        assert_eq!(config.watch.directories[0], "/path/to/watch");
    }
}
