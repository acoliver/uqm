# AIFF Decoder Plan — Technical Correctness Review

Scope reviewed:
- `specification.md`
- `plan/00-overview.md`
- `analysis/domain-model.md`
- `analysis/pseudocode/aiff.md`
- `analysis/pseudocode/aiff_ffi.md`
- `plan/04-parser-tdd.md`
- `plan/05-parser-impl.md`
- `plan/08-pcm-decode-impl.md`
- `plan/11-sdx2-decode-impl.md`
- `plan/14-seek-impl.md`
- `plan/17-ffi-impl.md`
- `plan/18-integration.md`
- `../rust-decoder.md`

Date: 2026-02-25
Reviewer: LLxprt Code

---

## Executive Summary

Overall assessment: **Mostly strong and implementable**, with good structure and traceability. The plan is near production-ready, but there are **several technical correctness issues that should be fixed before implementation**.

Key outcomes:
- Requirement coverage: **84/84 requirements are assigned somewhere in the plan.**
- Major algorithm risk: **IEEE 754 80-bit parsing pseudocode is not technically correct for normalized values in general** (works accidentally for common rates, but math model is wrong/incomplete).
- SDX2: **Core decode formula and predictor handling are directionally correct**, including per-channel predictor and saturation. One critical detail should be clarified/verified: output byte-order write path consistency with trait and C expectations.
- Integration plan: **complete in intent** (flag, decoder.c switch, header, config/build vars, replacement path).
- Verification quality: present and generally good; at least 3 phases include real behavioral checks.

Recommendation: **Proceed after fixing the f80 algorithm spec + a few precision/robustness clarifications listed below.**

---

## 1) Requirement Coverage (84 total)

### Requirement families checked
- FP-1..15 (15)
- SV-1..13 (13)
- CH-1..7 (7)
- DP-1..6 (6)
- DS-1..8 (8)
- SK-1..4 (4)
- EH-1..6 (6)
- LF-1..10 (10)
- FF-1..15 (15)

Total = 84.

### Coverage verdict
From `plan/00-overview.md`, every family is explicitly assigned across phases P03..P18 (with analysis/pseudocode phases carrying full requirement sets and implementation phases carrying concrete subsets).

**Result: PASS — all 84 requirements are assigned.**

Notes:
- Coverage is explicit and traceable via phase tables.
- Parser/PCM/SDX2/Seek/FFI/Integration decomposition is coherent.

---

## 2) Technical Feasibility Review

## 2.1 IEEE 754 80-bit float parsing

### What the plan currently specifies
In `analysis/pseudocode/aiff.md`, `read_be_f80()`:
- Reads sign+exp, mantissa_hi, mantissa_lo
- Discards low 32 bits
- Treats exp==0 as 0
- exp==0x7FFF => invalid
- Uses `mantissa = (mantissa_hi >> 1)` and shift arithmetic with hardcoded 31-bit assumptions

### Technical correctness verdict
**Needs correction.**

Problems:
1. **Extended-precision format is mishandled conceptually**:
   - 80-bit ext has explicit integer bit + 63 fraction bits.
   - Current pseudocode effectively uses only top 31 bits of mantissa and an ad hoc shift.
2. **Discarding mantissa low 32 bits** introduces avoidable quantization and possible off-by-one on non-common sample rates.
3. **No proper handling of integer bit semantics** for normalized numbers.
4. The algorithm likely returns expected values for canonical AIFF sample rates (44100 etc.) due to favorable bit patterns, but it is **not a generally correct ext80->int conversion method**.

### Required fix
Specify proper conversion:
- Parse sign, exponent, 64-bit significand (integer bit + fraction).
- Handle classes:
  - exp=0 and significand=0 => 0
  - exp=0 and significand!=0 => denormal (for this decoder can map to 0 or InvalidData; document choice)
  - exp=0x7FFF => InvalidData
- For normal values:
  - value = (-1)^sign * significand * 2^(exp-16383-63)
  - Convert to integer sample rate with deterministic rounding/truncation policy (prefer truncation toward zero to mimic C integer cast behavior).
  - Clamp to i32 bounds if needed before validation.

---

## 2.2 SDX2 ADPCM decode correctness

### What is good
From pseudocode and phase plans:
- Formula: `v = (sample * abs(sample)) << 1`
- Delta mode when odd LSB set
- Saturation to [-32768, 32767]
- Per-channel `prev_val[ch]`
- Interleaved channel iteration by frame
- Predictor reset on open and seek

This aligns with expected SDX2-style predictor behavior and requirements DS-1..8.

### Points to tighten
1. **Delta mode toggle basis**: currently uses `sample_byte & 1`; that is plausible and likely right, but should be explicitly cross-checked against `aiffaud.c` bit-test semantics in tests.
2. **Output endianness path** in pseudocode uses `to_ne_bytes()` for no-swap. Ensure this matches existing decoder convention expected by mixer/FFI format flags.
3. **Potential arithmetic overflow concerns** are low here (input i8), but keep operations in i32 exactly as planned.

### Verdict
**Feasible and mostly correct**, pending explicit test vectors proving parity with C reference for odd/even, sign behavior, and stereo predictor independence.

---

## 2.3 PCM decode correctness

### What is correct
- DP-5 signed 8-bit to unsigned via `wrapping_add(128)` is correct for AIFF PCM8.
- DP-1/2/3/4/6 frame counting, copy, position updates, EOF semantics are coherent.

### Gap
- The review prompt asks to verify **16-bit endian swap**. The plan text does not clearly spell out a dedicated 16-bit swap step in `decode_pcm()`. It relies on metadata + `need_swap` usage in broader pipeline.

### Recommendation
Add explicit statement/tests for 16-bit endianness behavior for PCM path:
- Either decode outputs in host order with swap in decoder,
- Or output stays file endian and `need_swap` contract is honored by consumer.

Right now this is underspecified in PCM plan docs.

---

## 2.4 Seeking correctness

Plan behavior:
- clamp to max
- update `cur_pcm` and `data_pos`
- reset predictor

Verdict: **Correct and feasible**.

One subtlety: SDX2 seek-to-middle with predictor reset does not reproduce C streaming predictor history unless format semantics define independent frame decode. Plan explicitly chooses reset behavior (REQ-SK-3), so it is internally consistent.

---

## 2.5 FFI Init behavior (no init_module/init inside)

Plan explicitly states in `aiff_ffi.md` and `plan/17-ffi-impl.md`:
- `rust_aifa_Init` allocates Box only.
- `Open` must not call `init_module()/init()`.

Verdict: **PASS** for requested constraint.

---

## 2.6 In-memory architecture fitness for UQM AIFF use

Plan chooses full audio payload load at open (wav-like). This is technically feasible for typical UQM effects/music assets and significantly simplifies correctness/testing.

Risk envelope:
- Large asset memory spikes possible vs streaming C decoder.
- No explicit upper-bound safeguards in plan.

Verdict: **Feasible for expected UQM files**, but add optional size guard or documented assumption.

---

## 3) Integration Completeness

Checked against requested items:

1. **`USE_RUST_AIFF` in config_unix.h**
   - Planned via `config_unix.h.in` placeholder `@SYMBOL_USE_RUST_AIFF_DEF@`.
   - **PASS**.

2. **decoder.c registration matching USE_RUST_DUKAUD pattern**
   - Plan includes conditional include and sd_decoders switch for `"aif"`.
   - **PASS**.

3. **aiffaud.c fully replaceable**
   - Conditional vtable replacement path present; C fallback preserved.
   - Functionally yes, assuming symbol linkage and build vars wired.
   - **PASS (conditional on build glue correctness)**.

---

## 4) Verification Quality

At least 3 phases with real behavioral checks:

- **P04 Parser TDD**: rich behavioral matrix (valid/invalid headers, chunk alignment, duplicates, f80 edge classes, compression detection).
- **P11 SDX2 Impl**: semantic checks for odd/even delta behavior, clamping, predictor accumulation, channel independence.
- **P18 Integration**: dual-mode C integration checks (`USE_RUST_AIFF` on/off), table registration, linkage expectations.

Additional strong phases: P05, P08, P14, P17 all include semantic verification lists beyond mere compile checks.

Verdict: **PASS**.

---

## Critical Findings (must-fix)

1. **F80 algorithm spec is technically incorrect/incomplete** (highest severity).
2. **PCM 16-bit endian behavior is underspecified in implementation plan**.
3. **Potential panic hazard in pseudocode**: `self.formats.unwrap()` in `open_from_bytes()` SDX2 swap override path; if lifecycle ever violated, this panics. Prefer explicit error path.
4. **Chunk iteration arithmetic robustness**: `remaining` and per-chunk consume should guard underflow/overrun explicitly when malformed sizes claim beyond file end.

---

## Recommended Corrections to Plan Text

1. Replace f80 pseudocode with mathematically correct ext80 parsing (using full 64-bit significand).
2. Add parser/decoder tests that compare f80 decode against known raw bytes including non-canonical rates and edge-class values.
3. Clarify and test PCM16 endian contract explicitly.
4. In pseudocode, replace `formats.unwrap()` with guarded access + decoder error.
5. Add malformed-chunk boundary tests where `chunk_size` exceeds remaining bytes.
6. In SDX2 tests, include golden vectors from C decoder output to prove exact parity.

---

## Final Verdict

**Technical correctness status: CONDITIONAL PASS (after fixes).**

The plan is comprehensive and well-structured, and most core logic is feasible. However, **the current IEEE 80-bit float algorithm section is not sufficiently correct as written** and should be fixed before implementation proceeds. With that correction plus endian-contract clarification, this plan is solid for execution.
