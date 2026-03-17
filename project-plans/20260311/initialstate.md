# Rust Port State — 2026-03-11

## Overview

This document captures the verified current state of the UQM Rust port as of 2026-03-11, derived from direct inspection of source code, build configuration, FFI boundaries, and C-side guard patterns. It serves as the root reference for the `project-plans/20260311/` plan set.

The Rust crate (`rust/`) is a `staticlib` linked into the C build. Every ported subsystem is gate-controlled by a `USE_RUST_*` preprocessor define in `sc2/config_unix.h`. As of this date **all 20 defines** are active (`#define`), meaning the game is running entirely on Rust for those subsystems. The C originals are compiled-out or shimmed through.

**Crate stats:** 122 `.rs` source files under `rust/src/`, organized into 14 top-level modules. 135 `#[cfg(test)]` blocks indicate broad unit-test coverage. The `Cargo.toml` lists `sdl2`, `rodio`/`cpal`, `lewton`/`ogg`, `crossbeam`, `parking_lot`, `image`, `xbrz-rs`, and `fast_image_resize` as key dependencies.

Subsystem docs under this date directory will expand only the partially-ported and unported systems. Already-ported systems remain summarized in this document and are not replanned unless regressions surface.

---

## Verified Active Rust Systems

These subsystems are fully wired: C code is guarded out, Rust FFI symbols resolve at link time, and the game exercises the Rust code paths in normal play.

### Clock / Time (`USE_RUST_CLOCK`)
- **Rust modules:** `time::{mod, game_date, game_clock, events, clock_bridge, ffi}` (6 files)
- **C shim:** `clock_rust.c` calls `rust_clock_init`, `rust_clock_tick`, etc. Original `clock.c` has `#error` guard.
- **Evidence:** 22 `#[no_mangle]` FFI exports in `time/ffi.rs`. `GameDate`, `GameClock`, `SharedGameClock` have full unit tests. Clock bridge handles init/uninit/tick/advance/lock/unlock lifecycle.

### State Helpers & State File I/O (`USE_RUST_STATE`)
- **Rust modules:** `state::{game_state, planet_info, state_file, ffi}` (4 files)
- **C shim:** `globdata.c` calls `rust_get_game_state_bits_from_bytes`, `rust_set_game_state_bits_in_bytes`, `rust_get_game_state32_from_bytes`, etc. `state.c` calls `rust_open_state_file`, `rust_read_state_file`, `rust_write_state_file`, `rust_init_planet_info`, `rust_get_planet_info`, `rust_put_planet_info`.
- **Evidence:** 28 `#[no_mangle]` FFI exports in `state/ffi.rs`. Bitfield read/write, state-file open/close/seek/length, and planet-info get/put are all wired. Unit tests cover game-state bit manipulation, state-file operations, and planet-info round-trips.

### Input / VControl (`USE_RUST_INPUT`)
- **Rust modules:** `input::{vcontrol, keyboard, joystick, keynames, templates, ffi}` (6 files)
- **C shim:** `rust_vcontrol_impl.c` calls `rust_VControl_HandleEvent`, `rust_VControl_AddGestureBinding`, etc. Original `vcontrol.c` has `#error` guard.
- **Evidence:** 33 `#[no_mangle]` FFI exports. Global `VCONTROL` state protected by `RwLock`. Full keyboard/joystick binding, gesture parsing, control template system. Tested.

### Threading Primitives (`USE_RUST_THREADS`)
- **Rust modules:** `threading::{mod, tests}` (single large module, ~700 lines)
- **C shim:** `rust_thrcommon.c` wraps every threading call — thread spawn/join, mutex create/lock/unlock/depth, semaphore create/acquire/release, condvar create/wait/signal/broadcast, thread-local create/destroy, `TaskSwitch`, `HibernateThread`. Original `thrcommon.c` guarded out.
- **Evidence:** Complete set of FFI exports for `Thread`, `UqmMutex` (recursive), `UqmCondVar` (generation-counted), `Semaphore` (counting), thread-local storage with FFI guard. All primitives have unit tests.

### Low-Level Audio Mixer / Backend (`USE_RUST_AUDIO`, `USE_RUST_MIXER`)
- **Rust modules:** `sound::mixer::{buffer, source, resample, mix, types, ffi}` (6 files) + `sound::rodio_backend` + `sound::rodio_audio`
- **C shim:** `audiocore_rust.c` calls `rust_mixer_Init`, `rust_mixer_GenSources`, `rust_mixer_BufferData`, etc. (30+ extern declarations). Original `mixer.c` guarded out.
- **Evidence:** 26 `#[no_mangle]` exports in `mixer/ffi.rs`. Full OpenAL-like API: buffer management, source lifecycle, resampling, mixing. `rodio_backend.rs` (875 lines) implements a complete `audio_Driver` using rodio/cpal. Tested.

### Decoder Vtables (`USE_RUST_OGG`, `USE_RUST_WAV`, `USE_RUST_MOD`, `USE_RUST_DUKAUD`, `USE_RUST_AIFF`)
- **Rust modules:** `sound::{ogg, wav, wav_ffi, mod_decoder, mod_ffi, dukaud, dukaud_ffi, aiff, aiff_ffi, ffi, decoder, formats, null}` (13+ files)
- **C integration:** `decoder.c` conditionally includes Rust vtable headers and inserts `rust_ova_DecoderVtbl`, `rust_wav_DecoderVtbl`, `rust_mod_DecoderVtbl`, `rust_duka_DecoderVtbl`, `rust_aifa_DecoderVtbl` into the decoder table.
- **Evidence:** Each decoder (OGG via `lewton`, WAV native, MOD via `mod_player`, DukAud native, AIFF native) implements the `SoundDecoder` trait and exposes a C-compatible vtable. All have `#[cfg(test)]` blocks.

### Resource Bridge (`USE_RUST_RESOURCE`)
- **Rust modules:** `resource::{index, loader, cache, dispatch, resource_system, resource_type, type_registry, propfile, stringbank, config_api, ffi, ffi_bridge, ffi_types, tests}` (15 files)
- **C shim:** `rust_resource.c` calls `rust_cache_init`, `rust_resource_load`, `rust_cache_get`, `rust_resource_exists`, etc. C files `getres.c`, `resinit.c`, `loadres.c`, `propfile.c`, `filecntl.c` all guarded out.
- **Evidence:** 25+ `#[no_mangle]` exports in `resource/ffi.rs`. LRU cache (64 MiB default), resource indexing, prop-file parsing, string-bank management, type registry. 7 dedicated test modules in `tests.rs`.

### Video Decoder & Player (`USE_RUST_VIDEO`, `USE_RUST_VIDPLAYER`)
- **Rust modules:** `video::{mod, decoder, player, scaler, ffi}` (5 files)
- **C shim:** `rust_video.c` calls `rust_play_video`, `rust_stop_video`, `rust_process_video_frame`, `rust_get_video_position`. `videodec.c` inserts `rust_dukv_DecoderVtbl`. Original `vidplayer.c` guarded out.
- **Evidence:** 16 `#[no_mangle]` exports in `video/ffi.rs`. DukVid header parsing, delta decoding, frame management, pixel format conversion all implemented and tested. Player integrates with SDL surface blitting.

### Graphics (`USE_RUST_GFX`)
- **Rust modules:** `graphics::{drawable, frame, pixmap, font, context, render_context, gfx_common, cmap, dcqueue, tfb_draw, scaling, scaling_new, sdl::{common, sdl2, opengl}, ffi, canvas_ffi, cmap_ffi, dcq_ffi}` (20 files)
- **C shim:** `sdl_common.c` initializes `rust_backend` with `rust_gfx_preprocess`, `rust_gfx_postprocess`, `rust_gfx_screen`, `rust_gfx_color`, `rust_gfx_init`, etc. Scaler C files (`2xscalers_sse.c`, `2xscalers_mmx.c`, `2xscalers_3dnow.c`) guarded out.
- **Evidence:** 17 `#[no_mangle]` exports in `graphics/ffi.rs`. Full drawable registry, frame registry, pixmap registry, color-map manager with fade support, draw-command queue (DCQ) with batching, font rendering, canvas primitives, bilinear/trilinear/nearest scaling, OpenGL and SDL2 software backend drivers. Extensive unit tests across all submodules.

### Memory Allocator (`USE_RUST_MEM`)
- **Rust module:** `memory.rs` (single file)
- **C integration:** `w_memlib.c` has `#error` guard. `rust_hmalloc`, `rust_hfree`, `rust_hcalloc`, `rust_hrealloc`, `rust_mem_init`, `rust_mem_uninit` are linked directly.
- **Evidence:** Thin wrappers over `libc::malloc`/`free`/`realloc` with abort-on-OOM. Tested.

### File I/O (`USE_RUST_FILE`)
- **Rust modules:** `io::{dirs, files, temp, ffi, mod}` (5 files)
- **C integration:** `files.c` has `#error` guard. 5 `#[no_mangle]` exports in `io/ffi.rs`.
- **Evidence:** Directory operations, file copy/delete/exists/size, temp-file management. All have unit tests.

### Bridge / Logging (`USE_RUST_BRIDGE`)
- **Rust modules:** `bridge_log.rs`, `logging.rs`, `c_bindings.rs`, `cli.rs`, `config.rs`
- **C integration:** `uqm.c` calls `rust_bridge_init()` at startup.
- **Evidence:** `rust_bridge_log` called from C code across multiple shim files.

### Game Init (partial Rust)
- **Rust modules:** `game_init::{init, master, setup, ffi}` (4 files)
- **Evidence:** 15 `#[no_mangle]` FFI exports. Init/uninit space/ships with ref-counted guards. Note: `init.rs` bodies are largely stub/placeholder ("In a real implementation, this would: …") — the FFI surface exists but the init logic mostly forwards to C.

---

## Partially Ported Systems Needing Parity Work

These subsystems have Rust code and active `USE_RUST_*` defines, but code inspection reveals incomplete implementations, stub bodies, feature-gated sections, or heavy C fallback dependency.

### UIO / Virtual File System (`USE_RUST_UIO`)
- **Rust module:** `io/uio_bridge.rs` (2,483 lines — the largest single file in the crate)
- **Status:** Mount-point registry, `uio_DirHandle`/`uio_Stream` types, `uio_fopen`/`uio_fread`/`uio_fclose`/`uio_fstat` implemented with stdio backend. ZIP filesystem support is partial (mount registry tracks `UIO_FSTYPE_ZIP` but extraction relies on C). Some operations still call into C (`uio_fread_shim.c` exists as a C-side bridge back into Rust).
- **Gap:** No native ZIP archive reading. Directory enumeration for mounted ZIPs falls back to C. The bidirectional C↔Rust shim layer (`uio_fread_shim.c`) suggests not all I/O paths are fully Rust-owned yet.

### Audio Heart / High-Level Sound (`USE_RUST_AUDIO_HEART`, feature-gated)
- **Rust modules:** `sound::{stream, trackplayer, music, sfx, control, fileinst, types, heart_ffi}` (8 files, ~5,400 lines total)
- **Status:** `heart_ffi.rs` (1,475 lines) is the largest FFI layer. Streaming engine, track player, music/speech playback, SFX with positional audio, and volume/fade control are implemented. Feature-gated behind `#[cfg(feature = "audio_heart")]` in Cargo and `USE_RUST_AUDIO_HEART` in C.
- **Gap:** `#![allow(dead_code, unused_imports, unused_variables)]` at top of `stream.rs`, `music.rs`, `sfx.rs`, `trackplayer.rs` signals incomplete integration. The `heart_ffi.rs` has multiple `#[cfg(test)]` mock blocks for C externs (`uio_fopen`, `AllocStringTable`), indicating dependency on C-side UIO and resource systems that are not yet Rust-native from this module's perspective. The C files (`stream.c`, `trackplayer.c`, `music.c`, `sfx.c`, `sound.c`, `fileinst.c`) are guarded out only when this define is active, which is separate from the core audio defines.

### Communication System (`USE_RUST_COMM`)
- **Rust modules:** `comm::{state, track, subtitle, response, animation, oscilloscope, types, ffi}` (8 files)
- **Status:** Init/uninit, track start/stop/rewind/jump/seek, subtitle chunking, response entry management, animation context, oscilloscope — all have FFI exports (48 `#[no_mangle]` in `comm/ffi.rs`). `rust_comm.c` shim wires `InitCommunication`/`UninitCommunication`.
- **Gap:** The C-side `comm.c` is only partially guarded (`#ifndef USE_RUST_COMM` at line 400, not wrapping the entire file). The comm system depends heavily on per-race dialogue scripts (`sc2/src/uqm/comm/{race}/*.c` — 25+ race-specific C files) which are untouched. The Rust side manages state and playback mechanics but the dialogue tree logic and callback dispatch remain C-owned.

### File / Memory Direct Integration
- **Status:** `memory.rs` is thin wrappers over libc; `io/` modules use `std::fs`. Both are wired and functional.
- **Gap:** No Rust-native allocator (still delegates to `libc::malloc`). The memory module exists for ABI compatibility, not for memory-safety improvement. Potential future work: arena allocators, tracked allocation for leak detection.

### Resource Full Takeover
- **Status:** Resource index, loader, cache, string bank, prop files, type registry, config API, dispatch — all implemented with extensive tests.
- **Gap:** `ffi_bridge.rs` has 11 `#[cfg(test)]` mock blocks and a `#[cfg(feature = "audio_heart")]` conditional. Some resource loading paths (especially those that produce audio handles or graphics handles) still call into C functions to complete the load. Full Rust resource ownership requires the downstream consumers (graphics subsystem, audio heart) to also accept Rust-native handles end-to-end.

---

## Unported Systems (Still C-Owned)

These systems have **no Rust code** and **no `USE_RUST_*` define**. They are entirely implemented in C.

### SuperMelee / Combat
- **C files:** `battle.c`, `battlecontrols.c`, `collide.c`, `process.c`, `pickship.c`, `melee.c`, `meleesetup.c`, `pickmele.c`, `buildpick.c`, `loadmele.c` (10 files) + `ship.c`, `cyborg.c`, `intel.c`, `tactrans.c`
- **Scope:** Combat engine, battle controls, collision detection, element processing, melee setup/pick screens, AI. Deeply coupled to the element dispatch list, ship `RaceDesc` structures, and frame-level physics.

### Ships (All 26 Races)
- **C files:** `sc2/src/uqm/ships/{race}/{race}.c` — 28 files covering all playable races plus `lastbat` and `probe`.
- **Scope:** Per-ship weapon logic, special abilities, thrust/turn characteristics, AI hints. Each file is self-contained but references global combat state, element manipulation functions, and ship descriptors.

### Planet / Solar System Gameplay
- **C files:** `sc2/src/uqm/planets/` — 16 files: `solarsys.c`, `lander.c`, `planets.c`, `plangen.c`, `scan.c`, `surface.c`, `gentopo.c`, `orbits.c`, `cargo.c`, `devices.c`, `roster.c`, `pstarmap.c`, `report.c`, `calc.c`, `oval.c`, `pl_stuff.c`
- **Scope:** Solar system navigation, planetary landing, terrain generation, mineral/bio scanning, orbit calculations, cargo management, starmap. Heavy math and procedural generation.

### Broader Campaign / Gameplay Flow
- **C files:** `hyper.c`, `ipdisp.c`, `encount.c`, `gameev.c`, `gameinp.c`, `gameopt.c`, `galaxy.c`, `grpinfo.c`, `globdata.c`, `save.c`, `load.c`, `load_legacy.c`, `restart.c`, `setup.c` (in uqm/), `fmv.c`, `intro.c`, `credits.c`, `outfit.c`, `menu.c`, `flash.c`, `border.c`, `cnctdlg.c`, `confirm.c`, `demo.c`, `gendef.c`, `getchar.c`, `displist.c`, `oscill.c`, `misc.c`, `build.c`, `cleanup.c`, `cons_res.c`, `plandata.c`, `loadship.c`, `master.c`, `dummy.c`
- **Scope:** Hyperspace travel, interplanetary display, encounter logic, game events, save/load, main menu, outfit screen, full game loop orchestration.
- **Note:** `globdata.c` has partial Rust integration (state bitfield helpers) but campaign-flow logic is entirely C.

### Comm Dialogue Scripts
- **C files:** `sc2/src/uqm/comm/{race}/` — 25+ race-specific dialogue trees (e.g., `arilouc.c`, `vuxc.c`, `orzc.c`)
- **Scope:** NPC dialogue text, branching conversation trees, phrase enable/disable logic. These are content-heavy, algorithmically simple, and tightly coupled to `comm.c` dispatch.

### Netplay
- **C files:** `sc2/src/uqm/supermelee/netplay/` — 17 files
- **Scope:** Network connection, packet serialization, net input, net state sync, checksumming. Currently disabled in the game but represents a distinct subsystem.

---

## Recommended Subsystem Work Order

Priority is based on three factors: (1) dependency depth — systems that other unported systems depend on, (2) proximity to existing Rust FFI boundaries — systems adjacent to already-ported code, (3) risk reduction — eliminating unsafe C↔Rust interop surface area.

| Priority | Subsystem | Rationale |
|----------|-----------|-----------|
| **1** | UIO full takeover (native ZIP, remove C shim) | Every file-loading path passes through UIO. Eliminating the bidirectional C↔Rust shim removes a fragile interop surface and unblocks full Rust ownership of resource loading. |
| **2** | Audio heart stabilization | Feature-gated and marked with `allow(dead_code)`. Stabilize stream/trackplayer/music/sfx to remove the feature gate and `allow` annotations. Prerequisite for comm audio to be fully Rust-owned. |
| **3** | Resource full takeover | Close the gap where resource loads still call back into C for handle creation. Requires UIO (#1) and benefits from audio heart (#2) for audio resource loads. |
| **4** | Comm dialogue + dispatch integration | Extend Rust comm beyond state management into dialogue tree dispatch. Race-specific scripts are algorithmically simple but numerous. Can be done incrementally per-race. |
| **5** | Game init completion | Replace stub bodies in `game_init/init.rs` with real initialization logic. Depends on graphics and resource systems being fully Rust-owned. |
| **6** | Planet / solar system logic | Large, math-heavy. Low external dependency (mostly uses graphics + state + resource). Can be ported independently once foundations (#1–#3) are solid. |
| **7** | Ships (per-race) | 28 self-contained files. Each can be ported independently. Blocked on combat engine (#8). |
| **8** | SuperMelee / combat engine | Core element-processing loop, collision, physics. Must precede or accompany ship ports. Largest single-system effort. |
| **9** | Campaign / gameplay flow | Hyperspace, encounters, save/load flow, menus, game loop. Depends on most other systems. Natural final milestone. |
| **10** | Netplay | Currently disabled. Lowest priority. Can be ported or reimplemented independently at any point. |

---

## Notes on Evidence Standards

- **"Wired"** means: `USE_RUST_*` is `#define`d in `sc2/config_unix.h`, the C original has a `#ifndef`/`#error`/`#ifdef` guard that compiles it out, and the Rust FFI symbols are linked and exercised at runtime.
- **"Tested"** means: the Rust module contains `#[cfg(test)]` blocks with assertions that exercise core logic. It does not imply integration-test or end-to-end coverage.
- **"Stub"** means: FFI surface exists and links, but function bodies contain placeholder comments ("In a real implementation…") or `#![allow(dead_code)]` at module level.
- **Line counts** and file counts are from `rust/src/` excluding `target/` build artifacts and `.bak` files.
- The `sc2/config_unix.h` inspected is the generated config, not the `.in` template — it reflects the actual build-time state.
- All assessments are based on source inspection. No runtime profiling or coverage instrumentation was used.
