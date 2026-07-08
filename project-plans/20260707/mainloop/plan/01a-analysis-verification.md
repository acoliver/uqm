# Phase 01a: Analysis Verification

## Phase ID
`PLAN-20260707-MAINLOOP.P01a`

## Purpose
Verify that the Phase 01 analysis covers all requirements from the specification.

---

## Requirements Coverage Matrix

| Requirement | Analysis Section | Covered? | Evidence |
|-------------|-----------------|----------|----------|
| REQ-ML-001 (Rust game loop) | §4 (old code to replace) | YES | `Starcon2Main` delegates to `rust_game_loop()` listed |
| REQ-ML-002 (Init (C owns startup)) | §2 (init ordering table) | YES | C owns startup; Rust does Starcon2Main-specific init only |
| REQ-ML-003 (CurrentActivity accessors) | §3 (FFI touchpoints) + §1 (state model) | YES | `get_current_activity`/`set_current_activity` listed as NEW accessors |
| REQ-ML-004 (Activity state machine) | §1 (transition diagram + key transitions) | YES | All 3 dispatch branches + win/loss documented |
| REQ-ML-005 (Boundary test suite) | §3 + spec §8 | YES | FFI touchpoints enumerated; test approach in spec |
| REQ-ML-006 (Init ordering observable) | §2 (ordered table) | YES | 21-step ordered list with step numbers |
| REQ-ML-007 (Loop outer/inner structure) | §1 (transition diagram) | YES | Outer (`StartGame`) + inner (`do...while CHECK_ABORT`) shown |
| REQ-ML-008 (Shutdown sequence) | §3 (shutdown functions) | YES | `UninitGameKernel`, `FreeMasterShipList`, `FreeKernel` listed |
| REQ-ML-009 (C-to-Rust callback) | §3 (Rust functions C must call) | YES | `rust_dispatch_activity` listed |
| REQ-ML-010 (Game state round-trip) | §3 + §5 | YES | `get_game_state_byte`/`set_game_state_byte` + CHMMR_BOMB_STATE in transitions |

---

## FFI Completeness Check

- [x] Every C function called from Rust in the init sequence is listed (§2 + §3)
- [x] Every C function called from Rust in the game loop is listed (§3)
- [x] Every C function called from Rust in shutdown is listed (§3)
- [x] Every NEW accessor function needed is identified (§3)
- [x] Every Rust function C must call back is identified (§3)

## Old-Code-to-Replace Check

- [x] `main()` in `uqm.c` — current vs new behavior documented (§4)
- [x] `StartThread` launch — identified for removal (§4)
- [x] `Starcon2Main` body — identified for replacement (§4)
- [x] Backward compatibility strategy defined (§4 — `USE_RUST_MAINLOOP` guard)

## Edge Case Coverage

- [x] LoadKernel failure → fatal exit (§5)
- [x] SetPlayerInputAll failure → abort (§5)
- [x] StartGame false → shutdown (§5)
- [x] Player death → break inner loop (§5)
- [x] Unknown activity → default to Battle (§5)

---

## Gate Decision

- [x] PASS: all 10 requirements represented in analysis
- [x] PASS: all FFI touchpoints enumerated
- [x] PASS: old-code-to-replace list is explicit
- [x] PASS: edge cases mapped

**Result: PASS — proceed to Phase 02 (Pseudocode).**
