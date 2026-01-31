//! Olal Config - Configuration management for Olal.

mod config;
mod error;
mod paths;

pub use config::*;
pub use error::{ConfigError, ConfigResult};
pub use paths::AppPaths;
