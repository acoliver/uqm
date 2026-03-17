# Phase 09a: Deferred Transitions & Dispatch Verification

## Phase ID
`PLAN-20260314-CAMPAIGN.P09a`

## Prerequisites
- Required: Phase 09 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] Campaign loop function exists and compiles
- [ ] All dispatch branches present

## Semantic Verification Checklist
- [ ] Dispatch priority: deferred > encounter/starbase > interplanetary > hyperspace
- [ ] Deferred transition consumed once, does not repeat
- [ ] No save-slot mutation from deferred transitions
- [ ] New-game: Sol/Interplanetary/start-date verified
- [ ] Restart: all state cleared, no carry-over
- [ ] Terminal conditions work correctly

## Gate Decision
- [ ] PASS: proceed to Phase 10
- [ ] FAIL: fix issues, re-verify

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P09a.md`
