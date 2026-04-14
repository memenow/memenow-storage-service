//! Service layer for handling file upload business logic
//!
//! This module contains the core business logic for processing file uploads,
//! coordinating between the API layer and infrastructure services.

use crate::config::Config;
use crate::domain::ports::{ContentAddressedStorage, ObjectStorage};
use crate::error::StorageError;
use bytes::Buf;
use futures_util::stream::TryStreamExt;
use log::{debug, error, info, warn};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::try_join;
use uuid::Uuid;
use warp::multipart::FormData;

/// Response structure for successful file uploads
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UploadResponse {
    /// The URL where the file can be accessed on S3
    pub s3_url: String,
    /// The IPFS hash (CID) of the uploaded file
    pub ipfs_hash: String,
    /// The original filename
    pub filename: String,
    /// The size of the uploaded file in bytes
    pub size: u64,
}

/// Handle file upload request
///
/// Extracts the file from multipart form data, saves it to a temporary location,
/// then concurrently uploads to both object storage and content-addressed storage.
///
/// # Arguments
///
/// * `form` - Multipart form data containing the file to upload
/// * `s3` - Object storage backend
/// * `ipfs` - Content-addressed storage backend
/// * `config` - Application configuration
///
/// # Returns
///
/// JSON response with S3 URL, IPFS hash, filename, and size on success.
pub async fn handle_upload<S, C>(
    form: FormData,
    s3: Arc<S>,
    ipfs: Arc<C>,
    config: Config,
) -> Result<impl warp::Reply, warp::Rejection>
where
    S: ObjectStorage,
    C: ContentAddressedStorage,
{
    debug!("Processing upload request");

    let (filepath, filename, file_size) =
        extract_and_save_file(form, &config).await.map_err(|e| {
            error!("Failed to extract file from form data: {}", e);
            warp::reject::custom(e)
        })?;

    info!(
        "File saved to temporary location: {} (size: {} bytes)",
        filepath.display(),
        file_size
    );

    let file_key = generate_file_key(&filename, &config.s3.key_prefix);

    let filepath_str = filepath.to_str().unwrap().to_string();
    let s3_future = s3.upload(&filepath_str, &config.s3.bucket, &file_key);
    let ipfs_future = ipfs.upload(&filepath_str);

    let upload_result = try_join!(s3_future, ipfs_future);

    // Clean up temp file regardless of upload outcome
    if let Err(e) = tokio::fs::remove_file(&filepath).await {
        warn!(
            "Failed to remove temporary file {}: {}",
            filepath.display(),
            e
        );
    } else {
        debug!("Temporary file removed: {}", filepath.display());
    }

    let (s3_url, ipfs_hash) = upload_result.map_err(|e| {
        error!("Failed to upload file: {}", e);
        warp::reject::custom(StorageError::UploadError(e.to_string()))
    })?;

    info!(
        "File '{}' uploaded successfully - S3: {}, IPFS: {}",
        filename, s3_url, ipfs_hash
    );

    let response = UploadResponse {
        s3_url,
        ipfs_hash,
        filename,
        size: file_size,
    };

    Ok(warp::reply::json(&response))
}

/// Extract file from form data and save to temporary location
///
/// Processes parts one at a time (via `try_next`) to avoid multer lock issues
/// that occur when holding multiple `Part` references simultaneously.
async fn extract_and_save_file(
    form: FormData,
    config: &Config,
) -> Result<(PathBuf, String, u64), StorageError> {
    let mut form = form;

    while let Some(mut part) = form
        .try_next()
        .await
        .map_err(|e| StorageError::MultipartError(e.to_string()))?
    {
        if part.name() == "file" {
            let filename = part
                .filename()
                .ok_or(StorageError::NoFileError)?
                .to_string();

            debug!("Processing file: {}", filename);

            let temp_filename = format!("{}_{}", Uuid::new_v4(), filename);
            let filepath = PathBuf::from(&config.upload.temp_dir).join(temp_filename);

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
                    return Err(StorageError::UploadError(format!(
                        "File size exceeds maximum allowed size of {} bytes",
                        config.upload.max_file_size
                    )));
                }

                file.write_all(bytes).await.map_err(StorageError::IoError)?;
            }

            file.flush().await.map_err(StorageError::IoError)?;

            debug!("File written successfully: {} bytes", total_size);

            return Ok((filepath, filename, total_size));
        }
    }

    Err(StorageError::NoFileError)
}

/// Generate a unique S3 key for the uploaded file
fn generate_file_key(filename: &str, prefix: &str) -> String {
    let uuid = Uuid::new_v4();
    let sanitized_filename = sanitize_filename(filename);
    format!("{}/{}_{}", prefix, uuid, sanitized_filename)
}

/// Sanitize filename by removing or replacing unsafe characters
fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_' => c,
            _ => '_',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("test.jpg"), "test.jpg");
        assert_eq!(sanitize_filename("test file.jpg"), "test_file.jpg");
        assert_eq!(sanitize_filename("test@#$.jpg"), "test___.jpg");
        assert_eq!(
            sanitize_filename("../../../etc/passwd"),
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
