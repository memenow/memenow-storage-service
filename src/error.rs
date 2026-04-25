//! Error types for the MemeNow Storage Service.
//!
//! The service uses a single error enum with explicit categories so every failure can
//! be mapped to a stable HTTP status and API error code.

use serde::Serialize;
use serde_json::{json, Value};
use thiserror::Error;
use warp::http::StatusCode;
use warp::reject::Reject;

/// Structured error payload returned to callers.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorPayload,
}

/// Structured error details for API responses.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

/// Metadata for backend partial successes.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UploadSuccessSnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipfs_hash: Option<String>,
}

/// Main error type for the storage service.
#[derive(Error, Debug)]
pub enum StorageError {
    /// Input validation failed before upload started.
    #[error("Validation failed: {message}")]
    ValidationError { message: String },

    /// Multipart payload exceeded configured maximum size.
    #[error("Payload too large. Max allowed bytes: {max_bytes}")]
    PayloadTooLarge {
        max_bytes: usize,
        actual_bytes: Option<u64>,
    },

    /// Backend upload failed without any side success.
    #[error("Upload failed on backend '{backend}': {message}")]
    StorageBackendError { backend: String, message: String },

    /// One backend failed while the other backend succeeded.
    #[error("Partial upload failure on '{failed_backend}': {message}")]
    PartialUploadFailure {
        failed_backend: String,
        message: String,
        partial_success: UploadSuccessSnapshot,
    },

    /// Cleanup failed after upload stage completed or partially failed.
    #[error("Cleanup failed for temporary file '{path}': {message}")]
    CleanupError { path: String, message: String },

    /// Missing or invalid file field.
    #[error("No file found in upload request")]
    NoFileError,

    /// Multipart form parsing error.
    #[error("Failed to parse multipart form data: {0}")]
    MultipartError(String),

    /// Missing or invalid configuration.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// AWS SDK related failure.
    #[error("AWS SDK error: {0}")]
    AwsError(String),

    /// S3 specific error.
    #[error("S3 operation failed: {0}")]
    S3Error(String),

    /// Generic internal error.
    #[error("Internal server error: {0}")]
    InternalError(String),

    /// File system errors.
    #[error("File I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Custom implementation to convert `StorageError` into a Warp rejection.
impl Reject for StorageError {}

/// Result type alias using `StorageError`.
pub type StorageResult<T> = std::result::Result<T, StorageError>;

impl StorageError {
    /// Maps a service error into the corresponding HTTP status code.
    pub fn status_code(&self) -> StatusCode {
        match self {
            StorageError::ValidationError { .. }
            | StorageError::MultipartError(_)
            | StorageError::NoFileError => StatusCode::BAD_REQUEST,
            StorageError::PayloadTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            StorageError::PartialUploadFailure { .. } => StatusCode::BAD_GATEWAY,
            StorageError::StorageBackendError { .. } => StatusCode::BAD_GATEWAY,
            StorageError::CleanupError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            StorageError::ConfigError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            StorageError::IoError(_)
            | StorageError::AwsError(_)
            | StorageError::S3Error(_)
            | StorageError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Stable service error code to expose to callers.
    pub fn error_code(&self) -> &'static str {
        match self {
            StorageError::ValidationError { .. } | StorageError::MultipartError(_) | StorageError::NoFileError => {
                "INVALID_REQUEST"
            }
            StorageError::PayloadTooLarge { .. } => "PAYLOAD_TOO_LARGE",
            StorageError::PartialUploadFailure { .. } => "UPLOAD_PARTIAL_FAILURE",
            StorageError::StorageBackendError { .. } => "STORAGE_BACKEND_ERROR",
            StorageError::CleanupError { .. } => "CLEANUP_ERROR",
            StorageError::ConfigError(_) => "CONFIG_ERROR",
            StorageError::IoError(_) => "IO_ERROR",
            StorageError::AwsError(_) => "AWS_ERROR",
            StorageError::S3Error(_) => "S3_ERROR",
            StorageError::InternalError(_) => "INTERNAL_ERROR",
        }
    }

    /// Builds the structured JSON body for API responses.
    pub fn to_response(&self, request_id: Option<String>) -> ErrorResponse {
        ErrorResponse {
            error: ErrorPayload {
                code: self.error_code().to_string(),
                message: self.to_string(),
                request_id,
                details: self.error_details(),
            },
        }
    }

    fn error_details(&self) -> Option<Value> {
        match self {
            StorageError::PayloadTooLarge {
                max_bytes,
                actual_bytes,
            } => Some(json!({"max_bytes": max_bytes, "actual_bytes": actual_bytes})),
            StorageError::StorageBackendError { backend, .. } => {
                Some(json!({"backend": backend}))
            }
            StorageError::PartialUploadFailure {
                failed_backend,
                partial_success,
                ..
            } => Some(json!({
                "failed_backend": failed_backend,
                "partial_success": partial_success
            })),
            StorageError::CleanupError { path, .. } => Some(json!({ "path": path })),
            _ => None,
        }
    }
}

/// Convert AWS S3 errors to `StorageError`.
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
    fn test_payload_too_large_status_and_code() {
        let error = StorageError::PayloadTooLarge {
            max_bytes: 10,
            actual_bytes: Some(20),
        };
        assert_eq!(error.status_code(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(error.error_code(), "PAYLOAD_TOO_LARGE");
    }

    #[test]
    fn test_partial_failure_status_and_code() {
        let error = StorageError::PartialUploadFailure {
            failed_backend: "ipfs".to_string(),
            message: "ipfs unavailable".to_string(),
            partial_success: UploadSuccessSnapshot {
                s3_url: Some("https://bucket.s3.amazonaws.com/key".to_string()),
                ipfs_hash: None,
            },
        };
        assert_eq!(error.status_code(), StatusCode::BAD_GATEWAY);
        assert_eq!(error.error_code(), "UPLOAD_PARTIAL_FAILURE");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let storage_error: StorageError = io_error.into();
        assert!(matches!(storage_error, StorageError::IoError(_)));
    }
}
