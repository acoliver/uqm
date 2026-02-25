# Phase 12a: Music + SFX Stub Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P12a`

## Prerequisites
- Required: Phase P12 completed
- Expected files: `rust/src/sound/music.rs`, `rust/src/sound/sfx.rs`, updated `mod.rs`

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo check --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm
```

## Structural Verification Checklist
- [ ] `rust/src/sound/music.rs` exists
- [ ] `rust/src/sound/sfx.rs` exists
- [ ] `mod.rs` updated with `pub mod music;` and `pub mod sfx;`
- [ ] `@plan PLAN-20260225-AUDIO-HEART.P12` markers present
- [ ] `cargo check` passes
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes

## Semantic Verification Checklist

### Deterministic checks
- [ ] music.rs has 12+ public function signatures: `grep -c "pub fn\|pub(crate) fn" rust/src/sound/music.rs` >= 12
- [ ] sfx.rs has 10+ public function signatures: `grep -c "pub fn\|pub(crate) fn" rust/src/sound/sfx.rs` >= 10
- [ ] MusicRef type used in music.rs: `grep -c "MusicRef" rust/src/sound/music.rs` >= 2
- [ ] SoundBank type used in sfx.rs: `grep -c "SoundBank" rust/src/sound/sfx.rs` >= 2
- [ ] SoundPosition imported in sfx.rs: `grep -c "SoundPosition" rust/src/sound/sfx.rs` >= 1
- [ ] `parking_lot::Mutex` used (not bare `Mutex`): `grep -c "parking_lot" rust/src/sound/sfx.rs` >= 1

### Subjective checks
- [ ] Music API surface matches spec — does music.rs have plr_play_song, plr_stop, plr_playing, plr_seek, plr_pause, plr_resume, snd_play_speech, snd_stop_speech, get_music_data, release_music_data, set_music_volume, fade_music?
- [ ] SFX API surface matches spec — does sfx.rs have play_channel, stop_channel, channel_playing, set_channel_volume, check_finished_channels, update_sound_position, get/set_positional_object, get_sound_bank_data, release_sound_bank_data?
- [ ] 3D positioning approach documented — does sfx.rs include a comment about using 3 separate `mixer_source_f` calls instead of `mixer_source_fv`?
- [ ] Module imports from stream.rs and types.rs are correct paths
- [ ] All stubs use `todo!()` — no fake success behavior

### GIVEN/WHEN/THEN contracts
- GIVEN the music module is compiled, WHEN heart_ffi.rs imports `plr_play_song`, THEN it compiles successfully
- GIVEN the sfx module is compiled, WHEN heart_ffi.rs imports `play_channel`, THEN it compiles successfully
- GIVEN the sfx module references `SoundPosition`, WHEN it is imported from types.rs, THEN it compiles

## Deferred Implementation Detection

```bash
grep -n "todo!()" rust/src/sound/music.rs | wc -l
grep -n "todo!()" rust/src/sound/sfx.rs | wc -l
# Both should be > 0 (stubs)
grep -n "TODO\|FIXME\|HACK\|placeholder" rust/src/sound/music.rs rust/src/sound/sfx.rs
# Should return 0 results
```

## Success Criteria
- [ ] All signatures compile
- [ ] Both modules registered in mod.rs
- [ ] Importable from other modules
- [ ] 3D positioning approach documented
- [ ] C build not broken

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/mod.rs` and `rm rust/src/sound/music.rs rust/src/sound/sfx.rs`
- blocking issues: If stream.rs or types.rs signatures need adjustment, fix upstream first

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P12a.md`
