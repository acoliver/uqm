# Phase 15a: FFI Stub Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P15a`

## Prerequisites
- Required: Phase 15 completed
- Expected files: `rust/src/sound/aiff_ffi.rs`, updated `mod.rs`

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Compile check
cargo check --all-features

# All existing tests pass
cargo test --lib --all-features -- aiff

# Verify key symbols
grep "rust_aifa_DecoderVtbl" src/sound/aiff_ffi.rs
grep "TFB_RustAiffDecoder" src/sound/aiff_ffi.rs
grep "pub mod aiff_ffi" src/sound/mod.rs

# Format and lint
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Deferred implementation check
grep -n "todo!()" src/sound/aiff_ffi.rs
```

## Structural Verification Checklist
- [ ] `rust/src/sound/aiff_ffi.rs` exists
- [ ] `pub mod aiff_ffi;` in `mod.rs`
- [ ] `cargo check --all-features` succeeds
- [ ] `#[repr(C)] pub struct TFB_RustAiffDecoder` defined
- [ ] `#[no_mangle] pub static rust_aifa_DecoderVtbl` defined
- [ ] 12 function pointers in vtable struct
- [ ] `RUST_AIFA_FORMATS` Mutex defined
- [ ] `read_uio_file()` implemented
- [ ] Null checks in all functions that take `*mut TFB_SoundDecoder`
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy` passes

## Semantic Verification Checklist (Mandatory)

### Deterministic Checks
- [ ] GetName returns `b"Rust AIFF\0"` pointer
- [ ] InitModule stores formats in Mutex (REQ-FF-3)
- [ ] TermModule clears Mutex to None
- [ ] Init does ONLY `Box::new(AiffDecoder::new())` + `Box::into_raw` + store pointer — does NOT call init_module()/init() (REQ-FF-4, matching dukaud_ffi.rs pattern)
- [ ] Term does Box::from_raw + drop + null out pointer (REQ-FF-5)
- [ ] All functions with decoder arg have null check (REQ-FF-10)
- [ ] read_uio_file matches dukaud_ffi.rs pattern (open/fstat/read loop/close)
- [ ] Open and Decode are `todo!()` stubs (complex logic deferred)

### Subjective Checks
- [ ] Does the Init function match the dukaud_ffi.rs Init pattern exactly (allocate, store, set need_swap=false)?
- [ ] Does read_uio_file handle partial reads correctly (loop until all bytes read or error)?
- [ ] Is TFB_RustAiffDecoder's first field the TFB_SoundDecoder base (required for C struct inheritance)?
- [ ] Does the vtable static have the correct `#[no_mangle]` attribute for C symbol visibility?

## Deferred Implementation Detection

```bash
# Stub phase: todo!() allowed in Open and Decode
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()" src/sound/aiff_ffi.rs
# Should show: rust_aifa_Open, rust_aifa_Decode
```

## Success Criteria
- [ ] `cargo check --all-features` succeeds
- [ ] All existing tests pass (parser + PCM + SDX2 + seek)
- [ ] Vtable static defined with all 12 function pointers
- [ ] Simple FFI functions implemented (GetName, InitModule, TermModule, GetStructSize, GetError, Init, Term, Close, Seek, GetFrame)
- [ ] Complex FFI functions stubbed (Open, Decode)
- [ ] Init matches dukaud_ffi.rs pattern (no double-init)

## Failure Recovery
- Return to Phase 15 and fix compilation errors
- If TFB_SoundDecoderFuncs fields don't match, check ffi.rs
- rollback: `git checkout -- rust/src/sound/aiff_ffi.rs rust/src/sound/mod.rs`

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P15a.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P15a
- timestamp
- verification result: PASS/FAIL
- gate decision: proceed to P16 or return to P15
