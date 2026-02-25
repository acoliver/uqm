# Phase 12a: Music + SFX Stub Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P12a`

## Prerequisites
- Required: Phase P12 completed
- Expected files: `music.rs`, `sfx.rs`, updated `mod.rs`

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo check --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Checks
- [ ] Both files compile
- [ ] music.rs has 13+ function signatures
- [ ] sfx.rs has 11+ function signatures
- [ ] Correct imports from stream.rs and types.rs
- [ ] MusicState and SfxState defined

## Gate Decision
- [ ] PASS: proceed to P13
- [ ] FAIL: fix stubs
