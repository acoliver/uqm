# Phase 15: FFI Stub

## Phase ID
`PLAN-20260225-AIFF-DECODER.P15`

## Prerequisites
- Required: Phase 14 completed (aiff.rs feature-complete)
- Expected files: `rust/src/sound/aiff.rs` with all methods implemented

## Requirements Implemented (Expanded)

### REQ-FF-1: FFI Wrapper Struct
**Requirement text**: Define `TFB_RustAiffDecoder` with `TFB_SoundDecoder` as first field and `*mut c_void` for Rust decoder.

Behavior contract:
- GIVEN: The FFI module is created
- WHEN: Compiled
- THEN: `TFB_RustAiffDecoder` is `#[repr(C)]` with base as first field

### REQ-FF-2: Vtable Export
**Requirement text**: Export `rust_aifa_DecoderVtbl` with all 12 function pointers.

Behavior contract:
- GIVEN: The vtable static is defined
- WHEN: The library is compiled
- THEN: `rust_aifa_DecoderVtbl` is available as a `#[no_mangle]` exported symbol

### REQ-FF-3: Module Format Storage
**Requirement text**: Store `DecoderFormats` in `static RUST_AIFA_FORMATS: Mutex<Option<DecoderFormats>>`.

### REQ-FF-10: Null Pointer Safety
**Requirement text**: Every FFI function checks for null decoder and null rust_decoder.

### REQ-FF-11: GetStructSize
**Requirement text**: Returns `size_of::<TFB_RustAiffDecoder>()`.

### REQ-FF-12: GetName
**Requirement text**: Returns pointer to `"Rust AIFF\0"`.

Why it matters:
- Establishes the FFI bridge skeleton
- All 12 vtable functions defined with safe defaults/stubs
- Vtable can be referenced from C

## Implementation Tasks

### Files to create
- `rust/src/sound/aiff_ffi.rs` — C FFI bridge
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P15`
  - marker: `@requirement REQ-FF-1, REQ-FF-2, REQ-FF-3, REQ-FF-10, REQ-FF-11, REQ-FF-12`
  - Imports: `std::ffi::{c_char, c_int, c_void, CStr}`, `std::ptr`, `std::sync::Mutex`
  - Imports: `crate::bridge_log::rust_bridge_log_msg`
  - Imports: decoder types from `super::aiff::AiffDecoder`, `super::decoder::SoundDecoder`, `super::formats::DecoderFormats`
  - Imports: FFI types from `super::ffi::{uio_DirHandle, TFB_DecoderFormats, TFB_SoundDecoder, TFB_SoundDecoderFuncs}`
  - UIO extern declarations: `uio_open`, `uio_read`, `uio_close`, `uio_fstat`
  - `TFB_RustAiffDecoder` struct (`#[repr(C)]`)
  - `RUST_AIFA_FORMATS: Mutex<Option<DecoderFormats>>`
  - `RUST_AIFA_NAME: &[u8] = b"Rust AIFF\0"`
  - `read_uio_file()` — fully implemented (matches `dukaud_ffi.rs` pattern)
  - All 12 `extern "C"` functions:
    - `rust_aifa_GetName` — implemented (returns name pointer)
    - `rust_aifa_InitModule` — implemented (stores formats)
    - `rust_aifa_TermModule` — implemented (clears formats)
    - `rust_aifa_GetStructSize` — implemented (returns size_of)
    - `rust_aifa_GetError` — implemented (null check + delegate)
     - `rust_aifa_Init` — implemented (Box::new + call dec.init_module(0, &formats) from global Mutex + call dec.init() + store pointer; matches wav_ffi.rs Init pattern lines 138-147)
    - `rust_aifa_Term` — implemented (Box::from_raw + drop)
    - `rust_aifa_Open` — `todo!()` stub (complex UIO + open_from_bytes)
    - `rust_aifa_Close` — implemented (null check + delegate close)
    - `rust_aifa_Decode` — `todo!()` stub (complex buffer conversion)
    - `rust_aifa_Seek` — implemented (null check + delegate seek)
    - `rust_aifa_GetFrame` — implemented (null check + delegate get_frame)
  - `rust_aifa_DecoderVtbl` static export

### Files to modify
- `rust/src/sound/mod.rs`
  - Add: `pub mod aiff_ffi;`
  - Add: `pub use aiff_ffi::rust_aifa_DecoderVtbl;` (export vtable symbol)
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P15`

### Pseudocode traceability
- Uses pseudocode lines: 1–5 (struct), 6–30 (read_uio_file), 31–78 (simple functions), 179–193 (vtable)

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Must compile
cargo check --all-features

# All existing tests pass
cargo test --lib --all-features -- aiff

# Format and lint
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] `rust/src/sound/aiff_ffi.rs` created
- [ ] `pub mod aiff_ffi;` in `mod.rs`
- [ ] `TFB_RustAiffDecoder` defined with `#[repr(C)]`
- [ ] `rust_aifa_DecoderVtbl` static with `#[no_mangle]`
- [ ] All 12 function pointers in vtable
- [ ] `RUST_AIFA_FORMATS` Mutex defined
- [ ] `read_uio_file()` implemented
- [ ] Null checks in all functions that take `*mut TFB_SoundDecoder`

## Semantic Verification Checklist (Mandatory)
- [ ] GetName returns valid C string pointer
- [ ] InitModule/TermModule handle Mutex correctly
- [ ] Init allocates Box and stores raw pointer
- [ ] Term reconstructs Box and drops (frees memory)
- [ ] GetStructSize returns correct size
- [ ] Close delegates to AiffDecoder::close()
- [ ] Seek delegates to AiffDecoder::seek()
- [ ] GetFrame delegates to AiffDecoder::get_frame()
- [ ] Open and Decode are `todo!()` stubs (complex logic deferred)

## Deferred Implementation Detection (Mandatory)

```bash
# Stub phase: todo!() allowed in Open and Decode
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()" src/sound/aiff_ffi.rs
# Should show: rust_aifa_Open, rust_aifa_Decode
```

## Success Criteria
- [ ] `cargo check --all-features` succeeds
- [ ] All existing tests pass
- [ ] Vtable static defined with all 12 function pointers
- [ ] Simple FFI functions implemented (GetName, InitModule, etc.)
- [ ] Complex FFI functions stubbed (Open, Decode)

## Failure Recovery
- rollback steps: `git checkout -- rust/src/sound/aiff_ffi.rs rust/src/sound/mod.rs`
- blocking issues: If TFB_SoundDecoderFuncs fields don't match, check ffi.rs

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P15.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P15
- timestamp
- files changed: `rust/src/sound/aiff_ffi.rs` (created), `rust/src/sound/mod.rs` (modified)
- tests added/updated: None (stub phase)
- verification outputs
- semantic verification summary
