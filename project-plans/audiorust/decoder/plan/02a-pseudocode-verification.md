# Phase 02a: Pseudocode Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P02a`

## Prerequisites
- Required: Phase 02 completed
- Expected files: `analysis/pseudocode/aiff.md`, `analysis/pseudocode/aiff_ffi.md`

## Verification Checklist

### Structural
- [ ] `analysis/pseudocode/aiff.md` exists, non-empty, has numbered lines
- [ ] `analysis/pseudocode/aiff_ffi.md` exists, non-empty, has numbered lines
- [ ] `aiff.md` line numbers are sequential and referenceable

### Coverage — aiff.md
- [ ] Constructor (`new()`) — pseudocode lines exist
- [ ] Byte readers (`read_be_u16`, `read_be_u32`, `read_be_i16`) — pseudocode lines exist
- [ ] IEEE 754 80-bit float (`read_be_f80`) — 12-step algorithm per FP-14
- [ ] Chunk header parsing — pseudocode lines exist
- [ ] Common chunk parsing — handles min size, extended fields, extra data skip
- [ ] Sound data chunk parsing — offset/block_size + data_start calculation
- [ ] `open_from_bytes()` — full flow with all 8+ validation checks
- [ ] `decode_pcm()` — frame count, data copy, 8-bit conversion, position update
- [ ] `decode_sdx2()` — frame count, compressed data read, SDX2 algorithm, channel iteration
- [ ] `seek()` — clamp, position update, predictor reset
- [ ] `close()` — data clear, position reset
- [ ] All 15 trait methods — pseudocode present

### Coverage — aiff_ffi.md
- [ ] Module-level state (Mutex, name string) — pseudocode lines exist
- [ ] `read_uio_file()` — open/fstat/read loop/close sequence
- [ ] All 12 vtable functions — pseudocode present
- [ ] Null pointer checks in every function
- [ ] Box lifecycle: `Box::new()` in Init, `Box::from_raw()` in Term
- [ ] Format mapping in Open
- [ ] Decode return value mapping (Ok→n, EndOfFile→0, Err→0)

### Semantic
- [ ] REQ-FP-14 (f80 parsing): All 12 steps from spec are in pseudocode
- [ ] REQ-DS-4 (SDX2 algorithm): Square-with-sign, odd-bit delta, clamp, predictor — all present
- [ ] REQ-EH-3 (open failure cleanup): `self.close()` called before every Err return in open_from_bytes
- [ ] REQ-CH-7 (SDX2 endianness): `cfg!(target_endian)` logic present in pseudocode
- [ ] REQ-DP-5 (8-bit conversion): `wrapping_add(128)` present in decode_pcm

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
grep -q "Box::from_raw" project-plans/audiorust/decoder/analysis/pseudocode/aiff_ffi.md && echo "PASS: Box cleanup" || echo "FAIL"
```

## Gate Decision
- [ ] PASS: proceed to Phase 03
- [ ] FAIL: return to Phase 02 and address gaps
