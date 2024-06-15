// Import the necessary modules and types from the `std`, `uuid`, and `io` crates.
use std::env;
use std::fs;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// The `create_dir_if_not_exists` function creates a directory if it does not already exist.
///
/// It takes a string representing the path of the directory to be created.
///
/// # Arguments
///
/// * `dir` - A string representing the path of the directory to be created.
///
/// # Returns
///
/// This function returns an `io::Result` that contains `()` if the directory was successfully created,
/// or an error if there was an issue during the directory creation process.
pub fn create_dir_if_not_exists(dir: &str) -> io::Result<()> {
    let path = Path::new(dir);
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

/// The `generate_temp_filepath` function generates a temporary file path.
///
/// It takes a string representing the extension of the file.
///
/// # Arguments
///
/// * `extension` - A string representing the extension of the file.
///
/// # Returns
///
/// This function returns a `PathBuf` that represents the temporary file path.
pub fn generate_temp_filepath(extension: &str) -> PathBuf {
    let temp_dir = env::temp_dir();
    let filename = format!("{}.{}", Uuid::new_v4(), extension);
    temp_dir.join(filename)
}

/// The `delete_file` function deletes a file.
///
/// It takes a string representing the path of the file to be deleted.
///
/// # Arguments
///
/// * `filepath` - A string representing the path of the file to be deleted.
///
/// # Returns
///
/// This function returns an `io::Result` that contains `()` if the file was successfully deleted,
/// or an error if there was an issue during the file deletion process.
pub fn delete_file(filepath: &str) -> io::Result<()> {
    fs::remove_file(filepath)
}

/// The `read_file_to_string` function reads a file and returns its contents as a string.
///
/// It takes a string representing the path of the file to be read.
///
/// # Arguments
///
/// * `filepath` - A string representing the path of the file to be read.
///
/// # Returns
///
/// This function returns an `io::Result` that contains a string if the file was successfully read,
/// or an error if there was an issue during the file reading process.
pub fn read_file_to_string(filepath: &str) -> io::Result<String> {
    let path = Path::new(filepath);
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

/// The `write_string_to_file` function writes a string to a file.
///
/// It takes a string representing the path of the file to be written to, and a string representing the contents to be written.
///
/// # Arguments
///
/// * `filepath` - A string representing the path of the file to be written to.
/// * `contents` - A string representing the contents to be written.
///
/// # Returns
///
/// This function returns an `io::Result` that contains `()` if the string was successfully written to the file,
/// or an error if there was an issue during the file writing process.
pub fn write_string_to_file(filepath: &str, contents: &str) -> io::Result<()> {
    let path = Path::new(filepath);
    let mut file = File::create(path)?;
    file.write_all(contents.as_bytes())
}
