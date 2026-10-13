/// @plan PLAN-20260314-FILE-IO.P09
/// @requirement REQ-FIO-ARCHIVE-MOUNT
/// @requirement REQ-FIO-ARCHIVE-EDGE
///
/// ZIP archive reader for mounting .uqm and .zip package files.
///
/// This module provides ZIP archive indexing and reading capabilities
/// for the UIO subsystem. Archives are indexed at mount time by reading
/// the central directory, and entries are made available through the
/// virtual filesystem namespace.
///
/// Path normalization:
/// - Leading "/" and "\" are stripped
/// - Backslashes are converted to forward slashes
/// - Trailing "/" on directory entries is stripped
/// - Duplicate entries: last central directory entry wins
/// - Lookup is case-sensitive
///
/// Synthetic directories:
/// - Implied parent directories are synthesized (e.g., "a/b/c.txt" creates "a" and "a/b")
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::FileExt;
use std::path::Path;
use std::sync::Arc;
use zip::ZipArchive;

use super::zip_stat::{CentralRecord, UnsupportedEntry, ZipEntryStat, ZipObject};

/// The bytes of an archive, owned by the index built over them. Every read of
/// the archive goes through the source the index was made from, never through
/// a path looked up again: a host archive keeps the descriptor it was opened
/// with, and an archive inside another archive keeps the decoded bytes of its
/// entry, so neither depends on what its parent mount or file does later.
#[derive(Clone)]
pub enum ArchiveSource {
    /// A host file, opened once when the archive was indexed.
    Host(Arc<File>),
    /// The decoded, checksum-verified bytes of an entry of another archive.
    Entry(Arc<[u8]>),
}

impl fmt::Debug for ArchiveSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchiveSource::Host(file) => f.debug_tuple("Host").field(file).finish(),
            ArchiveSource::Entry(bytes) => write!(f, "Entry({} bytes)", bytes.len()),
        }
    }
}

impl ArchiveSource {
    /// Opens the host archive at `path` and keeps it open.
    pub fn open_host(path: &Path) -> std::io::Result<Self> {
        File::open(path).map(|file| ArchiveSource::Host(Arc::new(file)))
    }

    /// A reader of its own over the source, positioned at the start.
    fn reader(&self) -> SourceReader {
        SourceReader {
            source: self.clone(),
            position: 0,
        }
    }

    fn len(&self) -> std::io::Result<u64> {
        match self {
            ArchiveSource::Host(file) => file.metadata().map(|meta| meta.len()),
            ArchiveSource::Entry(bytes) => Ok(bytes.len() as u64),
        }
    }

    /// Whether anything still holds the source, for tests that check which
    /// owner keeps an archive alive.
    #[cfg(test)]
    pub fn probe(&self) -> SourceProbe {
        match self {
            ArchiveSource::Host(file) => SourceProbe::Host(Arc::downgrade(file)),
            ArchiveSource::Entry(bytes) => SourceProbe::Entry(Arc::downgrade(bytes)),
        }
    }
}

/// A weak view of an [`ArchiveSource`] that does not keep it alive.
#[cfg(test)]
pub enum SourceProbe {
    Host(std::sync::Weak<File>),
    Entry(std::sync::Weak<[u8]>),
}

#[cfg(test)]
impl SourceProbe {
    pub fn is_alive(&self) -> bool {
        match self {
            SourceProbe::Host(file) => file.strong_count() > 0,
            SourceProbe::Entry(bytes) => bytes.strong_count() > 0,
        }
    }
}

/// A `Read + Seek` view of an [`ArchiveSource`] with its own position. Reads
/// are positional, so several readers share one descriptor without sharing
/// an offset.
struct SourceReader {
    source: ArchiveSource,
    position: u64,
}

impl Read for SourceReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = match &self.source {
            ArchiveSource::Host(file) => file.read_at(buf, self.position)?,
            ArchiveSource::Entry(bytes) => {
                let start = usize::try_from(self.position)
                    .unwrap_or(usize::MAX)
                    .min(bytes.len());
                let count = buf.len().min(bytes.len() - start);
                buf[..count].copy_from_slice(&bytes[start..start + count]);
                count
            }
        };
        self.position += read as u64;
        Ok(read)
    }
}

impl Seek for SourceReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let (base, offset) = match pos {
            SeekFrom::Start(offset) => {
                self.position = offset;
                return Ok(offset);
            }
            SeekFrom::Current(offset) => (self.position, offset),
            SeekFrom::End(offset) => (self.source.len()?, offset),
        };
        self.position = base.checked_add_signed(offset).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid seek to negative position",
            )
        })?;
        Ok(self.position)
    }
}

/// Metadata for a single entry in a ZIP archive
#[derive(Debug, Clone)]
pub struct ZipEntry {
    /// Normalized path (forward slashes, no leading/trailing slashes for files)
    pub path: String,
    /// Compressed size in bytes
    pub compressed_size: u64,
    /// Uncompressed size in bytes
    pub uncompressed_size: u64,
    /// Compression method (0 = stored, 8 = deflate, etc.)
    pub compression_method: u16,
    /// CRC32 checksum
    pub crc32: u32,
    /// Offset of local file header in archive
    pub local_header_offset: u64,
    /// True if this is a directory entry
    pub is_directory: bool,
    /// Index in the ZIP archive
    pub index: usize,
    /// What `stat` reports for the entry, from its central-directory record;
    /// the reason when the record describes nothing a file system can show.
    pub stat: Result<ZipEntryStat, UnsupportedEntry>,
}

/// Index of a ZIP archive, built at mount time
#[derive(Debug)]
pub struct ZipIndex {
    /// Map from normalized path to entry metadata
    pub entries: HashMap<String, ZipEntry>,
    /// Set of all directory paths (including synthetic)
    pub directories: HashSet<String>,
    /// Where the archive's bytes are read from, for the life of the index
    source: ArchiveSource,
}

impl ZipIndex {
    /// Create a new ZIP index by parsing the archive at the given path
    ///
    /// This reads the central directory and builds an in-memory index
    /// of all entries. Synthetic parent directories are created for
    /// any file paths that imply intermediate directories.
    ///
    /// Returns Err if the archive cannot be opened or parsed.
    pub fn new(archive_path: &Path) -> Result<Self, std::io::Error> {
        Self::from_source(ArchiveSource::open_host(archive_path)?)
    }

    /// Indexes the archive held in `bytes`, the decoded data of an entry of
    /// another archive.
    pub fn from_entry_bytes(bytes: Arc<[u8]>) -> Result<Self, std::io::Error> {
        Self::from_source(ArchiveSource::Entry(bytes))
    }

    /// Indexes the archive `source` holds, the same way for every source.
    pub fn from_source(source: ArchiveSource) -> Result<Self, std::io::Error> {
        let mut archive = ZipArchive::new(source.reader()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to parse ZIP archive: {}", e),
            )
        })?;

        // The `zip` crate does not expose the host that made an entry or its
        // raw attributes, so the central headers are read from the source the
        // archive reads, through a reader of their own.
        let mut central = source.reader();
        let mut entries = HashMap::new();
        let mut directories = HashSet::new();

        // Process all entries in the archive (last entry wins for duplicates)
        for i in 0..archive.len() {
            let entry = archive.by_index(i).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read ZIP entry {}: {}", i, e),
                )
            })?;

            let raw_name = entry.name();
            let normalized = normalize_zip_path(raw_name);

            // Skip empty paths
            if normalized.is_empty() {
                continue;
            }

            let is_dir = entry.is_dir();

            // Create entry metadata
            let compression_method = match entry.compression() {
                zip::CompressionMethod::Stored => 0,
                zip::CompressionMethod::Deflated => 8,
                _ => 99, // Unknown/unsupported methods
            };

            let record = CentralRecord::read(&mut central, entry.central_header_start())?;
            let stat = record.stat(is_dir, entry.size(), entry.last_modified());

            let zip_entry = ZipEntry {
                path: normalized.clone(),
                compressed_size: entry.compressed_size(),
                uncompressed_size: entry.size(),
                compression_method,
                crc32: entry.crc32(),
                local_header_offset: entry.data_start(),
                is_directory: is_dir,
                index: i,
                stat,
            };

            // Last entry wins for duplicates
            entries.insert(normalized.clone(), zip_entry);

            // Synthesize parent directories
            if !is_dir {
                synthesize_parent_dirs(&normalized, &mut directories);
            } else {
                // Add the directory itself
                directories.insert(normalized.clone());
                // Also synthesize its parents
                synthesize_parent_dirs(&normalized, &mut directories);
            }
        }

        Ok(ZipIndex {
            entries,
            directories,
            source,
        })
    }

    /// A weak view of the archive's source, for tests of who keeps it alive.
    #[cfg(test)]
    pub fn source_probe(&self) -> SourceProbe {
        self.source.probe()
    }

    /// Check if a path exists in the archive (file or directory)
    pub fn contains(&self, path: &str) -> bool {
        let normalized = normalize_zip_path(path);
        self.entries.contains_key(&normalized) || self.directories.contains(&normalized)
    }

    /// Get entry metadata for a file path
    pub fn get_entry(&self, path: &str) -> Option<&ZipEntry> {
        let normalized = normalize_zip_path(path);
        self.entries.get(&normalized)
    }

    /// Check if a path is a directory
    pub fn is_directory(&self, path: &str) -> bool {
        let normalized = normalize_zip_path(path);
        self.directories.contains(&normalized)
    }

    /// What `stat` reports for the object at `path`; `None` when the archive
    /// holds nothing there. A directory wins over a file of the same name, as
    /// it does for every lookup of the archive.
    pub fn stat(&self, path: &str) -> Option<Result<ZipObject, UnsupportedEntry>> {
        let normalized = normalize_zip_path(path);
        if normalized.is_empty() {
            return Some(Ok(ZipObject::ImpliedDirectory));
        }
        let entry = self.entries.get(&normalized);
        if self.directories.contains(&normalized) {
            return Some(match entry {
                Some(entry) if entry.is_directory => entry.stat.map(ZipObject::Recorded),
                _ => Ok(ZipObject::ImpliedDirectory),
            });
        }
        entry.map(|entry| entry.stat.map(ZipObject::Recorded))
    }

    /// List all entries in a directory (non-recursive)
    pub fn list_directory(&self, dir_path: &str) -> Vec<String> {
        let normalized_dir = if dir_path.is_empty() {
            String::new()
        } else {
            let mut n = normalize_zip_path(dir_path);
            if !n.is_empty() && !n.ends_with('/') {
                n.push('/');
            }
            n
        };

        let mut results = HashSet::new();

        // Find all entries that are direct children of this directory
        for entry_path in self.entries.keys() {
            if let Some(child_name) = get_direct_child(&normalized_dir, entry_path) {
                results.insert(child_name);
            }
        }

        // Also check synthetic directories
        for dir_path in &self.directories {
            if let Some(child_name) = get_direct_child(&normalized_dir, dir_path) {
                results.insert(child_name);
            }
        }

        results.into_iter().collect()
    }

    /// Read an entry from the archive into a buffer
    pub fn read_entry(&self, path: &str) -> Result<Vec<u8>, std::io::Error> {
        let normalized = normalize_zip_path(path);
        let entry = self.entries.get(&normalized).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Entry not found in archive")
        })?;

        if entry.is_directory {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Cannot read a directory entry",
            ));
        }

        let mut archive = ZipArchive::new(self.source.reader()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to open ZIP archive: {}", e),
            )
        })?;

        let mut zip_file = archive.by_index(entry.index).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to read ZIP entry: {}", e),
            )
        })?;

        let mut buffer = Vec::with_capacity(entry.uncompressed_size as usize);
        zip_file.read_to_end(&mut buffer)?;

        // Validate CRC32
        let actual_crc = crc32fast::hash(&buffer);
        if actual_crc != entry.crc32 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "CRC32 mismatch for '{}': expected {:08x}, got {:08x}",
                    path, entry.crc32, actual_crc
                ),
            ));
        }

        Ok(buffer)
    }

    /// Open an entry for streaming reads
    pub fn open_entry(&self, path: &str) -> Result<ZipEntryReader, std::io::Error> {
        let normalized = normalize_zip_path(path);
        let entry = self.entries.get(&normalized).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Entry not found in archive")
        })?;

        if entry.is_directory {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Cannot open a directory entry for reading",
            ));
        }

        ZipEntryReader::new(&self.source, entry.index, entry.stat)
    }
}

/// Streaming reader for a ZIP entry.
/// Eagerly decompresses and CRC-validates the full entry on construction
/// so that every subsequent read is served from a validated in-memory buffer.
/// @requirement REQ-FIO-ARCHIVE-MOUNT (CRC validation on read path)
pub struct ZipEntryReader {
    data: Vec<u8>,
    position: u64,
    /// The entry's central-directory `stat`, captured when the handle was
    /// opened so that `fstat` answers from the handle and never from the
    /// archive path, which may have left the mount stack or the host by then.
    stat: Result<ZipEntryStat, UnsupportedEntry>,
}

impl ZipEntryReader {
    fn new(
        source: &ArchiveSource,
        index: usize,
        stat: Result<ZipEntryStat, UnsupportedEntry>,
    ) -> Result<Self, std::io::Error> {
        let mut archive = ZipArchive::new(source.reader()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to open ZIP archive: {}", e),
            )
        })?;

        let expected_crc = archive
            .by_index_raw(index)
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to read ZIP entry metadata: {}", e),
                )
            })?
            .crc32();

        let mut zip_file = archive.by_index(index).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to read ZIP entry: {}", e),
            )
        })?;

        let mut data = Vec::new();
        zip_file.read_to_end(&mut data)?;

        let actual_crc = crc32fast::hash(&data);
        if actual_crc != expected_crc {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "CRC mismatch for ZIP entry: expected 0x{:08x}, got 0x{:08x}",
                    expected_crc, actual_crc
                ),
            ));
        }

        Ok(ZipEntryReader {
            data,
            position: 0,
            stat,
        })
    }

    pub fn size(&self) -> u64 {
        self.data.len() as u64
    }

    /// The length, mode and time `fstat` reports: the entry's record, with
    /// the length of the decoded bytes the handle serves, so it agrees with
    /// every read and with a file block over the handle. An entry whose
    /// record describes no regular file has no `stat` and is refused as
    /// unsupported.
    pub fn stat(&self) -> Result<ZipEntryStat, std::io::Error> {
        let recorded = self.stat?;
        Ok(ZipEntryStat {
            size: self.size(),
            ..recorded
        })
    }

    pub fn position(&self) -> u64 {
        self.position
    }
}

impl Read for ZipEntryReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let remaining = self.data.len() as u64 - self.position;
        if remaining == 0 {
            return Ok(0);
        }
        let to_copy = std::cmp::min(buf.len() as u64, remaining) as usize;
        let start = self.position as usize;
        buf[..to_copy].copy_from_slice(&self.data[start..start + to_copy]);
        self.position += to_copy as u64;
        Ok(to_copy)
    }
}

impl Seek for ZipEntryReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::Current(offset) => self.position as i64 + offset,
            SeekFrom::End(offset) => self.data.len() as i64 + offset,
        };

        if new_pos < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid seek to negative position",
            ));
        }

        self.position = new_pos as u64;
        Ok(self.position)
    }
}

/// Normalize a ZIP entry path according to UIO rules:
/// - Strip leading "/" and "\"
/// - Convert backslashes to forward slashes
/// - Strip trailing "/" for files (but preserve for detection)
fn normalize_zip_path(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }

    // Convert backslashes to forward slashes
    let mut normalized = path.replace('\\', "/");

    // Strip leading slashes
    while normalized.starts_with('/') {
        normalized = normalized[1..].to_string();
    }

    // Strip trailing slashes (will be re-added for directories if needed)
    while normalized.ends_with('/') {
        normalized = normalized[..normalized.len() - 1].to_string();
    }

    normalized
}

/// Synthesize all parent directories for a given path
///
/// For example, "a/b/c.txt" will add "a" and "a/b" to the directories set
fn synthesize_parent_dirs(path: &str, directories: &mut HashSet<String>) {
    let parts: Vec<&str> = path.split('/').collect();

    for i in 0..parts.len() - 1 {
        let parent = parts[0..=i].join("/");
        if !parent.is_empty() {
            directories.insert(parent);
        }
    }
}

/// Get the direct child name if `entry_path` is a direct child of `dir_path`
///
/// Returns None if `entry_path` is not a direct child.
/// Returns Some(child_name) with just the name component (not full path).
fn get_direct_child(dir_path: &str, entry_path: &str) -> Option<String> {
    // For root directory
    if dir_path.is_empty() {
        if let Some(slash_pos) = entry_path.find('/') {
            return Some(entry_path[..slash_pos].to_string());
        } else {
            return Some(entry_path.to_string());
        }
    }

    // For non-root directories
    if !entry_path.starts_with(dir_path) {
        return None;
    }

    let remainder = &entry_path[dir_path.len()..];
    if let Some(slash_pos) = remainder.find('/') {
        Some(remainder[..slash_pos].to_string())
    } else {
        Some(remainder.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_normalize_zip_path() {
        assert_eq!(normalize_zip_path(""), "");
        assert_eq!(normalize_zip_path("file.txt"), "file.txt");
        assert_eq!(normalize_zip_path("/file.txt"), "file.txt");
        assert_eq!(normalize_zip_path("\\file.txt"), "file.txt");
        assert_eq!(normalize_zip_path("dir/file.txt"), "dir/file.txt");
        assert_eq!(normalize_zip_path("dir\\file.txt"), "dir/file.txt");
        assert_eq!(normalize_zip_path("/dir/file.txt"), "dir/file.txt");
        assert_eq!(
            normalize_zip_path("dir/subdir/file.txt"),
            "dir/subdir/file.txt"
        );
        assert_eq!(normalize_zip_path("dir/"), "dir");
        assert_eq!(normalize_zip_path("/dir/"), "dir");
    }

    #[test]
    fn test_synthesize_parent_dirs() {
        let mut dirs = HashSet::new();
        synthesize_parent_dirs("a/b/c.txt", &mut dirs);

        assert!(dirs.contains("a"));
        assert!(dirs.contains("a/b"));
        assert!(!dirs.contains("a/b/c.txt")); // The file itself is not a directory
        assert_eq!(dirs.len(), 2);
    }

    #[test]
    fn test_get_direct_child() {
        // Root directory
        assert_eq!(
            get_direct_child("", "file.txt"),
            Some("file.txt".to_string())
        );
        assert_eq!(
            get_direct_child("", "dir/file.txt"),
            Some("dir".to_string())
        );

        // Non-root directory
        assert_eq!(
            get_direct_child("dir/", "dir/file.txt"),
            Some("file.txt".to_string())
        );
        assert_eq!(
            get_direct_child("dir/", "dir/subdir/file.txt"),
            Some("subdir".to_string())
        );
        assert_eq!(get_direct_child("dir/", "other/file.txt"), None);
        assert_eq!(get_direct_child("a/", "a/b/c.txt"), Some("b".to_string()));
    }

    #[test]
    fn test_zip_index_with_test_archive() -> Result<(), std::io::Error> {
        // Create a temporary directory and ZIP file
        let temp_dir = TempDir::new()?;
        let zip_path = temp_dir.path().join("test.zip");

        // Create a simple ZIP archive
        let file = File::create(&zip_path)?;
        let mut zip = zip::ZipWriter::new(file);

        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        // Add a file
        zip.start_file("readme.txt", options)?;
        zip.write_all(b"Hello, World!")?;

        // Add a file in a subdirectory
        zip.start_file("data/config.ini", options)?;
        zip.write_all(b"[settings]\nvalue=1")?;

        // Add an explicit directory entry
        zip.add_directory("docs/", options)?;

        zip.finish()?;

        // Now test the index
        let index = ZipIndex::new(&zip_path)?;

        // Check entries
        assert!(index.contains("readme.txt"));
        assert!(index.contains("data/config.ini"));

        // Check synthetic directories
        assert!(index.is_directory("data"));
        assert!(index.is_directory("docs"));

        // Check listing
        let root_entries = index.list_directory("");
        assert!(root_entries.contains(&"readme.txt".to_string()));
        assert!(root_entries.contains(&"data".to_string()));
        assert!(root_entries.contains(&"docs".to_string()));

        let data_entries = index.list_directory("data");
        assert!(data_entries.contains(&"config.ini".to_string()));

        // Check reading
        let content = index.read_entry("readme.txt")?;
        assert_eq!(content, b"Hello, World!");

        Ok(())
    }

    #[test]
    #[ignore] // zip v2 doesn't allow duplicate filenames
    fn test_zip_index_duplicate_handling() -> Result<(), std::io::Error> {
        // This test verifies that the last entry wins when there are duplicates
        let temp_dir = TempDir::new()?;
        let zip_path = temp_dir.path().join("duplicates.zip");

        let file = File::create(&zip_path)?;
        let mut zip = zip::ZipWriter::new(file);

        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        // Add first version
        zip.start_file("file.txt", options)?;
        zip.write_all(b"First version")?;

        // Add second version (should win)
        zip.start_file("file.txt", options)?;
        zip.write_all(b"Second version")?;

        zip.finish()?;

        let index = ZipIndex::new(&zip_path)?;
        let content = index.read_entry("file.txt")?;

        // The last entry should win
        assert_eq!(content, b"Second version");

        Ok(())
    }

    #[test]
    fn test_zip_entry_reader() -> Result<(), std::io::Error> {
        let temp_dir = TempDir::new()?;
        let zip_path = temp_dir.path().join("reader_test.zip");

        let file = File::create(&zip_path)?;
        let mut zip = zip::ZipWriter::new(file);

        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        zip.start_file("test.txt", options)?;
        zip.write_all(b"0123456789")?;
        zip.finish()?;

        let index = ZipIndex::new(&zip_path)?;
        let mut reader = index.open_entry("test.txt")?;

        // Test reading
        let mut buf = [0u8; 5];
        let n = reader.read(&mut buf)?;
        assert_eq!(n, 5);
        assert_eq!(&buf, b"01234");

        // Test seeking
        reader.seek(SeekFrom::Start(7))?;
        let n = reader.read(&mut buf)?;
        assert_eq!(n, 3);
        assert_eq!(&buf[..3], b"789");

        Ok(())
    }

    fn stored_archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (name, data) in entries {
            zip.start_file(*name, options).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn an_index_over_entry_bytes_matches_the_index_over_the_same_host_file(
    ) -> Result<(), std::io::Error> {
        let bytes = stored_archive(&[("a/b.txt", b"bee"), ("c.txt", b"sea")]);
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().join("same.zip");
        std::fs::write(&path, &bytes)?;

        let host = ZipIndex::new(&path)?;
        let memory = ZipIndex::from_entry_bytes(Arc::from(bytes))?;
        let mut host_names: Vec<_> = host.entries.keys().cloned().collect();
        let mut memory_names: Vec<_> = memory.entries.keys().cloned().collect();
        host_names.sort();
        memory_names.sort();
        assert_eq!(host_names, memory_names);
        assert!(memory.is_directory("a"));
        assert_eq!(memory.read_entry("a/b.txt")?, b"bee");
        assert_eq!(
            memory.get_entry("c.txt").unwrap().stat,
            host.get_entry("c.txt").unwrap().stat
        );
        Ok(())
    }

    #[test]
    fn bytes_that_are_not_an_archive_are_invalid_data_not_a_host_guess() {
        let error = ZipIndex::from_entry_bytes(Arc::from(&b"plain text"[..])).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(error.raw_os_error(), None);
    }

    #[test]
    fn a_source_reader_seeks_like_a_file_and_refuses_a_negative_position() {
        let source = ArchiveSource::Entry(Arc::from(&b"0123456789"[..]));
        let mut reader = source.reader();
        assert_eq!(reader.seek(SeekFrom::End(-3)).unwrap(), 7);
        let mut buf = [0u8; 8];
        assert_eq!(reader.read(&mut buf).unwrap(), 3);
        assert_eq!(&buf[..3], b"789");
        assert_eq!(reader.read(&mut buf).unwrap(), 0);
        assert_eq!(reader.seek(SeekFrom::Current(-10)).unwrap(), 0);
        assert!(reader.seek(SeekFrom::Current(-1)).is_err());
        assert_eq!(reader.seek(SeekFrom::Start(99)).unwrap(), 99);
        assert_eq!(reader.read(&mut buf).unwrap(), 0);
    }

    #[test]
    fn a_host_source_reader_keeps_its_own_position_over_a_shared_descriptor(
    ) -> Result<(), std::io::Error> {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().join("shared.bin");
        std::fs::write(&path, b"abcdefghij")?;
        let source = ArchiveSource::open_host(&path)?;
        let (mut first, mut second) = (source.reader(), source.reader());
        let mut buf = [0u8; 4];
        first.read_exact(&mut buf)?;
        assert_eq!(&buf, b"abcd");
        second.seek(SeekFrom::End(-2))?;
        second.read_exact(&mut buf[..2])?;
        assert_eq!(&buf[..2], b"ij");
        first.read_exact(&mut buf)?;
        assert_eq!(&buf, b"efgh");
        Ok(())
    }
}
