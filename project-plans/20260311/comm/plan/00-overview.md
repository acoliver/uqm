# Plan: Communication / Dialogue Subsystem Gap Closure

Plan ID: PLAN-20260314-COMM
Generated: 2026-03-14
Total Phases: 23 (P00.5 through P11 with verification sub-phases, plus final integration phase P12)
Requirements: EC-REQ-*, DS-REQ-*, PS-REQ-*, RS-REQ-*, TP-REQ-*, SS-REQ-*, AO-REQ-*, SB-REQ-*, OL-REQ-*, IN-REQ-*, CB-REQ-*, CV-REQ-*, SC-REQ-*

## Context

The communication/dialogue subsystem is **partially ported**. `USE_RUST_COMM` is active, 48 FFI exports exist in Rust, and Rust owns state management, track/subtitle/response/animation/oscilloscope models. However, the C side (`comm.c`, `commglue.c`, `commanim.c`) is still the authoritative runtime — only `init_communication()` / `uninit_communication()` are actually routed through Rust. The 27 race-specific dialogue tree scripts remain in C and must continue to compile and work without modification.

This plan closes all gaps between the current Rust code and the specification/requirements, making Rust the authoritative comm runtime while keeping race scripts in C.

## Gap Summary

| # | Gap | Severity | Requirements |
|---|-----|----------|-------------|
| G1 | Response callback ABI mismatch: Rust uses `extern "C" fn()` but C expects `void (*)(RESPONSE_REF)` | Critical | RS-REQ-011, RS-REQ-012, CV-REQ-008 |
| G2 | `rust_SpliceTrack` missing `timestamps` and `callback` parameters vs. spec §14.2 | Critical | TP-REQ-001, DS-REQ-008, SS-REQ-001 |
| G3 | No `rust_SpliceMultiTrack` FFI export (spec §14.2) | High | TP-REQ-002, DS-REQ-009 |
| G4 | No `rust_JumpTrack` current-phrase skip semantics (current impl takes offset arg) | High | TP-REQ-005, TP-REQ-006 |
| G5 | TrackManager is a synthetic timeline — no integration with authoritative trackplayer subtitle/history/pending-completion model | Critical | TP-REQ-001–013, SS-REQ-001–017 |
| G6 | No phrase-level completion callback dispatch or pending-completion poll loop | Critical | TP-REQ-003, TP-REQ-010, TP-REQ-012, CB-REQ-001–002 |
| G7 | No subtitle history enumeration or conversation summary integration via trackplayer-owned history APIs | High | TP-REQ-008, SS-REQ-013–017 |
| G8 | No phrase enable/disable state (`PHRASE_ENABLED`/`DISABLE_PHRASE` support) | High | PS-REQ-001–007, DS-REQ-012, SC-REQ-002 |
| G9 | `CommData` (Rust `types.rs`) missing most LOCDATA fields (resources, text layout, anim descriptors) | Critical | EC-REQ-003, EC-REQ-007, DS-REQ-004, SC-REQ-003 |
| G10 | No encounter lifecycle orchestration in Rust (`InitCommunication`, `HailAlien`, resource load/teardown) | Critical | EC-REQ-001–016 |
| G11 | No glue-layer implementation: `NPCPhrase_cb`, `NPCPhrase_splice`, `NPCNumber`, `construct_response` | Critical | DS-REQ-005–010, SC-REQ-001 |
| G12 | No `setSegue`/`getSegue` implementation in Rust | High | DS-REQ-011, SB-REQ-001–006 |
| G13 | No coherent `init_race`/LOCDATA dispatch bridge from Rust into existing C race-script initialization | Critical | DS-REQ-001–002, DS-REQ-004 |
| G14 | Animation engine is generic — missing ANIMATION_DESC, BlockMask, WAIT_TALKING, ambient/talk/transit model | Critical | AO-REQ-001–010, AO-REQ-016 |
| G15 | No talk segue, DoCommunication main loop, or AlienTalkSegue implementation | Critical | SS-REQ-006–011, IN-REQ-010 |
| G16 | No response rendering, scrolling, or selection UI integration | High | RS-REQ-006–009, IN-REQ-006 |
| G17 | No speech graphics (oscilloscope rendering into RadarContext, slider) | Medium | AO-REQ-011–015 |
| G18 | Unsafe subtitle pointer lifetime in `rust_GetSubtitle()` | High | OL-REQ-009–010, CV-REQ-005 |
| G19 | C-side `comm.c` body not guarded — only init/uninit behind `#ifndef USE_RUST_COMM` | Critical | IN-REQ-012 |
| G20 | Lock discipline: `COMM_STATE` RwLock held during callback dispatch → deadlock risk | Critical | CB-REQ-006, CB-REQ-008–009 |
| G21 | No `CommitTrackAdvancement` / `PollPendingTrackCompletion` integration with trackplayer | Critical | TP-REQ-012, CB-REQ-002 |
| G22 | `rust_ExecuteResponse` calls callback as `fn()` not `fn(RESPONSE_REF)` | Critical | RS-REQ-011 |
| G23 | `RaceCommunication()` remains C-owned / unspecified even though the spec requires Rust ownership of both public entry points | Critical | EC-REQ-001, IN-REQ-012 |
| G24 | Required all-27-script audit for the narrowed phrase-disable semantic model is not planned as a gating artifact | Critical | PS-REQ-007 |
| G25 | Saved-game SIS display update step in encounter setup ordering is not explicitly planned | High | EC-REQ-001, EC-REQ-003 |

## Phase Structure

| Phase | Title | Gaps Addressed | Est. LoC |
|-------|-------|---------------|----------|
| P00.5 | Preflight Verification | G24 | 0 |
| P01 | Analysis | G23, G24, G25 | 0 |
| P01a | Analysis Verification | G23, G24, G25 | 0 |
| P02 | Pseudocode | G23, G25 | 0 |
| P02a | Pseudocode Verification | G23, G25 | 0 |
| P03 | CommData & LOCDATA FFI | G9, G13 | ~450 |
| P03a | CommData Verification | -- | 0 |
| P04 | Phrase State & Glue Layer | G8, G11, G12 | ~600 |
| P04a | Phrase State & Glue Verification | -- | 0 |
| P05 | FFI Signature Corrections | G1, G2, G3, G4, G18, G22 | ~350 |
| P05a | FFI Corrections Verification | -- | 0 |
| P05b | Trackplayer C Wrapper Seam | G5, G6, G7, G21 | ~180 (C) |
| P05ba | Trackplayer Wrapper Verification | -- | 0 |
| P06 | Track Model & Trackplayer Integration | G5, G6, G7, G21 | ~700 |
| P06a | Track Integration Verification | -- | 0 |
| P07 | Animation Engine | G14 | ~800 |
| P07a | Animation Engine Verification | -- | 0 |
| P08 | Encounter Lifecycle & Entry Points | G10, G23, G25 | ~700 |
| P08a | Encounter Lifecycle Verification | -- | 0 |
| P09 | Talk Segue & Main Loop | G15, G20 | ~500 |
| P09a | Talk Segue Verification | -- | 0 |
| P10 | Response UI & Speech Graphics | G16, G17 | ~400 |
| P10a | Response UI Verification | -- | 0 |
| P11 | C-Side Bridge Wiring | G19, G23 | ~300 (C) |
| P11a | C-Side Wiring Verification | -- | 0 |
| P12 | End-to-End Integration & Verification | All | ~200 |

Total estimated new/modified LoC: ~5000 (Rust) + ~480 (C)

## Execution Order

```
P00.5 -> P01 -> P01a -> P02 -> P02a
       -> P03 -> P03a -> P04 -> P04a
       -> P05 -> P05a -> P05b -> P05ba -> P06 -> P06a
       -> P07 -> P07a -> P08 -> P08a
       -> P09 -> P09a -> P10 -> P10a
       -> P11 -> P11a -> P12
```

Each phase MUST be completed and verified before the next begins. No skipping.

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 0.5)
2. Integration points are explicitly listed
3. TDD cycle is defined per slice
4. Lint/test/coverage gates are declared
5. Any requirement intentionally treated as existing behavior has explicit verification evidence in P00.5/P01a

## Definition of Done

1. All `cargo test --workspace --all-features` pass
2. All `cargo clippy --workspace --all-targets --all-features -- -D warnings` pass
3. `cargo fmt --all --check` passes
4. Game boots with `USE_RUST_COMM=1` and dialogue encounters work correctly
5. Both public entry points are Rust-owned in Rust mode: `RaceCommunication()` and `InitCommunication()`
6. Response callback ABI matches C convention: `void (*)(RESPONSE_REF)`
7. All 27 race scripts compile without modification against updated headers
8. Required all-27-script phrase-disable audit is recorded and any exceptions are explicitly handled
9. `comm.c`, `commglue.c`, `commanim.c` bodies guarded behind `#ifndef USE_RUST_COMM`
10. Phrase enable/disable works correctly across encounters
11. Conversation summary shows correct history using trackplayer enumeration as source of truth
12. Animation scheduling matches C behavior (ambient/talk/transit, BlockMask, WAIT_TALKING)
13. No deadlocks when C callbacks re-enter Rust comm API
14. Saved-game SIS display update step is preserved in encounter setup ordering
15. No placeholder stubs or TODO markers remain in implementation code

## Plan Files

```
plan/
  00-overview.md                              (this file)
  00a-preflight-verification.md               P00.5
  01-analysis.md                              P01
  01a-analysis-verification.md                P01a
  02-pseudocode.md                            P02
  02a-pseudocode-verification.md              P02a
  03-commdata-locdata-ffi.md                  P03
  03a-commdata-locdata-ffi-verification.md    P03a
  04-phrase-state-glue-layer.md               P04
  04a-phrase-state-glue-verification.md       P04a
  05-ffi-signature-corrections.md             P05
  05a-ffi-signature-corrections-verification.md  P05a
  05b-trackplayer-c-wrapper-seam.md           P05b
  05ba-trackplayer-wrapper-verification.md    P05ba
  06-track-model-trackplayer.md               P06
  06a-track-model-trackplayer-verification.md P06a
  07-animation-engine.md                      P07
  07a-animation-engine-verification.md        P07a
  08-encounter-lifecycle.md                   P08
  08a-encounter-lifecycle-verification.md     P08a
  09-talk-segue-main-loop.md                  P09
  09a-talk-segue-main-loop-verification.md    P09a
  10-response-ui-speech-graphics.md           P10
  10a-response-ui-speech-graphics-verification.md  P10a
  11-c-side-bridge-wiring.md                  P11
  11a-c-side-bridge-wiring-verification.md    P11a
  12-e2e-integration-verification.md          P12
  execution-tracker.md
```

## Deferred Items

The following are explicitly out of scope:

- **Race script porting to Rust**: The 27 C dialogue tree scripts remain in C. Porting them is a future effort after the Rust comm runtime is fully validated.
- **Trackplayer (audio-heart) porting**: If audio-heart remains C-owned, comm integrates via FFI. This plan does not port the trackplayer itself.
- **Advanced HQxx scalers / OpenGL rendering**: These are graphics subsystem concerns, not comm concerns.
- **Number-speech table authoring**: `NPCNumber` will support existing `AlienNumberSpeech` tables through FFI; no new tables are created.
*: `NPCNumber` will support existing `AlienNumberSpeech` tables through FFI; no new tables are created.
