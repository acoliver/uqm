# Phase 20a: FFI Implementation Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P20a`

## Prerequisites
- Required: Phase P20 completed
- Expected: heart_ffi.rs fully implemented

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::heart_ffi::tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
grep -RIn "TODO\|FIXME\|HACK\|todo!()" rust/src/sound/heart_ffi.rs
# Symbol count
cd /Users/acoliver/projects/uqm/rust && cargo build --lib --all-features
nm target/debug/libuqm_rust.a 2>/dev/null | grep " T " | grep -c "rust_"
```

## Checks
- [ ] All FFI tests pass (17+)
- [ ] All workspace tests pass
- [ ] Zero deferred markers
- [ ] fmt and clippy pass
- [ ] 60+ symbols exported
- [ ] All unsafe blocks documented

## Gate Decision
- [ ] PASS: proceed to P21
- [ ] FAIL: fix FFI implementation
