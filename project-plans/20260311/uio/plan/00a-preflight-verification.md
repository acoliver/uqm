# Phase 00a: Preflight Verification

## Phase ID
`PLAN-20260314-UIO.P00a`

## Purpose
Verify assumptions before implementation.

## Toolchain Verification
- [ ] `cargo --version` — must be 1.75+ for stable features used
- [ ] `rustc --version` — must be 1.75+ for stable `catch_unwind` and FFI patterns
- [ ] `cargo clippy --version` — must be available
- [ ] `cargo llvm-cov --version` — verify availability if coverage gate required

## Dependency Verification

### Existing crate dependencies confirmed in `rust/Cargo.toml`
- [ ] `libc = "0.2"` — already present, used for C types
- [ ] `serial_test = "3.0"` — already present in dev-dependencies

### New dependencies required
- [ ] `zip = "2.x"` or `rc-zip` — for ZIP/UQM archive reading (REQ-UIO-ARCHIVE-001 through ARCHIVE-008). Verify which ZIP crate is best suited: `zip` (mature, Deflate support) vs `rc-zip` (streaming). **Decision criterion:** must support Stored and Deflate entries, random-access seeking within decompressed content, and directory traversal.
- [ ] `regex = "1.x"` — for general POSIX ERE-compatible regex matching in `uio_getDirList` (REQ-UIO-LIST-009). Verify `regex` crate supports the patterns used by C startup code: `\.[rR][mM][pP]$` and `\.([zZ][iI][pP]|[uU][qQ][mM])$`.

### Dependency compatibility check
- [ ] Verify `zip` crate does not conflict with existing dependencies
- [ ] Verify `regex` crate does not conflict with existing dependencies
- [ ] Run `cargo check --workspace` after adding dependencies to confirm

## Type/Interface Verification

### C-ABI struct layouts
- [ ] `uio_Stream` struct field order matches `uiostream.h` — verify `buf`, `dataStart`, `dataEnd`, `bufEnd`, `handle`, `status`, `operation`, `openFlags` field names and types
  - Current Rust: `buf`, `data_start`, `data_end`, `buf_end`, `handle`, `status`, `operation`, `open_flags` at `rust/src/io/uio_bridge.rs:270-279`
  - C header: `sc2/src/libs/uio/uiostream.h:28-59`
- [ ] `uio_DirList` struct matches C definition exactly — verify only `names`, `numNames` are ABI-visible
  - Current Rust: `rust/src/io/uio_bridge.rs:254-259` — note extra `buffer` field not in C struct
  - C header: verify actual C `uio_DirList` definition
  - Decision required before Phase 09: either change the Rust public struct to match the C header exactly or introduce a private allocation wrapper that keeps bookkeeping out of the public `repr(C)` layout
- [ ] `uio_DirHandle` struct — verify `repr(C)` layout compatibility
- [ ] `uio_MountHandle` struct — verify `repr(C)` layout compatibility
- [ ] Audit all other FFI-visible structs consumed by C/Rust callers for `REQ-UIO-FFI-004` / `REQ-UIO-INT-004`

### Existing export symbols and FFI behavior
- [ ] Verify all `#[no_mangle] extern "C"` functions in `uio_bridge.rs` compile and link
- [ ] Verify `rust_uio_fread` is the symbol name used by the C shim at `sc2/src/libs/uio/uio_fread_shim.c:6`
- [ ] Verify `uio_fread_shim.c` references `rust_uio_fread` (not `uio_fread`)
- [ ] Inventory every exported `uio_*` symbol and record its success sentinel, failure sentinel, null-input handling, and current stub/real status for later REQ-UIO-ERR-007/012 audit
- [ ] Verify existing `catch_unwind` usage actually wraps all exported entry points that can panic, or document missing coverage for REQ-UIO-ERR-008 / REQ-UIO-FFI-003

### Mount registry and concurrency baseline
- [ ] `MountInfo` struct at `rust/src/io/uio_bridge.rs:38-46` — verify fields
- [ ] `MOUNT_REGISTRY` global at `rust/src/io/uio_bridge.rs:48` — verify `OnceLock<Mutex<Vec<MountInfo>>>`
- [ ] `sort_mount_registry` at `rust/src/io/uio_bridge.rs:347-354` — verify current sort criteria (active, path-length, id)
- [ ] Determine whether current reader paths and mutation paths take the same lock and therefore satisfy the baseline “no torn mount-state update” requirement of REQ-UIO-CONC-002
- [ ] Identify places where future archive lookup, transplant, or stdio temp-copy work could hold global mount locks across blocking file I/O, for REQ-UIO-CONC-005

## Call-Path Feasibility

### Stream state fix call paths
- [ ] `uio_feof` at line 838 — verify it's called from `sdluio.c` (via `uio_ferror` check pattern)
- [ ] `uio_ferror` at line 844 — verify `sdluio.c:92-100` calls it after zero-byte reads
- [ ] `uio_clearerr` at line 887 — verify no engine-critical caller (compatibility-complete only)
- [ ] `uio_fseek` at line 1926 — verify it should clear EOF status (specification §5.3)
- [ ] `uio_fclose` at line 1818 — verify buffer leak at line 1821-1826

### Archive mount call paths
- [ ] `uio_mountDir` at line 1451 — verify `active_in_registry = _fsType != UIO_FSTYPE_ZIP` at line 1489
- [ ] C startup calls `uio_mountDir` with `uio_FSTYPE_ZIP` — verify at `options.c:477-480`
- [ ] `resolve_virtual_mount_path` at line 421 — verify it filters `active_in_registry`
- [ ] `uio_getDirList` at line 1979 — verify it only reads single STDIO directory
- [ ] Enumerate all stream functions that must work correctly over archive-backed handles: `uio_fopen`, `uio_fclose`, `uio_fread`, `uio_fseek`, `uio_ftell`, `uio_fgetc`, `uio_fgets`, `uio_ungetc`, `uio_feof`, `uio_ferror`, `uio_clearerr`

### Lifecycle and shutdown feasibility
- [ ] Verify current `uio_closeRepository` / `uio_unmountDir` behavior against REQ-UIO-LIFE-001 and REQ-UIO-LIFE-002
- [ ] Identify current behavior when a live directory handle, file handle, or stream survives unmount, for REQ-UIO-LIFE-003 through REQ-UIO-LIFE-005

### errno integration points
- [ ] Verify `libc::__error()` or equivalent is available for setting `errno` on macOS
- [ ] Alternative: use `std::io::Error::raw_os_error()` pattern

## Test Infrastructure Verification
- [ ] Existing tests at `uio_bridge.rs:2218-2483` run with `cargo test`
- [ ] `serial_test` attribute works for mount registry tests
- [ ] `tempfile` crate available in dev-dependencies for filesystem tests
- [ ] Confirm `cargo test --workspace` passes currently
- [ ] Decide how to express concurrency/lifecycle verification tests without introducing flakiness (e.g. barriers, serial registry reset, deterministic temp fixtures)

## Boundary and integration verification
- [ ] Confirm startup policy remains in `options.c`, not in Rust UIO (REQ-UIO-BOUND-001/002)
- [ ] Confirm SDL adapter remains a pure consumer of public UIO contracts (REQ-UIO-BOUND-003)
- [ ] Confirm engine globals (`contentDir`, `configDir`, `saveDir`, `meleeDir`, `contentMountHandle`) remain externally owned integration points (REQ-UIO-INT-002)

## Blocking Issues
[List any blockers discovered during verification. If non-empty, stop and revise plan first.]

## Gate Decision
- [ ] PASS: proceed to Phase 01
- [ ] FAIL: revise plan — document what needs to change
