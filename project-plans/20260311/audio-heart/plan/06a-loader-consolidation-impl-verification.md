# Phase 06a: Loader Consolidation Implementation — Verification

## Phase ID
`PLAN-20260314-AUDIO-HEART.P06a`

## Prerequisites
- Required: Phase P06 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | head -50
cargo test --workspace --all-features

# Verify single canonical path
grep -rn 'load_music_canonical\|load_sound_bank_canonical' rust/src/sound/
# Expected: defined in loading.rs, called from music.rs, sfx.rs (indirectly via fileinst or directly)

# Verify old inline loaders removed
grep -c 'uio_fread.*buf\|decode_all\|mixer_buffer_data' rust/src/sound/heart_ffi.rs
# Should be 0 or minimal (only in loading.rs now)
```

## Structural Verification Checklist
- [ ] `loading.rs` — no `todo!()` remaining
- [ ] `music.rs` — `get_music_data` delegates to canonical loader
- [ ] `sfx.rs` — `get_sound_bank_data` delegates to canonical loader
- [ ] `heart_ffi.rs` — `LoadMusicFile` and `LoadSoundFile` route through fileinst
- [ ] No duplicate loading implementations exist

## Semantic Verification Checklist
- [ ] Music loading produces a decoder-attached MusicRef
- [ ] Bank loading produces a SoundBank with mixer-buffered samples
- [ ] Empty filename returns appropriate error
- [ ] Missing file returns ResourceNotFound
- [ ] Unknown extension returns appropriate error
- [ ] Integration tests pass (if available)

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/sound/loading.rs rust/src/sound/music.rs rust/src/sound/sfx.rs
```

## Success Criteria
- [ ] All verification commands pass
- [ ] Single canonical loading path verified
- [ ] No stubs remaining in music.rs or sfx.rs internal loaders

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P06a.md`
