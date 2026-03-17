# Phase 03a: Types & Domain Model Verification

## Phase ID
`PLAN-20260314-CAMPAIGN.P03a`

## Prerequisites
- Required: Phase 03 completed
- Expected files: `rust/src/campaign/mod.rs`, `activity.rs`, `types.rs`, `session.rs`

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] All four source files created
- [ ] `lib.rs` updated with `pub mod campaign`
- [ ] No compilation errors
- [ ] Plan markers present in source files

## Semantic Verification Checklist
- [ ] `CampaignActivity` has exactly 5 variants matching §3.1
- [ ] `TransitionFlags` covers start_encounter, start_interplanetary, check_load, check_restart, check_abort
- [ ] `CampaignSession::new()` produces valid default state
- [ ] `CampaignSession::clear()` resets all fields without leaving stale state
- [ ] C flag constants match `globdata.h` values (cross-reference manually)
- [ ] Enum serialization produces lowercase_with_underscores strings
- [ ] Tests fail when behavior is broken (mutation testing or manual verification)

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/campaign/
```

## Gate Decision
- [ ] PASS: proceed to Phase 04
- [ ] FAIL: fix issues, re-verify

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P03a.md`
