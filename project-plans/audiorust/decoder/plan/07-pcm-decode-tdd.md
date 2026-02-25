# Phase 07: PCM Decode TDD

## Phase ID
`PLAN-20260225-AIFF-DECODER.P07`

## Prerequisites
- Required: Phase 06 completed (decode stubs exist)
- Expected files: `rust/src/sound/aiff.rs` with `decode_pcm()` stub

## Requirements Implemented (Expanded)

### REQ-DP-1: PCM Frame Count
**Requirement text**: Calculate frames as `min(bufsize / block_align, max_pcm - cur_pcm)`.

Behavior contract:
- GIVEN: A decoder opened with a 100-frame mono16 AIFF (block_align=2), buffer size=50 bytes
- WHEN: `decode()` is called
- THEN: It decodes 25 frames (50/2), returns Ok(50)

### REQ-DP-2: PCM Data Read (No Inline Byte Swap)
**Requirement text**: Copy `dec_pcm * file_block` bytes from self.data to output. Do NOT perform inline byte swapping — the C framework handles it.

Behavior contract:
- GIVEN: A decoder with known PCM data [0x00, 0x01, 0x02, 0x03]
- WHEN: `decode()` is called with sufficient buffer
- THEN: Output buffer contains the exact same bytes (raw big-endian, no swap)

### REQ-DP-3: PCM Position Update
**Requirement text**: Advance cur_pcm and data_pos after decode.

Behavior contract:
- GIVEN: Decoder at cur_pcm=0, decode 10 frames
- WHEN: decode completes
- THEN: cur_pcm=10, data_pos=10*file_block

### REQ-DP-4: PCM Return Value
**Requirement text**: Return `Ok(dec_pcm * block_align)` bytes written.

Behavior contract:
- GIVEN: 10 frames decoded, block_align=4 (stereo 16-bit)
- WHEN: decode returns
- THEN: Returns Ok(40)

### REQ-DP-5: 8-bit Signed-to-Unsigned Conversion
**Requirement text**: Apply `wrapping_add(128)` to every byte when bits_per_sample==8.

Behavior contract:
- GIVEN: An 8-bit AIFF file with signed sample data [-128, -1, 0, 127] (stored as [0x80, 0xFF, 0x00, 0x7F])
- WHEN: `decode()` is called
- THEN: Output contains [0, 127, 128, 255]

### REQ-DP-6: PCM EOF
**Requirement text**: Return Err(EndOfFile) when cur_pcm >= max_pcm.

Behavior contract:
- GIVEN: A decoder that has already decoded all frames
- WHEN: `decode()` is called again
- THEN: Returns `Err(DecodeError::EndOfFile)`

Why it matters:
- Tests define PCM decode contract completely
- Verifies correct byte copying, format conversion, and position tracking

## Implementation Tasks

### Files to modify
- `rust/src/sound/aiff.rs` — Add PCM decode tests
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P07`
  - marker: `@requirement REQ-DP-1, REQ-DP-2, REQ-DP-3, REQ-DP-4, REQ-DP-5, REQ-DP-6`

### Test Cases to Write

1. `test_decode_pcm_mono16` — Mono 16-bit: decode known data, verify output matches
2. `test_decode_pcm_stereo16` — Stereo 16-bit: verify interleaved channel data
3. `test_decode_pcm_mono8_signed_to_unsigned` — Mono 8-bit: verify wrapping_add(128) conversion
4. `test_decode_pcm_stereo8_signed_to_unsigned` — Stereo 8-bit: same conversion, 2 channels
5. `test_decode_pcm_partial_buffer` — Buffer smaller than full data → partial decode
6. `test_decode_pcm_multiple_calls` — Two sequential decodes, second starts where first ended
7. `test_decode_pcm_eof` — Decode past end → Err(EndOfFile)
8. `test_decode_pcm_exact_fit` — Buffer exactly fits remaining data
9. `test_decode_pcm_returns_byte_count` — Verify return value = dec_pcm * block_align
10. `test_decode_pcm_position_update` — After decode, cur_pcm and data_pos advanced
11. `test_decode_pcm_16bit_no_inline_swap` — 16-bit PCM does NOT swap bytes inline:
    - GIVEN: AIFF with 16-bit big-endian sample data [0x03, 0xE8] (1000 as big-endian i16)
    - AND: need_swap is true (mixer wants little-endian)
    - WHEN: decode() is called
    - THEN: output buffer contains [0x03, 0xE8] (bytes unchanged — raw big-endian)
    - NOTE: The C framework's SoundDecoder_Decode() handles byte swapping via need_swap field
12. `test_decode_pcm_16bit_raw_bytes_preserved` — 16-bit PCM with need_swap=false:
    - GIVEN: AIFF with 16-bit big-endian sample data [0x03, 0xE8] (1000 as big-endian i16)
    - AND: need_swap is false (mixer wants big-endian)
    - WHEN: decode() is called
    - THEN: output buffer contains [0x03, 0xE8] (bytes unchanged from file)
13. `test_decode_pcm_16bit_stereo_raw_bytes` — Stereo 16-bit raw bytes preserved:
    - GIVEN: Stereo AIFF with big-endian data [0x00, 0x01, 0x00, 0x02] (L=1, R=2)
    - AND: need_swap is true (C framework will swap, not the decoder)
    - WHEN: decode() is called
    - THEN: output contains [0x00, 0x01, 0x00, 0x02] (raw big-endian, no inline swap)
14. `test_decode_pcm_8bit_no_endian_swap` — 8-bit PCM ignores need_swap:
    - GIVEN: 8-bit AIFF with need_swap=true
    - WHEN: decode() is called
    - THEN: only signed→unsigned conversion applied, no byte swapping
15. `test_decode_pcm_zero_length_buffer` — Zero-length output buffer:
    - GIVEN: A valid opened decoder with data
    - WHEN: decode() is called with an empty (zero-length) buffer
    - THEN: Returns Ok(0) without advancing position
16. `test_need_swap_set_correctly_for_16bit` — Verify need_swap is set on 16-bit AIFF:
    - GIVEN: A 16-bit big-endian AIFF file opened on a little-endian host
    - WHEN: open_from_bytes() completes
    - THEN: `self.formats.need_swap` is true (the C framework reads this to decide byte-swapping)
    - AND: `self.formats.big_endian` is true (AIFF is always big-endian)
    - NOTE: This test verifies the _contract_ between the Rust decoder and the C framework.
      The decoder sets need_swap; the framework's SoundDecoder_Decode() reads it.

### Pseudocode traceability
- Tests cover pseudocode lines: 239–267 (decode_pcm, no inline byte swap)

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Tests compile but fail (RED)
cargo test --lib --all-features -- aiff::tests::test_decode_pcm --no-run

# Format and lint
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] At least 15 PCM decode test functions defined
- [ ] Tests compile (`--no-run`)
- [ ] Tests use the `build_aiff_file()` helper to create test data
- [ ] Tests verify output buffer contents, not just return values

## Semantic Verification Checklist (Mandatory)
- [ ] 8-bit conversion tests verify actual byte values after wrapping_add(128)
- [ ] 16-bit output tests verify raw big-endian bytes pass through unchanged (NO inline swap)
- [ ] 16-bit with need_swap=true test verifies bytes are still raw big-endian (C framework handles swap)
- [ ] Stereo 16-bit test verifies raw bytes preserved (no swap regardless of need_swap)
- [ ] 8-bit endian test verifies need_swap has no effect on 8-bit output
- [ ] Zero-length buffer test verifies Ok(0) without advancing position
- [ ] Position tracking tests check both cur_pcm and data_pos
- [ ] EOF test verifies the exact `DecodeError::EndOfFile` variant
- [ ] Partial decode test verifies correct frame count calculation
- [ ] Multi-call test verifies continuation from previous position

## Deferred Implementation Detection (Mandatory)

```bash
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()" src/sound/aiff.rs
# decode_pcm and decode_sdx2 should still have todo!()
```

## Success Criteria
- [ ] All test functions compile
- [ ] Tests would fail when run (RED phase)
- [ ] All REQ-DP-* requirements covered

## Failure Recovery
- rollback steps: `git checkout -- rust/src/sound/aiff.rs`
- blocking issues: If test data construction is complex, simplify synthetic AIFF files

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P07.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P07
- timestamp
- files changed: `rust/src/sound/aiff.rs` (tests added)
- tests added: ~15 PCM decode tests
- verification outputs
- semantic verification summary
