# Phase 0.5: Preflight Verification

## Phase ID
`PLAN-20260314-AUDIO-HEART.P00.5`

## Purpose
Verify assumptions about the current codebase before any implementation work begins.

## Toolchain Verification

```bash
cargo --version
rustc --version
cargo clippy --version
```

- [ ] Rust toolchain available and recent enough for `std::sync::LazyLock` (1.80+)
- [ ] `parking_lot` crate available in `Cargo.toml`
- [ ] `cpal` crate available (used by mixer pump in `stream.rs`)
- [ ] `log` crate available (used by `trackplayer.rs`)

## Feature Flag Verification

- [ ] `Cargo.toml` defines `audio_heart = []` feature
- [ ] `rust/src/sound/mod.rs:33-34` gates `heart_ffi` on `#[cfg(feature = "audio_heart")]`
- [ ] `sc2/config_unix.h:80-81` defines `USE_RUST_AUDIO_HEART`
- [ ] Build system passes `--features audio_heart` when C macro is defined

```bash
grep -n 'audio_heart' rust/Cargo.toml
grep -n 'cfg.*audio_heart' rust/src/sound/mod.rs
grep -n 'USE_RUST_AUDIO_HEART' sc2/config_unix.h
```

## Type/Interface Verification

### Types that must exist (in `rust/src/sound/types.rs`)
- [ ] `AudioError` enum with all 14 variants per spec §19.1
- [ ] `AudioResult<T>` type alias
- [ ] `SoundSample` struct with fields: decoder, length, buffers, buffer_tags, offset, looping, data, callbacks
- [ ] `SoundSource` struct with fields: handle, sample, stream_should_be_playing, start_time, pause_time, positional_object, etc.
- [ ] `SoundTag` struct with buf_handle, data
- [ ] `MusicRef` newtype wrapper around `Arc<Mutex<SoundSample>>`
- [ ] `SoundBank` struct with samples, source_file
- [ ] `FadeState` struct
- [ ] `SoundPosition` with `#[repr(C)]`
- [ ] `StreamCallbacks` trait
- [ ] Constants: NUM_SOUNDSOURCES=7, MUSIC_SOURCE=5, SPEECH_SOURCE=6, MAX_VOLUME=255, NORMAL_VOLUME=160, ONE_SECOND=840

```bash
grep -n 'pub struct SoundSample' rust/src/sound/types.rs
grep -n 'pub struct SoundSource' rust/src/sound/types.rs
grep -n 'pub struct MusicRef' rust/src/sound/types.rs
grep -n 'pub struct SoundBank' rust/src/sound/types.rs
grep -n 'pub struct FadeState' rust/src/sound/types.rs
grep -n 'pub trait StreamCallbacks' rust/src/sound/types.rs
grep -n 'NORMAL_VOLUME' rust/src/sound/types.rs rust/src/sound/control.rs
```

### Functions that must exist
- [ ] `stream::init_stream_decoder()` — in `rust/src/sound/stream.rs`
- [ ] `stream::uninit_stream_decoder()` — in `rust/src/sound/stream.rs`
- [ ] `stream::create_sound_sample()` — in `rust/src/sound/stream.rs`
- [ ] `stream::play_stream()` — in `rust/src/sound/stream.rs`
- [ ] `stream::stop_stream()` — in `rust/src/sound/stream.rs`
- [ ] `stream::with_source()` — in `rust/src/sound/stream.rs`
- [ ] `trackplayer::splice_track()` — in `rust/src/sound/trackplayer.rs`
- [ ] `trackplayer::splice_multi_track()` — in `rust/src/sound/trackplayer.rs`
- [ ] `music::plr_play_song()` — in `rust/src/sound/music.rs`
- [ ] `music::plr_pause()` — in `rust/src/sound/music.rs`
- [ ] `music::get_music_data()` — in `rust/src/sound/music.rs` (stub)
- [ ] `sfx::get_sound_bank_data()` — in `rust/src/sound/sfx.rs` (stub)
- [ ] `fileinst::load_sound_file()` — in `rust/src/sound/fileinst.rs`
- [ ] `fileinst::load_music_file()` — in `rust/src/sound/fileinst.rs`
- [ ] `control::wait_for_sound_end()` — in `rust/src/sound/control.rs`

```bash
grep -n 'pub fn init_stream_decoder' rust/src/sound/stream.rs
grep -n 'pub fn play_stream' rust/src/sound/stream.rs
grep -n 'pub fn splice_multi_track' rust/src/sound/trackplayer.rs
grep -n 'pub fn plr_pause' rust/src/sound/music.rs
grep -n 'pub fn get_music_data' rust/src/sound/music.rs
grep -n 'pub fn get_sound_bank_data' rust/src/sound/sfx.rs
```

## Call-Path Feasibility

### Loader consolidation path
- [ ] `heart_ffi.rs::LoadMusicFile` → currently does real loading inline (lines 1209-1283)
- [ ] `fileinst.rs::load_music_file` → currently routes to `music::get_music_data` (stub)
- [ ] Both paths must converge to a single canonical implementation
- [ ] The canonical implementation needs access to UIO FFI functions (uio_fopen, etc.)

### Multi-track decoder path
- [ ] `heart_ffi.rs::SpliceMultiTrack` → calls `trackplayer::splice_multi_track` (lines 658-672)
- [ ] The FFI shim currently loads decoders at the FFI boundary for SpliceTrack but NOT for SpliceMultiTrack
- [ ] `trackplayer::splice_multi_track` creates chunks with `decoder: None`
- [ ] Fix requires either: (a) FFI shim loads decoders before calling splice_multi_track, or (b) splice_multi_track takes a decoder-factory callback

### Pending-completion path
- [ ] No `PollPendingTrackCompletion` or `CommitTrackAdvancement` exists anywhere in Rust code
- [ ] The comm subsystem's `CheckSubtitles` in C currently handles subtitle polling
- [ ] Verify whether comm C code already expects these operations or if they need to be added to `audio_heart_rust.h`

```bash
grep -rn 'PollPending\|CommitTrack' sc2/src/libs/
grep -rn 'PollPending\|CommitTrack' rust/src/
```

## Test Infrastructure Verification

- [ ] Existing tests compile and pass:
```bash
cargo test --workspace --all-features 2>&1 | tail -20
```
- [ ] Test modules exist in all audio-heart source files
- [ ] `#[cfg(test)]` mock implementations exist for `GetTimeCounter`, `QuitPosted` in `types.rs`

```bash
grep -n '#\[cfg(test)\]' rust/src/sound/types.rs
grep -n 'cfg(test)' rust/src/sound/stream.rs
```

## Current Warning/Error State

```bash
cargo clippy --workspace --all-features -- -D warnings 2>&1 | head -50
```

- [ ] Note: clippy may currently fail due to `#![allow(dead_code)]` masking issues
- [ ] Record current warning count as baseline

## Blocking Issues

List any blockers discovered. If non-empty, revise plan before proceeding.

## Gate Decision
- [ ] PASS: proceed to P01
- [ ] FAIL: revise plan (describe required revisions)
