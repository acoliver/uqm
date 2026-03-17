# Phase 11a: Encounter Handoff & Post-Encounter Verification

## Phase ID
`PLAN-20260314-CAMPAIGN.P11a`

## Prerequisites
- Required: Phase 11 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] Encounter functions exist and compile
- [ ] Battle FFI path defined

## Semantic Verification Checklist
- [ ] Battle setup correct (ships, backdrop, flagship)
- [ ] Suppress conditions all working
- [ ] Victory/escape/defeat consequences correct
- [ ] Race identification reliable
- [ ] Navigation resumes after encounter

## Gate Decision
- [ ] PASS: proceed to Phase 12
- [ ] FAIL: fix issues, re-verify

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P11a.md`
