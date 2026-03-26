# Plan: Communication Subsystem Completion (Part 2)

Plan ID: `PLAN-20260326-COMMPT2`
Generated: 2026-03-26
Total Phases: P00.5 through P08 (10 implementation/analysis phases + 8 verification phases)
Requirements: REQ-HL-001–007, REQ-IP-001–008, REQ-NP-001–004, REQ-RB-001–004, REQ-AT-001–003, REQ-DI-001–004, REQ-CS-001–003, REQ-E2E-001–006

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 0.5)
2. Integration points are explicitly listed
3. TDD cycle is defined per slice
4. Lint/test/coverage gates are declared
5. Sequential execution: P00.5 → P01 → P01a → P02 → P02a → P03 → P03a → ... → P08 → P08a
6. No `TODO`/`FIXME`/`HACK`/placeholder markers in production code after P08
7. Both `USE_RUST_COMM=on` and `USE_RUST_COMM=off` builds must compile and link at every phase

## Problem Summary

PLAN-20260314-COMM (Part 1) built Rust modules for animation, track management,
encounter lifecycle, response handling, subtitle tracking, speech graphics, and the
talk-segue state machine.  However, four critical integration gaps remain:

1. **`rust_HailAlien` is empty** — conversations are silently skipped under `USE_RUST_COMM=on`
2. **Input bridge functions return hardcoded `false`** — dialogue is non-interactive
3. **`rust_NPCPhrase_cb`/`rust_NPCPhrase_splice` are stubs** — NPC speech doesn't work
4. **C-side rendering bridges are stubs** — player responses and summary don't render

This plan closes all four gaps.

## Execution Tracker

| Phase | Title | Status | Verified | Semantic Verified | Notes |
|------:|-------|--------|----------|-------------------|-------|
| P00.5 | Preflight Verification | ⬜ | ⬜ | N/A | |
| P01   | Analysis | ⬜ | ⬜ | N/A | |
| P01a  | Analysis Verification | ⬜ | ⬜ | N/A | |
| P02   | Pseudocode | ⬜ | ⬜ | N/A | |
| P02a  | Pseudocode Verification | ⬜ | ⬜ | N/A | |
| P03   | Input Bridge | ⬜ | ⬜ | ⬜ | ~100 LoC |
| P03a  | Input Bridge Verification | ⬜ | ⬜ | ⬜ | |
| P04   | NPC Phrase | ⬜ | ⬜ | ⬜ | ~150 LoC |
| P04a  | NPC Phrase Verification | ⬜ | ⬜ | ⬜ | |
| P05   | C Rendering Bridges | ⬜ | ⬜ | ⬜ | ~200 C LoC |
| P05a  | C Rendering Verification | ⬜ | ⬜ | ⬜ | |
| P06   | Resource Bridge | ⬜ | ⬜ | ⬜ | ~300 C LoC |
| P06a  | Resource Bridge Verification | ⬜ | ⬜ | ⬜ | |
| P07   | HailAlien | ⬜ | ⬜ | ⬜ | ~500 Rust LoC |
| P07a  | HailAlien Verification | ⬜ | ⬜ | ⬜ | |
| P08   | Integration Sweep | ⬜ | ⬜ | ⬜ | ~100 LoC |
| P08a  | Final E2E Verification | ⬜ | ⬜ | ⬜ | |

## Phase Summary

| Phase | Description | Est. LoC | Key Files |
|-------|-------------|----------|-----------|
| P00.5 | Verify toolchain, existing bridges, constants, tests | — | — |
| P01 | Stub inventory, call-path traces, resource lifecycle map | — | — |
| P02 | Pseudocode for HailAlien, input bridge, NPCPhrase, rendering | — | — |
| P03 | Wire `check_*_input` → `c_GetPulsedMenuKey`; fix `has_transition_anim` | ~100 | `talk_segue.rs` |
| P04 | Implement `rust_NPCPhrase_cb`/`rust_NPCPhrase_splice` | ~150 | `ffi.rs` |
| P05 | Implement C rendering stubs (`c_FeedbackPlayerPhrase`, etc.) | ~200 | `rust_comm.c` |
| P06 | Add C bridge wrappers for resource load/destroy/context ops | ~300 | `rust_comm.c`, `rust_comm.h` |
| P07 | Implement `rust_HailAlien` encounter orchestration | ~500 | New `hail.rs`, `ffi.rs`, `mod.rs` |
| P08 | Integration sweep: eliminate markers, full test pass | ~100 | All comm files |

## Architecture Invariants

- **C is authoritative for rendering** — Rust delegates all drawing to C bridge wrappers
- **C is authoritative for resource management** — Rust calls `c_Load*`/`c_Destroy*` wrappers
- **C is authoritative for input polling** — Rust calls `c_GetPulsedMenuKey` for real input
- **C is authoritative for the DoInput frame loop** — Rust calls `c_DoInput` or equivalent
- **Rust is authoritative for dialogue state** — `CommState`, talk segue, response system
- **Tests remain Rust-only** — `#[cfg(test)]` paths use simulated input, not C bridges
- **No race script changes** — All 27 race scripts remain untouched C code

## Key Integration Points

| From | To | Mechanism |
|------|----|-----------|
| `comm.c:1458` | `rust_HailAlien()` | `#ifdef USE_RUST_COMM` guard |
| `commglue.c` | `rust_NPCPhrase_cb/splice` | `#ifdef USE_RUST_COMM` guard |
| `talk_segue.rs` | `c_GetPulsedMenuKey()` | Rust FFI extern "C" |
| `hail.rs` (new) | `c_Load*/c_Destroy*` | Rust FFI extern "C" |
| `hail.rs` (new) | `c_CreateContext/SetContext` | Rust FFI extern "C" |
| `hail.rs` (new) | `c_DrawSIS*/c_DoInput` | Rust FFI extern "C" |
| Rust response_ui | `c_FeedbackPlayerPhrase` | Rust FFI extern "C" → C bridge |
| Rust response_ui | `c_RefreshResponses` | Rust FFI extern "C" → C bridge |
| Rust summary | `c_SelectConversationSummary` | Rust FFI extern "C" → C bridge |

## Definition of Done (from PLAN-20260314-COMM.P12)

- [ ] `rust_HailAlien` executes the full encounter loop
- [ ] NPC phrases play audio and display subtitles
- [ ] Player can select responses and see them rendered
- [ ] Conversation summary is accessible
- [ ] All 267+ comm tests pass
- [ ] Both build modes compile, link, and run
- [ ] No `TODO`/`FIXME`/`Stub`/placeholder markers in production code
- [ ] All 27 race encounters work identically to C-only mode
