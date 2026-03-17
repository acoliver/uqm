# Phase 01: Analysis

## Phase ID
`PLAN-20260314-UIO.P01`

## Prerequisites
- Required: Phase 00a (preflight) completed
- All toolchain and dependency checks pass
- Type/interface verification confirms layout assumptions

## Purpose
Produce a complete gap analysis mapping every requirement to current implementation state, and document all integration touchpoints, edge cases, error handling paths, concurrency/lifecycle safety floors, and FFI/ABI obligations.

## Gap Analysis: Requirements vs Current Code

### Tier 1 — Engine-Critical Gaps

#### GAP-01: ZIP mounts inactive in registry
- **Requirements:** REQ-UIO-ARCHIVE-001, REQ-UIO-ARCHIVE-002, REQ-UIO-ARCHIVE-ACCEPT
- **Current code:** `rust/src/io/uio_bridge.rs:1489` — `active_in_registry = _fsType != UIO_FSTYPE_ZIP`
- **Consumer:** `sc2/src/options.c:477-480` calls `uio_mountDir` with `uio_FSTYPE_ZIP`
- **Impact:** All archive-backed content unreachable. Highest-risk gap.
- **Fix:** Parse ZIP central directory on mount, build entry index, set `active_in_registry = true`

#### GAP-02: No archive entry index or decompression
- **Requirements:** REQ-UIO-ARCHIVE-003, REQ-UIO-ARCHIVE-004, REQ-UIO-ARCHIVE-008
- **Current code:** No ZIP parsing code exists anywhere in Rust crate
- **Consumer:** `sdluio.c`, sound decoders, resource bridge all read archive content
- **Impact:** Cannot read any file inside a mounted archive
- **Fix:** Add `zip` crate dependency, parse archives, decompress on read, provide seekable decompressed view

#### GAP-03: `uio_feof` hardcoded to 1
- **Requirements:** REQ-UIO-STREAM-007, REQ-UIO-ERR-005
- **Current code:** `rust/src/io/uio_bridge.rs:838-841` — always returns 1
- **Consumer:** `sdluio.c:92-100` uses EOF/error distinction
- **Impact:** Callers always see EOF, cannot distinguish successful reads from end-of-file
- **Fix:** Return `((*stream).status == UIO_STREAM_STATUS_EOF) as c_int`

#### GAP-04: `uio_ferror` hardcoded to 0
- **Requirements:** REQ-UIO-STREAM-008, REQ-UIO-ERR-006, REQ-UIO-INT-003
- **Current code:** `rust/src/io/uio_bridge.rs:844-847` — always returns 0
- **Consumer:** `sdluio.c:92-100` calls `uio_ferror(stream)` after zero-byte reads to call `SDL_SetError`
- **Impact:** I/O errors silently swallowed through SDL adapter
- **Fix:** Return `((*stream).status == UIO_STREAM_STATUS_ERROR) as c_int`

#### GAP-05: `errno` not set on failure
- **Requirements:** REQ-UIO-ERR-002, REQ-UIO-ERR-010, REQ-UIO-ERR-011
- **Current code:** All error paths return -1/null without setting errno
- **Consumer:** `sdluio.c` calls `strerror(errno)`, `options.c` checks errno
- **Impact:** Error messages meaningless; failure diagnosis impossible
- **Fix:** Set `errno` before every error return using `libc::__error()` or platform-appropriate setter

#### GAP-06: Stream buffer leaks on `uio_fclose`
- **Requirements:** REQ-UIO-MEM-004
- **Current code:** `rust/src/io/uio_bridge.rs:1821-1826` — buffer intentionally leaked
- **Impact:** Memory leak on every stream close
- **Fix:** Track buffer allocation size in stream or use self-describing allocation

#### GAP-07: Mount ordering design does not encode the documented provisional rule
- **Requirements:** REQ-UIO-MOUNT-002, REQ-UIO-MOUNT-003
- **Current code:** `sort_mount_registry` at line 347-354 sorts by `active`, `mount_point.len()`, `id`
- **Consumer:** `options.c` uses `uio_MOUNT_TOP`, `uio_MOUNT_BOTTOM`, `uio_MOUNT_ABOVE`
- **Impact:** Mount precedence may be incorrect; content from wrong mount may be served, especially in overlap cases where placement and path specificity interact
- **Fix:** Implement explicit placement ordering plus resolution logic that preserves the provisional rule: placement precedence first, then longest matching mount-point prefix, then recency/insertion order; add overlap tests, not just same-mount-point tests

#### GAP-08: `uio_fseek` does not clear EOF status
- **Requirements:** REQ-UIO-STREAM-010
- **Current code:** `rust/src/io/uio_bridge.rs:1926-1952` — seeks file but does not touch `stream.status`
- **Impact:** After EOF, seek does not restore stream to usable state
- **Fix:** Set `(*stream).status = UIO_STREAM_STATUS_OK` on successful seek

#### GAP-09: `uio_fflush(NULL)` incorrectly succeeds
- **Requirements:** Specification §5.3, REQ-UIO-ERR-004, REQ-UIO-FFI-001
- **Current code:** `rust/src/io/uio_bridge.rs:813-816` — returns 0 for null stream
- **Impact:** Divergent behavior from legacy contract and weak null-input handling
- **Fix:** Return -1 (EOF) for null stream argument and set `errno = EINVAL`

#### GAP-10: FFI safety coverage is incomplete and overstated
- **Requirements:** REQ-UIO-ERR-008, REQ-UIO-FFI-001 through REQ-UIO-FFI-004, REQ-UIO-INT-004
- **Current code:** only partial `catch_unwind` / null handling assumptions documented; no exported-surface audit
- **Consumer:** C startup, SDL adapter, Rust FFI callers all rely on ABI-stable public entry points
- **Impact:** Plan cannot claim FFI robustness without auditing all exported functions, all FFI-visible layouts, and panic containment paths
- **Fix:** Add explicit exported-surface audit, ABI verification checklist, and panic-containment verification across all public UIO entry points

#### GAP-11: Concurrency requirements are unplanned despite mount-registry and handle refactors
- **Requirements:** REQ-UIO-CONC-001, REQ-UIO-CONC-002, REQ-UIO-CONC-003, REQ-UIO-CONC-004
- **Current code:** global `Mutex<Vec<MountInfo>>` exists, but no documented verification of reader/mutator serialization, same-handle integrity, or unmount-with-live-handle behavior
- **Consumer:** concurrent audio decoding on separate handles; future archive handles and transplant/unmount paths increase risk
- **Impact:** mount updates, archive registry changes, and handle refactors could introduce races or torn-observation behavior
- **Fix:** add analysis and tests for separate-handle concurrency, serialized mount mutation, same-handle integrity expectations, and unmount/read interactions

#### GAP-12: Lifecycle-after-unmount safety floors are largely unspecified
- **Requirements:** REQ-UIO-LIFE-001 through REQ-UIO-LIFE-005, REQ-UIO-DIR-007
- **Current code:** no explicit plan for repository-close ordering, invalidating mount handles after unmount, or safe behavior for live directory/file/stream objects across unmount/shutdown misuse
- **Impact:** refactors in archive support, transplant, and stdio access could leave dangling references or misleading success behavior
- **Fix:** define and test no-crash/no-UB floors for live objects after unmount and shutdown-order violations; ensure mount handles become invalid after unmount

#### GAP-13: Boundary and integration obligations are not fully captured
- **Requirements:** REQ-UIO-INT-001, REQ-UIO-INT-002, REQ-UIO-INT-004, REQ-UIO-INT-005, REQ-UIO-BOUND-001 through REQ-UIO-BOUND-003
- **Current code:** integration table exists, but no explicit analysis of external ownership model, startup-policy boundary, or ABI preservation for Rust/C consumers
- **Impact:** plan could drift into moving startup policy into UIO or break caller assumptions about ownership/layout
- **Fix:** record these as explicit invariants in analysis and verify them again in Phase 12

### Tier 2 — Compatibility-Complete Gaps

#### GAP-14: `uio_clearerr` is no-op
- **Requirements:** REQ-UIO-STREAM-009
- **Current code:** `rust/src/io/uio_bridge.rs:887-889` — empty body
- **Fix:** Set `(*stream).status = UIO_STREAM_STATUS_OK`

#### GAP-15: Pattern matching hard-coded
- **Requirements:** REQ-UIO-LIST-009
- **Current code:** `rust/src/io/uio_bridge.rs:1296-1323` — only `.rmp` and `.zip`/`.uqm` patterns
- **Fix:** Use `regex` crate for `MATCH_REGEX` type

#### GAP-16: Cross-mount directory merge missing
- **Requirements:** REQ-UIO-LIST-002, REQ-UIO-LIST-003, REQ-UIO-LIST-016, REQ-UIO-LIST-017
- **Current code:** `uio_getDirList` at line 2024 only calls `fs::read_dir` on single resolved path
- **Fix:** Enumerate all contributing mounts, merge entries with dedup by name, preserve the documented provisional `.rmp` ordering rule

#### GAP-17: `uio_DirList` public ABI/layout is unresolved
- **Requirements:** REQ-UIO-LIST-012, REQ-UIO-LIST-013, REQ-UIO-MEM-005, REQ-UIO-MEM-007, REQ-UIO-FFI-004, REQ-UIO-INT-004
- **Current code:** `rust/src/io/uio_bridge.rs:254-259` includes extra `buffer` field not present in C struct; `uio_DirList_free` depends on side-channel bookkeeping
- **Impact:** cannot safely redesign allocation/free without first resolving the FFI-visible layout mismatch
- **Fix:** explicitly choose a final ABI-safe strategy before Phase 09 logic work: either change Rust `uio_DirList` to exact C layout or hide bookkeeping in private allocation wrapper memory

#### GAP-18: `uio_access` ignores mode bits
- **Requirements:** REQ-UIO-FILE-012, REQ-UIO-ARCHIVE-009
- **Current code:** `rust/src/io/uio_bridge.rs:114-136` — existence check only, `_mode` unused
- **Fix:** Check R_OK/W_OK/X_OK against file permissions and mount read-only status

#### GAP-19: FileBlock API all stubs
- **Requirements:** REQ-UIO-FB-001 through FB-007, REQ-UIO-ERR-012
- **Current code:** `rust/src/io/uio_bridge.rs:907-962` — stubs returning dummy/error
- **Fix:** Implement with real file subrange access or ensure clean ENOTSUP failure until implemented

#### GAP-20: StdioAccess API stubs
- **Requirements:** REQ-UIO-STDIO-001 through STDIO-006, REQ-UIO-ARCHIVE-011, REQ-UIO-ERR-012
- **Current code:** `rust/src/io/uio_bridge.rs:1190-1206` — return dummy handles
- **Fix:** Return real filesystem paths for STDIO mounts, temp-copy for archive content, clean failure on partial setup errors

#### GAP-21: `uio_transplantDir` semantics are underspecified for archive-backed content
- **Requirements:** REQ-UIO-MOUNT-008, REQ-UIO-LIFE-003, REQ-UIO-LIFE-004
- **Current code:** `rust/src/io/uio_bridge.rs:561-595` — calls `register_mount` but ignores `relative` for insertion position and does not define distinct transplanted mount ownership
- **Impact:** reusing an existing mount identity for a transplant could break unmount behavior, diagnostics, and lifecycle invariants
- **Fix:** create a distinct transplanted mount record that references shared backing content while preserving independent mount identity and unmount behavior

#### GAP-22: `uio_vfprintf` stub
- **Requirements:** REQ-UIO-STREAM-019, REQ-UIO-ERR-007, REQ-UIO-ERR-012
- **Current code:** `rust/src/io/uio_bridge.rs:748-755` — returns -1
- **Fix:** Use `libc::vsnprintf` or similar to format, then write to stream; if still deferred, fail explicitly with `errno = ENOTSUP`

#### GAP-23: `uio_getStdioAccess` returns dummy handle
- **Requirements:** REQ-UIO-STDIO-001, REQ-UIO-STDIO-002, REQ-UIO-ERR-012
- **Current code:** `rust/src/io/uio_bridge.rs:1190-1198` — creates empty struct
- **Fix:** For STDIO mounts, return real path. For archives, create temp copy.

#### GAP-24: `uio_getFileLocation` behavior for archive content must be planned explicitly
- **Requirements:** REQ-UIO-PATH-005, REQ-UIO-PATH-006, REQ-UIO-ARCHIVE-010
- **Current code:** `resolve_file_location` at line 445 only searches STDIO mounts
- **Impact:** temp-copy stdio bridge cannot inspect archive ownership consistently
- **Fix:** search archive entry indexes too and return successful owning-mount information for archive content, while still keeping startup/consumer-visible semantics grounded in requirements

#### GAP-25: Unsupported/stubbed exported APIs are not audited systematically
- **Requirements:** REQ-UIO-ERR-007, REQ-UIO-ERR-012, REQ-UIO-INT-005
- **Current code:** multiple exported stubs still return dummy handles, placeholder success, or ambiguous failures
- **Impact:** callers may proceed past the actual failing operation and fail later in confusing ways
- **Fix:** add an exported-surface audit enumerating every stub and its required failure sentinel / `errno`, then either implement or convert to clean failure

### Tier 3 — Quality/Cleanup Gaps

#### GAP-26: `uio_fread` requires C shim
- **Requirements:** REQ-UIO-STREAM-018
- **Current code:** Exported as `rust_uio_fread` at line 1837; C shim at `uio_fread_shim.c`
- **Fix:** Export directly as `uio_fread` from Rust

#### GAP-27: No native Rust API for Rust consumers
- **Requirements:** Specification §12.1
- **Current code:** Rust sound/resource modules use FFI `extern "C"` declarations
- **Fix:** Add safe Rust wrappers (non-blocking, after all parity work)

#### GAP-28: Diagnostics are no-ops
- **Requirements:** REQ-UIO-INT-007, REQ-UIO-LOG-001, REQ-UIO-LOG-002
- **Current code:** `uio_printMounts` and `uio_DirHandle_print` log a stub marker
- **Fix:** Print actual mount/dir info

#### GAP-29: `uio_fwrite` does not set stream status on error
- **Requirements:** REQ-UIO-STREAM-006
- **Current code:** `rust/src/io/uio_bridge.rs:850-884` — returns 0 on error but does not set status
- **Fix:** Set `stream.status = UIO_STREAM_STATUS_ERROR` on write failure

#### GAP-30: `uio_fputc`/`uio_fputs` do not set stream operation/status
- **Requirements:** REQ-UIO-STREAM-015
- **Current code:** Lines 758-810 — write but don't update `stream.operation` or `stream.status`
- **Fix:** Set `operation = WRITE`, set `status = ERROR` on failure

#### GAP-31: Cleanup paths need explicit null-safe and partial-failure cleanup review
- **Requirements:** REQ-UIO-ERR-003, REQ-UIO-MEM-006, REQ-UIO-MEM-007, REQ-UIO-CONC-005
- **Current code:** cleanup strategy exists piecemeal, but mount/repository teardown, temp-copy cleanup, archive unmount cleanup, and null-no-op cases are not audited comprehensively
- **Fix:** add cleanup-path review and tests for partial allocation failure, null-safe cleanup entry points, and mount/repository teardown leaks

## Integration Touchpoints

| Integration point | File | Functions involved | Direction |
|---|---|---|---|
| C startup | `sc2/src/options.c` | All mount/dir/list APIs | C calls Rust |
| SDL graphics | `sc2/src/libs/graphics/sdl/sdluio.c` | Stream APIs + ferror/feof | C calls Rust |
| Sound decoders | `rust/src/sound/aiff_ffi.rs` etc. | uio_open/read/close/fstat | Rust calls Rust-FFI |
| Audio heart | `rust/src/sound/heart_ffi.rs` | uio_fopen/fread/fseek/ftell | Rust calls Rust-FFI |
| Resource bridge | `rust/src/resource/ffi_bridge.rs` | Stream + unlink | Rust calls Rust-FFI |
| C shim | `sc2/src/libs/uio/uio_fread_shim.c` | rust_uio_fread → uio_fread | C wraps Rust |
| Build system | `sc2/src/libs/uio/Makeinfo` | File list selection | Conditional compile |
| Config | `sc2/config_unix.h:73-74` | `USE_RUST_UIO` define | Feature flag |

## External ownership and boundary invariants

- Startup policy remains in `options.c`; Rust UIO must not absorb addon/package/resource-index policy decisions (REQ-UIO-BOUND-001/002)
- SDL adapters and Rust engine consumers continue using only the public UIO API/ABI surface (REQ-UIO-BOUND-003, REQ-UIO-INT-004/006)
- Engine globals such as `contentDir`, `configDir`, `saveDir`, `meleeDir`, and `contentMountHandle` remain owned by external engine code (REQ-UIO-INT-002)
- Port-completeness work must not hide unsupported behavior behind fake-success shims (REQ-UIO-INT-005)

## Old Code to Replace/Remove

| File | What | When |
|---|---|---|
| `rust/src/io/uio_bridge.rs:838-841` | Hardcoded `uio_feof` | Phase 05 |
| `rust/src/io/uio_bridge.rs:844-847` | Hardcoded `uio_ferror` | Phase 05 |
| `rust/src/io/uio_bridge.rs:887-889` | No-op `uio_clearerr` | Phase 05 |
| `rust/src/io/uio_bridge.rs:1821-1826` | Leaking buffer in `uio_fclose` | Phase 05 |
| `rust/src/io/uio_bridge.rs:347-354` | Sort-based mount ordering | Phase 06 |
| `rust/src/io/uio_bridge.rs:1489` | `active_in_registry = false` for ZIP | Phase 08 |
| `rust/src/io/uio_bridge.rs:1296-1323` | Hard-coded regex patterns | Phase 09 |
| `rust/src/io/uio_bridge.rs:2024-2033` | Single-dir `read_dir` in getDirList | Phase 09 |
| `rust/src/io/uio_bridge.rs:2179-2211` | Buffer size side-channel registry | Phase 11 |
| `sc2/src/libs/uio/uio_fread_shim.c` | C shim for uio_fread | Phase 11 |

## Error Handling Map

| Error class | errno | Functions affected |
|---|---|---|
| File not found | ENOENT | uio_open, uio_fopen, uio_stat, uio_access, uio_unlink, uio_rename, uio_getDirList |
| Write to read-only | EROFS / EACCES | uio_open (write), uio_write, uio_unlink, uio_mkdir, uio_rmdir, uio_rename |
| Invalid argument | EINVAL | All with pointer/flag args |
| Unsupported operation | ENOTSUP | Write on archive, unsupported FS type, any still-unimplemented exported API |
| I/O error | EIO | uio_read, uio_write, uio_fread, uio_fwrite, uio_lseek, uio_fseek |
| Directory not empty | ENOTEMPTY | uio_rmdir |
| Already exists | EEXIST | uio_open (O_CREAT|O_EXCL), uio_mkdir |
| Archive parse failure | EIO / EINVAL | uio_mountDir (ZIP) |

## Requirement coverage matrix expectations

Phase 01 output must explicitly map every REQ family below to either existing verified behavior or one or more GAP entries:
- INIT / REPO / MOUNT / PATH / DIR / LIST / FILE / STREAM / ARCHIVE / FB / STDIO
- MEM / ERR / CONC / LIFE / INT / FFI / BOUND / LOG

No family may be left implicit.
