# Phase 06a: Team Model & Persistence Verification

## Phase ID
`PLAN-20260314-SUPERMELEE.P06a`

## Prerequisites
- Required: Phase 06 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features team_tests persistence_tests config_tests
```

## Structural Verification Checklist
- [ ] `team.rs`, `persistence.rs`, and `config.rs` are implemented under `setup/`
- [ ] Matching test files exist and are wired into the crate test layout
- [ ] No persistence/config work needed for scoped requirements is deferred to unrelated later phases

## Semantic Verification Checklist
- [ ] Team mutation preserves distinct empty-slot semantics and consistent fleet values
- [ ] Built-in catalog initialization and saved-team enumeration are both verified
- [ ] Valid built-in-team load and valid saved-team load are verified separately
- [ ] Malformed/unreadable saved-team artifacts fail without corrupting active setup state
- [ ] Save-failure cleanup behavior is explicitly verified
- [ ] `melee.cfg` restore sanitizes transient network-only startup modes when required
- [ ] Legacy `.mle` loading verifies semantic interoperability rather than unsupported byte-for-byte assumptions

## Gate Decision
- [ ] PASS: proceed to Phase 07
- [ ] FAIL: fix persistence/model/config gaps

## Phase Completion Marker
Create: `project-plans/20260311/supermelee/.completed/P06a.md`
