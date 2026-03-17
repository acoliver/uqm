# Phase 04a: RNG & World Classification Verification

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P04a`

## Prerequisites
- Required: Phase 04 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
# Run only planet tests for focused verification:
cargo test -p uqm --lib planets:: --all-features
```

## Structural Verification Checklist
- [ ] `rng.rs` has no `todo!()` or `unimplemented!()`
- [ ] `world_class.rs` has no `todo!()` or `unimplemented!()`
- [ ] Test files exist: `rng_tests.rs`, `world_class_tests.rs`
- [ ] Tests cover: seed determinism, star seed derivation, classification, indexing, matching

## Semantic Verification Checklist
- [ ] RNG sequence for seed 0x12345678 matches captured C reference
- [ ] `get_random_seed_for_star` for Sol matches C output
- [ ] `world_is_planet` returns true for PlanetDesc entries, false for MoonDesc entries
- [ ] `planet_index` returns 0-based index within PlanetDesc array
- [ ] `moon_index` returns 0-based index within MoonDesc array
- [ ] `match_world` with MATCH_PLANET correctly matches planets regardless of moon_i

## Gate Decision
- [ ] PASS: proceed to Phase 05
- [ ] FAIL: fix issues and re-verify
