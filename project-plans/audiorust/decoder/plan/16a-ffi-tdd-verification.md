# Phase 16a: FFI TDD Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P16a`

## Prerequisites
- Required: Phase 16 completed
- Expected: FFI tests in `aiff_ffi.rs`

## Verification Checklist

### Structural
- [ ] At least 12 test functions in `aiff_ffi.rs`
- [ ] Tests compile: `cargo test --lib --all-features -- aiff_ffi --no-run`

### Semantic
- [ ] Vtable existence test checks all 12 function pointers
- [ ] GetName test validates string content
- [ ] Null pointer tests for all functions taking decoder arg
- [ ] InitModule/TermModule lifecycle tested
- [ ] GetStructSize value tested

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust
cargo test --lib --all-features -- aiff_ffi --no-run
grep -c "#\[test\]" src/sound/aiff_ffi.rs
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Gate Decision
- [ ] PASS: proceed to Phase 17
- [ ] FAIL: return to Phase 16
