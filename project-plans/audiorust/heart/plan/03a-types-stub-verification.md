# Phase 03a: Types Stub Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P03a`

## Prerequisites
- Required: Phase P03 completed
- Expected files: `rust/src/sound/types.rs`, updated `rust/src/sound/mod.rs`

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo check --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Checks
- [ ] `rust/src/sound/types.rs` exists
- [ ] `mod.rs` contains `pub mod types;`
- [ ] `types.rs` has `@plan PLAN-20260225-AUDIO-HEART.P03` marker
- [ ] All compilation passes

## Semantic Checks
- [ ] `use crate::sound::types::AudioError;` compiles from another module
- [ ] `use crate::sound::types::{NUM_SFX_CHANNELS, MUSIC_SOURCE, SPEECH_SOURCE};` compiles
- [ ] `AudioError::from(MixerError::NoError)` compiles
- [ ] `SoundPosition::NON_POSITIONAL` exists and is const
- [ ] `MusicRef` is repr(transparent) wrapping a raw pointer

## Gate Decision
- [ ] PASS: proceed to P04
- [ ] FAIL: fix types.rs
