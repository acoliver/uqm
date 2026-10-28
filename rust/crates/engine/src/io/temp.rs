//! Temporary file and directory operations
//!
//! This module provides safe utilities for creating and managing temporary
//! files and directories, with automatic cleanup support.

use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Atomic counter for generating unique temp directory names
static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Get a unique temporary directory path for testing
///
/// Uses PID + atomic counter to ensure uniqueness across test runs
///
/// # Returns
/// A new unique temp directory path
pub fn get_unique_temp_dir() -> PathBuf {
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut dir = env::temp_dir();
    dir.push(format!("uqm_test_{:08}_{}", std::process::id(), counter));
    dir
}

/// Create a temporary directory
///
/// # Returns
/// Path to the newly created temporary directory
///
/// # Errors
/// Returns an error if directory creation fails
pub fn create_temp_dir() -> io::Result<PathBuf> {
    let dir = get_unique_temp_dir();
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Create a temporary file with optional content
///
/// # Arguments
/// * `content` - Optional content to write to the file
///
/// # Returns
/// Path to the newly created temporary file
///
/// # Errors
/// Returns an error if file creation or writing fails
pub fn create_temp_file(content: Option<&[u8]>) -> io::Result<PathBuf> {
    let dir = create_temp_dir()?;
    let file_path = dir.join("temp_file.txt");
    let mut file = File::create(&file_path)?;
    if let Some(data) = content {
        file.write_all(data)?;
    }
    Ok(file_path)
}

/// Write content to a file
///
/// # Arguments
/// * `path` - Path to the file
/// * `content` - Content to write
///
/// # Errors
/// Returns an error if the file cannot be opened or written
pub fn write_file(path: &Path, content: &[u8]) -> io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(content)?;
    Ok(())
}

/// Read content from a file
///
/// # Arguments
/// * `path` - Path to the file
///
/// # Returns
/// The file content as a vector of bytes
///
/// # Errors
/// Returns an error if the file cannot be opened or read
pub fn read_file(path: &Path) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

/// Append content to a file
///
/// # Arguments
/// * `path` - Path to the file
/// * `content` - Content to append
///
/// # Errors
/// Returns an error if the file cannot be opened or appended to
pub fn append_file(path: &Path, content: &[u8]) -> io::Result<()> {
    let mut file = File::options().append(true).open(path)?;
    file.write_all(content)?;
    Ok(())
}

/// Get the size of a file in bytes
///
/// # Arguments
/// * `path` - Path to the file
///
/// # Returns
/// The file size in bytes
///
/// # Errors
/// Returns an error if the file metadata cannot be read
pub fn file_size(path: &Path) -> io::Result<u64> {
    let metadata = fs::metadata(path)?;
    Ok(metadata.len())
}

/// Remove a directory and all its contents
///
/// # Arguments
/// * `path` - Path to the directory
///
/// # Errors
/// Returns an error if the directory cannot be removed
pub fn cleanup_dir(path: &Path) -> io::Result<()> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unique_temp_dir() {
        let dir1 = get_unique_temp_dir();
        let dir2 = get_unique_temp_dir();
        assert_ne!(dir1, dir2);
    }

    #[test]
    fn test_create_temp_dir() {
        let dir = create_temp_dir().unwrap();
        assert!(dir.exists());
        assert!(dir.is_dir());
        cleanup_dir(&dir).unwrap();
    }

    #[test]
    fn test_create_temp_file_with_content() {
        let content = b"Hello, World!";
        let file_path = create_temp_file(Some(content)).unwrap();
        assert!(file_path.exists());
        assert!(file_path.is_file());
        cleanup_dir(file_path.parent().unwrap()).unwrap();
    }

    #[test]
    fn test_create_temp_file_empty() {
        let file_path = create_temp_file(None).unwrap();
        assert!(file_path.exists());
        assert_eq!(file_size(&file_path).unwrap(), 0);
        cleanup_dir(file_path.parent().unwrap()).unwrap();
    }

    #[test]
    fn test_write_and_read_file() {
        let dir = create_temp_dir().unwrap();
        let file_path = dir.join("test.bin");
        let content = b"test content";

        write_file(&file_path, content).unwrap();
        assert!(file_path.exists());

        let read_data = read_file(&file_path).unwrap();
        assert_eq!(read_data, content);

        cleanup_dir(&dir).unwrap();
    }

    #[test]
    fn test_append_file() {
        let dir = create_temp_dir().unwrap();
        let file_path = dir.join("append_test.txt");

        write_file(&file_path, b"initial").unwrap();
        let initial_size = file_size(&file_path).unwrap();

        append_file(&file_path, b" appended").unwrap();
        let final_size = file_size(&file_path).unwrap();

        assert_eq!(final_size, initial_size + 9);

        let full_content = read_file(&file_path).unwrap();
        assert_eq!(String::from_utf8(full_content).unwrap(), "initial appended");

        cleanup_dir(&dir).unwrap();
    }

    #[test]
    fn test_file_size() {
        let file_path = create_temp_file(Some(b"12345")).unwrap();
        assert_eq!(file_size(&file_path).unwrap(), 5);
        cleanup_dir(file_path.parent().unwrap()).unwrap();
    }

    #[test]
    fn test_cleanup_dir() {
        let dir = create_temp_dir().unwrap();
        let file_path = dir.join("cleanup_test.txt");
        write_file(&file_path, b"cleanup test").unwrap();

        cleanup_dir(&dir).unwrap();
        assert!(!dir.exists());
    }

    #[test]
    fn test_multiple_isolated_dirs() {
        let dir1 = create_temp_dir().unwrap();
        let dir2 = create_temp_dir().unwrap();

        let file1 = dir1.join("file1.txt");
        let file2 = dir2.join("file2.txt");

        write_file(&file1, b"dir1 content").unwrap();
        write_file(&file2, b"dir2 content").unwrap();

        let content1 = read_file(&file1).unwrap();
        let content2 = read_file(&file2).unwrap();

        assert_eq!(content1, b"dir1 content");
        assert_eq!(content2, b"dir2 content");

        cleanup_dir(&dir1).unwrap();
        cleanup_dir(&dir2).unwrap();
    }
}
