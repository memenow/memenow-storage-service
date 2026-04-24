//! Upload API endpoint
//!
//! This module defines the HTTP API routes for file upload operations.

use crate::config::Config;
use crate::error::{ErrorResponse, StorageError};
use crate::domain::ports::{ContentAddressedStorage, ObjectStorage};
use crate::domain::services::handle_upload;
use std::convert::Infallible;
use std::sync::Arc;
use warp::http::StatusCode;
use warp::{Filter, Reply};
use uuid::Uuid;

/// Injects a cloneable value into the Warp filter chain.
fn with_val<T: Clone + Send>(val: T) -> impl Filter<Extract = (T,), Error = Infallible> + Clone {
    warp::any().map(move || val.clone())
}

/// Injects a request identifier into the filter chain.
fn with_request_id() -> impl Filter<Extract = (String,), Error = Infallible> + Clone {
    warp::any().map(|| Uuid::new_v4().simple().to_string().chars().take(8).collect())
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
) -> impl Filter<Extract = (impl warp::Reply,), Error = Infallible> + Clone
where
    S: ObjectStorage,
    C: ContentAddressedStorage,
{
    warp::path("upload")
        .and(warp::post())
        .and(with_request_id())
        .and(warp::multipart::form().max_length(config.upload.max_file_size as u64))
        .and(with_val(s3))
        .and(with_val(ipfs))
        .and(with_val(config))
        .and_then(handle_upload)
        .recover(handle_rejection)
}

fn build_error_response(
    status_code: StatusCode,
    payload: serde_json::Value,
) -> impl warp::Reply {
    let payload = serde_json::json!({
        "error": {
            "code": payload["error"]["code"],
            "message": payload["error"]["message"],
            "details": payload["error"].get("details"),
            "request_id": payload["error"].get("request_id"),
        }
    });
    warp::reply::with_status(warp::reply::json(&payload), status_code)
}

async fn handle_rejection(
    err: warp::reject::Rejection,
) -> Result<warp::reply::Response, Infallible> {
    let (status, payload) = if let Some(storage_error) = err.find::<StorageError>() {
        (
            storage_error.status_code(),
            serde_json::to_value(storage_error.to_response(None)).unwrap(),
        )
    } else if err.find::<warp::reject::PayloadTooLarge>().is_some() {
        (
            StatusCode::PAYLOAD_TOO_LARGE,
            serde_json::to_value(ErrorResponse {
                error: crate::error::ErrorPayload {
                    code: "PAYLOAD_TOO_LARGE".to_string(),
                    message: "Multipart payload is too large".to_string(),
                    request_id: None,
                    details: None,
                },
            })
            .unwrap(),
        )
    } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
        (
            StatusCode::METHOD_NOT_ALLOWED,
            serde_json::to_value(ErrorResponse {
                error: crate::error::ErrorPayload {
                    code: "METHOD_NOT_ALLOWED".to_string(),
                    message: "Method not allowed".to_string(),
                    request_id: None,
                    details: None,
                },
            })
            .unwrap(),
        )
    } else if err.find::<warp::reject::UnsupportedMediaType>().is_some() {
        (
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            serde_json::to_value(ErrorResponse {
                error: crate::error::ErrorPayload {
                    code: "INVALID_MEDIA_TYPE".to_string(),
                    message: "Unsupported media type".to_string(),
                    request_id: None,
                    details: None,
                },
            })
            .unwrap(),
        )
    } else if err.is_not_found() {
        (
            StatusCode::NOT_FOUND,
            serde_json::to_value(ErrorResponse {
                error: crate::error::ErrorPayload {
                    code: "NOT_FOUND".to_string(),
                    message: "Not found".to_string(),
                    request_id: None,
                    details: None,
                },
            })
            .unwrap(),
        )
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::to_value(ErrorResponse {
                error: crate::error::ErrorPayload {
                    code: "INTERNAL_ERROR".to_string(),
                    message: "Internal server error".to_string(),
                    request_id: None,
                    details: None,
                },
            })
            .unwrap(),
        )
    };

    Ok(build_error_response(status, payload).into_response())
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

    fn test_filter() -> impl Filter<Extract = (impl warp::Reply,), Error = Infallible> + Clone
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
