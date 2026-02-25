# AIFF Decoder Plan — Pedantic Technical Review

**Reviewer:** Technical review (4th round)
**Date:** 2026-02-24
**Scope:** `rust-decoder.md` (spec), `specification.md`, `analysis/domain-model.md`, all 37 plan files in `plan/`
**Method:** Line-by-line reading in 200-line chunks, cross-referenced against C source (`aiffaud.c`, `decoder.c`), existing Rust decoders (`wav.rs`, `wav_ffi.rs`, `dukaud_ffi.rs`), and the `SoundDecoder` trait.

---

## Summary

The plan is comprehensive and well-structured after 3 fix rounds. The spec-to-plan traceability is strong, the TDD cycle is properly sequenced, and the FFI lifecycle is correctly modeled. I found **4 genuine technical issues** (1 medium, 3 low), **5 pedantic observations**, and **2 internal contradictions** between documents.

---

## ISSUE-1 [MEDIUM]: SDX2 odd-bit check — `i8` sign-extension changes bit 0 semantics

**Location:** Spec §3.4 (DS-4 step 3), Plan P10 (test cases), Plan P11 (implementation)

**Problem:** The spec says:

> DS-4 step 3: If `(sample_byte as u8) & 1 != 0`: `v += self.prev_val[ch]`

And annotates: "check the original byte, not the widened i32."

The C code does:

```c
sint8 *src;
// ...
if (*src & 1)
    v += *prev;
```

In C, `*src` is `sint8` (-128..127). The expression `*src & 1` promotes `*src` to `int` (sign-extending), then ANDs with 1. For negative values like -1 (0xFF as unsigned), sign-extension to int gives `0xFFFFFFFF`, and `0xFFFFFFFF & 1 == 1`. This is identical to checking the LSB of the unsigned representation.

The Rust spec says `(sample_byte as u8) & 1`. This works correctly because casting `i8` to `u8` in Rust preserves the bit pattern (e.g., `-1i8 as u8 == 0xFF`, `0xFF & 1 == 1`). However, an implementer might instead write `(sample as i32) & 1` after widening, which would also be correct because Rust's sign-extension from `i8` to `i32` preserves the LSB. **Both are equivalent** for the LSB check.

**Verdict:** The spec's `(sample_byte as u8) & 1` is correct. But the parenthetical note "check the original byte, not the widened i32" could mislead an implementer into thinking `(sample as i32) & 1` would be wrong (it wouldn't be — sign extension preserves bit 0). Consider clarifying that any representation works for bit 0, but the `as u8` form is preferred for clarity.

**Severity justification:** Medium because a confused implementer might introduce an unnecessary intermediate variable or incorrect cast trying to satisfy the "original byte" note, when the widened i32 check is equally correct.

---

## ISSUE-2 [LOW]: Contradictory `need_swap` logic between spec body §3.8 and CH-7

**Location:** Spec §3.8 "SDX2 Mode" vs CH-7

**Problem:** The spec body §3.8 says:

> ```rust
> self.need_swap = cfg!(target_endian = "big") != self.formats.unwrap().want_big_endian;
> ```

But CH-7 (the EARS requirement, which was explicitly fixed in a prior round) says:

> `self.need_swap = self.formats.as_ref().unwrap().big_endian != self.formats.as_ref().unwrap().want_big_endian`

And the CH-7 **Note** explicitly says:

> This uses the runtime `formats.big_endian` field (from `TFB_DecoderFormats`), NOT the compile-time `cfg!(target_endian = "big")`.

The C reference (line 500-501) confirms:

```c
This->need_swap = (aifa_formats->big_endian != aifa_formats->want_big_endian);
```

**Verdict:** CH-7 is correct. The §3.8 code snippet is stale/wrong — it still uses `cfg!(target_endian = "big")` instead of `formats.big_endian`. The EARS requirement takes precedence, but the inconsistency could confuse an implementer reading the spec top-to-bottom.

**Recommendation:** Update §3.8's SDX2 code snippet to match CH-7.

---

## ISSUE-3 [LOW]: f80 algorithm step numbering is inconsistent with overflow guard

**Location:** Spec §3.2 steps 10-12, FP-14 steps 10-12

**Problem:** Step 10 says:

> `shift = biased_exp as i32 - 16383 - 63`

Step 11 says:

> If `shift >= 0` and would overflow `i32`: clamp `abs_val = 0x7FFF_FFFF`; else `abs_val = significand << shift`

This conflates the overflow check with the left-shift case. For extremely large exponents, `significand << shift` could overflow `u64` *before* being truncated to `i32`. The spec should be clear that:

1. The overflow check applies to the *result* fitting in `i32`, not `u64`.
2. For typical audio sample rates (shift is always negative), this path is never taken.

The C code handles this more simply:

```c
if (shift > 0)
    mant = 0x7fffffff; // already too big
```

The C code clamps *unconditionally* for any positive shift because after the `mant >>= 1` (which makes it 31-bit), any positive shift would exceed 31 bits.

The Rust spec attempts a more nuanced approach ("if would overflow"), but the condition for "would overflow" isn't precisely defined. In practice, any `shift >= 0` with a nonzero 64-bit significand would produce a value exceeding `i32::MAX`, so the simpler C approach of clamping on any `shift >= 0` is functionally equivalent and less error-prone.

**Recommendation:** Simplify step 11 to: "If `shift >= 0`: `abs_val = 0x7FFF_FFFF` (clamp — value exceeds i32 range)". This matches the C logic exactly and removes the ambiguous "would overflow" conditional.

---

## ISSUE-4 [LOW]: P18a verification checklist contradicts P15/P15a on Init pattern

**Location:** P18a Semantic Verification (Deterministic Checks), bullet:

> `FFI Init function does NOT call init_module()/init() (matching dukaud_ffi.rs pattern)`

**Problem:** This directly contradicts:

- **REQ-FF-4** (spec): "Init MUST call `dec.init_module()` and `dec.init()` to propagate formats"
- **P15** (FFI Stub): "Init — implemented (Box::new + call dec.init_module(0, &formats) from global Mutex + call dec.init() + store pointer; matches wav_ffi.rs Init pattern)"
- **P15a** (FFI Stub Verification): "Init does `Box::new(AiffDecoder::new())` + `dec.init_module(0, &formats)` from global Mutex + `dec.init()` + `Box::into_raw`"
- **P17** (FFI Impl): "Open does NOT call init_module()/init() — they are already called in Init()"

The wav_ffi.rs source (lines 138-150) confirms that Init DOES call `init_module()` and `init()`. The DukAud FFI does NOT call these (because DukAud doesn't implement the `SoundDecoder` trait). The AIFF decoder implements `SoundDecoder` like WAV, so it follows the WAV pattern.

**Verdict:** P18a has a stale/incorrect verification check. It should read: "FFI Init function DOES call init_module()/init() (matching wav_ffi.rs pattern, NOT dukaud_ffi.rs pattern)."

---

## ISSUE-5 [PEDANTIC]: `data_start` calculation ambiguity in SSND parsing

**Location:** Spec §3.1 "Sound Data Chunk (SSND) Parsing" and FP-12

**Problem:** The spec says:

> Compute `data_start = current_cursor_position + offset`.

But then later says:

> the total data size is determined later from `sample_frames * file_block`

This implies `data_start` is saved, and the actual extraction happens during validation (SV-10). However, the SSND chunk has additional data *after* the 8-byte header and before the audio samples (the `offset` field specifies this gap). The `data_start` must account for the cursor position *after* reading the SSND header (offset + block_size), plus the `offset` value itself.

If chunk iteration then skips past the SSND chunk via `cursor.seek(chunk_size - 8)` (per FP-13), the actual audio data bytes are within that skipped region. The decoder must have saved `data_start` before skipping.

This works correctly if implemented as described, but the interaction between "save data_start" and "skip remaining SSND data" should be clearer — the skip in FP-13 skips over the audio data too, and the extraction in SV-10 goes back to `data_start` from the original input slice.

**Verdict:** Correct but could benefit from a note: "After saving `data_start`, the SSND chunk's remaining data is skipped during chunk iteration. The audio data is extracted from the input byte slice at `data[data_start..]` during validation, after all chunks have been parsed."

---

## ISSUE-6 [PEDANTIC]: Test vector for 22050 Hz f80 encoding has non-obvious derivation

**Location:** P04 (Parser TDD), test case 17

**Problem:** The plan provides f80 test vectors and includes derivation:

> For 44100: biased_exp = 15 + 16383 = 0x400E, significand = 0xAC44_0000_0000_0000.

The derivation for 44100 is shown, but the vector for 22050 uses `0x400D` as the exponent. Let me verify:

- 22050 in binary: 0b101_0110_0010_0010 (15 bits, so floor(log2(22050)) = 14)
- biased_exp = 14 + 16383 = 16397 = 0x400D [OK]
- significand = 22050 << (63 - 14) = 22050 << 49 = 0xAC44_0000_0000_0000
- But wait: 22050 = 0x5622, and 0x5622 << 49 = 0xAC44_0000_0000_0000 [OK] (0x5622 << 1 = 0xAC44, then shifted by 48 more)

The vectors are correct. The derivation comment only shows 44100, but all 6 vectors check out mathematically.

**Verdict:** Correct. No issue. But the derivation section could show at least one more example (e.g., 8000 Hz) to help future reviewers verify without manual calculation.

---

## ISSUE-7 [PEDANTIC]: REQ-SV-9 file_block for SDX2 may produce non-integer for odd block_align

**Location:** Spec REQ-SV-9, specification.md REQ-SV-9

**Problem:** `file_block = block_align / 2`. If `bits_per_sample` were 8 and channels were 1, then `block_align = 1`, and `file_block = 0` (integer division truncation in Rust). This would cause division-by-zero in the data extraction (SV-10: `sample_frames * file_block`).

However, REQ-CH-5 requires `bits_per_sample == 16` for SDX2, so `block_align >= 2` is guaranteed. And REQ-SV-3 requires channels is 1 or 2, so block_align for SDX2 is either 2 (mono) or 4 (stereo). The division is always exact.

**Verdict:** No actual bug — the validation ordering prevents this. But the spec doesn't explicitly state the invariant that `block_align` is always even for SDX2. An assertion comment in the implementation would be prudent.

---

## ISSUE-8 [PEDANTIC]: Specification validation ordering vs C ordering documented but not in plan

**Location:** `specification.md` "Intentional Deviations from C" item 1

**Problem:** The specification explicitly documents that validation ordering differs from C:

> The Rust decoder checks SSND presence *before* block_align and compression type validation, while C validates *after* block_align.

This is documented in `specification.md` but not mentioned in the plan phases (P05 parser implementation). An implementer following only the plan might reorder validations to match C without realizing the intentional deviation.

**Verdict:** P05 should reference the intended validation ordering from `specification.md` to prevent an implementer from "fixing" it to match C.

---

## ISSUE-9 [PEDANTIC]: SDX2 endianness — inline swap vs framework swap tension

**Location:** Spec §3.8, specification.md "Intentional Deviations from C" item 3, P11 (SDX2 impl)

**Problem:** The specification.md states:

> The Rust decoder performs the byte swap *inline during SDX2 decode* (via `swap_bytes().to_ne_bytes()`)

But the spec body §3.4 says only:

> Write v as i16 to output (respecting endianness via need_swap)

And the C framework's `SoundDecoder_Decode()` in decoder.c (lines 556-561) *also* does byte swapping when `need_swap` is true:

```c
if (decoder->need_swap && decoded_bytes > 0 &&
        (decoder->format == decoder_formats.stereo16 ||
        decoder->format == decoder_formats.mono16))
{
    SoundDecoder_SwapWords(decoder->buffer, decoded_bytes);
}
```

**Critical question:** If the Rust SDX2 decoder performs inline byte swapping AND the C framework also swaps based on `need_swap`, wouldn't that cause **double-swapping**?

The answer is **no**, because CH-7 sets `need_swap = formats.big_endian != formats.want_big_endian` for SDX2 (overriding the PCM default of `!want_big_endian`). The C SDX2 decoder writes `*dst = v` which produces native-endian output, and then the framework may swap. The Rust SDX2 decoder would write bytes in the target byte order directly, and `need_swap` would be set such that the framework does NOT additionally swap.

BUT — the spec's CH-7 `need_swap` formula (`big_endian != want_big_endian`) is designed for the case where the decoder produces machine-native output (like C's `*dst = v`). If the Rust decoder instead writes in the *target* byte order directly (respecting `want_big_endian`), then `need_swap` should be `false` (no framework swap needed). These are two different strategies:

**Strategy A (C-like):** Write native endian → framework swaps if `need_swap`
**Strategy B (Rust inline):** Write target endian directly → `need_swap = false`

The spec seems to describe Strategy B (inline swap) but uses Strategy A's `need_swap` formula. If the implementer follows CH-7's `need_swap = big_endian != want_big_endian` AND does inline swap to target endian, the framework would double-swap when `need_swap` is true.

**Resolution:** Actually, re-reading more carefully: the inline swap writes in the byte order indicated by `want_big_endian`. And `need_swap = big_endian != want_big_endian`. On a little-endian machine with `big_endian = false` and `want_big_endian = false`: `need_swap = false`, no framework swap, and the Rust decoder writes little-endian (which IS native, so `to_ne_bytes()` suffices). On a little-endian machine with `big_endian = false` and `want_big_endian = true`: `need_swap = true`, framework swaps, and... the Rust decoder writes in what byte order? If it writes native (little-endian), then the framework swap produces big-endian, which is correct. So the inline swap should write in native byte order, not target byte order.

This is confusing. The `specification.md` says "performs the byte swap inline" but if `need_swap` is also set, the framework will swap again. The correct approach for SDX2 is: **write native-endian i16 (no inline swap), let the framework swap via `need_swap`** — exactly like C does with `*dst = v`.

**Verdict:** The spec body (§3.4 "respecting endianness via need_swap") and CH-7 are consistent with the C approach (write native, framework swaps). The `specification.md` "Intentional Deviations" item 3 claiming inline swap is **wrong** and contradicts the rest of the spec. The implementer should write `v as i16` in native byte order (using `.to_ne_bytes()`) and let `need_swap` + the framework handle it.

**Severity upgrade consideration:** This should arguably be Medium, but the EARS requirements (CH-7, DS-4) and the spec body are all consistent and correct. Only the `specification.md` deviation note is wrong. Since the plan references the EARS requirements (not the deviation note), the risk of incorrect implementation is low.

---

## ISSUE-10 [INTERNAL CONTRADICTION]: P18a vs FF-4 on Init pattern

This is the same as ISSUE-4, listed separately for tracking.

**P18a says:** "FFI Init function does NOT call init_module()/init() (matching dukaud_ffi.rs pattern)"
**FF-4 says:** "Init MUST call dec.init_module() and dec.init()"
**P15/P17 say:** "Init calls init_module and init, matching wav_ffi.rs"

**Action:** Fix P18a to match FF-4 / P15 / P17.

---

## ISSUE-11 [INTERNAL CONTRADICTION]: Requirement count — "84" vs actual count

**Location:** P18a Semantic Verification: "All 84 requirements... implemented"

**Count check:**
- REQ-FP: 1-15 = 15
- REQ-SV: 1-13 = 13
- REQ-CH: 1-7 = 7
- REQ-DP: 1-6 = 6
- REQ-DS: 1-8 = 8
- REQ-SK: 1-4 = 4
- REQ-EH: 1-6 = 6
- REQ-LF: 1-10 = 10
- REQ-FF: 1-15 = 15
- **Total: 84** [OK]

**Verdict:** Count is correct. Not an issue.

---

## Items Verified as Correct

The following were specifically checked and found to be correct:

1. **f80 test vectors** — All 6 sample rate encodings verified by manual IEEE 754 80-bit calculation. The exponent and significand values are correct.

2. **SDX2 algorithm** — `v = (sample * abs(sample)) << 1` matches C's `v = (*src * abs(*src)) << 1` exactly. Sign preservation through the square-with-absolute-value operation is correct.

3. **FFI Init lifecycle** — `Box::new()` → `init_module(0, &formats)` → `init()` → `Box::into_raw()` matches `wav_ffi.rs` lines 138-150 exactly.

4. **FFI Term lifecycle** — `Box::from_raw()` → drop → null pointer. Correct, prevents double-free.

5. **PCM no inline swap** — Both the spec and C reference (`aifa_DecodePCM`) copy raw big-endian bytes. The framework's `SoundDecoder_Decode()` (decoder.c lines 556-561) handles swap via `need_swap`. Correct.

6. **8-bit signed-to-unsigned** — `wrapping_add(128)` is equivalent to C's `*ptr += 128` with unsigned wrap semantics for the byte type.

7. **Chunk alignment padding** — Skip 1 byte for odd-sized chunks. Matches AIFF spec and C implementation.

8. **Validation error codes** — `last_error = -2` for format errors matches C's `aifae_BadFile`.

9. **Close-before-error-return** — `open_from_bytes()` calls `self.close()` on every error path, matching C's `aifa_Close(This)` before `return false`.

10. **Seek predictor reset** — `memset(prev_val, 0, ...)` in C matches `prev_val = [0i32; MAX_CHANNELS]` in Rust.

11. **TDD phasing** — RED→GREEN cycle is correctly sequenced: stub → tests (compile-only) → implementation → all tests pass.

12. **Plan phase dependencies** — Each phase correctly lists prerequisites. No phase references artifacts from a future phase.

13. **Deferred implementation detection** — Every phase includes `grep todo!()` checks with expected output. No phase allows `todo!()` in methods that should be implemented.

---

## Recommendations Summary

| # | Severity | Action |
|---|----------|--------|
| 1 | MEDIUM | Clarify DS-4 step 3 note about odd-bit check — both `(sample_byte as u8) & 1` and `(sample as i32) & 1` are correct for bit 0 |
| 2 | LOW | Fix §3.8 SDX2 need_swap snippet to use `formats.big_endian` (not `cfg!`) — matches CH-7 |
| 3 | LOW | Simplify f80 step 11 to unconditionally clamp on `shift >= 0` (matches C) |
| 4 | LOW | Fix P18a verification checklist — Init DOES call init_module/init (wav_ffi pattern, not dukaud) |
| 5 | PEDANTIC | Add clarifying note to SSND parsing about data_start vs chunk skip interaction |
| 6 | PEDANTIC | Show f80 derivation for more sample rates beyond 44100 |
| 7 | PEDANTIC | Note that block_align is guaranteed even for SDX2 (by CH-5 + SV-3) |
| 8 | PEDANTIC | P05 should reference the intentional validation ordering deviation |
| 9 | PEDANTIC | Fix `specification.md` deviation item 3 — SDX2 should write native-endian, not inline-swap to target endian. The EARS requirements (CH-7, DS-4) are correct; only the deviation note is wrong. |

---

## Overall Assessment

**The plan is ready for execution.** The 1 medium issue is a documentation clarity concern, not a correctness bug — both interpretations of the odd-bit check produce identical results. The 3 low issues are internal inconsistencies between documents that should be fixed before implementation to avoid confusion, but they don't represent algorithmic errors. The pedantic items are suggestions for defensive documentation.

The spec's EARS requirements (the authoritative source) are technically correct on all points I verified. The plan's TDD structure will catch any implementation errors that slip through — the test vectors for f80, the known-value SDX2 tests, and the PCM byte-preservation tests all serve as strong correctness gates.
