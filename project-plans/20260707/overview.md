# UQM Porting Project — State Assessment

*Assessed 2026-07-07 via direct code, build, and test inspection (not docs).*

## Architecture

This is a **C-primary / Rust-library** hybrid, not a full rewrite. The Rust
code (`rust/`) builds as a static library `libuqm_rust.a` that the original C
binary links against. Subsystems are ported incrementally via `#ifdef
USE_RUST_*` **dispatch guards** in the C source — when a flag is on, the C
function delegates to a `rust_*` bridge function; otherwise it runs the
original C.

```
C binary (sc2/) ──links──▶ libuqm_rust.a (rust/)
         │
         └─ #ifdef USE_RUST_SHIPS → rust_ships_load(...)
            #else                 → c_load_ship(...)
```

The Rust codebase is ~122K LOC across ~190 files, organized into 12
subsystems. It contains **zero** `todo!()` / `unimplemented!()` panics;
implementations reference their C source line numbers.

## Build & Test Status — GREEN

| Check | Result |
|-------|--------|
| Rust compile (`cargo build`) | Passes (356 warnings — mostly FFI signature mismatches) |
| Rust tests (`cargo test --lib`) | **2580 passed, 0 failed, 6 ignored** |
| C+Rust link | Produces `uqm` binary (13.7 MB arm64 Mach-O) |
| Binary runs | `--help` works; full boot through all subsystems to main menu, clean shutdown |

## Verified Runtime (fresh run, 2026-07-07)

A clean compile + run was performed. The binary boots through every Rust
subsystem in sequence with **zero errors or warnings**:

1. SDL 2.32.10 init → Rust memory + thread systems up
2. Rust UIO mounts config, content, and 4 addon packs (3domusic, 3dovideo,
   3dovoice, remix)
3. **Rust graphics driver** active (3 screen surfaces, 320x240)
4. **Rust mixer** + cpal audio output stream started
5. Sound banks loaded and WAV-decoded via Rust decoders
6. Main menu reached (input-polling loop running)
7. Clean shutdown: `Starcon2Main done (returned 0)` → Rust memory deinitialized

The DCQ livelock and `libpng` errors seen in the Feb 2026 logs did **not**
reproduce in this run.

## What's Actually Ported & Active

**All 22 `USE_RUST_*` flags are enabled** in `build.vars`. Confirmed genuine
bridge integration (real implementations, not stubs):

### Engine / I-O layer (fully Rust at runtime)
- **File I/O** (`FILE`, `UIO`) — `RUST_UIO: uio_openDir` dominates run logs
- **Audio** (`AUDIO`, `OGG`, `MOD`, `WAV`, `DUKAUD`, `AIFF`, `MIXER`)
- **Video** (`VIDEO`, `VIDPLAYER`) — DUK decoder, scalers; logs show
  `intro.duk` playing
- **Graphics** (`GFX`) — primitives, DCQ, SDL/OpenGL drivers
- **Core systems** (`CLOCK`, `THREADS`, `MEM`, `INPUT`, `RESOURCE`, `BRIDGE`)

### Game systems (bridged via genuine C-to-Rust dispatch)
- **Ships** (`SHIPS`) — 43 files, ~20K LOC. All 25 races have Rust
  implementations. Bridges: `rust_ships_load/free/spawn/build/death/catalog`.
- **Communication** (`COMM`) — 20 files, ~10.6K LOC. Dialogue/encounter loop,
  NPCPhrase, subtitles, response UI.
- **State** (`STATE`) — game-state get/set bits, planet info, state-file I/O.
- **Super Melee** (`SUPERMELEE`) — setup, team/build, pick melee.

## What's NOT Ported — Still Pure C (~20K LOC, no bridge)

| Subsystem | Files |
|-----------|-------|
| Main game loop | `process.c`, `encount.c`, `uqm.c`, `starcon.c`, `restart.c` |
| Galaxy / hyperspace | `galaxy.c`, `starmap.c` |
| Planet exploration | `plandata.c`, `intel.c` |
| Menus / UI | `menu.c`, `gameopt.c`, `setupmenu.c`, `settings.c`, `setup.c`, `confirm.c` |
| SIS management | `sis.c`, `shipyard.c`, `outfit.c`, `status.c`, `build.c` |
| Save/Load | `save.c`, `load.c`, `load_legacy.c` |
| Intro/credits | `intro.c`, `credits.c`, `fmv.c`, `demo.c` |

## What's Broken / In-Process

1. **Battle loop is Rust-written but NOT wired.** There's a 12K-LOC
   `rust/src/battle/` module and a `rust_battle_wrappers.c` bridge — but it's
   gated behind a **separate** `USE_RUST_BATTLE_LOOP` flag that is **absent
   from `build.vars`** (disabled). At runtime `battle.c` runs pure C.

2. **356 compiler warnings** — mostly FFI signature mismatches (e.g.,
   `uio_read` declared as `*mut uio_Handle` in one module, `*mut c_void` in
   another). Latent soundness bugs in the FFI layer.

3. **Runtime graphics issue (did not reproduce on 2026-07-07).** The Feb 2026
   run log showed threads repeatedly `blocking on 'DCQ'` (Deferred Command
   Queue) and `libpng error: Not a PNG file`. The fresh run reached the main
   menu with neither symptom. These may be resolved or content/flag-dependent;
   they warrant re-verification under gameplay (intro video, melee) before
   declaring them fixed.

4. **Loose files in repo root** — `draw_fontchar_impl.rs`, `test_copy_fix`,
   `test_copy_fix.rs` are stray artifacts that don't belong to the build.

## Summary

| Area | Status |
|------|--------|
| Engine layer (I/O, audio, video, graphics, timing, memory) | Ported & active |
| Ships, Comm, State, Super Melee | Ported & active (bridged) |
| Battle loop | Ported in Rust but **not wired** (disabled flag) — runs in C |
| Main game loop, galaxy, planets, menus, save/load, SIS | Not started (pure C, ~20K LOC) |
| Build + tests | Green (2580 tests pass) |
| Runtime | Boots clean to main menu (verified); deeper gameplay paths unverified |

**Bottom line:** The porting strategy has successfully extracted and
Rust-ified the **engine subsystems and several game systems**. The project
compiles, links, runs clean to the main menu, and has strong test coverage
(2580 tests). However, the **core single-player game logic** (~20K LOC)
remains entirely in C, and the Rust **battle engine is written but not
connected** (`USE_RUST_BATTLE_LOOP` disabled). The foremost next step is
wiring the Rust battle loop and verifying gameplay paths (intro, melee,
combat) beyond the main menu.
