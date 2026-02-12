//! Resource loader - loads resources from the filesystem using ResourceIndex
//!
//! This module provides the ResourceLoader which maps resource names to files
//! and loads their contents. Currently supports directory-based loading;
//! pack file (.uqm) support can be added later.
//!
//! # Example
//! ```ignore
//! let index = ResourceIndex::from_file("content/uqm.rmp")?;
//! let loader = ResourceLoader::new("/path/to/content", index);
//! let data = loader.load("comm.orz.dialogue")?;
//! ```
//!
//! # Reference
//! See `sc2/src/libs/resource/getres.c` for the C implementation.

use std::fs;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use super::index::{ResourceEntry, ResourceIndex};

/// Error type for resource loading operations
#[derive(Debug)]
pub enum LoaderError {
    /// Resource not found in the index
    ResourceNotFound(String),
    /// I/O error reading resource file
    IoError(io::Error),
    /// Invalid path (path traversal, etc.)
    InvalidPath(String),
    /// Invalid UTF-8 when loading as string
    Utf8Error(std::string::FromUtf8Error),
}

impl std::fmt::Display for LoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoaderError::ResourceNotFound(name) => write!(f, "Resource not found: {}", name),
            LoaderError::IoError(e) => write!(f, "I/O error: {}", e),
            LoaderError::InvalidPath(path) => write!(f, "Invalid path: {}", path),
            LoaderError::Utf8Error(e) => write!(f, "UTF-8 error: {}", e),
        }
    }
}

impl std::error::Error for LoaderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LoaderError::IoError(e) => Some(e),
            LoaderError::Utf8Error(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for LoaderError {
    fn from(err: io::Error) -> Self {
        LoaderError::IoError(err)
    }
}

impl From<std::string::FromUtf8Error> for LoaderError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        LoaderError::Utf8Error(err)
    }
}

pub type Result<T> = std::result::Result<T, LoaderError>;

/// Resource loader - loads resources from the filesystem
///
/// The loader uses a ResourceIndex to map resource names to file paths,
/// then loads the file contents from the base content directory.
#[derive(Debug)]
pub struct ResourceLoader {
    /// Base path for content directory
    base_path: PathBuf,
    /// Resource index mapping names to files
    index: ResourceIndex,
}

impl ResourceLoader {
    /// Create a new resource loader
    ///
    /// # Arguments
    /// * `base_path` - Base directory for content files
    /// * `index` - Resource index mapping names to file paths
    ///
    /// # Example
    /// ```ignore
    /// let index = ResourceIndex::from_file("content/uqm.rmp")?;
    /// let loader = ResourceLoader::new("/path/to/content", index);
    /// ```
    pub fn new<P: AsRef<Path>>(base_path: P, index: ResourceIndex) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
            index,
        }
    }

    /// Get the base path
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// Get a reference to the resource index
    pub fn index(&self) -> &ResourceIndex {
        &self.index
    }

    /// Resolve a resource entry to a full filesystem path
    ///
    /// Returns an error if the path contains path traversal attempts.
    fn resolve_path(&self, entry: &ResourceEntry) -> Result<PathBuf> {
        let file_path = &entry.file_path;

        // Check for path traversal attempts
        if file_path.contains("..") {
            return Err(LoaderError::InvalidPath(format!(
                "Path traversal not allowed: {}",
                file_path
            )));
        }

        // Build the full path
        let full_path = self.base_path.join(file_path);

        // Verify the resolved path is still under base_path
        // This handles edge cases like symlinks
        if let Ok(canonical_base) = self.base_path.canonicalize() {
            if let Ok(canonical_full) = full_path.canonicalize() {
                if !canonical_full.starts_with(&canonical_base) {
                    return Err(LoaderError::InvalidPath(format!(
                        "Path escapes content directory: {}",
                        file_path
                    )));
                }
            }
        }

        Ok(full_path)
    }

    /// Load a resource as raw bytes
    ///
    /// # Arguments
    /// * `name` - Resource name to load
    ///
    /// # Returns
    /// The raw bytes of the resource
    ///
    /// # Errors
    /// * `ResourceNotFound` - Resource name not in index
    /// * `IoError` - Error reading the file
    /// * `InvalidPath` - Path traversal attempt detected
    pub fn load(&self, name: &str) -> Result<Vec<u8>> {
        // Look up the resource in the index
        let entry = self
            .index
            .lookup(name)
            .ok_or_else(|| LoaderError::ResourceNotFound(name.to_string()))?;

        // Resolve to full path
        let full_path = self.resolve_path(entry)?;

        // Load the file
        if entry.file_offset > 0 || entry.file_size > 0 {
            // Packed resource with offset/size
            self.load_partial(&full_path, entry.file_offset as u64, entry.file_size as usize)
        } else {
            // Whole file
            fs::read(&full_path).map_err(LoaderError::from)
        }
    }

    /// Load a partial file (for packed resources)
    fn load_partial(&self, path: &Path, offset: u64, size: usize) -> Result<Vec<u8>> {
        let mut file = fs::File::open(path)?;
        file.seek(SeekFrom::Start(offset))?;

        let mut buffer = vec![0u8; size];
        file.read_exact(&mut buffer)?;

        Ok(buffer)
    }

    /// Load a resource as a UTF-8 string
    ///
    /// # Arguments
    /// * `name` - Resource name to load
    ///
    /// # Returns
    /// The resource content as a UTF-8 string
    ///
    /// # Errors
    /// * `ResourceNotFound` - Resource name not in index
    /// * `IoError` - Error reading the file
    /// * `InvalidPath` - Path traversal attempt detected
    /// * `Utf8Error` - File content is not valid UTF-8
    pub fn load_string(&self, name: &str) -> Result<String> {
        let bytes = self.load(name)?;
        String::from_utf8(bytes).map_err(LoaderError::from)
    }

    /// Check if a resource exists in the index
    ///
    /// Note: This only checks if the resource is in the index,
    /// not whether the actual file exists on disk.
    ///
    /// # Arguments
    /// * `name` - Resource name to check
    pub fn exists(&self, name: &str) -> bool {
        self.index.contains(name)
    }

    /// Check if a resource file exists on disk
    ///
    /// This checks both that the resource is in the index AND
    /// that the file exists on disk.
    ///
    /// # Arguments
    /// * `name` - Resource name to check
    pub fn file_exists(&self, name: &str) -> bool {
        if let Some(entry) = self.index.lookup(name) {
            if let Ok(path) = self.resolve_path(entry) {
                return path.exists();
            }
        }
        false
    }

    /// Get the number of resources in the index
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Check if the loader has no resources
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// List all resource names in the index
    pub fn resource_names(&self) -> impl Iterator<Item = &str> {
        self.index.names()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper to create a test environment with files and index
    fn setup_test_env() -> (TempDir, ResourceLoader) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let base_path = temp_dir.path();

        // Create test files
        fs::write(base_path.join("test.txt"), "Hello, World!").unwrap();
        fs::write(base_path.join("data.bin"), vec![0x01, 0x02, 0x03, 0x04]).unwrap();

        // Create subdirectory with files
        fs::create_dir(base_path.join("subdir")).unwrap();
        fs::write(base_path.join("subdir/nested.txt"), "Nested content").unwrap();

        // Create index
        let mut index = ResourceIndex::new();
        index.insert(ResourceEntry::new("test.resource", "test.txt"));
        index.insert(ResourceEntry::new("binary.data", "data.bin"));
        index.insert(ResourceEntry::new("nested.resource", "subdir/nested.txt"));

        let loader = ResourceLoader::new(base_path, index);
        (temp_dir, loader)
    }

    #[test]
    fn test_loader_new() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let index = ResourceIndex::new();
        let loader = ResourceLoader::new(temp_dir.path(), index);

        assert_eq!(loader.base_path(), temp_dir.path());
        assert!(loader.is_empty());
        assert_eq!(loader.len(), 0);
    }

    #[test]
    fn test_load_existing_resource() {
        let (_temp_dir, loader) = setup_test_env();

        let data = loader.load("test.resource").expect("Should load");
        assert_eq!(data, b"Hello, World!");

        // Binary file
        let bin_data = loader.load("binary.data").expect("Should load binary");
        assert_eq!(bin_data, vec![0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn test_load_nonexistent_resource() {
        let (_temp_dir, loader) = setup_test_env();

        let result = loader.load("nonexistent.resource");
        assert!(result.is_err());

        match result.unwrap_err() {
            LoaderError::ResourceNotFound(name) => {
                assert_eq!(name, "nonexistent.resource");
            }
            _ => panic!("Expected ResourceNotFound error"),
        }
    }

    #[test]
    fn test_load_string() {
        let (_temp_dir, loader) = setup_test_env();

        let content = loader.load_string("test.resource").expect("Should load string");
        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_load_string_invalid_utf8() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let base_path = temp_dir.path();

        // Create a file with invalid UTF-8
        fs::write(base_path.join("invalid.bin"), vec![0xFF, 0xFE, 0x00, 0x01]).unwrap();

        let mut index = ResourceIndex::new();
        index.insert(ResourceEntry::new("invalid.resource", "invalid.bin"));

        let loader = ResourceLoader::new(base_path, index);
        let result = loader.load_string("invalid.resource");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LoaderError::Utf8Error(_)));
    }

    #[test]
    fn test_exists() {
        let (_temp_dir, loader) = setup_test_env();

        assert!(loader.exists("test.resource"));
        assert!(loader.exists("binary.data"));
        assert!(loader.exists("nested.resource"));
        assert!(!loader.exists("nonexistent.resource"));

        // Case insensitive (default)
        assert!(loader.exists("TEST.RESOURCE"));
        assert!(loader.exists("Test.Resource"));
    }

    #[test]
    fn test_loader_with_subdirectories() {
        let (_temp_dir, loader) = setup_test_env();

        let content = loader.load_string("nested.resource").expect("Should load nested");
        assert_eq!(content, "Nested content");
    }

    #[test]
    fn test_path_traversal_protection() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let mut index = ResourceIndex::new();
        index.insert(ResourceEntry::new("evil.resource", "../../../etc/passwd"));

        let loader = ResourceLoader::new(temp_dir.path(), index);
        let result = loader.load("evil.resource");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LoaderError::InvalidPath(_)));
    }

    #[test]
    fn test_file_exists() {
        let (_temp_dir, loader) = setup_test_env();

        // Resource exists in index and file exists on disk
        assert!(loader.file_exists("test.resource"));

        // Resource doesn't exist in index
        assert!(!loader.file_exists("nonexistent.resource"));
    }

    #[test]
    fn test_file_exists_missing_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let mut index = ResourceIndex::new();
        index.insert(ResourceEntry::new("missing.resource", "missing.txt"));

        let loader = ResourceLoader::new(temp_dir.path(), index);

        // Resource exists in index but file doesn't exist on disk
        assert!(loader.exists("missing.resource")); // In index
        assert!(!loader.file_exists("missing.resource")); // Not on disk
    }

    #[test]
    fn test_resource_names() {
        let (_temp_dir, loader) = setup_test_env();

        let names: Vec<&str> = loader.resource_names().collect();
        assert_eq!(names.len(), 3);

        // All resources should be listed (order may vary)
        assert!(loader.exists("test.resource"));
        assert!(loader.exists("binary.data"));
        assert!(loader.exists("nested.resource"));
    }

    #[test]
    fn test_load_partial_resource() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let base_path = temp_dir.path();

        // Create a file with known content
        let content: Vec<u8> = (0..100).collect();
        fs::write(base_path.join("packed.dat"), &content).unwrap();

        // Create index with offset and size
        let mut index = ResourceIndex::new();
        index.insert(ResourceEntry::with_offset("partial.resource", "packed.dat", 10, 20));

        let loader = ResourceLoader::new(base_path, index);
        let data = loader.load("partial.resource").expect("Should load partial");

        // Should get bytes 10-29 (20 bytes starting at offset 10)
        let expected: Vec<u8> = (10..30).collect();
        assert_eq!(data, expected);
    }

    #[test]
    fn test_loader_error_display() {
        let err = LoaderError::ResourceNotFound("test".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Resource not found"));
        assert!(msg.contains("test"));

        let err2 = LoaderError::InvalidPath("../evil".to_string());
        let msg2 = format!("{}", err2);
        assert!(msg2.contains("Invalid path"));
    }

    #[test]
    fn test_loader_index_accessor() {
        let (_temp_dir, loader) = setup_test_env();

        let index = loader.index();
        assert_eq!(index.len(), 3);
        assert!(index.contains("test.resource"));
    }
}
