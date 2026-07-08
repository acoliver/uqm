# Phase 02a: Pseudocode Verification (Revised — iteration 4)

## Phase ID
`PLAN-20260707-MAINLOOP.P02a`

## Purpose
Verify that the pseudocode covers all requirements and is algorithmically
complete (validation points, error handling, ordering constraints,
integration boundaries, side effects).

---

## Requirements Coverage

| Requirement | Pseudocode Lines | Covered? |
|-------------|-----------------|----------|
| REQ-ML-001 | 1-7 | YES — `rust_game_loop` entry |
| REQ-ML-002 | 10-20 | YES — Starcon2Main-specific init only (C owns startup) |
| REQ-ML-003 | 35, 42-43, 105, 109 | YES — get/set activity via FFI |
| REQ-ML-004 | 44-47 (evaluate+execute), 48-54 (encounter post-dispatch) | YES |
| REQ-ML-005 | (implicit — every FFI call is a boundary) | YES |
| REQ-ML-006 | (removed — merged with REQ-ML-002, C owns startup) | N/A |
| REQ-ML-007 | 22-70 | YES — outer while + inner loop-until-CHECK_ABORT |
| REQ-ML-008 | 86-94 | YES — game-kernel cleanup only (starcon.c:313-318) |
| REQ-ML-009 | (P07 — C-to-Rust callback) | Deferred |
| REQ-ML-010 | 71-73 (named accessors), 123-124 | YES — game state via FFI accessor |

## Algorithmic Completeness Checklist

- [x] Validation points: LoadKernel check, StartGame return
- [x] Error handling: `?` propagation, Err→EXIT_FAILURE
- [x] Ordering: SetStatusMessageMode BEFORE load path (starcon.c:235)
- [x] CurrentActivity=0 before splash (starcon.c:205)
- [x] START_ENCOUNTER set before VisitStarBase/RaceCommunication (starcon.c:254,259)
- [x] Dispatch evaluate→execute→re-read (starcon.c:243-286)
- [x] Encounter-only post-dispatch mutation (starcon.c:263-268)
- [x] LastActivity from fresh CurrentActivity (starcon.c:290)
- [x] Combined win/loss/death condition (starcon.c:292-303)
- [x] Game-kernel cleanup only, not subsystem teardown

## Pseudocode ↔ C Source Cross-Reference

| Pseudocode | C Source (starcon.c) | Match? |
|-----------|----------------------|--------|
| Lines 10-20 (audio+kernel+clear+splash) | starcon.c:168-207 | YES |
| Line 22 (while StartGame) | starcon.c:198 | YES |
| Lines 23-28 (game init) | starcon.c:199-205 | YES |
| Line 32-33 (SetStatusMessageMode) | starcon.c:235 | YES |
| Lines 35-43 (load/velocity check) | starcon.c:237-241 | YES |
| Lines 44-47 (evaluate+execute) | starcon.c:243-286 | YES |
| Lines 48-54 (encounter post-dispatch) | starcon.c:263-268 | YES |
| Lines 120-134 (win/loss/death) | starcon.c:292-303 | YES |
| Lines 86-94 (game-kernel cleanup) | starcon.c:313-318 | YES |

---

## Gate Decision

- [x] PASS: all active requirements have pseudocode coverage
- [x] PASS: pseudocode is numbered and traceable
- [x] PASS: every C source line in the game loop has a pseudocode equivalent
- [x] PASS: error paths and ordering are explicit
- [x] PASS: CurrentActivity=0 before splash verified
- [x] PASS: START_ENCOUNTER before encounter calls verified
- [x] PASS: encounter-only post-dispatch verified
- [x] PASS: game-kernel cleanup only (not subsystem teardown)

**Result: PASS — proceed to implementation phases (P02b).**
