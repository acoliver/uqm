// File I/O Operations

use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileError {
    NotFound,
    PermissionDenied,
    AlreadyExists,
    InvalidPath,
    IoError,
    IsADirectory,
}

impl std::fmt::Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileError::NotFound => write!(f, "File not found"),
            FileError::PermissionDenied => write!(f, "Permission denied"),
            FileError::AlreadyExists => write!(f, "File already exists"),
            FileError::InvalidPath => write!(f, "Invalid path"),
            FileError::IoError => write!(f, "I/O error"),
            FileError::IsADirectory => write!(f, "Is a directory"),
        }
    }
}

impl std::error::Error for FileError {}

impl From<io::Error> for FileError {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound => FileError::NotFound,
            io::ErrorKind::PermissionDenied => FileError::PermissionDenied,
            io::ErrorKind::AlreadyExists => FileError::AlreadyExists,
            io::ErrorKind::InvalidInput => FileError::InvalidPath,
            _ => FileError::IoError,
        }
    }
}

pub fn file_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

pub fn copy_file(src_path: &Path, dst_path: &Path) -> Result<(), FileError> {
    if !src_path.exists() {
        return Err(FileError::NotFound);
    }

    if dst_path.exists() {
        return Err(FileError::AlreadyExists);
    }

    let src_metadata = fs::metadata(src_path)?;
    if src_metadata.is_dir() {
        return Err(FileError::IsADirectory);
    }

    fs::copy(src_path, dst_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// Get a unique test directory for the current test
    fn get_test_dir() -> PathBuf {
        let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut dir = env::temp_dir();
        dir.push(format!(
            "uqm_files_test_{:08}_{}",
            std::process::id(),
            counter
        ));
        dir
    }

    /// Setup: create the test directory
    fn setup_test_env() -> PathBuf {
        let test_dir = get_test_dir();
        fs::create_dir_all(&test_dir)
            .unwrap_or_else(|e| panic!("Failed to create test directory {:?}: {:?}", test_dir, e));
        test_dir
    }

    /// Cleanup: remove the test directory
    fn cleanup_test_env(test_dir: &Path) {
        if test_dir.exists() {
            let _ = fs::remove_dir_all(test_dir);
        }
    }

    use std::path::PathBuf;

    #[test]
    fn test_file_exists() {
        let test_dir = setup_test_env();
        let test_file = test_dir.join("test_file.txt");
        let not_exist_file = test_dir.join("not_exist.txt");

        assert!(!file_exists(&test_file));
        fs::write(&test_file, "test content").unwrap();
        assert!(file_exists(&test_file));
        assert!(!file_exists(&not_exist_file));
        cleanup_test_env(&test_dir);
    }

    #[test]
    fn test_copy_file() {
        let test_dir = setup_test_env();
        let src_file = test_dir.join("src.txt");
        let dst_file = test_dir.join("dst.txt");

        fs::write(&src_file, "test content for copying").unwrap();

        let result = copy_file(&src_file, &dst_file);
        assert!(result.is_ok());
        assert!(file_exists(&dst_file));
        let dst_content = fs::read_to_string(&dst_file).unwrap();
        assert_eq!(dst_content, "test content for copying");
        cleanup_test_env(&test_dir);
    }

    #[test]
    fn test_copy_file_source_not_found() {
        let test_dir = setup_test_env();
        let src_file = test_dir.join("nonexistent.txt");
        let dst_file = test_dir.join("dst.txt");

        let result = copy_file(&src_file, &dst_file);
        assert_eq!(result, Err(FileError::NotFound));
        cleanup_test_env(&test_dir);
    }

    #[test]
    fn test_copy_file_destination_exists() {
        let test_dir = setup_test_env();
        let src_file = test_dir.join("src.txt");
        let dst_file = test_dir.join("dst.txt");

        fs::write(&src_file, "source").unwrap();
        fs::write(&dst_file, "destination").unwrap();

        let result = copy_file(&src_file, &dst_file);
        assert_eq!(result, Err(FileError::AlreadyExists));
        let dst_content = fs::read_to_string(&dst_file).unwrap();
        assert_eq!(dst_content, "destination");
        cleanup_test_env(&test_dir);
    }

    #[test]
    fn test_copy_directory_as_file() {
        let test_dir = setup_test_env();
        let src_dir = test_dir.join("srcdir");
        let dst_file = test_dir.join("dst.txt");

        fs::create_dir(&src_dir).unwrap();
        let result = copy_file(&src_dir, &dst_file);
        assert_eq!(result, Err(FileError::IsADirectory));
        cleanup_test_env(&test_dir);
    }

    #[test]
    fn test_copy_empty_file() {
        let test_dir = setup_test_env();
        let src_file = test_dir.join("empty.txt");
        let dst_file = test_dir.join("empty_copy.txt");

        fs::write(&src_file, "").unwrap();

        let result = copy_file(&src_file, &dst_file);
        assert!(result.is_ok());
        assert_eq!(fs::metadata(&dst_file).unwrap().len(), 0);
        cleanup_test_env(&test_dir);
    }

    #[test]
    fn test_file_error_display() {
        let err = FileError::NotFound;
        let display = format!("{}", err);
        assert!(display.contains("not found"));

        let err = FileError::PermissionDenied;
        let display = format!("{}", err);
        let display_lower = display.to_lowercase();
        assert!(display_lower.contains("permission"));

        let err = FileError::AlreadyExists;
        let display = format!("{}", err);
        assert!(display.contains("already exists"));
    }

    #[test]
    fn test_file_error_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "test");
        let file_err: FileError = io_err.into();
        assert_eq!(file_err, FileError::NotFound);

        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "test");
        let file_err: FileError = io_err.into();
        assert_eq!(file_err, FileError::PermissionDenied);

        let io_err = io::Error::new(io::ErrorKind::AlreadyExists, "test");
        let file_err: FileError = io_err.into();
        assert_eq!(file_err, FileError::AlreadyExists);
    }
}
