# Plan: File I/O Subsystem Gap Closure

Plan ID: PLAN-20260314-FILE-IO
Generated: 2026-03-14
Total Phases: 14 (00–13)
Requirements: REQ-FIO-* (plan-defined traceability IDs mapped to requirements.md)

## Scope

This plan closes the gaps between the current Rust UIO implementation (`rust/src/io/uio_bridge.rs` and supporting files) and the specification/requirements. The subsystem is already ported and wired — `USE_RUST_UIO` is active, C core files (`io.c`, `uiostream.c`) are guarded out, and the game boots. This plan does **not** reimplement working functionality; it targets only specification-gap closure.

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 00a)
2. Integration points are explicitly listed
3. TDD cycle is defined per slice
4. Lint/test/coverage gates are declared

## Requirement Traceability Scheme

The requirements document is written as normative bullets rather than pre-numbered requirement IDs. This plan uses stable `REQ-FIO-*` identifiers as an internal traceability layer. Every such identifier must map to one or more SHALL statements in `requirements.md`; no phase may invent a new identifier without adding it consistently to the overview, phase files, and final summary.

Canonical traceability IDs used by this plan:
- `REQ-FIO-STREAM-STATUS`
- `REQ-FIO-STREAM-WRITE`
- `REQ-FIO-BUILD-BOUNDARY`
- `REQ-FIO-ACCESS-MODE`
- `REQ-FIO-MOUNT-ORDER`
- `REQ-FIO-MOUNT-AUTOMOUNT`
- `REQ-FIO-MOUNT-TEMP`
- `REQ-FIO-MUTATION`
- `REQ-FIO-PATH-NORM`
- `REQ-FIO-PATH-CONFINEMENT`
- `REQ-FIO-ERRNO`
- `REQ-FIO-PANIC-SAFETY`
- `REQ-FIO-DIRLIST-REGEX`
- `REQ-FIO-DIRLIST-UNION`
- `REQ-FIO-DIRLIST-EMPTY`
- `REQ-FIO-FILEBLOCK`
- `REQ-FIO-STDIO-ACCESS`
- `REQ-FIO-COPY`
- `REQ-FIO-ARCHIVE-MOUNT`
- `REQ-FIO-ARCHIVE-EDGE`
- `REQ-FIO-LIFECYCLE`
- `REQ-FIO-RESOURCE-MGMT`
- `REQ-FIO-POST-UNMOUNT-CLEANUP`
- `REQ-FIO-THREAD-SAFETY`
- `REQ-FIO-ABI-AUDIT`
- `REQ-FIO-UTILS-AUDIT`

## Gap Summary

The following gaps exist between the current code and the specification:

### G1 — Stream status tracking (EOF/error/clearerr) — CRITICAL
- `uio_feof()` hardcoded to return `1` (should reflect actual stream state)
- `uio_ferror()` hardcoded to return `0` (should reflect actual stream state)
- `uio_clearerr()` is a no-op (should clear both flags)
- **Impact**: SDL RWops adapter (`sdluio.c`) misclassifies EOF vs error after `uio_fread()` returns 0
- **Files**: `rust/src/io/uio_bridge.rs` lines 837–889
- **Requirements**: REQ-FIO-STREAM-STATUS

### G2 — `uio_vfprintf` stubbed — MODERATE
- Returns `-1` unconditionally; netplay debug logging uses `uio_fprintf`/`uio_vfprintf`
- **Files**: `rust/src/io/uio_bridge.rs` lines 745–755
- **Requirements**: REQ-FIO-STREAM-WRITE

### G3 — `uio_fread` C shim still required — MODERATE
- Rust exports `rust_uio_fread`; C shim `uio_fread_shim.c` provides the `uio_fread` symbol
- Specification requires all `uio_*` symbols exported directly from Rust
- **Files**: `rust/src/io/uio_bridge.rs` lines 1836–1842, `sc2/src/libs/uio/uio_fread_shim.c`
- **Requirements**: REQ-FIO-BUILD-BOUNDARY

### G4 — `uio_access()` ignores mode — MODERATE
- Only performs existence check; should honor `F_OK`/`R_OK`/`W_OK`/`X_OK` semantics per overlay
- **Files**: `rust/src/io/uio_bridge.rs` lines 113–136
- **Requirements**: REQ-FIO-ACCESS-MODE

### G5 — Regex matching is hardcoded special-case — MODERATE
- Only supports two known patterns via string comparison; specification requires regex semantics beyond ad hoc pattern checks
- **Files**: `rust/src/io/uio_bridge.rs` lines 1296–1325
- **Requirements**: REQ-FIO-DIRLIST-REGEX

### G6 — Mount ordering semantics simplified — MODERATE
- `TOP`/`BOTTOM`/`ABOVE`/`BELOW` placement flags are accepted but not used for ordering
- Sort heuristic replaces proper mount-placement semantics
- **Files**: `rust/src/io/uio_bridge.rs` lines 316–354, 1450–1520
- **Requirements**: REQ-FIO-MOUNT-ORDER

### G7 — ZIP/archive mount resolution disabled — MAJOR
- `active_in_registry` is set to `false` for ZIP mounts with `sourceDir`
- Archive-backed content is unreachable through normal path resolution
- **Files**: `rust/src/io/uio_bridge.rs` line 1489
- **Requirements**: REQ-FIO-ARCHIVE-MOUNT, REQ-FIO-ARCHIVE-EDGE

### G8 — FileBlock APIs stubbed — MODERATE
- `uio_accessFileBlock` returns `-1`; `uio_copyFileBlock` returns `-1`
- FileBlock ABI coverage is incomplete until all exported APIs, including buffer clearing, are implemented with correct signatures
- **Files**: `rust/src/io/uio_bridge.rs` lines 906–962
- **Requirements**: REQ-FIO-FILEBLOCK

### G9 — `uio_getStdioAccess` / `uio_releaseStdioAccess` stubbed — MODERATE
- Returns dummy handle; no actual path resolution or temp-copy support
- `utils.c` is excluded from Rust-UIO build, so Rust must provide these
- **Files**: `rust/src/io/uio_bridge.rs` lines 1190–1206
- **Requirements**: REQ-FIO-STDIO-ACCESS, REQ-FIO-MOUNT-TEMP

### G10 — `uio_fclose` stream buffer leak — MINOR
- Buffer allocated by stream is not freed on close (acknowledged TODO)
- **Files**: `rust/src/io/uio_bridge.rs` lines 1821–1826
- **Requirements**: REQ-FIO-RESOURCE-MGMT

### G11 — Cross-mount directory listing missing — MODERATE
- `uio_getDirList` reads only one physical directory; should merge entries across all mounts at a virtual path
- **Files**: `rust/src/io/uio_bridge.rs` lines 2024–2045
- **Requirements**: REQ-FIO-DIRLIST-UNION, REQ-FIO-DIRLIST-EMPTY

### G12 — Path normalization incomplete — MINOR
- `resolve_path()` does simple join; no `.`/`..` resolution, no repeated-slash collapse, no root clamping, no host-root confinement verification
- **Files**: `rust/src/io/uio_bridge.rs` lines 1281–1287
- **Requirements**: REQ-FIO-PATH-NORM, REQ-FIO-PATH-CONFINEMENT

### G13 — `uio_init`/`uio_unInit` are no-ops — MINOR
- No global state initialization or cleanup
- **Files**: `rust/src/io/uio_bridge.rs` lines 1334–1342
- **Requirements**: REQ-FIO-LIFECYCLE, REQ-FIO-THREAD-SAFETY

### G14 — Mutation resolution rules not implemented — MODERATE
- `uio_rename`, `uio_unlink`, `uio_mkdir`, `uio_rmdir` operate directly on host paths
- No overlay-aware resolution, no read-only mount enforcement, no `EXDEV` for cross-mount rename, no parent-path shadowing checks
- **Files**: `rust/src/io/uio_bridge.rs` lines 81–213
- **Requirements**: REQ-FIO-MUTATION

### G15 — `errno` not set on failures — MINOR
- Most Rust functions return `-1` on failure but don't set `errno`
- Invalid arguments, invalid mode strings, unsupported flag combinations, and partial-failure cleanup paths are not mapped systematically
- **Requirements**: REQ-FIO-ERRNO

### G16 — `uio_copyFile` not implemented in Rust — MODERATE
- `utils.c` (which contains `uio_copyFile`) is excluded from Rust-UIO build
- Rust `ffi.rs` has `copyFile` (upper-case C) but not the UIO-level `uio_copyFile`
- **Requirements**: REQ-FIO-COPY, REQ-FIO-UTILS-AUDIT

### G17 — `uio_getFileLocation` fails for archive-backed files (correct per spec) but doesn't set errno — MINOR
- Should set `errno = ENOENT` per spec and distinguish merged/synthetic cases correctly
- **Requirements**: REQ-FIO-ERRNO, REQ-FIO-STDIO-ACCESS

### G18 — Conditional compatibility requirements not yet planned concretely — MODERATE
- AutoMount parity and process-level temp-directory mounting are open questions in the spec, but implementation branches are not yet reserved
- **Requirements**: REQ-FIO-MOUNT-AUTOMOUNT, REQ-FIO-MOUNT-TEMP

### G19 — Cleanup safety after mount removal is under-specified — MODERATE
- `uio_close`, `uio_fclose`, `uio_closeDir`, and `uio_releaseStdioAccess` must remain safe after mount removal, but no dedicated phase coverage exists yet
- **Requirements**: REQ-FIO-POST-UNMOUNT-CLEANUP

### G20 — FFI panic containment not planned explicitly — CRITICAL
- Specification requires all `extern "C"` entry points to catch panics and convert them to safe error returns
- Current plan had no canonical traceability ID, implementation phase ownership, or verification gate for this ABI boundary contract
- **Requirements**: REQ-FIO-PANIC-SAFETY

## Phase Sequence

| Phase | Title | Gaps Addressed | Dependencies |
|-------|-------|---------------|-------------|
| 00a | Preflight Verification | — | — |
| 01 | Analysis | All | 00a |
| 01a | Analysis Verification | — | 01 |
| 02 | Pseudocode | All | 01a |
| 02a | Pseudocode Verification | — | 02 |
| 03 | Stream Status Tracking | G1, G10 | 02a |
| 03a | Stream Status Verification | — | 03 |
| 04 | Stream Output & `uio_fread` Direct Export | G2, G3 | 03a |
| 04a | Stream Output Verification | — | 04 |
| 05 | Path Normalization & `errno` | G12, G15, G17, G20 | 04a |
| 05a | Path Normalization Verification | — | 05 |
| 06 | Mount Ordering & Access Semantics | G6, G4, G14, G18 | 05a |
| 06a | Mount Ordering Verification | — | 06 |
| 07 | Regex & Cross-Mount Listing | G5, G11, G18 | 06a |
| 07a | Regex & Listing Verification | — | 07 |
| 08 | FileBlock Implementation | G8 | 07a |
| 08a | FileBlock Verification | — | 08 |
| 09 | ZIP Archive Mount Resolution | G7 | 08a |
| 09a | ZIP Archive Verification | — | 09 |
| 10 | StdioAccess & `uio_copyFile` | G9, G16, G17, G18 | 09a |
| 10a | StdioAccess Verification | — | 10 |
| 11 | Lifecycle & Init/Uninit | G13, G19 | 10a |
| 11a | Lifecycle Verification | — | 11 |
| 12 | Integration Testing | All | 11a |
| 12a | Integration Verification | — | 12 |
| 13 | Final Verification & Cleanup | All | 12a |

## Execution Tracker

| Phase | Status | Verified | Semantic Verified | Notes |
|------:|--------|----------|-------------------|-------|
| P00a  | ⬜     | ⬜       | N/A               |       |
| P01   | ⬜     | ⬜       | ⬜                |       |
| P01a  | ⬜     | ⬜       | ⬜                |       |
| P02   | ⬜     | ⬜       | ⬜                |       |
| P02a  | ⬜     | ⬜       | ⬜                |       |
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
| P12a  | ⬜     | ⬜       | ⬜                |       |
| P13   | ⬜     | ⬜       | ⬜                |       |
