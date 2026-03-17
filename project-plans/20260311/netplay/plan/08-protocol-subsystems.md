# Phase 08: Protocol Sub-systems (Ready/Confirm/Reset) — Stub/TDD/Impl

## Phase ID
`PLAN-20260314-NETPLAY.P08`

## Prerequisites
- Required: Phase 07a (Connection & Transport Verification) completed and passed
- Expected: connection, packet, and core types fully implemented

## Requirements Implemented (Expanded)

### REQ-NP: Ready Synchronization (Ubiquitous + When)
**Requirement text**: The subsystem shall provide a reusable ready-synchronization mechanism for phases that require both peers to rendezvous before progressing. When both peers complete a given ready rendezvous, the subsystem shall trigger the corresponding completion action exactly once.

Behavior contract:
- GIVEN: Two peers in a ready-meaningful state
- WHEN: Both call local_ready
- THEN: The callback fires exactly once on whichever side completes last
- GIVEN: Only one side is ready
- WHEN: The other side calls remote_ready
- THEN: Both sides' ready flags are cleared and callback fires

### REQ-NP: Setup Confirmation (When)
**Requirement text**: When SuperMelee requests setup confirmation during setup state, the subsystem shall enter the setup-confirmation protocol. When both peers complete, the connection advances to pre-battle. When cancelled, cancellation signaling is transmitted.

Behavior contract:
- GIVEN: Connection in InSetup
- WHEN: `confirm()` called → `Handshake0` sent
- WHEN: Remote sends `Handshake0` → `remote_ok` set → if local_ok, send `Handshake1`
- WHEN: Both sides send `Handshake1` → `handshake_complete()` → state = PreBattle

### REQ-NP: Reset Behavior (When)
**Requirement text**: When the local side requests a gameplay reset, the subsystem shall send reset signaling. When remote reset is received before local, the subsystem confirms back. When both sides have reset, completion is signaled.

Behavior contract:
- GIVEN: Connection in any gameplay state
- WHEN: `local_reset()` called
- THEN: `Reset` packet sent, `local_reset` flag set
- WHEN: `remote_reset()` called (remote sent first)
- THEN: Confirming `Reset` sent back, `remote_reset` flag set
- WHEN: Both flags set and callback registered
- THEN: Callback fires exactly once

## Implementation Tasks

### Files to create

- `rust/src/netplay/proto/ready.rs` — Generic ready rendezvous
  - marker: `@plan PLAN-20260314-NETPLAY.P08`
  - marker: `@requirement REQ-NP: ready synchronization`
  - Contents:
    - `fn local_ready(conn: &mut NetConnection, callback: Box<dyn FnOnce(&mut NetConnection)>, send_packet: bool) -> Result<bool, NetplayError>`
    - `fn remote_ready(conn: &mut NetConnection) -> Result<bool, NetplayError>`
    - `fn both_ready(conn: &mut NetConnection, callback: Option<Box<dyn FnOnce(&mut NetConnection)>>)`
  - Pseudocode lines: 165-194

- `rust/src/netplay/proto/confirm.rs` — Setup confirmation protocol
  - marker: `@plan PLAN-20260314-NETPLAY.P08`
  - marker: `@requirement REQ-NP: setup confirmation`
  - Contents:
    - `fn confirm(conn: &mut NetConnection) -> Result<(), NetplayError>`
    - `fn cancel_confirmation(conn: &mut NetConnection) -> Result<(), NetplayError>`
    - `fn handle_handshake0(conn: &mut NetConnection) -> Result<(), NetplayError>`
    - `fn handle_handshake1(conn: &mut NetConnection) -> Result<(), NetplayError>`
    - `fn handle_handshake_cancel(conn: &mut NetConnection) -> Result<(), NetplayError>`
    - `fn handle_handshake_cancel_ack(conn: &mut NetConnection) -> Result<(), NetplayError>`
    - `fn handshake_complete(conn: &mut NetConnection) -> Result<(), NetplayError>`
  - Pseudocode lines: 195-232

- `rust/src/netplay/proto/reset.rs` — Reset coordination protocol
  - marker: `@plan PLAN-20260314-NETPLAY.P08`
  - marker: `@requirement REQ-NP: reset behavior`
  - Contents:
    - `fn local_reset(conn: &mut NetConnection, reason: ResetReason) -> Result<(), NetplayError>`
    - `fn remote_reset(conn: &mut NetConnection, reason: ResetReason) -> Result<(), NetplayError>`
    - `fn set_reset_callback(conn: &mut NetConnection, callback: Box<dyn FnOnce(&mut NetConnection)>)`
    - `fn try_reset_complete(conn: &mut NetConnection)`
  - Pseudocode lines: 233-258

### Files to modify

- `rust/src/netplay/proto/mod.rs` — Add sub-module declarations
  - Declare: `pub mod ready;`, `pub mod confirm;`, `pub mod reset;`

### Tests

**proto/ready.rs tests:**
- `test_local_ready_first` — local ready, then remote_ready triggers callback
- `test_remote_ready_first` — remote ready flag set, then local_ready triggers callback
- `test_local_ready_sends_packet` — verify Ready packet queued when send_packet=true
- `test_local_ready_no_send` — no packet when send_packet=false
- `test_both_ready_clears_flags` — both flags cleared after rendezvous
- `test_callback_fires_exactly_once` — use counter to verify single invocation
- `test_local_ready_wrong_state` — error in Unconnected state
- `test_duplicate_local_ready` — error when already ready

**proto/confirm.rs tests:**
- `test_confirm_sends_handshake0` — first confirm sends Handshake0
- `test_confirm_when_remote_already_ok` — sends Handshake1 instead
- `test_confirm_while_canceling` — defers sending
- `test_cancel_sends_handshake_cancel` — HandshakeCancel queued
- `test_cancel_not_confirmed` — error when not confirmed
- `test_handle_handshake0_sets_remote_ok` — flag set
- `test_handle_handshake1_completes` — calls handshake_complete
- `test_handle_handshake1_while_canceling` — only records remote_ok
- `test_handle_cancel_sends_cancel_ack` — HandshakeCancelAck sent
- `test_handle_cancel_ack_resumes` — re-sends Handshake0/1 if still confirmed
- `test_handshake_complete_transitions_to_prebattle` — state becomes PreBattle
- `test_confirm_wrong_state` — error outside InSetup
- `test_full_confirmation_exchange` — simulate both sides confirming

**proto/reset.rs tests:**
- `test_local_reset_sends_packet` — Reset packet queued
- `test_remote_reset_confirms` — confirming Reset sent back
- `test_both_reset_triggers_callback` — callback fires when both flags set
- `test_reset_callback_fires_once` — verify single invocation
- `test_duplicate_local_reset` — error when already reset
- `test_reset_clears_on_completion` — flags cleared after callback
- `test_set_reset_callback_immediate` — if both already set, fires immediately

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --all-features -- netplay::proto
```

## Structural Verification Checklist
- [ ] All 3 protocol sub-module files exist
- [ ] `proto/mod.rs` declares all sub-modules
- [ ] Ready protocol has local/remote/both_ready functions
- [ ] Confirm protocol has confirm/cancel/handle_* functions
- [ ] Reset protocol has local/remote/set_callback/try_complete functions
- [ ] At least 28 tests defined across all protocol modules

## Semantic Verification Checklist (Mandatory)
- [ ] Ready rendezvous fires callback exactly once regardless of ordering
- [ ] Confirmation protocol handles the full 4-message dance: Handshake0→Handshake1→Cancel→CancelAck
- [ ] Confirmation only meaningful in InSetup state
- [ ] Reset protocol handles bidirectional initiation correctly
- [ ] Reset completion requires BOTH local and remote flags AND a registered callback
- [ ] All protocols queue packets via `conn.queue_packet()`, not direct socket writes
- [ ] State predicates from P05 are used for state validation

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|todo!()\|unimplemented!()\|placeholder" rust/src/netplay/proto/ | grep -v test
```

## Success Criteria
- [ ] All protocol tests pass
- [ ] Ready protocol is reusable across multiple phases
- [ ] Confirmation protocol matches C behavior documented in initialstate.md
- [ ] Reset protocol matches C behavior documented in initialstate.md

## Failure Recovery
- rollback: `git checkout -- rust/src/netplay/proto/`
- blocking issues: callback lifetime issues, borrow checker with mutable connection + callback

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P08.md`
