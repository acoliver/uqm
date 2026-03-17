# Phase 10: Setup Sync & Notifications — Stub/TDD/Impl

## Phase ID
`PLAN-20260314-NETPLAY.P10`

## Prerequisites
- Required: Phase 09a (Packet Handlers Verification) completed and passed
- Expected: handlers, protocol sub-systems, connection, packet infrastructure all working

## Requirements Implemented (Expanded)

### REQ-NP-SETUP-002 / REQ-NP-SETUP-003 / REQ-NP-SETUP-006
**Requirement text**: When the local side changes synchronized fleet or team-name state during setup, the subsystem shall transmit corresponding setup updates. When local and remote setup updates conflict, the subsystem shall resolve deterministically.

Behavior contract:
- GIVEN: Connection in `InSetup`, local not confirmed
- WHEN: Local fleet slot changes
- THEN: Fleet packet queued for that connection
- GIVEN: Crossing edits from both sides
- WHEN: Both sides receive the conflicting edit
- THEN: the deterministic conflict-resolution policy defined in analysis/P12 produces identical state on both peers

### REQ-NP-SETUP-001 / REQ-NP-FLEET-001 / REQ-NP-FLEET-002
**Requirement text**: When a connection enters synchronized setup flow or re-enters setup after battle/reset, the subsystem shall support full bootstrap synchronization of the current team state.

Behavior contract:
- GIVEN: Connection just entered `InSetup`
- WHEN: Bootstrap sync is triggered
- THEN: Full fleet + team name packets sent for this player's team

### REQ-NP-DISC-001
**Requirement text**: When a remote peer disconnects during setup, the subsystem shall surface the disconnect so SuperMelee can stop waiting.

## Implementation Tasks

### Files to create

- `rust/src/netplay/notify/per_connection.rs` — Per-connection notification functions
  - marker: `@plan PLAN-20260314-NETPLAY.P10`
  - marker: `@requirement REQ-NP-SETUP-002 REQ-NP-SETUP-003 REQ-NP-SETUP-006 REQ-NP-DELAY-001 REQ-NP-SEED-001 REQ-NP-INPUT-002 REQ-NP-SHIP-002 REQ-NP-CHECK-001 REQ-NP-END-002`
  - Contents:
    - `fn notify_team_name_change(conn: &mut NetConnection, side: FleetSide, name: &str) -> Result<(), NetplayError>`
      - Validates `InSetup`, `!handshake.local_ok`
      - Queues `TeamName` packet
    - `fn notify_full_fleet(
          conn: &mut NetConnection,
          side: FleetSide,
          fleet: &[(FleetSlot, ShipId)],
      ) -> Result<(), NetplayError>`
      - Validates `InSetup`, `!handshake.local_ok`
      - Queues full `Fleet` packet
    - `fn notify_fleet_change(
          conn: &mut NetConnection,
          side: FleetSide,
          slot: FleetSlot,
          ship: ShipId,
      ) -> Result<(), NetplayError>`
      - Validates `InSetup`, `!handshake.local_ok`
      - Queues single-entry `Fleet` packet
    - `fn notify_input_delay(conn: &mut NetConnection, delay: u32) -> Result<(), NetplayError>`
      - Validates `PreBattle`
      - Queues `InputDelay` packet
    - `fn notify_seed_random(conn: &mut NetConnection, seed: u32) -> Result<(), NetplayError>`
      - Validates `PreBattle`, `conn.discriminant`
      - Queues `SeedRandom` packet
    - `fn notify_battle_input(conn: &mut NetConnection, input: u8) -> Result<(), NetplayError>`
      - Validates battle-active state
      - Queues `BattleInput` packet
    - `fn notify_ship_selected(conn: &mut NetConnection, ship: ShipId) -> Result<(), NetplayError>`
      - Validates `SelectShip`
      - Queues `SelectShip` packet
    - `fn notify_checksum(conn: &mut NetConnection, frame: u32, checksum: u32) -> Result<(), NetplayError>`
      - Validates battle-active state
      - Queues `Checksum` packet
    - `fn notify_frame_count(conn: &mut NetConnection, frame_count: u32) -> Result<(), NetplayError>`
      - Validates ending-battle states
      - Queues `FrameCount` packet

- `rust/src/netplay/notify/broadcast.rs` — Fan-out to all connections
  - marker: `@plan PLAN-20260314-NETPLAY.P10`
  - marker: `@requirement REQ-NP-SETUP-001 REQ-NP-FLEET-001 REQ-NP-FLEET-002`
  - Contents:
    - `fn broadcast_team_name_change(registry: &mut ConnectionRegistry, side: FleetSide, name: &str)`
    - `fn broadcast_full_fleet(registry: &mut ConnectionRegistry, side: FleetSide, fleet: &[(FleetSlot, ShipId)])`
    - `fn broadcast_fleet_change(registry: &mut ConnectionRegistry, side: FleetSide, slot: FleetSlot, ship: ShipId)`
    - `fn broadcast_input_delay(registry: &mut ConnectionRegistry, delay: u32)`
    - `fn broadcast_battle_input(registry: &mut ConnectionRegistry, input: u8)`
    - `fn broadcast_checksum(registry: &mut ConnectionRegistry, frame: u32, checksum: u32)`
    - each iterates connected peers in the appropriate state and calls the per-connection variant
    - errors on individual connections are logged but do not abort the broadcast

### Files to modify

- `rust/src/netplay/notify/mod.rs` — Add sub-module declarations

### Notes on conflict resolution ownership

P10 only owns transmission of local setup mutations and bootstrap sync. Deterministic convergence for crossing local/remote setup edits is finalized at the integration boundary in P12, where transport-delivered remote updates are reconciled against owner state using the explicit policy from analysis.

### Tests

**notify/per_connection.rs tests:**
- `test_notify_team_name_in_setup`
- `test_notify_team_name_wrong_state`
- `test_notify_team_name_while_confirmed`
- `test_notify_full_fleet`
- `test_notify_fleet_change_single`
- `test_notify_input_delay_in_prebattle`
- `test_notify_seed_random_discriminant`
- `test_notify_seed_random_non_discriminant`
- `test_notify_battle_input_in_battle`
- `test_notify_ship_selected_in_select`
- `test_notify_checksum_in_battle`
- `test_notify_frame_count_in_ending`

**notify/broadcast.rs tests:**
- `test_broadcast_team_name_to_all`
- `test_broadcast_skips_disconnected`
- `test_broadcast_skips_wrong_state`
- `test_broadcast_battle_input_to_all`
- `test_broadcast_continues_on_individual_error`

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --all-features -- netplay::notify
```

## Structural Verification Checklist
- [ ] Both notify files exist (`per_connection.rs`, `broadcast.rs`)
- [ ] `notify/mod.rs` declares both sub-modules
- [ ] Per-connection functions validate state and confirmation flags
- [ ] Broadcast functions iterate registry and call per-connection
- [ ] Notification function names and signatures match the canonical public API lock
- [ ] At least 17 tests defined

## Semantic Verification Checklist (Mandatory)
- [ ] Notification functions respect the `!handshake.local_ok` guard
- [ ] Fleet notification with 1 ship creates a 1-entry Fleet packet, not full fleet
- [ ] SeedRandom only sent by discriminant side
- [ ] Broadcast skips disconnected and wrong-state connections
- [ ] Broadcast logs but doesn't abort on individual connection errors
- [ ] All packet types match the codec from P06
- [ ] Notification layer does not invent a conflicting setup-resolution policy separate from P12 integration logic

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|todo!()\|unimplemented!()\|placeholder" rust/src/netplay/notify/ | grep -v test
```

## Success Criteria
- [ ] All notification tests pass
- [ ] Per-connection API mirrors the needed `notify.c` function set using canonical Rust types
- [ ] Broadcast API mirrors the needed `notifyall.c` function set
- [ ] State guards match C assertions

## Failure Recovery
- rollback: `git checkout -- rust/src/netplay/notify/`
- blocking issues: registry borrow issues in broadcast (may need index-based iteration)

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P10.md`
