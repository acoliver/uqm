# Plan: UIO Virtual File System — Complete Rust Port

Plan ID: PLAN-20260314-UIO
Generated: 2026-03-14
Total Phases: 18 (P00a through P12)
Requirements: REQ-UIO-INIT-*, REQ-UIO-REPO-*, REQ-UIO-MOUNT-*, REQ-UIO-PATH-*, REQ-UIO-DIR-*, REQ-UIO-LIST-*, REQ-UIO-FILE-*, REQ-UIO-STREAM-*, REQ-UIO-ARCHIVE-*, REQ-UIO-FB-*, REQ-UIO-STDIO-*, REQ-UIO-MEM-*, REQ-UIO-ERR-*, REQ-UIO-CONC-*, REQ-UIO-LIFE-*, REQ-UIO-INT-*, REQ-UIO-FFI-*, REQ-UIO-BOUND-*, REQ-UIO-SAFE-*, REQ-UIO-LOG-*

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 00a)
2. Integration points are explicitly listed
3. TDD cycle is defined per slice
4. Lint/test/coverage gates are declared
5. Requirement coverage must stay explicit — every REQ family is mapped in analysis and final traceability, including concurrency, lifecycle, FFI, boundary, and integration obligations

## Subsystem Context

UIO is the virtual filesystem layer for UQM. It is **partially ported**: `USE_RUST_UIO` is active, and core exported `uio_*` entry points are implemented in Rust. However, significant gaps remain — most critically, no native ZIP/UQM archive reading, hardcoded stream state stubs, memory leaks, and no cross-mount directory merging.

This plan closes the gap between the current partial port and full specification compliance.

## Current State Summary

### What works (Rust-owned)
- Repository open/close
- STDIO mount/unmount with registry
- Directory handle open/close/relative
- File descriptor I/O (uio_open/close/read/write/fstat/lseek)
- Buffered stream I/O (uio_fopen/fclose/fread/fseek/ftell/fgets/fgetc/fwrite)
- Character/string output (uio_fputc/fputs/fflush)
- Directory listing for single STDIO directories
- Pattern matching for `.rmp` and `.zip`/`.uqm` regex (hard-coded)
- Basic file metadata (uio_stat/access/mkdir/rmdir/rename/unlink)
- Mount registry with path resolution

### What is broken or stubbed
- **ZIP/UQM archive reading** — mounts registered but `active_in_registry = false` for ZIP mounts; no archive parsing, decompression, or entry index
- **`uio_feof`** — hardcoded to return 1 (always EOF)
- **`uio_ferror`** — hardcoded to return 0 (never error)
- **`uio_clearerr`** — no-op stub
- **`uio_fclose`** — leaks stream buffer
- **`uio_fseek`** — does not clear EOF status
- **`errno` setting** — not set on failure in any function
- **Mount ordering** — current registry ordering must be reworked to encode the provisional rule from requirements: explicit placement precedence first, then longest matching mount-point prefix, then recency/insertion order
- **`uio_fflush(NULL)`** — returns success (legacy rejects NULL)
- **Pattern matching** — hard-coded regex for two patterns only
- **Cross-mount directory enumeration** — reads single STDIO dir, no merge
- **`uio_DirList` ABI/layout** — Rust definition includes extra allocation bookkeeping not present in the C public struct; must be resolved before relying on new DirList allocation helpers
- **`uio_access`** — existence check only, ignores mode bits
- **`uio_transplantDir`** — basic but does not yet define distinct transplanted mount semantics for archive-backed content or unmount/lifecycle behavior
- **FileBlock API** — all stubs (return dummy non-null or -1)
- **StdioAccess API** — stubs returning dummy handles
- **GPDir/GPFile/PRoot/GPRoot** — all stubs
- **`uio_vfprintf`** — stub returning -1
- **`uio_getStdioAccess`** — returns dummy handle, no real path
- **`uio_fread`** — exported as `rust_uio_fread`, requires C shim
- **Diagnostics** — `uio_printMounts`, `uio_DirHandle_print` are no-ops
- **Concurrency / lifecycle / FFI audit** — not yet planned in enough detail for mount mutations, unmount-with-live-handles, or full FFI-visible ABI validation

## Architecture

All UIO code lives in `rust/src/io/uio_bridge.rs` (currently 2483 lines). The plan will decompose this monolith into focused submodules while preserving the existing `#[no_mangle] extern "C"` ABI surface. Because this refactor is ABI-sensitive and cross-cutting, module extraction is treated as an explicit early deliverable rather than as an incidental side effect of later feature work:

```
rust/src/io/
  uio_bridge.rs          — FFI exports (thin wrappers calling into submodules)
  uio/
    mod.rs               — submodule re-exports
    mount.rs             — mount registry, ordering, resolution
    archive.rs           — ZIP/UQM archive reading and entry index
    stream.rs            — uio_Stream state machine (EOF/error/buffer)
    dirlist.rs           — directory enumeration, pattern matching, merge
    fileblock.rs         — FileBlock API
    stdio_access.rs      — StdioAccess bridge
    diagnostics.rs       — printMounts, DirHandle_print, logging
    types.rs             — shared types, constants, C-compatible structs
```

## Cross-Cutting Verification Themes

These themes cut across multiple phases and must be carried through analysis, implementation, and final verification rather than treated as one-off checks:

- **FFI robustness and ABI preservation** — null handling, failure sentinels, panic containment, and layout validation for all public structs/functions
- **Concurrency safety** — serialized mount mutations, safe independent-handle concurrency, and same-handle integrity expectations
- **Lifecycle safety floors** — no-crash/no-UB guarantees for live directory handles, file handles, streams, and shutdown-order violations
- **Boundary and integration preservation** — startup policy remains outside UIO; SDL/Rust consumers continue using public UIO contracts without special knowledge
- **Unsupported-surface correctness** — any still-unimplemented exported API must fail cleanly with the right sentinel and `errno`, never with fake success objects

## Phase Summary

| Phase | Title | Tier | Key Requirements |
|-------|-------|------|-----------------|
| 00a | Preflight Verification | — | Toolchain, dependency, ABI/FFI, concurrency baseline verification |
| 01 | Analysis | — | Full requirement matrix, gap analysis, integration/boundary map |
| 01a | Analysis Verification | — | Verify all requirement families are mapped |
| 02 | Pseudocode | — | Algorithmic design for all slices, including lifecycle/concurrency rules |
| 02b | Refactor & Audit Scaffold | — | Introduce module split skeleton and durable exported-surface audit artifact |
| 02a | Pseudocode Verification | — | Review pseudocode coverage |
| 03 | Stream State Fix — Stub | T1 | REQ-UIO-STREAM-007/008/009, REQ-UIO-ERR-005/006 |
| 03a | Stream State Fix — Verification | T1 | — |
| 04 | Stream State Fix — TDD | T1 | REQ-UIO-STREAM-007/008/009 |
| 04a | Stream State Fix — Verification | T1 | — |
| 05 | Stream State Fix — Impl | T1 | REQ-UIO-STREAM-007/008/009, REQ-UIO-MEM-004 |
| 05a | Stream State Fix — Verification | T1 | — |
| 06 | Mount Ordering & errno — Stub/TDD/Impl | T1 | REQ-UIO-MOUNT-002/003, REQ-UIO-ERR-002/004/010/011, REQ-UIO-CONC-002, REQ-UIO-LIFE-001/002 |
| 06a | Mount Ordering & errno — Verification | T1 | — |
| 07 | Archive Support — Stub/TDD | T1 | REQ-UIO-ARCHIVE-001 through ARCHIVE-008, ARCHIVE-ACCEPT |
| 07a | Archive Support — Verification | T1 | — |
| 08 | Archive Support — Impl | T1 | REQ-UIO-ARCHIVE-001 through ARCHIVE-ACCEPT, REQ-UIO-LIFE-004, REQ-UIO-CONC-001/003/004 |
| 08a | Archive Support — Verification | T1 | — |
| 09 | Directory Enumeration Merge & Regex — Stub/TDD/Impl | T1/T2 | REQ-UIO-LIST-001 through LIST-013, REQ-UIO-LIST-016/017, provisional `.rmp` acceptance coverage from REQ-UIO-LIST-015, REQ-UIO-FFI-004, REQ-UIO-MEM-005/007 |
| 09a | Directory Enumeration Merge & Regex — Verification | T1/T2 | — |
| 10 | Compatibility-Complete APIs — Stub/TDD/Impl | T2 | REQ-UIO-FB-*, REQ-UIO-STDIO-*, REQ-UIO-MOUNT-008, REQ-UIO-FILE-012/013/015/016, REQ-UIO-PATH-005/006, REQ-UIO-LIFE-003/005 |
| 10a | Compatibility-Complete APIs — Verification | T2 | — |
| 11 | Port-Completeness & Cleanup | T3 | REQ-UIO-STREAM-018/019, REQ-UIO-LOG-*, REQ-UIO-MEM-005/006/007, REQ-UIO-ERR-007/012, exported-surface audit artifact completion |
| 11a | Port-Completeness & Cleanup — Verification | T3 | — |
| 12 | Integration & End-to-End Verification | — | REQ-UIO-INT-*, REQ-UIO-BOUND-*, REQ-UIO-FFI-*, REQ-UIO-CONC-*, REQ-UIO-LIFE-*, REQ-UIO-ARCHIVE-ACCEPT |

## Integration Contract

### Existing Callers
- `sc2/src/options.c` → `uio_openRepository`, `uio_mountDir`, `uio_openDir`, `uio_openDirRelative`, `uio_closeDir`, `uio_getDirList`, `uio_DirList_free`, `uio_stat`, `uio_unmountDir`, `uio_transplantDir`
- `sc2/src/libs/graphics/sdl/sdluio.c` → `uio_fopen`, `uio_fclose`, `uio_fread`, `uio_fwrite`, `uio_fseek`, `uio_ftell`, `uio_ferror`
- `rust/src/sound/aiff_ffi.rs` → `uio_open`, `uio_close`, `uio_read`, `uio_fstat`
- `rust/src/sound/wav_ffi.rs` → `uio_open`, `uio_close`, `uio_read`, `uio_fstat`
- `rust/src/sound/mod_ffi.rs` → `uio_open`, `uio_close`, `uio_read`, `uio_fstat`
- `rust/src/sound/dukaud_ffi.rs` → `uio_open`, `uio_close`, `uio_read`, `uio_fstat`
- `rust/src/sound/heart_ffi.rs` → `uio_fopen`, `uio_fread`, `uio_fseek`, `uio_ftell`
- `rust/src/resource/ffi_bridge.rs` → `uio_fopen`, `uio_fclose`, `uio_fread`, `uio_fwrite`, `uio_fseek`, `uio_ftell`, `uio_fgetc`, `uio_fputc`, `uio_unlink`

### Existing Code Replaced/Removed
- `sc2/src/libs/uio/uio_fread_shim.c` — eliminated when `uio_fread` exported directly
- `uio_bridge.rs` internal buffer-size side-channel — replaced with self-describing allocation after the public `uio_DirList` layout issue is resolved explicitly
- Hard-coded regex patterns — replaced with real regex engine

### End-to-End Verification
- Build: `cd sc2 && make` (C+Rust combined build)
- Unit tests: `cd rust && cargo test --workspace --all-features`
- Integration: Mount real `.uqm` archive, open/read/seek/tell/stat a known asset
- SDL_RWops: Verify `uio_ferror` returns 0 after successful read, non-zero after I/O error
- Game startup: Verify engine starts with `USE_RUST_UIO=1` and loads content from archives
- FFI/lifecycle/concurrency: verify no panic escapes FFI, public layouts remain ABI-compatible, independent-handle concurrency is safe, mount mutations are serialized, and live objects preserve post-unmount safety floors

## Execution Tracker

| Phase | Status | Verified | Semantic Verified | Notes |
|------:|--------|----------|-------------------|-------|
| P00a  | ⬜     | ⬜       | N/A               |       |
| P01   | ⬜     | ⬜       | ⬜                |       |
| P01a  | ⬜     | ⬜       | ⬜                |       |
| P02   | ⬜     | ⬜       | ⬜                |       |
| P02a  | ⬜     | ⬜       | ⬜                |       |
| P02b  | ⬜     | ⬜       | ⬜                |       |
| P03   | ⬜     | ⬜       | ⬜                |       |
| P03a  | ⬜     | ⬜       | ⬜                |       |
| P04   | ⬜     | ⬜       | ⬜                |       |
| P04a  | ⬜     | ⬜       | ⬜                |       |
| P05   | ⬜     | ⬜       | ⬜                |       |
| P05a  | ⬜     | ⬜       | ⬜                |       |
| P06   | ⬜     | ⬜       | ⬜                |       |
| P06a  | ⬜     | ⬜       | ⬜                |       |
| P07   | ⬜     | ⬜       | ⬜                |       |
| P07a  | ⬜     | ⬜       | ⬜                |       |
| P08   | ⬜     | ⬜       | ⬜                |       |
| P08a  | ⬜     | ⬜       | ⬜                |       |
| P09   | ⬜     | ⬜       | ⬜                |       |
| P09a  | ⬜     | ⬜       | ⬜                |       |
| P10   | ⬜     | ⬜       | ⬜                |       |
| P10a  | ⬜     | ⬜       | ⬜                |       |
| P11   | ⬜     | ⬜       | ⬜                |       |
| P11a  | ⬜     | ⬜       | ⬜                |       |
| P12   | ⬜     | ⬜       | ⬜                |       |
