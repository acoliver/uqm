# Plan: Main Game Loop Port to Rust — Revised (iteration 5)

Plan ID: `PLAN-20260707-MAINLOOP`
Generated: 2026-07-07 (revised iteration 5)
Specification: [`../specification.md`](../specification.md)

## Revision Summary (iteration 5)
**Test architecture fix**: explicit 3-tier testing strategy to address
cargo test linking limitations. **Module path consistency**: unified to
`game_loop.rs`. **FFI surface audit**: all macros/globals use C wrappers.
**Removed `rust_dispatch_activity`**: no C call site exists.

## Revision Summary (iteration 3)
**Architectural simplification based on deepthinker review**: C `main()`
owns startup (`uqm.c:283-452`), the main-thread event pump
(`uqm.c:456-472`), and full subsystem shutdown (`uqm.c:479-507`).
**Rust replaces only the `Starcon2Main()` body** and does only
game-kernel cleanup (`starcon.c:313-318`). No startup wrapper.
No subsystem teardown in Rust (prevents double-free). Static-function
wrappers moved to `starcon.c` (same translation unit).

---

## Test Architecture (3 Tiers)

**Tier 1 — Pure Rust unit tests** (`cargo test --workspace --lib`):
No C linkage. Uses injected traits/hooks for all game-state reads and
side effects. Tests state-machine logic, flag arithmetic, loop control.

**Tier 2 — C ABI shim tests** (`cargo test` with `rust_test_bridge.c`):
A self-contained C shim in `rust/build.rs` that defines test-local
globals (not real UQM globals) and implements the accessor wrappers
against them. Tests ABI shape: type sizes, flag values, round-trips.

**Tier 3 — External process tests** (`cd sc2 && ./build.sh uqm`):
Full binary build. Verify symbols via `nm`, boot via `./uqm -o -f`
with timeout. NO cargo test for full-UQM FFI calls (requires SDL,
content dirs, initialized engine).

---

## Critical Reminders

1. **C main-thread event pump is PRESERVED.** Rust replaces the
   `Starcon2Main()` *body*, not the threading model. The main thread
   continues running `TFB_ProcessEvents`/`ProcessUtilityKeys`/
   `ProcessThreadLifecycles`/`TFB_FlushGraphics` (uqm.c:456-472).
2. **C `main()` owns startup AND subsystem shutdown.** Rust does NOT
   wrap or replicate `uqm.c:283-452` startup, and does NOT call
   `uqm.c:479-507` subsystem teardown. Rust's shutdown is only
   `starcon.c:313-318` (game-kernel cleanup) + sets `MainExited`.
3. **Static C function wrappers go IN starcon.c.** `on_battle_frame`
   and `BackgroundInitKernel` are `static` in `starcon.c` — wrappers
   must be added to the same file, not a separate `.c` file.
4. **Game state is bit-packed.** Use named C wrappers
   (`uqm_get_chmmr_bomb_state()`), NOT byte offsets.
5. **Re-read CurrentActivity after dispatch.** C activity functions
   mutate it; using a stale value causes incorrect loop behavior.
6. **Set LastActivity after dispatch.** It's a standalone global
   (`setup.h:60`), not in GlobData.
7. **NextActivity is required.** Load/restart path reads
   `CurrentActivity | NextActivity` (starcon.c:237). Accessor needed.
8. **Encounter-only post-dispatch mutation.** The
   clear-START_ENCOUNTER / maybe-set-START_INTERPLANETARY block runs
   only after encounter branches, NOT all branches (starcon.c:263-268).
9. **Real C boundary tests, not mocks.** But split into safe unit
   tests (pure Rust state machine) and integration tests (real C calls
   requiring SDL/content).
10. **Sequential phase execution only.** P02b → P03 → P04 → P05 → P06 → P07 → P08.

---

## Phase List

| Phase | Title | Type | Primary REQs |
|------:|-------|------|--------------|
| P0.5  | Preflight Verification | Verification | (all — assumption check) |
| P01   | Domain Analysis | Analysis | (all — requirement mapping) |
| P01a  | Analysis Verification | Verification | (all — REQ coverage) |
| P02   | Pseudocode | Design | (all — algorithmic traceability) |
| P02a  | Pseudocode Verification | Verification | (all — REQ coverage) |
| P02b  | **C Wrapper Functions (in starcon.c + accessors)** | **Impl (C)** | **(prerequisite for all)** |
| P03   | Activity Types + FFI Accessors (Rust side) | Stub+TDD+Impl | REQ-ML-003, REQ-ML-005, REQ-ML-010 |
| P03a  | P03 Verification | Verification | REQ-ML-003, REQ-ML-005, REQ-ML-010 |
| P04   | Startup Verification (C owns startup) | Verification gate | REQ-ML-002 |
| P05   | Activity State Machine | Stub+TDD+Impl | REQ-ML-004 |
| P05a  | P05 Verification | Verification | REQ-ML-004 |
| P06   | Game Loop Body (`rust_game_loop`) | Stub+TDD+Impl | REQ-ML-001, REQ-ML-007 |
| P06a  | P06 Verification | Verification | REQ-ML-001, REQ-ML-007 |
| P07   | Game-Kernel Cleanup + C-to-Rust Callback + Wiring | Stub+TDD+Impl | REQ-ML-008, REQ-ML-009 |
| P07a  | P07 Verification | Verification | REQ-ML-008, REQ-ML-009 |
| P08   | End-to-End Integration Verification | Integration | REQ-ML-001…010 (holistic) |
| P08a  | P08 Verification | Verification | (all) |

---

## Execution Tracker

| Phase | Status | Verified | Notes |
|------:|--------|----------|-------|
| P00.5 | ⬜     | ⬜       | Preflight gate |
| P01   | ⬜     | ⬜       | Analysis |
| P01a  | ⬜     | ⬜       | |
| P02   | ⬜     | ⬜       | Pseudocode |
| P02a  | ⬜     | ⬜       | |
| P02b  | ⬜     | ⬜       | **C wrappers in starcon.c + accessors** |
| P03   | ⬜     | ⬜       | Rust types + FFI externs |
| P03a  | ⬜     | ⬜       | |
| P04   | ⬜     | ⬜       | Startup verification (C owns startup) |
| P05   | ⬜     | ⬜       | State machine (named accessors) |
| P05a  | ⬜     | ⬜       | |
| P06   | ⬜     | ⬜       | Game loop body |
| P06a  | ⬜     | ⬜       | |
| P07   | ⬜     | ⬜       | Game-kernel cleanup + callback + wiring |
| P07a  | ⬜     | ⬜       | |
| P08   | ⬜     | ⬜       | E2E integration |
| P08a  | ⬜     | ⬜       | |

---

## Requirements → Phase Mapping

| Requirement | Title | Primary Phase | Verification |
|-------------|-------|---------------|--------------|
| REQ-ML-001 | Rust Game Loop Body | P06 | P06a, P08a |
| REQ-ML-002 | Init Sequence (C owns startup) | P04 (verify) | P08a |
| REQ-ML-003 | CurrentActivity/NextActivity/LastActivity Accessors | P03 (Rust) + P02b (C) | P03a, P08a |
| REQ-ML-004 | Activity State Machine | P05 | P05a, P08a |
| REQ-ML-005 | FFI Boundary Tests | P03+P06+P07 | P03a+P08a |
| REQ-ML-006 | (removed — merged with REQ-ML-002) | — | — |
| REQ-ML-007 | Loop Outer/Inner Structure | P06 | P06a, P08a |
| REQ-ML-008 | Game-Kernel Cleanup (starcon.c:313-318 only) | P07 | P07a, P08a |
| REQ-ML-009 | C-to-Rust Callback | P07 | P07a, P08a |
| REQ-ML-010 | Game State (Named Accessors) | P02b (C) + P03 (Rust) + P05 (consumer) | P03a, P05a, P08a |

---

## Module Layout (target)

```
rust/src/mainloop/
  mod.rs              — public re-exports, MainLoopError
  activity.rs         — ActivityValue, ActivityKind, ActivityFlags (P03)
  c_extern.rs         — extern "C" declarations for C wrappers (P03-P07)
  bridge.rs           — safe wrappers over C calls (P03-P07)
  state_machine.rs    — ActivityStateMachine (P05)
  game_loop.rs        — rust_game_loop() + loop body (P06)
  shutdown.rs         — game-kernel cleanup: starcon.c:313-318 only (P07)

sc2/src/uqm/starcon.c                  — static-function wrappers (P02b)
sc2/src/uqm/rust_bridge_mainloop.c     — activity/global accessors (P02b)
sc2/src/uqm/rust_bridge_mainloop.h     — prototypes (P02b)
```

---

## Gates

**Per-phase structural gate:**
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

**Per-phase anti-fraud gate:**
```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/mainloop/
```

**Test tiering:**
- Tier 1: Pure Rust unit tests (state machine decisions) — run in all CI
- Tier 2: Scalar FFI boundary tests (activity accessors) — run in CI with C lib
- Tier 3: Integration/SDL tests (full boot) — external process test, not cargo test

**End-to-end gate (P08):**
```bash
cd sc2 && ./build.sh uqm
nm sc2/uqm | grep rust_game_loop
cd sc2 && ./uqm -o -f &   # full boot test (proves init + loop + cleanup)
```

---

## Definition of Done

- [ ] All REQ-ML-* requirements have a passing test
- [ ] `rust_game_loop()` replaces Starcon2Main body
- [ ] C main-thread pump preserved (uqm.c:456-472 unchanged)
- [ ] C `main()` startup preserved (uqm.c:283-452 unchanged)
- [ ] CurrentActivity re-read after every dispatch; LastActivity set
- [ ] NextActivity read for load/restart path (starcon.c:237-241)
- [ ] Encounter-only post-dispatch mutation (starcon.c:263-268)
- [ ] Game-kernel cleanup only (starcon.c:313-318); C main() does subsystem teardown
- [ ] C-to-Rust callback `rust_dispatch_activity()` works
- [ ] No `unwrap()`/`expect()` in any `mainloop/` FFI bridge file
- [ ] `cargo fmt`, `cargo clippy -D warnings`, `cargo test` all green
- [ ] Zero hits from deferred-implementation grep
- [ ] Named game-state accessors (not byte offsets)
- [ ] C wrappers for 2 static functions (on_battle_frame, BackgroundInitKernel) in starcon.c
