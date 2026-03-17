# Phase 08a: Legacy Save Compatibility Verification

## Phase ID
`PLAN-20260314-CAMPAIGN.P08a`

## Prerequisites
- Required: Phase 08 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] Legacy reader exists and compiles
- [ ] Format detection integrated into load path

## Semantic Verification Checklist
- [ ] Legacy save fixture data parses correctly
- [ ] All covered context save types load from legacy format
- [ ] End-state saves round-trip correctly
- [ ] Corrupt legacy saves are rejected safely

## Gate Decision
- [ ] PASS: proceed to Phase 09
- [ ] FAIL: fix issues, re-verify

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P08a.md`
