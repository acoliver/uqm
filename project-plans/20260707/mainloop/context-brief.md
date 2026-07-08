# Context Brief: Main Game Loop Port to Rust

*Prepared 2026-07-07. This brief provides ground-truth codebase facts for the
plan-writer and reviewer. All information was verified by direct code
inspection, not from stale docs.*

---

## 1. Project Architecture (Verified)

**C-primary / Rust-library hybrid.** The Rust code (`rust/`) builds as a
static library `libuqm_rust.a`. The C binary (`sc2/`) links it. Subsystems are
ported incrementally via `#ifdef USE_RUST_*` dispatch guards in C source —
when a flag is set, C delegates to a `rust_*` bridge function.

- **All 22 `USE_RUST_*` flags are enabled** in `sc2/build.vars`.
- Rust codebase: ~122K LOC, ~190 files, 12 subsystems.
- **2580 Rust unit tests pass, 0 failures.**
- Binary compiles, links, and runs (boots to main menu cleanly).

### Critical lesson (the RaceDesc ABI bug, fixed 2026-07-07)

The bottom-up approach hit a structural wall: when Rust tries to **own** a
C struct that C also reads by raw offset, the layouts mismatch and C reads
garbage/NULL. Specifically: Rust's `RaceDesc` (288 bytes, not `#[repr(C)]`)
vs C's `RACE_DESC` (384 bytes) caused NULL ship frames in melee. Fix: route
battle-critical struct-dependent paths (load/spawn/preprocess/postprocess) to
C-native code. Rust stays for ABI-safe pure services (catalog, costs, decoders).

**This is WHY the plan must include C-boundary tests proving Rust↔C data
passing works.** The whole point of this plan is to establish a clean,
tested FFI contract so the main loop can live in Rust without repeating this
class of bug.

---

## 2. The C Entry Point (Verified)

Real `main()` is in **`sc2/src/uqm.c:233`** (the OUTER file, NOT
`sc2/src/uqm/uqm.c`).

### Init sequence in `main()`:

```
parseOptions(argc, argv, &options)          // CLI parsing
log_init(15)
rust_bridge_init()                          // [USE_RUST_BRIDGE] Rust logging
freopen(logFile)                            // redirect stderr to logfile
// version/usage handling
TFB_PreInit()                               // SDL preinit
mem_init()                                  // [USE_RUST_MEM] memory mgmt
InitThreadSystem()                          // [USE_RUST_THREADS] threading
log_initThreads()
initIO()                                    // [USE_RUST_UIO] I/O system
prepareConfigDir(configDir)
LoadResourceIndex(configDir, "uqm.cfg")     // config file
getUserConfigOptions(&options)
// set global option vars from options struct
prepareContentDir / prepareMeleeDir / prepareSaveDir
InitTimeSystem()                            // [USE_RUST_CLOCK] time system
InitTaskSystem()
Alarm_init()
Callback_init()
// NETPLAY: Network_init, NetManager_init
TFB_InitGraphics(driver, flags, backend, w, h)   // [USE_RUST_GFX] graphics
setGammaCorrection()
InitColorMaps()
init_communication()
TFB_SetInputVectors(...)
TFB_InitInput(TFB_INPUTDRIVER_SDL, 0)       // [USE_RUST_INPUT]
StartThread(Starcon2Main, NULL, 1024, "Starcon2Main")  // launch game loop
// main thread: spin until MainExited || QuitPosted
```

### Key observation about threading

`main()` runs the **init sequence**, then launches `Starcon2Main` on a
**separate thread** via `StartThread()`. The main thread then just waits.
The game loop runs entirely on the Starcon2Main thread. Several C comments
note: "Once threading is gone, these become local variables again" — the
threading exists mainly as a legacy artifact, not a design requirement.

---

## 3. The Game Loop (Verified: `sc2/src/uqm/starcon.c:155`)

`Starcon2Main(void *threadArg)`:

```
initAudio(snddriver, soundflags)            // audio init
if (!LoadKernel(0,0)) → FATAL ERROR         // load base content pack
SplashScreen(BackgroundInitKernel)           // splash + background kernel init

while (StartGame())                          // OUTER: new-game loop
{
    SetPlayerInputAll()
    InitGameStructures()
    InitGameClock()
    AddInitialGameEvents()

    do {                                     // INNER: activity state machine
        if (CurrentActivity & START_ENCOUNTER || CHMMR_BOMB_STATE == 2)
            VisitStarBase() | RaceCommunication()
        else if (CurrentActivity & START_INTERPLANETARY)
            ExploreSolarSys()
        else
            Battle(&on_battle_frame)         // hyperspace/quasispace

        // death/win/restart handling
    } while (!(CurrentActivity & CHECK_ABORT))

    StopSound()
    UninitGameClock()
    UninitGameStructures()
    ClearPlayerInputAll()
}
UninitGameKernel()
FreeMasterShipList()
FreeKernel()
```

### Activity state machine

The loop is driven by `GLOBAL(CurrentActivity)` — a 16-bit value where the
low byte is the "activity" enum (IN_HYPERSPACE, IN_INTERPLANETARY,
IN_ENCOUNTER, IN_BATTLE, WON_LAST_BATTLE, etc.) and high bits are flags
(CHECK_ABORT, CHECK_LOAD, START_ENCOUNTER, START_INTERPLANETARY, etc.).

Key C functions in the loop that need FFI bridges if Rust drives:
- `StartGame()` → BOOLEAN (new game / load game menu)
- `InitGameStructures()`, `InitGameClock()`, `AddInitialGameEvents()`
- `VisitStarBase()`, `RaceCommunication()`, `ExploreSolarSys()`, `Battle()`
- `UninitGameClock()`, `UninitGameStructures()`, `StopSound()`
- `UninitGameKernel()`, `FreeMasterShipList()`, `FreeKernel()`
- `LoadKernel()`, `SplashScreen()`, `SetPlayerInputAll()`

---

## 4. What the Plan Must Achieve

**Goal:** Move the main startup sequence and game loop into Rust, so Rust
becomes the driver (top-down approach). Rust owns the entry point and the
activity state machine. C subsystems remain callable via FFI bridges during
transition.

**The test-first mandate:** Every FFI bridge must have tests proving data
crosses the Rust↔C boundary correctly. This is the explicit lesson from the
RaceDesc ABI bug — we must prove with tests that:

1. Rust can call C init functions and observe correct side effects.
2. C can call back into Rust (the loop body delegates).
3. Shared global state (CurrentActivity, game state flags) round-trips
   correctly across the boundary.
4. The init sequence ordering is preserved and observable.
5. The activity state machine transitions are driven correctly from Rust.

---

## 5. Existing FFI Infrastructure

- `rust/src/c_bindings.rs` — central FFI declarations
- `rust/src/bridge_log.rs` — Rust bridge logging
- `rust/src/game_init/` — has `ffi.rs`, `init.rs`, `master.rs`, `setup.rs`
  (5 files, ~993 LOC) — **partial game-init Rust code already exists**
- `rust/src/state/game_state.rs` — game state get/set via FFI
  (`rust_get_game_state`, `rust_set_game_state`)
- `sc2/src/uqm/rust_bridge_macros.c` — C-side bridge macros
- `sc2/src/uqm/clock_rust.c` — clock bridge (228 LOC)
- `sc2/src/uqm/rust_comm.c` — comm bridge (2143 LOC)

The game_init module already has some structure — check what exists before
planning new code.

---

## 6. Build System Integration

- Rust staticlib: `rust/target/release/libuqm_rust.a`
- C build: `cd sc2 && ./build.sh uqm`
- Build feature flags in `sc2/build.vars` (all 22 `USE_RUST_*` = 1)
- New Rust entry-point functions need `#[no_mangle] pub extern "C"` exports
- The C `main()` in `sc2/src/uqm.c` is where the transition would start

---

## 7. Constraints from dev-docs

- **TDD is mandatory** (RED → GREEN → REFACTOR)
- **Sequential phases only** (no skipping)
- **No placeholders/TODO in implementation phases**
- **Integration-first** (feature must be reachable via real app flows)
- **Test the C boundary explicitly** (per user requirement)
- Plan ID format: `PLAN-20260707-MAINLOOP`
- Follow `dev-docs/PLAN-TEMPLATE.md` structure
- Pseudocode must be numbered with line references
