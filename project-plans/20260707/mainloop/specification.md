# Specification: Main Game Loop Port to Rust (Top-Down)

Plan ID: `PLAN-20260707-MAINLOOP`
Created: 2026-07-07

---

## 1. Purpose / Problem Statement

The UQM codebase is a C-primary / Rust-library hybrid. Today, the C
`main()` function (`sc2/src/uqm.c:233`) owns the entire startup sequence
and launches `Starcon2Main` (`sc2/src/uqm/starcon.c:155`) on a separate
thread to run the game loop. Rust subsystems (22 `USE_RUST_*` flags) are
called bottom-up from C but Rust does not own control flow.

The **bottom-up porting approach hit a structural wall** — the RaceDesc
ABI bug (fixed 2026-07-07) where Rust's struct layout didn't match C's,
causing NULL ship frames. The lesson: whenever Rust and C share a struct
by raw memory offset, layout mismatches cause silent corruption.

This plan establishes a **top-down** approach: Rust becomes the entry
point and owns the activity state machine. C subsystems remain callable
via FFI bridges. The critical innovation is that **every FFI bridge is
proven correct by tests that exercise the real C boundary**, preventing
a repeat of the RaceDesc class of bug.

**Goal:** Move the `Starcon2Main()` game-loop body into Rust, with C
`main()` retaining startup, event pump, and subsystem shutdown.

---

## 2. Architectural Boundaries

### 2.1 Ownership Model

```
┌─────────────────────────────────────────────────────┐
│  C main() — process startup, event pump, shutdown   │
│    ├─ Startup (uqm.c:283-452): options, config,     │
│    │  NETPLAY, graphics, input — all stays in C     │
│    ├─ StartThread(Starcon2Main)                     │
│    ├─ Main-thread pump (uqm.c:456-472): preserved   │
│    └─ Subsystem shutdown (uqm.c:479-507): after     │
│       MainExited — stays in C                       │
│                                                     │
├──────────── FFI Boundary (tested) ──────────────────┤
│                                                     │
│  Rust (Starcon2Main body replacement)               │
│    ├─ rust_game_loop() — runs on Starcon2Main thread│
│    │    ├─ Init: audio, LoadKernel, splash          │
│    │    ├─ ActivityStateMachine (Rust)              │
│    │    │    reads CurrentActivity/NextActivity     │
│    │    │    dispatches to C activity functions     │
│    │    └─ Game-kernel cleanup (starcon.c:313-318)  │
│    └─ sets MainExited for C main() to tear down     │
│                                                     │
├──────────── FFI Boundary (tested) ──────────────────┤
│                                                     │
│  C (subsystem services)                             │
│    ├─ Activity functions: StartGame, Battle, ...    │
│    ├─ Global state: CurrentActivity, NextActivity,  │
│    │  LastActivity, game state (bit-packed)         │
│    └─ Bridge helpers: get_current_activity(), ...   │
└─────────────────────────────────────────────────────┘
```

### 2.2 Key Design Decisions

1. **Rust replaces only the `Starcon2Main` body.** C `main()` owns
   process startup (`uqm.c:283-452`), the main-thread event pump
   (`uqm.c:456-472`: `TFB_ProcessEvents`, `ProcessUtilityKeys`,
   `ProcessThreadLifecycles`, `TFB_FlushGraphics`), and full subsystem
   shutdown (`uqm.c:479-507`). Rust's `rust_game_loop()` replaces the
   `Starcon2Main()` body and runs on the Starcon2Main thread via the
   existing `StartThread` mechanism. **Threading, startup, and
   subsystem shutdown are all preserved in C.** Rust owns only the
   game-loop control flow and the game-kernel cleanup
   (`starcon.c:313-318`).

2. **CurrentActivity, NextActivity, and LastActivity are C globals.**
   `CurrentActivity` is in `GlobData.Game_state` (`globdata.h:930`).
   `LastActivity` is a standalone global (`setup.h:60`). `NextActivity`
   is a standalone global (`save.h:66`). Rust accesses ALL of them
   exclusively through FFI accessor functions — never by direct memory
   offset. **Critical: Rust must re-read `CurrentActivity` via FFI
   after every C activity dispatch call**, because C activity functions
   mutate it.

3. **Activity state machine logic moves to Rust.** The branching logic
   in `Starcon2Main` (encounter/interplanetary/hyperspace dispatch)
   becomes a Rust enum-driven state machine. The C activity functions
   remain in C and are called via FFI. **Post-dispatch flag mutation**
   (clearing `START_ENCOUNTER`, setting `START_INTERPLANETARY`) happens
   in Rust after re-reading the mutated `CurrentActivity`.

4. **C wrappers for static functions, placed in the correct C file.**
   Several C functions the game loop needs are `static` (internal
   linkage) and cannot be called via FFI: `on_battle_frame`
   (starcon.c:80), `BackgroundInitKernel` (starcon.c:92). Wrappers for
   these MUST be added **inside `starcon.c`** (the same translation
   unit where the static functions live), not in a separate `.c` file.
   A new exported function in starcon.c: `uqm_splash_with_bg_init_kernel()`
   calls `SplashScreen(BackgroundInitKernel)`, and
   `uqm_battle_with_frame_callback()` calls `Battle(&on_battle_frame)`.

5. **Named game-state accessors, not byte offsets.** UQM game states
   are **bit-packed** via `getGameState(state, startBit, endBit)`, not
   byte-addressable (see `globdata.h:1001-1015`). Rust accesses game
   state through named C wrappers that use the `GET_GAME_STATE` /
   `SET_GAME_STATE` macros internally — e.g.,
   `uqm_get_chmmr_bomb_state()`, `uqm_set_chmmr_bomb_state(v)`.
   Raw byte-offset accessors would be **unsafe** (wrong bit range).

6. **Rust does only game-kernel cleanup, not subsystem shutdown.**
   Rust's shutdown mirrors `starcon.c:313-318`:
   `UninitGameKernel` → `FreeMasterShipList` → `FreeKernel` →
   `log_showBox` → `MainExited = TRUE`. The full subsystem teardown
   (`uqm.c:479-507`: input, audio, comm, graphics, colormaps, NETPLAY,
   callbacks, alarms, tasks, time, threads, memory, IO,
   `HFree(options.addons)`) is owned by C `main()` after `MainExited`
   is set. **Rust must NOT call subsystem teardown** — that would
   double-free with C `main()`.

---

## 3. Data Contracts and Invariants

### 3.1 Activity Type (C → Rust)

The C `ACTIVITY` type is `UWORD` (uint16_t). Its layout:
- **Low byte (LOBYTE):** activity enum — `SUPER_MELEE(0)`,
  `IN_LAST_BATTLE(1)`, `IN_ENCOUNTER(2)`, `IN_HYPERSPACE(3)`,
  `IN_INTERPLANETARY(4)`, `WON_LAST_BATTLE(5)`, etc.
- **High byte:** flags — `CHECK_PAUSE(0x01)`, `IN_BATTLE(0x02)`,
  `START_ENCOUNTER(0x04)`, `START_INTERPLANETARY(0x08)`,
  `CHECK_LOAD(0x10)`, `CHECK_RESTART(0x20)`, `CHECK_ABORT(0x40)`.

Source: `sc2/src/uqm/globdata.h:893-918`, `sc2/src/libs/compiler.h:58`
(`MAKE_WORD(lo, hi)`, `LOBYTE(x)`).

Rust representation:
```rust
/// 16-bit activity value matching C's ACTIVITY (UWORD).
/// Layout: low byte = activity enum, high byte = flags.
#[repr(transparent)]
pub struct ActivityValue(pub u16);

/// Low-byte activity enum values (matching C).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityKind {
    SuperMelee = 0,
    InLastBattle = 1,
    InEncounter = 2,
    InHyperspace = 3,
    InInterplanetary = 4,
    WonLastBattle = 5,
    InQuasispace = 6,
    InPlanetOrbit = 7,
    InStarbase = 8,
}

/// High-byte activity flags (matching C bit positions).
#[repr(u8)]
pub struct ActivityFlags(pub u8);
impl ActivityFlags {
    pub const CHECK_PAUSE: u8 = 1 << 0;
    pub const IN_BATTLE: u8 = 1 << 1;
    pub const START_ENCOUNTER: u8 = 1 << 2;
    pub const START_INTERPLANETARY: u8 = 1 << 3;
    pub const CHECK_LOAD: u8 = 1 << 4;
    pub const CHECK_RESTART: u8 = 1 << 5;
    pub const CHECK_ABORT: u8 = 1 << 6;
}
```

**Invariant:** `ActivityValue` is always `#[repr(transparent)]` over
`u16` so it matches C's `UWORD` exactly. No struct layout ambiguity.

### 3.2 FFI Bridge Contract

Every C function called from Rust in the init/game-loop sequence gets:
1. An `extern "C"` declaration in `rust/src/game_init/c_extern.rs`
2. A safe Rust wrapper in `rust/src/game_init/bridge.rs`
3. At least one boundary test proving the call works

Every Rust function called from C gets:
1. A `#[no_mangle] pub extern "C"` definition
2. A C prototype in a `.h` file
3. At least one boundary test

### 3.3 Startup Boundary

C `main()` owns the full startup sequence (`uqm.c:283-452`). Rust does
NOT replicate this. Rust's `rust_game_loop()` does only Starcon2Main-
specific init (audio, LoadKernel, splash) before entering the game loop.

---

## 4. Integration Points with Existing Modules

| Integration Point | C Function | Existing Rust Module | Action |
|---|---|---|---|
| Memory init | `mem_init()` | `rust/src/memory.rs` | Already has `rust_mem_init()`; FFI bridge exists |
| Thread init | `InitThreadSystem()` | `rust/src/threading/mod.rs` | FFI bridge exists; will be called from init sequence |
| IO init | `initIO()` | `rust/src/io/` | FFI bridge exists |
| Clock init | `InitTimeSystem()` | `rust/src/time/` | FFI bridge exists |
| Graphics init | `TFB_InitGraphics()` | `rust/src/graphics/` | FFI bridge exists |
| Game state | `GET_GAME_STATE` / `SET_GAME_STATE` | `rust/src/state/` | FFI bridge exists |
| Game kernel | `InitGameKernel()` | `rust/src/game_init/setup.rs` | Partially implemented; needs C-FFI variant |
| Ships | `LoadMasterShipList()` | `rust/src/game_init/master.rs` | Rust wrapper exists |
| CurrentActivity | `GLOBAL(CurrentActivity)` | — | **NEW**: FFI accessor bridge |
| StartGame | `StartGame()` | — | **NEW**: FFI bridge |
| Activity functions | `VisitStarBase()`, etc. | — | **NEW**: FFI bridges |
| Game loop | `Starcon2Main()` | — | **NEW**: Rust reimplementation |

---

## 5. Functional Requirements

### REQ-ML-001: Rust Game Loop Body
Rust provides `rust_game_loop() -> c_int` as the replacement for the
`Starcon2Main()` body. C `main()` runs startup as before, then
`Starcon2Main` delegates to `rust_game_loop()` via a `#ifdef USE_RUST_MAINLOOP`
guard.

**GIVEN** the program starts
**WHEN** C `main()` completes startup and calls `StartThread(Starcon2Main)`
**THEN** `Starcon2Main` calls `rust_game_loop()` which owns the game loop

### REQ-ML-002: Init Sequence
C `main()` owns the full startup sequence (`uqm.c:283-452`). Rust does
Starcon2Main-specific init only (audio, LoadKernel, splash). This phase
is a verification gate confirming C startup works unchanged.

**GIVEN** the program starts
**WHEN** C `main()` runs startup
**THEN** the init sequence runs unchanged in C
**AND** `Starcon2Main` calls `rust_game_loop()` which does its own init

### REQ-ML-003: CurrentActivity FFI Accessors
Rust can read and write `GLOBAL(CurrentActivity)` through FFI accessor
functions, never by direct memory offset.

**GIVEN** C global `GlobData.Game_state.CurrentActivity` is set to value X
**WHEN** Rust calls `get_current_activity()`
**THEN** Rust receives X
**AND** when Rust calls `set_current_activity(Y)`, C reads Y

### REQ-ML-004: Activity State Machine in Rust
The activity dispatch logic from `Starcon2Main` (starcon.c:210-290) is
implemented in Rust as a typed state machine.

**GIVEN** `CurrentActivity` has specific flags set
**WHEN** the Rust state machine evaluates the next activity
**THEN** it dispatches to the correct C activity function:
- `START_ENCOUNTER` flag or `CHMMR_BOMB_STATE == 2` → `VisitStarBase` or `RaceCommunication`
- `START_INTERPLANETARY` flag → `ExploreSolarSys`
- Otherwise → `Battle`

### REQ-ML-005: FFI Boundary Test Suite
Every FFI bridge in the init/loop sequence has integration tests proving
data crosses the boundary correctly.

**GIVEN** an FFI bridge function exists
**WHEN** the boundary test suite runs
**THEN** it proves: (a) Rust can call the C function, (b) side effects are
observable, (c) return values are correct, (d) shared global state
round-trips

### REQ-ML-006: (Removed — merged with REQ-ML-002)
C `main()` owns startup. No Rust init-sequence ordering to test.

### REQ-ML-007: Game Loop Outer/Inner Structure
The two-level loop from `Starcon2Main` is preserved: outer loop
(`while StartGame()`) for new-game/load-game, inner loop (`do...while
!CHECK_ABORT`) for activity state machine.

**GIVEN** `StartGame()` returns true (player starts/loads a game)
**WHEN** the inner loop runs
**THEN** activity functions dispatch until `CHECK_ABORT` is set

### REQ-ML-008: Game-Kernel Cleanup
After the game loop exits, Rust calls only the game-kernel cleanup from
`starcon.c:313-318`: `UninitGameKernel` → `FreeMasterShipList` →
`FreeKernel` → `log_showBox` → `MainExited = TRUE`.
The full subsystem teardown (`uqm.c:479-507`: input, audio, comm,
graphics, colormaps, NETPLAY, callbacks, alarms, tasks, time, threads,
memory, IO, `HFree(options.addons)`) is owned by C `main()` after
`MainExited` is set. **Rust must NOT call subsystem teardown.**

### REQ-ML-009: C-to-Rust Callback for Activity Dispatch
C can call back into Rust for activity dispatch decisions, establishing
bidirectional FFI.

**GIVEN** C needs to evaluate the next activity
**WHEN** C calls `rust_dispatch_activity()`
**THEN** Rust evaluates `CurrentActivity` and calls the appropriate C
activity function

### REQ-ML-010: Game State Round-Trip
Game state flags (e.g., `CHMMR_BOMB_STATE`) read from Rust match what C
wrote, and vice versa.

**GIVEN** C sets `CHMMR_BOMB_STATE` to 2
**WHEN** Rust reads it via FFI
**THEN** Rust sees 2
**AND** when Rust sets it to 0, C reads 0

---

## 6. Error / Edge Case Expectations

- **Init function failure:** If any init function fails (e.g.,
  `LoadKernel` returns false), Rust logs a fatal error and exits with
  `EXIT_FAILURE`, matching C's behavior (starcon.c:175-185).
- **Activity state corruption:** If `CurrentActivity` has unexpected
  flag combinations, the state machine falls through to the default
  (Battle) case, matching C's `else` branch.
- **Game loop exit:** When `CHECK_ABORT` is set (by `SignalStopMainThread`
  or natural game exit), the inner loop exits, cleanup runs, outer loop
  re-evaluates `StartGame()`.
- **Double-init protection:** Init functions are idempotent or return
  errors on double-init (matching existing C behavior).

---

## 7. Non-Functional Requirements

### Reliability
- No `unwrap()`/`expect()` in FFI bridge code (project standard).
- All FFI calls return `Result<T, MainLoopError>` at the safe wrapper level.
- `unsafe` is isolated to FFI declaration and call sites only.

### Performance
- Init sequence executes in the same number of function calls as C — no
  redundant indirection.
- Game loop has no per-frame allocation; state machine uses stack-only types.

### Operability
- Init sequence failures produce context-rich error messages identifying
  which step failed.
- Logging integrates with existing `log_add` system via bridge.

---

## 8. Testability Requirements

### Boundary Tests (Real C Functions, Not Mocks)
1. `test_rust_calls_c_mem_init` — calls `mem_init()` via FFI, verifies
   memory subsystem is active.
2. `test_current_activity_round_trip` — sets activity from Rust, reads
   from C, sets from C, reads from Rust.
3. `test_game_state_chmmr_bomb_round_trip` — sets `CHMMR_BOMB_STATE` from
   Rust, reads via C `GET_GAME_STATE`, and reverse.
4. `test_rust_game_loop_symbol_exists` — verifies linkage to C binary
   completion.
5. `test_activity_dispatch_encounter` — sets `START_ENCOUNTER` flag,
   verifies state machine dispatches to encounter branch.
6. `test_activity_dispatch_interplanetary` — sets `START_INTERPLANETARY`,
   verifies interplanetary dispatch.
7. `test_c_calls_rust_dispatch` — C callback into Rust activity dispatch.
8. `test_load_kernel_failure_handling` — verifies fatal error path when
   `LoadKernel` fails.

### Test Infrastructure
- A new C test bridge file (`rust_test_bridge.c`) compiled by
  `rust/build.rs` into a test-only static library.
- Tests use `#[link(name = "uqm_test_bridge")]` to access C test helpers.
- Tests that require full C initialization are gated behind a feature
  flag `c_integration` (not all CI environments have SDL).

---

## 9. Scope Boundaries

### In Scope
- Rust game-loop body (`rust_game_loop`) — replaces Starcon2Main body only
- Activity type definitions (`ActivityValue`, `ActivityKind`, `ActivityFlags`)
- CurrentActivity, NextActivity, LastActivity FFI accessors
- Starcon2Main-specific init (audio, LoadKernel, splash) from inside loop
- Activity state machine
- Game loop reimplementation (outer + inner loop)
- Game-kernel cleanup (starcon.c:313-318 only)
- FFI boundary test suite
- C-side wrapper functions for static functions + accessors

### Out of Scope (Deferred)
- **C main() startup sequence** (`uqm.c:283-452`) — stays in C entirely.
  Rust does not wrap or replicate option parsing, config loading,
  graphics init, NETPLAY init, etc.
- **Full subsystem shutdown** (`uqm.c:479-507`) — stays in C `main()`.
  Rust does only `starcon.c:313-318` game-kernel cleanup.
- **Main-thread event pump** (`uqm.c:456-472`) — stays in C, runs
  concurrently as before. The threading model is preserved.
- Porting individual activity function bodies (`VisitStarBase`,
  `RaceCommunication`, `ExploreSolarSys`, `Battle`) to Rust — these stay
  in C and are called via FFI.
- Removing the threading system entirely — the Starcon2Main thread and
  main-thread pump both remain. Rust runs on the Starcon2Main thread.
- Netplay integration (the `#ifdef NETPLAY` blocks are preserved but not
  Rust-driven).
- CLI option parsing (`parseOptions`) — stays in C; Rust never calls it.

---

## 10. RaceDesc ABI Bug — Defensive Requirements

The RaceDesc bug (Rust struct 288 bytes vs C struct 384 bytes, different
field ordering) caused NULL ship frames. This plan prevents recurrence by:

1. **No shared structs by offset.** `CurrentActivity` and `LastActivity`
   are accessed only through function calls (`get_current_activity()`,
   etc.), never by reading `GlobData` memory directly from Rust.
2. **`#[repr(transparent)]` for the `ACTIVITY` scalar only.**
   `ActivityValue` is `#[repr(transparent)]` over `u16` — this prevents
   layout ambiguity for that one scalar (C `typedef UWORD ACTIVITY`).
   It does NOT prevent broader FFI risks (wrong callback types, wrong
   BOOLEAN definitions, wrong options_struct layout). The broader
   defense is **semantic C wrappers and boundary tests**.
3. **Named game-state accessors, not byte offsets.** Game state is
   bit-packed (`getGameState(state, startBit, endBit)`). Each state
   gets a named C wrapper using `GET_GAME_STATE`/`SET_GAME_STATE`
   macros. Raw byte-offset access is forbidden.
4. **Layout assertions.** C-side `_Static_assert(sizeof(UWORD) == 2)`
   in the wrapper header.
5. **Round-trip tests.** Every shared value is tested in both directions
   (Rust writes → C reads, C writes → Rust reads).
6. **No Rust ownership of C structs.** Rust never holds a pointer to a
   C struct with layout Rust assumes. All struct access goes through
   function-call accessors that C implements.
