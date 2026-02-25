# Plan: AIFF Audio Decoder — Rust Port

Plan ID: PLAN-20260225-AIFF-DECODER
Generated: 2026-02-25
Total Phases: 18 (P01–P18, plus P00a preflight)
Requirements: REQ-FP-1..15, REQ-SV-1..13, REQ-CH-1..7, REQ-DP-1..6, REQ-DS-1..8, REQ-SK-1..4, REQ-EH-1..6, REQ-LF-1..10, REQ-FF-1..15

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 0.5)
2. Integration points are explicitly listed
3. TDD cycle is defined per slice
4. Lint/test/coverage gates are declared

## Plan Structure

| Phase | Title | Type | Requirements |
|-------|-------|------|-------------|
| P00a | Preflight Verification | Verification | — |
| P01 | Analysis | Analysis | All REQ-FP-1..15, REQ-SV-1..13, REQ-CH-1..7, REQ-DP-1..6, REQ-DS-1..8, REQ-SK-1..4, REQ-EH-1..6, REQ-LF-1..10, REQ-FF-1..15 |
| P01a | Analysis Verification | Verification | — |
| P02 | Pseudocode | Design | All REQ-FP-1..15, REQ-SV-1..13, REQ-CH-1..7, REQ-DP-1..6, REQ-DS-1..8, REQ-SK-1..4, REQ-EH-1..6, REQ-LF-1..10, REQ-FF-1..15 |
| P02a | Pseudocode Verification | Verification | — |
| P03 | Parser Stub | Stub | REQ-FP-1..15, REQ-SV-1..13, REQ-CH-1..7, REQ-LF-1..10, REQ-EH-1..4 (stubs) |
| P03a | Parser Stub Verification | Verification | — |
| P04 | Parser TDD | TDD | REQ-FP-1, REQ-FP-2, REQ-FP-3, REQ-FP-5, REQ-FP-7, REQ-FP-8, REQ-FP-9, REQ-FP-10, REQ-FP-14, REQ-SV-1..6, REQ-CH-1..6 |
| P04a | Parser TDD Verification | Verification | — |
| P05 | Parser Implementation | Impl | REQ-FP-1..15, REQ-SV-1..13, REQ-CH-1..7, REQ-LF-7, REQ-LF-8, REQ-EH-3 |
| P05a | Parser Implementation Verification | Verification | — |
| P06 | PCM Decode Stub | Stub | REQ-DP-1 (dispatch), REQ-EH-6 |
| P06a | PCM Decode Stub Verification | Verification | — |
| P07 | PCM Decode TDD | TDD | REQ-DP-1, REQ-DP-2, REQ-DP-3, REQ-DP-4, REQ-DP-5, REQ-DP-6 |
| P07a | PCM Decode TDD Verification | Verification | — |
| P08 | PCM Decode Implementation | Impl | REQ-DP-1, REQ-DP-2, REQ-DP-3, REQ-DP-4, REQ-DP-5, REQ-DP-6 |
| P08a | PCM Decode Implementation Verification | Verification | — |
| P09 | SDX2 Decode Stub | Stub | REQ-DS-1 (dispatch confirmation) |
| P09a | SDX2 Decode Stub Verification | Verification | — |
| P10 | SDX2 Decode TDD | TDD | REQ-DS-1, REQ-DS-2, REQ-DS-3, REQ-DS-4, REQ-DS-5, REQ-DS-6, REQ-DS-7, REQ-DS-8 |
| P10a | SDX2 Decode TDD Verification | Verification | — |
| P11 | SDX2 Decode Implementation | Impl | REQ-DS-1, REQ-DS-2, REQ-DS-3, REQ-DS-4, REQ-DS-5, REQ-DS-6, REQ-DS-7, REQ-DS-8 |
| P11a | SDX2 Decode Implementation Verification | Verification | — |
| P12 | Seek Stub | Stub | REQ-SK-1 (stub confirmation) |
| P12a | Seek Stub Verification | Verification | — |
| P13 | Seek TDD | TDD | REQ-SK-1, REQ-SK-2, REQ-SK-3, REQ-SK-4 |
| P13a | Seek TDD Verification | Verification | — |
| P14 | Seek Implementation | Impl | REQ-SK-1, REQ-SK-2, REQ-SK-3, REQ-SK-4 |
| P14a | Seek Implementation Verification | Verification | — |
| P15 | FFI Stub | Stub | REQ-FF-1, REQ-FF-2, REQ-FF-3, REQ-FF-4, REQ-FF-5, REQ-FF-10, REQ-FF-11, REQ-FF-12, REQ-FF-15 |
| P15a | FFI Stub Verification | Verification | — |
| P16 | FFI TDD | TDD | REQ-FF-2, REQ-FF-10, REQ-FF-11, REQ-FF-12 |
| P16a | FFI TDD Verification | Verification | — |
| P17 | FFI Implementation | Impl | REQ-FF-4, REQ-FF-5, REQ-FF-6, REQ-FF-7, REQ-FF-8, REQ-FF-9, REQ-FF-13, REQ-FF-14, REQ-FF-15 |
| P17a | FFI Implementation Verification | Verification | — |
| P18 | Integration | Integration | REQ-FF-2, REQ-FF-7 |
| P18a | Integration Verification | Verification | — |

## Execution Tracker

| Phase | Status | Verified | Semantic Verified | Notes |
|------:|--------|----------|-------------------|-------|
| P00a  | ⬜     | ⬜       | N/A               |       |
| P01   | ⬜     | ⬜       | ⬜                |       |
| P01a  | ⬜     | ⬜       | ⬜                |       |
| P02   | ⬜     | ⬜       | ⬜                |       |
| P02a  | ⬜     | ⬜       | ⬜                |       |
| P03   | ⬜     | ⬜       | ⬜                |       |
| P03a  | ⬜     | ⬜       | ⬜                |       |
| P04   | ⬜     | ⬜       | ⬜                |       |
| P04a  | ⬜     | ⬜       | ⬜                |       |
| P05   | ⬜     | ⬜       | ⬜                |       |
| P05a  | ⬜     | ⬜       | ⬜                |       |
| P06   | ⬜     | ⬜       | ⬜                |       |
| P06a  | ⬜     | ⬜       | ⬜                |       |
| P07   | ⬜     | ⬜       | ⬜                |       |
| P07a  | ⬜     | ⬜       | ⬜                |       |
| P08   | ⬜     | ⬜       | ⬜                |       |
| P08a  | ⬜     | ⬜       | ⬜                |       |
| P09   | ⬜     | ⬜       | ⬜                |       |
| P09a  | ⬜     | ⬜       | ⬜                |       |
| P10   | ⬜     | ⬜       | ⬜                |       |
| P10a  | ⬜     | ⬜       | ⬜                |       |
| P11   | ⬜     | ⬜       | ⬜                |       |
| P11a  | ⬜     | ⬜       | ⬜                |       |
| P12   | ⬜     | ⬜       | ⬜                |       |
| P12a  | ⬜     | ⬜       | ⬜                |       |
| P13   | ⬜     | ⬜       | ⬜                |       |
| P13a  | ⬜     | ⬜       | ⬜                |       |
| P14   | ⬜     | ⬜       | ⬜                |       |
| P14a  | ⬜     | ⬜       | ⬜                |       |
| P15   | ⬜     | ⬜       | ⬜                |       |
| P15a  | ⬜     | ⬜       | ⬜                |       |
| P16   | ⬜     | ⬜       | ⬜                |       |
| P16a  | ⬜     | ⬜       | ⬜                |       |
| P17   | ⬜     | ⬜       | ⬜                |       |
| P17a  | ⬜     | ⬜       | ⬜                |       |
| P18   | ⬜     | ⬜       | ⬜                |       |
| P18a  | ⬜     | ⬜       | ⬜                |       |

## Integration Contract

### Existing Callers
- `sc2/src/libs/sound/decoders/decoder.c` → `sd_decoders[]` table → `"aif"` extension entry
- C audio mixer → vtable function pointers → `Open`, `Decode`, `Seek`, `Close`

### Existing Code Replaced
- `aifa_DecoderVtbl` (from `aiffaud.c`) replaced by `rust_aifa_DecoderVtbl` when `USE_RUST_AIFF` is defined

### User Access Path
- Any `.aif` file loaded by the game's sound system (music, effects)

### Module Registration
- `rust/src/sound/mod.rs`: `pub mod aiff;` added in Phase P03 (parser stub)
- `rust/src/sound/mod.rs`: `pub mod aiff_ffi;` + `pub use aiff_ffi::rust_aifa_DecoderVtbl;` added in Phase P15 (FFI stub)

### Data/State Migration
- None — the vtable API is identical; only the implementation changes

### End-to-End Verification
- Build with `USE_RUST_AIFF=1`, load a game that plays `.aif` audio, verify playback
- `cargo test --lib --all-features` for unit tests
- `cd sc2 && ./build.sh uqm` for C integration build
