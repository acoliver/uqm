# Phase 03.5a: C-State Accessor Bridge Verification

## Phase ID
`PLAN-20260314-CAMPAIGN.P03.5a`

## Prerequisites
- Required: Phase 03.5 completed
- Expected files: `rust/src/campaign/state_bridge.rs`, `sc2/src/uqm/campaign_state_bridge.h`, `sc2/src/uqm/campaign_state_bridge.c`

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] Bridge module and C bridge files created
- [ ] Every accessor has seam-inventory backing from P01
- [ ] Ownership mode documented per accessor (`read_only`, `write_through`, `staged_commit`, `leave_in_c`)
- [ ] No speculative queue/global accessor surface remains

## Semantic Verification Checklist
- [ ] Activity globals can be inspected and mutated through explicit bridge APIs
- [ ] Queue readers/writers preserve intended source-of-truth ownership
- [ ] Snapshot/rollback behavior preserves pre-load state on simulated failure
- [ ] Starbase marker bridge supports both save/load and runtime dispatch needs
- [ ] Adjunct dependency classification matches the context-indexed rule set needed later by P07/P14/P16

## Gate Decision
- [ ] PASS: proceed to Phase 04
- [ ] FAIL: fix issues, re-verify

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P03.5a.md`
