# Phase 04a: Rust Fixes — TDD Verification

## Phase ID
`PLAN-20260224-MEM-SWAP.P04a`

## Prerequisites
- Required: Phase 04 completed
- `test_fatal_alias` added to `logging.rs`

## Verification Checks

### Structural
- [ ] `test_fatal_alias` exists in `rust/src/logging.rs` test module
- [ ] Test has `@plan` and `@requirement` markers
- [ ] Test asserts both equality and numeric value

### Semantic
- [ ] `cargo test -p uqm_rust test_fatal_alias` passes
- [ ] `cargo test --workspace --all-features` passes (no regressions)
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- [ ] `cargo fmt --all --check` passes

### TDD Discipline
- [ ] Test would fail if `Fatal` alias pointed to a different variant

## Verification Commands

```bash
cargo test -p uqm_rust test_fatal_alias -- --nocapture
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Gate Decision
- [ ] PASS: proceed to Phase 05
- [ ] FAIL: fix test or alias in Phase 03/04
