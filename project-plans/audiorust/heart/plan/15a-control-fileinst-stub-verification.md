# Phase 15a: Control + FileInst Stub Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P15a`

## Prerequisites
- Required: Phase P15 completed
- Expected files: `rust/src/sound/control.rs`, `rust/src/sound/fileinst.rs`, updated `mod.rs`

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo check --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm
```

## Structural Verification Checklist
- [ ] `rust/src/sound/control.rs` exists
- [ ] `rust/src/sound/fileinst.rs` exists
- [ ] `mod.rs` updated with `pub mod control;` and `pub mod fileinst;`
- [ ] `@plan PLAN-20260225-AUDIO-HEART.P15` markers present
- [ ] `cargo check` passes
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes

## Semantic Verification Checklist

### Deterministic checks
- [ ] control.rs has 10+ public function signatures: `grep -c "pub fn\|pub(crate) fn" rust/src/sound/control.rs` >= 10
- [ ] fileinst.rs has 5+ public function signatures: `grep -c "pub fn\|pub(crate) fn" rust/src/sound/fileinst.rs` >= 5
- [ ] SoundSourceArray uses `parking_lot::Mutex`: `grep -c "parking_lot" rust/src/sound/control.rs` >= 1
- [ ] FileInstState has concurrency guard: `grep -c "cur_resfile_name\|FileLoadGuard\|ConcurrentLoad" rust/src/sound/fileinst.rs` >= 1
- [ ] VolumeState struct defined: `grep -c "VolumeState" rust/src/sound/control.rs` >= 1

### Subjective checks
- [ ] control.rs covers init/uninit, stop/clean_source, volume control, sound_playing, wait_for_sound_end — is the full control API surface present?
- [ ] fileinst.rs has FileLoadGuard (RAII) pattern — does it use a guard struct to ensure `cur_resfile_name` is cleared on drop?
- [ ] SoundSourceArray has NUM_SOUNDSOURCES entries — is it the right array size?
- [ ] VolumeState has all volume fields: music_volume, music_volume_scale, sfx_volume_scale, speech_volume_scale
- [ ] All stubs use `todo!()` — no fake success behavior

### GIVEN/WHEN/THEN contracts
- GIVEN the control module is compiled, WHEN music.rs imports `stop_source`, `clean_source`, THEN it compiles
- GIVEN the fileinst module is compiled, WHEN heart_ffi.rs imports `load_sound_file`, `load_music_file`, THEN it compiles
- GIVEN VolumeState is defined, WHEN it has default volume values, THEN `VolumeState::new()` returns correct defaults

## Deferred Implementation Detection

```bash
grep -n "todo!()" rust/src/sound/control.rs | wc -l
grep -n "todo!()" rust/src/sound/fileinst.rs | wc -l
# Both should be > 0 (stubs)
grep -n "TODO\|FIXME\|HACK\|placeholder" rust/src/sound/control.rs rust/src/sound/fileinst.rs
# Should return 0 results
```

## Success Criteria
- [ ] All signatures compile
- [ ] Both modules registered in mod.rs
- [ ] SoundSourceArray compiles with array of `parking_lot::Mutex<SoundSource>`
- [ ] FileLoadGuard pattern established
- [ ] C build not broken

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/mod.rs` and `rm rust/src/sound/control.rs rust/src/sound/fileinst.rs`
- blocking issues: If types.rs needs new types, add to types.rs first

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P15a.md`
