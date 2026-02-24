# Phase 07a: C Header Redirect — TDD Verification

## Phase ID
`PLAN-20260224-MEM-SWAP.P07a`

## Prerequisites
- Required: Phase 07 completed
- All 3 build tests executed

## Verification Checks

### Build Path Tests
- [ ] C path (flag off): clean build succeeded with exit code 0
- [ ] Rust path (flag on): clean build succeeded with exit code 0
- [ ] `#error` guard: compilation of `w_memlib.c` with `-DUSE_RUST_MEM` failed with expected error

### State After Phase
- [ ] `config_unix.h` has `USE_RUST_MEM` still commented out (flag OFF)
- [ ] No residual build artifacts from Rust-path test

### No Regressions
- [ ] `cargo test --workspace --all-features` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- [ ] `cargo fmt --all --check` passes

## Verification Commands

```bash
# Confirm flag is still off
grep 'USE_RUST_MEM' sc2/config_unix.h

# Rust checks
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Gate Decision
- [ ] PASS: proceed to Phase 08
- [ ] FAIL: fix build issues before proceeding
