//! Service layer for handling file upload business logic
//!
//! This module contains the core business logic for processing file uploads,
//! coordinating between the API layer and infrastructure services.

use crate::config::Config;
use crate::error::StorageError;
use crate::infrastructure::{ipfs, s3};
use bytes::Buf;
use futures_util::stream::TryStreamExt;
use log::{debug, error, info, warn};
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::try_join;
use uuid::Uuid;
use warp::multipart::{FormData, Part};

/// Response structure for successful file uploads
///
/// This structure is returned when a file has been successfully uploaded
/// to both S3 and IPFS.
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
/// This is the main entry point for processing file uploads. It performs the following steps:
/// 1. Extracts the file from the multipart form data
/// 2. Saves the file to a temporary location
/// 3. Concurrently uploads the file to both S3 and IPFS
/// 4. Returns the upload results
///
/// # Arguments
///
/// * `form` - Multipart form data containing the file to upload
/// * `config` - Application configuration containing upload settings
///
/// # Returns
///
/// Returns a JSON response containing the S3 URL and IPFS hash on success,
/// or a warp rejection on failure.
///
/// # Errors
///
/// This function will return an error if:
/// - No file is found in the form data
/// - The file cannot be saved to temporary storage
/// - The upload to S3 or IPFS fails
///
/// # Examples
///
/// ```no_run
/// use warp::multipart::FormData;
/// use memenow_storage_service::domain::services::handle_upload;
/// use memenow_storage_service::config::Config;
///
/// # async fn example(form: FormData) -> Result<(), Box<dyn std::error::Error>> {
/// let config = Config::default();
/// let response = handle_upload(form, config).await?;
/// # Ok(())
/// # }
/// ```
pub async fn handle_upload(
    form: FormData,
    config: Config,
) -> Result<impl warp::Reply, warp::Rejection> {
    debug!("Processing upload request");

    // Extract file from multipart form data
    let (filepath, filename, file_size) = extract_and_save_file(form, &config)
        .await
        .map_err(|e| {
            error!("Failed to extract file from form data: {}", e);
            warp::reject::custom(e)
        })?;

    info!(
        "File saved to temporary location: {} (size: {} bytes)",
        filepath.display(),
        file_size
    );

    // Generate unique key for S3
    let file_key = generate_file_key(&filename, &config.s3.key_prefix);

    // Upload to S3 and IPFS concurrently
    let s3_future = s3::upload_to_s3(
        filepath.to_str().unwrap(),
        &config.s3.bucket,
        &file_key,
    );
    let ipfs_future = ipfs::upload_to_ipfs(filepath.to_str().unwrap());

    let (s3_url, ipfs_hash) = try_join!(s3_future, ipfs_future).map_err(|e| {
        error!("Failed to upload file: {}", e);
        warp::reject::custom(StorageError::UploadError(e.to_string()))
    })?;

    // Clean up temporary file
    if let Err(e) = tokio::fs::remove_file(&filepath).await {
        warn!(
            "Failed to remove temporary file {}: {}",
            filepath.display(),
            e
        );
    } else {
        debug!("Temporary file removed: {}", filepath.display());
    }

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
/// # Arguments
///
/// * `form` - Multipart form data
/// * `config` - Application configuration
///
/// # Returns
///
/// Returns a tuple containing (filepath, filename, file_size)
///
/// # Errors
///
/// Returns an error if:
/// - No file is found in the form data
/// - File creation or writing fails
async fn extract_and_save_file(
    form: FormData,
    config: &Config,
) -> Result<(PathBuf, String, u64), StorageError> {
    let parts: Vec<Part> = form
        .try_collect()
        .await
        .map_err(|e| StorageError::MultipartError(e.to_string()))?;

    let mut file_data: Option<(PathBuf, String, u64)> = None;

    for mut part in parts {
        if part.name() == "file" {
            let filename = part
                .filename()
                .ok_or(StorageError::NoFileError)?
                .to_string();

            debug!("Processing file: {}", filename);

            // Generate unique temporary filepath
            let temp_filename = format!("{}_{}", Uuid::new_v4(), filename);
            let filepath = PathBuf::from(&config.upload.temp_dir).join(temp_filename);

            let mut file = File::create(&filepath)
                .await
                .map_err(StorageError::IoError)?;

            let mut total_size = 0u64;

            // Read and write file chunks
            while let Some(chunk) = part.data().await {
                let data = chunk.map_err(|e| {
                    StorageError::MultipartError(format!("Failed to read chunk: {}", e))
                })?;

                let bytes = data.chunk();
                total_size += bytes.len() as u64;

                // Check file size limit
                if total_size > config.upload.max_file_size as u64 {
                    // Clean up the partially written file
                    let _ = tokio::fs::remove_file(&filepath).await;
                    return Err(StorageError::UploadError(format!(
                        "File size exceeds maximum allowed size of {} bytes",
                        config.upload.max_file_size
                    )));
                }

                file.write_all(bytes)
                    .await
                    .map_err(StorageError::IoError)?;
            }

            // Ensure all data is written to disk
            file.flush().await.map_err(StorageError::IoError)?;

            debug!("File written successfully: {} bytes", total_size);

            file_data = Some((filepath, filename, total_size));
            break;
        }
    }

    file_data.ok_or(StorageError::NoFileError)
}

/// Generate a unique S3 key for the uploaded file
///
/// # Arguments
///
/// * `filename` - Original filename
/// * `prefix` - S3 key prefix from configuration
///
/// # Returns
///
/// Returns a unique S3 key combining the prefix, UUID, and filename
fn generate_file_key(filename: &str, prefix: &str) -> String {
    let uuid = Uuid::new_v4();
    let sanitized_filename = sanitize_filename(filename);
    format!("{}/{}_{}", prefix, uuid, sanitized_filename)
}

/// Sanitize filename by removing or replacing unsafe characters
///
/// # Arguments
///
/// * `filename` - Original filename to sanitize
///
/// # Returns
///
/// Returns a sanitized filename safe for use in URLs and file systems
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
            ".._.._..._etc_passwd"
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
