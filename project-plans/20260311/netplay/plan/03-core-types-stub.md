# Phase 03: Core Types & State Machine — Stub

## Phase ID
`PLAN-20260314-NETPLAY.P03`

## Prerequisites
- Required: Phase 02a (Pseudocode Verification) completed and passed
- Expected artifacts from previous phases: analysis, pseudocode

## Requirements Implemented (Expanded)

### REQ-NP-STATE-001
**Requirement text**: The subsystem shall maintain connection state sufficient to distinguish unconnected, connecting, init, in-setup, pre-battle, inter-battle, select-ship, in-battle, and end-of-battle synchronization phases.

Behavior contract:
- GIVEN: A netplay connection exists
- WHEN: Any state query is made
- THEN: The current phase is unambiguously identifiable

Why it matters: Every protocol operation depends on knowing the current connection state.

### REQ-NP-PROTO-001
**Requirement text**: When a peer initialization packet is received, the subsystem shall validate the peer's protocol version against the supported protocol version policy.

Behavior contract:
- GIVEN: Protocol constants are defined
- WHEN: An Init packet is parsed
- THEN: Version fields are available for comparison

### REQ-NP-ERROR-004
**Requirement text**: The subsystem shall preserve deterministic convergence for valid sessions and shall fail loudly/cleanly for invalid sessions.

Behavior contract:
- GIVEN: Any netplay operation
- WHEN: An error occurs
- THEN: The error is typed, not a panic

## Implementation Tasks

### Files to create

- `rust/src/netplay/mod.rs` — Module root, feature-gated
  - marker: `@plan PLAN-20260314-NETPLAY.P03`
  - Contents:
    - `#![cfg(feature = "netplay")]` attribute
    - Sub-module declarations: `error`, `constants`, `options`, `state`
    - Sub-module declarations (empty): `connection`, `packet`, `handlers`, `proto`, `notify`, `input`, `checksum`, `integration`

- `rust/src/netplay/error.rs` — Error types
  - marker: `@plan PLAN-20260314-NETPLAY.P03`
  - marker: `@requirement REQ-NP-ERROR-004`
  - Contents:
    - `NetplayError` enum with variants:
      - `ConnectionFailed(String)`
      - `ConnectionClosed`
      - `VersionMismatch { local: String, remote: String }`
      - `InvalidState { expected: String, actual: String }`
      - `InvalidTransition { from: NetState, to: NetState }`
      - `ProtocolError(String)`
      - `PacketError(String)`
      - `BufferFull`
      - `BufferEmpty`
      - `TransportError(std::io::Error)`
      - `AlreadyConfirmed`
      - `NotConfirmed`
      - `AlreadyReady`
      - `NotInSetup`
      - `SyncLoss { frame: u32 }`
      - `Timeout`
      - `Aborted(AbortReason)`
    - `AbortReason` enum: `Unspecified`, `VersionMismatch`, `InvalidHash`, `ProtocolError`
    - `ResetReason` enum: `Unspecified`, `ManualReset`, `SyncLoss`
    - shared canonical aliases used everywhere else in the plan:
      - `pub type PlayerId = usize`
      - `pub type FleetSide = u8`
      - `pub type FleetSlot = u8`
      - `pub type ShipId = u16`
    - `impl From<std::io::Error> for NetplayError`

- `rust/src/netplay/constants.rs` — Protocol constants
  - marker: `@plan PLAN-20260314-NETPLAY.P03`
  - marker: `@requirement REQ-NP-PROTO-001`
  - Contents:
    - `PROTOCOL_VERSION_MAJOR: u8 = 0`
    - `PROTOCOL_VERSION_MINOR: u8 = 4`
    - `MIN_UQM_VERSION_MAJOR: u8 = 0`
    - `MIN_UQM_VERSION_MINOR: u8 = 6`
    - `MIN_UQM_VERSION_PATCH: u8 = 9`
    - `READ_BUF_SIZE: usize = 2048`
    - `CONNECT_TIMEOUT_MS: u64 = 2000`
    - `RETRY_DELAY_MS: u64 = 2000`
    - `LISTEN_BACKLOG: u32 = 2`
    - `NUM_PLAYERS: usize = 2`
    - `DEFAULT_PORT: u16 = 21837`
    - `DEFAULT_INPUT_DELAY: u32 = 2`
    - `MAX_INPUT_DELAY: u32 = 24` (BATTLE_FRAME_RATE)
    - `CHECKSUM_INTERVAL: u32 = 1`
    - `NETWORK_POLL_DELAY_MS: u64 = 1`
    - `MAX_BLOCK_TIME_MS: u64 = 500`
    - `PACKET_HEADER_SIZE: usize = 4`

- `rust/src/netplay/options.rs` — Runtime configuration
  - marker: `@plan PLAN-20260314-NETPLAY.P03`
  - Contents:
    - `PeerOptions` struct: `host: String`, `port: u16`, `is_server: bool`
    - `NetplayOptions` struct: `peers: [PeerOptions; NUM_PLAYERS]`, `input_delay: u32`
    - `impl Default for PeerOptions` (localhost:21837, is_server=true)
    - `impl Default for NetplayOptions`

- `rust/src/netplay/state.rs` — NetState enum and predicates
  - marker: `@plan PLAN-20260314-NETPLAY.P03`
  - marker: `@requirement REQ-NP-STATE-001`
  - Contents:
    - `NetState` enum with 10 variants: `Unconnected`, `Connecting`, `Init`, `InSetup`, `PreBattle`, `InterBattle`, `SelectShip`, `InBattle`, `EndingBattle`, `EndingBattle2`
    - `impl NetState`:
      - `fn is_handshake_meaningful(&self) -> bool`
      - `fn is_ready_meaningful(&self) -> bool`
      - `fn is_battle_active(&self) -> bool`
      - `fn name(&self) -> &'static str` (debug names)
    - `StateFlags` struct with nested sub-structs:
      - `connected: bool`
      - `disconnected: bool`
      - `discriminant: bool`
      - `handshake: HandshakeFlags { local_ok, remote_ok, canceling }`
      - `ready: ReadyFlags { local_ready, remote_ready }`
      - `reset: ResetFlags { local_reset, remote_reset }`
      - `agreement: AgreementFlags { random_seed }`
      - `input_delay: u32`
      - `checksum_interval: u32`
    - `impl Default for StateFlags`

- `rust/src/netplay/connection/mod.rs` — Connection sub-module root (empty declarations)
  - marker: `@plan PLAN-20260314-NETPLAY.P03`

- `rust/src/netplay/packet/mod.rs` — Packet sub-module root (empty declarations)
  - marker: `@plan PLAN-20260314-NETPLAY.P03`

- `rust/src/netplay/handlers/mod.rs` — Handlers sub-module root (empty declarations)
  - marker: `@plan PLAN-20260314-NETPLAY.P03`

- `rust/src/netplay/proto/mod.rs` — Protocol sub-module root (empty declarations)
  - marker: `@plan PLAN-20260314-NETPLAY.P03`

- `rust/src/netplay/notify/mod.rs` — Notify sub-module root (empty declarations)
  - marker: `@plan PLAN-20260314-NETPLAY.P03`

- `rust/src/netplay/input/mod.rs` — Input sub-module root (empty declarations)
  - marker: `@plan PLAN-20260314-NETPLAY.P03`

- `rust/src/netplay/checksum/mod.rs` — Checksum sub-module root (empty declarations)
  - marker: `@plan PLAN-20260314-NETPLAY.P03`

- `rust/src/netplay/integration/mod.rs` — Integration sub-module root (empty declarations)
  - marker: `@plan PLAN-20260314-NETPLAY.P03`

### Files to modify

- `rust/src/lib.rs` — Add `#[cfg(feature = "netplay")] pub mod netplay;`
  - marker: `@plan PLAN-20260314-NETPLAY.P03`

- `rust/Cargo.toml` — Add `netplay = []` to `[features]` section
  - marker: `@plan PLAN-20260314-NETPLAY.P03`

### Pseudocode traceability
- Uses pseudocode lines: 1-31 (Component 001: State Machine), 62-92 (Component 003: Connection struct outline)

## Verification Commands

```bash
# Structural gate — must pass with netplay feature
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Verify feature gating — must also pass WITHOUT netplay feature
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Structural Verification Checklist
- [ ] `rust/src/netplay/mod.rs` exists and declares all sub-modules
- [ ] `rust/src/netplay/error.rs` exists with `NetplayError`, `AbortReason`, `ResetReason`, and the canonical type aliases
- [ ] `rust/src/netplay/constants.rs` exists with all protocol constants
- [ ] `rust/src/netplay/options.rs` exists with `PeerOptions`, `NetplayOptions`
- [ ] `rust/src/netplay/state.rs` exists with `NetState`, `StateFlags`
- [ ] All 8 sub-module `mod.rs` files exist (connection, packet, handlers, proto, notify, input, checksum, integration)
- [ ] `rust/src/lib.rs` includes `#[cfg(feature = "netplay")] pub mod netplay`
- [ ] `Cargo.toml` has `netplay = []` feature
- [ ] All files compile with `--all-features`
- [ ] All files compile WITHOUT `netplay` feature (module is excluded)

## Semantic Verification Checklist (Mandatory)
- [ ] `NetState` has exactly 10 variants matching `netstate.h:25-42`
- [ ] `is_handshake_meaningful()` returns true only for `InSetup`
- [ ] `is_ready_meaningful()` returns true for the 7 states documented in `netmisc.h`
- [ ] `is_battle_active()` returns true for `InBattle`, `EndingBattle`, `EndingBattle2`
- [ ] `AbortReason` has 4 variants matching `packet.h:47-55`
- [ ] Protocol constants match `netplay.h:24-55` and `netoptions.c:19-37`
- [ ] Canonical aliases `PlayerId`, `FleetSide`, `FleetSlot`, and `ShipId` are defined once and reused consistently by later phases
- [ ] `StateFlags` nested structure matches `netconnection.h:95-143`
- [ ] Error enum covers all error scenarios identified in analysis

## Deferred Implementation Detection (Mandatory)

```bash
# Stub phase: todo!() is ALLOWED in method bodies only
# Verify no fake success returns
grep -RIn "Ok(())\|return true\|return false" rust/src/netplay/ | grep -v test | grep -v "todo!"
```

## Success Criteria
- [ ] All new files compile with `--all-features`
- [ ] Module structure matches plan
- [ ] Types are defined with correct field types
- [ ] Feature gating works (build passes with and without `netplay` feature)
- [ ] `cargo test` passes (no tests yet, just compilation)

## Failure Recovery
- rollback: `git checkout -- rust/src/netplay/ rust/src/lib.rs rust/Cargo.toml`
- blocking issues: feature gating syntax, type definition issues

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P03.md`
