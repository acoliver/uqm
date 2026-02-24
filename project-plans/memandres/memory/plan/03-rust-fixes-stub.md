# Phase 03: Rust Fixes — Stub

## Phase ID
`PLAN-20260224-MEM-SWAP.P03`

## Prerequisites
- Required: Phase 02a (Pseudocode Verification) completed
- Verify pseudocode artifacts exist
- Expected files from previous phase: `analysis/pseudocode/component-001.md`

## Requirements Implemented (Expanded)

### REQ-MEM-005: OOM Log Level Correctness
**Requirement text**: The Rust `memory.rs` OOM log calls must use a log level semantically equivalent to C's `log_Fatal`. Add a `Fatal` alias to the Rust `LogLevel` enum to document that `Fatal == User == 1`, matching the C header `log_Fatal = log_User`.

Behavior contract:
- GIVEN: The Rust `LogLevel` enum exists with `User = 1`
- WHEN: A `Fatal` associated constant is added as an alias for `User`
- THEN: Code can use `LogLevel::Fatal` which resolves to value `1`, matching C's `log_Fatal`

Why it matters:
- Semantic clarity — reading `LogLevel::User` in an OOM handler is confusing
- Documents the C equivalence explicitly
- Prevents future misunderstanding about which log level to use for fatal errors

## Implementation Tasks

### Files to modify

1. **`rust/src/logging.rs`**
   - Add `pub const Fatal: LogLevel = LogLevel::User;` in an `impl LogLevel` block
   - marker: `@plan PLAN-20260224-MEM-SWAP.P03`
   - marker: `@requirement REQ-MEM-005`

This is a stub phase — the alias is added but not yet used in `memory.rs`. That happens in P05.

### Pseudocode traceability
- Uses pseudocode lines: 60-63 (LogLevel Fatal alias)

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust/src/logging.rs` modified to include `Fatal` constant
- [ ] No other files changed
- [ ] Plan/requirement traceability present in code comment
- [ ] Tests compile and run

## Semantic Verification Checklist (Mandatory)
- [ ] `LogLevel::Fatal` resolves to the same value as `LogLevel::User` (both == 1)
- [ ] Existing tests still pass — no behavioral change
- [ ] No placeholder/deferred implementation patterns remain

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/logging.rs
```

## Success Criteria
- [ ] `LogLevel::Fatal` constant exists and equals `LogLevel::User`
- [ ] All verification commands pass
- [ ] No regressions

## Failure Recovery
- Rollback: `git checkout rust/src/logging.rs`
- Blocking issues: if `impl LogLevel` block doesn't exist, add one

## Phase Completion Marker
Create: `project-plans/memandres/memory/.completed/P03.md`

Contents:
- phase ID
- timestamp
- files changed: `rust/src/logging.rs`
- tests added/updated: none (stub phase)
- verification outputs
- semantic verification summary
