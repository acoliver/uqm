# Phase 08: res_OpenResFile Directory Sentinel + LoadResourceFromPath Guard — TDD + Implementation

## Phase ID
`PLAN-20260314-RESOURCE.P08`

## Prerequisites
- Required: Phase 07/07a completed
- Verify previous phase markers/artifacts exist
- Expected files from previous phase: `07-save-index-filtering.md`, `07a-save-index-filtering-verification.md`
- **Preflight dependency:** Phase 0.5 must have recorded the exact `uio_stat` ABI/signature and stat-buffer type from the authoritative C headers/source

## Requirements Implemented (Expanded)

### REQ-RES-FILE-002: Open helper compatibility
**Requirement text**: When a caller opens a resource file through the established helper API, the resource subsystem shall preserve the externally visible success, failure, and special-case behaviors required by existing resource consumers.

Behavior contract:
- GIVEN: a resource path resolves through UIO
- WHEN: `res_OpenResFile` is called
- THEN: directories return the sentinel, regular files return a normal stream, and stat failures degrade gracefully to normal open behavior

Why it matters:
- The open helper is the front door for downstream file-backed loaders.

### REQ-RES-FILE-003: Directory sentinel compatibility
**Requirement text**: When the established resource-file open helper encounters a directory-like target, the resource subsystem shall return the established sentinel handle value rather than null or a normal stream pointer, so that callers that test for the sentinel can distinguish directories from regular files. The file-length helper shall return 1 for the sentinel handle.

Behavior contract:
- GIVEN: Path "comm/arilou" is a directory within the UIO content mount
- WHEN: `res_OpenResFile(contentDir, "comm/arilou", "rb")` is called
- THEN: Returns `STREAM_SENTINEL` (all-bits-set pointer, `~0`)

- GIVEN: `STREAM_SENTINEL` is passed to `LengthResFile`
- WHEN: `LengthResFile(STREAM_SENTINEL)` is called
- THEN: Returns `1`

Why it matters:
- Loose-file speech detection uses `res_OpenResFile` and checks for the sentinel to determine if a resource path points to a directory of individual voice files vs. a single package file.

### REQ-RES-FILE-005: File-backed typed load integration
**Requirement text**: When a type-specific loader is invoked through the file-backed load helper, the resource subsystem shall open the resource relative to the established content/UIO environment, pass a compatible file handle and length to the loader callback, and close the file according to the established ownership contract after the callback returns.

Behavior contract:
- GIVEN: `LoadResourceFromPath` receives a path whose open result is `STREAM_SENTINEL`
- WHEN: the helper evaluates that open result
- THEN: it treats the sentinel as a failed file-backed load, does not invoke the loader callback, and returns null

- GIVEN: `LoadResourceFromPath` opens a zero-byte file
- WHEN: the helper measures its length
- THEN: it warns, closes the file handle, does not invoke the loader callback, and returns null

Why it matters:
- A directory sentinel is not a loadable file stream, and zero-byte content must be rejected before callback dispatch.

### REQ-RES-FILE-008: No leaked file handles on failure
**Requirement text**: If a file-backed load or raw-data operation fails after opening a file or allocating intermediate state, then the resource subsystem shall release any subsystem-owned file handles and intermediate allocations before reporting failure.

Behavior contract:
- GIVEN: `LoadResourceFromPath` opens a valid stream but discovers an invalid length condition
- WHEN: it returns failure
- THEN: it closes the file before returning and publishes no callback state

Why it matters:
- Invalid-path handling must not leak open streams or invoke loaders on unusable inputs.

## Implementation Tasks

### TDD: Tests to add

Tests in this phase must directly cover both the sentinel-producing open helper and the sentinel-rejecting load helper. Logic-extraction tests are acceptable only if the actual FFI entry points are also covered somewhere in the phase.

1. **`test_length_res_file_returns_1_for_sentinel`**
   - Call `LengthResFile(STREAM_SENTINEL)`
   - Assert: returns 1
   - marker: `@plan PLAN-20260314-RESOURCE.P08`
   - marker: `@requirement REQ-RES-FILE-003`

2. **`test_stream_sentinel_constant_is_all_bits_set`**
   - Assert: `STREAM_SENTINEL == !0usize as *mut c_void`
   - marker: `@plan PLAN-20260314-RESOURCE.P08`
   - marker: `@requirement REQ-RES-FILE-003`

3. **`test_load_resource_from_path_rejects_stream_sentinel_without_callback`**
   - Arrange for `res_OpenResFile` / the tested helper path to produce `STREAM_SENTINEL`
   - Invoke `LoadResourceFromPath`
   - Assert: returns null and the loader callback is never invoked
   - marker: `@plan PLAN-20260314-RESOURCE.P08`
   - marker: `@requirement REQ-RES-FILE-005`

4. **`test_load_resource_from_path_zero_length_closes_and_skips_callback`**
   - Arrange a valid file handle with reported length 0
   - Invoke `LoadResourceFromPath`
   - Assert: returns null, closes the stream, and does not invoke the loader callback
   - marker: `@plan PLAN-20260314-RESOURCE.P08`
   - marker: `@requirement REQ-RES-FILE-008`

### Implementation: Modify `rust/src/resource/ffi_bridge.rs`

#### 1. Add `uio_stat` to extern block using preflight-confirmed ABI

**Location:** extern `"C"` block (~lines 28-49)

- Use the exact signature and stat-buffer type captured in Phase 0.5.
- Do not improvise the ABI during implementation.

```rust
// @plan PLAN-20260314-RESOURCE.P08
// @requirement REQ-RES-FILE-003
// Signature/body details here must match the Phase 0.5 artifact exactly.
fn uio_stat(/* preflight-confirmed parameters */) -> c_int;
```

#### 2. Modify `res_OpenResFile`

**Location:** `res_OpenResFile` function (~lines 986-995)

- Use the preflight-confirmed stat type and directory predicate.
- Return `STREAM_SENTINEL` for directory targets.
- Fall through to `uio_fopen` for non-directories and stat failures.

#### 3. Modify `LoadResourceFromPath`

**Location:** `LoadResourceFromPath` function (~lines 1124-1158)

- Treat `STREAM_SENTINEL` as a failed file-backed open for this helper.
- Preserve the zero-length guard here as well.
- Do not set `_cur_resfile_name` or invoke the loader callback on sentinel or zero-length failure paths.

### Pseudocode traceability
- Uses pseudocode lines: PC-7 (180-194), PC-8 (200-221)

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Targeted tests
cargo test --lib -- resource::tests::test_length_res_file_returns_1_for_sentinel
cargo test --lib -- resource::tests::test_load_resource_from_path_rejects_stream_sentinel_without_callback
cargo test --lib -- resource::tests::test_load_resource_from_path_zero_length_closes_and_skips_callback
```

## Structural Verification Checklist
- [ ] `uio_stat` declared in extern block using the preflight-confirmed ABI
- [ ] `res_OpenResFile` calls `uio_stat` before `uio_fopen`
- [ ] Returns `STREAM_SENTINEL` for directories
- [ ] Falls through to `uio_fopen` for non-directories and stat failures
- [ ] `LoadResourceFromPath` explicitly rejects `STREAM_SENTINEL`
- [ ] `LoadResourceFromPath` has a zero-length check before callback invocation
- [ ] Plan/requirement traceability present

## Semantic Verification Checklist (Mandatory)
- [ ] Directories return `STREAM_SENTINEL`
- [ ] Regular files return normal `uio_fopen` result
- [ ] `uio_stat` failure falls through to `uio_fopen` (graceful degradation)
- [ ] `LengthResFile(STREAM_SENTINEL)` returns 1
- [ ] `LoadResourceFromPath` does not pass sentinel handles into loader callbacks
- [ ] `LoadResourceFromPath` closes valid handles on zero-length failure
- [ ] All other file wrappers handle sentinel correctly (verify existing code)
- [ ] Integration points validated end-to-end for sentinel and failure-path handling

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/resource/ffi_bridge.rs
```

## Success Criteria
- [ ] Tests pass
- [ ] `res_OpenResFile` correctly detects directories
- [ ] `LoadResourceFromPath` rejects sentinel and zero-length files before callback invocation
- [ ] No regressions in file I/O wrappers

## Failure Recovery
- rollback steps: `git checkout -- rust/src/resource/ffi_bridge.rs`
- blocking issues to resolve before next phase: unresolved `uio_stat` ABI or inability to test sentinel rejection in `LoadResourceFromPath`

## Phase Completion Marker
Create: `project-plans/20260311/resource/.completed/P08.md`

Contents:
- phase ID
- timestamp
- files changed
- tests added/updated
- verification outputs
- semantic verification summary
