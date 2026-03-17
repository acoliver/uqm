# Plan: Netplay Subsystem — Full C-to-Rust Port

Plan ID: PLAN-20260314-NETPLAY
Generated: 2026-03-14
Total Phases: 27 (P00.5 through P13, each with verification sub-phase)
Requirements: stable REQ-NP-* IDs from `netplay/requirements.md`

## Context

The netplay subsystem is **completely unported** and **currently disabled** in the game. All code remains in C under `sc2/src/uqm/supermelee/netplay/` (17 source files + 6 proto/ files + headers). There is no Rust-side presence whatsoever. This is the lowest-priority subsystem in the port.

The subsystem provides peer-to-peer TCP network play for SuperMelee battles: connection management, protocol version negotiation, setup synchronization, confirmation/ready/reset sub-protocols, battle-time ship-selection and input exchange with deterministic delay, and optional checksum-based sync verification.

This plan defines the full port of:
1. **Connection management** — NetConnection lifecycle, global registry, server/client branching
2. **Protocol wire format** — packet framing, serialization, all 18 packet types
3. **State machine** — 10-state NetState machine with transition enforcement
4. **Protocol sub-systems** — confirmation/handshake, ready rendezvous, reset coordination
5. **Setup synchronization** — fleet/team-name exchange, deterministic conflict resolution, bootstrap sync
6. **Pre-battle negotiation** — RNG seed exchange, input-delay negotiation
7. **Battle input delivery** — cyclic input buffers, delayed deterministic input, blocking wait with progress
8. **Ship-selection synchronization** — select-ship phase coordination and semantic-invalid-selection reset handoff
9. **Checksum verification** — CRC computation, checksum buffering, delayed verification, desync detection
10. **End-of-battle synchronization** — two-stage frame-count exchange and ready rendezvous
11. **Integration with SuperMelee** — setup menu, battle startup, ship-pick, battle-end hooks

This subsystem is **tightly coupled** with the SuperMelee subsystem (which is also unported). The plan defines trait boundaries that allow independent development. Transitional C-callable bridges are **not** implemented in this plan; they remain deferred to the SuperMelee integration plan unless a later revision explicitly adds a dedicated interop phase.

## Canonical Type Model and Public API Lock

These type aliases and signatures are the canonical contract for all later phases. Later phase files must conform exactly unless the overview is revised first.

```rust
pub type PlayerId = usize;
pub type FleetSide = u8;
pub type FleetSlot = u8;
pub type ShipId = u16;
```

Fleet packet helpers that use `(slot, ship)` tuples shall therefore use `Vec<(FleetSlot, ShipId)>` or `&[(FleetSlot, ShipId)]` consistently.

### Netplay → SuperMelee (events emitted by netplay)

```rust
pub enum NetplayEvent {
    Connected { player: PlayerId },
    ConnectionFailed { player: PlayerId, error: NetplayError },
    ConnectionClosed { player: PlayerId },
    ConfirmationInvalidated { player: PlayerId },
    ResetReceived { player: PlayerId, reason: ResetReason, by_remote: bool },
    AbortReceived { player: PlayerId, reason: AbortReason },
    RemoteFleetUpdate {
        player: PlayerId,
        side: FleetSide,
        ships: Vec<(FleetSlot, ShipId)>,
    },
    RemoteTeamNameUpdate { player: PlayerId, side: FleetSide, name: String },
    RemoteShipSelected { player: PlayerId, ship: ShipId },
    RandomSeedReceived { player: PlayerId, seed: u32 },
    InputDelayReceived { player: PlayerId, delay: u32 },
    SyncLoss { frame: u32 },
}

pub trait NetplayEventSink {
    fn on_event(&mut self, event: NetplayEvent);
}
```

### SuperMelee → Netplay (calls from melee into netplay)

```rust
pub trait NetplayApi {
    fn open_connection(&mut self, player: PlayerId, options: &PeerOptions) -> Result<(), NetplayError>;
    fn close_connection(&mut self, player: PlayerId);
    fn close_all_connections(&mut self);
    fn is_connected(&self, player: PlayerId) -> bool;
    fn is_connected_and_in_setup(&self, player: PlayerId) -> bool;
    fn connection_state(&self, player: PlayerId) -> Option<NetState>;
    fn num_connected(&self) -> usize;

    // Setup phase
    fn notify_fleet_change(&mut self, player: PlayerId, side: FleetSide, slot: FleetSlot, ship: ShipId)
        -> Result<(), NetplayError>;
    fn notify_team_name_change(&mut self, player: PlayerId, side: FleetSide, name: &str)
        -> Result<(), NetplayError>;
    fn notify_full_fleet(
        &mut self,
        player: PlayerId,
        side: FleetSide,
        fleet: &[(FleetSlot, ShipId)],
    ) -> Result<(), NetplayError>;
    fn bootstrap_sync(
        &mut self,
        player: PlayerId,
        side: FleetSide,
        fleet: &[(FleetSlot, ShipId)],
        name: &str,
    ) -> Result<(), NetplayError>;
    fn confirm_setup(&mut self, player: PlayerId) -> Result<(), NetplayError>;
    fn cancel_confirmation(&mut self, player: PlayerId) -> Result<(), NetplayError>;

    // Pre-battle
    fn advertise_input_delay(&mut self, delay: u32) -> Result<(), NetplayError>;
    fn send_random_seed(&mut self, player: PlayerId, seed: u32) -> Result<(), NetplayError>;
    fn negotiate_ready(&mut self, target_state: NetState) -> Result<(), NetplayError>;

    // Battle
    fn init_battle(&mut self, input_delay: u32, checksum_enabled: bool) -> Result<(), NetplayError>;
    fn uninit_battle(&mut self);
    fn send_battle_input(&mut self, input: u8) -> Result<(), NetplayError>;
    fn receive_battle_input(&mut self, player: PlayerId) -> Result<u8, NetplayError>;
    fn send_ship_selected(&mut self, player: PlayerId, ship: ShipId) -> Result<(), NetplayError>;

    // Checksum
    fn send_checksum(&mut self, frame: u32, checksum: u32) -> Result<(), NetplayError>;
    fn verify_checksum(&mut self, frame: u32) -> Result<bool, NetplayError>;
    fn setup_input_delay(&self) -> u32;

    // Battle end
    fn signal_battle_end(&mut self, frame_count: u32) -> Result<(), NetplayError>;

    // Reset/abort
    fn reset(&mut self, reason: ResetReason) -> Result<(), NetplayError>;
    fn wait_reset(&mut self, target_state: NetState) -> Result<(), NetplayError>;
    fn abort(&mut self, reason: AbortReason) -> Result<(), NetplayError>;

    // Polling/progress
    fn poll(&mut self) -> Result<Vec<NetplayEvent>, NetplayError>;
    fn flush(&mut self) -> Result<(), NetplayError>;
}
```

## C Files Being Replaced

### Core Transport & State (sc2/src/uqm/supermelee/netplay/)
| C File | Rust Module | Purpose |
|--------|-------------|---------|
| `netconnection.c` / `netconnection.h` | `netplay::connection` | Connection object, lifecycle |
| `nc_connect.ci` | `netplay::connection::transport` | Server/client TCP setup |
| `netstate.c` / `netstate.h` | `netplay::state` | NetState enum, debug names |
| `netmelee.c` / `netmelee.h` | `netplay::registry` | Global registry, polling, blocking waits |
| `netmisc.c` / `netmisc.h` | `netplay::callbacks` | Connect/close/error callbacks, state predicates |
| `netoptions.c` / `netoptions.h` | `netplay::options` | Runtime configuration |
| `netplay.h` | `netplay::constants` | Protocol constants, feature flags |

### Packet Layer (sc2/src/uqm/supermelee/netplay/)
| C File | Rust Module | Purpose |
|--------|-------------|---------|
| `packet.c` / `packet.h` | `netplay::packet` | Packet types, framing, type registry |
| `packethandlers.c` / `packethandlers.h` | `netplay::handlers` | Per-type receive dispatch |
| `packetsenders.c` / `packetsenders.h` | `netplay::senders` | Per-type send helpers |
| `packetq.c` / `packetq.h` | `netplay::queue` | Send queue FIFO |
| `netrcv.c` / `netrcv.h` | `netplay::receive` | Read buffer, packet extraction |
| `netsend.c` / `netsend.h` | `netplay::send` | Socket write loop |

### Notifications (sc2/src/uqm/supermelee/netplay/)
| C File | Rust Module | Purpose |
|--------|-------------|---------|
| `notify.c` / `notify.h` | `netplay::notify` | Per-connection notifications |
| `notifyall.c` / `notifyall.h` | `netplay::notify` | Broadcast to all connections |

### Protocol Sub-modules (sc2/src/uqm/supermelee/netplay/proto/)
| C File | Rust Module | Purpose |
|--------|-------------|---------|
| `npconfirm.c` / `npconfirm.h` | `netplay::proto::confirm` | Setup confirmation handshake |
| `ready.c` / `ready.h` | `netplay::proto::ready` | Generic ready rendezvous |
| `reset.c` / `reset.h` | `netplay::proto::reset` | Reset coordination |

### Battle Input & Sync (sc2/src/uqm/supermelee/netplay/)
| C File | Rust Module | Purpose |
|--------|-------------|---------|
| `netinput.c` / `netinput.h` | `netplay::input` | Battle input buffers, delivery |
| `checkbuf.c` / `checkbuf.h` | `netplay::checksum::buffer` | Checksum ring buffer |
| `checksum.c` / `checksum.h` | `netplay::checksum` | CRC computation, verification |
| `crc.c` / `crc.h` | `netplay::checksum::crc` | CRC32 algorithm |

## New Rust Module Structure

```
rust/src/netplay/
  mod.rs                          # Module root, feature-gated, re-exports
  error.rs                        # NetplayError enum (thiserror)
  constants.rs                    # Protocol version, compile-time constants
  options.rs                      # NetplayOptions, PeerOptions runtime config
  state.rs                        # NetState enum, transition rules, predicates

  connection/
    mod.rs                        # Connection sub-module root
    net_connection.rs             # NetConnection struct, lifecycle, state flags
    transport.rs                  # Server/client TCP connect, accept
    registry.rs                   # Global connection array, add/remove/iterate

  packet/
    mod.rs                        # Packet sub-module root
    types.rs                      # PacketType enum, PacketHeader, all Packet_* structs
    codec.rs                      # Serialize/deserialize, network byte order, padding
    queue.rs                      # PacketQueue FIFO
    receive.rs                    # Read buffer, packet extraction, dispatch
    send.rs                       # Socket write, flush

  handlers/
    mod.rs                        # Handler sub-module root, dispatch table
    init.rs                       # PacketHandler_Init (version validation)
    setup.rs                      # Fleet, TeamName, Handshake* handlers
    battle.rs                     # BattleInput, SelectShip, FrameCount handlers
    sync.rs                       # Checksum, SeedRandom, InputDelay handlers
    control.rs                    # Ready, Abort, Reset, Ping/Ack handlers

  proto/
    mod.rs                        # Protocol sub-module root
    confirm.rs                    # Setup confirmation/cancellation protocol
    ready.rs                      # Generic ready rendezvous primitive
    reset.rs                      # Reset coordination protocol

  notify/
    mod.rs                        # Notification sub-module root
    per_connection.rs             # Per-connection notification functions
    broadcast.rs                  # Fan-out to all connected peers

  input/
    mod.rs                        # Battle input sub-module root
    buffer.rs                     # BattleInputBuffer, cyclic buffer
    delivery.rs                   # networkBattleInput(), blocking wait

  checksum/
    mod.rs                        # Checksum sub-module root
    crc.rs                        # CRC32 computation
    buffer.rs                     # ChecksumBuffer, ChecksumEntry ring buffer
    verify.rs                     # Checksum comparison, desync detection

  integration/
    mod.rs                        # Integration sub-module root
    melee_hooks.rs                # SuperMelee setup integration callbacks
    battle_hooks.rs               # Battle startup/shutdown integration
    event.rs                      # NetplayEvent enum for UI feedback
```

## Netplay → Lower Network Layer

```rust
pub trait NetworkTransport {
    fn listen(&mut self, port: u16) -> Result<TransportHandle, NetplayError>;
    fn connect(&mut self, host: &str, port: u16) -> Result<TransportHandle, NetplayError>;
    fn send(&mut self, handle: &TransportHandle, data: &[u8]) -> Result<usize, NetplayError>;
    fn recv(&mut self, handle: &TransportHandle, buf: &mut [u8]) -> Result<usize, NetplayError>;
    fn close(&mut self, handle: TransportHandle);
    fn poll(&mut self, timeout_ms: u32) -> Result<Vec<TransportEvent>, NetplayError>;
}
```

`std::net` remains the leading implementation candidate, but transport implementation is not considered locked until preflight/analysis explicitly validate the progress model against specification §8.4 and §18. The plan therefore treats `std::net` + non-blocking polling as a design candidate subject to validation, not as an already-proven replacement for the old lower networking layer.

## Progress Model Lock

All blocking waits in later phases (`receive_battle_input`, `negotiate_ready`, `wait_reset`) must use one shared progress model:
1. flush queued outgoing packets,
2. poll/receive transport traffic,
3. dispatch received packets,
4. run one-shot completion callbacks/events produced by dispatch,
5. re-check the awaited condition,
6. sleep/yield only when no forward progress was possible.

This model exists to satisfy specification §8.4 and §18.2: waiting must not starve receive processing, deferred callbacks, or queued outbound traffic.

## Integration Points

### Existing Rust Subsystems
| Subsystem | Integration |
|-----------|-------------|
| `supermelee` (future) | Setup flow, confirmation, team sync, ship-pick |
| `state` | `CurrentActivity` flags for `CHECK_ABORT` |
| `threading` | Sleep/yield during blocking waits |
| `config` | `netplayOptions` configuration |

### C-Side Integration
| C System | Direction | Purpose |
|----------|-----------|---------|
| `melee.c` callbacks | Rust→C (deferred) | connectedFeedback, closeFeedback, etc. |
| `battle.c` hooks | Rust↔C (deferred) | Input buffer init, checksum hooks |
| `pickmele.c` | Rust↔C (deferred) | Ship selection sync |
| `tactrans.c` | Rust↔C (deferred) | Battle-end sync |
| `libs/network/` | Behavioral boundary only | Rust transport must preserve progress semantics |

### Feature Gate

The entire subsystem is gated behind `#[cfg(feature = "netplay")]` in Rust, mirroring the C `#ifdef NETPLAY` conditional compilation. The `netplay` feature is added to `Cargo.toml` but **not** included in `default` features.

## Phase Structure

| Phase | Title | Est. LoC |
|-------|-------|----------|
| P00.5 | Preflight Verification | 0 |
| P01 | Analysis | 0 |
| P01a | Analysis Verification | 0 |
| P02 | Pseudocode | 0 |
| P02a | Pseudocode Verification | 0 |
| P03 | Core Types & State Machine — Stub | ~450 |
| P03a | Core Types Verification | 0 |
| P04 | Core Types & State Machine — TDD | ~350 |
| P04a | Core Types TDD Verification | 0 |
| P05 | Core Types & State Machine — Impl | ~250 |
| P05a | Core Types Impl Verification | 0 |
| P06 | Packet Codec & Wire Format — Stub/TDD/Impl | ~900 |
| P06a | Packet Codec Verification | 0 |
| P07 | Connection & Transport — Stub/TDD/Impl | ~800 |
| P07a | Connection & Transport Verification | 0 |
| P08 | Protocol Sub-systems (Ready/Confirm/Reset) — Stub/TDD/Impl | ~700 |
| P08a | Protocol Sub-systems Verification | 0 |
| P09 | Packet Handlers & Dispatch — Stub/TDD/Impl | ~1100 |
| P09a | Packet Handlers Verification | 0 |
| P10 | Setup Sync & Notifications — Stub/TDD/Impl | ~600 |
| P10a | Setup Sync Verification | 0 |
| P11 | Battle Input & Checksum Primitives + Delivery — Stub/TDD/Impl | ~800 |
| P11a | Battle Input & Checksum Verification | 0 |
| P12 | Integration & Event Hooks — Stub/TDD/Impl | ~500 |
| P12a | Integration Verification | 0 |
| P13 | End-to-End Integration & Verification | ~300 |

Total estimated new LoC: ~6750 (Rust)

## Execution Order

```
P00.5 → P01 → P01a → P02 → P02a
      → P03 → P03a → P04 → P04a → P05 → P05a
      → P06 → P06a
      → P07 → P07a
      → P08 → P08a
      → P11 → P11a
      → P09 → P09a
      → P10 → P10a
      → P12 → P12a
      → P13
```

Each phase MUST be completed and verified before the next begins. No skipping.

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 0.5)
2. Integration points are explicitly listed
3. TDD cycle is defined per slice
4. Lint/test/coverage gates are declared
5. Requirement references use stable IDs from `requirements.md`
6. The canonical public API and type aliases in this file are the source of truth

## Definition of Done

1. All `cargo test --workspace --all-features` pass (with `netplay` feature)
2. All `cargo clippy --workspace --all-targets --all-features -- -D warnings` pass
3. `cargo fmt --all --check` passes
4. Two peers can establish a TCP connection and complete protocol initialization
5. Setup synchronization exchanges fleet and team-name data correctly
6. Deterministic setup conflict resolution is specified, implemented, and tested with crossing edits
7. Confirmation handshake protocol works with cancellation
8. Ready rendezvous completes for all required phase transitions without starving progress
9. Pre-battle negotiation exchanges RNG seed and input delay
10. Battle input buffering delivers deterministic delayed input
11. Checksum verification detects desync and triggers reset
12. End-of-battle synchronization exchanges frame counts and completes cleanly
13. Reset protocol works bidirectionally with correct state transitions
14. Abort protocol terminates connections cleanly
15. Ship-selection synchronization works during battle, including semantic-invalid-selection reset handling
16. All events are surfaced to the SuperMelee integration boundary
17. No placeholder stubs or TODO markers remain in implementation code
18. Wire compatibility is validated with C-derived fixtures, mixed-peer interop, or both when interoperability is required

## Plan Files

```
plan/
  00-overview.md                                    (this file)
  00a-preflight-verification.md                     P00.5
  01-analysis.md                                    P01
  01a-analysis-verification.md                      P01a
  02-pseudocode.md                                  P02
  02a-pseudocode-verification.md                    P02a
  03-core-types-stub.md                             P03
  03a-core-types-stub-verification.md               P03a
  04-core-types-tdd.md                              P04
  04a-core-types-tdd-verification.md                P04a
  05-core-types-impl.md                             P05
  05a-core-types-impl-verification.md               P05a
  06-packet-codec.md                                P06
  06a-packet-codec-verification.md                  P06a
  07-connection-transport.md                        P07
  07a-connection-transport-verification.md          P07a
  08-protocol-subsystems.md                         P08
  08a-protocol-subsystems-verification.md           P08a
  09-packet-handlers.md                             P09
  09a-packet-handlers-verification.md               P09a
  10-setup-sync-notifications.md                    P10
  10a-setup-sync-notifications-verification.md      P10a
  11-battle-input-checksum.md                       P11
  11a-battle-input-checksum-verification.md         P11a
  12-integration-event-hooks.md                     P12
  12a-integration-event-hooks-verification.md       P12a
  13-e2e-integration-verification.md                P13
  requirements-traceability-matrix.md
  execution-tracker.md
```

## Deferred Items

The following are explicitly out of scope for this plan:

- **SuperMelee subsystem porting**: Menu flow, team state, battle orchestration remain C-owned until the SuperMelee plan executes. This plan defines trait boundaries only.
- **C transitional FFI bridge implementation**: No minimal C-callable bridge layer is implemented in this plan.
- **Metaserver/discovery**: The `NetplayOptions` struct includes metaserver fields but the active code does not use them. Metaserver support is deferred.
- **UDP transport**: The current implementation is TCP-only. No UDP path is planned.
- **More than 2 players**: The implementation is hardcoded for 2-player sessions. Multi-player expansion is deferred.
- **Campaign netplay**: Only SuperMelee netplay is in scope.

## Dependency on Other Subsystem Plans

| Dependency | Nature | Status |
|-----------|--------|--------|
| SuperMelee (PLAN-20260314-SUPERMELEE) | Netplay integrates into melee.c, battle.c, pickmele.c, tactrans.c | Unported, plan exists |
| Ships | Ship IDs used in fleet/selection packets | Unported |
| Graphics | Not directly used by netplay | Partially ported |
| Input | Battle input state type | Partially ported |
| Threading | Sleep/yield for blocking waits | Ported |

Netplay can be developed independently using trait-based boundaries. Full game integration testing requires the SuperMelee plan or a later dedicated interop phase.
