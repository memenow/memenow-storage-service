//! Amazon S3 Integration Module
//!
//! This module provides functionality for uploading files to Amazon S3.
//! It handles AWS SDK initialization, authentication, and file upload operations.

use crate::domain::ports::ObjectStorage;
use anyhow::{Context, Result};
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use log::{debug, info};
use std::path::Path;

/// Concrete S3 adapter implementing [`ObjectStorage`].
#[derive(Clone)]
pub struct S3Uploader;

impl ObjectStorage for S3Uploader {
    /// Uploads a file to Amazon S3.
    ///
    /// # Arguments
    ///
    /// * `filepath` - Path to the local file to upload
    /// * `bucket` - Name of the S3 bucket
    /// * `key` - S3 object key
    ///
    /// # Returns
    ///
    /// The public HTTPS URL of the uploaded file.
    async fn upload(&self, filepath: &str, bucket: &str, key: &str) -> Result<String> {
        debug!(
            "Initiating S3 upload: file={}, bucket={}, key={}",
            filepath, bucket, key
        );

        let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;

        debug!("AWS configuration loaded, region: {:?}", config.region());

        let client = Client::new(&config);

        let file_path = Path::new(filepath);
        let body = ByteStream::from_path(file_path)
            .await
            .context(format!("Failed to read file: {}", filepath))?;

        debug!("File stream created, uploading to S3...");

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

        let url = format!("https://{}.s3.amazonaws.com/{}", bucket, key);

        info!("File uploaded successfully to S3: {}", url);

        Ok(url)
    }
}

/// Get the region-specific S3 URL for a bucket
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
