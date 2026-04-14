//! Port traits for the storage service
//!
//! These traits define the interfaces that infrastructure adapters must implement,
//! completing the hexagonal architecture pattern. Using traits enables integration
//! testing with mock backends.

use anyhow::Result;

/// Port for object storage backends (e.g. Amazon S3).
pub trait ObjectStorage: Send + Sync + 'static {
    /// Upload a file to object storage.
    ///
    /// # Arguments
    ///
    /// * `filepath` - Path to the local file to upload
    /// * `bucket` - Target bucket name
    /// * `key` - Object key within the bucket
    ///
    /// # Returns
    ///
    /// The public URL of the uploaded object.
    fn upload(
        &self,
        filepath: &str,
        bucket: &str,
        key: &str,
    ) -> impl std::future::Future<Output = Result<String>> + Send;
}

/// Port for content-addressed storage backends (e.g. IPFS).
pub trait ContentAddressedStorage: Send + Sync + 'static {
    /// Upload a file to content-addressed storage.
    ///
    /// # Arguments
    ///
    /// * `filepath` - Path to the local file to upload
    ///
    /// # Returns
    ///
    /// The content hash (CID) of the uploaded file.
    fn upload(&self, filepath: &str) -> impl std::future::Future<Output = Result<String>> + Send;
}
