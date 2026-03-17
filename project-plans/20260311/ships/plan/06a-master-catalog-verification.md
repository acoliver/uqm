# Phase 06a: Master Ship Catalog Verification

## Phase ID
`PLAN-20260314-SHIPS.P06a`

## Prerequisites
- Required: Phase 06 (Master Catalog) completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `catalog.rs` exports all lookup functions
- [ ] `game_init/master.rs` delegates to `ships::catalog`
- [ ] No compilation errors

## Semantic Verification Checklist
- [ ] Catalog contains exactly 25 melee-eligible species
- [ ] No non-melee ships in catalog (SIS, SaMatra, Probe)
- [ ] Sort order matches C behavior (alphabetical by race name)
- [ ] All lookup accessors work correctly
- [ ] Load/free lifecycle is clean (no resource leaks)
- [ ] Thread safety via Mutex works correctly

## Gate Decision
- [ ] PASS: proceed to Phase 07
- [ ] FAIL: return to Phase 06 and fix issues
