//! Application paths management.

use directories::ProjectDirs;
use std::path::PathBuf;

/// Manages all application paths following platform conventions.
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub config_file: PathBuf,
    pub database_file: PathBuf,
    pub log_dir: PathBuf,
}

impl AppPaths {
    /// Create paths using platform-specific directories.
    pub fn new() -> Option<Self> {
        let proj_dirs = ProjectDirs::from("com", "olal", "olal")?;

        let config_dir = proj_dirs.config_dir().to_path_buf();
        let data_dir = proj_dirs.data_dir().to_path_buf();

        Some(Self {
            config_file: config_dir.join("config.toml"),
            log_dir: data_dir.join("logs"),
            database_file: data_dir.join("olal.db"),
            config_dir,
            data_dir,
        })
    }

    /// Create all necessary directories.
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(&self.log_dir)?;
        Ok(())
    }

    /// Check if olal has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.config_file.exists() && self.database_file.exists()
    }
}

impl Default for AppPaths {
    fn default() -> Self {
        Self::new().expect("Could not determine application directories")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_paths_creation() {
        let paths = AppPaths::new();
        assert!(paths.is_some());

        let paths = paths.unwrap();
        assert!(paths.config_file.to_string_lossy().contains("config.toml"));
        assert!(paths.database_file.to_string_lossy().contains("olal.db"));
    }
}
