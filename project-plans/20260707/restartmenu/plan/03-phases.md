# Phase 03: Implementation Phases

## Plan ID
`PLAN-20260707-RESTARTMENU.P03`

## TDD Discipline

Every implementation phase follows RED → GREEN → REFACTOR:
1. Write failing tests first (RED)
2. Write minimal implementation to pass (GREEN)
3. Refactor for clarity (REFACTOR)

Each phase has an implementation sub-phase (P0x) and a verification
sub-phase (P0xa) reviewed by deepthinker.

---

## P01: Domain Analysis (this document set)
- Status: analysis complete
- Output: 01-analysis.md, 02-pseudocode.md, 03-phases.md

## P01a: Analysis Verification
- deepthinker reviews analysis for completeness/correctness

---

## P02: Restart Menu Types (TDD)
**Requirement:** REQ-RM-001

**RED:** Write tests for:
- `RestartMenuItem` enum has exactly 5 variants with correct u8 values
- `RestartMenuItem::COUNT == 5`
- `SelectionResult` variants carry correct data
- `MenuInputState` Default is all-false

**GREEN:** Create `rust/src/mainloop/restart_menu/types.rs`:
- `RestartMenuItem` enum (#[repr(u8)])
- `SelectionResult` enum
- `MenuInputState` struct

**Files:**
- `rust/src/mainloop/restart_menu/mod.rs` (module declaration)
- `rust/src/mainloop/restart_menu/types.rs`
- `rust/src/lib.rs` (add `pub mod restart_menu` if new top-level, or nest under mainloop)

---

## P02a: Types Verification
- deepthinker verifies types match restart.c enum values

---

## P03: Menu Navigation Logic (TDD — pure functions)
**Requirement:** REQ-RM-002

**RED:** Write tests for `menu_logic.rs`:
- `navigate_up(NewGame)` → `Quit` (wrap)
- `navigate_up(LoadGame)` → `NewGame`
- `navigate_up(Quit)` → `Setup`
- `navigate_down(Quit)` → `NewGame` (wrap)
- `navigate_down(NewGame)` → `LoadGame`
- `navigate_down(Setup)` → `Quit`
- `apply_selection(NewGame)` → `StartGame{new_game: true}`
- `apply_selection(LoadGame)` → `StartGame{new_game: false}`
- `apply_selection(SuperMelee)` → `SuperMelee`
- `apply_selection(Setup)` → `StayInMenu`
- `apply_selection(Quit)` → `Quit`
- `check_timeout` with various deltas

**GREEN:** Create `rust/src/mainloop/restart_menu/menu_logic.rs`:
- `navigate_up(current) -> RestartMenuItem`
- `navigate_down(current) -> RestartMenuItem`
- `apply_selection(item) -> SelectionResult`
- `check_timeout(now, last_input, timeout) -> bool`

**These are 100% pure functions.** No FFI, no globals. Fully testable.

---

## P03a: Navigation Logic Verification
- deepthinker verifies against restart.c:183-202 (up/down) and 144-177 (selection)

---

## P04a: C Wrappers — Game-State Accessors
**Requirement:** REQ-RM-003 (game-state subset)

**C wrappers:**
- `uqm_get_game_state(offset)`, `uqm_set_game_state(offset, value)`
- `uqm_get_crew_enlisted()`
- `uqm_get_last_activity()`, `uqm_set_last_activity(val)`
- `uqm_set_next_activity(val)`
- `uqm_set_player_control(player, control)`

**Rust externs:** Matching declarations in `c_extern.rs`

**Tier-2 tests:** Round-trip get/set for each accessor

---

## P04b: C Wrappers — Input/Time/Sleep
**Requirement:** REQ-RM-003 (input subset)

**C wrappers:**
- `uqm_get_pulsed_key(key_index)` → reads `PulsedInputState.menu[key_index]`
- `uqm_get_mouse_button_down()`
- `uqm_get_time_counter()`
- `uqm_set_game_paused(val)`
- `uqm_sleep_thread_until(time)`, `uqm_sleep_thread(duration)`

**Constants mirrored in Rust:**
- `KEY_MENU_SELECT`, `KEY_MENU_UP`, `KEY_MENU_DOWN`, `KEY_MENU_LEFT`, `KEY_MENU_RIGHT` (from controls.h)
- `ONE_SECOND` (from clock.h)

---

## P04c: C Wrappers — Music/Flash
**Requirement:** REQ-RM-003 (music/flash subset)

**C wrappers:**
- `uqm_load_music(ref)`, `uqm_play_music(handle, loop_, vol)`, `uqm_stop_music()`, `uqm_destroy_music(handle)`, `uqm_fade_music(volume, duration)`
- `uqm_flash_create_overlay()`, `uqm_flash_process(ctx)`, `uqm_flash_pause(ctx)`, `uqm_flash_continue(ctx)`, `uqm_flash_start(ctx)`, `uqm_flash_terminate(ctx)`
- `uqm_flash_set_merge_factors(ctx, a, b, c)`, `uqm_flash_set_speed(ctx, a, b, c, d)`, `uqm_flash_set_frame_time(ctx, t)`, `uqm_flash_set_state(ctx, state, t)`, `uqm_flash_set_overlay(ctx, origin, frame)`

**Constants mirrored in Rust:**
- `MAINMENU_MUSIC`, `NORMAL_VOLUME`
- `FlashState_fadeIn`
- `FadeAllToColor`, `FadeAllToBlack`, `FadeAllToWhite`

**Handle types:** `MusicHandle` (typedef to pointer), `FlashContextPtr` (opaque pointer)

---

## P04d: C Wrappers — Graphics/Rendering
**Requirement:** REQ-RM-003 (graphics subset)

**C wrappers:**
- `uqm_set_context(ctx)`, `uqm_batch_graphics()`, `uqm_unbatch_graphics()`
- `uqm_clear_drawable()`, `uqm_flush_color_xforms()`, `uqm_screen_transition(a, b)`
- `uqm_capture_drawable(load_result)`, `uqm_load_graphic(ref)`, `uqm_destroy_drawable(handle)`, `uqm_release_drawable(handle)`, `uqm_set_abs_frame_index(frame, idx)`
- `uqm_set_transition_source(ptr)`, `uqm_set_menu_sounds(s0, s1)`, `uqm_set_default_menu_repeat_delay()`
- `uqm_draw_restart_menu_graphic(pMS)` — wrapper for **de-staticized** `DrawRestartMenuGraphic`
- `uqm_draw_restart_menu(pMS, state, frame)` — wrapper for **de-staticized** `DrawRestartMenu`
- `uqm_seed_random_numbers()`
- `uqm_splash_screen_no_callback()`
- `uqm_assign_star_planet_globals()`
- `uqm_do_popup_window(msg_id)`

**Requires:** Remove `static` from `DrawRestartMenuGraphic` and `DrawRestartMenu` in restart.c when `USE_RUST_RESTART` defined.

**Constants mirrored in Rust:**
- `RESTART_PMAP_ANIM`, `SCREEN_WIDTH`, `SCREEN_HEIGHT`
- `MAINMENU_STRING_BASE + 54` (mouse-not-supported message ID)

---

## P04e: C Wrappers — Lifecycle Subsystems
**Requirement:** REQ-RM-003 (lifecycle subset)

**C wrappers:**
- `uqm_melee()` → calls `Melee()`
- `uqm_setup_menu()` → calls `SetupMenu()`
- `uqm_free_game_data()` → calls `FreeGameData()`
- `uqm_introduction()` → calls `Introduction()`
- `uqm_credits(victory)` → calls `Credits(victory)`
- `uqm_victory()` → calls `Victory()`
- `uqm_reinit_race_queues()` → calls `ReinitQueue(&race_q[0])`, `ReinitQueue(&race_q[1])`
- `uqm_do_input(pMS, reset)` → calls `DoInput(pMS, reset)`
- `uqm_fade_screen(mode, duration)` → calls `FadeScreen(mode, duration)`

**C callback bridge:**
- `rust_do_restart_frame_c(pMS)` → `#[no_mangle] extern "C" fn` that reads `pMS->privData`, calls Rust `do_restart_frame(state, pMS)`
- C wrapper `uqm_set_rust_input_func(pMS)` sets `pMS->InputFunc` to the C-side trampoline that calls `rust_do_restart_frame_c`

---

## P05: FFI Externs, Safe Wrappers, RestartMenuOps Trait (TDD)
**Requirement:** REQ-RM-004, REQ-RM-005

**RED:** Write Tier-1 tests using MockOps:
- Test that RestartMenuOps trait has all required methods
- Test MockOps can control return values for activity, input, time, game-state

**GREEN:** Create `rust/src/mainloop/restart_menu/c_extern.rs`:
- All extern "C" declarations matching P04a-P04e C wrappers
- Constants mirrored from C (KEY_MENU_*, ONE_SECOND, MAINMENU_MUSIC, etc.)

Create `rust/src/mainloop/restart_menu/restart_ops.rs`:
- `RestartMenuOps` trait — each method maps 1:1 to a C source line
- Includes: activity accessors, input, time, game-state, music, flash,
  graphics, lifecycle, player-control, game-paused
- No `...` placeholders — every method has a concrete signature

Create `rust/src/mainloop/restart_menu/ffi_bridge.rs`:
- `#[cfg(not(test))]` CffiOps struct implementing RestartMenuOps with real FFI

---

## P05a: Trait Verification
- deepthinker verifies trait covers ALL C calls in DoRestart (restart.c:99-250)
  AND RestartMenu (restart.c:252-337) AND TryStartGame (restart.c:339-371)
  AND StartGame (restart.c:373-412)
- Verify no FFI call is missing from the trait

---

## P06: DoRestart Frame Logic (TDD)
**Requirement:** REQ-RM-006

**RED:** Write Tier-1 tests with MockOps:
- First call initializes: loads music, creates flash, draws menu, sets initialized, does NOT process input
- First call returns true even if input is already pressed
- `GamePaused` is set to false every frame before other logic
- `Flash_process` called only when already initialized
- CHECK_ABORT on subsequent call → returns false
- Select priority over up/down/left/right/mouse/timeout (checked first)
- KEY_MENU_SELECT + NewGame → sets `LastActivity = CHECK_LOAD | CHECK_RESTART`, `CurrentActivity = IN_INTERPLANETARY`, returns false
- KEY_MENU_SELECT + LoadGame → sets `LastActivity = CHECK_LOAD` (NO IN_INTERPLANETARY in LastActivity), `CurrentActivity = IN_INTERPLANETARY`, returns false
- KEY_MENU_SELECT + SuperMelee → sets CurrentActivity=SUPER_MELEE, returns false
- KEY_MENU_SELECT + Setup → calls SetupMenu, redraws, returns true (stays in menu)
- KEY_MENU_SELECT + Quit → sets CHECK_ABORT, returns false
- Both up+down pressed → up wins (C checks up first, restart.c:189)
- KEY_MENU_UP wraps NewGame(0)→Quit(4)
- KEY_MENU_DOWN wraps Quit(4)→NewGame(0)
- Left/right count as input and reset timeout, do NOT redraw
- Mouse popup: pauses flash, redraws graphic+menu, transitions, continues flash, resets LastInputTime
- Timeout: calls FadeMusic(0,ONE_SECOND), StopMusic, FadeMusic(NORMAL_VOLUME,0), sets CurrentActivity=~0, returns false
- Timeout wrapping: counter wraps around u32 boundary correctly
- No input, not timed out → returns true
- Cleanup runs after DoInput regardless of exit path (verified in P07)

**GREEN:** Create `rust/src/mainloop/restart_menu/do_restart.rs`:
- `do_restart_frame<O: RestartMenuOps>(ops, state) -> bool`
- Uses `menu_logic` pure functions for navigation/selection

**Also:** Create C callback bridge in `rust_bridge_restart.c`:
- `rust_do_restart_frame()` — #[no_mangle] extern "C" entry called by C InputFunc wrapper

---

## P06a: DoRestart Verification
- deepthinker verifies against restart.c:99-250 line-by-line

---

## P07: RestartMenu Orchestration (TDD)
**Requirement:** REQ-RM-007

**RED:** Write Tier-1 tests with MockOps:
- Normal path: reinit queues, fade, load graphic, DoInput, cleanup
- Utwig bomb path: white flash, 1/8s timeout
- Victory path: WonLastBattle → Victory + Credits + FreeGameData
- Timeout exit: returns false (early return at restart.c:323)
- Quit exit: returns false (early return at restart.c:326)
- SuperMelee exit: returns false BUT still performs fade/flush/seed (NOT early return — goes through restart.c:329-336)
- Normal game exit: returns true (goes through fade/flush/seed, returns LOBYTE != SUPER_MELEE)
- Cleanup-sequence test: StopMusic → DestroyMusic(if nonzero) → Flash_terminate → DestroyDrawable(ReleaseDrawable) in correct order
- Cleanup runs on ALL exit paths (not just normal)
- CurState preservation: selecting SuperMelee → Melee returns → menu reopens on same item

**GREEN:** Create `rust/src/mainloop/restart_menu/restart_menu.rs`:
- `restart_menu_impl<O: RestartMenuOps>(ops, state) -> bool`

---

## P07a: RestartMenu Verification
- deepthinker verifies against restart.c:252-337

---

## P08: TryStartGame Loop (TDD)
**Requirement:** REQ-RM-008

**RED:** Write Tier-1 tests with MockOps:
- RestartMenu returns true immediately → TryStartGame returns true
- RestartMenu returns false, SuperMelee → calls Melee, resets only Initialized (NOT CurState), retries, then true
- RestartMenu returns false, timeout → returns false
- RestartMenu returns false, CHECK_ABORT → returns false
- CurState preserved across SuperMelee retry: menu reopens on SuperMelee item

**GREEN:** Create `rust/src/mainloop/restart_menu/try_start_game.rs`:
- `try_start_game_impl<O: RestartMenuOps>(ops) -> bool`

---

## P08a: TryStartGame Verification
- deepthinker verifies against restart.c:339-371

---

## P09: StartGame Outer Loop (TDD)
**Requirement:** REQ-RM-009

**RED:** Write Tier-1 tests with MockOps:
- TryStartGame true, no CHECK_RESTART → sets PlayerControl, returns true
- TryStartGame true, CHECK_RESTART → calls Introduction, sets PlayerControl
- TryStartGame false, timeout → calls SplashScreen + Credits, retries
- TryStartGame false, CHECK_ABORT → returns false

**GREEN:** Create `rust/src/mainloop/restart_menu/start_game.rs`:
- `start_game_impl<O: RestartMenuOps>(ops) -> bool`
- `rust_start_game()` — #[no_mangle] extern "C" entry point

---

## P09a: StartGame Verification
- deepthinker verifies against restart.c:373-412

---

## P10: C Wiring + Build Integration
**Requirement:** REQ-RM-010

**Changes:**
1. `sc2/src/uqm/restart.c`: Add `#ifdef USE_RUST_RESTART` at top of `StartGame()`:
   ```c
   #ifdef USE_RUST_RESTART
       extern BOOLEAN rust_start_game(void);
       return rust_start_game();
   #else
       // ... original body ...
   #endif
   ```
   Also: When `USE_RUST_RESTART`, remove `static` from `DrawRestartMenuGraphic` and `DrawRestartMenu`.

2. `sc2/src/uqm/Makeinfo`: Add `rust_bridge_restart.c` to `uqm_CFILES`, `.h` to `HFILES`.

3. **Build vars integration** (mirror successful mainloop pattern):
   - `sc2/build.vars.in`: Add `USE_RUST_RESTART` template variables
   - `sc2/build/unix/config_functions` or equivalent: Add config option
   - Verify pattern matches existing `USE_RUST_MAINLOOP`, `USE_RUST_SHIPS` entries

4. **Linkage verification:**
   - Build with `./build.sh uqm`
   - `cargo test --lib` passes
   - `nm uqm | grep rust_start_game` shows symbol
   - `nm uqm | grep rust_do_restart_frame` shows symbol
   - Build with `USE_RUST_RESTART` disabled still works (backward compat)

**WARNING:** Previous C+Rust link work required entries in build.vars.in template AND compression libs (-llzma -lbz2) in LDFLAGS. Verify these are already present from mainloop integration.

---

## P10a: Wiring Verification
- deepthinker verifies wiring pattern matches mainloop approach

---

## P11: End-to-End Integration
**Requirement:** All REQ-RM-*

**Tests:**
1. Build with `USE_RUST_RESTART` enabled
2. Full boot test: `./uqm -o -f` for 10 seconds
3. Verify: splash → main menu appears → menu navigation works (via log output)
4. Verify: selecting "New Game" or "Super Melee" does not crash
5. `cargo test --lib` — all tests pass

---

## P11a: E2E Verification
- deepthinker reviews full integration

---

## Phase Dependency Graph

```
P01 (analysis) → P01a (verify)
      ↓
P02 (types) → P02a (verify)
      ↓
P03 (menu logic) → P03a (verify)     ← pure functions, no deps, NOT blocked by P04
      ↓                                  ↓
P04a-e (C wrappers)           ← can run in parallel with P03
      ↓
P05 (FFI externs + trait) → P05a (verify)
      ↓
P06 (DoRestart frame) → P06a (verify)  ← uses P03 logic
      ↓
P07 (RestartMenu) → P07a (verify)      ← uses P06
      ↓
P08 (TryStartGame) → P08a (verify)     ← uses P07
      ↓
P09 (StartGame) → P09a (verify)        ← uses P08
      ↓
P10 (C wiring + build) → P10a (verify)
      ↓
P11 (E2E, manual) → P11a (verify)
```

**Key optimization:** P03 (pure menu logic) is NOT blocked by P04 (C wrappers). They can be developed in parallel.

## Requirements Summary

| Req ID | Description | Phase |
|--------|-------------|-------|
| REQ-RM-001 | Domain types (RestartMenuItem, SelectionResult, MenuInputState) | P02 |
| REQ-RM-002 | Pure navigation/selection logic | P03 |
| REQ-RM-003 | C wrapper functions for game-state access | P04 |
| REQ-RM-004 | FFI extern declarations + safe wrappers | P04 |
| REQ-RM-005 | RestartMenuOps trait + CffiOps | P05 |
| REQ-RM-006 | DoRestart per-frame callback logic | P06 |
| REQ-RM-007 | RestartMenu lifecycle orchestration | P07 |
| REQ-RM-008 | TryStartGame retry loop | P08 |
| REQ-RM-009 | StartGame outer loop + player control | P09 |
| REQ-RM-010 | C dispatch guard wiring + build integration | P10 |

## Risk Areas

1. **Static variables in DoRestart:** `LastInputTime` and `InactTimeOut` are C
   statics. In Rust, these become fields on the `RestartMenuState` struct,
   eliminating the static state. Test carefully for state persistence across
   frames.

2. **DoInput callback pattern:** The C `DoInput` calls `InputFunc` via a
   function pointer on `MENU_STATE`. The callback bridge must use
   `MENU_STATE.privData` to pass the Rust state pointer. The callback is
   `BOOLEAN rust_restart_input_func(MENU_STATE *pMS)` which reads
   `pMS->privData`. State is allocated before DoInput, freed after.
   **Reentrancy risk:** `DoInput` calls `Async_process` and `TaskSwitch`
   which can trigger other callbacks. The Rust state must be thread-safe
   or guaranteed single-threaded during the menu loop.

3. **Flash overlay lifecycle:** The Flash system has many API calls with
   specific ordering. Missing any call could leave the flash system in a bad
   state. Test the full init → process → pause/continue → terminate lifecycle.

4. **Large FFI surface:** restart.c calls ~40 distinct C functions. P04 is
   split into P04a-e to manage complexity. Each sub-phase is independently
   verifiable.

5. **C macro hazards:** Many C constructs are macros (`GLOBAL()`,
   `GLOBAL_SIS()`, `LOBYTE`, `GET_GAME_STATE`, `SET_GAME_STATE`,
   `BUILD_COLOR`, `MAKE_RGB15`, `FadeAllToColor`, `FlashState_fadeIn`,
   `MAINMENU_MUSIC`, `RESTART_PMAP_ANIM`, etc.). Each must be either:
   - Mirrored as a Rust constant with validation test, OR
   - Exposed through a C wrapper function.
   Rust cannot link against macros. This project has hit this class of
   issue before.

6. **BOOLEAN ABI:** `BOOLEAN` is C `enum` = `int` (4 bytes). Rust must use
   `c_int`, not `bool`. All callbacks and entry points use `CBoolean = c_int`.

7. **Post-call re-reads:** After any C dispatch call (`SetupMenu`, `Melee`,
   `DoInput`, `Credits`, `Introduction`), `CurrentActivity` must be re-read
   from C because C may have mutated it. Do not rely on cached Rust values.

8. **Static helper visibility:** `DrawRestartMenuGraphic` and `DrawRestartMenu`
   are `static` in restart.c. Must remove `static` when `USE_RUST_RESTART`
   is defined, or move the drawing logic into C wrappers within restart.c.

9. **Handle nullability:** C tolerates null handles (`hMusic == 0`,
   `flashContext == NULL`). Rust must model these as nullable opaque
   pointers, not panic on null.

10. **DoInput internals:** `DoInput` calls `Async_process()`, `TaskSwitch()`,
    `UpdateInputState()`, menu sound playback, and `inputCallback()`. These
    must all work correctly when the `InputFunc` callback is Rust-provided.
    The Rust callback must NOT bypass pause/exit handling.
