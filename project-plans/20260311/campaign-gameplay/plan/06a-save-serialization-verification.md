# Phase 06a: Save Serialization Verification

## Phase ID
`PLAN-20260314-CAMPAIGN.P06a`

## Prerequisites
- Required: Phase 06 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] Serialization functions exist and compile
- [ ] Error type covers all failure modes

## Semantic Verification Checklist
- [ ] Round-trip fidelity verified for all §9.1 fields
- [ ] Queue data round-trips correctly
- [ ] Save-time adjustments tested for special contexts
- [ ] Failed saves produce errors, not silent corruption

## Gate Decision
- [ ] PASS: proceed to Phase 07
- [ ] FAIL: fix issues, re-verify

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P06a.md`
