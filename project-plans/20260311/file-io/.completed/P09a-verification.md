# Phase P09a Verification - ZIP/UQM Archive Mount Support

## Verdict
REJECT

## Basis
The implementation includes substantial ZIP support, but it does not satisfy the phase requirements as written, and the requested verification command does not pass.

## What verifies correctly
- `rust/src/io/zip_reader.rs` exists and uses `zip::ZipArchive`, so mount-time indexing is based on ZIP central-directory parsing via the crate API.
- Path normalization is implemented in `normalize_zip_path()`:
  - converts backslashes to forward slashes
  - strips leading slashes
  - strips trailing slashes
- Synthetic directory generation is implemented via `synthesize_parent_dirs()`.
- Duplicate entry handling is implemented with `HashMap::insert()` during archive iteration, so last entry wins.
- Lookup is case-sensitive because normalized paths are stored and queried as exact `String` keys with no case folding.
- CRC validation exists in `ZipIndex::read_entry()`.
- `MountInfo` contains `zip_index: Option<Arc<ZipIndex>>`.
- ZIP mounts are activated in the registry (`active_in_registry = true`) and indexed during `register_mount()`.
- `uio_getDirList()` includes ZIP entries through `zip_index.list_directory()`.
- `uio_open()` supports ZIP-backed reads through `zip_index.open_entry()`.
- `rust/Cargo.toml` includes `zip = "2"` and `crc32fast = "1.4"`.

## Required items that fail verification
1. `uio_fopen` does not implement ZIP-backed opening.
   - The function resolves a host filesystem path and calls `OpenOptions::open()` directly.
   - It does not consult the mount registry for ZIP entries.
   - This fails the plan item requiring `uio_open / uio_fopen` support for archive-backed files.

2. CRC validation is not enforced on streaming ZIP reads used by UIO.
   - `uio_open()` uses `zip_index.open_entry()` which returns `ZipEntryReader`.
   - `ZipEntryReader::read()` streams data via `ZipArchive::by_index()` but does not compute or compare CRC.
   - CRC validation exists only in `ZipIndex::read_entry()`, which is not the path used by `uio_open()`.
   - This means the requested requirement "CRC validation on read" is not fully satisfied for the mounted-file read path.

3. `uio_access()` is incorrect for ZIP directories with `X_OK`.
   - ZIP hits return `(mount, mount.mounted_root.clone())` as a placeholder host path.
   - Later, `X_OK` checks `host_path.is_dir()`.
   - For a ZIP mount, `mounted_root` is the archive file, not the synthetic/archive directory being queried.
   - Therefore ZIP directory traversal semantics are not correctly implemented as specified.

4. `uio_stat()` does not initialize a full `stat` structure and uses simplified mode handling.
   - It does set file/dir mode bits and uncompressed size for ZIP entries.
   - However, verification is weakened because the implementation is partial and not clearly aligned with full stat semantics.

5. Requested test command failed.
   - Command run: `cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features 2>&1 | tail -5`
   - Result included:
     - `error: could not compile uqm (lib test) due to 14 previous errors`
   - Because the requested verification run does not succeed, the phase cannot be accepted.

## Evidence summary
- `zip_reader.rs`
  - central-directory indexing via `ZipArchive::new(...)`
  - duplicate overwrite via `entries.insert(...)`
  - synthetic directories via `synthesize_parent_dirs(...)`
  - exact-key lookups via `HashMap<String, ZipEntry>`
  - CRC only in `read_entry()`, not `ZipEntryReader`
- `uio_bridge.rs`
  - `MountInfo.zip_index` present
  - `uio_mountDir()` activates ZIP mounts
  - `register_mount()` indexes ZIP and fails with `EIO` on indexing error
  - `uio_getDirList()` includes ZIP entries
  - `uio_open()` includes ZIP entry open path
  - `uio_fopen()` lacks ZIP registry lookup
  - write attempts on ZIP through `uio_HandleInner::Write` fail, but `uio_fopen()` itself is not ZIP-aware

## Final assessment
Phase P09 is partially implemented but not complete enough to accept. The missing ZIP path in `uio_fopen`, lack of CRC validation in the streaming read path actually used by mounts, incorrect `uio_access(X_OK)` handling for ZIP directories, and failing requested library test command require a REJECT verdict.
