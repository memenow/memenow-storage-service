//! IPFS (InterPlanetary File System) Integration Module
//!
//! This module provides functionality for uploading files to IPFS, a peer-to-peer
//! distributed file system. Files uploaded to IPFS are content-addressed and can
//! be retrieved from any IPFS node.
//!
//! # IPFS Overview
//!
//! IPFS uses content-addressing to uniquely identify files. Each file receives a
//! Content Identifier (CID), which is a cryptographic hash of the file's content.
//! This ensures data integrity and enables deduplication.
//!
//! # Connection
//!
//! By default, this module connects to a local IPFS daemon at `http://127.0.0.1:5001`.
//! Ensure you have IPFS installed and running:
//!
//! ```bash
//! ipfs daemon
//! ```
//!
//! # Examples
//!
//! ```no_run
//! use memenow_storage_service::infrastructure::ipfs::upload_to_ipfs;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let cid = upload_to_ipfs("/tmp/myfile.jpg").await?;
//! println!("File CID: {}", cid);
//! println!("Access at: https://ipfs.io/ipfs/{}", cid);
//! # Ok(())
//! # }
//! ```

use anyhow::{Context, Result};
use ipfs_api::{IpfsApi, IpfsClient, TryFromUri};
use log::{debug, info};
use std::path::Path;
use tokio::task;

/// Upload a file to IPFS
///
/// This function uploads a file to the InterPlanetary File System (IPFS) and returns
/// the Content Identifier (CID) that can be used to retrieve the file from any IPFS node.
///
/// The function uses `spawn_blocking` to handle the blocking IPFS client operations
/// without blocking the async runtime. This ensures good performance even when
/// uploading large files.
///
/// # Arguments
///
/// * `filepath` - Path to the local file to upload to IPFS
///
/// # Returns
///
/// Returns the IPFS Content Identifier (CID/hash) as a string on success.
/// This hash can be used to retrieve the file from any IPFS gateway:
/// - Public gateway: `https://ipfs.io/ipfs/{hash}`
/// - Local gateway: `http://127.0.0.1:8080/ipfs/{hash}`
///
/// # Errors
///
/// This function will return an error if:
/// - The IPFS daemon is not running or not accessible
/// - The local file cannot be read
/// - The file path is invalid
/// - Network errors occur during upload
/// - The IPFS client returns an empty response
///
/// # Examples
///
/// ```no_run
/// use memenow_storage_service::infrastructure::ipfs::upload_to_ipfs;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Upload a file to IPFS
/// let cid = upload_to_ipfs("/tmp/document.pdf").await?;
///
/// println!("File uploaded to IPFS");
/// println!("CID: {}", cid);
/// println!("View at: https://ipfs.io/ipfs/{}", cid);
/// # Ok(())
/// # }
/// ```
///
/// # Performance Considerations
///
/// - Large files are chunked automatically by IPFS
/// - The upload operation runs in a blocking thread pool to avoid blocking the async runtime
/// - Files are deduplicated automatically - uploading the same file twice returns the same CID
/// - Consider pinning important files to ensure they remain available:
///   ```bash
///   ipfs pin add <CID>
///   ```
///
/// # IPFS Daemon Configuration
///
/// The default IPFS daemon listens on `http://127.0.0.1:5001` for API requests.
/// To use a different IPFS node, you can modify the client connection or set
/// environment variables according to the `ipfs-api` crate documentation.
pub async fn upload_to_ipfs(filepath: &str) -> Result<String> {
    debug!("Initiating IPFS upload: file={}", filepath);

    // Clone filepath for the blocking task
    let filepath_owned = filepath.to_string();

    // Spawn a blocking task to handle the IPFS upload
    // This prevents blocking the async runtime since ipfs-api uses blocking operations
    let hash = task::spawn_blocking(move || {
        // Create IPFS client connected to local daemon
        // Default endpoint: http://127.0.0.1:5001
        let client = IpfsClient::default();

        debug!("IPFS client created, adding file to IPFS...");

        // Upload the file to IPFS
        // This operation may take some time for large files as they are chunked and hashed
        let add_response = futures::executor::block_on(client.add_path(Path::new(&filepath_owned)))
            .context("Failed to add file to IPFS")?;

        debug!("IPFS add operation completed, processing response...");

        // Extract the hash (CID) from the response
        // The response is a vector because add_path can add multiple files (e.g., directories)
        let hash = add_response
            .first()
            .ok_or_else(|| anyhow::anyhow!("Empty response from IPFS - file may not have been added"))?
            .hash
            .clone();

        debug!("IPFS hash extracted: {}", hash);

        Ok(hash) as Result<String>
    })
    .await
    .context("IPFS upload task panicked or was cancelled")??;

    info!("File uploaded successfully to IPFS: {}", hash);
    info!("Access via gateway: https://ipfs.io/ipfs/{}", hash);

    Ok(hash)
}

/// Create a custom IPFS client with a specific endpoint URL
///
/// This function creates an IPFS client configured to connect to a custom
/// IPFS daemon endpoint instead of the default localhost.
///
/// # Arguments
///
/// * `url` - The full URL of the IPFS daemon API endpoint (e.g., "http://127.0.0.1:5001")
///
/// # Returns
///
/// Returns a configured `IpfsClient` instance
///
/// # Panics
///
/// Panics if the URL is invalid
///
/// # Examples
///
/// ```no_run
/// use memenow_storage_service::infrastructure::ipfs::create_ipfs_client;
///
/// let client = create_ipfs_client("http://192.168.1.100:5001");
/// ```
pub fn create_ipfs_client(url: &str) -> IpfsClient {
    IpfsClient::from_str(url)
        .expect("Failed to create IPFS client with custom endpoint")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_ipfs_client_localhost() {
        let client = create_ipfs_client("http://127.0.0.1:5001");
        // Client creation should succeed
        // We can't test actual operations without a running IPFS daemon
        drop(client);
    }

    #[test]
    fn test_create_ipfs_client_custom() {
        let client = create_ipfs_client("http://192.168.1.100:5001");
        drop(client);
    }
}
