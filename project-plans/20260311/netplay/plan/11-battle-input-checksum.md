# Phase 11: Battle Input & Checksum Primitives + Delivery — Stub/TDD/Impl

## Phase ID
`PLAN-20260314-NETPLAY.P11`

## Prerequisites
- Required: Phase 08a (Protocol Sub-systems Verification) completed and passed
- Expected: protocol, connection, packet infrastructure

## Requirements Implemented (Expanded)

### REQ-NP-INPUT-001..007
**Requirement text**: When battle begins, the subsystem shall initialize battle input buffers supporting deterministic delayed-input startup. When battle requests network-controlled input and it is available, return next buffered input in order. When not yet available, continue servicing network while waiting. The subsystem shall not deliver remote battle input out of order.

Behavior contract:
- GIVEN: Input delay = 2
- WHEN: Battle input buffers initialized
- THEN: Each buffer has capacity `2*2+2 = 6` and is pre-filled with 2 neutral frames
- GIVEN: Buffer has data
- WHEN: `pop()` called
- THEN: Returns oldest entry (FIFO order)
- GIVEN: Buffer empty, connection alive
- WHEN: `receive_battle_input()` called
- THEN: Uses the shared progress loop until input arrives or timeout/abort/disconnect

### REQ-NP-CHECK-001..004
**Requirement text**: When checksum verification is enabled, the subsystem shall periodically transmit checksums and compare them after the delayed-input window has elapsed. When mismatch detected, initiate sync-loss reset.

Behavior contract:
- GIVEN: Checksum enabled, input_delay=2, interval=1
- WHEN: Frame N completed
- THEN: CRC of game state computed, stored locally, sent to peer
- GIVEN: Both local and remote checksums available for frame N-delay
- WHEN: `verify_checksum(N-delay)` called
- THEN: Returns `Ok(true)` if match, `Ok(false)` if mismatch

### REQ-NP-DELAY-001..004 / REQ-NP-READY-004
**Requirement text**: Input delay negotiation must be deterministic, and waits must continue servicing network progress.

Behavior contract:
- GIVEN: Local prefers delay=2, remote advertises delay=3
- WHEN: `setup_input_delay()` called after negotiation
- THEN: Effective delay = max(2, 3) = 3

## Implementation Tasks

### Files to create

- `rust/src/netplay/input/buffer.rs` — Cyclic battle input buffer
  - marker: `@plan PLAN-20260314-NETPLAY.P11`
  - marker: `@requirement REQ-NP-INPUT-001 REQ-NP-INPUT-003 REQ-NP-INPUT-004 REQ-NP-INPUT-007`
  - Contents:
    - `BattleInputBuffer` struct
    - `fn new(input_delay: u32) -> BattleInputBuffer`
    - `fn push(&mut self, input: u8) -> Result<(), NetplayError>`
    - `fn pop(&mut self) -> Option<u8>`
    - `fn len(&self) -> usize`
    - `fn is_empty(&self) -> bool`
    - `fn is_full(&self) -> bool`
    - `fn capacity(&self) -> usize`
    - `fn clear(&mut self)`

- `rust/src/netplay/input/delivery.rs` — Network battle input delivery and wait-progress loop
  - marker: `@plan PLAN-20260314-NETPLAY.P11`
  - marker: `@requirement REQ-NP-INPUT-005 REQ-NP-INPUT-006 REQ-NP-DELAY-002 REQ-NP-READY-004`
  - Contents:
    - `BattleInputState` struct: `buffers: [BattleInputBuffer; NUM_PLAYERS]`, `input_delay: u32`
    - `fn init_buffers(input_delay: u32) -> BattleInputState`
    - `fn receive_input(state: &mut BattleInputState, player: usize, conn_registry: &mut ConnectionRegistry) -> Result<u8, NetplayError>`
      - Try pop from buffer
      - If empty: enter shared progress loop
      - Shared progress loop must flush queues, poll transport, dispatch handlers, run completion callbacks/events, then retry buffer pop
      - On connection lost or abort: return error
    - `fn setup_input_delay(registry: &ConnectionRegistry, local_pref: u32) -> u32`
      - Scan all connections' advertised delays
      - Return max of all + local preference
    - `fn input_delay(&self) -> u32`

- `rust/src/netplay/checksum/crc.rs` — CRC32 computation
  - marker: `@plan PLAN-20260314-NETPLAY.P11`
  - marker: `@requirement REQ-NP-CHECK-001 REQ-NP-COMPAT-003`
  - Contents:
    - `CRC_TABLE: [u32; 256]`
    - `fn crc32_init() -> u32`
    - `fn crc32_update(crc: u32, byte: u8) -> u32`
    - `fn crc32_finalize(crc: u32) -> u32`
    - `fn crc32_of_bytes(data: &[u8]) -> u32`
    - `fn crc32_of_u32(crc: u32, value: u32) -> u32` — feed bytes in the order required by the C compatibility target

- `rust/src/netplay/checksum/buffer.rs` — Checksum ring buffer
  - marker: `@plan PLAN-20260314-NETPLAY.P11`
  - marker: `@requirement REQ-NP-CHECK-002 REQ-NP-CHECK-003`
  - Contents:
    - `ChecksumEntry` struct
    - `ChecksumBuffer` struct: `entries: Vec<Option<ChecksumEntry>>`, `capacity: usize`, `checksum_interval: u32`
    - `fn new(input_delay: u32, checksum_interval: u32) -> ChecksumBuffer`
    - `fn add(&mut self, frame: u32, checksum: u32) -> Result<(), NetplayError>`
    - `fn get(&self, frame: u32) -> Option<u32>`
    - `fn clear(&mut self)`

- `rust/src/netplay/checksum/verify.rs` — Checksum comparison and desync detection
  - marker: `@plan PLAN-20260314-NETPLAY.P11`
  - marker: `@requirement REQ-NP-CHECK-004 REQ-NP-RESET-005`
  - Contents:
    - `ChecksumVerifier` struct: `local_buffer: ChecksumBuffer`, `remote_buffers: Vec<ChecksumBuffer>`, `input_delay: u32`, `checksum_interval: u32`
    - `fn new(input_delay: u32, checksum_interval: u32, num_peers: usize) -> ChecksumVerifier`
    - `fn add_local(&mut self, frame: u32, checksum: u32)`
    - `fn add_remote(&mut self, peer: usize, frame: u32, checksum: u32)`
    - `fn verify(&self, frame: u32) -> Result<bool, NetplayError>`
    - `fn verification_frame(&self, current_frame: u32) -> Option<u32>`

### Files to modify

- `rust/src/netplay/input/mod.rs` — Add sub-module declarations
- `rust/src/netplay/checksum/mod.rs` — Add sub-module declarations

### Why this phase precedes packet handlers

P09 handlers for `BattleInput` and `Checksum` require concrete `BattleInputBuffer` and `ChecksumBuffer` primitives. This phase therefore defines those primitives before handlers are implemented, avoiding the earlier ordering bug.

### Tests

**input/buffer.rs tests:**
- `test_new_buffer_capacity`
- `test_new_buffer_prefilled`
- `test_push_pop_fifo`
- `test_push_full_error`
- `test_pop_empty_none`
- `test_clear_resets`
- `test_wrap_around`
- `test_zero_delay`
- `test_large_delay`

**input/delivery.rs tests:**
- `test_init_buffers`
- `test_setup_input_delay_max`
- `test_setup_input_delay_local_higher`
- `test_setup_input_delay_no_connections`
- `test_receive_input_uses_shared_progress_loop`
- `test_receive_input_disconnect_during_wait`

**checksum/crc.rs tests:**
- `test_crc32_empty`
- `test_crc32_known_vector`
- `test_crc32_init_finalize`
- `test_crc32_of_u32_matches_c_compat_order`
- `test_crc32_incremental`
- `test_crc32_matches_c_fixture`

**checksum/buffer.rs tests:**
- `test_buffer_capacity_formula`
- `test_add_and_get`
- `test_get_missing`
- `test_overwrite_slot`
- `test_clear`

**checksum/verify.rs tests:**
- `test_verify_matching`
- `test_verify_mismatch`
- `test_verify_missing_remote`
- `test_verify_missing_local`
- `test_verification_frame`
- `test_multiple_peers`

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --all-features -- netplay::input
cargo test --all-features -- netplay::checksum
```

## Structural Verification Checklist
- [ ] All 5 files exist (`input/buffer.rs`, `input/delivery.rs`, `checksum/crc.rs`, `checksum/buffer.rs`, `checksum/verify.rs`)
- [ ] Both mod.rs files declare sub-modules
- [ ] Buffer capacity formulas match C code
- [ ] CRC32 uses the compatibility-required polynomial/byte order
- [ ] At least 30 tests defined

## Semantic Verification Checklist (Mandatory)
- [ ] Input buffer pre-fill matches C `initBattleInputBuffers()` behavior
- [ ] FIFO ordering is never violated
- [ ] Buffer full → error, not silent drop
- [ ] CRC32 matches the compatibility target for the same input
- [ ] Checksum verification only triggers desync when BOTH checksums are present
- [ ] Missing remote checksum = "not yet available" = skip (not error)
- [ ] `setup_input_delay()` returns max of all peers' values and local preference
- [ ] `receive_input()` blocks via the shared progress loop, not busy-spinning and not starving flush/dispatch/callback work

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|todo!()\|unimplemented!()\|placeholder" rust/src/netplay/input/ rust/src/netplay/checksum/ | grep -v test
```

## Success Criteria
- [ ] All battle input and checksum tests pass
- [ ] CRC32 validated against known test vectors and compatibility evidence
- [ ] Buffer behavior matches the compatibility target
- [ ] Delivery mechanism handles empty-buffer case correctly

## Failure Recovery
- rollback: `git checkout -- rust/src/netplay/input/ rust/src/netplay/checksum/`
- blocking issues: CRC polynomial/byte-order mismatch, blocking/progress model issues

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P11.md`
