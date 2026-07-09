# Plan: Restart/Menu System Port to Rust

Plan ID: `PLAN-20260707-RESTARTMENU`
Generated: 2026-07-07

## Scope

Port the UQM restart/menu system from C to Rust. This is the next layer
down from the already-ported main game loop (`rust_game_loop`). The
mainloop's `GameLoopOps::start_game()` currently calls the C
`StartGame()` function via FFI. This plan replaces that C function (and
its call tree) with Rust.

### C source files in scope

| File | Lines | Functions | Status |
|------|-------|-----------|--------|
| `sc2/src/uqm/restart.c` | 413 | `StartGame`, `TryStartGame`, `DoRestart`, `RestartMenu`, `DrawRestartMenu`, `DrawRestartMenuGraphic` | **In scope** |
| `sc2/src/uqm/restart.h` | 28 | `StartGame` prototype | **In scope** |
| `sc2/src/uqm/menustat.h` | 92 | `MENU_STATE` struct | **In scope (type only)** |
| `sc2/src/uqm/menu.c` | 603 | Generic menu navigation | **Out of scope** (retained as C dependency) |
| `sc2/src/uqm/gameinp.c` | `DoInput()` | Input pump loop | **Out of scope** (retained as C, calls Rust callback) |

### Relationship to existing work

```
starcon.c / rust_game_loop()
  └─ while ops.start_game()   ← currently calls C StartGame() via FFI
       └─ rust_start_game()   ← THIS PLAN replaces it
            ├─ try_start_game()
            │   └─ restart_menu()  ← menu loop
            │       └─ do_restart()  ← per-frame input handler
            └─ (game starts → inner activity loop)
```

## Architectural Approach

Following the established mainloop pattern:

1. **Pure logic extracted to stateless Rust functions** — menu state
   transitions, wrap-around logic, activity-flag manipulation. These
   take values as parameters, not globals. Fully unit-testable (Tier 1).

2. **Trait-based FFI abstraction** — a `RestartMenuOps` trait (like
   `GameLoopOps`) abstracts all C-side operations: input reading,
   graphics/music/flash, activity globals, game-state predicates.
   Production uses `CffiOps`; tests use mock implementations.

3. **Three-tier testing** — same as mainloop:
   - Tier 1: pure Rust unit tests (state machine, menu transitions)
   - Tier 2: C ABI shim round-trip tests (activity/game-state accessors)
   - Tier 3: external process tests (full binary boot to menu)

4. **`#ifdef USE_RUST_RESTART` guard** — when defined, `StartGame()` in
   `restart.c` (NOT starcon.c) delegates to `rust_start_game()`. When
   undefined, the original C implementation runs unchanged.

## Key Design Decisions

### What stays in C (not ported)

The in-game menu system (`DoMenuChooser`, `DrawMenuStateStrings`, and the
`PM_*` state machine in `menu.c`) is used **throughout the game** —
starmap, encounter, outfit, shipyard, settings — not just at the restart
menu. Porting it requires porting or FFI-wrapping dozens of call sites.
This plan ports only the **restart menu** path and defers the generic
`menu.c` in-game menu system to a future plan. `menu.c` is retained as a
C dependency (not in porting scope).

`DoInput()` in `gameinp.c` is also kept in C. It handles `Async_process`,
`TaskSwitch`, `UpdateInputState`, menu sound playback, and final
`FlushInput`. Rust provides an `InputFunc` callback via the `privData`
field on `MENU_STATE`.

### Static helper handling

`DrawRestartMenuGraphic()` and `DrawRestartMenu()` are `static` in
restart.c. When `USE_RUST_RESTART` is defined, the `static` keyword is
removed (making them file-visible) so `rust_bridge_restart.c` wrappers
can call them. This avoids reimplementing drawing logic in Rust.

### MENU_STATE + privData bridge

The C `MENU_STATE` struct has a `void *privData` field. The Rust port
uses this to pass a pointer to a `RestartMenuState` struct that holds:
- `item: RestartMenuItem` (selected menu item)
- `initialized: bool`
- `music: MusicHandle` (opaque)
- `flash_context: FlashContextPtr` (opaque)
- `last_input: u32` (TimeCount, replaces C static)
- `timeout: u32` (replaces C static)

Rust allocates the state before `DoInput()`, sets `pMS->privData`, and
frees it after. The callback reads `privData` each frame.

### DoInput integration

`DoInput(pMS, TRUE)` is the C input-pumping loop that repeatedly calls
`pMS->InputFunc` (which is `DoRestart` for the restart menu). In Rust,
we restructure this as an explicit loop that calls `do_restart_frame()`
each iteration, with `DoInput` itself remaining in C (it handles key
repeat timing, input flushing, and task yielding).

**Decision**: Keep `DoInput` in C. Call it from Rust with a Rust-managed
callback. The callback approach: Rust provides a `rust_do_restart_frame`
function that C's `DoInput` calls via the `InputFunc` function pointer.

### Activity timeout behavior

The restart menu has an inactivity timeout (120 seconds with music, 20
without) that sets `CurrentActivity = (ACTIVITY)~0` to signal "timed
out, go to splash/credits." This is pure logic (compare timestamps) but
requires `GetTimeCounter()` access.

## Phase List

| Phase | Title | Type |
|------:|-------|------|
| P01 | Domain Analysis | Analysis |
| P01a | Analysis Verification | Verification |
| P02 | Restart Menu State Types | TDD+Impl |
| P02a | Types Verification | Verification |
| P03 | Menu Navigation Logic (pure functions) | TDD+Impl |
| P03a | Navigation Logic Verification | Verification |
| P04a | C Wrappers: Game-State Accessors | Impl (C) |
| P04b | C Wrappers: Input/Time/Sleep | Impl (C) |
| P04c | C Wrappers: Music/Flash | Impl (C) |
| P04d | C Wrappers: Graphics/Rendering | Impl (C) |
| P04e | C Wrappers: Lifecycle (Melee/Setup/Credits) | Impl (C) |
| P05 | FFI Externs, Safe Wrappers, RestartMenuOps Trait | TDD+Impl |
| P05a | Trait Verification | Verification |
| P06 | DoRestart Frame Logic (InputFunc callback) | TDD+Impl |
| P06a | DoRestart Verification | Verification |
| P07 | RestartMenu Orchestration | TDD+Impl |
| P07a | RestartMenu Verification | Verification |
| P08 | TryStartGame Loop | TDD+Impl |
| P08a | TryStartGame Verification | Verification |
| P09 | StartGame Outer Loop | TDD+Impl |
| P09a | StartGame Verification | Verification |
| P10 | C-to-Rust Wiring + Build Integration | Impl (C + build) |
| P10a | Wiring Verification | Verification |
| P11 | End-to-End Integration | Verification (manual) |

## Requirements → Phase Mapping

| Requirement | Title | Phase |
|-------------|-------|-------|
| REQ-RM-001 | Restart menu state types | P03 |
| REQ-RM-002 | Menu navigation wrap-around | P04 |
| REQ-RM-003 | C wrapper accessors for game state | P05 |
| REQ-RM-004 | FFI safe wrappers | P06 |
| REQ-RM-005 | DoRestart per-frame logic | P07 |
| REQ-RM-006 | RestartMenu lifecycle | P08 |
| REQ-RM-007 | TryStartGame retry loop | P09 |
| REQ-RM-008 | StartGame outer loop + player control setup | P10 |
| REQ-RM-009 | C dispatch guard wiring | P11 |
| REQ-RM-010 | Build system flag | P12 |

## Definition of Done

- [ ] All REQ-RM-* requirements have passing tests
- [ ] `rust_start_game()` replaces `StartGame()` body when
      `USE_RUST_RESTART` is defined
- [ ] C `StartGame()` still works when `USE_RUST_RESTART` is undefined
- [ ] No `unwrap()`/`expect()` in production paths
- [ ] `cargo fmt`, `cargo clippy -D warnings`, `cargo test` all green
- [ ] Zero `TODO`/`FIXME`/`HACK` placeholders in implementation
- [ ] Binary boots to main menu and accepts input (P13 verification)
