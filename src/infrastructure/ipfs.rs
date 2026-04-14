//! IPFS (InterPlanetary File System) Integration Module
//!
//! This module provides functionality for uploading files to IPFS.

use crate::domain::ports::ContentAddressedStorage;
use anyhow::{Context, Result};
use ipfs_api::{IpfsApi, IpfsClient};
use log::{debug, info};
use std::path::Path;
use tokio::task;

/// Concrete IPFS adapter implementing [`ContentAddressedStorage`].
#[derive(Clone)]
pub struct IpfsUploader;

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

        let filepath_owned = filepath.to_string();

        let hash = task::spawn_blocking(move || {
            let client = IpfsClient::default();

            debug!("IPFS client created, adding file to IPFS...");

            let add_response =
                futures::executor::block_on(client.add_path(Path::new(&filepath_owned)))
                    .context("Failed to add file to IPFS")?;

            debug!("IPFS add operation completed, processing response...");

            let hash = add_response
                .first()
                .ok_or_else(|| {
                    anyhow::anyhow!("Empty response from IPFS - file may not have been added")
                })?
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
}
