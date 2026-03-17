# Phase 09a: Packet Handlers & Dispatch — Verification

## Phase ID
`PLAN-20260314-NETPLAY.P09a`

## Prerequisites
- Required: Phase 09 (Packet Handlers) completed
- Expected: all handler files implemented with tests passing

## Verification Tasks

### Dispatch Coverage
- [ ] Every one of the 18 packet types has a match arm in `dispatch_packet()`
- [ ] No match arm uses `_ =>` catch-all that silently drops unknown types
- [ ] Dispatch function signature passes all required context (buffers, etc.)
- [ ] One named stale-packet policy helper owns post-reset packet-family behavior

### State Enforcement
- [ ] `handle_init` rejects non-Init states
- [ ] `handle_fleet` rejects non-InSetup states
- [ ] `handle_team_name` rejects non-InSetup states
- [ ] `handle_handshake*` rejects non-InSetup states (via handshake_meaningful)
- [ ] `handle_battle_input` rejects non-battle-active states
- [ ] `handle_select_ship` rejects non-SelectShip states
- [ ] `handle_frame_count` rejects non-ending-battle states
- [ ] `handle_checksum` rejects non-battle-active states
- [ ] `handle_seed_random` rejects non-PreBattle states
- [ ] `handle_input_delay` rejects non-PreBattle states
- [ ] `handle_ready` rejects non-ready-meaningful states
- [ ] `handle_abort` accepted in any connected state
- [ ] `handle_reset` accepted in any connected state (reset has broad scope)

### Event Emission
- [ ] Fleet handler emits `RemoteFleetUpdate`
- [ ] TeamName handler emits `RemoteTeamNameUpdate`
- [ ] Fleet/TeamName during confirmation emit `ConfirmationInvalidated`
- [ ] SelectShip handler emits `RemoteShipSelected`
- [ ] SeedRandom handler emits event with seed value
- [ ] Abort handler emits `AbortReceived`

### Post-reset Stale Packet Policy
- [ ] `Reset` remains legal after reset has started
- [ ] `Abort` remains legal after reset has started
- [ ] Stale `Checksum` packets are ignored safely after reset has started
- [ ] Stale `BattleInput` packets are rejected or ignored exactly as documented by the shared policy
- [ ] Stale `FrameCount` packets are rejected or ignored exactly as documented by the shared policy
- [ ] Stale `SelectShip` packets are rejected or ignored exactly as documented by the shared policy
- [ ] Stale setup packets (`Fleet`, `TeamName`, `Handshake*`, `Ready`, `SeedRandom`, `InputDelay`) are rejected or ignored exactly as documented by the shared policy

### Protocol Delegation
- [ ] Handshake handlers delegate to `proto::confirm`
- [ ] Ready handler delegates to `proto::ready`
- [ ] Reset handler delegates to `proto::reset`
- [ ] All delegations pass `conn` correctly

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --all-features -- netplay::handlers
```

## Gate Decision
- [ ] PASS: proceed to Phase 10
- [ ] FAIL: return to Phase 09 and fix issues

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P09a.md`
