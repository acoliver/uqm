# Phase 06a: Stream Stub Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P06a`

## Prerequisites
- Required: Phase P06 completed
- Expected files: `rust/src/sound/stream.rs`, updated `mod.rs`

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo check --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Checks
- [ ] `stream.rs` compiles
- [ ] All 19+ public functions have signatures
- [ ] StreamEngine struct has all fields
- [ ] Module importable: `use crate::sound::stream::play_stream;` etc.
- [ ] Correct use of `parking_lot::Mutex`, `parking_lot::Condvar`
- [ ] `AtomicBool` used for shutdown flag
- [ ] No fake success behavior (all stubs use `todo!()`)

## Gate Decision
- [ ] PASS: proceed to P07
- [ ] FAIL: fix stream stub
