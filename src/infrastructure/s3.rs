// Import the necessary modules and types from the `aws_sdk_s3` and `aws_config` crates.
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
// Import the `Path` type from the `std::path` module.
use std::path::Path;
// Import the `Result` type from the `anyhow` crate.
use anyhow::Result;

/// The `upload_to_s3` function uploads a file to Amazon S3.
///
/// It takes a string representing the path of the file to be uploaded, the name of the S3 bucket, and the key for the object in the bucket.
/// The function creates a `Client` for the S3 service, uses it to upload the file to the specified bucket,
/// and then returns the URL of the uploaded file.
///
/// # Arguments
///
/// * `filepath` - A string representing the path of the file to be uploaded.
/// * `bucket` - A string representing the name of the S3 bucket.
/// * `key` - A string representing the key for the object in the bucket.
///
/// # Returns
///
/// This function returns a `Result` that contains the URL of the uploaded file if the upload was successful,
/// or an error if there was an issue during the upload process.
pub async fn upload_to_s3(filepath: &str, bucket: &str, key: &str) -> Result<String> {
    // Create a region provider with a default region, or "us-east-1" if no default region is set.
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    // Load the AWS configuration from the environment.
    let config = aws_config::from_env().region(region_provider).load().await;
    // Create a new `Client` for the S3 service.
    let client = Client::new(&config);

    // Create a `ByteStream` from the file at the specified path.
    let file = ByteStream::from_path(Path::new(filepath)).await?;

    // Use the `Client` to upload the `ByteStream` to the specified bucket and key.
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(file)
        .send()
        .await?;

    // Return the URL of the uploaded file.
    Ok(format!("https://{}.s3.amazonaws.com/{}", bucket, key))
}
