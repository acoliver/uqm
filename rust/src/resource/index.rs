//! Resource index system
//!
//! Parses and manages .rmp resource index files that map resource names to files.
//!
//! # RMP File Format
//! The C implementation (index.c, getres.c) uses a key-value format:
//! ```text
//! resource.name = path/to/file.ext
//! another.resource = another/path.ext
//! ```
//!
//! # Reference
//! See `sc2/src/libs/resource/index.c` and `getres.c` for the C implementation.

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;

/// Error type for resource index operations
#[derive(Debug)]
pub enum IndexError {
    /// I/O error reading index file
    IoError(io::Error),
    /// Parse error in index file
    ParseError(String),
    /// Resource not found
    NotFound(String),
}

impl std::fmt::Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexError::IoError(e) => write!(f, "I/O error: {}", e),
            IndexError::ParseError(s) => write!(f, "Parse error: {}", s),
            IndexError::NotFound(s) => write!(f, "Resource not found: {}", s),
        }
    }
}

impl std::error::Error for IndexError {}

impl From<io::Error> for IndexError {
    fn from(err: io::Error) -> Self {
        IndexError::IoError(err)
    }
}

pub type Result<T> = std::result::Result<T, IndexError>;

/// An entry in the resource index
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceEntry {
    /// Resource name (the key used for lookup)
    pub name: String,
    /// Path to the resource file (relative to content root)
    pub file_path: String,
    /// Offset within the file (for packed resources, 0 for standalone files)
    pub file_offset: u32,
    /// Size of the resource data (0 if unknown/whole file)
    pub file_size: u32,
}

impl ResourceEntry {
    /// Create a new resource entry
    pub fn new(name: impl Into<String>, file_path: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            file_path: file_path.into(),
            file_offset: 0,
            file_size: 0,
        }
    }

    /// Create a resource entry with offset and size (for packed resources)
    pub fn with_offset(
        name: impl Into<String>,
        file_path: impl Into<String>,
        offset: u32,
        size: u32,
    ) -> Self {
        Self {
            name: name.into(),
            file_path: file_path.into(),
            file_offset: offset,
            file_size: size,
        }
    }
}

/// Resource index - maps resource names to file locations
///
/// The index is loaded from .rmp files and provides fast lookup
/// of resource locations by name.
#[derive(Debug, Default)]
pub struct ResourceIndex {
    /// Map from resource name to entry
    entries: HashMap<String, ResourceEntry>,
    /// Whether lookups are case-sensitive
    case_sensitive: bool,
}

impl ResourceIndex {
    /// Create an empty resource index
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            case_sensitive: false, // UQM uses case-insensitive lookups
        }
    }

    /// Create an empty resource index with case sensitivity setting
    pub fn with_case_sensitivity(case_sensitive: bool) -> Self {
        Self {
            entries: HashMap::new(),
            case_sensitive,
        }
    }

    /// Parse a resource index from a reader
    ///
    /// The format is simple key = value pairs:
    /// ```text
    /// resource.name = path/to/file.ext
    /// # Comments start with #
    /// ```
    pub fn parse<R: Read>(reader: R) -> Result<Self> {
        let mut index = Self::new();
        let buf_reader = BufReader::new(reader);

        for (line_num, line_result) in buf_reader.lines().enumerate() {
            let line = line_result?;
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Parse key = value
            if let Some((key, value)) = trimmed.split_once('=') {
                let name = key.trim().to_string();
                let file_path = value.trim().to_string();

                if name.is_empty() {
                    return Err(IndexError::ParseError(format!(
                        "Empty resource name at line {}",
                        line_num + 1
                    )));
                }

                let entry = ResourceEntry::new(name.clone(), file_path);
                index.insert(entry);
            } else {
                // Line doesn't contain '=' - might be a parse error or different format
                // For now, skip non-conforming lines (the C code is lenient)
            }
        }

        Ok(index)
    }

    /// Parse a resource index from a file path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        Self::parse(file)
    }

    /// Parse a resource index from a string
    pub fn from_str(s: &str) -> Result<Self> {
        Self::parse(s.as_bytes())
    }

    /// Insert a resource entry
    pub fn insert(&mut self, entry: ResourceEntry) {
        let key = if self.case_sensitive {
            entry.name.clone()
        } else {
            entry.name.to_lowercase()
        };
        self.entries.insert(key, entry);
    }

    /// Look up a resource by name
    ///
    /// Returns `None` if the resource is not found.
    pub fn lookup(&self, name: &str) -> Option<&ResourceEntry> {
        let key = if self.case_sensitive {
            name.to_string()
        } else {
            name.to_lowercase()
        };
        self.entries.get(&key)
    }

    /// Look up a resource by name, returning an error if not found
    pub fn get(&self, name: &str) -> Result<&ResourceEntry> {
        self.lookup(name)
            .ok_or_else(|| IndexError::NotFound(name.to_string()))
    }

    /// Check if a resource exists
    pub fn contains(&self, name: &str) -> bool {
        self.lookup(name).is_some()
    }

    /// Get the number of entries in the index
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over all entries
    pub fn iter(&self) -> impl Iterator<Item = &ResourceEntry> {
        self.entries.values()
    }

    /// Get all resource names
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.values().map(|e| e.name.as_str())
    }

    /// Merge another index into this one
    ///
    /// Entries from `other` will overwrite entries with the same name in `self`.
    pub fn merge(&mut self, other: ResourceIndex) {
        for entry in other.entries.into_values() {
            self.insert(entry);
        }
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_entry_new() {
        let entry = ResourceEntry::new("test.resource", "path/to/file.ext");
        assert_eq!(entry.name, "test.resource");
        assert_eq!(entry.file_path, "path/to/file.ext");
        assert_eq!(entry.file_offset, 0);
        assert_eq!(entry.file_size, 0);
    }

    #[test]
    fn test_resource_entry_with_offset() {
        let entry = ResourceEntry::with_offset("packed.res", "archive.pak", 1024, 2048);
        assert_eq!(entry.name, "packed.res");
        assert_eq!(entry.file_path, "archive.pak");
        assert_eq!(entry.file_offset, 1024);
        assert_eq!(entry.file_size, 2048);
    }

    #[test]
    fn test_resource_index_parse_rmp() {
        let rmp_content = r#"
# This is a comment
resource.one = path/to/first.ext
resource.two = another/path.ext

# Another comment
resource.three = third/file.dat
"#;

        let index = ResourceIndex::from_str(rmp_content).expect("Should parse");
        assert_eq!(index.len(), 3);

        let entry1 = index
            .lookup("resource.one")
            .expect("Should find resource.one");
        assert_eq!(entry1.file_path, "path/to/first.ext");

        let entry2 = index
            .lookup("resource.two")
            .expect("Should find resource.two");
        assert_eq!(entry2.file_path, "another/path.ext");

        let entry3 = index
            .lookup("resource.three")
            .expect("Should find resource.three");
        assert_eq!(entry3.file_path, "third/file.dat");
    }

    #[test]
    fn test_resource_index_lookup_by_name() {
        let mut index = ResourceIndex::new();
        index.insert(ResourceEntry::new("my.resource", "my/file.txt"));
        index.insert(ResourceEntry::new("other.resource", "other/file.txt"));

        let entry = index.lookup("my.resource");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().file_path, "my/file.txt");

        let entry2 = index.get("other.resource");
        assert!(entry2.is_ok());
        assert_eq!(entry2.unwrap().file_path, "other/file.txt");
    }

    #[test]
    fn test_resource_index_lookup_nonexistent() {
        let index = ResourceIndex::new();

        assert!(index.lookup("nonexistent").is_none());
        assert!(!index.contains("nonexistent"));

        let result = index.get("nonexistent");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), IndexError::NotFound(_)));
    }

    #[test]
    fn test_resource_index_list_all() {
        let rmp_content = r#"
res.a = a.txt
res.b = b.txt
res.c = c.txt
"#;

        let index = ResourceIndex::from_str(rmp_content).expect("Should parse");

        let names: Vec<&str> = index.names().collect();
        assert_eq!(names.len(), 3);

        // Check all resources are present (order may vary due to HashMap)
        assert!(index.contains("res.a"));
        assert!(index.contains("res.b"));
        assert!(index.contains("res.c"));
    }

    #[test]
    fn test_resource_index_empty() {
        let index = ResourceIndex::new();
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);

        // Parsing empty content should work
        let index2 = ResourceIndex::from_str("").expect("Should parse empty");
        assert!(index2.is_empty());

        // Parsing only comments should work
        let index3 = ResourceIndex::from_str("# just a comment\n# another").expect("Should parse");
        assert!(index3.is_empty());
    }

    #[test]
    fn test_resource_index_case_sensitivity() {
        // Default is case-insensitive (matching UQM behavior)
        let mut index = ResourceIndex::new();
        index.insert(ResourceEntry::new("MyResource", "file.txt"));

        // Should find with different cases
        assert!(index.lookup("myresource").is_some());
        assert!(index.lookup("MYRESOURCE").is_some());
        assert!(index.lookup("MyResource").is_some());

        // Case-sensitive index
        let mut sensitive = ResourceIndex::with_case_sensitivity(true);
        sensitive.insert(ResourceEntry::new("MyResource", "file.txt"));

        assert!(sensitive.lookup("MyResource").is_some());
        assert!(sensitive.lookup("myresource").is_none());
        assert!(sensitive.lookup("MYRESOURCE").is_none());
    }

    #[test]
    fn test_resource_index_merge() {
        let mut index1 = ResourceIndex::new();
        index1.insert(ResourceEntry::new("res.a", "a1.txt"));
        index1.insert(ResourceEntry::new("res.b", "b.txt"));

        let mut index2 = ResourceIndex::new();
        index2.insert(ResourceEntry::new("res.a", "a2.txt")); // Override
        index2.insert(ResourceEntry::new("res.c", "c.txt")); // New

        index1.merge(index2);

        assert_eq!(index1.len(), 3);
        assert_eq!(index1.lookup("res.a").unwrap().file_path, "a2.txt"); // Overwritten
        assert_eq!(index1.lookup("res.b").unwrap().file_path, "b.txt");
        assert_eq!(index1.lookup("res.c").unwrap().file_path, "c.txt");
    }

    #[test]
    fn test_resource_index_parse_with_spaces() {
        let rmp_content = "  spaced.resource  =  path/with spaces.txt  ";
        let index = ResourceIndex::from_str(rmp_content).expect("Should parse");

        let entry = index.lookup("spaced.resource").expect("Should find");
        assert_eq!(entry.file_path, "path/with spaces.txt");
    }

    #[test]
    fn test_resource_index_iter() {
        let rmp_content = "a = 1\nb = 2\nc = 3";
        let index = ResourceIndex::from_str(rmp_content).expect("Should parse");

        let entries: Vec<_> = index.iter().collect();
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn test_index_error_display() {
        let err = IndexError::NotFound("test".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("not found"));
        assert!(msg.contains("test"));

        let err2 = IndexError::ParseError("line 5".to_string());
        let msg2 = format!("{}", err2);
        assert!(msg2.contains("Parse error"));
    }
}
