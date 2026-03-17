# Phase 08a: Shared Runtime Pipeline Verification

## Phase ID
`PLAN-20260314-SHIPS.P08a`

## Prerequisites
- Required: Phase 08 (Runtime Pipeline) completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `runtime.rs` exports preprocess/postprocess/thrust/collision
- [ ] Pipeline steps are in correct order
- [ ] ElementState covers all needed fields

## Semantic Verification Checklist
- [ ] Energy regeneration: timing, increment, cap at max
- [ ] Turn: LEFT/RIGHT with turn_wait
- [ ] Thrust: acceleration, velocity cap, coast without input
- [ ] Weapon fire: conditions checked, energy deducted, cooldown set
- [ ] Special: conditions checked appropriately
- [ ] Cooldown: decrements each frame
- [ ] First-frame: APPEARING → setup → cleared
- [ ] Determinism: identical inputs → identical outputs
- [ ] Collision override dispatch works
- [ ] Default collision handles planet/crew/projectile

## Gate Decision
- [ ] PASS: proceed to Phase 09
- [ ] FAIL: return to Phase 08 and fix issues
