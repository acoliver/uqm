# Phase 21a: Integration Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P21a`

## Prerequisites
- Required: Phase P21 completed
- Expected: All modules integrated, USE_RUST_AUDIO_HEART flag working, C build succeeds

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm
# Verify Rust symbols replace C symbols
nm build/uqm 2>/dev/null | grep -c "PLRPlaySong\|PlayChannel\|SpliceTrack\|InitMusicPlayer"
# Verify C files excluded
grep -r "USE_RUST_AUDIO_HEART" sc2/src/libs/sound/
```

## Structural Verification Checklist
- [ ] `USE_RUST_AUDIO_HEART` flag added to `config_unix.h` (or equivalent)
- [ ] C header `audio_heart_rust.h` created with all FFI declarations
- [ ] 6 C files conditionally excluded: stream.c, trackplayer.c, music.c, sfx.c, sound.c, fileinst.c
- [ ] `build.sh uqm` succeeds with Rust audio heart enabled
- [ ] All workspace Rust tests pass
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes
- [ ] No new linker warnings

## Semantic Verification Checklist

### Deterministic checks
- [ ] All Rust tests pass: `cargo test --lib --all-features` shows 0 failures across ALL modules (types, stream, trackplayer, music, sfx, control, fileinst, heart_ffi)
- [ ] C build succeeds WITH Rust heart: `USE_RUST_AUDIO_HEART=1 ./build.sh uqm` returns exit code 0
- [ ] C build succeeds WITHOUT Rust heart: `./build.sh uqm` returns exit code 0 (backwards compatibility)
- [ ] Symbols linked correctly: `nm` on built binary shows Rust-provided symbols (PLRPlaySong, PlayChannel, etc.)
- [ ] No duplicate symbols: no linker warnings about symbol conflicts between Rust and C
- [ ] Zero deferred markers across ALL modules: `grep -rIn "TODO\|FIXME\|HACK\|todo!()" rust/src/sound/types.rs rust/src/sound/stream.rs rust/src/sound/trackplayer.rs rust/src/sound/music.rs rust/src/sound/sfx.rs rust/src/sound/control.rs rust/src/sound/fileinst.rs rust/src/sound/heart_ffi.rs` returns 0

### Backwards compatibility verification (USE_RUST_HEART disabled)

The game must work correctly with `USE_RUST_AUDIO_HEART` **disabled** (the default). This verifies that the header guard changes and build system modifications don't break the existing C audio path.

- [ ] **Build without flag**: `./build.sh uqm` (no `USE_RUST_AUDIO_HEART`) succeeds with zero errors and zero new warnings
- [ ] **C files included**: Verify that `stream.c`, `trackplayer.c`, `music.c`, `sfx.c`, `sound.c`, `fileinst.c` are compiled into the binary (check build log or `nm` output for C-implemented symbols)
- [ ] **No Rust audio symbols**: When built without the flag, `nm` should NOT show Rust-specific audio heart symbols (the Rust library may still be linked for mixer/decoder, but heart_ffi symbols should not appear in the audio call path)
- [ ] **Header guards correct**: Each modified header (`music.h`, `sfx.h`, `sound.h`, `stream.h`, `trackplayer.h`, `fileinst.h`) must compile cleanly in both `#ifdef` branches:
  - Without flag: C prototypes visible, `audio_heart_rust.h` NOT included
  - With flag: Rust prototypes visible via `audio_heart_rust.h`
- [ ] **Runtime regression test**: Launch game built without the flag. Verify:
  - Title screen music plays
  - Menu navigation SFX plays
  - Entering communication screen: speech plays, subtitles appear, oscilloscope renders
  - Volume controls in options work
  - Game exits cleanly without audio-related crashes or hangs
- [ ] **No behavioral difference**: The C-built game and Rust-built game should produce identical observable audio behavior for the same inputs

### Subjective checks — End-to-end scenarios
- [ ] **Music playback**: Can PLRPlaySong → plr_play_song → play_stream → decoder thread → mixer chain play audio end-to-end? Does the streaming thread correctly wake, decode buffers, queue to mixer, and track FPS?
- [ ] **Music fade**: Does FadeMusic schedule a fade that smoothly transitions volume via the decoder thread's process_music_fade?
- [ ] **Track/speech playback**: Can SpliceTrack → splice_track → play_track → play_stream chain play speech with subtitles? Does the track player correctly advance chunks, fire subtitle callbacks, and handle seek operations?
- [ ] **SFX playback**: Can PlayChannel resolve the opaque SOUND handle, look up the pre-decoded sample, and play it through the mixer? Does positional audio set coordinates correctly?
- [ ] **File loading**: Can LoadSoundFile → load_sound_file → get_sound_bank_data parse a sound list, pre-decode all samples, and return a usable SoundBank? Is concurrent loading properly guarded?
- [ ] **Volume control**: Do SetSFXVolume, SetSpeechVolume, and SetMusicVolume correctly apply gain to the right sources? Does music volume interact correctly with fade?
- [ ] **Shutdown**: Does UninitAudio → uninit_stream_decoder correctly join the decoder thread, clean up all resources, and leave the system in a safe state?
- [ ] **Error resilience**: Do null pointers, invalid handles, and concurrent access produce error codes (not panics)?
- [ ] **No regressions**: Does enabling USE_RUST_AUDIO_HEART produce the same observable behavior as the C implementation for known test scenarios?

### Performance sanity check
- [ ] **Decode throughput**: A simple benchmark test that calls `decode_all` on a representative audio file completes within a reasonable bound (no accidental O(n²) algorithms). Suggest: decode a 10-second 44.1kHz stereo WAV file in < 100ms on modern hardware.
- [ ] **No excessive allocations**: Check that streaming playback (music + speech simultaneously) does not grow memory over a 60-second run — scope buffers and mixer buffers should be fixed-size after initialization.

## Deferred Implementation Detection

```bash
grep -rIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|todo!()" \
  rust/src/sound/types.rs rust/src/sound/stream.rs rust/src/sound/trackplayer.rs \
  rust/src/sound/music.rs rust/src/sound/sfx.rs rust/src/sound/control.rs \
  rust/src/sound/fileinst.rs rust/src/sound/heart_ffi.rs
# Must return 0 results across ALL files
```

## Success Criteria
- [ ] All tests pass across all 8 modules
- [ ] C build succeeds with Rust audio heart
- [ ] No duplicate or missing symbols
- [ ] Zero deferred implementations
- [ ] End-to-end audio pipeline functional

## Failure Recovery
- rollback: `git stash` or revert the integration commit
- If linker issues: check for symbol conflicts, missing `#[no_mangle]`, or wrong `extern "C"` linkage
- If runtime crashes: enable Rust logging, check for null pointer issues at FFI boundary
- If audio silence: verify mixer initialization order, check that `init_stream_decoder` is called after `mixer_init`

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P21a.md`

## Plan Completion
When this phase passes, the entire PLAN-20260225-AUDIO-HEART plan is complete.
All 6 C files (stream.c, trackplayer.c, music.c, sfx.c, sound.c, fileinst.c) are replaced by Rust equivalents.
