# Phase 05: Stream State Fix — Implementation

## Phase ID
`PLAN-20260314-UIO.P05`

## Prerequisites
- Required: Phase 04a completed
- Stream state tests exist and their intended coverage is reviewed
- Scaffold helpers from Phase 03 are in place

## Requirements Implemented (Expanded)

### REQ-UIO-STREAM-007, REQ-UIO-STREAM-008, REQ-UIO-STREAM-009, REQ-UIO-STREAM-010
See Phase 03 for full behavior contracts.

### REQ-UIO-MEM-004: No memory leaks on stream close
See Phase 03 for full behavior contract.

### REQ-UIO-ERR-005, REQ-UIO-ERR-006: EOF and error indicators accurate
Covered by the stream status state machine implementation.

## Implementation Tasks

### Files to modify

#### `rust/src/io/uio_bridge.rs`

This phase performs the first real stream behavior changes. Phase 03 only established helper/scaffold support, and Phase 04 defined the tests that now drive the implementation and edge-case coverage.

- **Finalize `set_errno` for supported platforms**
  - marker: `@plan PLAN-20260314-UIO.P05`
  - Confirm `libc::__error()` on macOS, or use the platform-appropriate errno location helper
  - Add cfg-gated support if Linux builds are relevant to the repository

- **Implement dynamic stream status behavior in the production entry points**
  - marker: `@plan PLAN-20260314-UIO.P05`
  - marker: `@requirement REQ-UIO-STREAM-007, REQ-UIO-STREAM-008, REQ-UIO-STREAM-009, REQ-UIO-STREAM-010`
  - Change `uio_feof` from constant return behavior to real EOF-status inspection
  - Change `uio_ferror` from constant return behavior to real error-status inspection
  - Change `uio_clearerr` from no-op behavior to clearing EOF and error state
  - Change `uio_fseek` successful repositioning to clear EOF status

- **Implement stream-close buffer cleanup**
  - marker: `@plan PLAN-20260314-UIO.P05`
  - marker: `@requirement REQ-UIO-MEM-004`
  - Replace the intentional leak path in `uio_fclose` with correct buffer deallocation
  - Verify the buffer allocator/deallocator pair matches the actual allocation strategy

- **Implement NULL-stream flush failure behavior**
  - marker: `@plan PLAN-20260314-UIO.P05`
  - Change `uio_fflush(NULL)` from success to the legacy-compatible failure sentinel with `errno = EINVAL`

- **Implement write-side stream status tracking**
  - marker: `@plan PLAN-20260314-UIO.P05`
  - marker: `@requirement REQ-UIO-STREAM-006, REQ-UIO-STREAM-015`
  - Update `uio_fwrite` error paths to set error status
  - Update `uio_fputc` / `uio_fputs` to set write operation state and error status when writes fail

- **Apply errno to all stream function error paths**
  - marker: `@plan PLAN-20260314-UIO.P05`
  - marker: `@requirement REQ-UIO-ERR-002`
  - `uio_fopen` failure → `set_errno(ENOENT)` or appropriate code
  - `uio_fread` I/O failure → `set_errno(EIO)`
  - `uio_fwrite` I/O failure → `set_errno(EIO)`
  - `uio_fseek` failure → `set_errno(EIO)` or `EINVAL`
  - `uio_fclose` failure → `set_errno(EIO)`
  - Null pointer arguments → `set_errno(EINVAL)`

- **Ensure `uio_fread` sets STATUS_EOF on short read**
  - marker: `@plan PLAN-20260314-UIO.P05`
  - marker: `@requirement REQ-UIO-STREAM-005`
  - In `rust_uio_fread` at ~line 1900: verify that when fewer items are returned than requested, `stream.status` is set to `STATUS_EOF`
  - Verify that when read returns 0 bytes due to error (not EOF), `stream.status` is set to `STATUS_ERROR`

- **Ensure `uio_fgetc` sets EOF status**
  - marker: `@plan PLAN-20260314-UIO.P05`
  - marker: `@requirement REQ-UIO-STREAM-013`
  - When fgetc returns EOF (-1), set `stream.status = STATUS_EOF`

- **Remove any remaining buffer-leak comments/workarounds**
  - marker: `@plan PLAN-20260314-UIO.P05`
  - marker: `@requirement REQ-UIO-MEM-004`
  - Remove the comment block at lines 1821-1826 about intentional leaking
  - Verify `libc::free` call is correct (buffer was allocated with `libc::malloc`)

### Pseudocode traceability
- Uses pseudocode lines: 01-62, 63-72

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] All Phase 03 stubs are finalized (no remaining TODO/placeholder)
- [ ] `set_errno` is applied to all error return paths in stream functions
- [ ] `uio_fread` sets STATUS_EOF on short read
- [ ] `uio_fgetc` sets STATUS_EOF when returning EOF
- [ ] Buffer leak comment removed from `uio_fclose`
- [ ] Plan/requirement traceability present

## Semantic Verification Checklist
- [ ] All Phase 04 tests pass
- [ ] All existing tests pass
- [ ] `uio_feof` returns 0 for fresh streams, non-zero after EOF read
- [ ] `uio_ferror` returns 0 for good streams, non-zero after I/O error
- [ ] `uio_clearerr` restores stream to usable state
- [ ] `uio_fseek` after EOF allows subsequent reads
- [ ] `uio_fclose` does not leak buffer memory
- [ ] errno is set to meaningful values on failure

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|intentionally leak\|always returns\|hardcoded" rust/src/io/uio_bridge.rs
```

Expected: no hits related to stream state, EOF, error, or buffer leak.

## Success Criteria
- [ ] All stream state requirements verified by passing tests
- [ ] No memory leaks in stream close path
- [ ] errno set on all stream error paths
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git stash` or `git checkout -- rust/src/io/uio_bridge.rs`
- blocking issues: buffer allocation method mismatch (malloc vs Rust allocator)

## Phase Completion Marker
Create: `project-plans/20260311/uio/.completed/P05.md`
