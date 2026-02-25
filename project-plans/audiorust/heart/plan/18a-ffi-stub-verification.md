# Phase 18a: FFI Stub Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P18a`

## Prerequisites
- Required: Phase P18 completed
- Expected files: `rust/src/sound/heart_ffi.rs`, updated `mod.rs`

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo check --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm
```

## Structural Verification Checklist
- [ ] `rust/src/sound/heart_ffi.rs` exists
- [ ] `mod.rs` updated with `pub mod heart_ffi;`
- [ ] `@plan PLAN-20260225-AUDIO-HEART.P18` marker present
- [ ] `cargo check` passes
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes

## Semantic Verification Checklist

### Deterministic checks
- [ ] 60+ FFI functions defined: `grep -c "#\[no_mangle\]" rust/src/sound/heart_ffi.rs` >= 60
- [ ] All functions use `extern "C"`: `grep -c 'extern "C"' rust/src/sound/heart_ffi.rs` >= 60
- [ ] C-compatible return types: `grep -c "c_int\|c_uint\|c_void\|c_char\|bool" rust/src/sound/heart_ffi.rs` >= 20
- [ ] `USE_RUST_AUDIO_HEART` or `cfg` conditional: `grep -c "cfg\|USE_RUST" rust/src/sound/heart_ffi.rs` >= 1

### Subjective checks
- [ ] All 60+ C API functions from rust-heart.md have matching #[no_mangle] pub extern "C" stubs
- [ ] Function signatures match C header types exactly — are `*const c_char`, `*mut c_void`, `c_int`, `BOOLEAN` used correctly?
- [ ] All stubs use `todo!()` — no fake success behavior
- [ ] `SpliceTrack` accepts `*const u16` (UNICODE*) — is the parameter type correct for UTF-16?
- [ ] `PlayChannel` accepts `*mut c_void` (opaque SOUND handle) — is the type correctly opaque?
- [ ] CCallbackWrapper struct defined for wrapping C function pointers

### GIVEN/WHEN/THEN contracts
- GIVEN the FFI module is compiled, WHEN a C linker resolves `PLRPlaySong`, THEN it finds the symbol
- GIVEN the FFI module is compiled, WHEN it imports from music.rs, sfx.rs, control.rs, fileinst.rs, THEN all imports resolve
- GIVEN `#[no_mangle]` is on each function, WHEN `nm` is run on the static library, THEN all expected symbols appear

## Deferred Implementation Detection

```bash
grep -n "todo!()" rust/src/sound/heart_ffi.rs | wc -l
# Should be >= 60 (all stubs)
grep -n "TODO\|FIXME\|HACK\|placeholder" rust/src/sound/heart_ffi.rs
# Should return 0 results
```

## Success Criteria
- [ ] All 60+ FFI stubs compile
- [ ] Module registered in mod.rs
- [ ] C build links (even though stubs will crash at runtime)
- [ ] Function names match C header expectations

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/mod.rs` and `rm rust/src/sound/heart_ffi.rs`
- blocking issues: If C header signatures don't match, reconcile before proceeding

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P18a.md`
