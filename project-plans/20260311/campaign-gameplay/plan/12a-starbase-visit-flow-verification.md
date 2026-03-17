# Phase 12a: Starbase Visit Flow Verification

## Phase ID
`PLAN-20260314-CAMPAIGN.P12a`

## Prerequisites
- Required: Phase 12 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] Starbase functions exist and compile
- [ ] Progression-point types fully defined

## Semantic Verification Checklist
- [ ] Bomb-transport and Ilwrath response special sequences work
- [ ] Forced conversations fire at correct times
- [ ] Save/load resume matches progression-point contract
- [ ] Mandatory-next-action rule applied correctly
- [ ] Departure uses deferred transition

## Gate Decision
- [ ] PASS: proceed to Phase 13
- [ ] FAIL: fix issues, re-verify

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P12a.md`
