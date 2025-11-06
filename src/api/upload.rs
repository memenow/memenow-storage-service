//! Upload API endpoint
//!
//! This module defines the HTTP API routes for file upload operations.
//! It provides a REST endpoint that accepts multipart form data containing
//! files to be uploaded to S3 and IPFS.

use crate::config::Config;
use crate::domain::services::handle_upload;
use warp::Filter;

/// Create upload routes with the given configuration
///
/// This function constructs a warp filter that handles file upload requests.
/// It configures the multipart form parser with the maximum file size from
/// the configuration and sets up the POST /upload endpoint.
///
/// # Arguments
///
/// * `config` - Application configuration containing upload settings
///
/// # Returns
///
/// Returns a warp filter that can be used to handle upload requests.
///
/// # Route Details
///
/// - **Path**: `/upload`
/// - **Method**: POST
/// - **Content-Type**: multipart/form-data
/// - **Request Body**: Form field named "file" containing the file to upload
/// - **Response**: JSON object with S3 URL, IPFS hash, filename, and file size
///
/// # Examples
///
/// Using curl to upload a file:
///
/// ```bash
/// curl -X POST \
///   -F "file=@/path/to/image.jpg" \
///   http://localhost:8080/upload
/// ```
///
/// Expected response:
///
/// ```json
/// {
///   "s3_url": "https://bucket.s3.amazonaws.com/uploads/uuid_image.jpg",
///   "ipfs_hash": "QmX1y2z3...",
///   "filename": "image.jpg",
///   "size": 102400
/// }
/// ```
///
/// # Errors
///
/// The endpoint will return an error (HTTP 400 or 500) if:
/// - No file is provided in the request
/// - The file exceeds the maximum size limit
/// - The upload to S3 or IPFS fails
/// - The multipart form data is malformed
pub fn upload_routes(
    config: Config,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("upload")
        .and(warp::post())
        .and(warp::multipart::form().max_length(config.upload.max_file_size as u64))
        .and(with_config(config))
        .and_then(handle_upload)
}

/// Helper filter to inject configuration into route handlers
///
/// This creates a warp filter that clones the configuration and makes it
/// available to downstream handlers.
///
/// # Arguments
///
/// * `config` - Configuration to inject
///
/// # Returns
///
/// Returns a filter that extracts the configuration
fn with_config(
    config: Config,
) -> impl Filter<Extract = (Config,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || config.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp::http::StatusCode;
    use warp::test::request;

    #[tokio::test]
    async fn test_upload_route_requires_post() {
        let config = Config::default();
        let routes = upload_routes(config);

        // GET request should not match
        let response = request()
            .method("GET")
            .path("/upload")
            .reply(&routes)
            .await;

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn test_upload_route_path() {
        let config = Config::default();
        let routes = upload_routes(config);

        // Wrong path should not match
        let response = request()
            .method("POST")
            .path("/wrong")
            .reply(&routes)
            .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
