# Deepthinker Review: Issues and Resolutions

Plan: `PLAN-20260707-RESTARTMENU`
Reviewer: deepthinker
Status: 25 issues found (5 CRITICAL, 13 MAJOR, 7 MINOR)

---

## CRITICAL Issues

### C1: LastActivity set incorrectly for New/Load Game
**Problem:** Pseudocode says `set_last_activity(IN_INTERPLANETARY with CHECK_LOAD...)` but C does:
- `LOAD_SAVED_GAME`: `LastActivity = CHECK_LOAD` (no IN_INTERPLANETARY)
- `START_NEW_GAME`: `LastActivity = CHECK_LOAD | CHECK_RESTART`

**Resolution:** Fixed in `02-pseudocode.md`. Tests must assert exact values.

### C2: DoInput callback bridge underspecified
**Problem:** The callback bridge doesn't define how Rust state maps to C MENU_STATE. A stateless `rust_do_restart_frame()` is reentrancy risk.

**Resolution:** Use `MENU_STATE.privData` to store a pointer to a Rust-owned `RestartMenuState`:
- C wrapper signature: `BOOLEAN rust_restart_input_func(MENU_STATE *pMS)`
- It reads `pMS->privData` (set by Rust before DoInput) to get the state pointer
- State is allocated by Rust before DoInput, freed after
- Tests must verify state persistence across callback invocations

### C3: Missing SplashScreen + global array assignment
**Problem:** `start_game_impl` calls `ops.splash_screen()` and `ops.assign_global_arrays()` but neither is in the trait or C wrapper list.

**Resolution:** Added to trait and P04 wrapper list. Array assignment is a C-side wrapper (`uqm_assign_star_planet_globals`) because it touches `extern` symbols.

### C4: Static C helpers cannot be called from rust_bridge_restart.c
**Problem:** `DrawRestartMenuGraphic()` and `DrawRestartMenu()` are `static` in restart.c. A separate .c file cannot call them.

**Resolution:** Two-layer approach:
1. Make `DrawRestartMenuGraphic` and `DrawRestartMenu` non-static (remove `static` keyword) in restart.c when `USE_RUST_RESTART` is defined.
2. Add wrapper prototypes in `rust_bridge_restart.h`.
3. C wrappers in `rust_bridge_restart.c` call the now-visible functions.

### C5: FFI surface incomplete
**Problem:** Many missing trait methods, C wrappers, and constants.

**Resolution:** Complete line-by-line FFI inventory built in revised `01-analysis.md`. All macros, constants, and function calls catalogued with classification (direct extern / C wrapper / Rust constant / static helper).

---

## MAJOR Issues

### M6: wrapping_sub for timeout
**Resolution:** `check_timeout` uses `now.wrapping_sub(last_input) > timeout`. Added wraparound tests.

### M7: Trait too broad
**Resolution:** Trait split into concrete methods, each tied to a C source line. No `...` placeholders. High-level helpers like `init_menu_graphics` replaced with explicit sequences of primitive calls.

### M8: Missing branch-priority tests
**Resolution:** Added tests for:
- First call skips input processing
- Select priority over up/down/mouse/timeout
- Up wins over down (C checks up first)
- Left/right reset timeout without redraw
- Timeout path: FadeMusic → StopMusic → FadeMusic → set activity

### M9: CurState persistence across Super Melee retries
**Resolution:** Added test: select SuperMelee → Melee returns → re-enter menu preserves selected item, forces reinit.

### M10: Menu scope contradictory
**Resolution:** Overview corrected. menu.c removed from "in scope", marked as "out of porting scope, retained as C dependency."

### M11: Build integration incomplete
**Resolution:** P10 expanded to inspect and mirror the successful mainloop/ships build pattern. Explicit steps for build.vars.in, Makeinfo, config files.

### M12: C macro hazards
**Resolution:** Every macro catalogued in FFI inventory. Each either mirrored as Rust constant (with validation) or exposed through C wrapper.

### M13: BOOLEAN/FFI return types
**Resolution:** All callbacks use `CBoolean = c_int` (matching existing mainloop pattern). Rust `bool` never used directly in extern ABI.

### M14: Handle failure behavior
**Resolution:** Opaque handle types defined: `MusicHandle`, `FrameHandle`, `FlashContextPtr`. Null handles tolerated per C behavior (e.g., `hMusic == 0` path).

### M15: Cleanup ordering tests
**Resolution:** Added explicit cleanup-sequence tests for all exit paths (normal, SuperMelee, timeout, quit, post-setup).

### M16: Super Melee return path tests
**Resolution:** Added test: SuperMelee exit still does fade/flush/seed before returning false (only timeout and quit have early returns).

### M17: P04 too large
**Resolution:** Split P04 into P04a-P04e (game-state, input/time, music/flash, graphics, lifecycle). P03 (pure logic) is NOT blocked by any P04 sub-phase.

### M18: Impure logic called "pure"
**Resolution:** Analysis reworded. Loop orchestration classified as "extractable reducer decisions behind traits", not "pure logic."

---

## MINOR Issues (M19-M25)

All addressed:
- M19: Re-read CurrentActivity after C calls (SetupMenu, Melee, etc.)
- M20: DoInput analysis expanded with Async_process, TaskSwitch, inputCallback, final FlushInput, menu sounds
- M21: COUNT test paired with behavioral tests
- M22: P11 marked as manual verification if no automation available
- M23: ExitRequested/pause risk note added
- M24: Overview corrected — StartGame is in restart.c, not starcon.c
- M25: menu.c symbol visibility claims removed (out of scope)
