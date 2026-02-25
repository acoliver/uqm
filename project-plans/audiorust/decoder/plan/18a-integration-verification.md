# Phase 18a: Integration Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P18a`

## Prerequisites
- Required: Phase 18 completed
- Expected files: All Rust + C integration files

## Verification Checklist

### Structural — Rust Side
- [ ] `rust/src/sound/aiff.rs` — feature-complete, zero `todo!()`
- [ ] `rust/src/sound/aiff_ffi.rs` — feature-complete, zero `todo!()`
- [ ] `rust/src/sound/mod.rs` — includes `pub mod aiff; pub mod aiff_ffi;`
- [ ] All Rust tests pass: `cargo test --lib --all-features`
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes

### Structural — C Side
- [ ] `sc2/src/libs/sound/decoders/rust_aiff.h` exists
- [ ] `decoder.c` has `#ifdef USE_RUST_AIFF` include block
- [ ] `decoder.c` has `#ifdef USE_RUST_AIFF` sd_decoders entry
- [ ] `config_unix.h.in` has `@SYMBOL_USE_RUST_AIFF_DEF@`
- [ ] `build.vars.in` has `USE_RUST_AIFF` and `SYMBOL_USE_RUST_AIFF_DEF` variables

### Semantic — Integration
- [ ] Without USE_RUST_AIFF: C build succeeds, uses original aifa_DecoderVtbl
- [ ] With USE_RUST_AIFF: C build links against rust_aifa_DecoderVtbl
- [ ] `rust_aiff.h` matches pattern of `rust_dukaud.h` exactly
- [ ] `decoder.c` `sd_decoders[]` "aif" entry correctly conditionally compiled
- [ ] Complete integration path: C mixer → decoder.c → vtable → aiff_ffi.rs → aiff.rs

### Semantic — Completeness
- [ ] All 78+ requirements (FP, SV, CH, DP, DS, SK, EH, LF, FF) implemented
- [ ] All error paths tested
- [ ] No deferred implementation markers in any Rust code
- [ ] No `FIXME`/`HACK`/`placeholder` in any code

### Quality
- [ ] Both Rust files follow existing decoder patterns (wav.rs/dukaud.rs)
- [ ] FFI follows existing FFI patterns (wav_ffi.rs/dukaud_ffi.rs)
- [ ] C integration follows existing USE_RUST_* patterns

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
echo "=== rust_aiff.h ===" && head -5 src/libs/sound/decoders/rust_aiff.h
echo "=== decoder.c USE_RUST_AIFF ===" && grep -A3 "USE_RUST_AIFF" src/libs/sound/decoders/decoder.c
echo "=== config_unix.h.in ===" && grep "AIFF" src/config_unix.h.in
echo "=== build.vars.in ===" && grep "AIFF" build.vars.in

# Count all tests
cd /Users/acoliver/projects/uqm/rust
echo "aiff.rs tests:" && grep -c "#\[test\]" src/sound/aiff.rs
echo "aiff_ffi.rs tests:" && grep -c "#\[test\]" src/sound/aiff_ffi.rs
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

## Gate Decision
- [ ] PASS: **PLAN COMPLETE** — All phases executed, all requirements implemented, integration tested
- [ ] FAIL: Return to failing phase and remediate
