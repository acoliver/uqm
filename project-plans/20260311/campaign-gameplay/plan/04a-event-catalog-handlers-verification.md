# Phase 04a: Event Catalog & Handlers Verification

## Phase ID
`PLAN-20260314-CAMPAIGN.P04a`

## Prerequisites
- Required: Phase 04 completed
- Expected files: `rust/src/campaign/events/mod.rs`, `registration.rs`, `handlers.rs`

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] EventSelector has exactly 18 variants
- [ ] All handler functions exist and are non-trivial
- [ ] Tests exist for each handler

## Semantic Verification Checklist
- [ ] Arilou entrance/exit cycle produces indefinite alternation
- [ ] Hyperspace encounter self-reschedules daily
- [ ] Kohr-Ah victorious conditional branching correct
- [ ] Genocide targets nearest faction with Druuge tiebreaker
- [ ] Deferral handlers check homeworld presence
- [ ] Slylandro ramp-up caps at 4 and checks destruct code
- [ ] Yehat rebellion fraction is exactly 2/3 royalist

## Gate Decision
- [ ] PASS: proceed to Phase 05
- [ ] FAIL: fix issues, re-verify

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P04a.md`
