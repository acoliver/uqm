# Phase 07a: Setup Menu & Fleet Ship Pick Verification

## Phase ID
`PLAN-20260314-SUPERMELEE.P07a`

## Prerequisites
- Required: Phase 07 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features melee_tests build_pick_tests
```

## Structural Verification Checklist
- [ ] `melee.rs` and `build_pick.rs` are implemented under `setup/`
- [ ] Setup/menu and picker tests exist and are runnable
- [ ] The phase verifies actual setup/menu artifacts rather than unrelated battle-engine modules

## Semantic Verification Checklist
- [ ] Entry and fallback initialization produce a usable menu state
- [ ] Cancelled ship-pick and other transient subviews return cleanly without committing edits
- [ ] Confirmed fleet-edit selection updates the active team state
- [ ] Match start is blocked for unplayable fleets and permitted for valid local fleets
- [ ] Post-battle return restores a valid menu/setup state
- [ ] Exit path persists setup state and releases SuperMelee-owned resources

## Gate Decision
- [ ] PASS: proceed to Phase 08
- [ ] FAIL: fix setup/menu/picker gaps

## Phase Completion Marker
Create: `project-plans/20260311/supermelee/.completed/P07a.md`
