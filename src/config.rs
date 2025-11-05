//! Configuration module for the MemeNow Storage Service
//!
//! This module handles loading and validating configuration from environment variables.
//! It provides a centralized configuration structure for the entire application.

use crate::error::{StorageError, StorageResult};
use serde::{Deserialize, Serialize};
use std::env;

/// Main configuration structure for the storage service
///
/// This structure holds all configuration values needed to run the service,
/// including AWS S3 settings, server configuration, and upload limits.
///
/// # Examples
///
/// ```no_run
/// use memenow_storage_service::config::Config;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config = Config::from_env()?;
/// println!("Server will run on {}:{}", config.server.host, config.server.port);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// AWS S3 configuration
    pub s3: S3Config,
    /// Server configuration
    pub server: ServerConfig,
    /// Upload configuration
    pub upload: UploadConfig,
}

/// AWS S3 configuration
///
/// Contains all settings required to interact with Amazon S3,
/// including bucket name, key prefix, and region information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    /// The S3 bucket name where files will be uploaded
    pub bucket: String,
    /// The key prefix for uploaded files (e.g., "uploads/")
    pub key_prefix: String,
    /// AWS region (e.g., "us-east-1")
    pub region: String,
}

/// Server configuration
///
/// Defines the server's network settings including host and port.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// The host address to bind to (e.g., "0.0.0.0")
    pub host: String,
    /// The port number to listen on (e.g., 8080)
    pub port: u16,
}

/// Upload configuration
///
/// Controls upload behavior such as file size limits and temporary storage paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadConfig {
    /// Maximum file size in bytes (default: 5MB)
    pub max_file_size: usize,
    /// Directory for temporary file storage
    pub temp_dir: String,
}

impl Config {
    /// Load configuration from environment variables
    ///
    /// This function reads environment variables and constructs a `Config` instance.
    /// It expects a `.env` file to be present or environment variables to be set.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError::ConfigError` if any required environment variable
    /// is missing or invalid.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use memenow_storage_service::config::Config;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Config::from_env()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_env() -> StorageResult<Self> {
        // Load .env file if it exists
        dotenv::dotenv().ok();

        let s3 = S3Config {
            bucket: env::var("S3_BUCKET")
                .map_err(|_| StorageError::ConfigError("S3_BUCKET not set".to_string()))?,
            key_prefix: env::var("S3_KEY").unwrap_or_else(|_| "uploads".to_string()),
            region: env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
        };

        let server = ServerConfig {
            host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .map_err(|e| {
                    StorageError::ConfigError(format!("Invalid SERVER_PORT: {}", e))
                })?,
        };

        let upload = UploadConfig {
            max_file_size: env::var("MAX_FILE_SIZE")
                .unwrap_or_else(|_| "5242880".to_string()) // 5MB default
                .parse()
                .map_err(|e| {
                    StorageError::ConfigError(format!("Invalid MAX_FILE_SIZE: {}", e))
                })?,
            temp_dir: env::var("TEMP_DIR").unwrap_or_else(|_| "/tmp".to_string()),
        };

        Ok(Self {
            s3,
            server,
            upload,
        })
    }

    /// Validate the configuration
    ///
    /// Ensures that all configuration values are valid and within acceptable ranges.
    ///
    /// # Errors
    ///
    /// Returns a `StorageError::ConfigError` if any configuration value is invalid.
    pub fn validate(&self) -> StorageResult<()> {
        if self.s3.bucket.is_empty() {
            return Err(StorageError::ConfigError(
                "S3 bucket name cannot be empty".to_string(),
            ));
        }

        if self.server.port == 0 {
            return Err(StorageError::ConfigError(
                "Server port must be greater than 0".to_string(),
            ));
        }

        if self.upload.max_file_size == 0 {
            return Err(StorageError::ConfigError(
                "Max file size must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            s3: S3Config {
                bucket: String::from("default-bucket"),
                key_prefix: String::from("uploads"),
                region: String::from("us-east-1"),
            },
            server: ServerConfig {
                host: String::from("0.0.0.0"),
                port: 8080,
            },
            upload: UploadConfig {
                max_file_size: 5_242_880, // 5MB
                temp_dir: String::from("/tmp"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.upload.max_file_size, 5_242_880);
    }

    #[test]
    fn test_validate_empty_bucket() {
        let mut config = Config::default();
        config.s3.bucket = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_zero_port() {
        let mut config = Config::default();
        config.server.port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_valid_config() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }
}
