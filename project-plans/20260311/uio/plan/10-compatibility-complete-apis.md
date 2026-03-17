# Phase 10: Compatibility-Complete APIs — Stub/TDD/Impl

## Phase ID
`PLAN-20260314-UIO.P10`

## Prerequisites
- Required: Phase 09a completed
- All Tier 1 (engine-critical) gaps closed
- Archive support, stream state, mount ordering, directory merge all verified

## Requirements Implemented (Expanded)

### REQ-UIO-FB-001 through FB-007: FileBlock API
**Requirement text**: The FileBlock API provides block-oriented file access.

Behavior contract:
- GIVEN: an open file handle
- WHEN: `uio_openFileBlock(handle)` is called
- THEN: returns a FileBlock covering the entire file

- GIVEN: an open FileBlock
- WHEN: `uio_copyFileBlock(block, 0, buf, 100)` is called
- THEN: first 100 bytes of the file are copied to `buf`

- GIVEN: an open FileBlock
- WHEN: `uio_closeFileBlock(block)` is called
- THEN: block resources released, returns 0

### REQ-UIO-STDIO-001 through STDIO-006: StdioAccess API
**Requirement text**: The StdioAccess API provides real filesystem paths for content.

Behavior contract:
- GIVEN: a file backed by a STDIO mount
- WHEN: `uio_getStdioAccess(dir, path, 0)` is called
- THEN: returns handle with real filesystem path

- GIVEN: a file backed by a ZIP mount
- WHEN: `uio_getStdioAccess(dir, path, 0)` is called
- THEN: creates temp copy, returns handle with temp path

- GIVEN: a live StdioAccessHandle
- WHEN: `uio_releaseStdioAccess(handle)` is called
- THEN: temp files cleaned up

### REQ-UIO-MOUNT-008: Transplant directory
**Requirement text**: When a directory handle is transplanted, create a new mount exposing the same content at the new location.

Behavior contract:
- GIVEN: existing mount at `/addons/addon1/shadow-content`
- WHEN: `uio_transplantDir("/", shadowDir, uio_MOUNT_RDONLY | uio_MOUNT_ABOVE, contentMountHandle)` is called
- THEN: shadow content is visible at `/` with higher precedence than content mount
- AND: the transplanted mount has its own mount handle identity and lifecycle, even if it references shared backing content

### REQ-UIO-FILE-012: Access mode checks
**Requirement text**: `uio_access` evaluates R_OK/W_OK/X_OK, not just existence.

### REQ-UIO-FILE-013: Rename constraints
**Requirement text**: Both source and destination must satisfy mount/access constraints.

### REQ-UIO-FILE-015: mkdir
### REQ-UIO-FILE-016: rmdir

### REQ-UIO-ARCHIVE-010: File location for archive entries
**Requirement text**: `uio_getFileLocation` returns successful location info for archive entries.

### REQ-UIO-ARCHIVE-011: StdioAccess for archive entries via temp copy
### REQ-UIO-LIFE-003 / REQ-UIO-LIFE-005: Live-directory and shutdown-order safety floors
### REQ-UIO-ERR-003 / REQ-UIO-ERR-007 / REQ-UIO-ERR-012: partial-failure cleanup and clean unsupported behavior

## Implementation Tasks

### Files to create

#### `rust/src/io/uio/fileblock.rs`
- marker: `@plan PLAN-20260314-UIO.P10`
- marker: `@requirement REQ-UIO-FB-001 through FB-007`
- Implement `uio_FileBlock` struct with `handle`, `offset`, `size`, `data` fields
- Implement all FileBlock functions per pseudocode Component 006
- Uses `uio_Handle` for file access
- If a sub-surface remains intentionally deferred, it must fail immediately with the correct sentinel and `errno = ENOTSUP`, not return dummy success objects

#### `rust/src/io/uio/stdio_access.rs`
- marker: `@plan PLAN-20260314-UIO.P10`
- marker: `@requirement REQ-UIO-STDIO-001 through STDIO-006`
- Implement `uio_StdioAccessHandle` with `path: CString`, `temp_dir: Option<PathBuf>`
- Implement per pseudocode Component 006:
  - `get_stdio_access`: resolve mount type, return direct path or create temp copy
  - `release_stdio_access`: clean up temp resources
  - `get_path`: return CString pointer
- Partial setup failures must clean up all temp resources before returning failure

### Files to modify

#### `rust/src/io/uio/mod.rs`
- Add `pub mod fileblock;` and `pub mod stdio_access;`

#### `rust/src/io/uio_bridge.rs`

- **Replace FileBlock stubs with real implementations or explicit clean failures**
  - marker: `@plan PLAN-20260314-UIO.P10`
  - `uio_openFileBlock` → `fileblock::open_file_block`
  - `uio_openFileBlock2` → `fileblock::open_file_block_range`
  - `uio_closeFileBlock` → `fileblock::close_file_block`
  - `uio_accessFileBlock` → `fileblock::access_file_block`
  - `uio_copyFileBlock` → `fileblock::copy_file_block`
  - `uio_setFileBlockUsageHint` → `fileblock::set_usage_hint`
  - Any not-yet-supported case must fail with the documented sentinel and `errno = ENOTSUP`

- **Replace StdioAccess stubs with real implementations**
  - marker: `@plan PLAN-20260314-UIO.P10`
  - `uio_getStdioAccess` → `stdio_access::get_stdio_access`
  - `uio_releaseStdioAccess` → `stdio_access::release_stdio_access`
  - `uio_StdioAccessHandle_getPath` → `stdio_access::get_path`

- **Fix `uio_transplantDir` semantics**
  - marker: `@plan PLAN-20260314-UIO.P10`
  - marker: `@requirement REQ-UIO-MOUNT-008`
  - Extract `sourceDir`'s backing mount info / resolved view
  - Create a **new** mount record using the Phase 06 placement logic and the source directory's resolved backing content
  - For archive-backed sources: reference shared archive backing metadata, but do **not** reuse the original mount handle or original mount identity
  - Ensure unmounting the transplanted mount affects only that transplanted registration

- **Fix `uio_access` to check mode bits**
  - marker: `@plan PLAN-20260314-UIO.P10`
  - marker: `@requirement REQ-UIO-FILE-012`
  - Check mount read-only status for W_OK
  - For archive entries: R_OK succeeds, W_OK/X_OK fail
  - For STDIO entries: delegate to `libc::access` or equivalent permission check

- **Fix `uio_getFileLocation` for archive entries**
  - marker: `@plan PLAN-20260314-UIO.P10`
  - marker: `@requirement REQ-UIO-ARCHIVE-010`
  - When path resolves to archive entry: return owning mount information and a backing-location token/string sufficient to identify the archive content for the stdio bridge
  - This enables `uio_getStdioAccess` to determine backing type without public fake-failure behavior on archive-backed content

- **Fix `uio_ungetc` to provide real pushback**
  - marker: `@plan PLAN-20260314-UIO.P10`
  - marker: `@requirement REQ-UIO-STREAM-014`
  - Verify current implementation stores pushed-back character
  - Verify next `uio_fgetc` returns pushed-back character before reading from file

- **Audit live-object safety floors in compatibility-complete surfaces**
  - marker: `@plan PLAN-20260314-UIO.P10`
  - marker: `@requirement REQ-UIO-LIFE-003, REQ-UIO-LIFE-005`
  - Live directory handles surviving unmount must remain safe allocated objects until close
  - Shutdown-order misuse must preserve no-crash/no-UB behavior for directories, file blocks, stdio-access handles, and related cleanup paths

### Tests to add

#### FileBlock tests (in `fileblock.rs`)
- **`test_open_file_block_whole_file`** — open block, assert size matches file
- **`test_open_file_block_range`** — open block with offset+size, assert subrange
- **`test_copy_file_block`** — copy data from block, verify contents
- **`test_access_file_block`** — access returns valid length
- **`test_close_file_block`** — close without crash
- **`test_file_block_null_handle`** — null handle returns null block
- **`test_file_block_partial_failure_cleans_up`** — force allocation or read failure, assert no leaked partial state

#### StdioAccess tests (in `stdio_access.rs`)
- **`test_stdio_access_stdio_mount`** — get access, verify real path
- **`test_stdio_access_archive_mount_creates_temp`** — get access for archive file, verify temp path exists
- **`test_stdio_access_release_cleans_temp`** — release handle, verify temp removed
- **`test_stdio_access_path_valid_while_live`** — path pointer valid during handle lifetime
- **`test_stdio_access_null_input`** — null dir/path returns null
- **`test_stdio_access_partial_failure_cleans_temp`** — fail after temp creation, assert cleanup happened

#### Integration tests (in `uio_bridge.rs`)
- **`test_transplant_dir_basic`** — transplant a dir, verify content accessible at new location
- **`test_transplant_dir_above_relative`** — transplant above existing mount, verify precedence
- **`test_transplant_archive_creates_distinct_mount_identity`** — transplant archive-backed directory, verify new mount handle is distinct and independently unmountable
- **`test_access_mode_readonly_mount`** — W_OK on read-only mount returns -1
- **`test_access_mode_archive_entry`** — R_OK succeeds, W_OK fails on archive entry
- **`test_get_file_location_stdio`** — returns real path for STDIO file
- **`test_get_file_location_archive`** — returns location info for archive entry
- **`test_live_directory_handle_safe_after_unmount`** — open dir handle, unmount source, then operate/close safely
- **`test_shutdown_order_violation_stays_safe`** — intentionally violate close ordering and assert no crash / UB floor

### Pseudocode traceability
- Uses pseudocode Component 006 and Component 007

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `fileblock.rs` and `stdio_access.rs` created with full implementations
- [ ] All FileBlock stubs in `uio_bridge.rs` replaced with module calls or explicit clean failures
- [ ] All StdioAccess stubs replaced with module calls
- [ ] `uio_transplantDir` creates distinct mount identities and uses placement-aware insertion
- [ ] `uio_access` checks mode bits
- [ ] `uio_getFileLocation` handles archive entries
- [ ] partial-failure cleanup paths are implemented for temp-copy and fileblock setup
- [ ] 20+ new tests

## Semantic Verification Checklist
- [ ] FileBlock read/copy operations return correct data
- [ ] StdioAccess returns real paths for STDIO mounts
- [ ] StdioAccess creates and cleans up temp copies for archive mounts
- [ ] Partial setup failures clean up temp resources and return clean failure
- [ ] Transplant creates working mount at new location
- [ ] Archive-backed transplant produces distinct mount identity and independent unmount behavior
- [ ] Access mode checks respect mount read-only flag
- [ ] Live directory handles remain safe across unmount
- [ ] Shutdown-order misuse remains within the documented no-crash/no-UB floor
- [ ] All existing tests still pass
- [ ] No placeholder patterns remain

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|dummy\|stub" rust/src/io/uio/fileblock.rs rust/src/io/uio/stdio_access.rs rust/src/io/uio_bridge.rs
```

## Success Criteria
- [ ] All compatibility-complete APIs functional or explicitly clean-failing with `ENOTSUP`
- [ ] All tests pass
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git stash`
- blocking issues: FileBlock access to archive handles, temp directory creation failures, transplant shared-backing ownership design

## Phase Completion Marker
Create: `project-plans/20260311/uio/.completed/P10.md`
