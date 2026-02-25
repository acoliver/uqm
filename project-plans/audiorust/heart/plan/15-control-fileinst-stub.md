# Phase 15: Control + FileInst Stub

## Phase ID
`PLAN-20260225-AUDIO-HEART.P15`

## Prerequisites
- Required: Phase P14a (Music + SFX Implementation Verification) passed
- Expected files: `music.rs`, `sfx.rs` fully implemented

## Requirements Implemented (Expanded)

### REQ-VOLUME-INIT-01..05: Initialization Stubs
**Requirement text**: SoundSourceArray with mixer source handles; VolumeState with defaults.

Behavior contract:
- GIVEN: Mixer provides mixer_gen_sources
- WHEN: control.rs is created
- THEN: Source array and volume state compile

### REQ-VOLUME-CONTROL-01..05: Volume Control Stubs
### REQ-VOLUME-SOURCE-01..04: Source Management Stubs
### REQ-VOLUME-QUERY-01..03: Query Stubs
### REQ-FILEINST-LOAD-01..07: File Loading Stubs

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
1. `SoundSourceArray` struct with `[Mutex<SoundSource>; NUM_SOUNDSOURCES]`
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
- [ ] SoundSourceArray compiles with array of Mutex<SoundSource>
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
