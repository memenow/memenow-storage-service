//! Amazon S3 Integration Module
//!
//! This module provides functionality for uploading files to Amazon S3.
//! It handles AWS SDK initialization, authentication, and file upload operations.
//!
//! # Authentication
//!
//! Authentication is handled automatically through the AWS SDK, which looks for
//! credentials in the following order:
//! 1. Environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
//! 2. AWS credentials file (~/.aws/credentials)
//! 3. IAM instance profile (when running on EC2)
//!
//! # Examples
//!
//! ```no_run
//! use memenow_storage_service::infrastructure::s3::upload_to_s3;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let url = upload_to_s3(
//!     "/tmp/myfile.jpg",
//!     "my-bucket",
//!     "uploads/myfile.jpg"
//! ).await?;
//! println!("File uploaded to: {}", url);
//! # Ok(())
//! # }
//! ```

use anyhow::{Context, Result};
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use log::{debug, info};
use std::path::Path;

/// Upload a file to Amazon S3
///
/// This function uploads a file from the local filesystem to an Amazon S3 bucket.
/// It uses the AWS SDK to handle authentication and transfer, supporting multipart
/// uploads for large files automatically.
///
/// # Arguments
///
/// * `filepath` - Path to the local file to upload
/// * `bucket` - Name of the S3 bucket (must already exist)
/// * `key` - S3 object key (path within the bucket)
///
/// # Returns
///
/// Returns the public HTTPS URL of the uploaded file on success.
/// Note: The file may not be publicly accessible depending on bucket permissions.
///
/// # Errors
///
/// This function will return an error if:
/// - The local file cannot be read
/// - AWS credentials are not configured or invalid
/// - The S3 bucket does not exist or is not accessible
/// - Network errors occur during upload
/// - The AWS SDK returns an error
///
/// # Examples
///
/// ```no_run
/// use memenow_storage_service::infrastructure::s3::upload_to_s3;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Upload a file with automatic region detection
/// let url = upload_to_s3(
///     "/tmp/image.jpg",
///     "my-photos-bucket",
///     "2024/01/image.jpg"
/// ).await?;
///
/// assert!(url.starts_with("https://"));
/// # Ok(())
/// # }
/// ```
///
/// # Performance Considerations
///
/// - Files are streamed from disk, minimizing memory usage
/// - The AWS SDK automatically uses multipart uploads for large files
/// - Consider using AWS Transfer Acceleration for large files or global uploads
pub async fn upload_to_s3(filepath: &str, bucket: &str, key: &str) -> Result<String> {
    debug!(
        "Initiating S3 upload: file={}, bucket={}, key={}",
        filepath, bucket, key
    );

    // Configure AWS region
    // Tries to detect region from environment, config file, or defaults to us-east-1
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");

    // Load AWS configuration from environment
    // This includes credentials, region, and other AWS settings
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    debug!("AWS configuration loaded, region: {:?}", config.region());

    // Create S3 client
    let client = Client::new(&config);

    // Create a byte stream from the file
    // This streams the file in chunks rather than loading it entirely into memory
    let file_path = Path::new(filepath);
    let body = ByteStream::from_path(file_path)
        .await
        .context(format!("Failed to read file: {}", filepath))?;

    debug!("File stream created, uploading to S3...");

    // Upload the file to S3
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(body)
        .send()
        .await
        .context(format!(
            "Failed to upload file to S3: bucket={}, key={}",
            bucket, key
        ))?;

    // Construct the public URL
    // Note: This URL format works for standard buckets in most regions
    // For buckets in some regions or with special configurations, the format may differ
    let url = format!("https://{}.s3.amazonaws.com/{}", bucket, key);

    info!("File uploaded successfully to S3: {}", url);

    Ok(url)
}

/// Get the region-specific S3 URL for a bucket
///
/// Different AWS regions use different URL formats. This function generates
/// the correct URL based on the bucket and region.
///
/// # Arguments
///
/// * `bucket` - S3 bucket name
/// * `key` - Object key within the bucket
/// * `region` - AWS region (e.g., "us-east-1", "eu-west-1")
///
/// # Returns
///
/// Returns the properly formatted S3 URL for the specified region
///
/// # Examples
///
/// ```
/// use memenow_storage_service::infrastructure::s3::get_s3_url;
///
/// let url = get_s3_url("my-bucket", "file.jpg", "us-east-1");
/// assert_eq!(url, "https://my-bucket.s3.amazonaws.com/file.jpg");
///
/// let url = get_s3_url("my-bucket", "file.jpg", "eu-west-1");
/// assert_eq!(url, "https://my-bucket.s3.eu-west-1.amazonaws.com/file.jpg");
/// ```
pub fn get_s3_url(bucket: &str, key: &str, region: &str) -> String {
    if region == "us-east-1" {
        format!("https://{}.s3.amazonaws.com/{}", bucket, key)
    } else {
        format!("https://{}.s3.{}.amazonaws.com/{}", bucket, region, key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_s3_url_us_east_1() {
        let url = get_s3_url("test-bucket", "path/to/file.jpg", "us-east-1");
        assert_eq!(url, "https://test-bucket.s3.amazonaws.com/path/to/file.jpg");
    }

    #[test]
    fn test_get_s3_url_other_region() {
        let url = get_s3_url("test-bucket", "path/to/file.jpg", "eu-west-1");
        assert_eq!(
            url,
            "https://test-bucket.s3.eu-west-1.amazonaws.com/path/to/file.jpg"
        );
    }

    #[test]
    fn test_get_s3_url_special_characters() {
        let url = get_s3_url("test-bucket", "path/to/file with spaces.jpg", "us-west-2");
        assert!(url.contains("file with spaces.jpg"));
    }
}
