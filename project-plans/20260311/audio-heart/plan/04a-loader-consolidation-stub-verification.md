# Phase 04a: Loader Consolidation Stubs — Verification

## Phase ID
`PLAN-20260314-AUDIO-HEART.P04a`

## Prerequisites
- Required: Phase P04 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | head -50
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust/src/sound/loading.rs` exists
- [ ] `rust/src/sound/mod.rs` contains `pub mod loading`
- [ ] `loading.rs` exports `load_music_canonical` and `load_sound_bank_canonical`
- [ ] Both functions return `AudioResult<MusicRef>` and `AudioResult<SoundBank>` respectively
- [ ] UIO FFI extern declarations present
- [ ] Plan/requirement traceability markers present

## Semantic Verification Checklist
- [ ] No existing tests broken
- [ ] No existing behavior changed
- [ ] Stub functions are not yet called from production paths

## Success Criteria
- [ ] Verification commands pass
- [ ] Module structure is correct

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P04a.md`
