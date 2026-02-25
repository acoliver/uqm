# Phase 10: SDX2 Decode TDD

## Phase ID
`PLAN-20260225-AIFF-DECODER.P10`

## Prerequisites
- Required: Phase 09 completed (SDX2 decode stub confirmed)
- Expected files: `rust/src/sound/aiff.rs` with `decode_sdx2()` stub

## Requirements Implemented (Expanded)

### REQ-DS-1: SDX2 Frame Count
**Requirement text**: Calculate frames as `min(bufsize / block_align, max_pcm - cur_pcm)`.

Behavior contract:
- GIVEN: An AIFC/SDX2 file with 100 frames, block_align=4 (stereo 16-bit output), buf=80 bytes
- WHEN: `decode()` is called
- THEN: Decodes 20 frames (80/4), returns Ok(80)

### REQ-DS-4: SDX2 Decode Algorithm
**Requirement text**: Apply `v = (sample * abs(sample)) << 1`, odd-bit delta, clamp, predictor store.

Behavior contract:
- GIVEN: Compressed byte 0x10 (decimal 16, even → no delta)
- WHEN: Decoded with prev_val[ch]=0
- THEN: v = (16 * 16) << 1 = 512, clamped, prev_val[ch]=512, output = 512 as i16

- GIVEN: Compressed byte 0x11 (decimal 17, odd → delta mode)
- WHEN: Decoded with prev_val[ch]=512
- THEN: v = (17 * 17) << 1 + 512 = 578 + 512 = 1090, clamped, prev_val[ch]=1090

### REQ-DS-5: SDX2 Channel Iteration
**Requirement text**: Channels interleaved within each frame.

Behavior contract:
- GIVEN: Stereo AIFC/SDX2 file, compressed data [ch0_byte, ch1_byte, ch0_byte, ch1_byte, ...]
- WHEN: `decode()` is called
- THEN: Output has interleaved [ch0_i16, ch1_i16, ch0_i16, ch1_i16, ...]

### REQ-DS-7: SDX2 Predictor Initialization
**Requirement text**: `prev_val` initialized to `[0; MAX_CHANNELS]` at open.

Behavior contract:
- GIVEN: A freshly opened AIFC/SDX2 decoder
- WHEN: First decode is called
- THEN: All predictor values start at 0

### REQ-DS-8: SDX2 EOF
**Requirement text**: Return Err(EndOfFile) when cur_pcm >= max_pcm.

Behavior contract:
- GIVEN: All SDX2 frames already decoded
- WHEN: `decode()` is called again
- THEN: Returns `Err(DecodeError::EndOfFile)`

Why it matters:
- SDX2 is the complex decode path with predictor state
- Tests must verify the mathematical algorithm against known values

## Implementation Tasks

### Files to modify
- `rust/src/sound/aiff.rs` — Add SDX2 decode tests
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P10`
  - marker: `@requirement REQ-DS-1, REQ-DS-2, REQ-DS-3, REQ-DS-4, REQ-DS-5, REQ-DS-6, REQ-DS-7, REQ-DS-8`

### Test Cases to Write

**Helper**: Create a `build_aifc_sdx2_file()` test helper that constructs synthetic AIFC/SDX2 byte arrays.

1. `test_decode_sdx2_mono_single_frame` — Single mono frame with known byte, verify exact i16 output
2. `test_decode_sdx2_mono_even_no_delta` — Even byte (no delta): v = (s * |s|) << 1
3. `test_decode_sdx2_mono_odd_with_delta` — Odd byte (delta mode): v = (s * |s|) << 1 + prev_val
4. `test_decode_sdx2_negative_sample` — Negative compressed byte, verify sign preservation
5. `test_decode_sdx2_stereo_interleaved` — Stereo file, verify per-channel predictor independence
6. `test_decode_sdx2_predictor_accumulation` — Multiple frames, verify predictor state builds up
7. `test_decode_sdx2_clamp_positive` — Value exceeding 32767 → clamped
8. `test_decode_sdx2_clamp_negative` — Value below -32768 → clamped
9. `test_decode_sdx2_partial_buffer` — Buffer smaller than total → partial decode
10. `test_decode_sdx2_eof` — All frames decoded → Err(EndOfFile)
11. `test_decode_sdx2_returns_byte_count` — Return value = dec_pcm * block_align
12. `test_decode_sdx2_position_update` — cur_pcm and data_pos advance correctly
13. `test_decode_sdx2_zero_byte` — Compressed byte 0x00: v = 0, prev_val stays at delta or 0

### Pseudocode traceability
- Tests cover pseudocode lines: 270–315 (decode_sdx2)

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Tests compile but fail (RED)
cargo test --lib --all-features -- aiff::tests::test_decode_sdx2 --no-run

# Format and lint
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] At least 12 SDX2 decode test functions defined
- [ ] Tests compile (`--no-run`)
- [ ] `build_aifc_sdx2_file()` helper creates valid AIFC/SDX2 data
- [ ] Tests verify exact output values (not just success/failure)

## Golden Test Vectors (C Reference Parity)

> **Note:** The golden vectors described below are **procedural** — they will be extracted
> from the C decoder during implementation (Phase 11), not provided inline in the plan.
> The hand-calculated test vectors in the test cases above (e.g., 0x10 → 512) provide
> baseline correctness assurance, but the golden vectors from actual C decoder output are
> the definitive parity test. If extraction from C proves impractical, use a known AIFC
> file and compare output byte-for-byte between C and Rust at runtime.

Implementation tests **must** include golden test vectors extracted from the C decoder
(`aiffaud.c`) output to prove exact parity. Steps to produce golden vectors:

1. Build UQM with the C AIFF decoder and add a debug hook in `aiffaud.c` to dump
   the compressed input bytes and the decoded i16 output samples for a known AIFC/SDX2
   file (e.g., a short UQM sound effect).
2. Capture at least 3 sequences:
   - A pure non-delta sequence (several even bytes in a row)
   - A pure delta sequence (several odd bytes in a row)
   - A mixed sequence with channel interleaving (stereo)
3. Encode these as `const` byte arrays in the test module, alongside the expected i16
   output arrays from the C decoder.
4. Assert that the Rust `decode_sdx2()` output matches the C output **exactly** (byte-for-byte).

This is the single strongest test for correctness — it catches sign errors, off-by-one
in the shift, incorrect delta accumulation, and endianness bugs that unit tests with
hand-calculated values might miss.

## Semantic Verification Checklist (Mandatory)
- [ ] Even/odd byte distinction tested (delta vs non-delta mode)
- [ ] Negative sample handling tested (sign-preserving square)
- [ ] Per-channel predictor independence tested (stereo)
- [ ] Clamping tested for both overflow and underflow
- [ ] Predictor accumulation tested across multiple frames
- [ ] Golden test vectors from C decoder output included (proving exact parity)
- [ ] Tests would not pass with a trivial implementation

## Deferred Implementation Detection (Mandatory)

```bash
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()" src/sound/aiff.rs
# decode_sdx2 and seek should still have todo!()
```

## Success Criteria
- [ ] All test functions compile
- [ ] Tests would fail when run (RED phase)
- [ ] All REQ-DS-* requirements covered

## Failure Recovery
- rollback steps: `git checkout -- rust/src/sound/aiff.rs`
- blocking issues: If SDX2 expected values are wrong, hand-calculate from algorithm

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P10.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P10
- timestamp
- files changed: `rust/src/sound/aiff.rs` (tests added)
- tests added: ~13 SDX2 decode tests
- verification outputs
- semantic verification summary
