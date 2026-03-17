# Phase 04: Stream State Fix — TDD

## Phase ID
`PLAN-20260314-UIO.P04`

## Prerequisites
- Required: Phase 03a completed
- Stream scaffolding compiles and existing tests pass

## Requirements Implemented (Expanded)

Same as Phase 03 — this phase writes tests that will verify the stream state behavior.

## Implementation Tasks

### Files to modify

#### `rust/src/io/uio_bridge.rs` — test module (at end of file, ~line 2218+)

Add the following test functions:

- **`test_feof_returns_zero_when_status_ok`**
  - marker: `@plan PLAN-20260314-UIO.P04`
  - marker: `@requirement REQ-UIO-STREAM-007`
  - Create a stream with `status = STATUS_OK`
  - Assert `uio_feof(stream)` returns 0

- **`test_feof_returns_nonzero_when_status_eof`**
  - marker: `@plan PLAN-20260314-UIO.P04`
  - marker: `@requirement REQ-UIO-STREAM-007`
  - Create a stream with `status = STATUS_EOF`
  - Assert `uio_feof(stream)` returns non-zero

- **`test_feof_returns_zero_when_status_error`**
  - marker: `@plan PLAN-20260314-UIO.P04`
  - marker: `@requirement REQ-UIO-STREAM-007`
  - Create a stream with `status = STATUS_ERROR`
  - Assert `uio_feof(stream)` returns 0 (error is not EOF)

- **`test_ferror_returns_zero_when_status_ok`**
  - marker: `@plan PLAN-20260314-UIO.P04`
  - marker: `@requirement REQ-UIO-STREAM-008`
  - Create a stream with `status = STATUS_OK`
  - Assert `uio_ferror(stream)` returns 0

- **`test_ferror_returns_nonzero_when_status_error`**
  - marker: `@plan PLAN-20260314-UIO.P04`
  - marker: `@requirement REQ-UIO-STREAM-008`
  - Create a stream with `status = STATUS_ERROR`
  - Assert `uio_ferror(stream)` returns non-zero

- **`test_ferror_returns_zero_when_status_eof`**
  - marker: `@plan PLAN-20260314-UIO.P04`
  - marker: `@requirement REQ-UIO-STREAM-008`
  - Create a stream with `status = STATUS_EOF`
  - Assert `uio_ferror(stream)` returns 0 (EOF is not error)

- **`test_clearerr_resets_eof`**
  - marker: `@plan PLAN-20260314-UIO.P04`
  - marker: `@requirement REQ-UIO-STREAM-009`
  - Create a stream with `status = STATUS_EOF`
  - Call `uio_clearerr(stream)`
  - Assert `uio_feof(stream)` returns 0

- **`test_clearerr_resets_error`**
  - marker: `@plan PLAN-20260314-UIO.P04`
  - marker: `@requirement REQ-UIO-STREAM-009`
  - Create a stream with `status = STATUS_ERROR`
  - Call `uio_clearerr(stream)`
  - Assert `uio_ferror(stream)` returns 0

- **`test_fseek_clears_eof`**
  - marker: `@plan PLAN-20260314-UIO.P04`
  - marker: `@requirement REQ-UIO-STREAM-010`
  - Open a real file as a stream
  - Read to EOF (so status becomes EOF)
  - Call `uio_fseek(stream, 0, SEEK_SET)`
  - Assert `uio_feof(stream)` returns 0

- **`test_fclose_does_not_leak_buffer`**
  - marker: `@plan PLAN-20260314-UIO.P04`
  - marker: `@requirement REQ-UIO-MEM-004`
  - Open a real file as a stream, read some data (allocates buffer)
  - Call `uio_fclose(stream)`
  - Verify no crash (memory leak testing is best effort; structural verification that free is called)

- **`test_fflush_null_returns_error`**
  - marker: `@plan PLAN-20260314-UIO.P04`
  - Call `uio_fflush(std::ptr::null_mut())`
  - Assert returns -1

- **`test_fwrite_sets_error_status_on_failure`**
  - marker: `@plan PLAN-20260314-UIO.P04`
  - marker: `@requirement REQ-UIO-STREAM-006`
  - Open a read-only file stream
  - Attempt `uio_fwrite` to it
  - Assert `uio_ferror(stream)` returns non-zero

- **`test_feof_null_stream_returns_zero`**
  - marker: `@plan PLAN-20260314-UIO.P04`
  - marker: `@requirement REQ-UIO-SAFE-001`
  - Call `uio_feof(std::ptr::null_mut())`
  - Assert returns 0 (safe null handling)

- **`test_ferror_null_stream_returns_zero`**
  - marker: `@plan PLAN-20260314-UIO.P04`
  - marker: `@requirement REQ-UIO-SAFE-001`
  - Call `uio_ferror(std::ptr::null_mut())`
  - Assert returns 0

- **`test_clearerr_null_stream_safe`**
  - marker: `@plan PLAN-20260314-UIO.P04`
  - marker: `@requirement REQ-UIO-SAFE-001`
  - Call `uio_clearerr(std::ptr::null_mut())`
  - Assert no crash

### Test helper: stream creation
- Add a test helper `create_test_stream(status: c_int) -> *mut uio_Stream` that allocates a minimal stream struct for unit testing stream state functions
- This avoids needing full file I/O for simple state-machine tests

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] All 16 test functions added
- [ ] Test helper function for stream creation exists
- [ ] Tests use `@plan` and `@requirement` markers
- [ ] Tests compile

## Semantic Verification Checklist
- [ ] Tests verify behavior, not implementation internals
- [ ] Tests cover both success and failure paths
- [ ] Null-safety tests included
- [ ] Tests are independent (no shared mutable state between tests)
- [ ] Tests initially fail or remain pending until Phase 05 provides the first real behavior changes

## Success Criteria
- [ ] All 16 tests pass
- [ ] No existing tests broken
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git checkout -- rust/src/io/uio_bridge.rs`
- blocking issues: stream struct cannot be created outside of full fopen path

## Phase Completion Marker
Create: `project-plans/20260311/uio/.completed/P04.md`
