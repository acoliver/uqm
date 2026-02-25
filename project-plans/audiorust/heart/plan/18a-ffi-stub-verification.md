# Phase 18a: FFI Stub Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P18a`

## Prerequisites
- Required: Phase P18 completed
- Expected files: `heart_ffi.rs`, updated `mod.rs`

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo check --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
# Verify symbol export
cd /Users/acoliver/projects/uqm/rust && cargo build --lib --all-features 2>&1 | head -5
nm target/debug/libuqm_rust.a 2>/dev/null | grep -c "InitStreamDecoder\|PLRPlaySong\|PlayChannel"
```

## Checks
- [ ] `heart_ffi.rs` compiles
- [ ] 60+ `#[no_mangle]` functions present
- [ ] All use `extern "C"`
- [ ] CCallbackWrapper compiles
- [ ] Symbols visible in static library (nm check)

## Gate Decision
- [ ] PASS: proceed to P19
- [ ] FAIL: fix FFI stub
