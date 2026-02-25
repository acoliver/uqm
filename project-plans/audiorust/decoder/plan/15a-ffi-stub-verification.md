# Phase 15a: FFI Stub Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P15a`

## Prerequisites
- Required: Phase 15 completed
- Expected files: `rust/src/sound/aiff_ffi.rs`, updated `mod.rs`

## Verification Checklist

### Structural
- [ ] `rust/src/sound/aiff_ffi.rs` exists
- [ ] `pub mod aiff_ffi;` in `mod.rs`
- [ ] `cargo check --all-features` succeeds
- [ ] `#[repr(C)] pub struct TFB_RustAiffDecoder` defined
- [ ] `#[no_mangle] pub static rust_aifa_DecoderVtbl` defined
- [ ] 12 function pointers in vtable struct

### Semantic
- [ ] GetName returns `b"Rust AIFF\0"` pointer
- [ ] InitModule stores formats in Mutex
- [ ] TermModule clears Mutex
- [ ] Init does Box::new + Box::into_raw
- [ ] Term does Box::from_raw + drop + null
- [ ] All functions with decoder arg have null check
- [ ] read_uio_file matches dukaud_ffi.rs pattern

### Quality
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust
cargo check --all-features
cargo test --lib --all-features -- aiff
grep "rust_aifa_DecoderVtbl" src/sound/aiff_ffi.rs
grep "TFB_RustAiffDecoder" src/sound/aiff_ffi.rs
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Gate Decision
- [ ] PASS: proceed to Phase 16
- [ ] FAIL: return to Phase 15
