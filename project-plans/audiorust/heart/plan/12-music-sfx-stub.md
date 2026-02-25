# Phase 12: Music + SFX Stub

## Phase ID
`PLAN-20260225-AUDIO-HEART.P12`

## Prerequisites
- Required: Phase P11a (Track Player Implementation Verification) passed
- Expected files: `stream.rs` and `trackplayer.rs` fully implemented

## Requirements Implemented (Expanded)

### REQ-MUSIC-PLAY-01 through REQ-MUSIC-PLAY-08: Music Playback Stubs
**Requirement text**: Music play/stop/pause/resume/seek through MUSIC_SOURCE.

Behavior contract:
- GIVEN: stream.rs provides play_stream/stop_stream/etc.
- WHEN: music.rs function signatures are created
- THEN: All music API functions compile with `todo!()` bodies

### REQ-MUSIC-SPEECH-01, REQ-MUSIC-SPEECH-02: Speech Stubs
Behavior contract:
- GIVEN: stream.rs provides `play_stream`/`stop_stream` on SPEECH_SOURCE
- WHEN: `snd_play_speech`, `snd_stop_speech` stubs are defined
- THEN: They accept a MusicRef parameter and compile with `todo!()` bodies

### REQ-MUSIC-LOAD-01 through REQ-MUSIC-LOAD-06: Music Loading Stubs
Behavior contract:
- GIVEN: Decoder types exist in `sound::decoder`
- WHEN: `get_music_data`, `check_music_res_name` stubs are defined
- THEN: They accept filename/decoder parameters and return `AudioResult<MusicRef>`

### REQ-MUSIC-RELEASE-01 through REQ-MUSIC-RELEASE-04: Music Release Stubs
Behavior contract:
- GIVEN: MusicRef wraps a raw pointer to SoundSample
- WHEN: `release_music_data` stub is defined
- THEN: It accepts `MusicRef` and returns `AudioResult<()>`

### REQ-MUSIC-VOLUME-01: Music Volume Stub
Behavior contract:
- GIVEN: Volume is an i32 (0..MAX_VOLUME)
- WHEN: `set_music_volume` and `fade_music` stubs are defined
- THEN: They accept volume/time parameters and compile

### REQ-SFX-PLAY-01 through REQ-SFX-PLAY-09: SFX Playback Stubs
Behavior contract:
- GIVEN: SoundBank and SoundSource exist
- WHEN: `play_channel`, `stop_channel`, `channel_playing`, `set_channel_volume`, `check_finished_channels` stubs are defined
- THEN: They accept channel index / volume parameters and compile

### REQ-SFX-POSITION-01 through REQ-SFX-POSITION-05: Positional Audio Stubs
Behavior contract:
- GIVEN: SoundPosition is `#[repr(C)]` with positional, x, y
- WHEN: `update_sound_position`, `get_positional_object`, `set_positional_object` stubs are defined
- THEN: They accept source_index and SoundPosition parameters and compile

### REQ-SFX-VOLUME-01: SFX Volume Stub
Behavior contract:
- GIVEN: Volume is an i32 (0..MAX_VOLUME)
- WHEN: `set_sfx_volume` stub is defined
- THEN: It compiles with volume parameter

### REQ-SFX-LOAD-01 through REQ-SFX-LOAD-07: SFX Loading Stubs
Behavior contract:
- GIVEN: File I/O via uio_* FFI functions
- WHEN: `get_sound_bank_data` stub is defined
- THEN: It accepts a filename and returns `AudioResult<SoundBank>`

### REQ-SFX-RELEASE-01 through REQ-SFX-RELEASE-04: SFX Release Stubs
Behavior contract:
- GIVEN: SoundBank owns Vec<Option<SoundSample>>
- WHEN: `release_sound_bank_data` stub is defined
- THEN: It accepts SoundBank and returns `AudioResult<()>`

## Implementation Tasks

### Files to create
- `rust/src/sound/music.rs` — All public API from spec §3.3
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P12`
  - marker: `@requirement REQ-MUSIC-*`
- `rust/src/sound/sfx.rs` — All public API from spec §3.4
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P12`
  - marker: `@requirement REQ-SFX-*`

### Files to modify
- `rust/src/sound/mod.rs` — Add `pub mod music;` and `pub mod sfx;`
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P12`

### music.rs stub contents
1. `MusicRef` (re-exported from types or defined here with repr(transparent))
2. `MusicState` struct (cur_music_ref, cur_speech_ref, music_volume, music_volume_scale)
3. `lazy_static! { static ref MUSIC_STATE }` (or `OnceLock`)
4. All public functions with `todo!()`:
   - `plr_play_song`, `plr_stop`, `plr_playing`, `plr_seek`, `plr_pause`, `plr_resume`
   - `snd_play_speech`, `snd_stop_speech`
   - `get_music_data`, `release_music_data`, `check_music_res_name`
   - `set_music_volume`, `fade_music`

### sfx.rs stub contents
1. `SoundPosition` struct (re-exported from types)
2. `SoundBank` struct (re-exported from types)
3. `SfxState` struct
4. `lazy_static! { static ref SFX_STATE }` (or `OnceLock`)
5. Constants: `ATTENUATION`, `MIN_DISTANCE`, `MAX_FX`
6. All public functions with `todo!()`:
   - `play_channel`, `stop_channel`, `channel_playing`, `set_channel_volume`, `check_finished_channels`
   - `update_sound_position`, `get_positional_object`, `set_positional_object`
   - `get_sound_bank_data`, `release_sound_bank_data`

### 3D Positioning Approach (rust-heart.md Action Item #4 — REQ-SFX-POSITION-01)

The mixer module does NOT have `mixer_source_fv()` (vector setter). Instead of modifying the mixer, `update_sound_position` shall use three separate `mixer_source_f` calls to set X, Y, Z position components individually:
```rust
mixer_source_f(handle, SourceProp::Position, x)?;  // X component
mixer_source_f(handle, SourceProp::Position, y)?;  // Y component
mixer_source_f(handle, SourceProp::Position, z)?;  // Z component
```
Document this design decision in `sfx.rs` with a comment referencing this plan note.

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo check --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] `music.rs` and `sfx.rs` created
- [ ] `mod.rs` updated with both modules
- [ ] All function signatures present
- [ ] `cargo check` passes

## Semantic Verification Checklist (Mandatory)
- [ ] music.rs imports from stream.rs (play_stream, stop_stream, etc.)
- [ ] sfx.rs imports from control (stop_source, clean_source)
- [ ] MusicRef wraps a raw pointer
- [ ] SoundPosition is repr(C)
- [ ] SoundBank holds Vec<Option<SoundSample>>

## Deferred Implementation Detection (Mandatory)

```bash
grep -n "todo!()" rust/src/sound/music.rs | wc -l   # Expected > 0
grep -n "todo!()" rust/src/sound/sfx.rs | wc -l     # Expected > 0
```

## Success Criteria
- [ ] Both modules compile
- [ ] Both registered in mod.rs
- [ ] Import paths correct

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/mod.rs` and remove new files

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P12.md`
