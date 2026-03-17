# Phase 00a: Preflight Verification

## Phase ID
`PLAN-20260314-FILE-IO.P00a`

## Purpose
Verify assumptions about the current codebase, toolchain, dependencies, ABI constraints, and unresolved specification questions before implementation begins.

## Toolchain Verification
- [ ] `cargo --version`
- [ ] `rustc --version`
- [ ] `cargo clippy --version`

## Dependency Verification
- [ ] `libc` crate present in `Cargo.toml` (used extensively by uio_bridge.rs)
- [ ] `serial_test` crate present in `Cargo.toml` (used by existing tests)
- [ ] Determine whether an additional regex engine/crate is needed for externally visible regex semantics; do not assume `regex` crate is sufficient without audit
- [ ] Confirm `anyhow` is available (used by `mod.rs`)
- [ ] Determine whether ZIP decompression support already exists elsewhere in the Rust workspace before adding crates

## Type/Interface Verification

### Verify existing struct layouts match assumptions
- [ ] `uio_Stream` struct in `uio_bridge.rs` has `status` field — needed for `REQ-FIO-STREAM-STATUS`
- [ ] `uio_Handle` ownership/locking model is documented from actual code — needed for `REQ-FIO-FILEBLOCK` and `REQ-FIO-THREAD-SAFETY`
- [ ] `uio_DirHandle` has `path`, `repository`, `refcount`-equivalent ownership state — needed for mount and directory semantics
- [ ] `MountInfo` has `active_in_registry`, `fs_type`, `mount_point`, `mounted_root` fields — needed for archive and mount ordering work

### Verify C header contracts still match
- [ ] `uio_Stream` C layout in `uiostream.h` — audit whether any C code directly accesses fields (spec §17 open question 1)
- [ ] `uio_DirList` first two fields are `names` + `numNames` (io.h lines 57–63)
- [ ] `uio_AutoMount` struct layout in C headers — needed for mount `autoMount` parameter audit
- [ ] `uio_getStdioAccess` signature in `utils.h` has 4 params: `(dir, path, flags, tempDir)`
- [ ] `uio_openFileBlock2` signature in `fileblock.h` is `(handle, offset, size)`
- [ ] `uio_accessFileBlock` signature in `fileblock.h` is `ssize_t ... (block, offset, length, char **buffer)`
- [ ] `uio_clearFileBlockBuffers` is present in `fileblock.h` and therefore part of the public ABI
- [ ] Audit `utils.h` for `uio_asprintf` / `uio_vasprintf` impact under `USE_RUST_UIO`; determine whether they remain provided elsewhere or require Rust-mode coverage/audit

### Verify call-path feasibility
- [ ] `uio_fread_shim.c` is the sole source of the `uio_fread` symbol — confirm removing it after Rust provides `uio_fread` directly
- [ ] `uio_fread` declaration in `uiostream.h` under `USE_RUST_UIO` guard — understand signature change implications
- [ ] `utils.c` is excluded from Rust-UIO build — confirm Rust must provide `uio_copyFile`, `uio_getStdioAccess`, `uio_releaseStdioAccess`, `uio_StdioAccessHandle_getPath`, and record whether any additional public utils symbols need treatment

### Verify callers of currently stubbed or high-risk APIs
- [ ] FileBlock callers: search for `uio_openFileBlock`, `uio_openFileBlock2`, `uio_accessFileBlock`, `uio_clearFileBlockBuffers`, `uio_copyFileBlock`
- [ ] `uio_vfprintf` callers: confirm netplay code calls it (or `uio_fprintf` which wraps it)
- [ ] `uio_getStdioAccess` callers: identify all call sites in the engine
- [ ] `uio_getFileLocation` callers: identify direct and indirect dependencies for stdio-access behavior boundaries
- [ ] Search for invalid-mode and invalid-flag handling expectations in existing tests/callers
- [ ] Enumerate all exported `extern "C"` entry points in `rust/src/io/uio_bridge.rs` and any related Rust UIO modules — needed for `REQ-FIO-PANIC-SAFETY`

## Test Infrastructure Verification
- [ ] Existing tests in `uio_bridge.rs` use `#[serial]` from `serial_test` — confirm test harness works
- [ ] Existing tests compile and pass: `cargo test --workspace --all-features`
- [ ] The full build chain works: `make` in `sc2/` with `USE_RUST_UIO` active

## Open Questions from Specification §17 (Must Resolve)

### Q1: `uio_Stream` layout ABI visibility
Audit needed: Does any C code under `USE_RUST_UIO` directly access `uio_Stream` fields?
- Check for `uio_INTERNAL` macro usage
- Check `sdluio.c` — does it access stream fields or only call functions?
- **Resolution**: Determines whether Rust `uio_Stream` field order is frozen
- **Carry-forward requirement**: record the decision in Phase 03/04 tasks and verification, not only in preflight notes

### Q2: AutoMount parity requirement
Audit needed: Are any non-NULL `autoMount` arrays passed to `uio_mountDir` in practice?
- Check `options.c` mount calls for `autoMount` parameter values
- Audit addon/content loading for listing-driven mount mutation expectations
- **Resolution**: Determines whether AutoMount must be implemented now or can remain explicitly deferred
- **Plan branch required**:
  - If required: Phase 07 and/or Phase 09 must include AutoMount implementation and semantic verification
  - If not required: final summary must record the audit evidence for deferral

### Q3: Temp-directory mounting
Audit needed: Do current callers depend on a process-level temporary directory being mounted into the repository namespace?
- Search temp-directory mount operations and temp-related repository paths
- Distinguish this from per-handle `uio_getStdioAccess(..., tempDir)` behavior, which is already normative
- **Resolution**: Determines whether conditional temp-mount support must be implemented
- **Plan branch required**:
  - If required: Phase 10 and/or Phase 11 must reserve implementation and verification tasks
  - If not required: final summary must record the audit evidence for deferral

## Additional Preflight Decisions Required

### Regex compatibility decision
- [ ] Audit the actual regex patterns exercised by engine callers
- [ ] Decide whether exact POSIX ERE semantics are required for parity, or whether a narrower compatibility set is acceptable with evidence
- [ ] Record the decision so Phase 07 does not overclaim semantics the selected engine cannot provide

### ZIP / FileBlock dependency decision
- [ ] Audit whether Phase 09 must depend on public FileBlock, or whether Rust ZIP reading can be implemented directly while still completing all FileBlock ABI work in Phase 08
- [ ] Record dependency rationale in analysis artifacts

### Panic-safety boundary decision
- [ ] Audit the required failure return for each public `extern "C"` entry point family when panic containment triggers (`NULL`, `-1`, `EOF`, `0`, or no return value)
- [ ] Decide whether a small set of typed wrappers/macros/helpers can cover all entry points without changing the exported ABI surface
- [ ] Record how later phases will prove that no Rust panic can unwind across the FFI boundary

### Post-unmount cleanup safety audit
- [ ] Identify cleanup paths that may run after mount removal: `uio_close`, `uio_fclose`, `uio_closeDir`, `uio_releaseStdioAccess`
- [ ] Identify current ownership/state assumptions that would make those paths unsafe

## Blocking Issues
[To be filled during execution. If non-empty, stop and revise plan.]

## Gate Decision
- [ ] PASS: proceed
- [ ] FAIL: revise plan

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P00a.md` containing:
- audit decisions for Q1/Q2/Q3
- regex compatibility decision
- FileBlock/ZIP dependency rationale
- panic-safety boundary strategy
- unresolved blockers (if any)
