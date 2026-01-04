//! File I/O abstraction module for cross-platform file operations
//!
//! This module provides safe, idiomatic wrappers around common file operations
//! that were originally in the C codebase (files.c, dirs.c, temp.c).

pub mod dirs;
pub mod ffi;
pub mod files;
pub mod temp;

// Re-exports for convenience
pub use dirs::{
    create_directory, create_directory_all, current_dir, directory_exists, is_empty,
    list_directory, list_files, list_subdirs, remove_directory, remove_directory_all,
    set_current_dir, DirEntry, DirError, DirHandle,
};
pub use files::{
    copy_file, copy_file_with_buffer, delete_file, file_exists, get_file_size, is_directory,
    is_file, FileError,
};
pub use temp::{
    append_file, cleanup_dir, create_temp_dir, create_temp_file, file_size as temp_file_size,
    get_unique_temp_dir, read_file as temp_read_file, write_file as temp_write_file,
};

use anyhow::{Context, Result};
use std::fs::File;
use std::path::Path;

/// Copy a file from source to destination (alias to module function)
///
/// # Arguments
/// * `src` - Source file path
/// * `dst` - Destination file path
///
/// # Errors
/// Returns an error if the source doesn't exist or the copy fails
pub fn copy_file_simple(src: &Path, dst: &Path) -> Result<()> {
    std::fs::copy(src, dst)
        .map(|_| ())
        .with_context(|| format!("Failed to copy file from {:?} to {:?}", src, dst))
}

/// Delete a file (alias to module function)
///
/// # Arguments
/// * `path` - Path to the file to delete
///
/// # Errors
/// Returns an error if the file doesn't exist or deletion fails
pub fn delete_file_simple(path: &Path) -> Result<()> {
    std::fs::remove_file(path).with_context(|| format!("Failed to delete file {:?}", path))
}

/// Get file size in bytes (alias to module function)
///
/// # Arguments
/// * `path` - Path to the file
///
/// # Errors
/// Returns an error if the file doesn't exist or metadata can't be read
pub fn get_file_size_simple(path: &Path) -> Result<u64> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Failed to get metadata for file {:?}", path))?;
    Ok(metadata.len())
}

/// Get a temporary directory path
///
/// # Returns
/// A path to a temporary directory
pub fn temp_dir() -> Result<std::path::PathBuf> {
    Ok(std::env::temp_dir())
}

/// Create a temporary file and return its path
///
/// # Returns
/// Path to the newly created temporary file
pub fn create_temp_file_legacy() -> Result<std::path::PathBuf> {
    let temp_dir = temp_dir()?;
    let temp_file = temp_dir.join(format!("uqm_temp_{}", std::process::id()));
    File::create(&temp_file)
        .with_context(|| format!("Failed to create temp file {:?}", temp_file))?;
    Ok(temp_file)
}

/// List directory contents (legacy alias)
///
/// # Arguments
/// * `path` - Directory path to list
///
/// # Returns
/// Vector of file/directory names
pub fn list_directory_legacy(path: &Path) -> Result<Vec<String>> {
    let mut files = Vec::new();

    for entry in
        std::fs::read_dir(path).with_context(|| format!("Failed to list directory {:?}", path))?
    {
        let entry = entry.with_context(|| "Failed to read directory entry")?;
        if let Some(name) = entry.file_name().to_str() {
            files.push(name.to_string());
        }
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_file_exists() {
        // Test with a non-existent file
        assert!(!file_exists(Path::new("/this/path/should/not/exist.txt")));

        // Test with temp directory (should exist but isn't a file)
        let temp = temp_dir().unwrap();
        assert!(!file_exists(&temp)); // It's a directory, not a file
    }

    #[test]
    fn test_copy_file_simple() {
        let temp = temp_dir().unwrap();
        let src_file = temp.join("test_src.txt");
        let dst_file = temp.join("test_dst.txt");

        // Create source file
        {
            let mut f = File::create(&src_file).unwrap();
            f.write_all(b"test content").unwrap();
        }

        // Copy file
        let result = copy_file_simple(&src_file, &dst_file);
        assert!(result.is_ok());

        // Verify copy
        assert!(dst_file.exists());
        let content = std::fs::read_to_string(&dst_file).unwrap();
        assert_eq!(content, "test content");

        // Cleanup
        let _ = std::fs::remove_file(&src_file);
        let _ = std::fs::remove_file(&dst_file);
    }

    #[test]
    fn test_copy_file_nonexistent_source() {
        let temp = temp_dir().unwrap();
        let src_file = temp.join("nonexistent.txt");
        let dst_file = temp.join("dst.txt");

        let result = copy_file_simple(&src_file, &dst_file);
        assert!(result.is_err());

        // Destination should not exist
        assert!(!dst_file.exists());
    }

    #[test]
    fn test_delete_file_simple() {
        let temp = temp_dir().unwrap();
        let test_file = temp.join("delete_test.txt");

        // Create file
        {
            let mut f = File::create(&test_file).unwrap();
            f.write_all(b"delete me").unwrap();
        }

        assert!(test_file.exists());
        let result = delete_file_simple(&test_file);
        assert!(result.is_ok());
        assert!(!test_file.exists());
    }

    #[test]
    fn test_get_file_size_simple() {
        let temp = temp_dir().unwrap();
        let test_file = temp.join("size_test.txt");

        {
            let mut f = File::create(&test_file).unwrap();
            f.write_all(b"test content").unwrap();
        }

        let size = get_file_size_simple(&test_file).unwrap();
        assert_eq!(size, 12); // "test content" is 12 bytes

        // Cleanup
        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn test_list_directory_legacy() {
        let dir = create_temp_dir().unwrap();
        let test_files = vec!["test1.txt", "test2.txt", "test3.txt"];

        // Create test files
        for filename in &test_files {
            let path = dir.join(filename);
            let mut f = File::create(&path).unwrap();
            f.write_all(b"test").unwrap();
        }

        // List directory
        let files = list_directory_legacy(&dir).unwrap();

        // Check that our test files are present
        for test_file in &test_files {
            assert!(files.contains(&test_file.to_string()));
        }

        // Cleanup
        cleanup_dir(&dir).unwrap();
    }
}
