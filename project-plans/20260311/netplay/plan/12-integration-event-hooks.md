# Phase 12: Integration & Event Hooks — Stub/TDD/Impl

## Phase ID
`PLAN-20260314-NETPLAY.P12`

## Prerequisites
- Required: Phase 10a (Setup Sync & Notifications Verification) completed and passed
- Expected: all internal netplay modules implemented and tested

## Requirements Implemented (Expanded)

### REQ-NP-INTEG-001 / REQ-NP-INTEG-002
**Requirement text**: When SuperMelee queries whether a network-controlled side is connected and setup-ready, the subsystem shall provide accurate state. When SuperMelee needs player-facing feedback for connection/abort/reset/error events, the subsystem shall emit enough structured information.

Behavior contract:
- GIVEN: Connection established and in `InSetup`
- WHEN: SuperMelee asks `is_connected_and_in_setup(player)`
- THEN: Returns true

### REQ-NP-INTEG-003 / REQ-NP-INTEG-004
**Requirement text**: When battle starts, the subsystem shall provide initialized network state sufficient for battle-time input delivery and optional checksum verification. When battle ends or aborts, the subsystem shall leave state from which setup/inter-battle can continue.

### REQ-NP-SETUP-006 / specification §9.3
**Requirement text**: Conflicting setup updates must resolve deterministically and identically across peers.

### REQ-NP-SHIP-005 / specification §12.4
**Requirement text**: If a transport-valid remote ship selection fails semantic validation at the SuperMelee boundary, battle handoff must be blocked and reset initiated.

### REQ-NP-SEED-002 / REQ-NP-END-001..004 / REQ-NP-CONN-003 / REQ-NP-ERROR-003
**Requirement text**: Battle entry must not proceed until required seed exchange is complete. Battle-end synchronization must complete its two-stage frame-count/ready protocol before returning to inter-battle flow. Pre-establishment connection failures must surface cleanly through the event path. Stale post-reset packets must be handled consistently by the shared progress/dispatch path.

## Implementation Tasks

### Files to create

- `rust/src/netplay/integration/event.rs` — `NetplayEvent` enum and trait
  - marker: `@plan PLAN-20260314-NETPLAY.P12`
  - marker: `@requirement REQ-NP-INTEG-002 REQ-NP-CONN-003`
  - Contents:
    - Canonical `NetplayEvent` enum matching the overview API lock exactly:
      - `Connected { player: PlayerId }`
      - `ConnectionFailed { player: PlayerId, error: NetplayError }`
      - `ConnectionClosed { player: PlayerId }`
      - `ConfirmationInvalidated { player: PlayerId }`
      - `ResetReceived { player: PlayerId, reason: ResetReason, by_remote: bool }`
      - `AbortReceived { player: PlayerId, reason: AbortReason }`
      - `RemoteFleetUpdate { player: PlayerId, side: FleetSide, ships: Vec<(FleetSlot, ShipId)> }`
      - `RemoteTeamNameUpdate { player: PlayerId, side: FleetSide, name: String }`
      - `RemoteShipSelected { player: PlayerId, ship: ShipId }`
      - `RandomSeedReceived { player: PlayerId, seed: u32 }`
      - `InputDelayReceived { player: PlayerId, delay: u32 }`
      - `SyncLoss { frame: u32 }`
    - `trait NetplayEventSink { fn on_event(&mut self, event: NetplayEvent); }`

- `rust/src/netplay/integration/melee_hooks.rs` — SuperMelee integration facade
  - marker: `@plan PLAN-20260314-NETPLAY.P12`
  - marker: `@requirement REQ-NP-INTEG-001 REQ-NP-SETUP-006 REQ-NP-SHIP-005 REQ-NP-CONN-003`
  - Contents:
    - `NetplayFacade` struct wrapping `ConnectionRegistry` + optional `BattleInputState` + optional `ChecksumVerifier` + event sink
    - Public API implementing the canonical `NetplayApi` trait from the overview exactly:
      - `fn open_connection(&mut self, player: PlayerId, options: &PeerOptions) -> Result<(), NetplayError>`
      - `fn close_connection(&mut self, player: PlayerId)`
      - `fn close_all_connections(&mut self)`
      - `fn is_connected(&self, player: PlayerId) -> bool`
      - `fn is_connected_and_in_setup(&self, player: PlayerId) -> bool`
      - `fn connection_state(&self, player: PlayerId) -> Option<NetState>`
      - `fn num_connected(&self) -> usize`
      - `fn confirm_setup(&mut self, player: PlayerId) -> Result<(), NetplayError>`
      - `fn cancel_confirmation(&mut self, player: PlayerId) -> Result<(), NetplayError>`
      - `fn notify_fleet_change(&mut self, player: PlayerId, side: FleetSide, slot: FleetSlot, ship: ShipId) -> Result<(), NetplayError>`
      - `fn notify_team_name_change(&mut self, player: PlayerId, side: FleetSide, name: &str) -> Result<(), NetplayError>`
      - `fn notify_full_fleet(&mut self, player: PlayerId, side: FleetSide, fleet: &[(FleetSlot, ShipId)]) -> Result<(), NetplayError>`
      - `fn bootstrap_sync(&mut self, player: PlayerId, side: FleetSide, fleet: &[(FleetSlot, ShipId)], name: &str) -> Result<(), NetplayError>`
      - `fn poll(&mut self) -> Result<Vec<NetplayEvent>, NetplayError>`
      - `fn flush(&mut self) -> Result<(), NetplayError>`
    - Explicit deterministic setup conflict-resolution helpers:
      - `fn resolve_setup_conflict(...) -> SetupConflictResolution`
      - `fn apply_remote_fleet_update(...) -> Result<(), NetplayError>`
      - `fn apply_remote_team_name_update(...) -> Result<(), NetplayError>`
      - logic must use the stable discriminant/tie-breaker policy identified in analysis
      - confirmation invalidation and event emission are owned here, not in the transport-only notify layer
    - Connection-failure/event path helpers:
      - `fn drain_connection_failures(&mut self) -> Vec<NetplayEvent>`
      - `open_connection()` returns immediate configuration/setup errors directly when no asynchronous attempt is started
      - deferred listen/connect failures discovered after registration are converted into `NetplayEvent::ConnectionFailed` during `poll()` / shared progress-loop servicing after cleanup returns the slot to `Unconnected`
    - Ship-selection semantic boundary helpers:
      - `fn commit_remote_ship_selection(&mut self, player: PlayerId, ship: ShipId) -> Result<(), NetplayError>`
      - invokes SuperMelee-side semantic validation hook/callback
      - if rejected: block battle handoff immediately and initiate `reset(ResetReason::SyncLoss)` or protocol-defined invalid-selection reset reason

- `rust/src/netplay/integration/battle_hooks.rs` — Battle integration facade
  - marker: `@plan PLAN-20260314-NETPLAY.P12`
  - marker: `@requirement REQ-NP-INTEG-003 REQ-NP-INTEG-004 REQ-NP-READY-004 REQ-NP-SEED-002 REQ-NP-END-001 REQ-NP-END-002 REQ-NP-END-003 REQ-NP-END-004 REQ-NP-ERROR-003`
  - Contents:
    - methods on `NetplayFacade` implementing the canonical overview API exactly:
      - `fn init_battle(&mut self, input_delay: u32, checksum_enabled: bool) -> Result<(), NetplayError>`
      - `fn uninit_battle(&mut self)`
      - `fn send_battle_input(&mut self, input: u8) -> Result<(), NetplayError>`
      - `fn receive_battle_input(&mut self, player: PlayerId) -> Result<u8, NetplayError>`
      - `fn send_ship_selected(&mut self, player: PlayerId, ship: ShipId) -> Result<(), NetplayError>`
      - `fn send_checksum(&mut self, frame: u32, checksum: u32) -> Result<(), NetplayError>`
      - `fn verify_checksum(&mut self, frame: u32) -> Result<bool, NetplayError>`
      - `fn advertise_input_delay(&mut self, delay: u32) -> Result<(), NetplayError>`
      - `fn send_random_seed(&mut self, player: PlayerId, seed: u32) -> Result<(), NetplayError>`
      - `fn setup_input_delay(&self) -> u32`
      - `fn signal_battle_end(&mut self, frame_count: u32) -> Result<(), NetplayError>`
      - `fn negotiate_ready(&mut self, target_state: NetState) -> Result<(), NetplayError>`
      - `fn wait_reset(&mut self, target_state: NetState) -> Result<(), NetplayError>`
      - `fn reset(&mut self, reason: ResetReason) -> Result<(), NetplayError>`
      - `fn abort(&mut self, reason: AbortReason) -> Result<(), NetplayError>`
    - shared progress-loop implementation used by `poll`, `receive_battle_input`, `negotiate_ready`, and `wait_reset`
      - one named helper/engine is mandatory and all four entry points must call it rather than duplicating bespoke loops
      - flush queued packets
      - poll/receive
      - dispatch
      - deliver events, including drained `ConnectionFailed` events from pre-establishment transport failures
      - run deferred callbacks/completions
      - re-check wait predicate
      - timeout/abort/disconnect handling
      - respect the P09 shared stale-packet policy during post-reset traffic
    - explicit seed-agreement gate for battle entry:
      - `fn require_seed_agreement_before_battle(&self) -> Result<(), NetplayError>` or equivalent helper used by both `negotiate_ready(NetState::InBattle)` and `init_battle()`
      - battle-start progression must reject entry to `InBattle` until every required peer has completed random-seed agreement
      - the gate must be stronger than merely recording `agreement.random_seed = true`; battle entry must check it explicitly
    - explicit battle-end orchestration helpers:
      - `fn begin_battle_end(&mut self, frame_count: u32) -> Result<(), NetplayError>` — enter `EndingBattle`, send local frame count, mark local ready-to-end
      - `fn update_end_frame_target(&mut self, remote_frame_count: u32)` — choose max(local, remote)
      - `fn continue_until_end_frame(&mut self) -> Result<(), NetplayError>` — keep simulating/servicing progress until agreed terminal frame is reached
      - `fn finalize_battle_end_ready(&mut self) -> Result<(), NetplayError>` — run `EndingBattle2` final ready rendezvous and return to `InterBattle`
      - `signal_battle_end()` / `negotiate_ready()` must implement the full `EndingBattle -> EndingBattle2 -> InterBattle` algorithm, not defer the protocol semantics to callers

### Files to modify

- `rust/src/netplay/integration/mod.rs` — Add sub-module declarations
- `rust/src/netplay/mod.rs` — Add public re-exports
  - Re-export `NetplayFacade`, `NetplayEvent`, `NetplayEventSink`, `NetState`, `NetplayError`, `PeerOptions`, `NetplayOptions`, `AbortReason`, `ResetReason`

### Tests

**integration/event.rs tests:**
- `test_event_variants_constructible`
- `test_event_sink_receives_events`
- `test_event_debug_display`

**integration/melee_hooks.rs tests:**
- `test_facade_open_close`
- `test_facade_is_connected_and_in_setup`
- `test_facade_confirm_setup`
- `test_facade_cancel_confirmation`
- `test_facade_notify_fleet_change`
- `test_facade_notify_team_name_change`
- `test_facade_bootstrap_sync`
- `test_facade_poll_receives_events`
- `test_facade_close_all_connections`
- `test_facade_num_connected`
- `test_resolve_setup_conflict_crossing_fleet_edits`
- `test_resolve_setup_conflict_crossing_team_name_edits`
- `test_connection_failed_event_emitted_from_deferred_transport_failure`
- `test_connection_failed_cleanup_returns_slot_to_unconnected_before_event`
- `test_remote_ship_selection_semantic_reject_triggers_reset`
- `test_remote_ship_selection_semantic_reject_blocks_battle_handoff`

**integration/battle_hooks.rs tests:**
- `test_facade_init_battle`
- `test_facade_init_battle_requires_seed_agreement`
- `test_negotiate_ready_in_battle_requires_seed_agreement`
- `test_facade_uninit_battle`
- `test_facade_send_battle_input`
- `test_facade_send_checksum`
- `test_facade_verify_checksum_match`
- `test_facade_verify_checksum_desync`
- `test_facade_setup_input_delay`
- `test_signal_battle_end_enters_endingbattle_and_sends_frame_count`
- `test_signal_battle_end_uses_max_of_local_and_remote_frame_counts`
- `test_signal_battle_end_continues_until_agreed_terminal_frame`
- `test_signal_battle_end_final_ready_returns_to_interbattle`
- `test_facade_abort`
- `test_facade_reset`
- `test_poll_uses_shared_progress_engine`
- `test_receive_battle_input_uses_shared_progress_engine`
- `test_negotiate_ready_uses_shared_progress_engine`
- `test_wait_reset_uses_shared_progress_engine`

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --all-features -- netplay::integration
```

## Structural Verification Checklist
- [ ] All 3 integration files exist
- [ ] `integration/mod.rs` declares all sub-modules
- [ ] `NetplayFacade` wraps all internal state (registry, buffers, verifier)
- [ ] Public API exactly matches the canonical overview API lock
- [ ] `mod.rs` re-exports public API types
- [ ] One named shared progress helper/engine exists and is called by `poll`, `receive_battle_input`, `negotiate_ready`, and `wait_reset`
- [ ] Explicit battle-entry seed gate exists in battle hooks
- [ ] Explicit battle-end orchestration helpers exist for frame-target selection and final ready
- [ ] At least 38 tests defined

## Semantic Verification Checklist (Mandatory)
- [ ] `NetplayFacade` is the single public entry point for SuperMelee/battle
- [ ] `poll()` drives the receive→dispatch→event cycle
- [ ] `flush()` sends all queued packets
- [ ] `bootstrap_sync()` sends fleet then name in the compatibility-required order
- [ ] `init_battle()` creates correctly-sized input + checksum buffers
- [ ] `receive_battle_input()` blocks via the shared progress loop
- [ ] `negotiate_ready()` blocks via the shared progress loop
- [ ] `wait_reset()` blocks via the shared progress loop
- [ ] `abort()` sends abort packets before closing
- [ ] Event sink receives all events from handlers
- [ ] Deterministic setup conflict resolution is explicitly implemented and tested
- [ ] Semantically invalid remote ship selection triggers reset and blocks battle handoff per specification §12.4
- [ ] Battle entry is rejected until required random-seed exchange has completed for all peers
- [ ] Pre-establishment connect/listen failures surface through `ConnectionFailed` with cleanup completed first
- [ ] Battle-end sync explicitly chooses the max target frame, continues simulation until that frame, then performs the final ready rendezvous back to `InterBattle`
- [ ] Post-reset stale-packet handling remains consistent because the shared progress path honors the P09 stale-packet policy

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|todo!()\|unimplemented!()\|placeholder" rust/src/netplay/integration/ | grep -v test
```

## Success Criteria
- [ ] All integration tests pass
- [ ] `NetplayFacade` provides complete API for SuperMelee + battle
- [ ] Event system delivers all protocol events to the integration boundary
- [ ] No internal netplay types leak through the public API (encapsulation)
- [ ] Integration owns the explicit seed gate, connection-failure event path, and battle-end orchestration rather than leaving them implicit

## Failure Recovery
- rollback: `git checkout -- rust/src/netplay/integration/`
- blocking issues: borrow checker with facade holding mutable refs to multiple sub-components

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P12.md`
