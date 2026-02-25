# Phase 14: Music + SFX Implementation

## Phase ID
`PLAN-20260225-AUDIO-HEART.P14`

## Prerequisites
- Required: Phase P13a (Music + SFX TDD Verification) passed
- Expected: 25+ tests across music.rs and sfx.rs

## Requirements Implemented (Expanded)

All MUSIC-* (21) and SFX-* (26) requirements, grouped by category:

### Music Playback (MUSIC-PLAY-01..08)
- GIVEN: A loaded MusicRef and the streaming engine initialized
- WHEN: `plr_play_song`/`plr_stop`/`plr_playing`/`plr_pause`/`plr_resume`/`plr_seek` are called
- THEN: Delegates to stream engine on MUSIC_SOURCE with correct sample, handles already-playing replacement, and reports state accurately

### Music Speech (MUSIC-SPEECH-01..02)
- GIVEN: A loaded speech sample and the streaming engine initialized
- WHEN: `snd_play_speech`/`snd_stop_speech` are called
- THEN: Delegates to stream engine on SPEECH_SOURCE, scope buffer is allocated for oscilloscope rendering

### Music Loading (MUSIC-LOAD-01..06)
- GIVEN: A filename for a music resource
- WHEN: `get_music_data` is called
- THEN: A decoder is loaded via `load_decoder()`, a SoundSample with 64 buffers is created, and a MusicRef (raw pointer wrapper) is returned via `Box::into_raw`

### Music Release (MUSIC-RELEASE-01..04)
- GIVEN: A MusicRef previously returned by `get_music_data`
- WHEN: `release_music_data` is called
- THEN: Playback is stopped if active, the decoder and sample are dropped, and the Box is reclaimed (no double-free)

### Music Volume (MUSIC-VOLUME-01)
- GIVEN: A volume value (0..MAX_VOLUME)
- WHEN: `set_music_volume` is called
- THEN: The gain is computed from volume * volume_scale and applied to MUSIC_SOURCE via `mixer_source_f`

### SFX Playback (SFX-PLAY-01..09)
- GIVEN: A SoundBank with pre-decoded samples
- WHEN: `play_channel`/`stop_channel`/`channel_playing` are called
- THEN: The correct SFX source is selected (round-robin or priority), the pre-decoded sample buffer is queued, and playback starts/stops correctly

### SFX Positioning (SFX-POSITION-01..05)
- GIVEN: A playing SFX source and a SoundPosition
- WHEN: `update_sound_position` is called
- THEN: 3D position is set via three separate `mixer_source_f` calls (X, Y, Z), attenuation is computed from distance, and the gain is adjusted

### SFX Volume (SFX-VOLUME-01)
- GIVEN: A volume value and a channel index
- WHEN: `set_channel_volume` is called
- THEN: The gain is computed with sfx_volume_scale and applied to the correct SFX source

### SFX Loading (SFX-LOAD-01..07)
- GIVEN: A resource file containing SFX bank data
- WHEN: `get_sound_bank_data` is called
- THEN: Each sample line is parsed, a decoder is loaded, `decode_all` pre-decodes the audio, and buffer data is uploaded to the mixer via `mixer_buffer_data`

### SFX Release (SFX-RELEASE-01..04)
- GIVEN: A SoundBank previously returned by `get_sound_bank_data`
- WHEN: `release_sound_bank_data` is called
- THEN: All active channels using this bank are stopped, mixer buffers are deleted, and the SoundBank is dropped

### Pseudocode traceability
- `plr_play_song`: pseudocode `music.md` lines 1-25
- `plr_stop`: pseudocode `music.md` lines 30-43
- `plr_playing`: pseudocode `music.md` lines 50-63
- `snd_play_speech`/`snd_stop_speech`: pseudocode `music.md` lines 90-114
- `get_music_data`: pseudocode `music.md` lines 120-143
- `release_music_data`: pseudocode `music.md` lines 150-170
- `set_music_volume`: pseudocode `music.md` lines 190-196
- `fade_music`: pseudocode `music.md` lines 200-223 (expanded with fade replacement behavior)
- `play_channel`: pseudocode `sfx.md` lines 1-35
- `update_sound_position`: pseudocode `sfx.md` lines 80-105
- `get_sound_bank_data`: pseudocode `sfx.md` lines 120-172
- `release_sound_bank_data`: pseudocode `sfx.md` lines 180-200

## Implementation Tasks

### Files to modify
- `rust/src/sound/music.rs` — Replace all `todo!()` with implementations
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P14`
  - marker: `@requirement REQ-MUSIC-*`
- `rust/src/sound/sfx.rs` — Replace all `todo!()` with implementations
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P14`
  - marker: `@requirement REQ-SFX-*`

### music.rs implementation order
1. `set_music_volume` — simplest, state update + mixer call
2. `plr_play_song`/`plr_stop`/`plr_playing` — delegate to stream
3. `plr_seek`/`plr_pause`/`plr_resume` — delegate to stream
4. `snd_play_speech`/`snd_stop_speech` — delegate to stream on SPEECH_SOURCE
5. `get_music_data` — decoder loading + sample creation
6. `release_music_data` — cleanup with active-check
7. `check_music_res_name` — validation
8. `fade_music` — delegates to set_music_stream_fade

### sfx.rs implementation order
1. `update_sound_position` — 3D position math
2. `get_positional_object`/`set_positional_object` — accessor pair
3. `play_channel` — stop + setup + play
4. `stop_channel`/`channel_playing` — delegates
5. `set_channel_volume` — gain computation
6. `check_finished_channels` — polling loop
7. `get_sound_bank_data` — file parsing + decoder loading + buffer upload
8. `release_sound_bank_data` — cleanup with source-check

### Key implementation notes
- `MusicRef` is a raw pointer — all access requires `unsafe`
- Positional audio: `ATTENUATION = 160.0`, `MIN_DISTANCE = 0.5`
- **3D Positioning** (REQ-SFX-POSITION-01): Use three separate `mixer_source_f` calls for X, Y, Z position components (see P12 design note). The mixer does NOT have `mixer_source_fv()`.
- SFX bank loading: parse lines, each line is a WAV filename
- All `parking_lot::Mutex` references (never bare `Mutex` or `std::sync::Mutex`)

### Fade-in-progress replacement behavior

When `fade_music()` is called while a fade is already in progress, the new fade **replaces the current fade immediately**. This matches C behavior. The implementation in `set_music_stream_fade()` (stream.md §13, lines 380-389) unconditionally overwrites all `FadeState` fields:

1. `start_time` is reset to `get_time_counter()` (now)
2. `start_volume` is read from `current_music_volume()` — this captures the **mid-fade** volume at the moment of the call, not the old fade's start or target
3. `delta` is recomputed as `end_volume - start_volume` toward the new target
4. `interval` is set to the new duration

The old fade is simply abandoned — there is no completion callback or cleanup needed. The decoder thread's `process_music_fade()` will pick up the new fade state on its next iteration (within ~10ms). This design is intentionally simple: the fade state machine has no "pending" queue, just a single active fade that can be replaced at any time.

### C Resource System Integration (_GetMusicData / _ReleaseMusicData)

The `get_music_data` and `release_music_data` functions serve as Rust resource handlers that replace the C `_GetMusicData` and `_ReleaseMusicData` functions. The integration mechanism is:

1. **Registration**: The C resource system uses a vtable (function pointer table) for each resource type. During `init_sound()` (or at library link time via the FFI shims), the Rust FFI functions `GetMusicData` and `ReleaseMusicData` (declared in `heart_ffi.rs`) are registered in the music resource vtable slot, replacing the C implementations.

2. **Data flow**: When C code calls `res_GetMusic(filename)`:
   - The resource system calls through the vtable to `GetMusicData` (Rust FFI shim)
   - The FFI shim converts the C string filename to `&str`
   - It calls `music::get_music_data(filename)` which:
     - Loads a decoder via `load_decoder()` (which reads from the content directory via `uio_*` FFI)
     - Creates a `SoundSample` with 64 buffers
     - Leaks the sample as a raw pointer (`Box::into_raw`) → returned as `MusicRef`
   - The FFI shim returns the `MusicRef` as `*mut c_void` to C

3. **Release flow**: When C code calls `res_FreeMusic(handle)`:
   - The resource system calls through the vtable to `ReleaseMusicData` (Rust FFI shim)
   - The FFI shim receives `*mut c_void`, casts to `MusicRef`
   - Calls `music::release_music_data(music_ref)` which stops playback if active, drops the decoder, destroys the sample, and reclaims the `Box`

4. **Raw bytes**: The Rust decoder system receives file data through `uio_open`/`uio_read`/`uio_close` FFI calls (not raw byte slices). The decoder opens the file from the content directory, reads and decodes it. No separate "raw bytes + metadata" path is needed — the decoder handles I/O internally via the `uio_*` layer.

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::music::tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::sfx::tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] All `todo!()` removed from both files
- [ ] All tests pass
- [ ] fmt and clippy pass

## Semantic Verification Checklist (Mandatory)
- [ ] Music playback delegates to stream correctly
- [ ] Speech uses SPEECH_SOURCE, music uses MUSIC_SOURCE
- [ ] Positional audio math correct (distance, attenuation, min distance)
- [ ] SFX bank loading parses file list correctly
- [ ] Volume scaling applied with volume_scale factors
- [ ] Fade delegates to stream fade

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|todo!()" rust/src/sound/music.rs rust/src/sound/sfx.rs
# Must return 0 results
```

## Success Criteria
- [ ] All 25+ tests pass
- [ ] Zero deferred implementations
- [ ] Music and SFX fully operational (unit-level)

## Failure Recovery
- rollback: `git stash` or `git checkout -- rust/src/sound/music.rs rust/src/sound/sfx.rs`

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P14.md`
