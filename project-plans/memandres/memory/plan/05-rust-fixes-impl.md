# Phase 05: Rust Fixes — Implementation

## Phase ID
`PLAN-20260224-MEM-SWAP.P05`

## Prerequisites
- Required: Phase 04a (TDD Verification) completed
- `LogLevel::Fatal` alias exists and tested
- Expected files from previous phase: modified `rust/src/logging.rs` with `Fatal` constant and test

## Requirements Implemented (Expanded)

### REQ-MEM-005: OOM Log Level Correctness (Implementation)
**Requirement text**: Update `memory.rs` OOM log calls to use `LogLevel::Fatal` instead of `LogLevel::User` for semantic clarity.

Behavior contract:
- GIVEN: `memory.rs` uses `LogLevel::User` in OOM paths for `rust_hmalloc`, `rust_hcalloc`, `rust_hrealloc`
- WHEN: Those calls are changed to `LogLevel::Fatal`
- THEN: The numeric value sent to `log_add` remains `1` (no behavioral change), but the code reads clearly as a fatal error

Why it matters:
- Code clarity — `LogLevel::Fatal` in an OOM handler is self-documenting
- Matches the C code's use of `log_Fatal`
- No runtime behavior change (same numeric value)

## Implementation Tasks

### Files to modify

1. **`rust/src/memory.rs`**
   - Change `LogLevel::User` → `LogLevel::Fatal` in `rust_hmalloc` OOM path (line ~18)
   - Change `LogLevel::User` → `LogLevel::Fatal` in `rust_hcalloc` OOM path (line ~50)
   - Change `LogLevel::User` → `LogLevel::Fatal` in `rust_hrealloc` OOM path (line ~73)
   - marker: `@plan PLAN-20260224-MEM-SWAP.P05`
   - marker: `@requirement REQ-MEM-005`

### Pseudocode traceability
- Uses pseudocode lines: 70-74 (memory.rs log level update)

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Verify the change
grep 'LogLevel::Fatal' rust/src/memory.rs
grep -c 'LogLevel::User' rust/src/memory.rs  # should be 0 in OOM paths
```

## Structural Verification Checklist
- [ ] `rust/src/memory.rs` updated — 3 lines changed
- [ ] No `LogLevel::User` remains in OOM log calls in `memory.rs`
- [ ] `LogLevel::Fatal` used in all 3 OOM paths
- [ ] Plan/requirement traceability present
- [ ] Tests compile and run

## Semantic Verification Checklist (Mandatory)
- [ ] All existing memory tests pass (no behavioral change — same numeric value)
- [ ] `test_fatal_alias` passes (confirms Fatal == User == 1)
- [ ] No placeholder/deferred implementation patterns remain
- [ ] OOM behavior is unchanged at runtime

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/memory.rs
```

Note: `memory.rs` contains a comment "In later phases, this might initialize custom allocators" in `rust_mem_init` and `rust_mem_uninit`. These are pre-existing and outside the scope of this plan. They describe future work, not deferred implementation of current requirements.

## Success Criteria
- [ ] All 3 OOM log calls use `LogLevel::Fatal`
- [ ] All verification commands pass
- [ ] All existing tests pass

## Failure Recovery
- Rollback: `git checkout rust/src/memory.rs`
- Blocking issues: none — this is a pure semantic rename with no behavioral change

## Phase Completion Marker
Create: `project-plans/memandres/memory/.completed/P05.md`

Contents:
- phase ID
- timestamp
- files changed: `rust/src/memory.rs`
- tests added/updated: none (existing tests validate)
- verification outputs
- semantic verification summary
