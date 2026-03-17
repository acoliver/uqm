# Phase 05a: Two-Tier Loader Verification

## Phase ID
`PLAN-20260314-SHIPS.P05a`

## Prerequisites
- Required: Phase 05 (Two-Tier Loader) completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `loader.rs` exports `load_ship()`, `free_ship()`, `LoadTier`, `ShipError`
- [ ] `c_bridge.rs` exports resource loading/freeing bridge functions
- [ ] No compilation errors

## Semantic Verification Checklist
- [ ] Metadata-only load produces descriptor without battle assets
- [ ] Battle-ready load produces descriptor with all assets
- [ ] `free_ship()` invokes teardown hook
- [ ] `free_ship()` frees appropriate assets based on parameters
- [ ] Load failure returns Err and cleans up partial loads
- [ ] Resource bridge functions have correct extern "C" signatures

## Gate Decision
- [ ] PASS: proceed to Phase 06
- [ ] FAIL: return to Phase 05 and fix issues
