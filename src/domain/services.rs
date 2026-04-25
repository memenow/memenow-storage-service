//! Service layer for handling file upload business logic.
//!
//! This module contains the core business logic for processing file uploads,
//! coordinating between the API layer and infrastructure services.

use crate::config::Config;
use crate::domain::ports::{ContentAddressedStorage, ObjectStorage};
use crate::error::{StorageError, UploadSuccessSnapshot};
use bytes::Buf;
use futures_util::stream::TryStreamExt;
use log::{debug, error, info, warn};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;
use warp::multipart::FormData;

const MAX_SAFE_FILENAME_LENGTH: usize = 255;

/// Response structure for successful file uploads.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UploadResponse {
    /// The URL where the file can be accessed on S3.
    pub s3_url: String,
    /// The IPFS hash (CID) of the uploaded file.
    pub ipfs_hash: String,
    /// The sanitized filename used in the response.
    pub filename: String,
    /// The size of the uploaded file in bytes.
    pub size: u64,
}

/// Handle file upload request.
///
/// Extracts the file from multipart form data, saves it to a temporary location,
/// then concurrently uploads to both object storage and content-addressed storage.
pub async fn handle_upload<S, C>(
    request_id: String,
    form: FormData,
    s3: Arc<S>,
    ipfs: Arc<C>,
    config: Config,
) -> Result<impl warp::Reply, warp::Rejection>
where
    S: ObjectStorage,
    C: ContentAddressedStorage,
{
    debug!("request_id={} stage=receive start", request_id);

    let (filepath, filename, file_size) =
        extract_and_save_file(form, &config, &request_id).await.map_err(|e| {
            error!(
                "request_id={} stage=extract error={}",
                request_id, e
            );
            warp::reject::custom(e)
        })?;

    info!(
        "request_id={} stage=save file_path={} filename={} size={} bytes",
        request_id,
        filepath.display(),
        filename,
        file_size
    );

    let filepath_str = filepath.to_str().ok_or_else(|| {
        StorageError::ValidationError {
            message: format!("Invalid temporary file path: {:?}", filepath),
        }
    })?;

    let file_key = generate_file_key(&filename, &config.s3.key_prefix);
    debug!(
        "request_id={} stage=upload key={} bucket={}",
        request_id, file_key, config.s3.bucket
    );

    let (s3_result, ipfs_result) = tokio::join!(
        s3.upload(filepath_str, &config.s3.bucket, &file_key),
        ipfs.upload(filepath_str),
    );

    let (mut s3_url, mut ipfs_hash, upload_error) = match (s3_result, ipfs_result) {
        (Ok(s3_url), Ok(ipfs_hash)) => {
            (Some(s3_url), Some(ipfs_hash), None)
        }
        (Ok(s3_url), Err(ipfs_error)) => {
            error!(
                "request_id={} stage=upload failed backend=ipfs error={}",
                request_id, ipfs_error
            );
            (
                Some(s3_url.clone()),
                None,
                Some(StorageError::PartialUploadFailure {
                    failed_backend: "ipfs".to_string(),
                    message: format!("IPFS upload failed: {}", ipfs_error),
                    partial_success: UploadSuccessSnapshot {
                        s3_url: Some(s3_url.clone()),
                        ipfs_hash: None,
                    },
                }),
            )
        }
        (Err(s3_error), Ok(ipfs_hash)) => {
            error!(
                "request_id={} stage=upload failed backend=s3 error={}",
                request_id, s3_error
            );
            (
                None,
                Some(ipfs_hash.clone()),
                Some(StorageError::PartialUploadFailure {
                    failed_backend: "s3".to_string(),
                    message: format!("S3 upload failed: {}", s3_error),
                    partial_success: UploadSuccessSnapshot {
                        s3_url: None,
                        ipfs_hash: Some(ipfs_hash.clone()),
                    },
                }),
            )
        }
        (Err(s3_error), Err(ipfs_error)) => {
            error!(
                "request_id={} stage=upload failed both_backends s3_error={} ipfs_error={}",
                request_id, s3_error, ipfs_error
            );
            (
                None,
                None,
                Some(StorageError::StorageBackendError {
                    backend: "s3_and_ipfs".to_string(),
                    message: format!(
                        "S3 upload failed: {}; IPFS upload failed: {}",
                        s3_error, ipfs_error
                    ),
                }),
            )
        }
    };

    let cleanup_error = match tokio::fs::remove_file(&filepath).await {
        Ok(_) => {
            debug!(
                "request_id={} stage=cleanup path={} removed",
                request_id,
                filepath.display()
            );
            None
        }
        Err(error) => {
            warn!(
                "request_id={} stage=cleanup path={} error={}",
                request_id,
                filepath.display(),
                error
            );
            Some(StorageError::CleanupError {
                path: filepath.to_string_lossy().to_string(),
                message: error.to_string(),
            })
        }
    };

    if let Some(error) = upload_error {
        if cleanup_error.is_some() {
            warn!(
                "request_id={} stage=cleanup_failed_after_upload_failure",
                request_id
            );
        }
        return Err(warp::reject::custom(error));
    }

    if let Some(error) = cleanup_error {
        return Err(warp::reject::custom(error));
    }

    let (s3_url, ipfs_hash) = match (s3_url.take(), ipfs_hash.take()) {
        (Some(s3_url), Some(ipfs_hash)) => (s3_url, ipfs_hash),
        _ => {
            return Err(warp::reject::custom(StorageError::InternalError(
                "Upload futures returned inconsistent state".to_string(),
            )))
        }
    };

    info!(
        "request_id={} stage=success s3={} ipfs={}",
        request_id, s3_url, ipfs_hash
    );

    let response = UploadResponse {
        s3_url,
        ipfs_hash,
        filename,
        size: file_size,
    };

    Ok(warp::reply::json(&response))
}

/// Extract file from form data and save to temporary location.
async fn extract_and_save_file(
    form: FormData,
    config: &Config,
    request_id: &str,
) -> Result<(PathBuf, String, u64), StorageError> {
    let mut form = form;

    while let Some(mut part) = form
        .try_next()
        .await
        .map_err(|e| StorageError::MultipartError(e.to_string()))?
    {
        if part.name() != "file" {
            continue;
        }

        let raw_filename = part.filename().ok_or_else(|| {
            StorageError::ValidationError {
                message: "File part is missing filename".to_string(),
            }
        })?;

        let filename = sanitize_filename(raw_filename)?;
        debug!(
            "request_id={} stage=extract raw_filename={} sanitized_filename={}",
            request_id, raw_filename, filename
        );

        let filepath = PathBuf::from(&config.upload.temp_dir).join(generate_temp_filename());
        let mut file = File::create(&filepath)
            .await
            .map_err(StorageError::IoError)?;

        let mut total_size = 0u64;
        while let Some(chunk) = part.data().await {
            let data = chunk.map_err(|e| {
                StorageError::MultipartError(format!("Failed to read chunk: {}", e))
            })?;

            let bytes = data.chunk();
            total_size += bytes.len() as u64;
            if total_size > config.upload.max_file_size as u64 {
                let _ = tokio::fs::remove_file(&filepath).await;
                return Err(StorageError::PayloadTooLarge {
                    max_bytes: config.upload.max_file_size,
                    actual_bytes: Some(total_size),
                });
            }

            file.write_all(bytes).await.map_err(StorageError::IoError)?;
        }

        file.flush().await.map_err(StorageError::IoError)?;
        debug!(
            "request_id={} stage=extract saved size={} bytes",
            request_id, total_size
        );

        return Ok((filepath, filename, total_size));
    }

    Err(StorageError::NoFileError)
}

fn sanitize_filename(input: &str) -> Result<String, StorageError> {
    let normalized = input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();

    let normalized = normalized.trim();

    if normalized.is_empty() {
        return Err(StorageError::ValidationError {
            message: "Uploaded filename is empty after sanitization".to_string(),
        });
    }

    if normalized.len() > MAX_SAFE_FILENAME_LENGTH {
        return Err(StorageError::ValidationError {
            message: format!(
                "Uploaded filename exceeds maximum length of {}",
                MAX_SAFE_FILENAME_LENGTH
            ),
        });
    }

    Ok(normalized.to_string())
}

fn generate_temp_filename() -> String {
    format!("upload-{}", Uuid::new_v4())
}

fn generate_file_key(filename: &str, prefix: &str) -> String {
    let uuid = Uuid::new_v4();
    format!("{}/{}_{}", prefix, uuid, filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("test.jpg").unwrap(), "test.jpg");
        assert_eq!(sanitize_filename("test file.jpg").unwrap(), "test_file.jpg");
        assert_eq!(sanitize_filename("test@#$.jpg").unwrap(), "test___.jpg");
        assert_eq!(
            sanitize_filename("../../../etc/passwd").unwrap(),
            ".._.._.._etc_passwd"
        );
    }

    #[test]
    fn test_generate_file_key() {
        let key = generate_file_key("test.jpg", "uploads");
        assert!(key.starts_with("uploads/"));
        assert!(key.ends_with("_test.jpg"));
    }

    #[test]
    fn test_filename_too_long_is_validation_error() {
        let long_filename = "a".repeat(MAX_SAFE_FILENAME_LENGTH + 1);
        assert!(matches!(
            sanitize_filename(&long_filename),
            Err(StorageError::ValidationError { .. })
        ));
    }

    #[test]
    fn test_upload_response_serialization() {
        let response = UploadResponse {
            s3_url: "https://bucket.s3.amazonaws.com/file".to_string(),
            ipfs_hash: "QmHash123".to_string(),
            filename: "test.jpg".to_string(),
            size: 1024,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("s3_url"));
        assert!(json.contains("ipfs_hash"));
        assert!(json.contains("filename"));
        assert!(json.contains("size"));
    }
}
