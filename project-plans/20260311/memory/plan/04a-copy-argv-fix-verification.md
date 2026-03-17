# Phase 04a: `copy_argv_to_c` Fix Verification

## Phase ID
`PLAN-20260314-MEMORY.P04a`

## Prerequisites
- Required: Phase P04 completed
- Expected artifacts: Modified `rust/src/memory.rs` with fixed `copy_argv_to_c` and test

## Verification Commands

```bash
# Full quality gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Targeted test
cargo test -p uqm memory::tests::test_copy_argv_to_c -- --nocapture

# Confirm no libc::free on CString pointers
grep -n "libc::free" rust/src/memory.rs

# Deferred implementation detection
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/memory.rs
```

## Structural Verification Checklist
- [ ] No `libc::free` calls remain in `copy_argv_to_c` or its test
- [ ] `CString::from_raw` is used for string pointer cleanup in test
- [ ] Dead null-check branch is removed from `copy_argv_to_c`
- [ ] Doc comment documents allocator-family ownership
- [ ] Plan/requirement markers present

## Semantic Verification Checklist (Mandatory)
- [ ] `test_copy_argv_to_c` passes with correct cleanup
- [ ] String content verification unchanged (program, arg1, arg2)
- [ ] Null-termination verification unchanged
- [ ] Array pointer freed via `rust_hfree`
- [ ] All other tests still pass

## Gate Decision
- [ ] PASS: proceed to Phase 05
- [ ] FAIL: fix issues and re-verify
