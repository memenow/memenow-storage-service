//! Error types for the MemeNow Storage Service
//!
//! This module defines custom error types using the `thiserror` crate for
//! better error handling and more informative error messages throughout the application.

use thiserror::Error;

/// Main error type for the storage service
///
/// This enum represents all possible errors that can occur in the application,
/// providing detailed context for each error scenario.
#[derive(Error, Debug)]
pub enum StorageError {
    /// Error occurred while interacting with Amazon S3
    #[error("S3 operation failed: {0}")]
    S3Error(String),

    /// Error occurred while interacting with IPFS
    #[error("IPFS operation failed: {0}")]
    IpfsError(String),

    /// Error occurred during file I/O operations
    #[error("File I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Error occurred while parsing multipart form data
    #[error("Failed to parse multipart form data: {0}")]
    MultipartError(String),

    /// Error occurred due to missing or invalid configuration
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Error occurred during file upload processing
    #[error("Upload processing failed: {0}")]
    UploadError(String),

    /// Error occurred due to missing file in the upload request
    #[error("No file found in upload request")]
    NoFileError,

    /// Generic error for unexpected failures
    #[error("Internal server error: {0}")]
    InternalError(String),

    /// Error occurred during AWS SDK operations
    #[error("AWS SDK error: {0}")]
    AwsError(String),
}

/// Custom implementation to convert `StorageError` into a warp rejection
impl warp::reject::Reject for StorageError {}

/// Result type alias using `StorageError`
///
/// This provides a convenient way to return results throughout the application.
pub type StorageResult<T> = std::result::Result<T, StorageError>;

/// Convert AWS S3 errors to `StorageError`
impl<E> From<aws_sdk_s3::error::SdkError<E>> for StorageError
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(err: aws_sdk_s3::error::SdkError<E>) -> Self {
        Self::AwsError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = StorageError::NoFileError;
        assert_eq!(error.to_string(), "No file found in upload request");

        let error = StorageError::S3Error("bucket not found".to_string());
        assert_eq!(error.to_string(), "S3 operation failed: bucket not found");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let storage_error: StorageError = io_error.into();
        assert!(matches!(storage_error, StorageError::IoError(_)));
    }
}
