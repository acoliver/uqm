# P15: Netplay Integration Types & CRC Implementation Summary

**Plan:** PLAN-20260320-BATTLE.P15  
**Date:** 2026-03-20  
**Status:** [OK] Complete

## Overview

Implemented the CRC-32 checksum system and netplay synchronization types for the UQM battle engine Rust port. The implementation is bit-identical to the C reference code for netplay determinism.

## Files Created

### `rust/src/battle/netplay.rs` (new)
- CRC-32 core implementation
- Element checksum processing
- Netplay type definitions
- Comprehensive test coverage (19 tests)

## Implementation Details

### 1. CRC-32 Core (Bit-Identical to C)

**`CrcState` struct:**
- Matches C's `crc_State` from `crc.h`
- Accumulates CRC value with `u32` state

**CRC-32 table:**
- 256-entry lookup table using polynomial 0x04c11db7 (IEEE 802.3)
- Bit-identical to C's `crcTable` from `crc.c`
- Test verification: `CRC_TABLE[0..5]` match C exactly

**Core operations:**
- `new()` / `init()` — Initialize to 0xFFFFFFFF
- `update(byte: u8)` — Process single byte using table lookup
- `finish() -> u32` — Finalize with XOR (returns `!crc`)
- `process_u16(val: u16)` — Little-endian u16 processing
- `process_u32(val: u32)` — Little-endian u32 processing
- `process_i16(val: i16)` — Signed 16-bit (reinterpreted as u16)

### 2. Structured Type Processing

**Extent (width, height):**
- `process_extent(&Extent)` — Matches C's `crc_processEXTENT()`

**Point (x, y):**
- `process_point(&Point)` — Matches C's `crc_processPOINT()`

**VelocityDesc (travel angle, vector, fract, error, incr):**
- `process_velocity_desc(&VelocityDesc)` — Matches C's `crc_processVELOCITY_DESC()`

**Element state:**
- `process_element_state(&Point)` — Matches C's `crc_processSTATE()` (location only)

### 3. Element CRC Processing

**`process_element(&Element)`** — Matches C's `crc_processELEMENT()` exactly:

**Field order (CRITICAL for netplay determinism):**
1. `state_flags` (u16)
2. `life_span` (u16)
3. `crew_or_hp` (u16)
4. `mass_points` (u8)
5. `turn_wait` (u8)
6. `thrust_or_blast` (u8)
7. `velocity` (VelocityDesc — 5 Extents)
8. `current.location` (Point)
9. `next.location` (Point)

**BACKGROUND_OBJECT exclusion:**
- Elements with `BACKGROUND_OBJECT` flag are skipped entirely (don't contribute to CRC)
- Test verification: background elements produce unchanged CRC

### 4. Netplay Types (Type-Only Definitions)

**`BattleFrameCounter`** — `u32` frame counter for netplay synchronization

**`Checksum`** — `u32` CRC-32 checksum value

**`BattleEndPhase` enum** — 4-phase battle termination handshake:
- `Ongoing = 0` — Battle active
- `LocalEnd = 1` — Local side detects end
- `RemoteAck = 2` — Remote acknowledges
- `Synchronized = 3` — Both sides ready for cleanup

**`input_buffer` module:**
- `InputBufferEntry` — Frame + command pair
- `InputBuffer` — Per-player input buffer with capacity management

**`frame_sync` module:**
- `FrameSyncState` — Frame synchronization state
- Tracks local/remote checksums
- `verify()` method for checksum comparison

## Test Coverage (19 Tests)

### CRC-32 Core Tests
1. [OK] `crc_table_first_entries_match_c` — Table[0..5] match C values
2. [OK] `crc_table_complete` — 256 entries, last entry verified
3. [OK] `crc_init_sets_initial_value` — 0xFFFFFFFF initialization
4. [OK] `crc_finish_inverts_state` — XOR with 0xFFFFFFFF
5. [OK] `crc_update_single_byte` — Single byte processing verified
6. [OK] `crc_of_empty_data` — Empty CRC = 0x00000000
7. [OK] `crc_of_known_sequence` — "123456789" = 0xCBF43926 (standard test vector)

### Typed Processing Tests
8. [OK] `crc_process_u16_little_endian` — Low byte first, then high byte
9. [OK] `crc_process_i16_reinterprets_as_u16` — Signed→unsigned reinterpretation
10. [OK] `crc_process_extent` — Width, height in order
11. [OK] `crc_process_point` — X, Y in order
12. [OK] `crc_process_velocity_desc` — All 5 Extents + angle

### Element Processing Tests
13. [OK] `crc_process_element_field_order` — Exact field order verification
14. [OK] `crc_process_element_background_object_excluded` — BACKGROUND_OBJECT skipped
15. [OK] `crc_process_element_normal_vs_background` — Background produces unchanged CRC

### Netplay Types Tests
16. [OK] `battle_end_phase_enum_values` — Enum discriminants 0-3
17. [OK] `input_buffer_push_and_get` — Buffer operations
18. [OK] `input_buffer_full` — Capacity limit enforcement
19. [OK] `frame_sync_state_verify` — Checksum matching/mismatching

## Verification Results

**Compilation:** [OK] Clean (no errors, warnings expected from other modules)

**Test Results:** [OK] All 19 netplay tests pass  
**Total Project Tests:** 2096 passed, 0 failed

**Code Formatting:** [OK] `cargo fmt --all` applied

## Key Design Decisions

### 1. Bit-Identical CRC Implementation
- CRC table copied exactly from C (0x04c11db7 polynomial)
- Little-endian byte order enforced for all multi-byte types
- Test with known vector ("123456789") confirms correctness

### 2. BACKGROUND_OBJECT Exclusion
- Matches C behavior: `if (val->state_flags & BACKGROUND_OBJECT) { return; }`
- Background elements (planet graphics, etc.) don't affect battle state
- Early-exit prevents any CRC contribution

### 3. Field Order Enforcement
- Element CRC processes fields in exact C order
- Test `crc_process_element_field_order` verifies against manual computation
- Critical for netplay determinism across Rust/C boundaries

### 4. Type Safety
- `Checksum` and `BattleFrameCounter` are type aliases (not newtypes)
- Matches C's `typedef uint32 Checksum;`
- Simple and FFI-compatible

### 5. Future-Proof Netplay Types
- Input buffer and frame sync are placeholder implementations
- Basic functionality (push/get/verify) tested
- Ready for Phase 2 netplay integration

## References

**C Source Files Read:**
- `sc2/src/uqm/supermelee/netplay/crc.c` — CRC-32 table and core
- `sc2/src/uqm/supermelee/netplay/crc.h` — CRC types
- `sc2/src/uqm/supermelee/netplay/checksum.c` — Element processing
- `sc2/src/uqm/supermelee/netplay/checksum.h` — Function declarations
- `project-plans/20260311/battle/plan/02-pseudocode.md` — Section 4 (CRC)

**Rust Dependencies:**
- `rust/src/battle/element.rs` — Element, ElementFlags, Point
- `rust/src/battle/velocity.rs` — VelocityDesc, Extent

## Next Steps (Future Phases)

1. **Display queue CRC processing** (`crc_processDispQueue`)
2. **RNG state CRC processing** (`crc_processRNG`)
3. **Checksum buffer management** (circular buffer, delay accounting)
4. **Network protocol integration** (send/receive checksums)
5. **Desync detection and recovery**

## Notes

- **No FFI calls yet** — Pure Rust implementation, ready for FFI integration
- **TDD approach** — All tests written and passing before merge
- **Zero test failures** — 2096 total tests, 100% pass rate
- **C parity verified** — CRC test vector matches C implementation

---

**Completion Criteria Met:**
[OK] CRC-32 table matches C  
[OK] CRC-32 core operations (init, update, finish)  
[OK] Element processing with BACKGROUND_OBJECT exclusion  
[OK] Netplay type definitions  
[OK] 19 comprehensive tests  
[OK] `cargo test --lib` passes (2096 tests)  
[OK] `cargo fmt --all` applied  

**Test Count:** 2077 existing + 19 new = **2096 total** [OK]
