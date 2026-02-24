# Phase 10a: C Redirect — TDD Verification

## Phase ID
`PLAN-20260224-STATE-SWAP.P10a`

## Prerequisites
- Required: Phase P10 completed
- Expected: both build configurations tested

## Build Verification
- [ ] C path (USE_RUST_STATE off): `make` succeeds
- [ ] Rust library: `cargo build --release` succeeds
- [ ] Rust path (USE_RUST_STATE on): `make` succeeds (links to Rust lib)
- [ ] `nm` output shows all 7 `rust_*state*` symbols in static lib

## Symbol Verification
```bash
nm -g rust/target/release/libuqm_rust.a 2>/dev/null | grep -c "rust_.*state"
# Expected: at least 7 (open, close, delete, length, read, write, seek)
```
- [ ] All 7 symbols present

## Test Verification
```bash
cd rust && cargo test --workspace --all-features
```
- [ ] All Rust tests pass

## Config State
- [ ] `USE_RUST_STATE` is DISABLED in `config_unix.h` after testing (safe default)

## Gate Decision
- [ ] PASS: proceed to P11
- [ ] FAIL: fix build issues
