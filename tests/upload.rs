use memenow_storage_service::api::upload::upload_routes;
use memenow_storage_service::config::Config;
use memenow_storage_service::domain::ports::{ContentAddressedStorage, ObjectStorage};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct MockS3 {
    fail: bool,
}

impl ObjectStorage for MockS3 {
    async fn upload(&self, _filepath: &str, bucket: &str, key: &str) -> anyhow::Result<String> {
        if self.fail {
            anyhow::bail!("mock S3 failure");
        }
        Ok(format!("https://{}.s3.amazonaws.com/{}", bucket, key))
    }
}

#[derive(Clone)]
struct MockIpfs {
    fail: bool,
    hash: String,
}

impl ContentAddressedStorage for MockIpfs {
    async fn upload(&self, _filepath: &str) -> anyhow::Result<String> {
        if self.fail {
            anyhow::bail!("mock IPFS failure");
        }
        Ok(self.hash.clone())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn mock_s3_ok() -> Arc<MockS3> {
    Arc::new(MockS3 { fail: false })
}

fn mock_ipfs_ok() -> Arc<MockIpfs> {
    Arc::new(MockIpfs {
        fail: false,
        hash: "QmTestHash123".into(),
    })
}

/// Build a valid `multipart/form-data` body with one field.
fn multipart_body(field_name: &str, filename: &str, content: &[u8]) -> (String, Vec<u8>) {
    let boundary = "----TestBoundary7MA4YWxkTrZu0gW";
    let mut body = Vec::new();

    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
            field_name, filename
        )
        .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n");
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(content);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    let content_type = format!("multipart/form-data; boundary={}", boundary);
    (content_type, body)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn upload_returns_s3_url_and_ipfs_hash() {
    let config = Config::default();
    let filter = upload_routes(mock_s3_ok(), mock_ipfs_ok(), config);
    let (ct, body) = multipart_body("file", "test.txt", b"hello world");

    let resp = warp::test::request()
        .method("POST")
        .path("/upload")
        .header("content-type", &ct)
        .body(body)
        .reply(&filter)
        .await;

    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
    assert!(json["s3_url"].as_str().unwrap().contains("default-bucket"));
    assert_eq!(json["ipfs_hash"], "QmTestHash123");
    assert_eq!(json["filename"], "test.txt");
    assert!(json["size"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn get_upload_is_rejected() {
    let filter = upload_routes(mock_s3_ok(), mock_ipfs_ok(), Config::default());

    let resp = warp::test::request()
        .method("GET")
        .path("/upload")
        .reply(&filter)
        .await;

    assert_eq!(resp.status(), 405);
}

#[tokio::test]
async fn unknown_path_returns_404() {
    let filter = upload_routes(mock_s3_ok(), mock_ipfs_ok(), Config::default());

    let resp = warp::test::request()
        .method("POST")
        .path("/nonexistent")
        .reply(&filter)
        .await;

    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn s3_failure_returns_error() {
    let s3 = Arc::new(MockS3 { fail: true });
    let filter = upload_routes(s3, mock_ipfs_ok(), Config::default());
    let (ct, body) = multipart_body("file", "test.txt", b"data");

    let resp = warp::test::request()
        .method("POST")
        .path("/upload")
        .header("content-type", &ct)
        .body(body)
        .reply(&filter)
        .await;

    assert_ne!(resp.status(), 200);
}

#[tokio::test]
async fn ipfs_failure_returns_error() {
    let ipfs = Arc::new(MockIpfs {
        fail: true,
        hash: String::new(),
    });
    let filter = upload_routes(mock_s3_ok(), ipfs, Config::default());
    let (ct, body) = multipart_body("file", "test.txt", b"data");

    let resp = warp::test::request()
        .method("POST")
        .path("/upload")
        .header("content-type", &ct)
        .body(body)
        .reply(&filter)
        .await;

    assert_ne!(resp.status(), 200);
}

#[tokio::test]
async fn both_backends_fail_returns_error() {
    let s3 = Arc::new(MockS3 { fail: true });
    let ipfs = Arc::new(MockIpfs {
        fail: true,
        hash: String::new(),
    });
    let filter = upload_routes(s3, ipfs, Config::default());
    let (ct, body) = multipart_body("file", "test.txt", b"data");

    let resp = warp::test::request()
        .method("POST")
        .path("/upload")
        .header("content-type", &ct)
        .body(body)
        .reply(&filter)
        .await;

    assert_ne!(resp.status(), 200);
}

#[tokio::test]
async fn missing_file_field_returns_error() {
    let filter = upload_routes(mock_s3_ok(), mock_ipfs_ok(), Config::default());
    let (ct, body) = multipart_body("not_a_file", "test.txt", b"data");

    let resp = warp::test::request()
        .method("POST")
        .path("/upload")
        .header("content-type", &ct)
        .body(body)
        .reply(&filter)
        .await;

    assert_ne!(resp.status(), 200);
}

#[tokio::test]
async fn oversize_body_is_rejected() {
    let mut config = Config::default();
    config.upload.max_file_size = 100; // 100 bytes limit
    let filter = upload_routes(mock_s3_ok(), mock_ipfs_ok(), config);
    let big_content = vec![0u8; 200];
    let (ct, body) = multipart_body("file", "big.bin", &big_content);

    let resp = warp::test::request()
        .method("POST")
        .path("/upload")
        .header("content-type", &ct)
        .body(body)
        .reply(&filter)
        .await;

    assert_ne!(resp.status(), 200);
}
