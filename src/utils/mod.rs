//! Utilities Module
//!
//! This module provides standalone, reusable file-system helpers. They are part of the
//! library's public surface but are independent of the upload path: `handle_upload`
//! performs its own streaming temp-file handling in [`crate::domain::services`] rather
//! than going through these helpers.
//!
//! # Submodules
//!
//! - `file`: File system utility functions for reading, writing, and managing files

pub mod file;
