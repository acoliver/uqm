# Phase 05: Parser Implementation

## Phase ID
`PLAN-20260225-AIFF-DECODER.P05`

## Prerequisites
- Required: Phase 04 completed (parser tests exist and compile)
- Expected files: `rust/src/sound/aiff.rs` with failing tests

## Requirements Implemented (Expanded)

### REQ-FP-1 through REQ-FP-15: File Parsing
**Requirement text**: Implement complete AIFF/AIFC file parsing including FORM header, chunk iteration, COMM parsing, SSND parsing, and IEEE 754 80-bit float conversion.

Behavior contract:
- GIVEN: Valid AIFF or AIFC byte data
- WHEN: `open_from_bytes()` is called
- THEN: All header fields are correctly parsed, audio data is extracted into `self.data`, metadata is set, and compression type is determined

### REQ-SV-1 through REQ-SV-13: Sample Format Validation
**Requirement text**: Implement all validation checks with correct error types and messages.

Behavior contract:
- GIVEN: AIFF data with invalid parameters
- WHEN: `open_from_bytes()` is called
- THEN: The correct `DecodeError` variant is returned with a descriptive message, and `self.close()` is called before returning

### REQ-CH-1 through REQ-CH-7: Compression Handling
**Requirement text**: Implement compression type detection and SDX2-specific validation.

Behavior contract:
- GIVEN: AIFF (PCM) or AIFC (SDX2) file
- WHEN: `open_from_bytes()` is called
- THEN: `comp_type` is set correctly, SDX2 endianness override is applied, and invalid compression is rejected

### REQ-LF-7, REQ-LF-8: Open State Management
**Requirement text**: Reset state before parsing and set metadata after successful parse.

### REQ-EH-3: Open Failure Cleanup
**Requirement text**: Call `self.close()` on any failure during `open_from_bytes()`.

Why it matters:
- This is the GREEN phase — making all parser tests pass
- Establishes the foundation for decode and seek phases

## Implementation Tasks

### Files to modify
- `rust/src/sound/aiff.rs`
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P05`
  - Implement: `read_be_u16()`, `read_be_u32()`, `read_be_i16()` — pseudocode lines 20–31
   - Implement: `read_be_f80()` — pseudocode lines 32–93, REQ-FP-14
     - Read all 10 bytes: sign (1 bit), biased exponent (15 bits), significand (64 bits including explicit integer bit)
     - Reconstruct full 64-bit significand: `((sig_hi as u64) << 32) | (sig_lo as u64)`
     - Class 1 (exp=0, sig=0): return 0
     - Class 2 (exp=0, sig!=0): denormalized → return 0 (documented: near-zero, rejected by rate validation)
     - Class 3 (exp=0x7FFF): return Err(InvalidData) for infinity/NaN
     - Class 4 (normal): `value = significand * 2^(biased_exp - 16383 - 63)`, truncate to i32
       - Right shift for typical sample rates (shift < 0): `significand >> (-shift)`
       - Left shift with overflow clamp to i32::MAX (shift >= 0)
       - Apply sign
     - **Must use full 64-bit significand** — do NOT discard low 32 bits
  - Implement: `read_chunk_header()` — pseudocode lines 48–51
  - Implement: `read_common_chunk()` — pseudocode lines 52–68, REQ-FP-8 through REQ-FP-11
  - Implement: `read_sound_data_header()` — pseudocode lines 69–72, REQ-FP-12
  - Implement: `open_from_bytes()` — pseudocode lines 73–238, all FP/SV/CH requirements
  - Implement: `open()` — calls `std::fs::read()` then `open_from_bytes()`
  - Remove `todo!()` from all parsing methods

### Pseudocode traceability
- `read_be_u16/u32/i16`: pseudocode lines 20–31
- `read_be_f80`: pseudocode lines 32–93
- `read_chunk_header`: pseudocode lines 48–51
- `read_common_chunk`: pseudocode lines 52–68
- `read_sound_data_header`: pseudocode lines 69–72
- `open_from_bytes`: pseudocode lines 73–238

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# All tests must pass (GREEN phase)
cargo test --lib --all-features -- aiff

# Quality gates
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] All parsing methods implemented (no `todo!()` remaining in parsing functions)
- [ ] `open_from_bytes()` fully implemented
- [ ] `open()` implemented (file read + open_from_bytes)
- [ ] `use std::io::{Cursor, Read, Seek, SeekFrom}` imports present
- [ ] Plan/requirement traceability markers present

## Semantic Verification Checklist (Mandatory)
- [ ] All parser tests pass (`cargo test -- aiff`)
- [ ] Valid AIFF files parse correctly (frequency, format, length, data length match)
- [ ] All validation errors trigger with correct `DecodeError` variant
- [ ] f80 conversion uses full 64-bit significand (NOT truncated to 32 bits)
- [ ] f80 conversion produces correct integer sample rates for normal values (22050, 44100, 48000, 8000, 11025)
- [ ] f80 denormalized (exp=0, sig!=0) returns 0 (documented design choice)
- [ ] f80 zero (exp=0, sig=0) returns 0
- [ ] f80 infinity/NaN (exp=0x7FFF) returns Err(InvalidData)
- [ ] Odd chunk padding handled correctly
- [ ] Unknown chunks skipped without error
- [ ] Duplicate COMM chunks don't error (later overwrites earlier)
- [ ] `close()` called on every error path in `open_from_bytes()`
- [ ] `last_error` set to `-2` on parsing failures
- [ ] `need_swap` set unconditionally in `open_from_bytes()` for both PCM (`!want_big_endian`) and SDX2 (`big_endian != want_big_endian`) — does NOT rely on `init()` being called first

## Deferred Implementation Detection (Mandatory)

```bash
# Implementation phase: NO todo!/placeholder allowed in parsing functions
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()\|unimplemented!()" src/sound/aiff.rs
# Remaining todo!() should ONLY be in decode() and seek() (not yet implemented)
```

## Success Criteria
- [ ] All parser tests pass
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy` passes
- [ ] No `todo!()` in parsing functions
- [ ] `todo!()` only remains in `decode()` and `seek()`

## Failure Recovery
- rollback steps: `git checkout -- rust/src/sound/aiff.rs`
- blocking issues: If f80 algorithm produces wrong values, debug against known sample rate encodings

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P05.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P05
- timestamp
- files changed: `rust/src/sound/aiff.rs`
- tests added/updated: None (GREEN phase — making existing tests pass)
- verification outputs
- semantic verification summary
