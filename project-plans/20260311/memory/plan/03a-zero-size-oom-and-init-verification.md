# Phase 03a: Zero-Size OOM + Unit-Test Gap Closure Verification

## Phase ID
`PLAN-20260314-MEMORY.P03a`

## Prerequisites
- Required: Phase P03 completed
- Expected artifacts: Modified `rust/src/memory.rs` with OOM checks and additional spec-required unit tests

## Verification Commands

```bash
# Full quality gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Targeted tests
cargo test -p uqm memory::tests -- --nocapture

# Deferred implementation detection
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/memory.rs
```

## Structural Verification Checklist
- [ ] `rust_hmalloc` zero-size path has null check + log + abort
- [ ] `rust_hcalloc` zero-size path has null check + log + abort
- [ ] `rust_hrealloc` zero-size path has null check + log + abort
- [ ] `rust_hrealloc` positive-size path no longer has redundant `&& size > 0`
- [ ] `test_null_free_is_safe` exists and passes
- [ ] `test_realloc_null_ptr_acts_as_malloc` exists and passes
- [ ] Plan markers present in modified code

## Semantic Verification Checklist (Mandatory)
- [ ] The zero-size OOM paths are unreachable in normal operation but structurally correct
- [ ] `HFree(NULL)` is explicitly covered by a unit test and behaves as a safe no-op
- [ ] `HRealloc(NULL, size)` is explicitly covered by a unit test and behaves like allocation
- [ ] All pre-existing tests pass unchanged
- [ ] No tests verify only implementation internals

## Gate Decision
- [ ] PASS: proceed to Phase 04
- [ ] FAIL: fix issues and re-verify
