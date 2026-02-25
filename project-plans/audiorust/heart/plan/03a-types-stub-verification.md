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
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm
```

## Structural Verification Checklist
- [ ] `rust/src/sound/types.rs` exists
- [ ] `mod.rs` contains `pub mod types;`
- [ ] `types.rs` has `@plan PLAN-20260225-AUDIO-HEART.P03` marker
- [ ] All compilation passes (`cargo check`)
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy` passes with no warnings
- [ ] `build.sh uqm` succeeds (no regressions)

## Semantic Verification Checklist

### Deterministic checks
- [ ] `use crate::sound::types::AudioError;` compiles from another module
- [ ] `use crate::sound::types::{NUM_SFX_CHANNELS, MUSIC_SOURCE, SPEECH_SOURCE};` compiles
- [ ] `AudioError::from(MixerError::NoError)` compiles
- [ ] `SoundPosition::NON_POSITIONAL` exists and is const
- [ ] `MusicRef` is `#[repr(transparent)]` wrapping a raw pointer
- [ ] `SoundSample` struct has `looping: bool` field (set_looping resolution)
- [ ] `decode_all` function signature exists (may have `todo!()` body)
- [ ] `get_decoder_time` function signature exists (may have `todo!()` body)
- [ ] `parking_lot::Mutex` used (not `std::sync::Mutex`) — verify: `grep -c "std::sync::Mutex" rust/src/sound/types.rs` returns 0

### Subjective checks
- [ ] AudioError has exactly 14 variants matching spec §3.1 — are all error conditions that can arise in the audio pipeline represented?
- [ ] All From conversions compile (From<MixerError>, From<DecodeError>) — do they map to the correct AudioError variants?
- [ ] All 8 constant groups match spec values — NUM_SFX_CHANNELS=5, MAX_VOLUME=255, NORMAL_VOLUME=160, etc.
- [ ] SoundSample has all fields from spec §3.3 — decoder, length, buffers, num_buffers, buffer_tags, offset, data, callbacks, looping
- [ ] StreamCallbacks trait has 5 methods with correct signatures and default no-op implementations
- [ ] SoundPosition is `#[repr(C)]` with positional, x, y fields — suitable for FFI

## Deferred Implementation Detection

```bash
# Only todo!() allowed in decode_all and get_decoder_time stubs
grep -n "todo!()" rust/src/sound/types.rs
# Should show exactly 2 occurrences (decode_all and get_decoder_time)
grep -n "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/sound/types.rs
# Should return 0 results
```

## Success Criteria
- [ ] All types compile
- [ ] Module registered in mod.rs
- [ ] Constants accessible from other modules
- [ ] Error types have correct conversions
- [ ] SoundSample has looping field
- [ ] C build not broken

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/mod.rs rust/src/sound/types.rs`
- blocking issues: If MixerError or DecodeError signatures have changed, update From impls first

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P03a.md`
