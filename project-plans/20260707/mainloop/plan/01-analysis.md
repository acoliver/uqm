# Phase 01: Domain Analysis

## Phase ID
`PLAN-20260707-MAINLOOP.P01`

## Prerequisites
- Phase 0.5 preflight complete

---

## 1. Activity State Transition Model

The game loop is driven by `GLOBAL(CurrentActivity)` — a 16-bit value
(`UWORD`) in C's `GlobData.Game_state.CurrentActivity`. The low byte is
an activity enum; the high byte holds bit flags.

### Activity enum (low byte, from `globdata.h:893-918`)

| Value | Name | Meaning |
|------:|------|---------|
| 0 | `SUPER_MELEE` | Super Melee setup screen |
| 1 | `IN_LAST_BATTLE` | Final Sa-Matra battle |
| 2 | `IN_ENCOUNTER` | In an alien encounter/dialogue |
| 3 | `IN_HYPERSPACE` | Traveling in hyperspace |
| 4 | `IN_INTERPLANETARY` | In a solar system |
| 5 | `WON_LAST_BATTLE` | Game won |
| 6 | `IN_QUASISPACE` | In QuasiSpace |

### Activity flags (high byte)

| Bit | Name | Mask | Meaning |
|----:|------|------|---------|
| 0 | `CHECK_PAUSE` | `0x0100` | Paused |
| 1 | `IN_BATTLE` | `0x0200` | In melee combat |
| 2 | `START_ENCOUNTER` | `0x0400` | Start an encounter this tick |
| 3 | `START_INTERPLANETARY` | `0x0800` | Enter a solar system |
| 4 | `CHECK_LOAD` | `0x1000` | Load a saved game |
| 5 | `CHECK_RESTART` | `0x2000` | Restart the game |
| 6 | `CHECK_ABORT` | `0x4000` | Exit the inner loop |

### State transition diagram (from `starcon.c:210-290`)

```
                    ┌─────────────────────┐
                    │   StartGame() = TRUE │
                    │   (new/load game)    │
                    └──────────┬──────────┘
                               │
                    ┌──────────▼──────────┐
                    │  Inner loop entry    │
                    │  (do...while)        │
                    └──────────┬──────────┘
                               │
              ┌────────────────┼────────────────┐
              ▼                ▼                 ▼
    START_ENCOUNTER   START_INTERPLANETARY    (default)
    or BOMB==2                                 │
         │                 │                   │
         ▼                 ▼                   ▼
  VisitStarBase()   ExploreSolarSys()    Battle()
  / RaceComm()                                  │
         │                 │                   │
         └────────────────┼───────────────────┘
                          │
                 ┌────────▼────────┐
                 │ CHECK_ABORT set?│
                 └────────┬────────┘
                    no    │    yes
              ┌──────────┘    └──────────┐
              ▼                          ▼
       (loop back)               exit inner loop
                                     │
                          ┌──────────▼──────────┐
                          │ StartGame() again?  │
                          └──────────┬──────────┘
                             yes     │     no
                       ┌─────────────┘ └────────────┐
                       ▼                            ▼
                 (new game)                   shutdown
```

### Key transitions the Rust state machine must replicate

1. `START_ENCOUNTER` flag OR `CHMMR_BOMB_STATE == 2`:
   - If `!STARBASE_AVAILABLE` and `CHMMR_BOMB_STATE == 2` → `InstallBombAtEarth` (BGD mode)
   - If `GLOBAL_FLAGS_AND_DATA == ~0` OR `CHMMR_BOMB_STATE == 2` → `VisitStarBase`
   - Else → `RaceCommunication`
   - After: if not `CHECK_ABORT|CHECK_LOAD`, clear `START_ENCOUNTER`, set `START_INTERPLANETARY` if was `IN_INTERPLANETARY`

2. `START_INTERPLANETARY` flag (no encounter):
   - Set activity to `IN_INTERPLANETARY`
   - `DrawAutoPilotMessage(TRUE)`
   - `SetGameClockRate(INTERPLANETARY_CLOCK_RATE)`
   - `ExploreSolarSys()`

3. Default (neither flag):
   - Set activity to `IN_HYPERSPACE`
   - `DrawAutoPilotMessage(TRUE)`
   - `SetGameClockRate(HYPERSPACE_CLOCK_RATE)`
   - `Battle(&on_battle_frame)`

4. Win/loss check after each iteration:
   - If `WON_LAST_BATTLE` OR `CrewEnlisted == ~0` (died):
     - If `KOHR_AH_KILLED_ALL` → `InitCommunication(BLACKURQ_CONVERSATION)`
     - If `CHECK_RESTART` → clear it
     - Break inner loop

---

## 2. Init Sequence Ordering (from `uqm.c:348-452`)

**C `main()` owns this entire sequence.** Rust does NOT call any of these.
This table is preserved as reference for understanding C startup behavior.
Rust's init is limited to Starcon2Main-specific steps (Section 3).

| Step | C Function | Purpose | Failure handling |
|-----:|------------|---------|-----------------|
| 1 | `TFB_PreInit()` | SDL preinit | abort |
| 2 | `mem_init()` | Memory management | abort |
| 3 | `InitThreadSystem()` | Threading | abort |
| 4 | `log_initThreads()` | Thread logging | continue |
| 5 | `initIO()` | I/O system | abort |
| 6 | `prepareConfigDir(configDir)` | Config dir | abort |
| 7 | `LoadResourceIndex(configDir, "uqm.cfg")` | Config file | continue |
| 8 | `getUserConfigOptions(&options)` | Config options | continue |
| 9 | `prepareContentDir(...)` | Content dirs | abort |
| 10 | `prepareMeleeDir()` | Melee dir | continue |
| 11 | `prepareSaveDir()` | Save dir | continue |
| 12 | `InitTimeSystem()` | Time/clock | abort |
| 13 | `InitTaskSystem()` | Task system | continue |
| 14 | `Alarm_init()` | Alarms | continue |
| 15 | `Callback_init()` | Callbacks | continue |
| 16 | `TFB_InitGraphics(driver, flags, ...)` | Graphics | abort |
| 17 | `setGammaCorrection()` | Gamma | continue |
| 18 | `InitColorMaps()` | Color maps | continue |
| 19 | `init_communication()` | Comm system | continue |
| 20 | `TFB_SetInputVectors(...)` | Input vectors | continue |
| 21 | `TFB_InitInput(TFB_INPUTDRIVER_SDL, 0)` | Input | abort |

---

## 3. FFI Touchpoints

### C functions Rust must call (extern "C" declarations needed)

**Starcon2Main-specific init** (Rust calls these from inside `rust_game_loop`):
`initAudio`, `LoadKernel`, `StartGame`, `SetPlayerInputAll`,
`InitGameStructures`, `InitGameClock`, `AddInitialGameEvents`

**C `main()` owns startup** — Rust does NOT call:
`TFB_PreInit`, `mem_init`, `InitThreadSystem`, `initIO`,
`prepareConfigDir`, `LoadResourceIndex`, `getUserConfigOptions`,
`TFB_InitGraphics`, `InitColorMaps`, `init_communication`, `TFB_InitInput`, etc.

**Game loop dispatch:**
`VisitStarBase`, `RaceCommunication`, `ExploreSolarSys`, `Battle`,
`InstallBombAtEarth`, `InitCommunication`, `DrawAutoPilotMessage`,
`SetGameClockRate`, `StopSound`, `UninitGameClock`, `UninitGameStructures`,
`ClearPlayerInputAll`, `SetStatusMessageMode`, `ZeroVelocityComponents`,
`SetFlashRect`

**Game-kernel cleanup only** (starcon.c:313-318, NOT full subsystem teardown):
`UninitGameKernel`, `FreeMasterShipList`, `FreeKernel`, `log_showBox`

**Shutdown:**
`UninitGameKernel`, `FreeMasterShipList`, `FreeKernel`

**State accessors (NEW — must be added to C, using GET_GAME_STATE/SET_GAME_STATE internally):**
`get_current_activity()` → `u16`, `set_current_activity(u16)`,
`get_next_activity()` → `u16`, `set_next_activity(u16)`,
`get_last_activity()` → `u16`, `set_last_activity(u16)`,
`uqm_get_chmmr_bomb_state()` → `u8`, `uqm_set_chmmr_bomb_state(u8)`,
`uqm_get_starbase_available()` → `u8`,
`uqm_get_global_flags_and_data()` → `u8`,
`uqm_get_kohr_ah_killed_all()` → `u8`,
`uqm_get_crew_enlisted()` → `u16`

### Rust functions C must call (no_mangle extern "C")

`rust_game_loop() -> c_int`

---

## 4. Old Code to Replace/Remove (Revised — iteration 2)

**Key revision**: Rust replaces the `Starcon2Main()` body, NOT the entire
`main()`. The C main-thread event pump (`uqm.c:456-472`) is preserved.
The C init path (`main():283-452`) runs in C `main()` directly — no Rust wrapper.

| File | Lines | Current behavior | New behavior |
|------|-------|-----------------|--------------|
| `sc2/src/uqm/starcon.c` | 155-323 | `Starcon2Main` — full game loop | `#ifdef USE_RUST_MAINLOOP`: delegates to `rust_game_loop()` |
| `sc2/src/uqm/starcon.c` | 80-90 | `on_battle_frame` (static) | C wrapper `uqm_battle_with_frame_callback()` calls it (P02b) |
| `sc2/src/uqm/starcon.c` | 92-105 | `BackgroundInitKernel` (static) | C wrapper `uqm_splash_with_bg_init_kernel()` calls it (P02b) |
| `sc2/src/uqm.c` | 217-219 | `parseOptions`/`getUserConfigOptions` (static) | Not wrapped — C `main()` owns startup; Rust never calls these |
| `sc2/src/uqm.c` | 456-472 | Main-thread event pump | **PRESERVED UNCHANGED** (TFB_ProcessEvents, ProcessUtilityKeys, etc.) |
| `sc2/src/uqm.c` | 479-504 | Full subsystem shutdown | **PRESERVED in C** — Rust does NOT call subsystem teardown (prevents double-free) |

### Main-thread event pump (preserved, NOT removed)

The C main thread (`uqm.c:456-472`) does essential work while the game
loop runs on the Starcon2Main thread:
- `TFB_ProcessEvents()` — SDL event pumping
- `ProcessUtilityKeys()` — utility key handling
- `ProcessThreadLifecycles()` — task lifecycle management
- `TFB_FlushGraphics()` — graphics buffer flushing

**This pump MUST continue running.** Rust's `rust_game_loop()` runs on
the Starcon2Main thread (via existing `StartThread` mechanism). The
main-thread pump is not touched.

### NETPLAY paths

NETPLAY init (`uqm.c:420-423`: `Network_init`, `NetManager_init`) and
shutdown (`uqm.c:488-491`: `NetManager_uninit`, `Network_uninit`) are
owned by C `main()` for both startup and subsystem shutdown
(shutdown), respectively. Both are conditional on `#ifdef NETPLAY`.

### Backward compatibility

`#ifdef USE_RUST_MAINLOOP` guard: when undefined, the original C
`Starcon2Main` body runs unchanged. When defined, it delegates to
`rust_game_loop()`.

---

## 5. Edge / Error Handling Map

| Scenario | C behavior | Rust behavior |
|----------|-----------|---------------|
| `LoadKernel` fails | `log_add(log_Fatal, ...)`, `MainExited = TRUE`, `return EXIT_FAILURE` | `run_game_lifecycle()` returns `Err(MainLoopError::LoadKernelFailed)`, `rust_game_loop` returns `EXIT_FAILURE` |
| `SetPlayerInputAll` fails | `explode()` (abort) | Return error, log fatal, exit |
| `StartGame` returns false | Exit outer loop → shutdown | Same |
| `CHECK_ABORT` set | Exit inner loop | `ActivityStateMachine::should_abort()` returns true |
| Player dies (`CrewEnlisted == ~0`) | Break inner loop | `should_stop()` returns true |
| Unknown activity combo | Falls through to `Battle` (else branch) | Default arm dispatches to `Battle` |

---

## 6. Integration Touchpoints Summary

- **Caller of new behavior:** C `Starcon2Main` in `sc2/src/uqm/starcon.c` (modified to call `rust_game_loop`)
- **Old behavior replaced:** Starcon2Main body (game loop only) + thread launch + game loop
- **User trigger:** Running `./uqm` — identical UX, Rust drives internally
- **State migration:** None — C global state (`GlobData`) remains in C memory; Rust accesses via FFI accessors
- **Backward compat:** `#ifdef USE_RUST_MAINLOOP` guard; when undefined, original C path runs
