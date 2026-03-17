# Phase 08: Archive Support — Implementation

## Phase ID
`PLAN-20260314-UIO.P08`

## Prerequisites
- Required: Phase 07a completed
- Archive module structure in place
- Tests defined (some may be failing/ignored pending implementation)
- `zip` crate available

## Requirements Implemented (Expanded)

All REQ-UIO-ARCHIVE-* engine-critical requirements, plus the archive-related portions of REQ-UIO-CONC-001/003/004 and REQ-UIO-LIFE-004.

## Implementation Tasks

### Files to modify

#### `rust/src/io/uio/archive.rs` — Full implementation

- **`mount_archive`** — Parse ZIP central directory, build entry index
  - marker: `@plan PLAN-20260314-UIO.P08`
  - marker: `@requirement REQ-UIO-ARCHIVE-001`
  - Uses pseudocode Component 004
  - Open archive file with `std::fs::File::open`
  - Create `zip::ZipArchive::new(file)`
  - Iterate entries: `archive.by_index(i)`
  - For each entry: extract name, sizes, compression method
  - Normalize paths: strip leading `/`, collapse `.` / `..`
  - Strip `in_path` prefix if provided
  - Build `entries` HashMap keyed by normalized path
  - Synthesize parent directories into `directories` HashSet
  - Store `MountedArchive` in `ARCHIVE_REGISTRY`

- **`unmount_archive`** — Remove from registry
  - marker: `@plan PLAN-20260314-UIO.P08`
  - Find and remove entry matching `mount_id`
  - Do not invalidate already-open archive file handles in a way that can crash later operations

- **`lookup_archive_entry`** — Find entry by path
  - marker: `@plan PLAN-20260314-UIO.P08`
  - marker: `@requirement REQ-UIO-ARCHIVE-002`
  - Normalize lookup path
  - Search in `entries` HashMap

- **`is_archive_directory`** — Check directory existence
  - marker: `@plan PLAN-20260314-UIO.P08`
  - Check both `directories` HashSet and entries ending with `/`

- **`list_archive_directory`** — Enumerate direct children
  - marker: `@plan PLAN-20260314-UIO.P08`
  - marker: `@requirement REQ-UIO-ARCHIVE-002`
  - Build prefix from dir_path
  - Filter entries whose path starts with prefix
  - Keep only direct children (no additional `/` in remainder)
  - Include both files and synthesized subdirectories
  - Return Vec<String> of names

- **`open_archive_file`** — Decompress entry to memory
  - marker: `@plan PLAN-20260314-UIO.P08`
  - marker: `@requirement REQ-UIO-ARCHIVE-003`
  - Reopen archive file (or cache opened archives)
  - Find entry by name in `ZipArchive`
  - Read entry contents into `Vec<u8>` (handles both Stored and Deflate)
  - Return independently live archive-handle state rather than borrowing mutable global registry data

- **`archive_read`** — Read from decompressed buffer
  - marker: `@plan PLAN-20260314-UIO.P08`
  - Copy from `handle.data[position..]` to output buffer
  - Advance position
  - Return bytes read (0 at EOF)

- **`archive_seek`** — Seek within decompressed buffer
  - marker: `@plan PLAN-20260314-UIO.P08`
  - SEEK_SET: position = offset
  - SEEK_CUR: position += offset
  - SEEK_END: position = data.len() + offset
  - Validate bounds: 0 <= position <= data.len()
  - Return new position or 0/-1 consistent with the surrounding API

- **`archive_fstat_size`** — Report uncompressed size
  - marker: `@plan PLAN-20260314-UIO.P08`
  - Return `handle.data.len()` as u64

#### `rust/src/io/uio_bridge.rs` — Integration with archive module

- **Update `uio_mountDir` to call `mount_archive`**
  - marker: `@plan PLAN-20260314-UIO.P08`
  - marker: `@requirement REQ-UIO-ARCHIVE-001`
  - When `fsType == UIO_FSTYPE_ZIP`:
    - Resolve `sourceDir` + `sourcePath` to get the archive file path
    - Call `archive::mount_archive(mount_id, &archive_path, in_path_str)`
    - Set `active_in_registry = true` (remove the `!= UIO_FSTYPE_ZIP` check at line 1489)
    - If `mount_archive` fails: don't register mount, clean up partial state, set errno, return null

- **Update resolution paths to search archive entries**
  - marker: `@plan PLAN-20260314-UIO.P08`
  - marker: `@requirement REQ-UIO-ARCHIVE-002`
  - Update every concrete `rust/src/io/uio_bridge.rs` helper that resolves virtual paths or physical locations for public APIs, not just the direct open path
  - At minimum, audit and wire archive participation into the helper functions/sections used by:
    - `uio_open`
    - `uio_fopen`
    - `uio_stat`
    - `uio_access`
    - `uio_getDirList`
    - file-location / stdio-location queries used later by the compatibility-complete APIs
  - Mirror the exact helper list captured in the analysis and preflight phases, and update that list if refactor extraction renames the helpers
  - Ensure each helper checks ZIP mounts alongside STDIO mounts under the same ordering rule used in Phase 06
  - Also check `archive::is_archive_directory` for directory resolution

- **Update `uio_open` to handle archive-backed files**
  - marker: `@plan PLAN-20260314-UIO.P08`
  - marker: `@requirement REQ-UIO-ARCHIVE-003`
  - When resolved path is archive-backed:
    - Reject write flags (O_WRONLY, O_RDWR, O_CREAT, O_TRUNC) with errno = EROFS
    - Call `archive::open_archive_file(mount_id, entry_path)`
    - Store archive-handle state in a dispatchable handle representation

- **Refactor `uio_Handle` to support both STDIO and archive backing without ABI-visible breakage**
  - marker: `@plan PLAN-20260314-UIO.P08`
  - Introduce an internal handle-dispatch representation while preserving any required FFI-visible layout guarantees
  - Same-handle concurrent operations must remain synchronized (`REQ-UIO-CONC-003`)

- **Update `uio_read`, `uio_write`, `uio_lseek`, `uio_fstat`, `uio_close` to dispatch correctly**
  - marker: `@plan PLAN-20260314-UIO.P08`
  - `uio_write` rejects archive handles with `errno = EROFS`
  - `uio_close` drops archive-backed resources cleanly

- **Update `uio_fopen` and all archive-backed stream behavior, not just open**
  - marker: `@plan PLAN-20260314-UIO.P08`
  - When opening for read (`"r"`, `"rb"`), and path resolves to archive entry:
    - Open archive file handle
    - Wrap in stream with buffer
  - When opening for write/append: reject with errno = EROFS
  - Audit and verify correct behavior for:
    - `uio_fread`
    - `uio_fseek`
    - `uio_ftell`
    - `uio_fgetc`
    - `uio_fgets`
    - `uio_ungetc`
    - `uio_feof`
    - `uio_ferror`
    - `uio_clearerr`
  - The implementation plan must treat these as part of archive support, not as incidental reuse of STDIO-path behavior

- **Update `uio_unmountDir` to call `unmount_archive` and preserve safety floors for open objects**
  - marker: `@plan PLAN-20260314-UIO.P08`
  - When unmounting a ZIP mount: call `archive::unmount_archive(mount_id)`
  - Ensure already-open archive file handles/streams either continue via retained backing state or fail cleanly afterward, but do not crash or report fake success (`REQ-UIO-LIFE-004`, `REQ-UIO-CONC-004`)

- **Update `uio_stat` (path-based) to check archive entries**
  - marker: `@plan PLAN-20260314-UIO.P08`
  - When path resolves to archive entry: populate stat from entry info

- **Update `uio_access` to handle archive entries**
  - marker: `@plan PLAN-20260314-UIO.P08`
  - marker: `@requirement REQ-UIO-ARCHIVE-009`
  - F_OK: return 0 if entry exists
  - R_OK: return 0 (readable)
  - W_OK: return -1 (read-only)
  - X_OK: return -1 (not executable)

### Tests to add (integration tests)

- **`test_uio_mountdir_zip_active`**
  - Create test ZIP, call `uio_mountDir` with FSTYPE_ZIP
  - Assert mount is active (can resolve paths to archive entries)

- **`test_uio_open_archive_file_read`**
  - Mount ZIP, open known entry, read contents
  - Assert data matches original

- **`test_uio_open_archive_file_write_rejected`**
  - Mount ZIP, try to open entry for write
  - Assert returns null, errno == EROFS

- **`test_uio_fopen_archive_stream_read`**
  - Mount ZIP, fopen known entry in "rb" mode
  - Read via `uio_fread`, assert correct content

- **`test_uio_fseek_archive_stream`**
  - Mount ZIP, fopen entry, seek to known offset, read
  - Assert correct data at offset

- **`test_uio_ftell_archive_stream`**
  - Mount ZIP, fopen entry, read N bytes, call `ftell`
  - Assert position == N

- **`test_uio_fgetc_archive_stream`**
  - Mount ZIP, read bytewise from archive-backed stream
  - Assert byte sequence and EOF transition are correct

- **`test_uio_fgets_archive_stream`**
  - Mount ZIP, read line-based content from archive-backed stream
  - Assert newline and EOF semantics match stream contract

- **`test_uio_ungetc_archive_stream`**
  - Push back one character on archive-backed stream
  - Assert next `uio_fgetc` returns pushed-back byte

- **`test_uio_fstat_archive_entry`**
  - Mount ZIP, open entry, fstat
  - Assert `st_size` matches uncompressed size

- **`test_uio_stat_archive_entry`**
  - Mount ZIP, call `uio_stat` on archive path
  - Assert size correct

- **`test_uio_access_archive_entry`**
  - Mount ZIP, call `uio_access` with F_OK, R_OK, W_OK
  - Assert F_OK=0, R_OK=0, W_OK=-1

- **`test_uio_feof_after_archive_read_complete`**
  - Mount ZIP, fopen entry, read all, check `feof`
  - Assert `feof` returns non-zero after full read

- **`test_uio_ferror_after_archive_read`**
  - Mount ZIP, fopen entry, read partially
  - Assert `ferror` returns 0 (no error)

- **`test_archive_handle_survives_unmount_safely`**
  - Open archive-backed file handle, unmount backing mount, then read/seek/close
  - Assert behavior follows the documented safety floor: either still works via retained backing state or fails cleanly, but never crashes/UB

- **`test_archive_stream_survives_unmount_safely`**
  - Open archive-backed stream, unmount backing mount, then `fread`/`fseek`/`fclose`
  - Assert same no-crash/no-UB safety floor

- **`test_concurrent_independent_archive_reads`**
  - Open separate archive-backed handles/streams on multiple threads
  - Read concurrently
  - Assert correct data and no shared-state corruption (`REQ-UIO-CONC-001`)

- **`test_same_archive_handle_integrity_under_serialized_access`**
  - Exercise shared-handle synchronization expectations explicitly
  - Assert no data race / corrupted position state (`REQ-UIO-CONC-003`)

- **`test_archive_end_to_end_acceptance`**
  - marker: `@requirement REQ-UIO-ARCHIVE-ACCEPT`
  - Mount real-format `.uqm` archive
  - Enumerate contents via `uio_getDirList`
  - Open a known asset via `uio_fopen`
  - Read full contents via `uio_fread`
  - Verify seek/tell consistency
  - Confirm `uio_feof`/`uio_ferror` report correct status

### Pseudocode traceability
- Uses pseudocode Component 004

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `archive.rs` has full implementations (no stubs/todo!)
- [ ] archive-backed resolution is wired into all relevant resolution helpers
- [ ] `uio_mountDir` calls `mount_archive` for ZIP mounts
- [ ] `active_in_registry = false` for ZIP is removed
- [ ] `uio_open`/`uio_fopen` dispatch to archive handles
- [ ] `uio_read`/`uio_lseek`/`uio_fstat` dispatch correctly for archive handles
- [ ] `uio_write` rejects archive handles
- [ ] archive-backed stream operation audit covers `fread/fseek/ftell/fgetc/fgets/ungetc/feof/ferror/clearerr`
- [ ] live archive-backed objects have explicit post-unmount strategy
- [ ] 18+ new integration tests

## Semantic Verification Checklist
- [ ] All archive module tests pass
- [ ] All integration tests pass
- [ ] Stored entries read correctly
- [ ] Deflate entries decompress correctly
- [ ] Seek works on archive streams
- [ ] `fgetc`/`fgets`/`ungetc` work on archive streams
- [ ] `feof`/`ferror` work correctly on archive streams
- [ ] Write operations correctly rejected on archive content
- [ ] already-open archive handles/streams remain safe across unmount
- [ ] separate-handle concurrent archive reads are safe
- [ ] End-to-end acceptance test passes
- [ ] No placeholder/deferred patterns in archive code

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|todo!()\|unimplemented!()" rust/src/io/uio/archive.rs rust/src/io/uio_bridge.rs
```

## Success Criteria
- [ ] Archive entries can be mounted, discovered, opened, read, seeked, statted
- [ ] All archive-backed stream APIs behave correctly
- [ ] Live archive-backed handles/streams preserve the post-unmount safety floor
- [ ] Separate archive-backed handles can be used concurrently safely
- [ ] All tests pass
- [ ] End-to-end acceptance criterion met
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git stash`
- blocking issues: `zip` crate API incompatibility, handle-dispatch refactor breaks existing STDIO tests, post-unmount policy requires additional retained-state design

## Phase Completion Marker
Create: `project-plans/20260311/uio/.completed/P08.md`
