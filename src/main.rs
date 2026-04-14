//! MemeNow Storage Service

use log::{error, info};
use memenow_storage_service::api::upload::upload_routes;
use memenow_storage_service::config::Config;
use memenow_storage_service::infrastructure::ipfs::IpfsUploader;
use memenow_storage_service::infrastructure::s3::S3Uploader;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
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

    let s3 = Arc::new(S3Uploader::new());
    let ipfs = Arc::new(IpfsUploader);

    let routes = upload_routes(s3, ipfs, config.clone());

    let host =
        IpAddr::from_str(&config.server.host).unwrap_or(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));
    let addr = SocketAddr::new(host, config.server.port);

    info!("Server starting on http://{}", addr);
    info!("Upload endpoint: http://{}/upload", addr);
    info!("Ready to accept requests");

    warp::serve(routes).run(addr).await;

    Ok(())
}
