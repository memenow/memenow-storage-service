//! Configuration module for the MemeNow Storage Service
//!
//! This module handles loading and validating configuration from environment variables.
//! It provides a centralized configuration structure for the entire application.

use crate::error::{StorageError, StorageResult};
use serde::{Deserialize, Serialize};
use std::env;
use std::net::{IpAddr, ToSocketAddrs};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_MAX_FILE_SIZE_BYTES: usize = 5_242_880;
const MAX_FILE_SIZE_UPPER_LIMIT_BYTES: usize = 1_073_741_824; // 1 GiB
const DEFAULT_S3_UPLOAD_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_IPFS_UPLOAD_TIMEOUT_MS: u64 = 60_000;
const DEFAULT_IPFS_MAX_CONCURRENCY: usize = 4;

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
    /// Timeout for each S3 upload request in milliseconds
    pub upload_timeout_ms: u64,
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
    /// Timeout for each IPFS upload in milliseconds
    pub ipfs_upload_timeout_ms: u64,
    /// Maximum concurrent IPFS uploads executed in parallel
    pub ipfs_max_concurrency: usize,
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
            upload_timeout_ms: env::var("S3_UPLOAD_TIMEOUT_MS")
                .unwrap_or_else(|_| DEFAULT_S3_UPLOAD_TIMEOUT_MS.to_string())
                .parse::<u64>()
                .map_err(|e| {
                    StorageError::ConfigError(format!("Invalid S3_UPLOAD_TIMEOUT_MS: {}", e))
                })?,
        };

        let server = ServerConfig {
            host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .map_err(|e| StorageError::ConfigError(format!("Invalid SERVER_PORT: {}", e)))?,
        };

        let upload = UploadConfig {
            max_file_size: env::var("MAX_FILE_SIZE")
                .unwrap_or_else(|_| DEFAULT_MAX_FILE_SIZE_BYTES.to_string()) // 5MB default
                .parse::<usize>()
                .map_err(|e| StorageError::ConfigError(format!("Invalid MAX_FILE_SIZE: {}", e)))?,
            temp_dir: env::var("TEMP_DIR").unwrap_or_else(|_| "/tmp".to_string()),
            ipfs_upload_timeout_ms: env::var("IPFS_UPLOAD_TIMEOUT_MS")
                .unwrap_or_else(|_| DEFAULT_IPFS_UPLOAD_TIMEOUT_MS.to_string())
                .parse::<u64>()
                .map_err(|e| {
                    StorageError::ConfigError(format!("Invalid IPFS_UPLOAD_TIMEOUT_MS: {}", e))
                })?,
            ipfs_max_concurrency: env::var("IPFS_MAX_CONCURRENCY")
                .unwrap_or_else(|_| DEFAULT_IPFS_MAX_CONCURRENCY.to_string())
                .parse::<usize>()
                .map_err(|e| {
                    StorageError::ConfigError(format!("Invalid IPFS_MAX_CONCURRENCY: {}", e))
                })?,
        };

        Ok(Self { s3, server, upload })
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

        if !is_valid_host(&self.server.host) {
            return Err(StorageError::ConfigError(
                "SERVER_HOST is empty or not a valid host".to_string(),
            ));
        }

        if self.upload.max_file_size == 0 {
            return Err(StorageError::ConfigError(
                "Max file size must be greater than 0".to_string(),
            ));
        }

        if self.upload.max_file_size > MAX_FILE_SIZE_UPPER_LIMIT_BYTES {
            return Err(StorageError::ConfigError(format!(
                "Max file size cannot exceed {} bytes",
                MAX_FILE_SIZE_UPPER_LIMIT_BYTES
            )));
        }

        if self.s3.upload_timeout_ms == 0 {
            return Err(StorageError::ConfigError(
                "S3_UPLOAD_TIMEOUT_MS must be greater than 0".to_string(),
            ));
        }

        if self.upload.ipfs_upload_timeout_ms == 0 {
            return Err(StorageError::ConfigError(
                "IPFS_UPLOAD_TIMEOUT_MS must be greater than 0".to_string(),
            ));
        }

        if self.upload.ipfs_max_concurrency == 0 {
            return Err(StorageError::ConfigError(
                "IPFS_MAX_CONCURRENCY must be greater than 0".to_string(),
            ));
        }

        validate_temp_directory(&self.upload.temp_dir)?;

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
                upload_timeout_ms: DEFAULT_S3_UPLOAD_TIMEOUT_MS,
            },
            server: ServerConfig {
                host: String::from("0.0.0.0"),
                port: 8080,
            },
            upload: UploadConfig {
                max_file_size: DEFAULT_MAX_FILE_SIZE_BYTES,
                temp_dir: String::from("/tmp"),
                ipfs_upload_timeout_ms: DEFAULT_IPFS_UPLOAD_TIMEOUT_MS,
                ipfs_max_concurrency: DEFAULT_IPFS_MAX_CONCURRENCY,
            },
        }
    }
}

fn validate_temp_directory(temp_dir: &str) -> StorageResult<()> {
    if temp_dir.trim().is_empty() {
        return Err(StorageError::ConfigError(
            "TEMP_DIR cannot be empty".to_string(),
        ));
    }

    let path = Path::new(temp_dir);

    if !path.exists() {
        std::fs::create_dir_all(path).map_err(|e| {
            StorageError::ConfigError(format!("Failed to create TEMP_DIR '{}': {}", temp_dir, e))
        })?;
    }

    if !path.is_dir() {
        return Err(StorageError::ConfigError(format!(
            "TEMP_DIR '{}' exists but is not a directory",
            temp_dir
        )));
    }

    let probe_file = path.join(format!(
        ".memenow-storage-service-write-probe-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| {
                StorageError::ConfigError(format!("Failed to generate temp probe path: {}", e))
            })?
            .as_nanos()
    ));

    std::fs::write(&probe_file, b"")
        .map_err(|e| StorageError::ConfigError(format!("TEMP_DIR '{}' not writable: {}", temp_dir, e)))?;

    std::fs::remove_file(&probe_file).map_err(|e| {
        StorageError::ConfigError(format!("Cannot cleanup TEMP_DIR probe file '{}': {}", temp_dir, e))
    })?;

    Ok(())
}

fn is_valid_host(host: &str) -> bool {
    if host.trim().is_empty() {
        return false;
    }

    if host.parse::<IpAddr>().is_ok() {
        return true;
    }

    if host.contains("://") || host.contains('/') || host.contains(' ') {
        return false;
    }

    match (host, 0).to_socket_addrs() {
        Ok(mut addrs) => addrs.next().is_some(),
        Err(_) => {
            if is_numeric_ipv4_host(host) {
                return false;
            }

            is_valid_dns_host(host)
        }
    }
}

fn is_numeric_ipv4_host(host: &str) -> bool {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() != 4 {
        return false;
    }

    parts.iter().all(|part| {
        !part.is_empty() && part.chars().all(|c| c.is_ascii_digit())
    })
}

fn is_valid_dns_host(host: &str) -> bool {
    host.split('.').all(|label| {
        if label.is_empty() || label.len() > 63 {
            return false;
        }

        if label.starts_with('-') || label.ends_with('-') {
            return false;
        }

        label
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;

    fn unique_temp_path(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        path.push(format!("memenow-config-test-{}-{}", prefix, stamp));
        path
    }

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
    fn test_validate_bad_host() {
        let mut config = Config::default();
        config.server.host = "bad host".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_bad_max_file_size() {
        let mut config = Config::default();
        config.upload.max_file_size = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_concurrency() {
        let mut config = Config::default();
        config.upload.ipfs_max_concurrency = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_temp_dir_created_if_missing() {
        let missing = unique_temp_path("missing-dir");
        assert!(!missing.exists());

        let mut config = Config::default();
        config.upload.temp_dir = missing.to_string_lossy().to_string();

        assert!(config.validate().is_ok());
        assert!(missing.exists());
        assert!(missing.is_dir());

        fs::remove_dir_all(&missing).unwrap();
    }

    #[test]
    fn test_validate_temp_dir_is_file() {
        let file_path = unique_temp_path("file");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"probe").unwrap();

        let mut config = Config::default();
        config.upload.temp_dir = file_path.to_string_lossy().to_string();

        assert!(config.validate().is_err());

        fs::remove_file(&file_path).unwrap();
    }

    #[test]
    fn test_validate_bad_host_dns_style() {
        let mut config = Config::default();
        config.server.host = "bad_host!".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_bad_numeric_host() {
        let mut config = Config::default();
        config.server.host = "256.1.1.1".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_valid_config() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn test_validate_unwritable_temp_dir() {
        use std::os::unix::fs::PermissionsExt;

        let dir = unique_temp_path("readonly");
        fs::create_dir_all(&dir).unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o400)).unwrap();

        let mut config = Config::default();
        config.upload.temp_dir = dir.to_string_lossy().to_string();

        assert!(config.validate().is_err());

        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
        fs::remove_dir_all(&dir).unwrap();
    }
}
