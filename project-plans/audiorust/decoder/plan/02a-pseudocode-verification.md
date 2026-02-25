# Phase 02a: Pseudocode Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P02a`

## Prerequisites
- Required: Phase 02 completed
- Expected files: `analysis/pseudocode/aiff.md`, `analysis/pseudocode/aiff_ffi.md`


## Requirements Implemented (Expanded)

N/A — Verification-only phase. Requirements are verified, not implemented.

## Implementation Tasks

N/A — Verification-only phase. No code changes.
## Verification Commands

```bash
# Numbered lines count
echo "aiff.md lines:"
grep -cE "^[0-9 ]+:" project-plans/audiorust/decoder/analysis/pseudocode/aiff.md

echo "aiff_ffi.md lines:"
grep -cE "^[0-9 ]+:" project-plans/audiorust/decoder/analysis/pseudocode/aiff_ffi.md

# Key algorithm presence
grep -q "wrapping_add" project-plans/audiorust/decoder/analysis/pseudocode/aiff.md && echo "PASS: 8-bit conv" || echo "FAIL"
grep -q "abs" project-plans/audiorust/decoder/analysis/pseudocode/aiff.md && echo "PASS: SDX2 abs" || echo "FAIL"
grep -q "16383" project-plans/audiorust/decoder/analysis/pseudocode/aiff.md && echo "PASS: f80 unbias" || echo "FAIL"
grep -q "0x7FFF" project-plans/audiorust/decoder/analysis/pseudocode/aiff.md && echo "PASS: f80 edge cases" || echo "FAIL"
grep -q "Box::from_raw" project-plans/audiorust/decoder/analysis/pseudocode/aiff_ffi.md && echo "PASS: Box cleanup" || echo "FAIL"
grep -q "init_module" project-plans/audiorust/decoder/analysis/pseudocode/aiff_ffi.md && echo "PASS: init_module in Init" || echo "FAIL"
```

## Structural Verification Checklist
- [ ] `analysis/pseudocode/aiff.md` exists, non-empty, has numbered lines
- [ ] `analysis/pseudocode/aiff_ffi.md` exists, non-empty, has numbered lines
- [ ] `aiff.md` line numbers are sequential and referenceable
- [ ] `aiff.md` covers: new(), byte readers, read_be_f80(), chunk parsing, open_from_bytes(), decode_pcm(), decode_sdx2(), seek(), close(), all trait methods
- [ ] `aiff_ffi.md` covers: all 12 vtable functions, read_uio_file(), vtable static definition

## Semantic Verification Checklist (Mandatory)

### Deterministic Checks
- [ ] REQ-FP-14 (f80 parsing): denormalized case (exponent==0) → returns 0
- [ ] REQ-FP-14 (f80 parsing): infinity/NaN case (exponent==0x7FFF) → returns Err(InvalidData)
- [ ] REQ-FP-14 (f80 parsing): normal case unbias, shift, clamp all present
- [ ] REQ-DS-4 (SDX2 algorithm): `(sample * abs(sample)) << 1`, odd-bit delta, clamp, predictor store — all present
- [ ] REQ-EH-3 (open failure cleanup): `self.close()` called before every Err return in open_from_bytes
- [ ] REQ-CH-7 (SDX2 endianness): `cfg!(target_endian)` logic present in pseudocode
- [ ] REQ-DP-5 (8-bit conversion): `wrapping_add(128)` present in decode_pcm
- [ ] REQ-FF-4 (Init pattern): Init() does Box::new + init_module(0, &formats) + init() + Box::into_raw (matching wav_ffi.rs pattern)
- [ ] Open() does NOT call init_module()/init() — they are already called in Init()

### Subjective Checks
- [ ] Does the f80 parsing algorithm correctly handle the full range of real AIFF sample rates (8000, 11025, 22050, 44100, 48000, 96000)?
- [ ] Does the SDX2 decode algorithm correctly preserve sign through the square operation (negative byte → negative v)?
- [ ] Does the chunk iteration loop correctly handle the relationship between remaining bytes and alignment padding?
- [ ] Does the FFI Open function update ALL required base struct fields (frequency, format, length, is_null, need_swap)?
- [ ] Does the Box lifecycle in Init/Term/Open correctly prevent double-free and use-after-free?

## Deferred Implementation Detection

```bash
# N/A for pseudocode phase — no implementation code
echo "Pseudocode verification phase: no deferred implementation detection needed"
```

## Success Criteria
- [ ] Both pseudocode files exist with numbered algorithmic steps
- [ ] All 9 requirement categories have traceable pseudocode
- [ ] f80 edge cases (denormalized, infinity/NaN) are explicitly handled
- [ ] FFI Init matches wav_ffi.rs pattern (propagates formats via init_module/init)
- [ ] SDX2 algorithm is mathematically correct
- [ ] Validation points, error handling, side effects are explicit

## Failure Recovery
- Return to Phase 02 and address specific gaps
- If f80 edge cases missing, add them to pseudocode
- If FFI Init doesn't propagate formats, fix to match wav_ffi.rs pattern

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P02a.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P02a
- timestamp
- verification result: PASS/FAIL
- gaps identified (if any)
- gate decision: proceed to P03 or return to P02
