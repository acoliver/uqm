# Phase 09a: DoCommunication Stub Verification

## Phase ID
`PLAN-20260325-COMMPT3.P09a`

## Prerequisites
- Required: Phase P09 completed
- Expected artifacts: Modified `ffi.rs`, `talk_segue.rs` with new enum and stubs

## Verification Commands

```bash
cd rust && cargo check --workspace --all-features
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify enum shape
grep -A10 "enum CommunicationResult" rust/src/comm/talk_segue.rs

# Verify dispatch structure
grep -A25 "fn rust_DoCommunication" rust/src/comm/ffi.rs
```

## Structural Verification Checklist
- [ ] `CommunicationResult` has 4 variants (Talking, ResponseContinue, Selected, Done)
- [ ] `do_communication` returns `CommunicationResult` (stub: always `Talking`)
- [ ] `select_response` returns `Option<..>` (stub: always `None`)
- [ ] `rust_DoCommunication` matches all 4 arms
- [ ] Project compiles, tests pass

## Semantic Verification Checklist (Mandatory)
- [ ] `do_communication` stub does NOT implement real state machine logic
- [ ] `select_response` stub does NOT extract callback tuples
- [ ] Selected arm contains `todo!()` or equivalent (unreachable with current stub)

## Semantic Negative-Proof Gate (Mandatory)
- [ ] `do_communication` always returns `Talking` — never returns `Selected` or
  `Done` based on state (confirms TDD phase P10 is needed)
- [ ] `select_response` always returns `None` — never returns a callback tuple
  (confirms P10 TDD will define expectations that fail against this)
- [ ] Lock discipline is NOT yet verified — `drop(state)` may not be in correct
  position (confirms P10/P11 are needed)

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P09a.md`
