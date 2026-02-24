> **NOTE**: This file's name is a historical artifact from a phase reorder.
> Canonical phase: **P21a** (Colormap FFI Verification).
> Phase P22 is now in `22-guards-level0.md`.

# Phase 21a: Colormap FFI — Verification

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P21a`

## Prerequisites
- Required: Phase P21 (Colormap FFI Bridge) completed
- Expected: Colormap FFI exports compiled and linked
- Expected: ~8 `rust_cmap_*` symbols exported

## Requirements Verified

### REQ-CMAP-010–030: Colormap FFI Correctness
Verification:
- Run colormap unit tests in `cmap_ffi.rs`
- Verify fade step produces correct fade_amount values
- Verify colormap set/get round-trip

## Verification Tasks

### Task 1: Rust Test Suite

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Colormap-specific tests
cd rust && cargo test --lib -- cmap_ffi::tests --nocapture
```

### Task 2: Symbol Verification

```bash
# Verify all expected Rust colormap symbols are exported
cd rust && cargo build --release
nm -gU target/release/libuqm_rust.a 2>/dev/null | grep rust_cmap_ | sort
# Expected: >= 8 symbols
```

### Task 3: catch_unwind Coverage

```bash
# Verify catch_unwind on all exports
grep -c 'catch_unwind' rust/src/graphics/cmap_ffi.rs
# Expected: >= 8
```

## Structural Verification Checklist
- [ ] All ~8 colormap FFI exports present and linkable
- [ ] Each export has `catch_unwind` wrapper
- [ ] All Rust tests pass
- [ ] `cargo fmt`, `cargo clippy` pass

## Semantic Verification Checklist (Mandatory)
- [ ] Colormap fade_amount values in [0, 511] range
- [ ] Colormap set/get round-trip preserves data
- [ ] FFI signatures match C function expectations

## Success Criteria
- [ ] All colormap FFI tests pass
- [ ] All cargo gates pass
- [ ] Symbols correctly exported

## Failure Recovery
- rollback: `git checkout -- rust/src/graphics/cmap_ffi.rs`
- blocking issues: colormap API mismatch — fix signatures in cmap_ffi.rs

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P21a.md`

Contents:
- phase ID: P21a
- timestamp
- symbol count: N colormap FFI exports
- test results: all pass
