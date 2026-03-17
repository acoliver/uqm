# Phase 03a: Core Types & Error — Stub Verification

## Phase ID
`PLAN-20260314-SUPERMELEE.P03a`

## Prerequisites
- Required: Phase 03 completed
- Expected files: `rust/src/supermelee/{mod.rs, types.rs, error.rs, setup/mod.rs, setup/team.rs}`

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] All expected new files exist
- [ ] `rust/src/lib.rs` contains `pub mod supermelee`
- [ ] Module hierarchy compiles: `supermelee::{types, error, setup}`
- [ ] No compilation errors

## Semantic Verification Checklist
- [ ] `MeleeShip::from_u8()` conversion exists or is explicitly scheduled for the next phase
- [ ] Empty-slot sentinel is explicit and distinct from occupied ship IDs
- [ ] `SuperMeleeError` derives `Debug` and integrates with the project's error pattern
- [ ] `MeleeTeam::default()` initializes all slots to the empty sentinel
- [ ] Constants match audited C values: `MELEE_FLEET_SIZE == 14`, `NUM_SIDES == 2`
- [ ] Any battle-facing combatant wrapper/type is labeled as a design placeholder rather than a verified final signature

## Gate Decision
- [ ] PASS: proceed to Phase 04
- [ ] FAIL: fix compilation/type issues

## Phase Completion Marker
Create: `project-plans/20260311/supermelee/.completed/P03a.md`
