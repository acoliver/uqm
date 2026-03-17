# Phase 01: Analysis

## Phase ID
`PLAN-20260314-FILE-IO.P01`

## Prerequisites
- Required: Phase 00a completed (preflight verification passed)

## Purpose
Produce domain analysis artifacts that map every gap to concrete code locations, identify integration touchpoints, establish the "old code to replace" inventory, and prove that every normative requirement area from `requirements.md` and `specification.md` is either already satisfied or assigned to a concrete implementation/verification phase.

## Expected Outputs

### 1. Gap-to-Code Map

Each gap from the overview must be expanded into:
- Current code location (file, line range)
- Current behavior (what it does now)
- Target behavior (what the spec requires)
- Integration consumers (who calls this and depends on correct behavior)
- Risk level (what breaks if this gap is not closed)
- Traceability IDs affected (`REQ-FIO-*`)

### 2. Requirement-to-Phase Coverage Matrix

Create a matrix that maps each canonical plan traceability ID to:
- the normative SHALL statements in `requirements.md` and `specification.md`
- the phase(s) that implement it
- the phase(s) that verify it
- whether it is unconditional or conditional on a preflight audit outcome

Required rows include at minimum:
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

### 3. Integration Touchpoints

Document every C and Rust consumer that depends on the APIs being fixed:

| Consumer | File | APIs Used | Gap Dependency |
|----------|------|-----------|---------------|
| SDL RWops adapter | `sc2/src/libs/graphics/sdl/sdluio.c` | `uio_fread`, `uio_ferror`, `uio_fclose` | `REQ-FIO-STREAM-STATUS` |
| Netplay debug | `sc2/src/libs/network/netplay/packetq.c` | `uio_fprintf` | `REQ-FIO-STREAM-WRITE` |
| C startup (`mountDirZips`, `loadIndices`) | `sc2/src/options.c` | `uio_mountDir`, `uio_getDirList`, `uio_stat` | `REQ-FIO-MOUNT-ORDER`, `REQ-FIO-ARCHIVE-MOUNT`, `REQ-FIO-DIRLIST-UNION` |
| Resource loading | `sc2/src/libs/resource/loadres.c` | `uio_getStdioAccess`, `uio_getFileLocation` boundary expectations | `REQ-FIO-STDIO-ACCESS` |
| Sound decoders | `rust/src/sound/aiff_ffi.rs` et al | `uio_open`, `uio_read`, `uio_close`, `uio_fstat` | `REQ-FIO-ERRNO` |
| Audio heart | `rust/src/sound/heart_ffi.rs` | `uio_fopen`, `uio_fread`, `uio_fseek`, `uio_ftell`, `uio_fclose` | `REQ-FIO-STREAM-STATUS` |
| Archive support | `rust/src/io/zip_reader.rs` | archive indexing and read path semantics | `REQ-FIO-FILEBLOCK`, `REQ-FIO-ARCHIVE-EDGE` |
| FFI ABI boundary | `rust/src/io/uio_bridge.rs` | all exported `extern "C"` entry points | `REQ-FIO-PANIC-SAFETY` |

### 4. Old Code to Replace/Remove

| Current Code | Action | Replaced By |
|-------------|--------|-------------|
| `uio_fread_shim.c` | Remove from Makeinfo, delete or leave inert in-tree | Rust `uio_fread` direct export |
| `uio_feof` hardcoded return | Replace | Stream-state-aware implementation |
| `uio_ferror` hardcoded return | Replace | Stream-state-aware implementation |
| `uio_clearerr` no-op | Replace | Actual flag clearing |
| `uio_vfprintf` stub | Replace | ABI-correct formatted output implementation |
| `uio_access` existence-only | Replace | Mode-aware access check |
| `matches_pattern` regex special cases | Replace | audited compatibility implementation |
| FileBlock stub functions | Replace | ABI-correct FileBlock implementation |
| `uio_getStdioAccess` stub | Replace | Path-resolution + temp-copy implementation |
| `resolve_path` simple join | Replace | Normalization with confinement checks |
| naked FFI entry points without containment audit | Replace or wrap | panic-contained `extern "C"` boundary strategy |

### 5. Public API Audit Inventory

Document all public ABI functions and contracts that require explicit audit because they are easy to miss in implementation planning:
- `uio_openFileBlock`
- `uio_openFileBlock2`
- `uio_accessFileBlock`
- `uio_clearFileBlockBuffers`
- `uio_copyFileBlock`
- `uio_closeFileBlock`
- `uio_setFileBlockUsageHint`
- `uio_copyFile`
- `uio_getStdioAccess`
- `uio_releaseStdioAccess`
- `uio_StdioAccessHandle_getPath`
- `uio_asprintf` / `uio_vasprintf` audit result under `USE_RUST_UIO`
- exported panic-containment helper strategy for every `extern "C"` family

### 6. Edge/Error Handling Map

For each requirement area, document the error conditions, invalid-argument cases, and expected errno values, including:
- invalid `uio_access` mode combinations
- invalid or unsupported open flags
- bad `uio_fopen` mode strings
- null pointers on public entry points where detectably invalid
- partial-allocation cleanup obligations
- cleanup safety after mount removal
- archive duplicate-entry, normalization, and case-sensitive lookup rules
- mount-time archive indexing failures and rollback behavior
- panic-containment fallback return values for each exported ABI shape

### 7. SHALL-Statement Closure Appendix

Create a checklist or appendix that enumerates every normative SHALL statement in `requirements.md` and `specification.md` and assigns each one to exactly one canonical `REQ-FIO-*` row or marks it already satisfied with evidence. This appendix is the source of truth for final closure in P13.

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] All gaps from overview are covered in the gap-to-code map
- [ ] Requirement-to-phase matrix exists and uses only canonical `REQ-FIO-*` IDs from `00-overview.md`
- [ ] All integration consumers identified
- [ ] All old-code-to-replace entries identified
- [ ] Public API audit inventory includes all FileBlock functions, utils audit items, and the panic-containment strategy
- [ ] Edge/error handling map covers invalid arguments, partial-failure cleanup, mount-time archive failure rollback, and post-unmount cleanup safety
- [ ] SHALL-statement appendix exists and is complete

## Semantic Verification Checklist
- [ ] Every normative requirement area from `requirements.md` and `specification.md` maps to a concrete phase or is explicitly confirmed already satisfied
- [ ] Conditional requirements (AutoMount, process temp mount) have explicit branch outcomes
- [ ] ABI-sensitive assumptions from P00a are carried forward into later implementation phases
- [ ] No gap depends on an unresolved open question without an explicit branch or stop condition
- [ ] Panic-safety requirement coverage is explicit and assigned to implementation plus verification phases

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P01.md` containing:
- gap-to-code map
- requirement-to-phase coverage matrix
- public API audit inventory
- SHALL-statement appendix
- unresolved analysis blockers (if any)
