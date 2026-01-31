//! Configuration error types.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse config file: {0}")]
    ParseError(#[from] toml::de::Error),

    #[error("Failed to serialize config: {0}")]
    SerializeError(#[from] toml::ser::Error),

    #[error("Config directory not found")]
    NoConfigDir,

    #[error("Invalid configuration: {0}")]
    Invalid(String),
}

pub type ConfigResult<T> = Result<T, ConfigError>;
