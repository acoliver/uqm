# Phase 04: Rust Fixes — TDD

## Phase ID
`PLAN-20260224-MEM-SWAP.P04`

## Prerequisites
- Required: Phase 03a (Stub Verification) completed
- `LogLevel::Fatal` constant exists in `logging.rs`
- Expected files from previous phase: modified `rust/src/logging.rs`

## Requirements Implemented (Expanded)

### REQ-MEM-005: OOM Log Level Correctness (Test)
**Requirement text**: Add a test verifying that `LogLevel::Fatal` equals `LogLevel::User` and has numeric value 1.

Behavior contract:
- GIVEN: `LogLevel::Fatal` is defined as an alias for `LogLevel::User`
- WHEN: The test asserts equality and numeric value
- THEN: `LogLevel::Fatal == LogLevel::User` and `LogLevel::Fatal.as_i32() == 1`

Why it matters:
- Prevents accidental divergence if someone changes the alias
- Documents the C/Rust equivalence in executable form

### REQ-MEM-006: Behavioral Equivalence (Test)
**Requirement text**: Verify existing memory tests confirm behavioral equivalence.

Behavior contract:
- GIVEN: Existing tests for `rust_hmalloc`, `rust_hcalloc`, `rust_hrealloc`, `rust_hfree`
- WHEN: Tests are run
- THEN: All pass, confirming allocation/free behavior matches expectations

Why it matters:
- Ensures the Rust implementation is correct before wiring it into the C build

## Implementation Tasks

### Files to modify

1. **`rust/src/logging.rs`** — add test in existing `#[cfg(test)] mod tests`
   - Add `test_fatal_alias` test
   - marker: `@plan PLAN-20260224-MEM-SWAP.P04`
   - marker: `@requirement REQ-MEM-005`

### Files unchanged but verified
- `rust/src/memory.rs` — existing tests confirmed passing (REQ-MEM-006)

### Pseudocode traceability
- Uses pseudocode lines: 60-63 (Fatal alias — test validates the alias)

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Specific test
cargo test -p uqm_rust test_fatal_alias
```

## Structural Verification Checklist
- [ ] `test_fatal_alias` test added to `logging.rs`
- [ ] Test verifies `LogLevel::Fatal == LogLevel::User`
- [ ] Test verifies `LogLevel::Fatal.as_i32() == 1`
- [ ] Plan/requirement traceability present
- [ ] Tests compile and run

## Semantic Verification Checklist (Mandatory)
- [ ] Test fails if `Fatal` alias is changed to a different value (RED phase confirmed)
- [ ] Test passes with correct alias (GREEN)
- [ ] Existing memory tests pass (REQ-MEM-006 behavioral equivalence)
- [ ] No placeholder/deferred implementation patterns remain

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/logging.rs
```

## Success Criteria
- [ ] `test_fatal_alias` passes
- [ ] All existing tests pass
- [ ] All verification commands pass

## Failure Recovery
- Rollback: `git checkout rust/src/logging.rs`
- Blocking issues: none expected

## Phase Completion Marker
Create: `project-plans/memandres/memory/.completed/P04.md`

Contents:
- phase ID
- timestamp
- files changed: `rust/src/logging.rs`
- tests added: `test_fatal_alias`
- verification outputs
- semantic verification summary
