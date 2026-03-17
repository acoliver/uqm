# Phase 12a: FFI Bridge & C-Side Wiring Verification

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P12a`

## Prerequisites
- Required: Phase 12 completed

## Verification Commands

```bash
# Rust
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p uqm --lib planets::tests::ffi_tests --all-features -- --nocapture

# C (no regression)
cd sc2 && make clean && make

# C with Rust planets enabled
# Reconfigure with USE_RUST_PLANETS=1 and rebuild
```

## Structural Verification Checklist
- [ ] `ffi.rs` — no `todo!()` or `unimplemented!()`
- [ ] `rust_planets.h` declares all exported and imported functions
- [ ] `rust_planets.c` implements all C-side shims
- [ ] All replaced C files have `#ifndef USE_RUST_PLANETS` guards
- [ ] Build config has `USE_RUST_PLANETS` toggle
- [ ] Every actual boundary-crossing Rust type has a concrete `#[repr(C)]` mirror or an explicit scalar ABI representation

## Semantic Verification Checklist
- [ ] `USE_RUST_PLANETS=0` build is identical behavior to pre-plan (no regression)
- [ ] `USE_RUST_PLANETS=1` build links and compiles
- [ ] C `GenerateFunctions` accessible from Rust for dedicated and default systems
- [ ] Type sizes/layouts match across FFI boundary (`sizeof` or equivalent checks)
- [ ] Override/fallback semantics preserved through FFI
- [ ] Data-provider count/per-node semantics preserved through FFI
- [ ] Side-effect hook dispatch preserved through FFI
- [ ] External menu FFI dispatch compiles and links
- [ ] System-specific C generators remain compilable
- [ ] Per-star dispatch identity is verified for representative dedicated systems

## Deferred Implementation Detection

```bash
grep -RIn "todo!()\|unimplemented!()\|FIXME\|HACK" rust/src/planets/ffi.rs
# Must return 0
```

## Gate Decision
- [ ] PASS: proceed to Phase 13
- [ ] FAIL: fix FFI bridge
