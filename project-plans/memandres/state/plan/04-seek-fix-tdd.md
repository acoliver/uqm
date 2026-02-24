# Phase 04: Seek-Past-End Fix — TDD

## Phase ID
`PLAN-20260224-STATE-SWAP.P04`

## Prerequisites
- Required: Phase P03a (Stub Verification) completed
- Expected: `StateFile` has `used` field, `open_count` is `i32`

## Requirements Implemented (Expanded)

### REQ-SF-001: Seek-Past-End Allowed
**Requirement text**: `StateFile::seek` shall allow the cursor to exceed buffer bounds.

Behavior contract:
- GIVEN: A state file with 10 bytes of data
- WHEN: `seek(1000, SEEK_SET)` is called
- THEN: `position()` returns 1000

### REQ-SF-002: Write-After-Seek-Past-End Extends Buffer
**Requirement text**: Writing at a cursor position beyond the buffer grows it.

Behavior contract:
- GIVEN: A state file with 10 bytes, cursor at 100
- WHEN: `write(b"test")` is called
- THEN: Buffer grows to at least 104. `length()` returns 104. Bytes 10–99 are zero.

### REQ-SF-003: Read-After-Seek-Past-End Returns EOF
**Requirement text**: Reading at a cursor beyond physical buffer returns 0.

Behavior contract:
- GIVEN: A state file with 10 bytes, cursor at 1000
- WHEN: `read(&mut buf)` is called
- THEN: Returns 0 bytes read.

### REQ-SF-005: Separate Used and Physical Size Tracking
**Requirement text**: `length()` returns logical size, reads check physical size.

Behavior contract:
- GIVEN: Open with "wb", data allocated to size_hint, nothing written
- WHEN: `length()` called
- THEN: Returns 0
- WHEN: `read(&mut buf)` called at ptr=0
- THEN: Returns size_hint bytes (reads from physical buffer)

## Implementation Tasks

### Files to modify
- `rust/src/state/state_file.rs` — add tests to `mod tests`
  - marker: `@plan PLAN-20260224-STATE-SWAP.P04`
  - marker: `@requirement REQ-SF-001, REQ-SF-002, REQ-SF-003, REQ-SF-005`

### Tests to add

```rust
// @plan PLAN-20260224-STATE-SWAP.P04
// @requirement REQ-SF-001
#[test]
fn test_seek_past_end_allowed() {
    // Seek to position far beyond data, verify cursor is set
}

// @requirement REQ-SF-001
#[test]
fn test_seek_past_end_seek_set() {
    // SEEK_SET to large offset, verify position
}

// @requirement REQ-SF-001
#[test]
fn test_seek_past_end_seek_cur() {
    // SEEK_CUR with offset pushing past end, verify position
}

// @requirement REQ-SF-001
#[test]
fn test_seek_past_end_seek_end() {
    // SEEK_END with positive offset, verify position
}

// @requirement REQ-SF-003
#[test]
fn test_read_after_seek_past_end_returns_zero() {
    // Seek past end, read, verify 0 bytes returned
}

// @requirement REQ-SF-002
#[test]
fn test_write_after_seek_past_end_extends_buffer() {
    // Seek past end, write data, verify:
    // - buffer grew
    // - length() == seek_pos + write_len
    // - gap bytes are zero
}

// @requirement REQ-SF-005
#[test]
fn test_length_returns_used_not_physical() {
    // Open with "wb", verify length() == 0 despite buffer allocation
    // Write 10 bytes, verify length() == 10
}

// @requirement REQ-SF-005
#[test]
fn test_read_checks_physical_size_not_used() {
    // Open with "wb" (data allocated to size_hint, used=0)
    // Read at position 0 — should return data from physical buffer
    // This matches C behavior: ReadStateFile checks fp->size, not fp->used
}

// @requirement REQ-SF-001
#[test]
fn test_seek_negative_clamps_to_zero() {
    // Seek with large negative offset, verify position clamped to 0
    // Verify return value indicates clamping (0 vs 1)
}

// open_count regression test
#[test]
fn test_open_count_can_go_negative() {
    // Close without open, verify open_count == -1 (no panic)
}
```

### Pseudocode traceability
- Tests map to pseudocode lines 51–90 from component-001.md

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
# Tests should FAIL (RED phase of TDD) — implementation not yet done
cd rust && cargo test --workspace --all-features -- state_file 2>&1 | grep -c "FAILED"
# Expect: new tests fail, old tests may also fail due to stub changes
```

## Structural Verification Checklist
- [ ] All 10 tests listed above are present in `state_file.rs` tests module
- [ ] Tests have plan/requirement markers in comments
- [ ] Tests compile (even though they fail)
- [ ] No production code changes in this phase (tests only)

## Semantic Verification Checklist (Mandatory)
- [ ] `test_seek_past_end_allowed` asserts cursor > data length
- [ ] `test_read_after_seek_past_end_returns_zero` asserts 0 bytes read
- [ ] `test_write_after_seek_past_end_extends_buffer` asserts buffer growth AND gap is zero
- [ ] `test_length_returns_used_not_physical` asserts length() == 0 after wb open
- [ ] Tests fail with current implementation (RED phase verified)

## Success Criteria
- [ ] All tests compile
- [ ] New tests fail (RED — behavior not yet implemented)
- [ ] Test names clearly describe the behavior being tested

## Failure Recovery
- rollback: `git checkout -- rust/src/state/state_file.rs`

## Phase Completion Marker
Create: `project-plans/memandres/state/.completed/P04.md`

Contents:
- phase ID: P04
- files modified: `rust/src/state/state_file.rs`
- tests added: 10 tests for seek-past-end, write-extend, read-EOF, used-vs-physical, open_count
- RED phase verified: new tests fail
