//! Upload API endpoint
//!
//! This module defines the HTTP API routes for file upload operations.

use crate::config::Config;
use crate::domain::ports::{ContentAddressedStorage, ObjectStorage};
use crate::domain::services::handle_upload;
use std::convert::Infallible;
use std::sync::Arc;
use warp::Filter;

/// Injects a cloneable value into the Warp filter chain.
fn with_val<T: Clone + Send>(val: T) -> impl Filter<Extract = (T,), Error = Infallible> + Clone {
    warp::any().map(move || val.clone())
}

/// Create upload routes with injected storage backends and configuration.
///
/// # Arguments
///
/// * `s3` - Object storage backend
/// * `ipfs` - Content-addressed storage backend
/// * `config` - Application configuration
///
/// # Route Details
///
/// - **Path**: `/upload`
/// - **Method**: POST
/// - **Content-Type**: multipart/form-data
/// - **Response**: JSON with S3 URL, IPFS hash, filename, and file size
pub fn upload_routes<S, C>(
    s3: Arc<S>,
    ipfs: Arc<C>,
    config: Config,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone
where
    S: ObjectStorage,
    C: ContentAddressedStorage,
{
    warp::path("upload")
        .and(warp::post())
        .and(warp::multipart::form().max_length(config.upload.max_file_size as u64))
        .and(with_val(s3))
        .and(with_val(ipfs))
        .and(with_val(config))
        .and_then(handle_upload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp::http::StatusCode;
    use warp::test::request;

    #[derive(Clone)]
    struct StubS3;
    impl ObjectStorage for StubS3 {
        async fn upload(&self, _: &str, b: &str, k: &str) -> anyhow::Result<String> {
            Ok(format!("https://{}.s3.amazonaws.com/{}", b, k))
        }
    }

    #[derive(Clone)]
    struct StubIpfs;
    impl ContentAddressedStorage for StubIpfs {
        async fn upload(&self, _: &str) -> anyhow::Result<String> {
            Ok("QmStub".into())
        }
    }

    fn test_filter() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone
    {
        upload_routes(Arc::new(StubS3), Arc::new(StubIpfs), Config::default())
    }

    #[tokio::test]
    async fn test_upload_route_requires_post() {
        let routes = test_filter();
        let response = request().method("GET").path("/upload").reply(&routes).await;

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn test_upload_route_path() {
        let routes = test_filter();
        let response = request().method("POST").path("/wrong").reply(&routes).await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
