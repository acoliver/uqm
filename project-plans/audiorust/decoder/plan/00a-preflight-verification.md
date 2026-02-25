# Phase 00a: Preflight Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P00a`

## Prerequisites
- Required: Plan document approved
- Expected artifacts: specification.md, analysis/ directory structure


## Requirements Implemented (Expanded)

N/A — Verification-only phase. Requirements are verified, not implemented.

## Implementation Tasks

N/A — Verification-only phase. No code changes.
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

## Structural Verification Checklist

### Toolchain
- [ ] `cargo --version` — Rust toolchain available
- [ ] `rustc --version` — compiler version compatible
- [ ] `cargo clippy --version` — linter available
- [ ] `cd /Users/acoliver/projects/uqm/rust && cargo check --all-features` — existing code compiles
- [ ] Coverage tool: N/A — this plan does not require coverage gates (`cargo llvm-cov` is not used). If coverage requirements are added later, verify `cargo llvm-cov --version` is available.

### Dependencies
- [ ] `rust/Cargo.toml` contains `libc` dependency (needed for FFI `stat` struct)
- [ ] No new external crates needed for AIFF decoding (pure Rust, std::io::Cursor)
- [ ] `std::io::{Cursor, Read, Seek, SeekFrom}` available in std

### Types/Interfaces
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

## Semantic Verification Checklist (Mandatory)

### Deterministic Checks
- [ ] `cargo check --all-features` exits 0
- [ ] `cargo test --lib --all-features` exits 0 (existing tests pass)
- [ ] `sd_decoders[]` array in `decoder.c` has at least one `USE_RUST_*` conditional entry (proving the pattern works)

### Subjective Checks
- [ ] Does `rust/src/sound/mod.rs` structure allow adding `pub mod aiff; pub mod aiff_ffi;` without conflicts?
- [ ] Do existing FFI decoders (`dukaud_ffi.rs`, `wav_ffi.rs`) successfully export vtables — confirming the pattern is proven?
- [ ] Does `sc2/src/config_unix.h.in` support adding new `@SYMBOL_*_DEF@` placeholders?
- [ ] Does `sc2/build.vars.in` have a clear pattern for `USE_RUST_*` variables that can be replicated?
- [ ] Are existing decoder tests (wav, dukaud) using synthetic byte array testing — confirming the test pattern for AIFF?

### Call-Path Feasibility
- [ ] `rust/src/sound/mod.rs` structure allows adding `pub mod aiff; pub mod aiff_ffi;`
- [ ] Existing FFI decoders (`dukaud_ffi.rs`, `wav_ffi.rs`) successfully export vtables
- [ ] `sc2/src/libs/sound/decoders/decoder.c` `sd_decoders[]` array supports `#ifdef USE_RUST_AIFF` conditionals
- [ ] `sc2/src/config_unix.h.in` supports adding new `@SYMBOL_USE_RUST_AIFF_DEF@`
- [ ] `sc2/build.vars.in` supports adding `USE_RUST_AIFF` variables

### Test Infrastructure
- [ ] `cargo test --lib --all-features` works for existing sound module tests
- [ ] Existing decoder tests (wav, dukaud) provide patterns for synthetic byte array testing
- [ ] Test module pattern: `#[cfg(test)] mod tests { ... }` inside each source file

## Deferred Implementation Detection

```bash
# N/A for preflight verification — no implementation code
echo "Preflight phase: no deferred implementation detection needed"
```

## Success Criteria
- [ ] Toolchain available and compatible
- [ ] Existing code compiles and tests pass
- [ ] All required types and traits exist
- [ ] Integration path is feasible (USE_RUST_* pattern proven)
- [ ] Test infrastructure works

## Failure Recovery
- If toolchain is missing: install Rust via rustup
- If types don't match spec: update specification.md to match actual types
- If existing tests fail: fix before starting AIFF implementation

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P00a.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P00a
- timestamp
- verification result: PASS/FAIL
- blocking issues (if any)
- gate decision: proceed to P01 or FAIL with specific issues
