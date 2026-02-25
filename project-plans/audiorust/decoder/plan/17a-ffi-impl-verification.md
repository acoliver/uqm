# Phase 17a: FFI Implementation Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P17a`

## Prerequisites
- Required: Phase 17 completed
- Expected: Both `aiff.rs` and `aiff_ffi.rs` feature-complete

## Verification Checklist

### Structural
- [ ] **ZERO `todo!()` in `aiff.rs`**
- [ ] **ZERO `todo!()` in `aiff_ffi.rs`**
- [ ] All tests pass: `cargo test --lib --all-features`
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes

### Semantic
- [ ] Vtable has all 12 function pointers
- [ ] GetName returns "Rust AIFF"
- [ ] InitModule/TermModule lifecycle works
- [ ] Init/Term: Box allocation/deallocation correct
- [ ] Open: format mapping via RUST_AIFA_FORMATS
- [ ] Open: base struct field updates
- [ ] Decode: result mapping (Ok→n, EndOfFile→0, Err→0)
- [ ] All null pointer paths return safe defaults

### Quality
- [ ] No `FIXME`/`HACK`/`placeholder` markers
- [ ] Logging via `rust_bridge_log_msg` in Open
- [ ] Follows existing FFI patterns from dukaud_ffi.rs/wav_ffi.rs

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Full test suite
cargo test --lib --all-features

# Zero todo check
echo "aiff.rs todo count:" && grep -c "todo!()" src/sound/aiff.rs || echo "0"
echo "aiff_ffi.rs todo count:" && grep -c "todo!()" src/sound/aiff_ffi.rs || echo "0"

# Quality
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# No forbidden markers
grep -RIn "FIXME\|HACK\|placeholder\|for now\|will be implemented" src/sound/aiff.rs src/sound/aiff_ffi.rs || echo "CLEAN"
```

## Gate Decision
- [ ] PASS: proceed to Phase 18 (C integration) — **MILESTONE: Rust code complete**
- [ ] FAIL: return to Phase 17
