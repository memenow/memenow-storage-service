//! IPFS (InterPlanetary File System) Integration Module
//!
//! This module provides functionality for uploading files to IPFS.

use crate::domain::ports::ContentAddressedStorage;
use anyhow::{Context, Result};
use ipfs_api::{IpfsApi, IpfsClient};
use log::{debug, info};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task;
use tokio::time::{timeout, Duration};

/// Concrete IPFS adapter implementing [`ContentAddressedStorage`].
#[derive(Clone)]
pub struct IpfsUploader {
    timeout: Duration,
    concurrency: Arc<Semaphore>,
}

impl ContentAddressedStorage for IpfsUploader {
    /// Uploads a file to IPFS.
    ///
    /// Uses `spawn_blocking` to handle the blocking IPFS client operations.
    ///
    /// # Arguments
    ///
    /// * `filepath` - Path to the local file to upload
    ///
    /// # Returns
    ///
    /// The IPFS Content Identifier (CID/hash).
    async fn upload(&self, filepath: &str) -> Result<String> {
        debug!("Initiating IPFS upload: file={}", filepath);

        let permit = self
            .concurrency
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| anyhow::anyhow!("IPFS upload concurrency limit is unavailable"))?;

        let filepath_owned = filepath.to_string();
        let timeout_duration = self.timeout;

        let handle = task::spawn_blocking(move || {
            let _permit = permit;
            let client = IpfsClient::default();

            debug!("IPFS client created, adding file to IPFS...");

            let add_response = futures::executor::block_on(client.add_path(Path::new(&filepath_owned)))
                .context("Failed to add file to IPFS")?;

            let hash = add_response
                .first()
                .ok_or_else(|| {
                    anyhow::anyhow!("Empty response from IPFS - file may not have been added")
                })?
                .hash
                .clone();

            debug!("IPFS hash extracted: {}", hash);

            Ok(hash) as Result<String>
        });

        let hash = timeout(timeout_duration, handle)
            .await
            .map_err(|_| {
                anyhow::anyhow!(
                    "IPFS upload timed out after {} ms",
                    timeout_duration.as_millis()
                )
            })?
            .context("IPFS upload task panicked or was cancelled")??;

        info!("File uploaded successfully to IPFS: {}", hash);
        info!("Access via gateway: https://ipfs.io/ipfs/{}", hash);

        Ok(hash)
    }
}

impl IpfsUploader {
    /// Creates a new `IpfsUploader` with configurable timeout and concurrency.
    pub fn new(upload_timeout_ms: u64, max_concurrency: usize) -> Self {
        Self {
            timeout: Duration::from_millis(upload_timeout_ms),
            concurrency: Arc::new(Semaphore::new(max_concurrency)),
        }
    }
}

impl Default for IpfsUploader {
    fn default() -> Self {
        Self::new(60_000, 4)
    }
}
