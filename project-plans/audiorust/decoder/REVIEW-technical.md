# Technical Review — AIFF Decoder Plan

*Reviewed by deepthinker subagent, 2026-02-25*

## Verdict: CONDITIONAL PASS (4 issues)

Plan is mostly solid but requires fixes to f80 parsing spec, FFI robustness, and requirement-to-phase traceability before implementation.

## 1. Requirement Coverage

All 84 EARS requirements from rust-decoder.md (FP-*, SV-*, CH-*, DP-*, DS-*, SK-*, EH-*, LF-*, FF-*) are assigned to specific phases. The parser phase covers FP-*/SV-*/CH-*, PCM decode covers DP-*, SDX2 covers DS-*, seek covers SK-*/EH-*, FFI covers LF-*/FF-*.

## 2. Technical Feasibility

- AIFF chunk parsing algorithm is correct (FORM header → chunk iteration → COMM/SSND extraction)
- SDX2 ADPCM decode: predictor state, saturation clamp [-32768, 32767], delta mode, per-channel state — all correct
- PCM decode: 8-bit signed→unsigned (+128), 16-bit endian swap — correct
- Seeking with predictor reset — correct
- FFI vtable matches existing dukaud_ffi.rs/wav_ffi.rs patterns

## 3. Integration Completeness

- P18 wires in with USE_RUST_AIFF flag in config_unix.h
- Registration in decoder.c with conditional: `#ifdef USE_RUST_AIFF ... rust_aifa_DecoderVtbl ... #else ... aifa_DecoderVtbl`
- aiffaud.c will be fully replaceable

## 4. Issues Found

1. **[Must-fix]** IEEE 754 80-bit float parsing: the pseudocode should explicitly handle the denormalized case (exponent == 0) and the infinity/NaN case (exponent == 0x7FFF). The C code doesn't handle these either, but Rust should be robust. In practice, AIFF sample rates are always normal floats (22050, 44100, etc.), so this is low-risk but should be documented.
2. **[Should-fix]** FF-4 (Init function) conflates allocation with init_module()/init() calls — the docwriter review flagged this. The plan's FFI impl phase should NOT call init_module() inside Init(), matching dukaud_ffi.rs pattern.
3. **[Should-fix]** Requirement-to-phase traceability could be more explicit — some phases list requirements by category but not individual IDs.
4. **[Minor]** The plan doesn't address the EH-2 errno clarification from the spec review (positive errno values not applicable in pure Rust).
