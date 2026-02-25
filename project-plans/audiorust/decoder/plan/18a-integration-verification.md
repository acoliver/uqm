# Phase 18a: Integration Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P18a`

## Prerequisites
- Required: Phase 18 completed
- Expected files: All Rust files (aiff.rs, aiff_ffi.rs, mod.rs) + C integration files (rust_aiff.h, decoder.c, config_unix.h.in, build.vars.in)

## Verification Commands

```bash
# Full Rust verification
cd /Users/acoliver/projects/uqm/rust
cargo test --lib --all-features
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Zero deferred markers
grep -RIn "todo!()\|FIXME\|HACK\|placeholder\|for now\|will be implemented" src/sound/aiff.rs src/sound/aiff_ffi.rs || echo "ALL CLEAN"

# C build (without USE_RUST_AIFF for regression check)
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm

# Verify integration files
echo "=== rust_aiff.h ===" && head -5 sc2/src/libs/sound/decoders/rust_aiff.h
echo "=== decoder.c USE_RUST_AIFF ===" && grep -A3 "USE_RUST_AIFF" sc2/src/libs/sound/decoders/decoder.c
echo "=== config_unix.h.in ===" && grep "AIFF" sc2/src/config_unix.h.in
echo "=== build.vars.in ===" && grep "AIFF" sc2/build.vars.in

# Count all tests
cd /Users/acoliver/projects/uqm/rust
echo "aiff.rs tests:" && grep -c "#\[test\]" src/sound/aiff.rs
echo "aiff_ffi.rs tests:" && grep -c "#\[test\]" src/sound/aiff_ffi.rs
```

## Structural Verification Checklist

### Rust Side
- [ ] `rust/src/sound/aiff.rs` — feature-complete, zero `todo!()`
- [ ] `rust/src/sound/aiff_ffi.rs` — feature-complete, zero `todo!()`
- [ ] `rust/src/sound/mod.rs` — includes `pub mod aiff; pub mod aiff_ffi;`
- [ ] All Rust tests pass: `cargo test --lib --all-features`
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes

### C Side
- [ ] `sc2/src/libs/sound/decoders/rust_aiff.h` exists
- [ ] `decoder.c` has `#ifdef USE_RUST_AIFF` include block
- [ ] `decoder.c` has `#ifdef USE_RUST_AIFF` sd_decoders entry for `"aif"`
- [ ] `config_unix.h.in` has `@SYMBOL_USE_RUST_AIFF_DEF@`
- [ ] `build.vars.in` has `USE_RUST_AIFF` and `SYMBOL_USE_RUST_AIFF_DEF` variables

## Semantic Verification Checklist (Mandatory)

### Deterministic Checks
- [ ] Without `USE_RUST_AIFF`: C build succeeds, uses original `aifa_DecoderVtbl` — no regression
- [ ] With `USE_RUST_AIFF`: C build links against `rust_aifa_DecoderVtbl`
- [ ] `rust_aiff.h` matches pattern of `rust_dukaud.h` exactly (include guard, extern declaration, `#ifdef` wrapping)
- [ ] `decoder.c` `sd_decoders[]` "aif" entry correctly conditionally compiled with `#ifdef USE_RUST_AIFF` / `#else` / `#endif`
- [ ] All 84 requirements (REQ-FP-1..15, REQ-SV-1..13, REQ-CH-1..7, REQ-DP-1..6, REQ-DS-1..8, REQ-SK-1..4, REQ-EH-1..6, REQ-LF-1..10, REQ-FF-1..15) implemented
- [ ] No `todo!()`, `FIXME`, `HACK`, or `placeholder` in any Rust code
- [ ] FFI Init function does NOT call init_module()/init() (matching dukaud_ffi.rs pattern)

### Subjective Checks
- [ ] If a user builds with `USE_RUST_AIFF=1` and plays a game level with `.aif` audio, will they hear correct audio playback?
- [ ] Does the complete integration path C mixer → decoder.c → vtable → aiff_ffi.rs → aiff.rs work end-to-end without any missing links?
- [ ] Does the conditional compilation ensure that disabling `USE_RUST_AIFF` produces exactly the same binary as before this change?
- [ ] Are all C-side integration changes minimal and reversible (only adding `#ifdef` blocks, not modifying existing logic)?
- [ ] Does the `rust_aiff.h` header follow the exact same conventions as other `rust_*.h` headers in the same directory?
- [ ] Will the Rust AIFF decoder produce identical audio output to the C `aiffaud.c` for all valid AIFF/AIFC input files?

### Completeness
- [ ] All error paths tested
- [ ] Both Rust files follow existing decoder patterns (wav.rs/dukaud.rs)
- [ ] FFI follows existing FFI patterns (wav_ffi.rs/dukaud_ffi.rs)
- [ ] C integration follows existing `USE_RUST_*` patterns

## Deferred Implementation Detection

```bash
# Final check: NO deferred markers anywhere
cd /Users/acoliver/projects/uqm/rust && grep -RIn "todo!()\|unimplemented!()\|FIXME\|HACK\|placeholder\|for now\|will be implemented" src/sound/aiff.rs src/sound/aiff_ffi.rs || echo "ALL CLEAN"
```

## Success Criteria
- [ ] All Rust tests pass
- [ ] C build succeeds without `USE_RUST_AIFF` (no regression)
- [ ] All 84 requirements implemented and tested
- [ ] No deferred implementation markers
- [ ] Integration path complete and verified
- [ ] **PLAN COMPLETE**

## Failure Recovery
- Return to failing phase and remediate
- C-side rollback:
  ```bash
  git checkout -- sc2/src/libs/sound/decoders/decoder.c
  git checkout -- sc2/src/config_unix.h.in
  git checkout -- sc2/build.vars.in
  rm sc2/src/libs/sound/decoders/rust_aiff.h
  ```
- Rust-side rollback:
  ```bash
  git checkout -- rust/src/sound/aiff.rs
  git checkout -- rust/src/sound/aiff_ffi.rs
  git checkout -- rust/src/sound/mod.rs
  ```

## Final Plan Evaluation Checklist

- [ ] Uses plan ID (PLAN-20260225-AIFF-DECODER) + sequential phases (P01–P18)
- [ ] Preflight verification defined and executable
- [ ] All requirements expanded and testable (GIVEN/WHEN/THEN)
- [ ] Integration points explicit (decoder.c, config_unix.h.in, build.vars.in, rust_aiff.h)
- [ ] Legacy code replacement explicit (aifa_DecoderVtbl → rust_aifa_DecoderVtbl under USE_RUST_AIFF)
- [ ] Pseudocode line references present in implementation phases
- [ ] Verification phases include semantic checks
- [ ] Lint/test gates defined for every phase
- [ ] No reliance on placeholder completion

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P18a.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P18a
- timestamp
- verification result: PASS/FAIL — **PLAN COMPLETE** if PASS
- total test count across both Rust files
- gate decision: **PLAN COMPLETE** or return to failing phase
- **MILESTONE**: AIFF decoder Rust port — full plan execution complete
