// Import the necessary modules and types.
use crate::infrastructure::{ipfs, s3};
use anyhow::Result;
use bytes::Buf;
use futures_util::stream::TryStreamExt;
use log::{error, info};
use std::env;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::try_join;
use warp::multipart::FormData;
use warp::multipart::Part;

/// A custom error type for handling upload errors.
#[derive(Debug)]
struct CustomError {
    message: String,
}

// Implement the `Reject` trait for `CustomError` so it can be used with warp's rejection handling.
impl warp::reject::Reject for CustomError {}

/// The `handle_upload` function handles the upload of a file.
///
/// It takes a `FormData` object, which represents the multipart form data of the upload request.
/// The function processes the form data, saves the uploaded file to a temporary location,
/// and then uploads the file to both S3 and IPFS.
///
/// # Arguments
///
/// * `form` - A `FormData` object representing the multipart form data of the upload request.
///
/// # Returns
///
/// This function returns a `Result` that contains a warp `Reply` if the upload was successful,
/// or a warp `Rejection` if there was an error during the upload process.
///
/// The `Reply` is a JSON object that contains the URLs of the uploaded file on S3 and IPFS.
pub async fn handle_upload(form: FormData) -> Result<impl warp::Reply, warp::Rejection> {
    let parts: Vec<Part> = form.try_collect().await.map_err(|e| {
        error!("Failed to collect form data: {:?}", e);
        warp::reject::custom(CustomError {
            message: format!("Failed to collect form data: {:?}", e),
        })
    })?;

    let mut filepath = String::new();

    for mut part in parts {
        if part.name() == "file" {
            let filename = part.filename().unwrap_or("upload.tmp").to_string();
            filepath = format!("/tmp/{}", filename);

            let mut file = File::create(&filepath).await.map_err(|e| {
                error!("Failed to create file: {:?}", e);
                warp::reject::custom(CustomError {
                    message: format!("Failed to create file: {:?}", e),
                })
            })?;
            while let Some(chunk) = part.data().await {
                let data = chunk.map_err(|e| {
                    error!("Failed to read chunk: {:?}", e);
                    warp::reject::custom(CustomError {
                        message: format!("Failed to read chunk: {:?}", e),
                    })
                })?;
                let bytes = data.chunk();
                file.write_all(bytes).await.map_err(|e| {
                    error!("Failed to write to file: {:?}", e);
                    warp::reject::custom(CustomError {
                        message: format!("Failed to write to file: {:?}", e),
                    })
                })?;
            }
        }
    }

    let s3_key = env::var("S3_KEY").unwrap_or_else(|_| "uploaded_file".to_string());
    let bucket = env::var("S3_BUCKET").expect("S3_BUCKET must be set");

    let s3_future = s3::upload_to_s3(&filepath, &bucket, &s3_key);
    let ipfs_future = ipfs::upload_to_ipfs(&filepath);

    let (s3_url, ipfs_hash) = try_join!(s3_future, ipfs_future).map_err(|e| {
        error!("Failed to upload: {:?}", e);
        warp::reject::custom(CustomError {
            message: format!("Failed to upload: {:?}", e),
        })
    })?;

    info!(
        "File uploaded successfully: s3_url = {}, ipfs_hash = {}",
        s3_url, ipfs_hash
    );

    Ok(warp::reply::json(&serde_json::json!({
        "s3_url": s3_url,
        "ipfs_hash": ipfs_hash,
    })))
}
