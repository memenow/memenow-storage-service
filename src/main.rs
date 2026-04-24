//! MemeNow Storage Service

use log::{error, info};
use memenow_storage_service::api::upload::upload_routes;
use memenow_storage_service::error::StorageError;
use memenow_storage_service::config::Config;
use memenow_storage_service::infrastructure::ipfs::IpfsUploader;
use memenow_storage_service::infrastructure::s3::S3Uploader;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting MemeNow Storage Service...");

    let config = Config::from_env().map_err(|e| {
        error!("Failed to load configuration: {}", e);
        e
    })?;

    config.validate().map_err(|e| {
        error!("Configuration validation failed: {}", e);
        e
    })?;

    info!("Configuration loaded successfully");
    info!("S3 Bucket: {}", config.s3.bucket);
    info!("S3 Region: {}", config.s3.region);
    info!("Max file size: {} bytes", config.upload.max_file_size);
    info!("S3 upload timeout: {} ms", config.s3.upload_timeout_ms);
    info!("IPFS upload timeout: {} ms", config.upload.ipfs_upload_timeout_ms);
    info!("IPFS max concurrency: {}", config.upload.ipfs_max_concurrency);

    let s3 = Arc::new(S3Uploader::new(config.s3.upload_timeout_ms));
    let ipfs = Arc::new(IpfsUploader::new(
        config.upload.ipfs_upload_timeout_ms,
        config.upload.ipfs_max_concurrency,
    ));

    let routes = upload_routes(s3, ipfs, config.clone());

    let host = resolve_server_host(&config.server.host).map_err(|error| {
        error!("Invalid SERVER_HOST '{}': {}", config.server.host, error);
        StorageError::ConfigError(format!(
            "Invalid SERVER_HOST '{}': {}",
            config.server.host, error
        ))
    })?;
    let addr = SocketAddr::new(host, config.server.port);

    info!("Server starting on http://{}", addr);
    info!("Upload endpoint: http://{}/upload", addr);
    info!("Ready to accept requests");

    warp::serve(routes).run(addr).await;

    Ok(())
}

fn resolve_server_host(host: &str) -> Result<IpAddr, String> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(ip);
    }

    let mut addrs = (host, 0)
        .to_socket_addrs()
        .map_err(|_| format!("Cannot resolve host '{}'", host))?;

    addrs
        .next()
        .map(|addr| addr.ip())
        .ok_or_else(|| format!("Cannot resolve host '{}'", host))
}
