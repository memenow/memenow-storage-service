//! MemeNow Storage Service
//!
//! A high-performance, asynchronous microservice for handling file uploads to
//! both Amazon S3 and IPFS. This service provides a REST API endpoint for
//! uploading files with concurrent storage to ensure redundancy and
//! decentralized access.
//!
//! # Features
//!
//! - Concurrent uploads to Amazon S3 and IPFS
//! - Asynchronous request processing using Tokio
//! - Comprehensive error handling and logging
//! - Environment-based configuration
//! - File size validation and limits
//!
//! # Environment Variables
//!
//! The following environment variables are required:
//!
//! - `S3_BUCKET`: Amazon S3 bucket name
//! - `AWS_ACCESS_KEY_ID`: AWS access key
//! - `AWS_SECRET_ACCESS_KEY`: AWS secret key
//! - `AWS_REGION`: AWS region (optional, defaults to us-east-1)
//! - `S3_KEY`: S3 key prefix (optional, defaults to "uploads")
//! - `SERVER_HOST`: Server host (optional, defaults to "0.0.0.0")
//! - `SERVER_PORT`: Server port (optional, defaults to 8080)
//! - `MAX_FILE_SIZE`: Maximum file size in bytes (optional, defaults to 5MB)
//!
//! # Examples
//!
//! To start the service:
//!
//! ```bash
//! cargo run
//! ```
//!
//! To upload a file:
//!
//! ```bash
//! curl -X POST -F "file=@example.jpg" http://localhost:8080/upload
//! ```

mod api;
mod config;
mod domain;
mod error;
mod infrastructure;
mod utils;

use config::Config;
use log::{error, info};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;

/// Main entry point for the MemeNow Storage Service
///
/// This function initializes the application by:
/// 1. Setting up the logging system
/// 2. Loading and validating configuration
/// 3. Setting up API routes
/// 4. Starting the HTTP server
///
/// # Panics
///
/// The function will panic if:
/// - Required environment variables are missing
/// - Configuration validation fails
/// - The server fails to bind to the specified address
///
/// # Errors
///
/// Returns an error if configuration loading or validation fails.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger with default settings
    // Log level can be controlled via RUST_LOG environment variable
    // Example: RUST_LOG=debug cargo run
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting MemeNow Storage Service...");

    // Load configuration from environment variables
    let config = Config::from_env().map_err(|e| {
        error!("Failed to load configuration: {}", e);
        e
    })?;

    // Validate the configuration
    config.validate().map_err(|e| {
        error!("Configuration validation failed: {}", e);
        e
    })?;

    info!("Configuration loaded successfully");
    info!("S3 Bucket: {}", config.s3.bucket);
    info!("S3 Region: {}", config.s3.region);
    info!("Max file size: {} bytes", config.upload.max_file_size);

    // Set up API routes with the configuration
    let routes = api::upload::upload_routes(config.clone());

    // Parse the host address
    let host = IpAddr::from_str(&config.server.host).unwrap_or(IpAddr::V4(Ipv4Addr::new(
        0, 0, 0, 0,
    )));
    let addr = SocketAddr::new(host, config.server.port);

    info!("Server starting on http://{}", addr);
    info!("Upload endpoint: http://{}/upload", addr);
    info!("Ready to accept requests");

    // Start the server
    warp::serve(routes).run(addr).await;

    Ok(())
}
