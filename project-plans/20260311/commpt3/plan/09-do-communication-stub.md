# Phase 09: DoCommunication Response Dispatch — Stub

## Phase ID
`PLAN-20260325-COMMPT3.P09`

## Prerequisites
- Required: Phase P08a (Subtitle Display Impl Verification) completed
- Expected: subtitle bridges fully implemented and verified

## Requirements Addressed
- REQ-RL-001..004, REQ-DC-001..005 (stubs only — behavior in P11)

## Purpose
Create compile-safe skeletons for the new `CommunicationResult` enum and
updated function signatures. Wire `rust_DoCommunication` to use the new
enum with stub/todo arms.

## Stub Tasks

### Rust types (talk_segue.rs)
- Change `CommunicationResult` enum: add `Talking`, `ResponseContinue`,
  `Selected(extern "C" fn(u32), u32)` variants. Keep `Done`.
  Temporarily keep old `Continue` as alias if needed for compilation.
- Stub `do_communication()` to return `Talking` unconditionally
- Stub `select_response()` to return `None` always

### Rust dispatch (ffi.rs)
- Update `rust_DoCommunication()` to match on all 4 new variants
  with `todo!()` for the Selected arm's callback dispatch
- Keep existing behavior working for the non-Selected paths

### Allowed
- `todo!()` in the Selected arm (unreachable with stubbed do_communication)
- Temporary `Continue` alias if needed for compilation

### Not Allowed
- Fake success behavior
- Removing existing working behavior prematurely

## Pseudocode Traceability
- `CommunicationResult` enum: pseudocode `003-do-communication-rewrite.md` lines 01-06
- `do_communication` stub: pseudocode `003-do-communication-rewrite.md` lines 07-40 (structure only)
- `rust_DoCommunication` dispatch stub: pseudocode `003-do-communication-rewrite.md` lines 41-64 (structure only)

## Traceability Markers (in code)
```rust
/// @plan PLAN-20260325-COMMPT3.P09
/// @requirement REQ-DC-001, REQ-RL-001
/// @pseudocode 003-do-communication-rewrite lines 01-64
```

## Implementation Tasks

### Files to modify
- `rust/src/comm/talk_segue.rs` — new enum variants, stub `do_communication` and `select_response`
- `rust/src/comm/ffi.rs` — updated match arms in `rust_DoCommunication`

### Files to create
- None

## Verification Commands

```bash
# Build gate
cd rust && cargo check --workspace --all-features
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings

# Verify new enum variants exist
grep -A10 "enum CommunicationResult" rust/src/comm/talk_segue.rs

# Verify rust_DoCommunication matches on new variants
grep -A20 "fn rust_DoCommunication\|CommunicationResult" rust/src/comm/ffi.rs

# Existing tests may need updating for new enum — update to compile
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `CommunicationResult` has `Talking`, `ResponseContinue`, `Selected(..)`, `Done` variants
- [ ] `do_communication` compiles (returns `Talking` stub)
- [ ] `select_response` compiles (returns `None` stub)
- [ ] `rust_DoCommunication` matches on all 4 variants
- [ ] Project compiles

## Success Criteria
- [ ] Both build modes compile
- [ ] Existing tests updated for new enum (compile + pass)
- [ ] No functional behavior change yet

## Failure Recovery
- rollback: `git restore rust/src/comm/ffi.rs rust/src/comm/talk_segue.rs`

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P09.md`
