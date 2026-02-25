# Phase 04: Parser TDD

## Phase ID
`PLAN-20260225-AIFF-DECODER.P04`

## Prerequisites
- Required: Phase 03 completed (parser stub compiles)
- Expected files: `rust/src/sound/aiff.rs` with stub methods

## Requirements Implemented (Expanded)

### REQ-FP-1: File Header Validation
**Requirement text**: Read first 12 bytes as big-endian FORM file header.

Behavior contract:
- GIVEN: A byte array starting with valid FORM/AIFF header bytes
- WHEN: `open_from_bytes()` is called
- THEN: The decoder parses chunk_id, chunk_size, and form_type correctly

### REQ-FP-2: FORM ID Check
**Requirement text**: Reject non-FORM chunk ID with InvalidData.

Behavior contract:
- GIVEN: A byte array where first 4 bytes are NOT `0x464F524D`
- WHEN: `open_from_bytes()` is called
- THEN: Returns `Err(DecodeError::InvalidData(...))` containing the invalid ID

### REQ-FP-3: Form Type Check
**Requirement text**: Reject non-AIFF/AIFC form types.

Behavior contract:
- GIVEN: A byte array with valid FORM ID but form type is neither AIFF nor AIFC
- WHEN: `open_from_bytes()` is called
- THEN: Returns `Err(DecodeError::InvalidData(...))`

### REQ-FP-8: Common Chunk Parsing
**Requirement text**: Parse COMM chunk fields.

Behavior contract:
- GIVEN: A valid AIFF file with COMM chunk containing channels=1, sample_frames=100, sample_size=16, sample_rate=44100
- WHEN: `open_from_bytes()` is called
- THEN: Decoder state reflects parsed values

### REQ-FP-9: Common Chunk Minimum Size
**Requirement text**: Reject COMM chunks smaller than 18 bytes.

Behavior contract:
- GIVEN: An AIFF file with a COMM chunk whose size field is less than 18
- WHEN: `open_from_bytes()` is called
- THEN: Returns `Err(DecodeError::InvalidData("COMM chunk too small"))` and sets last_error to -2

### REQ-FP-14: IEEE 754 80-bit Float Conversion
**Requirement text**: Convert 10-byte 80-bit float to i32 sample rate.

Behavior contract:
- GIVEN: Known 80-bit float encodings for sample rates (44100, 22050, 8000, 48000, 96000, 11025)
- WHEN: `read_be_f80()` is called
- THEN: Returns the correct integer sample rate

### REQ-SV-1 through REQ-SV-6: Validation Tests
**Requirement text**: Each validation check rejects invalid input with the correct error.

Behavior contracts:
- GIVEN: bits_per_sample rounds to 0 or >16 → UnsupportedFormat
- GIVEN: channels is 3 → UnsupportedFormat
- GIVEN: sample_rate is 200 → UnsupportedFormat
- GIVEN: sample_frames is 0 → InvalidData
- GIVEN: No SSND chunk → InvalidData

### REQ-CH-1 through REQ-CH-4: Compression Type Detection
**Requirement text**: Correctly identify PCM vs SDX2 based on form_type and ext_type_id.

Behavior contracts:
- GIVEN: AIFF file (not AIFC) with ext_type_id=0 → CompressionType::None
- GIVEN: AIFF file with ext_type_id!=0 → UnsupportedFormat
- GIVEN: AIFC file with ext_type_id=SDX2 → CompressionType::Sdx2
- GIVEN: AIFC file with ext_type_id=unknown → UnsupportedFormat

Why it matters:
- Tests define the contract before implementation
- Failing tests drive the parser implementation in Phase 05

## Implementation Tasks

### Files to modify
- `rust/src/sound/aiff.rs` — Add `#[cfg(test)] mod tests` with parsing test cases
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P04`
  - marker: `@requirement REQ-FP-1, REQ-FP-2, REQ-FP-3, REQ-FP-8, REQ-FP-9, REQ-FP-14`
  - marker: `@requirement REQ-SV-1, REQ-SV-2, REQ-SV-3, REQ-SV-4, REQ-SV-5, REQ-SV-6`
  - marker: `@requirement REQ-CH-1, REQ-CH-2, REQ-CH-3, REQ-CH-4`

### Test Cases to Write

**Helper**: Create a `build_aiff_file()` test helper that constructs synthetic AIFF byte arrays with configurable COMM and SSND chunks.

1. `test_parse_valid_aiff_mono16` — Valid mono 16-bit PCM AIFF, verify frequency/format/length
2. `test_parse_valid_aiff_stereo16` — Valid stereo 16-bit PCM AIFF
3. `test_parse_valid_aiff_mono8` — Valid mono 8-bit PCM AIFF
4. `test_parse_valid_aiff_stereo8` — Valid stereo 8-bit PCM AIFF
5. `test_parse_valid_aifc_sdx2` — Valid AIFC with SDX2 compression
6. `test_reject_non_form_header` — Non-FORM chunk ID → InvalidData
7. `test_reject_non_aiff_form_type` — Valid FORM but wrong form type → InvalidData
8. `test_reject_small_comm_chunk` — COMM size < 18 → InvalidData, last_error=-2
9. `test_reject_zero_sample_frames` — sample_frames=0 → InvalidData
10. `test_reject_no_ssnd_chunk` — File with COMM but no SSND → InvalidData
11. `test_reject_unsupported_bits_per_sample` — bits>16 → UnsupportedFormat
12. `test_reject_unsupported_channels` — channels=3 → UnsupportedFormat
13. `test_reject_sample_rate_too_low` — rate=200 → UnsupportedFormat
14. `test_reject_sample_rate_too_high` — rate=200000 → UnsupportedFormat
15. `test_reject_aiff_with_extension` — AIFF + ext_type_id!=0 → UnsupportedFormat
16. `test_reject_aifc_unknown_compression` — AIFC + non-SDX2 → UnsupportedFormat
17. `test_f80_known_rates` — 44100, 22050, 8000, 48000, 96000, 11025 round-trips
18. `test_f80_denormalized_returns_zero` — Exponent==0 (denormalized) → read_be_f80 returns Ok(0)
19. `test_f80_infinity_returns_error` — Exponent==0x7FFF with mantissa!=0 or ==0 (infinity/NaN) → Err(InvalidData)
20. `test_chunk_alignment_padding` — Odd-sized chunk followed by another chunk
21. `test_unknown_chunk_skipped` — Unknown chunk ID is skipped, parsing continues
22. `test_duplicate_comm_chunk` — Later COMM overwrites earlier (no error)
23. `test_open_sets_metadata` — After successful open: frequency, format, length, max_pcm are correct
24. `test_sdx2_requires_16bit` — SDX2 with 8-bit → UnsupportedFormat
25. `test_sdx2_channel_limit` — SDX2 with >4 channels → UnsupportedFormat

### Pseudocode traceability
- Tests cover pseudocode lines: 73–224 (open_from_bytes), 32–47 (read_be_f80), 48–68 (chunk parsing)

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Tests should FAIL (RED phase) — stubs return todo!()
# But tests should compile
cargo test --lib --all-features -- aiff --no-run

# Format and lint
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] Test module exists in `rust/src/sound/aiff.rs`
- [ ] At least 22 test functions defined
- [ ] Test helper `build_aiff_file()` creates synthetic AIFF byte arrays
- [ ] Tests compile (`--no-run`)
- [ ] Plan/requirement traceability in test comments

## Semantic Verification Checklist (Mandatory)
- [ ] Tests verify behavior, not implementation internals
- [ ] Each error path test checks the specific `DecodeError` variant
- [ ] Valid-file tests check output state (frequency, format, length, max_pcm)
- [ ] f80 tests check known sample rate values (44100, 22050, 8000, 48000, 96000, 11025)
- [ ] f80 denormalized (exponent==0) test verifies result is Ok(0)
- [ ] f80 infinity/NaN (exponent==0x7FFF) test verifies Err(InvalidData)
- [ ] Edge case tests (odd alignment, unknown chunks, duplicate COMM) present
- [ ] No tests that would pass with a trivial/fake implementation

## Deferred Implementation Detection (Mandatory)

```bash
# TDD phase: implementation is still todo!(), which is expected
# Verify no fake implementations snuck in:
cd /Users/acoliver/projects/uqm/rust && grep -c "todo!()" src/sound/aiff.rs
```

## Success Criteria
- [ ] All test functions compile
- [ ] Tests would fail when run (RED phase — stubs panic)
- [ ] All requirement categories covered by tests
- [ ] Test helper produces valid synthetic AIFF byte arrays

## Failure Recovery
- rollback steps: `git checkout -- rust/src/sound/aiff.rs`
- blocking issues: If synthetic byte array construction is wrong, fix the helper first

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P04.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P04
- timestamp
- files changed: `rust/src/sound/aiff.rs` (tests added)
- tests added: ~25 parser tests
- verification outputs
- semantic verification summary
