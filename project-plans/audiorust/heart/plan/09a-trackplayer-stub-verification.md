# Phase 09a: Track Player Stub Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P09a`

## Prerequisites
- Required: Phase P09 completed
- Expected files: `rust/src/sound/trackplayer.rs`, updated `mod.rs`

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo check --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm
```

## Structural Verification Checklist
- [ ] `rust/src/sound/trackplayer.rs` exists
- [ ] `mod.rs` updated with `pub mod trackplayer;`
- [ ] `@plan PLAN-20260225-AUDIO-HEART.P09` marker present
- [ ] `cargo check` passes
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes

## Semantic Verification Checklist

### Deterministic checks
- [ ] 17+ public functions have signatures: `grep -c "pub fn\|pub(crate) fn" rust/src/sound/trackplayer.rs` >= 17
- [ ] SoundChunk linked list compiles: `grep -c "SoundChunk" rust/src/sound/trackplayer.rs` >= 3
- [ ] TrackCallbacks implements StreamCallbacks: `grep -c "impl StreamCallbacks" rust/src/sound/trackplayer.rs` >= 1
- [ ] Imports from stream.rs and types.rs work: `grep -c "use crate::sound::stream\|use crate::sound::types\|use super" rust/src/sound/trackplayer.rs` >= 2

### Subjective checks
- [ ] SoundChunk has `Box<dyn SoundDecoder>` field — is the decoder properly owned by the chunk?
- [ ] TrackPlayerState has raw pointer fields (chunks_tail, cur_chunk, cur_sub_chunk) — are they declared with correct types (`*mut SoundChunk`, `Option<NonNull<SoundChunk>>`)?
- [ ] `unsafe impl Send` present for TrackPlayerState with detailed `// SAFETY:` documentation explaining ownership invariant, single-writer invariant, lifetime invariant, and callback invariant
- [ ] `NonNull<SoundChunk>` used for cur_chunk/cur_sub_chunk — not bare `*mut`
- [ ] `AtomicU32` used for tracks_length (for lock-free position queries)
- [ ] TRACK_STATE uses `parking_lot::Mutex` (not `std::sync::Mutex`)

### GIVEN/WHEN/THEN contracts
- GIVEN the trackplayer module is compiled, WHEN heart_ffi.rs imports `splice_track`, `play_track`, `stop_track`, THEN it compiles successfully
- GIVEN SoundChunk is defined, WHEN a `Box<SoundChunk>` is created with a NullDecoder, THEN it compiles
- GIVEN TrackCallbacks is defined, WHEN it is passed as `Box<dyn StreamCallbacks>` to `set_sound_sample_callbacks`, THEN it compiles

## Deferred Implementation Detection

```bash
grep -n "todo!()" rust/src/sound/trackplayer.rs | wc -l
# Should be > 0 (stubs exist)
grep -n "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/sound/trackplayer.rs
# Should return 0 results
```

## Success Criteria
- [ ] All signatures compile
- [ ] Module registered in mod.rs
- [ ] Linked list type compiles
- [ ] Lifetime safety documentation present
- [ ] C build not broken

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/mod.rs` and `rm rust/src/sound/trackplayer.rs`
- blocking issues: If stream.rs signatures need adjustment, fix stream.rs first

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P09a.md`
