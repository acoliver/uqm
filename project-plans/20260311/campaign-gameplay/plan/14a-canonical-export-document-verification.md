# Phase 14a: Canonical Export Document Verification

## Phase ID
`PLAN-20260314-CAMPAIGN.P14a`

## Prerequisites
- Required: Phase 14 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] Export function compiles and produces JSON
- [ ] All 8 required sections present

## Semantic Verification Checklist
- [ ] Summary normalization correct per §9.2 for all covered contexts
- [ ] resume_context fields use exact closed vocabularies
- [ ] Scheduled events use catalog selectors
- [ ] Faction state covers 17 baseline factions
- [ ] Error export correct for malformed saves
- [ ] Deterministic output

## Gate Decision
- [ ] PASS: proceed to Phase 15
- [ ] FAIL: fix issues, re-verify

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P14a.md`
