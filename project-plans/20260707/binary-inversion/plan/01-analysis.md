# Binary Inversion — P01 Analysis

## Goal

Make Rust own `main()`. Currently C `main()` (in `sc2/src/uqm.c`) owns the
process lifecycle, spawns a separate thread for the game loop, and runs the
SDL event pump. The goal is to invert this: Rust binary → C library.

## Current Architecture

```
OS → C main() [main thread]
      ├─ parseOptions() — command-line
      ├─ init sequence (~25 calls, most Rust-backed)
      ├─ StartThread(Starcon2Main) ────▶ [game thread]
      │                                     └─ rust_game_loop()
      │                                         └─ run_game_lifecycle_impl()
      ├─ event pump [main thread, loops until MainExited]:
      │    TFB_ProcessEvents()
      │    ProcessUtilityKeys()
      │    ProcessThreadLifecycles()   ← reaps dead threads
      │    TFB_FlushGraphics()         ← drains DCQ
      └─ teardown (~15 calls)
```

## Target Architecture

```
OS → Rust main() [main thread]
      ├─ parse CLI (already have cli.rs)
      ├─ init sequence (direct calls, same thread)
      ├─ game loop + event pump [SAME thread, interleaved]:
      │    rust_game_loop() {
      │      ... game frame ...
      │      process_events()    ← SDL_PollEvent
      │      flush_graphics()    ← DCQ drain
      │    }
      └─ teardown (direct calls)
```

## What Gets Eliminated

| Mechanism | Why it exists | Why we can drop it |
|-----------|--------------|-------------------|
| StartThread(Starcon2Main) | Game loop on separate thread | Run on main thread |
| MainExited global | Cross-thread "game done" signal | Same thread, direct return |
| QuitPosted / SignalStopMainThread | Main thread tells game thread to stop | Direct break from loop |
| ProcessThreadLifecycles | Reaps dead game threads | No spawned game threads |
| GameActive hibernation | Throttle main thread when idle | Not needed single-threaded |
| HibernateThread in event pump | Sleep when no game active | Rust controls pacing |

## Init Sequence Classification (C main() lines 348-452)

### Already Rust-backed (call through C wrapper → Rust)
| Call | C file | Rust backend | Needs FFI extern? |
|------|--------|-------------|-------------------|
| mem_init() | libs/memory/w_memlib.c:33 | USE_RUST_MEM (stub returns true) | Yes — or call Rust directly |
| InitThreadSystem() | libs/threads/rust_thrcommon.c | calls rust_init_thread_system() | Yes — or call Rust directly |
| initIO() | uqm/setup.c:311 | USE_RUST_UIO | Yes |
| InitTimeSystem() | libs/time/timecommon.c:31 | calls NativeInitTimeSystem() | Yes |
| TFB_InitGraphics() | libs/graphics/sdl/sdl_common.c:95 | USE_RUST_GFX | Yes |
| init_communication() | uqm/comm.c | USE_RUST_COMM (ifdef'd out, thin wrapper) | Yes |
| TFB_InitInput() | libs/input/sdl/input.c:265 | USE_RUST_INPUT | Yes |

### Pure C, needs FFI extern
| Call | C file | Lines | Notes |
|------|--------|-------|-------|
| TFB_PreInit() | libs/graphics/sdl/sdl2_common.c:42 | ~20 | SDL pre-init (video subsystem) |
| log_initThreads() | libs/log/uqmlog.c:147 | ~5 | Thread-specific log setup |
| prepareConfigDir() | options.c:205 | ~50 | Mounts config dir via uio |
| LoadResourceIndex() | libs/resource/resinit.c:370 | ~40 | Loads uqm.cfg config |
| prepareContentDir() | options.c:136 | ~70 | Mounts content/addon dirs |
| prepareMeleeDir() | options.c:295 | ~15 | Mounts melee save dir |
| prepareSaveDir() | options.c:258 | ~35 | Mounts save dir |
| prepareShadowAddons() | options.c:554 | ~60 | Sets up addon overlays |
| InitTaskSystem() | libs/task/tasklib.c:120 | ~10 | Task system init |
| Alarm_init() | libs/callback/alarm.c:56 | ~15 | Alarm/timer system |
| Callback_init() | libs/callback/callback.c:63 | ~15 | Callback system |
| InitColorMaps() | libs/graphics/cmap.c:72 | ~20 | Color map system |
| setGammaCorrection() | (in graphics lib) | ~10 | Gamma ramp |
| TFB_SetInputVectors() | libs/input/sdl/input.c:212 | ~10 | Sets input key arrays |
| log_init() | libs/log/uqmlog.c | ~10 | Already called from Rust main.rs |
| getUserConfigOptions() | uqm.c:623 | ~80 | Parses uqm.cfg → options |

### Constants/globals to port
- snddriver, soundflags (set from options, read by initAudio)
- PlayerControls[0..1]
- opt3doMusic, optRemixMusic, optSpeech, etc. (~15 option globals)

## Event Pump Functions

| Call | C file | What it does |
|------|--------|-------------|
| TFB_ProcessEvents() | libs/graphics/sdl/sdl_common.c:208 | SDL_PollEvent loop, dispatches to ProcessInputEvent |
| ProcessUtilityKeys() | uqm/starcon.c:136 | Handles KEY_ABORT (exit), KEY_FULLSCREEN (toggle), KEY_DEBUG |
| ProcessThreadLifecycles() | libs/threads/rust_thrcommon.c:279 | Reaps dead threads — ELIMINATED in single-threaded |
| TFB_FlushGraphics() | libs/graphics/dcqueue.c:323 | Drains Deferred Command Queue, presents frame |

## Teardown Sequence (C main() lines 477-504)

All are simple function calls, need FFI externs:
TFB_UninitInput, unInitAudio, uninit_communication, TFB_PurgeDanglingGraphics,
UninitColorMaps, TFB_UninitGraphics, Callback_uninit, Alarm_uninit,
CleanupTaskSystem, UnInitTimeSystem, unprepareAllDirs, uninitIO,
UnInitThreadSystem, mem_uninit

## Threading Analysis

Current flow:
1. C main() calls InitThreadSystem() → initializes Rust thread system + lifecycle mutex
2. C main() calls StartThread(Starcon2Main) → spawns OS thread via rust_thread_spawn
3. Game thread runs rust_game_loop() → game loop
4. Main thread loops: ProcessEvents + ProcessThreadLifecycles + FlushGraphics
5. ProcessThreadLifecycles joins finished threads

Key constraint: `initAudio()` was moved to Starcon2Main because "initAudio calls
AssignTask, which currently blocks on ProcessThreadLifecycles." This means
initAudio uses the C task system to dispatch work. In the inverted design, we
either:
a) Keep calling initAudio through C (it already works from rust_game_loop), or
b) Port initAudio's AssignTask calls to direct calls

For Phase 1 of the inversion, option (a) is safest — initAudio stays as-is,
called from the game loop which now runs on the main thread.

## DCQ Same-Thread Safety

The DCQ exists because the game thread enqueues draw commands and the main
thread drains them via TFB_FlushGraphics. In the single-threaded design, the
DCQ must be drained synchronously within the same thread. If the game loop
enqueues commands inside a blocking activity (Battle, Comm, etc.) and the DCQ
fills up before the activity returns, the enqueue will deadlock waiting for a
drain that can never happen.

**Solution**: Add TFB_ProcessEvents() + TFB_FlushGraphics() calls inside
DoInput() (gameinp.c:361-408). Every blocking loop in UQM calls DoInput
per frame, so adding the event pump + DCQ drain there ensures both SDL events
and graphics are serviced during all gameplay, menus, communication, and battle.

This is the critical integration point -- NOT between activity dispatches.

## DoInput Per-Frame Path (gameinp.c:361-408)

```
DoInput(pInputState, resetInput):
  do:
    Async_process()        -- runs pending callbacks/alarms
    TaskSwitch()           -- yields to other threads (main thread pumps events)
    UpdateInputState()     -- reads ImmediateInputState (set by SDL events)
    ... menu sounds ...
    inputCallback()        -- custom callback (DoRestart etc.)
  while InputFunc(pInputState)
```

Key insight: TaskSwitch is the yield point where the main thread gets CPU
time to pump events. In single-threaded, TaskSwitch has nothing to switch to.
The fix is to pump events DIRECTLY in DoInput when running single-threaded.

### Proposed DoInput modification (guarded by ifdef RUST_OWNS_MAIN):
```c
do {
    Async_process();
#ifdef RUST_OWNS_MAIN
    TFB_ProcessEvents();     /* pump SDL events (was done by main thread) */
    TFB_FlushGraphics();     /* drain DCQ (was done by main thread) */
    ProcessUtilityKeys();    /* fullscreen toggle, abort, etc. */
#endif
    TaskSwitch();
    UpdateInputState();
    ...
} while (InputFunc(...));
```

## Risk Areas

1. **SDL main thread**: SDL_Init and SDL_PollEvent must be called from the OS
   main thread. Rust fn main() runs on the OS main thread. Safe.
   Build invariant: P06 must ensure Rust's binary entry point is the actual
   process entry, not routed through a C launcher or background thread.

2. **initAudio/AssignTask deadlock**: initAudio uses AssignTask which calls
   CreateThread. With the Rust thread backend (USE_RUST_THREADS=1, active),
   CreateThread_Core calls rust_thread_spawn directly (rust_thrcommon.c:140-162)
   -- it does NOT block on ProcessThreadLifecycles. Therefore initAudio is safe
   on the main thread. Hard dependency on Rust thread backend; legacy backend
   WOULD deadlock.

3. **ProcessThreadLifecycles elimination**: With the Rust backend, this function
   only reaps finished threads from pendingDeath[LIFECYCLE_SIZE] (max 8).
   AssignTask users are audio-related (long-lived streams), no accumulation risk.
   Final reap still happens in UnInitThreadSystem(). Safe to remove from
   per-frame loop.

4. **DCQ same-thread safety**: Solved by adding TFB_FlushGraphics to DoInput's
   per-frame loop. The DCQ drains every input frame, preventing fill-up deadlock.

5. **ProcessUtilityKeys calls exit()**: KEY_ABORT calls exit(EXIT_SUCCESS) directly
   (starcon.c:136-142). Bypasses RAII cleanup in Rust-owned main. For Phase 1,
   acceptable (same behavior as today). Future: replace exit() with quit flag.

6. **QuitPosted on SDL_QUIT**: TFB_ProcessEvents sets QuitPosted on SDL_QUIT
   (sdl_common.c:208-245). In single-threaded, checked inside DoInput's modified
   loop. Rust game loop should also check quit_posted() after C functions return.

7. **getUserConfigOptions**: Parses uqm.cfg into C options_struct. For Phase 1,
   keep calling through C, read resulting globals.

8. **Missing init items**: C main() also has: control template cleanup loop
   (res_Remove x6, uqm.c:365-377), optGamma assignment (uqm.c:436-439),
   NETPLAY init (Network_init/NetManager_init, uqm.c:420-423), array assert
   (uqm.c:447-449). These must be preserved in Rust port or documented as
   out-of-scope (NETPLAY).

9. **Missing teardown items**: NETPLAY uninit (uqm.c:488-491), HFree(options.addons)
   (uqm.c:507). Teardown only runs if MainExited is true (uqm.c:477) -- in
   Rust-owned main, the game loop return signals completion.

## Plan Summary

| Phase | What | Effort |
|-------|------|--------|
| P02 | FFI externs for all C-only init/teardown functions | ~30 externs |
| P02a | Verification | |
| P03 | Port event pump (TFB_ProcessEvents, ProcessUtilityKeys, TFB_FlushGraphics as FFI) | ~3 externs |
| P03a | Verification | |
| P04 | Port teardown sequence (FFI externs) | ~14 externs |
| P04a | Verification | |
| P05 | Rewrite Rust main.rs — init sequence + interleaved game loop + teardown | Core rewrite |
| P05a | Verification | |
| P06 | Build integration — Cargo.toml binary, link C as lib, #ifdef out C main() | Build system |
| P06a | Verification | |
| P07 | E2E boot test — Rust binary boots to menu, melee works, clean shutdown | Testing |
| P07a | Verification | |
