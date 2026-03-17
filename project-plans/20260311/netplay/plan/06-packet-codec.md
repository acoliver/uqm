# Phase 06: Packet Codec & Wire Format — Stub/TDD/Impl

## Phase ID
`PLAN-20260314-NETPLAY.P06`

## Prerequisites
- Required: Phase 05a (Core Types Impl Verification) completed and passed
- Expected: core types, state machine, error types fully implemented

## Requirements Implemented (Expanded)

### REQ-NP-PROTO-001 / REQ-NP-COMPAT-003
**Requirement text**: Each wire packet shall begin with a fixed-size header containing packet length and packet type. If mixed C/Rust interoperability is required, packet framing, field encoding, and packet semantics shall be preserved.

Behavior contract:
- GIVEN: A packet struct is created
- WHEN: It is serialized
- THEN: The output is network byte order, 4-byte aligned, with correct header
- GIVEN: Raw bytes from the wire
- WHEN: They are deserialized
- THEN: A correctly typed `Packet` enum variant is produced, or an error for malformed data

### REQ-NP-PROTO-004
**Requirement text**: The end-state protocol contract shall preserve the semantic packet set (18 types).

## Implementation Tasks

### Files to create

- `rust/src/netplay/packet/types.rs` — Packet type enum and all payload structs
  - marker: `@plan PLAN-20260314-NETPLAY.P06`
  - marker: `@requirement REQ-NP-PROTO-001 REQ-NP-COMPAT-003`
  - Contents:
    - `PacketType` enum with 18 variants + `u16` discriminants matching C `packet.h:24-43`
    - `PacketType::from_u16(v) -> Option<PacketType>`
    - `PacketType::min_length(&self) -> usize` — per-type minimum sizes matching C `packet.c:31-52`
    - `Packet` enum with a variant per type, each carrying its payload struct
    - Canonical type aliases imported from overview contract: `PlayerId`, `FleetSide`, `FleetSlot`, `ShipId`
    - Payload structs:
      - `InitPayload { protocol_major: u8, protocol_minor: u8, uqm_major: u8, uqm_minor: u8, uqm_patch: u8 }`
      - `PingPayload { id: u32 }`
      - `AckPayload { id: u32 }`
      - `FleetPayload { side: FleetSide, ships: Vec<(FleetSlot, ShipId)> }`
      - `TeamNamePayload { side: FleetSide, name: String }`
      - `SeedRandomPayload { seed: u32 }`
      - `InputDelayPayload { delay: u32 }`
      - `SelectShipPayload { ship: ShipId }`
      - `BattleInputPayload { state: u8 }`
      - `FrameCountPayload { frame_count: u32 }`
      - `ChecksumPayload { frame_nr: u32, checksum: u32 }`
      - `AbortPayload { reason: u16 }`
      - `ResetPayload { reason: u16 }`
      - (Ready, Handshake0/1, HandshakeCancel, HandshakeCancelAck have no payload)
    - NOTE: the exact wire width of `ShipId` must come from the preflight C audit; if the wire uses `u8`, conversion boundaries belong here and must be explicit/tested

- `rust/src/netplay/packet/codec.rs` — Serialization/deserialization
  - marker: `@plan PLAN-20260314-NETPLAY.P06`
  - marker: `@requirement REQ-NP-PROTO-001 REQ-NP-COMPAT-003`
  - Contents:
    - `fn round_up_to_4(n: usize) -> usize`
    - `fn serialize_packet(packet: &Packet) -> Vec<u8>` — header + payload + padding
    - `fn deserialize_header(buf: &[u8]) -> Result<(u16, PacketType), NetplayError>`
    - `fn deserialize_packet(pkt_type: PacketType, buf: &[u8]) -> Result<Packet, NetplayError>`
    - Helper serializers per packet type
    - Helper deserializers per packet type
    - Network byte order via standard library

- `rust/src/netplay/packet/queue.rs` — Packet send queue
  - marker: `@plan PLAN-20260314-NETPLAY.P06`
  - Contents:
    - `PacketQueue` struct wrapping `VecDeque<Vec<u8>>`
    - `fn new() -> PacketQueue`
    - `fn enqueue(&mut self, serialized: Vec<u8>)`
    - `fn len(&self) -> usize`
    - `fn is_empty(&self) -> bool`
    - `fn drain(&mut self) -> impl Iterator<Item = Vec<u8>>`

- `rust/src/netplay/packet/receive.rs` — Read buffer and packet extraction
  - marker: `@plan PLAN-20260314-NETPLAY.P06`
  - Contents:
    - `ReadBuffer` struct: `buf: Vec<u8>`, `end: usize`
    - `fn new(capacity: usize) -> ReadBuffer`
    - `fn available(&self) -> &[u8]`
    - `fn append(&mut self, data: &[u8])`
    - `fn consume(&mut self, n: usize)`
    - `fn extract_packets(&mut self) -> Result<Vec<Packet>, NetplayError>`

- `rust/src/netplay/packet/send.rs` — Socket write helpers
  - marker: `@plan PLAN-20260314-NETPLAY.P06`
  - Contents:
    - `fn send_all(stream: &mut TcpStream, data: &[u8]) -> Result<(), NetplayError>`
    - `fn flush_queue(queue: &mut PacketQueue, stream: &mut TcpStream) -> Result<usize, NetplayError>`

### Files to modify

- `rust/src/netplay/packet/mod.rs` — Add sub-module declarations

### Tests (TDD component)

Tests in each file's `#[cfg(test)] mod tests`:

**packet/types.rs tests:**
- `test_packet_type_from_u16_valid`
- `test_packet_type_from_u16_invalid`
- `test_packet_type_min_length`
- `test_ship_id_alias_matches_c_audit` — enforce the canonical width/representation decided in preflight

**packet/codec.rs tests:**
- `test_serialize_init_packet`
- `test_deserialize_init_packet`
- `test_serialize_ready_packet`
- `test_serialize_fleet_packet_single_ship`
- `test_serialize_fleet_packet_multiple_ships`
- `test_serialize_team_name_packet`
- `test_serialize_battle_input_packet`
- `test_serialize_checksum_packet`
- `test_deserialize_header_too_short`
- `test_deserialize_header_invalid_type`
- `test_deserialize_header_length_too_small`
- `test_round_up_to_4`
- `test_all_packets_4_byte_aligned`
- `test_network_byte_order`
- `test_c_derived_packet_fixture_round_trip` — packet bytes loaded from C-derived fixture set or generated by a side-by-side harness

**packet/queue.rs tests:**
- `test_queue_empty`
- `test_queue_enqueue_dequeue`
- `test_queue_len`

**packet/receive.rs tests:**
- `test_read_buffer_extract_single_packet`
- `test_read_buffer_extract_incomplete`
- `test_read_buffer_extract_multiple`
- `test_read_buffer_consume_shifts`
- `test_read_buffer_invalid_packet_type`

**packet/send.rs tests:**
- transport-specific tests remain primarily in P07

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] All 5 packet sub-module files exist
- [ ] `packet/mod.rs` declares all sub-modules
- [ ] `PacketType` has exactly 18 variants with correct discriminants
- [ ] All payload structs use canonical aliases/types consistently
- [ ] Serialization produces bytes, not higher-level abstractions
- [ ] At least 25 tests defined across all packet modules

## Semantic Verification Checklist (Mandatory)
- [ ] Init packet serializes to exactly the same bytes as C `Packet_Init_create()`
- [ ] Fleet packet serializes using the audited canonical ship-id width/type
- [ ] TeamName packet includes NUL terminator and padding matching C `Packet_TeamName_create()`
- [ ] All header fields are big-endian (network byte order)
- [ ] All packet lengths are multiples of 4
- [ ] `deserialize(serialize(packet)) == packet` for every packet type
- [ ] Invalid header bytes produce `PacketError`, not panic
- [ ] Compatibility claims are proven by C-derived fixtures or equivalent side-by-side evidence, not only handwritten byte arrays

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|todo!()\|unimplemented!()\|placeholder" rust/src/netplay/packet/ | grep -v test
```

## Success Criteria
- [ ] All packet tests pass
- [ ] Wire format is byte-compatible with the compatibility target
- [ ] No `todo!()` remains in packet module
- [ ] Full round-trip serialization verified for all 18 packet types

## Failure Recovery
- rollback: `git checkout -- rust/src/netplay/packet/`
- blocking issues: endianness mismatches, alignment differences with C, unresolved ship-id width audit

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P06.md`
