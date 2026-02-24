# Phase 05: Seek-Past-End Fix — Implementation

## Phase ID
`PLAN-20260224-STATE-SWAP.P05`

## Prerequisites
- Required: Phase P04a (TDD Verification) completed
- Expected: 10 failing tests for seek-past-end behavior

## Requirements Implemented (Expanded)

### REQ-SF-001: Seek-Past-End Allowed
**Requirement text**: Remove upper-bound clamping from `StateFile::seek`.

Behavior contract:
- GIVEN: Any state file
- WHEN: `seek(M, whence)` produces non-negative result
- THEN: Cursor set to M, return value 1

### REQ-SF-002: Write-After-Seek-Past-End Extends Buffer
**Requirement text**: Write at cursor > physical size grows buffer with zero-fill gap.

### REQ-SF-003: Read-After-Seek-Past-End Returns EOF
**Requirement text**: Read at cursor ≥ physical size returns 0.

### REQ-SF-005: Separate Used and Physical Size Tracking
**Requirement text**: `used` tracks high-water mark; reads check physical; length returns used.

## Implementation Tasks

### Files to modify
- `rust/src/state/state_file.rs`
  - marker: `@plan PLAN-20260224-STATE-SWAP.P05`
  - marker: `@requirement REQ-SF-001, REQ-SF-002, REQ-SF-003, REQ-SF-005`

### Specific changes

1. **`StateFile::seek`** (pseudocode lines 51–60):
   - Remove all `result > self.data.len() as i64` clamp branches
   - Compute `new_pos` per whence: Set=offset, Current=ptr+offset, End=used+offset
   - If `new_pos < 0`: set `ptr = 0`, return `Err` or a clamped indicator
   - Otherwise: set `ptr = new_pos as usize`, return `Ok`
   - **Note**: Seek uses `used` for SEEK_END (not `data.len()`), matching C's `fp->used`

2. **`StateFile::read`** (pseudocode lines 63–72):
   - Check `self.ptr >= self.data.len()` (physical size, not `used`)
   - Read up to `self.data.len() - self.ptr` bytes
   - Existing logic is close but needs to use `data.len()` explicitly

3. **`StateFile::write`** (pseudocode lines 75–86):
   - If `self.ptr + buf.len() > self.data.len()`: resize with 1.5x strategy
   - Copy data at `self.ptr`
   - After advancing `ptr`, update `self.used = max(self.used, self.ptr)`
   - Update `size_hint` if buffer grew past it

4. **`StateFile::open`** (pseudocode lines 93–107):
   - On first open (data is empty): `self.data = vec![0u8; self.size_hint]; self.used = 0`
   - On Write mode: `self.used = 0` (don't clear data — matches C debug paint behavior)
   - On Read/ReadWrite mode: preserve `used`
   - Reset `ptr = 0`

5. **`StateFile::delete`** (pseudocode lines 110–116):
   - `self.data.clear(); self.data.shrink_to_fit(); self.used = 0; self.ptr = 0`

6. **`StateFile::close`** (existing):
   - Change `open_count` decrement to work with `i32` (allow negative)

7. **Update existing tests**: Fix any tests that relied on the old behavior
   (e.g., `test_state_file_seek_negative` may need adjustment for new return type)

### Pseudocode traceability
- seek: lines 51–60
- read: lines 63–72
- write: lines 75–86
- length: lines 89–90
- open: lines 93–107
- delete: lines 110–116

## Verification Commands

```bash
# All gates must pass
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `StateFile::seek` has no upper clamp (no `data.len()` max)
- [ ] `StateFile::read` checks `self.ptr >= self.data.len()` for EOF
- [ ] `StateFile::write` updates `self.used = max(self.used, self.ptr)` after write
- [ ] `StateFile::length` returns `self.used`
- [ ] `StateFile::open` pre-allocates `data` to `size_hint` and sets `used = 0` for write
- [ ] `StateFile::delete` resets `used` to 0
- [ ] `open_count` is `i32` and handles negative values
- [ ] All `todo!()` markers from P03 are removed

## Semantic Verification Checklist (Mandatory)
- [ ] `test_seek_past_end_allowed` passes (cursor set beyond buffer)
- [ ] `test_read_after_seek_past_end_returns_zero` passes
- [ ] `test_write_after_seek_past_end_extends_buffer` passes (buffer grew, gap zeroed)
- [ ] `test_length_returns_used_not_physical` passes
- [ ] `test_read_checks_physical_size_not_used` passes
- [ ] `test_open_count_can_go_negative` passes
- [ ] All existing tests pass (updated as needed)
- [ ] No `todo!()`, `FIXME`, or `HACK` in state_file.rs

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/state/state_file.rs || echo "CLEAN"
```

## Success Criteria
- [ ] All 10 new tests pass (GREEN)
- [ ] All existing tests pass
- [ ] `cargo fmt`, `cargo clippy`, `cargo test` all pass
- [ ] No deferred implementation markers

## Failure Recovery
- rollback: `git checkout -- rust/src/state/state_file.rs`
- blocking issues: if physical-vs-used separation breaks FFI callers

## Phase Completion Marker
Create: `project-plans/memandres/state/.completed/P05.md`

Contents:
- phase ID: P05
- files modified: `rust/src/state/state_file.rs`
- changes: seek unclamped, used/physical separated, read/write/open/delete updated
- tests: all pass (10 new + existing)
- verification: cargo fmt/clippy/test all clean
