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
- [ ] C header `rust_audio_heart.h` created with all FFI declarations
- [ ] 6 C files conditionally excluded: stream.c, trackplayer.c, music.c, sfx.c, sound.c, fileinst.c
- [ ] `build.sh uqm` succeeds with Rust audio heart enabled
- [ ] All workspace Rust tests pass
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes
- [ ] No new linker warnings

## Semantic Verification Checklist

### Deterministic checks
- [ ] All Rust tests pass: `cargo test --lib --all-features` shows 0 failures across ALL modules (types, stream, trackplayer, music, sfx, control, fileinst, heart_ffi)
- [ ] C build succeeds: `./build.sh uqm` returns exit code 0
- [ ] Symbols linked correctly: `nm` on built binary shows Rust-provided symbols (PLRPlaySong, PlayChannel, etc.)
- [ ] No duplicate symbols: no linker warnings about symbol conflicts between Rust and C
- [ ] Zero deferred markers across ALL modules: `grep -rIn "TODO\|FIXME\|HACK\|todo!()" rust/src/sound/types.rs rust/src/sound/stream.rs rust/src/sound/trackplayer.rs rust/src/sound/music.rs rust/src/sound/sfx.rs rust/src/sound/control.rs rust/src/sound/fileinst.rs rust/src/sound/heart_ffi.rs` returns 0

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
