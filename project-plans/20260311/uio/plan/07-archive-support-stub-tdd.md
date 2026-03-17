# Phase 07: Archive Support — Stub/TDD

## Phase ID
`PLAN-20260314-UIO.P07`

## Prerequisites
- Required: Phase 06a completed
- Mount ordering is correct
- errno setting is in place
- `zip` crate added to `rust/Cargo.toml`

## Requirements Implemented (Expanded)

### REQ-UIO-ARCHIVE-001: ZIP mounts fully active
**Requirement text**: When a caller mounts archive content as a ZIP/UQM filesystem type, the subsystem shall register the mount as fully active and expose archive entries in the virtual namespace.

Behavior contract:
- GIVEN: a valid `.uqm` archive file at a known path
- WHEN: `uio_mountDir(repo, "/content", UIO_FSTYPE_ZIP, sourceDir, "archive.uqm", "/", NULL, flags, NULL)` is called
- THEN: the mount is registered with `active_in_registry = true` AND archive entries are indexed

### REQ-UIO-ARCHIVE-002: Archive content in path resolution and enumeration
**Requirement text**: While an archive mount is active, archive content appears in path resolution and directory enumeration.

Behavior contract:
- GIVEN: archive mounted containing `data/font.fon`
- WHEN: `uio_getDirList(dir, "data", "", MATCH_LITERAL)` is called
- THEN: listing includes `font.fon`

### REQ-UIO-ARCHIVE-003: Read access to decompressed content
**Requirement text**: Opening a file that resolves to archive-backed content provides read access to decompressed contents.

Behavior contract:
- GIVEN: archive mounted containing `music/track01.mod` (Deflate-compressed)
- WHEN: `uio_fopen(dir, "music/track01.mod", "rb")` is called
- THEN: returns a valid stream with full read/seek/tell/fstat support over decompressed content

### REQ-UIO-ARCHIVE-004: Metadata for archive entries
**Requirement text**: File metadata for archive entries reports uncompressed size and file-type indicators.

Behavior contract:
- GIVEN: archive entry with uncompressed_size = 12345
- WHEN: `uio_fstat(handle, &stat_buf)` is called
- THEN: `stat_buf.st_size == 12345` AND `stat_buf.st_mode` has `S_IFREG` set

### REQ-UIO-ARCHIVE-005: Write operations rejected
**Requirement text**: Write attempts on archive-backed content fail.

Behavior contract:
- GIVEN: file handle to archive-backed content
- WHEN: `uio_write(handle, data, len)` is called
- THEN: returns -1 with `errno == EROFS`

### REQ-UIO-ARCHIVE-006: Case-insensitive archive discovery
**Requirement text**: Archive files discovered with case-insensitive `.zip` and `.uqm` matching.

### REQ-UIO-ARCHIVE-008: Transparent archive access
**Requirement text**: Resource consumers read package content through same APIs as non-archive content.

### REQ-UIO-ARCHIVE-ACCEPT: End-to-end archive acceptance
**Requirement text**: After mounting, consumers discover, open, read, seek, query archive assets through standard APIs.

## Implementation Tasks

### Files to create

#### `rust/src/io/uio/mod.rs`
- Module declaration for UIO submodules
- marker: `@plan PLAN-20260314-UIO.P07`

#### `rust/src/io/uio/archive.rs`
- marker: `@plan PLAN-20260314-UIO.P07`
- marker: `@requirement REQ-UIO-ARCHIVE-001`
- **Structs:**
  - `ArchiveEntryInfo` — name, compressed_size, uncompressed_size, compression_method, offset, is_directory
  - `MountedArchive` — mount_id, archive_path, entries HashMap, directories HashSet
  - `ArchiveFileHandle` — data Vec<u8>, position usize, mount_id, entry_path
- **Static:**
  - `ARCHIVE_REGISTRY: OnceLock<Mutex<Vec<MountedArchive>>>`
- **Functions (stubs initially):**
  - `pub fn mount_archive(mount_id: usize, archive_path: &Path, in_path: &str) -> Result<(), std::io::Error>` — parse ZIP, build index
  - `pub fn unmount_archive(mount_id: usize)` — remove from registry
  - `pub fn lookup_archive_entry(mount_id: usize, path: &str) -> Option<ArchiveEntryInfo>` — find entry
  - `pub fn is_archive_directory(mount_id: usize, path: &str) -> bool` — check if path is a directory
  - `pub fn list_archive_directory(mount_id: usize, dir_path: &str) -> Vec<String>` — list direct children
  - `pub fn open_archive_file(mount_id: usize, path: &str) -> Result<ArchiveFileHandle, std::io::Error>` — decompress and return handle
  - `pub fn archive_read(handle: &mut ArchiveFileHandle, buf: &mut [u8]) -> usize` — read from position
  - `pub fn archive_seek(handle: &mut ArchiveFileHandle, offset: i64, whence: c_int) -> Result<u64, std::io::Error>` — seek
  - `pub fn archive_fstat_size(handle: &ArchiveFileHandle) -> u64` — uncompressed size

### Files to modify

#### `rust/Cargo.toml`
- Add `zip = "2"` to `[dependencies]`
- marker: `@plan PLAN-20260314-UIO.P07`

#### `rust/src/io/mod.rs`
- Add `pub mod uio;` to module declarations
- marker: `@plan PLAN-20260314-UIO.P07`

### Tests to add (in `rust/src/io/uio/archive.rs` test module)

- **`test_mount_archive_creates_entry_index`**
  - Create a temporary ZIP file with known entries
  - Call `mount_archive(1, &zip_path, "/")`
  - Assert entries are indexed
  - marker: `@requirement REQ-UIO-ARCHIVE-001`

- **`test_lookup_archive_entry_found`**
  - Mount test archive
  - Look up a known entry path
  - Assert found with correct size

- **`test_lookup_archive_entry_not_found`**
  - Mount test archive
  - Look up nonexistent path
  - Assert None returned

- **`test_list_archive_directory_root`**
  - Mount test archive with entries in subdirectories
  - List root directory
  - Assert expected direct children returned

- **`test_list_archive_directory_subdirectory`**
  - List a subdirectory
  - Assert only direct children (not nested) returned

- **`test_is_archive_directory_true_for_synthesized_dir`**
  - Mount archive with `data/file.txt`
  - Assert `data` is recognized as a directory

- **`test_open_archive_file_stored`**
  - Mount archive with a Stored entry
  - Open the entry
  - Assert decompressed content matches original

- **`test_open_archive_file_deflated`**
  - Mount archive with a Deflate-compressed entry
  - Open the entry
  - Assert decompressed content matches original

- **`test_archive_read_returns_correct_bytes`**
  - Open an archive file
  - Read in chunks
  - Assert all data read correctly

- **`test_archive_read_at_eof_returns_zero`**
  - Open and read entire file
  - Read again
  - Assert 0 bytes returned

- **`test_archive_seek_set`**
  - Open file, seek to offset 10, read byte
  - Assert correct byte value

- **`test_archive_seek_cur`**
  - Open file, read 5 bytes, seek +5 from current, read
  - Assert at position 10

- **`test_archive_seek_end`**
  - Open file, seek to -1 from end
  - Assert at last byte position

- **`test_archive_fstat_reports_uncompressed_size`**
  - Open file from archive
  - Assert size matches original uncompressed size

- **`test_unmount_archive_removes_entries`**
  - Mount, unmount, lookup
  - Assert None after unmount

### Test helper: create_test_zip
- Utility function using `zip` crate to create a temporary ZIP file with controlled entries
- Parameters: list of (name, content, compression_method) tuples
- Returns: `tempfile::NamedTempFile` or `PathBuf`

### Pseudocode traceability
- Uses pseudocode lines: 125-205

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust/src/io/uio/mod.rs` exists
- [ ] `rust/src/io/uio/archive.rs` exists with all listed structs and function signatures
- [ ] `zip` dependency added to `Cargo.toml`
- [ ] Module wired through `rust/src/io/mod.rs`
- [ ] 15+ test functions added
- [ ] `create_test_zip` helper exists
- [ ] All plan/requirement markers present

## Semantic Verification Checklist
- [ ] Code compiles with `cargo check`
- [ ] Tests compile (stubs may cause some tests to fail — that's expected in TDD)
- [ ] Test coverage includes: mount/unmount, lookup, directory listing, file read, seek, fstat
- [ ] Edge cases covered: not found, EOF, empty archive

## Success Criteria
- [ ] All code compiles
- [ ] Test structure complete (some tests may fail pending impl in Phase 08)
- [ ] Verification commands pass (clippy, fmt)

## Failure Recovery
- rollback: `git stash`
- blocking issues: `zip` crate API changes, `tempfile` not available

## Phase Completion Marker
Create: `project-plans/20260311/uio/.completed/P07.md`
