//! File system watcher for automatic ingestion.

use crate::error::{IngestError, IngestResult};
use olal_core::ItemType;
use glob::Pattern;
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, Debouncer};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Events emitted by the file watcher.
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// A new file was created or modified.
    FileChanged {
        path: PathBuf,
        item_type: ItemType,
    },
    /// A file was deleted.
    FileDeleted { path: PathBuf },
    /// An error occurred.
    Error(String),
}

/// Configuration for the file watcher.
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Directories to watch.
    pub directories: Vec<PathBuf>,
    /// Patterns to ignore.
    pub ignore_patterns: Vec<Pattern>,
    /// Debounce duration.
    pub debounce: Duration,
}

impl WatcherConfig {
    /// Create from config.
    pub fn from_config(config: &olal_config::WatchConfig) -> Self {
        let directories = config
            .directories
            .iter()
            .map(|s| {
                let expanded = shellexpand::tilde(s);
                PathBuf::from(expanded.as_ref())
            })
            .collect();

        let ignore_patterns = config
            .ignore_patterns
            .iter()
            .filter_map(|p| Pattern::new(p).ok())
            .collect();

        Self {
            directories,
            ignore_patterns,
            debounce: Duration::from_secs(config.poll_interval_seconds.max(1)),
        }
    }
}

/// File system watcher for detecting new files.
pub struct FileWatcher {
    config: WatcherConfig,
    debouncer: Debouncer<RecommendedWatcher>,
    receiver: Receiver<Result<Vec<DebouncedEvent>, notify::Error>>,
}

impl FileWatcher {
    /// Create a new file watcher.
    pub fn new(config: WatcherConfig) -> IngestResult<Self> {
        let (tx, rx) = channel();

        let debouncer = new_debouncer(config.debounce, tx)
            .map_err(|e| IngestError::WatchError(e.to_string()))?;

        Ok(Self {
            config,
            debouncer,
            receiver: rx,
        })
    }

    /// Start watching configured directories.
    pub fn start(&mut self) -> IngestResult<()> {
        for dir in &self.config.directories {
            if !dir.exists() {
                warn!("Watch directory does not exist: {:?}", dir);
                continue;
            }

            info!("Watching directory: {:?}", dir);
            self.debouncer
                .watcher()
                .watch(dir, RecursiveMode::Recursive)
                .map_err(|e| IngestError::WatchError(e.to_string()))?;
        }

        Ok(())
    }

    /// Poll for events (non-blocking).
    pub fn poll(&self) -> Vec<WatchEvent> {
        let mut events = Vec::new();

        while let Ok(result) = self.receiver.try_recv() {
            match result {
                Ok(debounced_events) => {
                    for event in debounced_events {
                        if let Some(watch_event) = self.process_event(event) {
                            events.push(watch_event);
                        }
                    }
                }
                Err(e) => {
                    error!("Watch error: {:?}", e);
                    events.push(WatchEvent::Error(e.to_string()));
                }
            }
        }

        events
    }

    /// Wait for the next event (blocking).
    pub fn next_event(&self) -> Option<WatchEvent> {
        match self.receiver.recv() {
            Ok(result) => match result {
                Ok(debounced_events) => {
                    for event in debounced_events {
                        if let Some(watch_event) = self.process_event(event) {
                            return Some(watch_event);
                        }
                    }
                    None
                }
                Err(e) => Some(WatchEvent::Error(e.to_string())),
            },
            Err(_) => None,
        }
    }

    /// Process a debounced event.
    fn process_event(&self, event: DebouncedEvent) -> Option<WatchEvent> {
        let path = &event.path;

        // Skip directories
        if path.is_dir() {
            return None;
        }

        // Check ignore patterns
        if self.should_ignore(path) {
            debug!("Ignoring file: {:?}", path);
            return None;
        }

        // Detect file type
        let item_type = path
            .extension()
            .and_then(|e| e.to_str())
            .and_then(ItemType::from_extension);

        match item_type {
            Some(it) => {
                // Check if file exists (could be a delete)
                if path.exists() {
                    info!("File changed: {:?} (type: {})", path, it);
                    Some(WatchEvent::FileChanged {
                        path: path.clone(),
                        item_type: it,
                    })
                } else {
                    info!("File deleted: {:?}", path);
                    Some(WatchEvent::FileDeleted { path: path.clone() })
                }
            }
            None => {
                debug!("Ignoring unsupported file type: {:?}", path);
                None
            }
        }
    }

    /// Check if a path should be ignored.
    fn should_ignore(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Check filename-based patterns
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            for pattern in &self.config.ignore_patterns {
                if pattern.matches(filename) {
                    return true;
                }
            }
        }

        // Check path-based patterns
        for pattern in &self.config.ignore_patterns {
            if pattern.matches(&path_str) {
                return true;
            }
        }

        // Ignore hidden files
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            if filename.starts_with('.') && filename != ".." {
                return true;
            }
        }

        false
    }
}

/// Scan a directory for existing files.
#[allow(dead_code)]
pub fn scan_directory(
    dir: &Path,
    ignore_patterns: &[Pattern],
) -> IngestResult<Vec<(PathBuf, ItemType)>> {
    let mut files = Vec::new();

    if !dir.exists() {
        return Err(IngestError::FileNotFound(dir.to_path_buf()));
    }

    for entry in walkdir::WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Check ignore patterns
        if should_ignore_path(path, ignore_patterns) {
            continue;
        }

        // Detect file type
        if let Some(item_type) = path
            .extension()
            .and_then(|e| e.to_str())
            .and_then(ItemType::from_extension)
        {
            files.push((path.to_path_buf(), item_type));
        }
    }

    Ok(files)
}

#[allow(dead_code)]
fn should_ignore_path(path: &Path, patterns: &[Pattern]) -> bool {
    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
        // Ignore hidden files
        if filename.starts_with('.') {
            return true;
        }

        for pattern in patterns {
            if pattern.matches(filename) {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_ignore() {
        let patterns = vec![
            Pattern::new("*.tmp").unwrap(),
            Pattern::new(".DS_Store").unwrap(),
        ];

        assert!(should_ignore_path(Path::new("/foo/bar/.hidden"), &patterns));
        assert!(should_ignore_path(Path::new("/foo/bar/file.tmp"), &patterns));
        assert!(should_ignore_path(Path::new("/foo/.DS_Store"), &patterns));
        assert!(!should_ignore_path(Path::new("/foo/bar/file.txt"), &patterns));
        assert!(!should_ignore_path(Path::new("/foo/bar/video.mp4"), &patterns));
    }
}
