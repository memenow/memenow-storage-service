//! Infrastructure Layer Module
//!
//! This module contains all the infrastructure services that interact with
//! external systems and storage providers. It provides abstractions for
//! file storage operations across different platforms.
//!
//! # Submodules
//!
//! - `s3`: Amazon S3 cloud storage integration
//! - `ipfs`: InterPlanetary File System (IPFS) decentralized storage integration
//!
//! # Design Pattern
//!
//! The infrastructure layer follows the Repository pattern, isolating the
//! domain logic from external storage implementations. This allows for:
//!
//! - Easy testing with mock implementations
//! - Flexibility to swap storage providers
//! - Clear separation of concerns
//!
//! # Examples
//!
//! ```no_run
//! use memenow_storage_service::infrastructure::{s3, ipfs};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Upload to S3
//! let s3_url = s3::upload_to_s3("/tmp/file.jpg", "my-bucket", "uploads/file.jpg").await?;
//!
//! // Upload to IPFS
//! let ipfs_hash = ipfs::upload_to_ipfs("/tmp/file.jpg").await?;
//! # Ok(())
//! # }
//! ```

pub mod ipfs;
pub mod s3;
