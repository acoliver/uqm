# Phase 08a: Combatant Selection Contract Verification

## Phase ID
`PLAN-20260314-SUPERMELEE.P08a`

## Prerequisites
- Required: Phase 08 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features pick_melee_tests
```

## Structural Verification Checklist
- [ ] `pick_melee.rs` is implemented under `setup/`
- [ ] Selection-contract tests exist and run
- [ ] The verification gate checks the actual preceding implementation artifacts

## Semantic Verification Checklist
- [ ] Initial combatant requests yield battle-ready handoff objects for valid fleets
- [ ] Next-combatant requests yield battle-ready handoff objects or a clean no-selection result as appropriate
- [ ] Consumed ships are not reselected
- [ ] Selection commit behavior updates SuperMelee-owned selection state correctly
- [ ] The handoff contract is not weakened to bare ship IDs, slot indexes, or abstract placeholders

## Gate Decision
- [ ] PASS: proceed to Phase 09
- [ ] FAIL: fix selection-contract issues

## Phase Completion Marker
Create: `project-plans/20260311/supermelee/.completed/P08a.md`
