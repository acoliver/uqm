# Phase 04: Core Types & State Machine — TDD

## Phase ID
`PLAN-20260314-NETPLAY.P04`

## Prerequisites
- Required: Phase 03a (Core Types Stub Verification) completed and passed
- Expected files from previous phase: all stub files from P03

## Requirements Implemented (Expanded)

### REQ-NP: Connection State Transitions
**Behavior contract**:
- GIVEN: A connection in state X
- WHEN: A transition to state Y is requested
- THEN: The transition succeeds if valid, fails with `InvalidTransition` if not

### REQ-NP: State Predicates
**Behavior contract**:
- GIVEN: A connection in any state
- WHEN: A predicate is queried
- THEN: The result matches the documented state sets

## Implementation Tasks

### Files to create

- `rust/src/netplay/state.rs` — Add `#[cfg(test)] mod tests` block
  - marker: `@plan PLAN-20260314-NETPLAY.P04`
  - marker: `@requirement REQ-NP: connection state transitions`
  - Tests to write:
    - `test_handshake_meaningful_only_in_setup` — verify only `InSetup` returns true
    - `test_ready_meaningful_states` — verify exactly 7 states return true
    - `test_battle_active_states` — verify exactly 3 states return true
    - `test_state_names_unique` — verify all 10 states have unique debug names
    - `test_state_default` — verify default state is `Unconnected`
    - `test_state_flags_default` — verify all flags default to false/0
    - `test_state_transition_valid_happy_path` — test each valid transition
    - `test_state_transition_invalid` — test disallowed transitions return error
    - `test_state_transition_reset_from_any_gameplay` — test InSetup reachable from gameplay states
    - `test_state_transition_disconnect_from_any` — test Unconnected reachable from any state

- `rust/src/netplay/error.rs` — Add `#[cfg(test)] mod tests` block
  - marker: `@plan PLAN-20260314-NETPLAY.P04`
  - Tests to write:
    - `test_error_from_io_error` — verify `From<io::Error>` conversion
    - `test_abort_reason_display` — verify all abort reasons have meaningful display strings
    - `test_reset_reason_display` — verify all reset reasons have meaningful display strings

- `rust/src/netplay/options.rs` — Add `#[cfg(test)] mod tests` block
  - marker: `@plan PLAN-20260314-NETPLAY.P04`
  - Tests to write:
    - `test_default_peer_options` — verify defaults match C: localhost, 21837, is_server=true
    - `test_default_netplay_options` — verify default input delay is 2
    - `test_options_clone` — verify options are Clone + Debug

- `rust/src/netplay/constants.rs` — Add `#[cfg(test)] mod tests` block
  - marker: `@plan PLAN-20260314-NETPLAY.P04`
  - Tests to write:
    - `test_protocol_version` — verify major=0, minor=4
    - `test_min_uqm_version` — verify 0.6.9
    - `test_buffer_constants` — verify READ_BUF_SIZE=2048, PACKET_HEADER_SIZE=4

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] Test modules added to `state.rs`, `error.rs`, `options.rs`, `constants.rs`
- [ ] At least 16 test functions defined
- [ ] All tests are `#[cfg(test)]` gated
- [ ] Tests compile

## Semantic Verification Checklist (Mandatory)
- [ ] State predicate tests cover ALL states (not just a few)
- [ ] Transition tests cover both valid and invalid cases
- [ ] Tests would FAIL if predicates returned wrong values
- [ ] Tests would FAIL if transitions allowed invalid paths
- [ ] Error conversion test uses real `std::io::Error` values

## Success Criteria
- [ ] All tests compile
- [ ] Tests that can pass with stub implementations do pass
- [ ] Tests that require real implementation are expected to fail (TDD)
- [ ] `cargo test --all-features` reports test results

## Failure Recovery
- rollback: `git checkout -- rust/src/netplay/`
- blocking issues: test infrastructure, feature gating in test modules

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P04.md`
