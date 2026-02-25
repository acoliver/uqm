# Phase 15: Control + FileInst Stub

## Phase ID
`PLAN-20260225-AUDIO-HEART.P15`

## Prerequisites
- Required: Phase P14a (Music + SFX Implementation Verification) passed
- Expected files: `music.rs`, `sfx.rs` fully implemented

## Requirements Implemented (Expanded)

### REQ-VOLUME-INIT-01 through REQ-VOLUME-INIT-05: Initialization Stubs
**Requirement text**: SoundSourceArray with mixer source handles; VolumeState with defaults.

Behavior contract:
- GIVEN: Mixer provides mixer_gen_sources
- WHEN: control.rs is created
- THEN: Source array and volume state compile

### REQ-VOLUME-CONTROL-01 through REQ-VOLUME-CONTROL-05: Volume Control Stubs
Behavior contract:
- GIVEN: Mixer provides `mixer_source_f(handle, SourceProp::Gain, gain)`
- WHEN: `set_sfx_volume`, `set_speech_volume` stubs are defined
- THEN: They accept volume as i32 and return `AudioResult<()>`

### REQ-VOLUME-SOURCE-01 through REQ-VOLUME-SOURCE-04: Source Management Stubs
Behavior contract:
- GIVEN: Mixer provides `mixer_source_stop`, `mixer_source_rewind`
- WHEN: `stop_source`, `clean_source`, `stop_sound` stubs are defined
- THEN: They accept source_index as usize and return `AudioResult<()>`

### REQ-VOLUME-QUERY-01 through REQ-VOLUME-QUERY-03: Query Stubs
Behavior contract:
- GIVEN: SoundSourceArray exists with per-source mutex
- WHEN: `sound_playing`, `wait_for_sound_end` stubs are defined
- THEN: `sound_playing` returns `bool`, `wait_for_sound_end` returns `AudioResult<()>`

### REQ-FILEINST-LOAD-01 through REQ-FILEINST-LOAD-07: File Loading Stubs
Behavior contract:
- GIVEN: uio_* FFI functions for file I/O
- WHEN: `load_sound_file`, `load_music_file`, `destroy_sound`, `destroy_music` stubs are defined
- THEN: Load functions accept filename and return `AudioResult<*mut c_void>`, destroy functions accept handle and return `AudioResult<()>`

## Implementation Tasks

### Files to create
- `rust/src/sound/control.rs` — All public API from spec §3.5
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P15`
  - marker: `@requirement REQ-VOLUME-*`
- `rust/src/sound/fileinst.rs` — All public API from spec §3.6
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P15`
  - marker: `@requirement REQ-FILEINST-*`

### Files to modify
- `rust/src/sound/mod.rs` — Add `pub mod control;` and `pub mod fileinst;`
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P15`

### control.rs stub contents
1. `SoundSourceArray` struct with `[parking_lot::Mutex<SoundSource>; NUM_SOUNDSOURCES]`
2. `VolumeState` struct
3. `lazy_static!` for `SOURCES` (pub(crate)) and `VOLUME`
4. All public functions with `todo!()`:
   - `stop_source`, `clean_source`, `stop_sound`
   - `set_sfx_volume`, `set_speech_volume`
   - `sound_playing`, `wait_for_sound_end`
   - `init_sound`, `uninit_sound`

### fileinst.rs stub contents
1. `FileInstState` struct
2. `FileLoadGuard` struct (RAII)
3. `lazy_static!` for `FILE_STATE`
4. All public functions with `todo!()`:
   - `load_sound_file`, `load_music_file`
   - `destroy_sound`, `destroy_music`

### Circular dependency resolution
- `control.rs` provides `SOURCES` used by `stream.rs`, `music.rs`, `sfx.rs`, `trackplayer.rs`
- This may require re-exporting `SOURCES` or adjusting module visibility
- Consider defining `SOURCES` in `control.rs` and importing everywhere else

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo check --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] `control.rs` and `fileinst.rs` created
- [ ] `mod.rs` updated
- [ ] SoundSourceArray compiles with array of `parking_lot::Mutex<SoundSource>`
- [ ] FileLoadGuard implements Drop
- [ ] `cargo check` passes

## Semantic Verification Checklist (Mandatory)
- [ ] SOURCES is `pub(crate)` — accessible to stream, trackplayer, music, sfx
- [ ] VolumeState defaults match spec (NORMAL_VOLUME, 1.0 scales)
- [ ] FileInstState has cur_resfile_name: Option<String>
- [ ] Import paths between modules resolve correctly

## Deferred Implementation Detection (Mandatory)

```bash
grep -n "todo!()" rust/src/sound/control.rs | wc -l   # Expected > 0
grep -n "todo!()" rust/src/sound/fileinst.rs | wc -l  # Expected > 0
```

## Success Criteria
- [ ] Both modules compile
- [ ] SOURCES accessible from other heart modules
- [ ] Module dependency graph acyclic

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/mod.rs` and remove new files
- blocking issues: If circular dependency arises, restructure SOURCES location

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P15.md`
