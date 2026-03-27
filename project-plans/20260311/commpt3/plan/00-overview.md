# Plan: Communication Subsystem Production Parity (Part 3)

Plan ID: `PLAN-20260325-COMMPT3`
Generated: 2026-03-25
Total Phases: P00 through P16a (17 phases + 17 verification phases = 34 total)
Requirements: REQ-CM-001–003, REQ-MU-001–003, REQ-SD-001–005, REQ-CS-002–003,
              REQ-RL-001–004, REQ-SM-001–002, REQ-DC-001–005, REQ-TS-001–004,
              REQ-E2E-001–007

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase P00a)
2. Requirements finalized and locked (Phase P00)
3. Structure/interface baseline confirmed (Phase P00a)
4. Integration points are explicitly listed
5. **Stub → TDD → Impl cycle** is MANDATORY for every feature slice.
   Each slice is split into three separate sequential phases with their own
   verification files:
   - **Stub phase**: compile-safe stubs only (`todo!()` or empty C bodies).
     Gate: project compiles and links in both build modes.
   - **TDD phase**: tests written against stubs that define expected behavior.
     Gate: tests compile, expected failures documented. Tests MUST fail when
     behavior is intentionally broken (semantic negative-proof gate).
   - **Impl phase**: real implementation satisfying all tests from TDD phase.
     Gate: all tests pass, no stubs/placeholders remain. Negative-proof gate:
     intentionally breaking implementation causes specific tests to fail.
   Verification/sign-off phases (P15, P16) are exempt from the cycle but MUST
   verify that all preceding implementation phases completed all three sub-phases.
6. **Semantic Negative-Proof Gate**: Every verification phase MUST include a
   negative-proof section confirming that tests fail when behavior is intentionally
   broken. This prevents vacuously-passing tests.
7. Lint/test/coverage gates are declared
8. Sequential execution: P00 → P00a → P01 → ... → P16 → P16a
   No phase may be started until its predecessor is verified.
9. No `TODO`/`FIXME`/`HACK`/placeholder markers in production code after P14
10. Both `USE_RUST_COMM=on` and `USE_RUST_COMM=off` builds must compile and link at every phase
11. All implementation phases include pseudocode traceability with line-level
    requirement-to-pseudocode contracts. Verification/sign-off phases include
    traceability marker AUDIT confirming all markers are present.
12. Deferred-implementation detection uses per-match classification (no ambiguous
    pipe filtering). EVERY grep match must be individually classified.

## Problem Summary

Parts 1 and 2 built the Rust comm module structure and wired the FFI bridge,
but five runtime parity gaps remain:

1. **Null colormap**: `set_colormap()` at `talk_segue.rs:1003` passes null to C instead of `CommData.AlienColorMap`
2. **Null music**: `play_alien_music()` at `talk_segue.rs:945` passes null to C instead of `CommData.AlienSong`
3. **Subtitle rendering disconnected**: Subtitle bridges at `rust_comm.c:562-576` route to Rust model instead of C drawing
4. **DoCommunication lock hazard**: Response callback dispatch at `ffi.rs:732-747` can deadlock on COMM_STATE re-entry
5. **Stale markers**: "for now" (`talk_segue.rs:1002`) and "not yet" (`ffi.rs:879,881`) comments indicate deferred work

## Phase Summary

| Phase | Title | Type | Description | Est. LoC | Key Files |
|-------|-------|------|-------------|----------|-----------|
| P00 | Requirements Lock | Setup | Freeze requirements against codebase | — | requirements.md |
| P00a | Preflight | Verify | Toolchain, types, interfaces, call paths | — | — |
| P01 | Analysis | Analysis | Map gaps to file/line, trace integration | — | analysis/ |
| P01a | Analysis Verification | Verify | Confirm all requirements represented | — | — |
| P02 | Pseudocode | Design | Algorithmic pseudocode for all fixes | — | pseudocode/ |
| P02a | Pseudocode Verification | Verify | Confirm pseudocode covers all reqs | — | — |
| **P03** | **Colormap+Music Stub** | **Stub** | C stubs + Rust caller rewiring | ~15 | talk_segue.rs, rust_comm.c/h |
| P03a | Stub Verification | Verify | Build gate, no behavior | — | — |
| **P04** | **Colormap+Music TDD** | **TDD** | Tests defining bridge behavior | ~20 | talk_segue.rs tests |
| P04a | TDD Verification | Verify | Expected failures documented | — | — |
| **P05** | **Colormap+Music Impl** | **Impl** | Real bridge implementations | ~25 | rust_comm.c, talk_segue.rs |
| P05a | Impl Verification | Verify | All TDD tests pass, negative-proof | — | — |
| **P06** | **Subtitle Display Stub** | **Stub** | comm.c stubs + routing rewire | ~15 | comm.c, rust_comm.c/h |
| P06a | Stub Verification | Verify | Build gate, routing confirmed | — | — |
| **P07** | **Subtitle Display TDD** | **TDD** | Structural tests for subtitle behavior | ~10 | grep-based tests |
| P07a | TDD Verification | Verify | Expected failures documented | — | — |
| **P08** | **Subtitle Display Impl** | **Impl** | comm.c subtitle implementations | ~65 | comm.c |
| P08a | Impl Verification | Verify | All TDD tests pass, negative-proof | — | — |
| **P09** | **DoCommunication Stub** | **Stub** | New enum + stub state machine | ~30 | talk_segue.rs, ffi.rs |
| P09a | Stub Verification | Verify | Build gate, stubs non-functional | — | — |
| **P10** | **DoCommunication TDD** | **TDD** | State machine + lock discipline tests | ~50 | talk_segue.rs, ffi.rs tests |
| P10a | TDD Verification | Verify | Expected failures documented | — | — |
| **P11** | **DoCommunication Impl** | **Impl** | Real state machine + lock lifecycle | ~90 | talk_segue.rs, ffi.rs |
| P11a | Impl Verification | Verify | All TDD tests pass, negative-proof | — | — |
| **P12** | **Summary Guard Stub** | **Stub** | cfg(test) bifurcation | ~15 | ffi.rs |
| P12a | Stub Verification | Verify | Build gate, delegation wired | — | — |
| **P13** | **Summary Guard TDD** | **TDD** | Delegation + marker sweep tests | ~10 | grep-based tests |
| P13a | TDD Verification | Verify | Expected failures documented | — | — |
| **P14** | **Summary Guard Impl** | **Impl** | Marker removal, final cleanup | ~10 | ffi.rs |
| P14a | Impl Verification | Verify | All TDD tests pass, negative-proof | — | — |
| P15 | Integration Build | Verify | Full cross-build, traceability audit | ~0 | — |
| P15a | Integration Verification | Verify | Both build modes, all tests | — | — |
| P16 | Final Parity Sign-off | Verify | E2E runtime + manual verification | — | — |
| P16a | Final Verification | Verify | Complete pass/fail decision | — | — |

Estimated total: ~270 new/modified LoC (Rust + C).

## Slice → Phase Mapping

| Slice | Feature | Stub | TDD | Impl | Pseudocode Reference |
|-------|---------|------|-----|------|---------------------|
| 1 | Colormap + Music Bridges | P03 | P04 | P05 | 001-colormap-music-bridges.md |
| 2 | Subtitle Display Fix | P06 | P07 | P08 | 002-subtitle-display-fix.md |
| 3 | DoCommunication Rewrite | P09 | P10 | P11 | 003-do-communication-rewrite.md |
| 4 | Summary Guard + Markers | P12 | P13 | P14 | 004-summary-guard-stale-markers.md |

## Architecture Invariants

- **C is authoritative for rendering** — Rust never draws pixels
- **C is authoritative for resource management** — Rust calls `c_*` bridge wrappers
- **C is authoritative for the DoInput frame loop** — Rust is called back per frame
- **C trackplayer is authoritative for subtitle content** — `GetTrackSubtitle()` is source of truth
- **Rust is authoritative for dialogue state** — CommState, talk segue, response system
- **Lock must be dropped before C callbacks** — COMM_STATE write lock released before `callback_fn(ref)`
- **Tests remain Rust-only** — `#[cfg(test)]` paths use simulated input/rendering

## Execution Order (Mandatory)

```
P00 → P00a → P01 → P01a → P02 → P02a →
P03 → P03a → P04 → P04a → P05 → P05a →
P06 → P06a → P07 → P07a → P08 → P08a →
P09 → P09a → P10 → P10a → P11 → P11a →
P12 → P12a → P13 → P13a → P14 → P14a →
P15 → P15a → P16 → P16a
```

No phase may be started until its predecessor is verified.
