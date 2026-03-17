# Phase 03: Stream Status Tracking

## Phase ID
`PLAN-20260314-FILE-IO.P03`

## Prerequisites
- Required: Phase 02a completed
- Verify previous phase markers/artifacts exist
- Expected files from previous phase: pseudocode component PC-01
- Blocking carry-forward from P00a: if direct C field access to `uio_Stream` exists, preserve exact field layout during all edits in this phase

## Requirements Implemented (Expanded)

### REQ-FIO-STREAM-STATUS: Accurate EOF and error reporting
**Requirement text**: When a stream reaches end of file, the subsystem SHALL set and report end-of-file status through `uio_feof` according to actual stream state. When a stream operation fails, the subsystem SHALL set and report stream error status through `uio_ferror` according to actual stream state. When a caller clears stream status, the subsystem SHALL clear end-of-file and error indicators through `uio_clearerr`.

Behavior contract:
- GIVEN: A stream that has read to EOF
- WHEN: `uio_feof(stream)` is called
- THEN: Returns non-zero

- GIVEN: A stream that had an I/O error
- WHEN: `uio_ferror(stream)` is called
- THEN: Returns non-zero

- GIVEN: A stream with EOF or error flag set
- WHEN: `uio_clearerr(stream)` is called
- THEN: Both flags are cleared; subsequent `uio_feof` and `uio_ferror` return 0

- GIVEN: A stream with EOF flag set
- WHEN: `uio_fseek(stream, ...)` is called
- THEN: EOF flag is cleared

Why it matters:
- SDL RWops adapter (`sdluio.c`) calls `uio_ferror()` after a zero-length `uio_fread()` to distinguish EOF from error. Hardcoded `0` causes all short reads to be misclassified as EOF.

### REQ-FIO-RESOURCE-MGMT: No stream buffer leaks
**Requirement text**: When a caller closes a stream, the subsystem SHALL flush or discard buffered state, release owned resources, and SHALL NOT leak stream-owned buffers.

Behavior contract:
- GIVEN: A stream opened with `uio_fopen`
- WHEN: `uio_fclose(stream)` is called
- THEN: All stream-owned memory (including internal buffer) is freed

### REQ-FIO-ABI-AUDIT: `uio_Stream` layout decision carried into implementation
**Requirement text**: If preflight audit finds direct C field access under Rust-UIO, exact `uio_Stream` field layout preservation is mandatory during stream work.

## Implementation Tasks

### Files to modify
- `rust/src/io/uio_bridge.rs`
  - **`uio_Stream` struct definition**: Preserve or document field layout according to P00a audit result before changing stream-status logic
    - marker: `@plan PLAN-20260314-FILE-IO.P03`
    - marker: `@requirement REQ-FIO-ABI-AUDIT`
  - **`uio_feof`**: Replace hardcoded return with status-field check
    - marker: `@requirement REQ-FIO-STREAM-STATUS`
  - **`uio_ferror`**: Replace hardcoded return with status-field check
  - **`uio_clearerr`**: Replace no-op with status reset
  - **`uio_fread` direct export body / current read path**: set EOF/error status on read completion
  - **`uio_fgets`**: Set EOF/error status appropriately
  - **`uio_fgetc`**: Set EOF/error status appropriately
  - **`uio_fwrite`**: Set error status on write failure
  - **`uio_fseek`**: Clear EOF flag on successful seek
  - **`uio_fclose`**: Fix buffer leak — free `stream.buf` if internally allocated
  - **`uio_fopen`**: Initialize `status` to `UIO_STREAM_STATUS_OK`

### Pseudocode traceability
- Uses pseudocode lines: PC-01 lines 01–43

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `uio_feof` no longer returns a hardcoded value
- [ ] `uio_ferror` no longer returns a hardcoded value
- [ ] `uio_clearerr` actually clears the status field
- [ ] Read and write paths update stream status consistently
- [ ] `uio_fseek` clears EOF flag
- [ ] `uio_fclose` frees the stream buffer
- [ ] `uio_fopen` initializes status to OK
- [ ] `uio_Stream` layout remained compliant with the P00a ABI audit decision

## Semantic Verification Checklist (Mandatory)
- [ ] After reading to EOF, `uio_feof` returns non-zero
- [ ] After reading to EOF, `uio_ferror` returns 0
- [ ] After `uio_clearerr`, both `uio_feof` and `uio_ferror` return 0
- [ ] After `uio_fseek`, `uio_feof` returns 0
- [ ] Write failure path sets error status
- [ ] `uio_fclose` does not leak the internal buffer
- [ ] SDL RWops adapter path: `uio_fread` returns 0 at EOF → `uio_ferror` returns 0 → adapter classifies as EOF (not error)

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/io/uio_bridge.rs
```

## Success Criteria
- [ ] Stream status tests pass
- [ ] Verification commands pass
- [ ] No TODO/FIXME in modified code sections

## Failure Recovery
- Rollback: `git checkout -- rust/src/io/uio_bridge.rs`
- Blocking issues: `uio_Stream` layout mismatch if P00a found direct field access

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P03.md` containing:
- stream-status test summary
- ABI-layout compliance note
