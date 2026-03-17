# Phase 09: Packet Handlers & Dispatch — Stub/TDD/Impl

## Phase ID
`PLAN-20260314-NETPLAY.P09`

## Prerequisites
- Required: Phase 11a (Battle Input & Checksum Verification) completed and passed
- Expected: protocol sub-systems, packet codec, connection infrastructure, and battle/checksum primitives

## Requirements Implemented (Expanded)

### REQ-NP-PROTO-001..003
**Requirement text**: When a peer initialization packet is received, the subsystem shall validate the peer's protocol version and product version.

### REQ-NP-ERROR-002 / REQ-NP-ERROR-003
**Requirement text**: When a structurally valid packet is received in an invalid connection state, the subsystem shall treat it as a protocol error. After reset begins, stale setup/gameplay packets must be ignored or rejected consistently according to one documented packet-family policy.

### REQ-NP-SETUP-004 / REQ-NP-SETUP-005 / REQ-NP-CONFIRM-004
**Requirement text**: When valid remote fleet/team-name updates are received during setup, the subsystem shall deliver them to the integration boundary, and remote setup edits can invalidate confirmation.

### REQ-NP-INPUT-003 / REQ-NP-SHIP-003 / REQ-NP-CHECK-003 / REQ-NP-ABORT-002
**Requirement text**: Battle input is buffered, ship selections are delivered, implausible checksum packets are safely ignored, and abort is surfaced cleanly.

## Implementation Tasks

### Files to create

- `rust/src/netplay/handlers/init.rs` — Init packet handler
  - marker: `@plan PLAN-20260314-NETPLAY.P09`
  - marker: `@requirement REQ-NP-PROTO-001 REQ-NP-PROTO-002 REQ-NP-PROTO-003`
  - Contents:
    - `fn handle_init(conn: &mut NetConnection, payload: &InitPayload) -> Result<Option<NetplayEvent>, NetplayError>`
    - Validates: state == Init, remote_ready not set, protocol version exact match, UQM version >= minimum
    - On success: calls `proto::ready::remote_ready(conn)`
    - On failure: sends Abort(VersionMismatch), returns error

- `rust/src/netplay/handlers/setup.rs` — Fleet, TeamName, Handshake handlers
  - marker: `@plan PLAN-20260314-NETPLAY.P09`
  - marker: `@requirement REQ-NP-SETUP-004 REQ-NP-SETUP-005 REQ-NP-CONFIRM-004 REQ-NP-ERROR-003`
  - Contents:
    - `fn handle_fleet(conn: &mut NetConnection, payload: &FleetPayload) -> Result<Option<NetplayEvent>, NetplayError>`
      - Validates state == InSetup
      - Validates slot bounds and ship IDs against the canonical audited type model
      - If reset is active, treat as stale setup traffic and reject consistently via the shared stale-packet policy helper
      - If local confirmation active, cancels it, emits `ConfirmationInvalidated`
      - Emits `RemoteFleetUpdate { player, side, ships }`
    - `fn handle_team_name(conn: &mut NetConnection, payload: &TeamNamePayload) -> Result<Option<NetplayEvent>, NetplayError>`
      - Same validation and cancel-on-edit behavior
      - If reset is active, treat as stale setup traffic and reject consistently via the shared stale-packet policy helper
      - Emits `RemoteTeamNameUpdate { player, side, name }`
    - handshake handlers delegate to `proto::confirm`
    - handshake handlers apply the shared stale-packet policy so post-reset handshake traffic is not accepted silently

- `rust/src/netplay/handlers/battle.rs` — BattleInput, SelectShip, FrameCount handlers
  - marker: `@plan PLAN-20260314-NETPLAY.P09`
  - marker: `@requirement REQ-NP-INPUT-003 REQ-NP-SHIP-003 REQ-NP-END-002 REQ-NP-ERROR-003`
  - Contents:
    - `fn handle_battle_input(
          conn: &mut NetConnection,
          payload: &BattleInputPayload,
          input_buffers: &mut [BattleInputBuffer],
      ) -> Result<Option<NetplayEvent>, NetplayError>`
      - Applies the shared stale-packet policy before normal state validation
      - Validates battle_active state
      - Pushes input into player's buffer
      - Returns error if buffer full
    - `fn handle_select_ship(conn: &mut NetConnection, payload: &SelectShipPayload) -> Result<Option<NetplayEvent>, NetplayError>`
      - Applies the shared stale-packet policy before normal state validation
      - Validates state == SelectShip
      - Emits `RemoteShipSelected { player, ship }`
      - Transport-level validation only; semantic validity is deferred to P12 integration boundary
    - `fn handle_frame_count(conn: &mut NetConnection, payload: &FrameCountPayload) -> Result<Option<NetplayEvent>, NetplayError>`
      - Applies the shared stale-packet policy before normal state validation
      - Validates state in {EndingBattle, EndingBattle2}
      - Updates end_frame_count to max of current and received
      - Calls `proto::ready::remote_ready(conn)`

- `rust/src/netplay/handlers/sync.rs` — Checksum, SeedRandom, InputDelay handlers
  - marker: `@plan PLAN-20260314-NETPLAY.P09`
  - marker: `@requirement REQ-NP-CHECK-003 REQ-NP-SEED-001 REQ-NP-DELAY-001 REQ-NP-DELAY-004 REQ-NP-ERROR-003`
  - Contents:
    - `fn handle_checksum(
          conn: &mut NetConnection,
          payload: &ChecksumPayload,
          remote_buf: &mut ChecksumBuffer,
      ) -> Result<Option<NetplayEvent>, NetplayError>`
      - Validates battle_active state
      - Ignores if reset active per the shared stale-packet policy
      - Validates frame on checksum interval
      - Soft-rejects out-of-range frames (warn + discard)
      - Stores in remote checksum buffer
    - `fn handle_seed_random(conn: &mut NetConnection, payload: &SeedRandomPayload) -> Result<Option<NetplayEvent>, NetplayError>`
      - Applies the shared stale-packet policy before normal state validation
      - Validates state == PreBattle
      - Validates !conn.discriminant (only non-discriminant side receives seed)
      - Sets agreement.random_seed = true
      - Emits `RandomSeedReceived`
    - `fn handle_input_delay(conn: &mut NetConnection, payload: &InputDelayPayload) -> Result<Option<NetplayEvent>, NetplayError>`
      - Applies the shared stale-packet policy before normal state validation
      - Validates state == PreBattle
      - Validates delay <= MAX_INPUT_DELAY
      - Stores delay in conn.state_flags.input_delay
      - Emits `InputDelayReceived`

- `rust/src/netplay/handlers/control.rs` — Ready, Abort, Reset, Ping/Ack handlers
  - marker: `@plan PLAN-20260314-NETPLAY.P09`
  - marker: `@requirement REQ-NP-READY-001 REQ-NP-ABORT-002 REQ-NP-RESET-002 REQ-NP-ERROR-003`
  - Contents:
    - `fn handle_ready(conn: &mut NetConnection) -> Result<Option<NetplayEvent>, NetplayError>`
      - Applies the shared stale-packet policy before normal state validation
      - Validates ready_meaningful state
      - Calls `proto::ready::remote_ready(conn)`
    - `fn handle_abort(conn: &mut NetConnection, payload: &AbortPayload) -> Result<Option<NetplayEvent>, NetplayError>`
      - Remains legal after reset starts so shutdown can still converge
      - Emits `AbortReceived`
      - Returns error to trigger connection close
    - `fn handle_reset(conn: &mut NetConnection, payload: &ResetPayload) -> Result<Option<NetplayEvent>, NetplayError>`
      - Remains legal after reset starts so duplicate/confirming reset traffic can still converge
      - Converts reason from u16
      - Calls `proto::reset::remote_reset(conn, reason)`
    - `fn handle_ping(conn: &mut NetConnection, payload: &PingPayload) -> Result<Option<NetplayEvent>, NetplayError>`
      - Queues Ack packet with same id
    - `fn handle_ack(conn: &mut NetConnection, payload: &AckPayload) -> Result<Option<NetplayEvent>, NetplayError>`
      - no-op unless later analysis finds observable behavior

- `rust/src/netplay/handlers/mod.rs` — Dispatch table
  - marker: `@plan PLAN-20260314-NETPLAY.P09`
  - Contents:
    - Sub-module declarations
    - `enum StalePacketDisposition { Ignore, ProtocolError, Allow }` or equivalent shared helper
    - `fn stale_packet_policy(conn: &NetConnection, packet: &Packet) -> StalePacketDisposition`
      - documents and centralizes the post-reset rule: `Reset` and `Abort` remain legal, stale checksum traffic is ignored, stale setup/gameplay packets (`Fleet`, `TeamName`, `Handshake*`, `Ready`, `BattleInput`, `FrameCount`, `SelectShip`, `SeedRandom`, `InputDelay`) are rejected or ignored according to one explicit policy, never ad hoc per handler
    - `fn dispatch_packet(conn: &mut NetConnection, packet: Packet, ...) -> Result<Option<NetplayEvent>, NetplayError>`
      - Match on `Packet` variant, delegate to appropriate handler
      - Applies the shared stale-packet policy consistently before or inside handlers as documented
      - On error: log, return error for caller to close connection

### Files to modify

- `rust/src/netplay/handlers/mod.rs` — declare sub-modules

### Tests

**handlers/init.rs tests:**
- `test_handle_init_valid`
- `test_handle_init_protocol_mismatch`
- `test_handle_init_uqm_version_too_old`
- `test_handle_init_wrong_state`
- `test_handle_init_duplicate`

**handlers/setup.rs tests:**
- `test_handle_fleet_in_setup`
- `test_handle_fleet_cancels_confirmation`
- `test_handle_fleet_wrong_state`
- `test_handle_fleet_rejected_after_reset_started`
- `test_handle_team_name_in_setup`
- `test_handle_team_name_cancels_confirmation`
- `test_handle_team_name_rejected_after_reset_started`
- `test_handle_handshake_delegates`
- `test_handle_handshake_rejected_after_reset_started`

**handlers/battle.rs tests:**
- `test_handle_battle_input_valid`
- `test_handle_battle_input_buffer_full`
- `test_handle_battle_input_wrong_state`
- `test_handle_battle_input_rejected_after_reset_started`
- `test_handle_select_ship_valid`
- `test_handle_select_ship_wrong_state`
- `test_handle_select_ship_rejected_after_reset_started`
- `test_handle_frame_count_updates_max`
- `test_handle_frame_count_triggers_remote_ready`
- `test_handle_frame_count_rejected_after_reset_started`

**handlers/sync.rs tests:**
- `test_handle_checksum_valid`
- `test_handle_checksum_ignored_during_reset`
- `test_handle_checksum_out_of_range`
- `test_handle_seed_random_valid`
- `test_handle_seed_random_discriminant_rejects`
- `test_handle_seed_random_wrong_state`
- `test_handle_seed_random_rejected_after_reset_started`
- `test_handle_input_delay_valid`
- `test_handle_input_delay_too_high`
- `test_handle_input_delay_rejected_after_reset_started`

**handlers/control.rs tests:**
- `test_handle_ready_triggers_remote_ready`
- `test_handle_ready_rejected_after_reset_started`
- `test_handle_abort_emits_event`
- `test_handle_reset_triggers_remote_reset`
- `test_handle_ping_sends_ack`

**handlers/mod.rs tests:**
- `test_dispatch_all_packet_types`
- `test_dispatch_returns_events`
- `test_stale_packet_policy_allows_reset_after_reset_started`
- `test_stale_packet_policy_allows_abort_after_reset_started`
- `test_stale_packet_policy_rejects_setup_packets_after_reset_started`
- `test_stale_packet_policy_rejects_battle_packets_after_reset_started`
- `test_stale_packet_policy_ignores_checksum_after_reset_started`

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --all-features -- netplay::handlers
```

## Structural Verification Checklist
- [ ] All 6 handler files exist (mod, init, setup, battle, sync, control)
- [ ] `dispatch_packet()` covers all 18 packet types
- [ ] Every handler validates connection state before processing
- [ ] Handler signatures use canonical event/type shapes from the overview API lock
- [ ] One shared stale-packet policy helper exists and is referenced by all packet families affected by REQ-NP-ERROR-003
- [ ] At least 43 tests defined across handler modules

## Semantic Verification Checklist (Mandatory)
- [ ] Init handler rejects version mismatches with Abort packet
- [ ] Fleet/TeamName handlers cancel active confirmation
- [ ] BattleInput handler pushes to correct player buffer
- [ ] Checksum handler soft-rejects out-of-range frames (no connection close)
- [ ] SeedRandom only accepted on non-discriminant side
- [ ] InputDelay rejects values > BATTLE_FRAME_RATE
- [ ] Abort handler returns error (triggers connection close upstream)
- [ ] Reset handler delegates to proto::reset
- [ ] SelectShip handler performs only transport-level validation; semantic invalid-selection handling is explicitly deferred to P12
- [ ] Post-reset stale packet policy is explicit and consistent across setup/gameplay packet families
- [ ] After reset starts, stale `BattleInput`, `FrameCount`, `SelectShip`, setup-edit, and setup-confirmation packets follow the documented shared policy
- [ ] All handlers return `Result<Option<NetplayEvent>>` consistently

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|todo!()\|unimplemented!()\|placeholder" rust/src/netplay/handlers/ | grep -v test
```

## Success Criteria
- [ ] All handler tests pass
- [ ] dispatch_packet covers every packet type
- [ ] State validation enforced in every handler
- [ ] Events emitted for all integration-relevant packets
- [ ] REQ-NP-ERROR-003 is owned by one explicit policy rather than ad hoc handler behavior

## Failure Recovery
- rollback: `git checkout -- rust/src/netplay/handlers/`
- blocking issues: none expected from battle/checksum primitive ordering because P11 now precedes P09

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P09.md`
