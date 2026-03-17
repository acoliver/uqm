# Phase 07a: Load Deserialization & Validation Verification

## Phase ID
`PLAN-20260314-CAMPAIGN.P07a`

## Prerequisites
- Required: Phase 07 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] Deserialization and validation modules created
- [ ] All error paths lead to LoadError, not panics

## Semantic Verification Checklist
- [ ] §9.4.1 rejection case 1: unknown event selector causes load failure
- [ ] §9.4.1 rejection case 2: structurally invalid metadata causes load failure
- [ ] Safe-failure: no partial state application (tested with corrupt saves)
- [ ] Safe-failure: in-session load failure preserves running session
- [ ] Resume mode correct for hyperspace, interplanetary, starbase, encounter, last-battle saves
- [ ] Adjunct validation correct per §9.4.0b table

## Gate Decision
- [ ] PASS: proceed to Phase 08
- [ ] FAIL: fix issues, re-verify

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P07a.md`
