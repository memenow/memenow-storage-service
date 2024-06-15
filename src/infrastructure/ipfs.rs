// Import the necessary modules and types from the `ipfs_api` crate.
use ipfs_api::IpfsApi;
use ipfs_api::IpfsClient;
// Import the `Path` type from the `std::path` module.
use std::path::Path;
// Import the `Result` type from the `anyhow` crate.
use anyhow::Result;
// Import the `task` module from the `tokio` crate.
use tokio::task;

/// The `upload_to_ipfs` function uploads a file to the InterPlanetary File System (IPFS).
///
/// It takes a string representing the path of the file to be uploaded.
/// The function creates an `IpfsClient`, uses it to upload the file to IPFS,
/// and then returns the hash of the uploaded file.
///
/// # Arguments
///
/// * `filepath` - A string representing the path of the file to be uploaded.
///
/// # Returns
///
/// This function returns a `Result` that contains the hash of the uploaded file if the upload was successful,
/// or an error if there was an issue during the upload process.
pub async fn upload_to_ipfs(filepath: &str) -> Result<String> {
    // Convert the filepath to a string.
    let filepath = filepath.to_string();
    // Spawn a new task to upload the file to IPFS.
    let hash = task::spawn_blocking(move || {
        // Create a new `IpfsClient`.
        let client = IpfsClient::default();
        // Upload the file to IPFS and get the response.
        let add_response = futures::executor::block_on(client.add_path(Path::new(&filepath)))?;
        // Get the hash of the uploaded file from the response.
        let hash = add_response
            .get(0)
            .ok_or_else(|| anyhow::anyhow!("No response from IPFS"))?
            .hash
            .clone();
        // Return the hash as a `Result`.
        Ok(hash) as Result<String>
    })
    .await??;

    // Return the hash.
    Ok(hash)
}
