//! File Utility Functions
//!
//! This module provides utility functions for common file system operations
//! such as creating directories, generating temporary file paths, and
//! reading/writing files.
//!
//! # Examples
//!
//! ```no_run
//! use memenow_storage_service::utils::file::{
//!     create_dir_if_not_exists,
//!     generate_temp_filepath,
//!     delete_file
//! };
//!
//! # fn main() -> std::io::Result<()> {
//! // Create a directory if it doesn't exist
//! create_dir_if_not_exists("/tmp/uploads")?;
//!
//! // Generate a temporary file path
//! let temp_path = generate_temp_filepath("jpg");
//!
//! // Delete a file
//! delete_file("/tmp/old_file.txt")?;
//! # Ok(())
//! # }
//! ```

use std::env;
use std::fs;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Create a directory if it does not already exist
///
/// This function creates a directory and all its parent directories if they
/// don't exist. It's idempotent - calling it multiple times has the same effect
/// as calling it once.
///
/// # Arguments
///
/// * `dir` - Path to the directory to create
///
/// # Returns
///
/// Returns `Ok(())` if the directory exists or was successfully created.
///
/// # Errors
///
/// Returns an error if:
/// - Insufficient permissions to create the directory
/// - The path exists but is not a directory
/// - I/O errors occur
///
/// # Examples
///
/// ```no_run
/// use memenow_storage_service::utils::file::create_dir_if_not_exists;
///
/// # fn main() -> std::io::Result<()> {
/// create_dir_if_not_exists("/tmp/uploads")?;
/// create_dir_if_not_exists("/tmp/uploads/images")?;
/// # Ok(())
/// # }
/// ```
pub fn create_dir_if_not_exists(dir: &str) -> io::Result<()> {
    let path = Path::new(dir);
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Generate a temporary file path with a unique UUID-based name
///
/// This function creates a unique temporary file path using the system's
/// temporary directory and a UUID v4 identifier. The file is not created,
/// only the path is returned.
///
/// # Arguments
///
/// * `extension` - File extension (without the leading dot)
///
/// # Returns
///
/// Returns a `PathBuf` containing the generated temporary file path
///
/// # Examples
///
/// ```
/// use memenow_storage_service::utils::file::generate_temp_filepath;
///
/// let temp_path = generate_temp_filepath("jpg");
/// assert!(temp_path.to_string_lossy().ends_with(".jpg"));
/// ```
pub fn generate_temp_filepath(extension: &str) -> PathBuf {
    let temp_dir = env::temp_dir();
    let filename = format!("{}.{}", Uuid::new_v4(), extension);
    temp_dir.join(filename)
}

/// Delete a file from the filesystem
///
/// This function removes a file at the specified path. If the file doesn't
/// exist, it returns an error.
///
/// # Arguments
///
/// * `filepath` - Path to the file to delete
///
/// # Returns
///
/// Returns `Ok(())` if the file was successfully deleted.
///
/// # Errors
///
/// Returns an error if:
/// - The file does not exist
/// - Insufficient permissions to delete the file
/// - The path points to a directory
/// - I/O errors occur
///
/// # Examples
///
/// ```no_run
/// use memenow_storage_service::utils::file::delete_file;
///
/// # fn main() -> std::io::Result<()> {
/// delete_file("/tmp/temporary_file.txt")?;
/// # Ok(())
/// # }
/// ```
pub fn delete_file(filepath: &str) -> io::Result<()> {
    fs::remove_file(filepath)
}

/// Read a file and return its contents as a string
///
/// This function reads the entire contents of a file into memory and
/// returns it as a UTF-8 string. Use this for text files only.
///
/// # Arguments
///
/// * `filepath` - Path to the file to read
///
/// # Returns
///
/// Returns the file contents as a `String` on success.
///
/// # Errors
///
/// Returns an error if:
/// - The file does not exist
/// - Insufficient permissions to read the file
/// - The file contains invalid UTF-8
/// - I/O errors occur
///
/// # Examples
///
/// ```no_run
/// use memenow_storage_service::utils::file::read_file_to_string;
///
/// # fn main() -> std::io::Result<()> {
/// let contents = read_file_to_string("/tmp/config.txt")?;
/// println!("File contents: {}", contents);
/// # Ok(())
/// # }
/// ```
///
/// # Performance Considerations
///
/// This function loads the entire file into memory. For large files,
/// consider using streaming approaches instead.
pub fn read_file_to_string(filepath: &str) -> io::Result<String> {
    let path = Path::new(filepath);
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

/// Write a string to a file, creating or overwriting it
///
/// This function writes the provided string content to a file, creating
/// the file if it doesn't exist or overwriting it if it does.
///
/// # Arguments
///
/// * `filepath` - Path to the file to write
/// * `contents` - String content to write to the file
///
/// # Returns
///
/// Returns `Ok(())` if the file was successfully written.
///
/// # Errors
///
/// Returns an error if:
/// - Insufficient permissions to write to the location
/// - The parent directory does not exist
/// - Disk is full
/// - I/O errors occur
///
/// # Examples
///
/// ```no_run
/// use memenow_storage_service::utils::file::write_string_to_file;
///
/// # fn main() -> std::io::Result<()> {
/// write_string_to_file("/tmp/output.txt", "Hello, world!")?;
/// # Ok(())
/// # }
/// ```
///
/// # Safety
///
/// This function will overwrite existing files without warning.
/// Ensure you want to replace the file before calling this function.
pub fn write_string_to_file(filepath: &str, contents: &str) -> io::Result<()> {
    let path = Path::new(filepath);
    let mut file = File::create(path)?;
    file.write_all(contents.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_generate_temp_filepath() {
        let path = generate_temp_filepath("txt");
        assert!(path.to_string_lossy().ends_with(".txt"));
    }

    #[test]
    fn test_generate_temp_filepath_different_extensions() {
        let path1 = generate_temp_filepath("jpg");
        let path2 = generate_temp_filepath("png");

        assert!(path1.to_string_lossy().ends_with(".jpg"));
        assert!(path2.to_string_lossy().ends_with(".png"));
        assert_ne!(path1, path2); // Should generate unique paths
    }

    #[test]
    fn test_write_and_read_file() -> io::Result<()> {
        let temp_path = generate_temp_filepath("txt");
        let temp_path_str = temp_path.to_str().unwrap();
        let content = "test content";

        write_string_to_file(temp_path_str, content)?;
        let read_content = read_file_to_string(temp_path_str)?;

        assert_eq!(content, read_content);

        // Clean up
        fs::remove_file(temp_path)?;

        Ok(())
    }
}
