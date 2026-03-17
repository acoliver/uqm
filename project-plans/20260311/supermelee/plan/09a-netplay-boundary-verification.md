# Phase 09a: Netplay Boundary Verification

## Phase ID
`PLAN-20260314-SUPERMELEE.P09a`

## Prerequisites
- Required: Phase 09 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features netplay_boundary_tests
```

## Structural Verification Checklist
- [ ] `netplay_boundary.rs` is implemented under `setup/`
- [ ] Netplay-boundary tests exist and run
- [ ] The verification phase checks implementation artifacts created by Phase 09

## Semantic Verification Checklist
- [ ] Local-only behavior succeeds with netplay disabled or unsupported
- [ ] Setup sync events are verified separately for slot changes, team-name changes, and whole-team bootstrap
- [ ] Start gating blocks invalid netplay start states until connection/readiness/confirmation preconditions are satisfied
- [ ] Local combatant-selection outcomes are exposed to the boundary where required
- [ ] Valid remote selections are committed and invalid remote selections are rejected without silent substitution
- [ ] Already accepted remote selections are not re-rejected later

## Gate Decision
- [ ] PASS: proceed to Phase 10
- [ ] FAIL: fix netplay-boundary gaps

## Phase Completion Marker
Create: `project-plans/20260311/supermelee/.completed/P09a.md`
