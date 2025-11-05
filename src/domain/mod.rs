//! Domain Layer Module
//!
//! This module contains the core business logic of the application.
//! It implements the domain model and orchestrates operations across
//! different infrastructure services.
//!
//! # Submodules
//!
//! - `services`: Service layer implementing business operations for file uploads
//!
//! # Architecture
//!
//! The domain layer is independent of infrastructure concerns and focuses on:
//!
//! - Business rules and validation
//! - Coordinating infrastructure services
//! - Data transformation and processing
//! - Error handling and recovery
//!
//! This separation allows the business logic to remain clean and testable,
//! independent of external service implementations.

pub mod services;
