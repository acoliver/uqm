# Phase 03: Seek-Past-End Fix — Stub

## Phase ID
`PLAN-20260224-STATE-SWAP.P03`

## Prerequisites
- Required: Phase P02a (Pseudocode Verification) completed
- Expected files: `analysis/pseudocode/component-001.md`

## Requirements Implemented (Expanded)

### REQ-SF-001: Seek-Past-End Allowed
**Requirement text**: `StateFile::seek` shall allow the cursor to be set to any non-negative value without upper-bound clamping.

Behavior contract:
- GIVEN: A state file with N bytes of data
- WHEN: `seek(M, SEEK_SET)` where M > N
- THEN: Cursor is set to M; no error, no clamp

### REQ-SF-005: Separate Used and Physical Size Tracking
**Requirement text**: `StateFile` shall have a separate `used` field tracking the logical high-water mark, distinct from `data.len()` (physical allocation).

Behavior contract:
- GIVEN: A state file opened with "wb"
- WHEN: Buffer is allocated but nothing written
- THEN: `length()` returns 0, but `data.len() == size_hint`

## Implementation Tasks

### Files to modify
- `rust/src/state/state_file.rs`
  - Add `used: usize` field to `StateFile` struct
  - Change `open_count: u32` to `open_count: i32`
  - Update `StateFile::new()` to initialize `used = 0`
  - Update `StateFile::length()` to return `self.used` instead of `self.data.len()`
  - Update `StateFile::seek()` signature — remove upper-bound clamping logic (replace with `todo!()` body for now to mark incomplete)
  - Update `StateFile::open()` — on "w" mode, set `self.used = 0` (not `self.data.clear()`)
  - Update `StateFile::open()` — pre-allocate `data` to `size_hint` on first open
  - Update `StateFile::delete()` — reset `self.used = 0`
  - Update `StateFile::read()` — stub: mark that it will check physical size (temporary `todo!()` in the changed check path)
  - Update `StateFile::write()` — stub: mark that it will update `self.used` (temporary `todo!()`)
  - marker: `@plan PLAN-20260224-STATE-SWAP.P03`
  - marker: `@requirement REQ-SF-001, REQ-SF-005`

### Pseudocode traceability
- Uses pseudocode lines: 51–60 (seek), 63–72 (read), 75–90 (write/length), 93–116 (open/delete)

### Allowed in stub phase
- `todo!()` in seek/read/write bodies where behavior changes
- Existing tests may fail (they will be updated in P04)

### Not allowed
- Fake success behavior
- Duplicate modules

## Verification Commands

```bash
# Structural gate — compilation must succeed
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
# Tests may fail in stub phase — that's expected
cd rust && cargo test --workspace --all-features 2>&1 || echo "Expected failures in stub phase"
```

## Structural Verification Checklist
- [ ] `StateFile` struct has `used: usize` field
- [ ] `StateFile` struct has `open_count: i32` (not u32)
- [ ] `StateFile::new()` initializes `used = 0`
- [ ] `StateFile::length()` returns `self.used`
- [ ] `StateFile::seek()` has upper-clamp code removed or marked for replacement
- [ ] `StateFile::open()` handles "w" mode by resetting `used` without clearing `data`
- [ ] Compilation succeeds (`cargo check`)

## Semantic Verification Checklist (Mandatory)
- [ ] `used` field is structurally present and initialized
- [ ] `open_count` type changed to handle negative values
- [ ] No fake success behavior in stub code
- [ ] `todo!()` markers are in specific locations (seek/read/write changes only)

## Deferred Implementation Detection (Mandatory)

```bash
# Stub phase: todo!() IS allowed, but only in the specific changed functions
grep -n "todo!()" rust/src/state/state_file.rs
# Verify these are only in seek, read, write — not in other functions
```

## Success Criteria
- [ ] `StateFile` has `used` field and `i32` open_count
- [ ] `length()` returns `used`
- [ ] Code compiles
- [ ] Stub markers (`todo!()`) are present only in changed function paths

## Failure Recovery
- rollback: `git checkout -- rust/src/state/state_file.rs`
- blocking issues: if field addition breaks other modules

## Phase Completion Marker
Create: `project-plans/memandres/state/.completed/P03.md`

Contents:
- phase ID: P03
- files modified: `rust/src/state/state_file.rs`
- changes: added `used` field, changed `open_count` to i32, updated length(), stubbed seek/read/write
- tests: compilation passes; some existing tests may fail (expected)
