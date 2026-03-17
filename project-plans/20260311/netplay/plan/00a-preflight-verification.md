# Phase 0.5: Preflight Verification

## Phase ID
`PLAN-20260314-NETPLAY.P00.5`

## Purpose
Verify assumptions about the codebase, toolchain, compatibility obligations, and transport/progress boundaries before any implementation begins.

## Toolchain Verification
- [ ] `cargo --version` — confirm Rust 2021 edition support
- [ ] `rustc --version` — confirm stable toolchain
- [ ] `cargo clippy --version` — confirm clippy available
- [ ] `cargo llvm-cov --version` — verify if coverage gate is feasible

## Dependency Verification

### Existing crates in Cargo.toml
- [ ] `thiserror` crate present — needed for `NetplayError`
- [ ] `libc` crate present — needed only if a later C bridge phase is added; not required by this plan itself
- [ ] `anyhow` crate present if already project-standard — otherwise do not add just for netplay
- [ ] `crossbeam` crate present only if a later design explicitly uses channels/event passing
- [ ] `parking_lot` crate present only if registry locking is proven necessary

### New crates to evaluate only if needed
- [ ] `mio` — evaluate for non-blocking TCP event loop if plain `std::net` cannot satisfy the progress contract
- [ ] `byteorder` — evaluate only if manual `to_be_bytes()/from_be_bytes()` becomes too error-prone
- [ ] `tempfile` — evaluate only if fixture generation or file-based interop tests require it
- [ ] `proptest` — evaluate only if packet round-trip/property tests are adopted during implementation
- [ ] Decision recorded with justification tied to a concrete use site, not speculative convenience

### Feature flag setup
- [ ] `Cargo.toml` does NOT currently have a `netplay` feature — must be added
- [ ] `netplay` feature should NOT be in `default` features
- [ ] All netplay code must compile-guard with `#[cfg(feature = "netplay")]`

## Type/Interface Verification

### Existing Rust Types That May Be Relevant
- [ ] `rust/src/state/` — verify game state types exist for `CurrentActivity` / `CHECK_ABORT` flags
- [ ] `rust/src/threading/` — verify sleep/yield primitives exist for blocking wait loops
- [ ] `rust/src/config.rs` — verify `Options` struct exists (netplay options will extend it)
- [ ] `rust/src/input/` — verify battle input state type exists or can be defined

### C Types That Must Be Understood for Wire Compatibility
- [ ] `Packet_Init` struct layout in `sc2/src/uqm/supermelee/netplay/packet.h:104-116`
- [ ] `PacketHeader` struct layout in `packet.h:75-78` — `uint16 len`, `uint16 type`
- [ ] `Packet_Fleet` struct with variable-length `FleetEntry[]` array
- [ ] Audit actual ship identifier width used by fleet and select-ship packets; record canonical Rust `ShipId`
- [ ] Audit `side` / `player` / `slot` meanings in the C protocol and map them to `FleetSide`, `PlayerId`, `FleetSlot`
- [ ] `Packet_TeamName` struct with variable-length name + NUL + padding
- [ ] `Packet_BattleInput` struct — single `uint8 state` payload
- [ ] `Packet_Checksum` struct — `uint32 frameNr`, `uint32 checksum`
- [ ] `BattleInputBuffer` struct in `netinput.h:28-36` — cyclic buffer model
- [ ] `ChecksumBuffer` / `ChecksumEntry` in `checkbuf.h:42-63`
- [ ] `NetConnection` struct in `netconnection.h:145-191` — all state fields
- [ ] `NetStateFlags` in `netconnection.h:95-143` — nested flag structs

### C Constants That Must Be Preserved
- [ ] Protocol version: major `0`, minor `4` (`netplay.h:24-25`)
- [ ] Min UQM version: `0.6.9` (`netplay.h:27-29`)
- [ ] Read buffer size: `2048` (`netplay.h:41`)
- [ ] Connect timeout: `2000` ms (`netplay.h:43`)
- [ ] Retry delay: `2000` ms (`netplay.h:45`)
- [ ] Listen backlog: `2` (`netplay.h:47`)
- [ ] Default input delay: `2` (`netoptions.c:36`)
- [ ] Default port: `21837` (`netoptions.c:30-31`)
- [ ] `BATTLE_FRAME_RATE` — used for input delay validation upper bound
- [ ] `NUM_PLAYERS = 2` — hardcoded peer count

## Transport and Progress-Model Feasibility

### Connection Lifecycle Path
```
SuperMelee → openPlayerNetworkConnection()
  → NetConnection_open() → NetConnection_go()
    → server: listenPort() → accept → NetConnection_connected()
    → client: connectHostByName() → connect → NetConnection_connected()
  → NetMelee_connectCallback() → sendInit() → Netplay_localReady()
  → PacketHandler_Init() → Netplay_remoteReady() → bothReady
  → NetMelee_enterState_inSetup() → Melee_bootstrapSyncTeam()
```

- [ ] Verify `std::net::TcpListener::bind()` + `accept()` is sufficient for server mode
- [ ] Verify `std::net::TcpStream::connect_timeout()` is sufficient for client mode
- [ ] Verify non-blocking TCP socket I/O is available via `set_nonblocking(true)`
- [ ] Verify the chosen transport approach can flush pending sends, receive packets, dispatch handlers, and run completion callbacks in one loop
- [ ] Verify the chosen approach satisfies specification §8.4 and §18.2 without starving receive processing or deferred protocol actions
- [ ] Record the polling/event-loop contract that later phases must obey during blocking waits

### Setup Sync Path
```
Local edit → Melee_LocalChange_ship() → Netplay_NotifyAll_setShip()
  → queuePacket(Packet_Fleet) → flushPacketQueues() → sendPacket()
Remote receive → dataReadyCallback() → PacketHandler_Fleet()
  → Melee_RemoteChange_ship() → conflict resolution → apply
```

- [ ] Verify packet serialization can be tested without a live socket (buffer-based)
- [ ] Verify conflict resolution logic can be tested deterministically
- [ ] Identify the exact data needed for deterministic setup conflict resolution (side, slot, ship, discriminant, ordering, local pending state)

### Battle Input Path
```
Battle start → initBattleInputBuffers() → pre-fill with neutral input
Battle loop → local: Netplay_NotifyAll_battleInput(state) → queue + flush
           → remote: networkBattleInput(player) → pop from BattleInputBuffer
             → if empty: netInputBlocking(500ms) → retry
```

- [ ] Verify cyclic buffer can be tested independently of socket I/O
- [ ] Verify blocking input delivery can be tested with mock transport or deterministic loopback
- [ ] Verify the same progress loop can service input waits, ready waits, and reset waits

## Test Infrastructure Verification
- [ ] `cargo test --workspace` succeeds currently
- [ ] Test files can be created in `rust/src/netplay/` with `#[cfg(test)]`
- [ ] `#[cfg(feature = "netplay")]` modules are included when testing with `--all-features`
- [ ] Fixture-generation path identified for byte-compatibility proofs (C-derived bytes, side-by-side harness, or mixed-peer interop)

## Wire Compatibility Verification
- [ ] Confirm whether the project requires C↔Rust interoperability (mixed peers)
- [ ] If yes: packet layouts, endianness, padding, and semantics must remain byte-compatible
- [ ] If yes: at least one later verification path must use C-derived fixtures or mixed C/Rust interop rather than handwritten expected bytes alone
- [ ] If no: semantic compatibility is sufficient, but the plan still preserves the legacy wire format until a deliberate compatibility decision changes it
- [ ] **Default assumption**: preserve byte-compatible wire format until explicitly decided otherwise

## Blocking Issues
[List any blockers found during verification. If non-empty, revise plan before proceeding.]

Potential blockers:
1. If `std::net` cannot support the required progress model cleanly, transport design must be revised before P07.
2. If ship/fleet identifier widths are inconsistent across C packet types, canonical Rust aliases must be fixed before P06/P10/P12.
3. If mixed-peer interoperability is required but no fixture/interop proof path is available, P13 verification needs revision before execution.

## Gate Decision
- [ ] PASS: proceed to Phase 01
- [ ] FAIL: revise plan — document specific issues and required changes
