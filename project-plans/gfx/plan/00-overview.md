# Plan: GFX Vtable Fix (Black Screen Bug)

Plan ID: `PLAN-20260223-GFX-VTABLE-FIX`
Generated: 2026-02-23
Total Phases: 18 (P00.5 through P14)
Requirements: REQ-INIT-010..100, REQ-UNINIT-010..030, REQ-SURF-010..070,
  REQ-PRE-010..050, REQ-SCR-010..170, REQ-SCALE-010..070, REQ-SCALE-025,
  REQ-SCALE-055, REQ-CLR-010..070, REQ-CLR-055, REQ-UTS-010..030,
  REQ-POST-010..030, REQ-SEQ-010..070, REQ-THR-010..035, REQ-ERR-010..065,
  REQ-INV-005..061, REQ-FMT-010..040, REQ-WIN-010..030, REQ-AUX-010..060,
  REQ-NP-010..070, REQ-ASM-010..050, REQ-FFI-010..060

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 0.5)
2. Integration points are explicitly listed
3. TDD cycle is defined per slice
4. Lint/test/coverage gates are declared
5. `unsafe` is explicitly approved for FFI boundary code in this feature
6. Single file to modify: `rust/src/graphics/ffi.rs`

## Slices

The implementation is divided into these logical slices:

| Slice | Name | Description |
|---|---|---|
| A | Preprocess Fix | Add blend mode reset before clear |
| B | Screen Compositing | Implement ScreenLayer unscaled path |
| C | Color Layer | Implement ColorLayer (fade/tint) |
| D | Scaling Integration | Move scaling from Postprocess to ScreenLayer |
| E | Postprocess Refactor | Reduce Postprocess to present-only |
| F | Error Handling | Negative rect guards, already-init guard, validation |
| INT | Integration | End-to-end wiring verification |

## Phase Map

| Phase | Type | Slice | Description |
|---|---|---|---|
| P00.5 | Preflight | — | Toolchain, deps, types, test infra |
| P01 | Analysis | — | Domain model, flow analysis |
| P01a | Verification | — | Analysis verification |
| P02 | Pseudocode | — | Algorithmic pseudocode |
| P02a | Verification | — | Pseudocode verification |
| P03 | Stub | A+E | Preprocess fix + Postprocess refactor stubs |
| P03a | Verification | A+E | Stub verification |
| P04 | TDD | A+E | Tests for preprocess fix + postprocess refactor |
| P04a | Verification | A+E | TDD verification |
| P05 | Impl | A+E | Implement preprocess fix + postprocess present-only |
| P05a | Verification | A+E | Implementation verification |
| P06 | Stub | B | ScreenLayer stub (compile-safe skeleton) |
| P06a | Verification | B | Stub verification |
| P07 | TDD | B | ScreenLayer tests |
| P07a | Verification | B | TDD verification |
| P08 | Impl | B | ScreenLayer unscaled implementation |
| P08a | Verification | B | Implementation verification |
| P09 | Stub | C | ColorLayer stub |
| P09a | Verification | C | Stub verification |
| P10 | TDD | C | ColorLayer tests |
| P10a | Verification | C | TDD verification |
| P11 | Impl | C | ColorLayer implementation |
| P11a | Verification | C | Implementation verification |
| P12 | Stub+TDD+Impl | D | Scaling integration (relocate from postprocess) |
| P12a | Verification | D | Scaling verification |
| P13 | Stub+TDD+Impl | F | Error handling hardening |
| P13a | Verification | F | Error handling verification |
| P14 | Integration | INT | End-to-end verification |
| P14a | Verification | INT | Integration verification |

## Phase Completion Markers

Phase completion marker files (`.completed/PNN.md`) are **post-execution
artifacts** — they are created after each phase is successfully executed
and verified, not as part of the plan itself. The `.completed/` directory
starts empty (with a `.gitkeep`). As each phase is executed:

1. Execute all implementation tasks in the phase
2. Run all verification commands and pass all gates
3. Create `.completed/PNN.md` with the contents specified in the phase file
4. Update the Execution Tracker below (Status → [OK], Verified → [OK])

The absence of `.completed/PNN.md` files before execution begins is
expected and correct.

## Execution Tracker

| Phase | Status | Verified | Semantic Verified | Notes |
|------:|--------|----------|-------------------|-------|
| P00.5 | ⬜     | ⬜       | N/A               |       |
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
