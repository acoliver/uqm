# Phase 05: Core Types & State Machine — Implementation

## Phase ID
`PLAN-20260314-NETPLAY.P05`

## Prerequisites
- Required: Phase 04a (Core Types TDD Verification) completed and passed
- Expected files: test modules in state.rs, error.rs, options.rs, constants.rs

## Requirements Implemented (Expanded)

### REQ-NP: Connection State Transitions
**Requirement text**: The subsystem shall maintain connection state sufficient to distinguish all 10 phases.

Behavior contract:
- GIVEN: `NetState::validate_transition(from, to)` is called
- WHEN: The transition is in the valid set
- THEN: `Ok(())` is returned
- WHEN: The transition is NOT in the valid set
- THEN: `Err(InvalidTransition { from, to })` is returned

## Implementation Tasks

### Files to modify

- `rust/src/netplay/state.rs` — Implement all methods
  - marker: `@plan PLAN-20260314-NETPLAY.P05`
  - marker: `@requirement REQ-NP: connection state transitions`
  - Implement:
    - `NetState::validate_transition(from, to) -> Result<(), NetplayError>` — full transition table
    - `is_handshake_meaningful()` — returns `*self == NetState::InSetup`
    - `is_ready_meaningful()` — match on 7 valid states
    - `is_battle_active()` — match on 3 states
    - `name()` — static string for each variant
    - `impl Display for NetState`
    - `impl Default for NetState` — `Unconnected`
    - `impl Default for StateFlags` — all false/0
    - `StateFlags::clear_handshake(&mut self)` — reset handshake flags
    - `StateFlags::clear_ready(&mut self)` — reset ready flags
    - `StateFlags::clear_reset(&mut self)` — reset reset flags
    - `StateFlags::clear_all(&mut self)` — reset everything to defaults
  - Pseudocode lines: 1-31

- `rust/src/netplay/error.rs` — Implement Display, From
  - marker: `@plan PLAN-20260314-NETPLAY.P05`
  - Implement:
    - `thiserror::Error` derive on `NetplayError`
    - `impl Display` for `AbortReason`, `ResetReason`
    - `impl From<std::io::Error> for NetplayError`
    - Numeric conversion methods for wire compatibility:
      - `AbortReason::from_u16(v: u16) -> Option<AbortReason>`
      - `AbortReason::to_u16(&self) -> u16`
      - `ResetReason::from_u16(v: u16) -> Option<ResetReason>`
      - `ResetReason::to_u16(&self) -> u16`

- `rust/src/netplay/options.rs` — Implement defaults
  - marker: `@plan PLAN-20260314-NETPLAY.P05`
  - Implement:
    - `impl Default for PeerOptions` — host "localhost", port 21837, is_server true
    - `impl Default for NetplayOptions` — 2 default peers, input_delay 2

- `rust/src/netplay/constants.rs` — Already complete from stub (constants are values)
  - Verify all values match C source

### Pseudocode traceability
- Uses pseudocode lines: 1-31 (state machine), 62-92 (connection struct fields that use StateFlags)

## Verification Commands

```bash
# All gates must pass
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] All `todo!()` removed from state.rs, error.rs, options.rs
- [ ] No stub methods remain in these files
- [ ] `@plan` and `@requirement` markers present

## Semantic Verification Checklist (Mandatory)
- [ ] ALL tests from Phase 04 now pass
- [ ] `validate_transition` rejects `(InBattle, InSetup)` without going through reset
- [ ] `validate_transition` allows `(any, Unconnected)` for disconnect
- [ ] `validate_transition` allows `(any_gameplay, InSetup)` for reset
- [ ] `AbortReason::from_u16` round-trips with `to_u16`
- [ ] `ResetReason::from_u16` round-trips with `to_u16`
- [ ] Default `StateFlags` has all booleans false and all numbers 0

## Deferred Implementation Detection (Mandatory)

```bash
# Must find NO matches in implementation code (tests excluded)
grep -RIn "TODO\|FIXME\|HACK\|todo!()\|unimplemented!()\|placeholder" rust/src/netplay/state.rs rust/src/netplay/error.rs rust/src/netplay/options.rs rust/src/netplay/constants.rs | grep -v "#\[cfg(test)\]" | grep -v "mod tests"
```

## Success Criteria
- [ ] All Phase 04 tests pass
- [ ] No `todo!()` remains in implemented files
- [ ] `cargo test --all-features` clean
- [ ] `cargo clippy --all-features` clean

## Failure Recovery
- rollback: `git checkout -- rust/src/netplay/state.rs rust/src/netplay/error.rs rust/src/netplay/options.rs`
- blocking issues: transition table edge cases, thiserror derive issues

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P05.md`
