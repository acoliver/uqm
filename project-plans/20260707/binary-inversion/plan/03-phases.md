# Binary Inversion — P03 Phase Specs

## P02: FFI Externs for Init/Teardown Functions

### Objective
Create a new `rust/src/mainloop/c_main_extern.rs` module with all FFI extern
declarations for C functions called from the init sequence, event pump, and
teardown.

### Tasks
1. Create `c_main_extern.rs` with extern "C" blocks for:
   - Init: TFB_PreInit, log_initThreads, prepareConfigDir, LoadResourceIndex,
     prepareContentDir, prepareMeleeDir, prepareSaveDir, prepareShadowAddons,
     InitTaskSystem, Alarm_init, Callback_init, InitColorMaps,
     setGammaCorrection, TFB_SetInputVectors, TFB_InitInput, TFB_InitGraphics
   - Event pump: TFB_ProcessEvents, ProcessUtilityKeys, TFB_FlushGraphics
   - Teardown: TFB_UninitInput, unInitAudio, uninit_communication,
     TFB_PurgeDanglingGraphics, UninitColorMaps, TFB_UninitGraphics,
     Callback_uninit, Alarm_uninit, CleanupTaskSystem, UnInitTimeSystem,
     unprepareAllDirs, uninitIO
   - Globals accessors: get/set for snddriver, soundflags, PlayerControls[2],
     opt* globals

2. Create a C bridge file `sc2/src/uqm/rust_bridge_main2.c` with:
   - Wrapper functions for setting C globals from Rust
   - Any functions that are static or need adapter signatures

3. Add `rust_bridge_main2.c` to Makeinfo

### Tests
- Tier-1: Verify extern declarations compile (link check only)
- No behavioral tests — these are FFI declarations

### Definition of Done
- `cargo build --lib` compiles with the new externs
- All existing tests still pass

---

## P03: DoInput Event Pump Integration (CRITICAL)

### Objective
The event pump currently runs on the main thread while the game loop runs on
a separate thread. In single-threaded mode, SDL events must be pumped from
inside DoInput's per-frame loop -- this is the universal per-frame function
called by every blocking loop in UQM.

### CRITICAL: Why not pump from Rust?
DoInput (gameinp.c:361-408) does NOT call SDL_PollEvent or TFB_ProcessEvents.
It calls Async_process, TaskSwitch, UpdateInputState. TaskSwitch yields to the
main thread which pumps events. In single-threaded, there's no main thread to
yield to. Therefore we MUST add event pumping inside DoInput itself.

### Tasks
1. Modify DoInput in `sc2/src/uqm/gameinp.c` to add event pumping under
   `#ifdef RUST_OWNS_MAIN`:
   ```c
   do {
       Async_process();
   #ifdef RUST_OWNS_MAIN
       TFB_ProcessEvents();
       ProcessUtilityKeys();
       TFB_FlushGraphics();
   #endif
       TaskSwitch();
       UpdateInputState();
       ...
   } while (InputFunc(...));
   ```

2. Add FFI externs for TFB_ProcessEvents, ProcessUtilityKeys, TFB_FlushGraphics
   in `c_main_extern.rs` (for Rust main.rs to call during splash and between
   activities as a safety net).

3. Verify all blocking loops use DoInput (search for DoInput callers).

### Tests
- Boot test: verify SDL events processed during gameplay (window doesn't freeze)
- Verify fullscreen toggle works during battle

---

## P04: Teardown Sequence

### Objective
Port the teardown sequence to Rust. Create safe wrapper functions.

### Tasks
1. Create teardown function in main.rs:
   ```rust
   fn teardown() {
       unsafe {
           TFB_UninitInput();
           unInitAudio();
           uninit_communication();
           TFB_PurgeDanglingGraphics();
           UninitColorMaps();
           TFB_UninitGraphics();
           Callback_uninit();
           Alarm_uninit();
           CleanupTaskSystem();
           UnInitTimeSystem();
           unprepareAllDirs();
           uninitIO();
           UnInitThreadSystem();
           mem_uninit();
       }
   }
   ```

2. Verify ordering matches C main() exactly.

### Tests
- Verify function is callable (compiles, links)

---

## P05: Rewrite Rust main.rs

### Objective
Replace the Phase-0 stub main.rs with the real init + game loop + teardown.

### Tasks
1. Port parseOptions to Rust (merge with existing cli.rs)
2. Port getUserConfigOptions (or call through C FFI for Phase 1)
3. Implement init sequence calling FFI externs
4. Call rust_game_loop() directly (no StartThread)
5. Call teardown() after game loop returns
6. Handle quit/abort cleanly

### Key change from current
- NO StartThread — game loop runs on main thread
- NO MainExited polling — game loop returns directly
- NO ProcessThreadLifecycles — eliminated
- Event pump is called from within the game loop via pump_events()

### Tests
- Tier-3: Boot test (Rust binary starts, reaches menu, shuts down)

---

## P06: Build Integration

### Objective
Make `cargo build` produce a working UQM binary.

### Tasks
1. Update Cargo.toml: add `[[bin]]` target
2. Update build.rs: link C object files into Rust binary
3. Add `#ifndef RUST_OWNS_MAIN` around C main() in uqm.c
4. Configure linker flags (SDL2, OpenGL, audio libs)
5. Verify the Rust binary links and runs

### Risk
The C build system (build.sh + Makefile.build) produces a binary, not a
library. We need to either:
a) Compile C to .o files, then link from Rust (preferred)
b) Compile C to a .a static library, then link from Rust
Option (b) is cleaner. Need to modify the C build to produce a .a instead of
a binary.

---

## P07: E2E Boot Test

### Objective
Verify the Rust binary boots UQM to the main menu, melee works, and shutdown
is clean.

### Tasks
1. Boot to main menu
2. Start Super Melee, pick ships, fight
3. Quit cleanly
4. Check for error messages, panics, resource leaks
