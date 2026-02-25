# Phase 0a: Preflight Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P00a`

## Purpose
Verify assumptions before implementation.

## Toolchain Verification
- [ ] `cargo --version` (requires Rust stable)
- [ ] `rustc --version`
- [ ] `cargo clippy --version`
- [ ] `parking_lot` crate present in `Cargo.toml`
- [ ] `lazy_static` crate present in `Cargo.toml`
- [ ] `log` crate present in `Cargo.toml`

## Dependency Verification
- [ ] `parking_lot` version compatible (same as mixer uses)
- [ ] `lazy_static` version compatible
- [ ] `log` version compatible
- [ ] Feature flags: verify `--all-features` passes currently

## Type/Interface Verification
- [ ] `SoundDecoder` trait exists in `rust/src/sound/decoder.rs` with methods: `decode()`, `seek()`, `frequency()`, `format()`, `length()`, `get_frame()`, `is_null()`, `open_from_bytes()`, `close()`
- [ ] `SoundDecoder: Send` bound exists
- [ ] `DecodeError` enum exists with variants: `EndOfFile`, `DecoderError(String)`, `IoError(String)`, `NotFound(String)`, `InvalidData(String)`, `SeekFailed(String)`, `NotInitialized`, `UnsupportedFormat(String)`
- [ ] `AudioFormat` type exists in `rust/src/sound/formats.rs` with `bytes_per_sample()` and `channels()` methods
- [ ] `MixerError` type exists in `rust/src/sound/mixer/types.rs`
- [ ] `SourceProp` enum has: `Gain`, `Looping`, `Buffer`, `SourceState`, `BuffersQueued`, `BuffersProcessed`, `Position`
- [ ] `BufferProp` enum has: `Size`
- [ ] `SourceState` enum has: `Playing`, `Stopped`, `Paused`, `Initial`
- [ ] Mixer functions exist: `mixer_gen_sources`, `mixer_delete_sources`, `mixer_gen_buffers`, `mixer_delete_buffers`, `mixer_source_play`, `mixer_source_stop`, `mixer_source_pause`, `mixer_source_rewind`, `mixer_source_queue_buffers`, `mixer_source_unqueue_buffers`, `mixer_buffer_data`, `mixer_source_i`, `mixer_source_f`, `mixer_get_source_i`, `mixer_get_source_f`, `mixer_get_buffer_i`
- [ ] `NullDecoder` exists for testing

## Gaps to Address Before Implementation
- [ ] `SoundDecoder` trait lacks `set_looping()` → Plan: store looping flag on `SoundSample` instead
- [ ] `SoundDecoder` trait lacks `decode_all()` → Plan: add free function `decode_all()`
- [ ] `SoundDecoder` trait lacks `get_time()` → Plan: add free function `get_decoder_time()`
- [ ] Mixer lacks `mixer_source_fv()` for 3D position → Plan: add to `mixer/source.rs` in P03 (types stub)

## Call-Path Feasibility
- [ ] `stream.rs` can import from `sound::mixer::{mixer_source_play, mixer_buffer_data, ...}`
- [ ] `stream.rs` can import from `sound::decoder::{SoundDecoder, DecodeError}`
- [ ] `control.rs` can import from `sound::mixer::{mixer_gen_sources, ...}`
- [ ] Module path `sound::stream` is available (not already declared in `mod.rs`)
- [ ] Module path `sound::trackplayer` is available
- [ ] Module path `sound::music` is available
- [ ] Module path `sound::sfx` is available
- [ ] Module path `sound::control` is available
- [ ] Module path `sound::fileinst` is available
- [ ] Module path `sound::heart_ffi` is available

## Test Infrastructure Verification
- [ ] `cargo test --lib --all-features` passes in `rust/` directory
- [ ] Test modules can be added to new `.rs` files (standard `#[cfg(test)] mod tests {}`)
- [ ] `NullDecoder` usable as test double in unit tests

## Build System Verification
- [ ] `sc2/build.sh uqm` builds C code
- [ ] Rust static library linked into C build
- [ ] New `#[no_mangle]` functions will be available to C linker via existing static lib

## Blocking Issues
[To be populated during execution. If non-empty, stop and revise plan.]

## Gate Decision
- [ ] PASS: proceed to P01
- [ ] FAIL: revise plan (document issues)
