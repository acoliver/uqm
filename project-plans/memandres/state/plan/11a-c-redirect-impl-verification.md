# Phase 11a: C Redirect — Implementation Verification

## Phase ID
`PLAN-20260224-STATE-SWAP.P11a`

## Prerequisites
- Required: Phase P11 completed
- Expected: USE_RUST_STATE enabled, full build succeeds

## Build Verification
```bash
cd rust && cargo build --release && cd ../sc2 && make clean && make
```
- [ ] Build succeeds

## Test Verification
```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```
- [ ] All quality gates pass
- [ ] All Rust tests pass

## Runtime Verification
- [ ] Game launches without crash
- [ ] Title screen displays
- [ ] New game starts (calls OpenStateFile → Rust)
- [ ] Enter a star system (calls GetPlanetInfo → Rust)
- [ ] Exit star system (calls PutPlanetInfo → Rust)
- [ ] Save game (state files serialized via Rust)
- [ ] Quit and reload save (state files deserialized via Rust)
- [ ] Game state matches pre-save state

## Deferred Implementation Detection
```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/state/ || echo "CLEAN"
```
- [ ] No deferred markers

## Config Verification
```bash
grep "USE_RUST_STATE" sc2/config_unix.h
```
- [ ] `#define USE_RUST_STATE` is enabled (not commented)

## Gate Decision
- [ ] PASS: proceed to P12
- [ ] FAIL: fix issues or rollback to C path
