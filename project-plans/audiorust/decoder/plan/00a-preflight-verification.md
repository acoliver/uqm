# Phase 0.5: Preflight Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P00a`

## Purpose
Verify assumptions about toolchain, types, interfaces, and test infrastructure before implementation begins.

## Toolchain Verification
- [ ] `cargo --version` — Rust toolchain available
- [ ] `rustc --version` — compiler version compatible
- [ ] `cargo clippy --version` — linter available
- [ ] `cd /Users/acoliver/projects/uqm/rust && cargo check --all-features` — existing code compiles

## Dependency Verification
- [ ] `rust/Cargo.toml` contains `libc` dependency (needed for FFI `stat` struct)
- [ ] No new external crates needed for AIFF decoding (pure Rust, std::io::Cursor)
- [ ] `std::io::{Cursor, Read, Seek, SeekFrom}` available in std

## Type/Interface Verification
- [ ] `SoundDecoder` trait exists in `rust/src/sound/decoder.rs` with all 15 methods
- [ ] `DecodeError` enum exists with variants: `NotFound`, `InvalidData`, `UnsupportedFormat`, `IoError`, `NotInitialized`, `EndOfFile`, `SeekFailed`, `DecoderError`
- [ ] `DecodeResult<T>` type alias exists as `Result<T, DecodeError>`
- [ ] `AudioFormat` enum exists in `rust/src/sound/formats.rs` with `Mono8`, `Stereo8`, `Mono16`, `Stereo16`
- [ ] `DecoderFormats` struct exists with `big_endian`, `want_big_endian`, `mono8`, `stereo8`, `mono16`, `stereo16`
- [ ] `TFB_SoundDecoder` struct exists in `rust/src/sound/ffi.rs`
- [ ] `TFB_SoundDecoderFuncs` struct exists with all 12 function pointer fields
- [ ] `TFB_DecoderFormats` struct exists in `rust/src/sound/ffi.rs`
- [ ] `uio_DirHandle` type exists in `rust/src/sound/ffi.rs`
- [ ] `rust_bridge_log_msg` function exists in `rust/src/bridge_log.rs`

## Call-Path Feasibility
- [ ] `rust/src/sound/mod.rs` structure allows adding `pub mod aiff; pub mod aiff_ffi;`
- [ ] Existing FFI decoders (`dukaud_ffi.rs`, `wav_ffi.rs`) successfully export vtables
- [ ] `sc2/src/libs/sound/decoders/decoder.c` `sd_decoders[]` array supports `#ifdef USE_RUST_AIFF` conditionals
- [ ] `sc2/src/config_unix.h.in` supports adding new `@SYMBOL_USE_RUST_AIFF_DEF@`
- [ ] `sc2/build.vars.in` supports adding `USE_RUST_AIFF` variables

## Test Infrastructure Verification
- [ ] `cargo test --lib --all-features` works for existing sound module tests
- [ ] Existing decoder tests (wav, dukaud) provide patterns for synthetic byte array testing
- [ ] Test module pattern: `#[cfg(test)] mod tests { ... }` inside each source file

## Verification Commands

```bash
# Toolchain
cargo --version
rustc --version
cargo clippy --version

# Compile check
cd /Users/acoliver/projects/uqm/rust && cargo check --all-features

# Existing tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features

# Type existence (grep verification)
grep -n "pub trait SoundDecoder" rust/src/sound/decoder.rs
grep -n "pub enum DecodeError" rust/src/sound/decoder.rs
grep -n "pub enum AudioFormat" rust/src/sound/formats.rs
grep -n "pub struct DecoderFormats" rust/src/sound/formats.rs
grep -n "pub struct TFB_SoundDecoder" rust/src/sound/ffi.rs
grep -n "pub struct TFB_SoundDecoderFuncs" rust/src/sound/ffi.rs
grep -n "pub type uio_DirHandle" rust/src/sound/ffi.rs

# C integration path
grep -n "sd_decoders" sc2/src/libs/sound/decoders/decoder.c | head -5
grep -n "USE_RUST_DUKAUD" sc2/src/libs/sound/decoders/decoder.c
```

## Blocking Issues
[List any blockers discovered during verification. If non-empty, stop and revise plan first.]

## Gate Decision
- [ ] PASS: proceed to Phase 1
- [ ] FAIL: revise plan — document specific issues
