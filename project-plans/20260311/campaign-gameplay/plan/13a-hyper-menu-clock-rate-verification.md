# Phase 13a: Hyperspace Menu & Clock Rate Policy Verification

## Phase ID
`PLAN-20260314-CAMPAIGN.P13a`

## Prerequisites
- Required: Phase 13 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] Menu and hyperspace runtime functions exist and compile
- [ ] Loop dispatch updated

## Semantic Verification Checklist
- [ ] Menu choices all work correctly
- [ ] Transition flags exit menu immediately
- [ ] Save/load from menu works
- [ ] Clock rate policy enforced per activity

## Gate Decision
- [ ] PASS: proceed to Phase 14
- [ ] FAIL: fix issues, re-verify

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P13a.md`
