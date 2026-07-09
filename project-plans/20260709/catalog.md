# UQM Port Catalog — July 9, 2026

## Overview

| Metric | C | Rust |
|--------|------|------|
| Total lines | 190,654 | 132,917 |
| Subsystems | ~25 | ~20 |

The Rust binary owns `main()`, the game loop, and all subsystem orchestration. C code runs as transitional legacy, called via FFI. The end goal is zero C code in our codebase.

## Status Legend

- **PORTED**: Rust module exists, compiles, is wired in via `USE_RUST_XXX`, and the C code is either fully compiled out or is a thin shim
- **BRIDGED**: Rust module exists and is called, but C code still runs the same function (dual path or partial)
- **PARTIAL**: Rust module exists but not wired in; C code still fully active (Rust module is dormant)
- **NOT STARTED**: No Rust module; C code is the sole implementation

---

## 1. Memory & Utility Layer

### libs/memory/w_memlib.c — 88 lines
- **Status**: PORTED (`USE_RUST_MEM`)
- C file is fully guarded out; all `HAlloc`/`HFree`/`HUninit` go to Rust
- Rust: `rust/src/memory.rs` (341 lines)

### libs/heap/heap.c — 197 lines
- **Status**: NOT STARTED
- Memory-mapped heap allocator (for save files)
- Rust: no equivalent

### libs/math/ — 287 lines (random.c, random2.c, sqrt.c)
- **Status**: NOT STARTED
- RNG and fast sqrt implementations
- Rust: `rust/src/math/mod.rs` (373 lines) — exists but PARTIAL (not wired in)

### libs/list/list.c — 132 lines
- **Status**: NOT STARTED
- Generic linked list
- Rust: `rust/src/collections/queue.rs` (466 lines) — exists, not wired

### libs/md5/md5.c — 452 lines
- **Status**: NOT STARTED
- MD5 hashing (for save game checksums)
- Rust: no equivalent (should use `md5` crate)

### libs/decomp/ — 998 lines (lzdecode, lzencode, update)
- **Status**: NOT STARTED
- LZ compression/decompression
- Rust: no equivalent

### libs/cdp/ — 1377 lines (cdp, cdpapi, windl)
- **Status**: NOT STARTED
- CD player abstraction (legacy, likely dead code)

### getopt/ — 1250 lines
- **Status**: NOT STARTED
- Command-line option parsing (replaced by `rust/src/cli.rs`)
- Rust: `rust/src/cli.rs` (408 lines) — BRIDGED (Rust handles CLI, C getopt not called)

### regex/regex.c — 99 lines
- **Status**: NOT STARTED

### abxadec/abxaud.c — 638 lines
- **Status**: NOT STARTED
- Audio decompression for ABX format

### libs/log/ — 331 lines (uqmlog.c, msgbox_stub, msgbox_win)
- **Status**: NOT STARTED
- Logging — Rust has `rust/src/logging.rs` (206 lines) + `rust/src/bridge_log.rs` (107 lines)
- Actually BRIDGED — Rust logging is active, C log still compiled but called from C code paths

---

## 2. Threading

### libs/threads/ — 2319 lines
- **Status**: PARTIAL (`USE_RUST_THREADS`)
- `rust_thrcommon.c` (486 lines) — shim calling Rust; ACTIVE
- `thrcommon.c` (455 lines) — guarded by `#ifndef USE_RUST_THREADS` (compiled out)
- `sdl/sdlthreads.c` (706 lines) — NOT guarded, still compiled and active
- `pthread/posixthreads.c` (672 lines) — compiled but not used on macOS?
- Rust: `rust/src/threading/mod.rs` (1427 lines) + tests (971 lines)
- **Key gap**: SDL thread backend still C

---

## 3. Time

### libs/time/ — 71 lines
- **Status**: PORTED (`USE_RUST_CLOCK`)
- `sdl/sdltime.c` and `timecommon.c` — thin wrappers
- `uqm/clock.c` (318 lines) + `uqm/clock_rust.c` (228 lines) — clock_rust is the shim
- Rust: `rust/src/time/` (2687 lines) — game_clock, game_date, events, ffi, clock_bridge

---

## 4. I/O (UIO)

### libs/uio/ — 11,925 lines
- **Status**: BRIDGED (`USE_RUST_UIO`)
- `io.c` (1864 lines) — guarded sections route to Rust, but file open/read still C
- `uio_bridge.rs` (7603 lines) — massive FFI bridge, ACTIVE
- Rust: `rust/src/io/` (9944 lines) — dirs, files, temp, zip_reader, uio_bridge
- **Key unported**: uio/zip/zip.c (1680 lines), ioaux.c (930 lines), debug.c (914 lines), mounttree.c (814 lines), gphys.c (620 lines), paths.c (602 lines), uiostream.c (608 lines), match.c (569 lines), stdio/stdio.c (854 lines), utils.c (497 lines), fileblock.c (332 lines), fstypes.c (272 lines), memdebug.c (293 lines), hashtable.c (374 lines), mount.c (168 lines), physical.c (174 lines), charhashtable.c (77 lines), defaultfs.c (41 lines)

### libs/file/ — 1199 lines (dirs.c, files.c, temp.c)
- **Status**: PORTED (`USE_RUST_FILE`)
- `files.c` fully guarded out
- Rust: `rust/src/io/files.rs`, `rust/src/io/dirs.rs`, `rust/src/io/temp.rs`

---

## 5. Resource Management

### libs/resource/ — 1663 lines
- **Status**: BRIDGED (`USE_RUST_RESOURCE`)
- `rust_resource.c` (127 lines) — shim, ACTIVE
- `loadres.c`, `getres.c`, `filecntl.c`, `propfile.c`, `resinit.c` — guarded by `#ifndef USE_RUST_RESOURCE` (compiled out)
- `direct.c` (101 lines), `stringbank.c` (181 lines) — NOT guarded, still C
- Rust: `rust/src/resource/` (6476 lines) — dispatch, ffi_bridge, propfile, resource_type, type_registry, tests

---

## 6. Strings

### libs/strings/ — 1703 lines
- **Status**: NOT STARTED
- `getstr.c` (643 lines), `strings.c` (347 lines), `unicode.c` (541 lines) — all C, no Rust guards
- Rust: no direct equivalent (some unicode handling may be in other modules)

---

## 7. Graphics

### libs/graphics/ — 18,206 lines
- **Status**: PARTIAL (`USE_RUST_GFX`)
- Rust SDL backend (`rust_gfx_preprocess`, `rust_gfx_screen`, `rust_gfx_postprocess`) is ACTIVE for the SDL rendering path
- **C code still fully active** (no `USE_RUST_GFX` guards on core files):
  - `dcqueue.c` (685 lines) — Draw Command Queue, the core rendering pipeline ⚡
  - `tfb_draw.c` (518 lines) — Draw screen operations ⚡
  - `context.c` (404 lines) — Graphics context management
  - `drawable.c` (501 lines) — Drawable/surface abstraction
  - `cmap.c` (663 lines) — ColorMap
  - `frame.c` (266 lines) — Frame abstraction
  - `pixmap.c` (170 lines) — Pixel map operations
  - `font.c` (334 lines) — Font rendering
  - `gfx_common.c` (196 lines) — Common graphics facade
  - `gfxload.c` (597 lines) — Graphics loading
  - `bbox.c` (133 lines) — Bounding box
  - `boxint.c` (183 lines) — Box intersection
  - `clipline.c` (241 lines) — Line clipping
  - `intersec.c` (415 lines) — Intersection routines
  - `loaddisp.c` (65 lines) — Load display pixmap
  - `filegfx.c` (72 lines), `resgfx.c` (54 lines) — Graphics file/resource helpers
  - `widgets.c` (941 lines) — UI widgets
  - `tfb_prim.c` (237 lines) — Primitives
  - SDL backend: `canvas.c` (2176 lines), `hq2x.c` (2888 lines), `rotozoom.c` (1038 lines), `primitives.c` (633 lines), `pure.c` (474 lines), `opengl.c` (575 lines), `png2sdl.c` (300 lines), `scalers.c` (295 lines), `sdl_common.c` (396 lines), `sdl2_common.c` (222 lines), `sdl2_pure.c` (465 lines), `sdl1_common.c` (247 lines), `sdluio.c` (153 lines), all 2xscaler variants
- Rust: `rust/src/graphics/` (23,360 lines) — dcqueue.rs, tfb_draw.rs, ffi.rs, sdl/, context, drawable, cmap, frame, pixmap, font, gfx_common, scaling — **all exist but are DORMANT (not wired into the rendering pipeline)**

---

## 8. Sound

### libs/sound/ — 9,623 lines
- **Status**: BRIDGED (`USE_RUST_AUDIO`, `USE_RUST_AUDIO_HEART`, `USE_RUST_MIXER`, `USE_RUST_OGG`, `USE_RUST_WAV`, `USE_RUST_DUKAUD`, `USE_RUST_AIFF`, `USE_RUST_MOD`)
- Rust audio heart is ACTIVE — sound playback goes through Rust
- `audiocore_rust.c` (405 lines) — shim to Rust, ACTIVE
- `mixer/mixer.c` (1769 lines) — guarded by `USE_RUST_MIXER`, Rust mixer active
- Decoders: `decoder.c` (978 lines) — guarded, routes to Rust decoders
- **Still C (not guarded out)**:
  - `stream.c` (819 lines) — partially guarded by `USE_RUST_AUDIO_HEART`
  - `trackplayer.c` (885 lines) — partially guarded by `USE_RUST_AUDIO_HEART`
  - `sfx.c` (316 lines), `music.c` (237 lines), `sound.c` (183 lines), `fileinst.c` (89 lines), `resinst.c` (65 lines) — partially guarded
  - `decoders/aiffaud.c` (650 lines), `decoders/dukaud.c` (546 lines), `decoders/modaud.c` (430 lines), `decoders/wav.c` (385 lines), `decoders/oggaud.c` (278 lines) — actual decoder implementations, guarded out
  - `mixer/sdl/audiodrv_sdl.c` (486 lines), `mixer/nosound/audiodrv_nosound.c` (410 lines) — audio drivers
- Rust: `rust/src/sound/` (18,952 lines) — mixer, heart_ffi, stream, trackplayer, sfx, music, all decoders, rodio backend

### libs/mikmod/ — 16,418 lines
- **Status**: NOT STARTED (no USE_RUST flag)
- Module music player (IT, MOD, S3M, XM, STM formats)
- `mplayer.c` (3429 lines), `mdulaw.c` (2119 lines), `virtch.c` (1351 lines), `virtch2.c` (1368 lines), `virtch_common.c` (475 lines), `load_it.c` (1038 lines), `mdriver.c` (971 lines), `load_xm.c` (840 lines), `mloader.c` (666 lines), `load_mod.c` (514 lines), `load_s3m.c` (480 lines), `mmio.c` (528 lines), `sloader.c` (530 lines), `load_stm.c` (382 lines), `mwav.c` (388 lines), `mlutil.c` (330 lines), `mmerror.c` (315 lines), `munitrk.c` (303 lines), `mmalloc.c` (142 lines), `npertab.c` (48 lines), `mdreg.c` (43 lines), `mlreg.c` (51 lines), `drv_nos.c` (107 lines)
- **Note**: The `USE_RUST_MOD` flag routes mod loading to Rust but mikmod player code is still compiled
- Rust: `rust/src/sound/mod_ffi.rs` (422 lines) exists for mod loading

---

## 9. Video

### libs/video/ — 2,240 lines
- **Status**: PORTED (`USE_RUST_VIDEO`, `USE_RUST_VIDPLAYER`)
- `rust_video.c` (138 lines) — shim, ACTIVE
- `video.c`, `videodec.c`, `vidplayer.c` — guarded, routed to Rust
- `dukvid.c` (748 lines), `legacyplayer.c` (81 lines) — still compiled but may be dead
- Rust: `rust/src/video/` (3540 lines) — decoder, player, scaler, ffi

---

## 10. Input

### libs/input/ — 2,290 lines
- **Status**: BRIDGED (`USE_RUST_INPUT`)
- `rust_vcontrol_impl.c` (99 lines) — shim, ACTIVE
- `vcontrol.c` (1304 lines) — guarded by `USE_RUST_INPUT`, C implementation compiled out
- `input.c` (638 lines), `keynames.c` (229 lines) — NOT guarded, still C
- `input_common.c` (20 lines) — trivial
- Rust: `rust/src/input/` (4098 lines) — vcontrol, keyboard, joystick, keynames, ffi, templates

---

## 11. Network

### libs/network/ — 2,943 lines
- **Status**: NOT STARTED
- BSD/Win socket abstraction, netmanager, connection handling
- No Rust equivalent
- **Note**: Only used for Super Melee netplay; may be lowest priority

---

## 12. Callback

### libs/callback/ — 426 lines
- **Status**: NOT STARTED
- Timer/callback system (alarm.c, async.c, callback.c)
- No Rust equivalent, but Rust threading may handle this

---

## 13. Game Logic (uqm/)

### Core game loop & top-level
- `uqm.c` (1726 lines) — PORTED (`RUST_OWNS_MAIN`) — main entry point is Rust, C uqm.c is guarded out
- `uqm/process.c` (1108 lines) — NOT STARTED — core game activity processing
- `uqm/starcon.c` (182 lines) — NOT STARTED — Star Control constants
- `uqm/init.c` (362 lines) — PARTIAL (`USE_RUST_SHIPS`) — initialization, ship init routed to Rust
- `uqm/globdata.c` (571 lines) — PARTIAL (`USE_RUST_STATE`) — global game data
- `uqm/state.c` (520 lines) — PARTIAL (`USE_RUST_STATE`) — save/load state, partially Rust
- `uqm/gameinp.c` (525 lines) — NOT STARTED — game input handling (partially RUST_OWNS_MAIN for event pumping)

### Restart/Menu
- `uqm/restart.c` (429 lines) — PORTED (`USE_RUST_RESTART`) — `do_restart.rs` (902 lines) active
- `rust_bridge_restart.c` (269 lines) — shim
- `uqm/setup.c` (332 lines), `uqm/setupmenu.c` (1613 lines) — NOT STARTED
- `uqm/settings.c` (102 lines) — NOT STARTED

### Comm (alien dialog)
- `uqm/comm.c` (1860 lines) — PARTIAL (`USE_RUST_COMM`) — comm FFI bridge active
- `uqm/rust_comm.c` (2156 lines) — bridge shim, ACTIVE
- `uqm/commglue.c` (430 lines) — guarded, partially Rust
- `uqm/commanim.c` (623 lines) — NOT STARTED
- `uqm/oscill.c` (191 lines) — NOT STARTED
- Alien dialog files (~25 files, ~17,000 lines) — NOT STARTED (individual alien conversation scripts)
  - arilouc.c, blackurc.c, chmmrc.c, comandr.c, druugec.c, ilwrathc.c, melnorm.c, myconc.c, orzc.c, pkunkc.c, rebel.c, shofixtc.c, slyhome.c, slyland.c, spahome.c, spathic.c, starbas.c, supoxc.c, syreenc.c, talkpet.c, thraddc.c, umgahc.c, urquanc.c, utwigc.c, vuxc.c, yehatc.c, zoqfotc.c
- Rust: `rust/src/comm/` (10,629 lines) — animation, encounter, hail, state, talk_segue, subtitle, response, ffi, etc. — ACTIVE for comm framework, but individual alien scripts still C

### Battle
- `uqm/battle.c` (517 lines) — NOT STARTED — battle setup
- `uqm/battlecontrols.c` (100 lines) — NOT STARTED
- `uqm/collide.c` (183 lines) — NOT STARTED — collision detection
- `uqm/velocity.c` (153 lines) — NOT STARTED
- `uqm/weapon.c` (414 lines) — NOT STARTED
- `uqm/gravity.c` (200 lines) — NOT STARTED
- `uqm/rust_battle_wrappers.c` (129 lines) — shim
- Rust: `rust/src/battle/` (13,141 lines) — process_loop, element, collision, velocity, weapon, tactical, ai, gravity, display_list, netplay — ACTIVE

### Ships
- `uqm/ship.c` (580 lines) — PARTIAL (`USE_RUST_SHIPS`)
- `uqm/loadship.c` (228 lines) — PARTIAL (`USE_RUST_SHIPS`)
- `uqm/build.c` (560 lines) — PARTIAL (`USE_RUST_SHIPS`)
- `uqm/master.c` (245 lines) — PARTIAL (`USE_RUST_SHIPS`)
- `uqm/shipstat.c` (437 lines) — NOT STARTED
- `uqm/rust_bridge_ships.c` (673 lines) — shim, ACTIVE
- Ship race implementations (~25 files, ~13,000 lines) — NOT STARTED (individual ship behavior scripts)
  - androsyn.c, arilou.c, blackurq.c, chenjesu.c, chmmr.c, druuge.c, human.c, ilwrath.c, lastbat.c, melnorme.c, mmrnmhrm.c, mycon.c, orz.c, pkunk.c, probe.c, shofixti.c, sis_ship.c, slylandr.c, spathi.c, supox.c, syreen.c, thradd.c, umgah.c, urquan.c, utwig.c, vux.c, yehat.c, zoqfot.c
- Rust: `rust/src/ships/` (19,867 lines) — registry, loader, runtime, types, ffi, queue, writeback, lifecycle, catalog, battle_bridge, races/ — ACTIVE for ship framework, individual race implementations exist in `races/`

### Planets
- `uqm/planets/` (~20,000 lines total) — NOT STARTED
  - `solarsys.c` (2021 lines) — solar system exploration
  - `lander.c` (2101 lines) — planetary lander
  - `plangen.c` (1954 lines) — planet generation
  - `pstarmap.c` (1631 lines) — star map
  - `scan.c` (1385 lines) — planetary scan
  - `calc.c` (530 lines), `devices.c` (690 lines), `planets.c` (483 lines), `roster.c` (428 lines), `orbits.c` (629 lines), `report.c` (271 lines), `surface.c` (251 lines), `pl_stuff.c` (318 lines), `oval.c` (329 lines), `cargo.c` (356 lines), `gentopo.c` (206 lines)
  - `generate/` (~30 files, ~5,000 lines) — individual planet generation scripts
- Rust: no equivalent

### Super Melee
- `uqm/supermelee/melee.c` (2654 lines) — PARTIAL (`USE_RUST_SUPERMELEE`)
- `uqm/supermelee/meleesetup.c` (529 lines) — PARTIAL
- `uqm/supermelee/loadmele.c` (826 lines), `pickmele.c` (957 lines), `buildpick.c` (221 lines) — NOT STARTED
- `uqm/supermelee/netplay/` (~4,000 lines) — NOT STARTED — netplay protocol
- Rust: `rust/src/supermelee/` (3445 lines) — setup, config, team, persistence, pick_melee, build_pick, netplay_boundary — ACTIVE

### Other game modules
- `uqm/hyper.c` (1747 lines) — NOT STARTED — hyperspace travel
- `uqm/cyborg.c` (1339 lines) — NOT STARTED — cyborg ship selection
- `uqm/tactrans.c` (1032 lines) — NOT STARTED — tactical transitions
- `uqm/credits.c` (839 lines) — NOT STARTED — credits screen
- `uqm/shipyard.c` (1495 lines) — NOT STARTED — shipyard
- `uqm/outfit.c` (795 lines) — NOT STARTED — ship outfitting
- `uqm/sis.c` (1741 lines) — NOT STARTED — SIS (player ship) display
- `uqm/ipdisp.c` (779 lines) — NOT STARTED — IP display
- `uqm/intro.c` (875 lines) — NOT STARTED — intro sequence
- `uqm/encount.c` (844 lines) — NOT STARTED — encounter handling
- `uqm/grpinfo.c` (867 lines) — NOT STARTED — group info
- `uqm/gameopt.c` (1347 lines) — NOT STARTED — game options
- `uqm/menu.c` (603 lines) — NOT STARTED — menu system
- `uqm/starbase.c` (602 lines) — NOT STARTED — starbase
- `uqm/status.c` (582 lines) — NOT STARTED — status display
- `uqm/save.c` (813 lines) — NOT STARTED — save/load
- `uqm/load.c` (774 lines), `load_legacy.c` (821 lines) — NOT STARTED
- `uqm/flash.c` (805 lines) — NOT STARTED — flash overlay system
- `uqm/displist.c` (274 lines) — NOT STARTED — display list
- `uqm/plandata.c` (1850 lines) — NOT STARTED — planet data
- `uqm/gameev.c` (729 lines) — NOT STARTED — game events
- `uqm/galaxy.c` (464 lines) — NOT STARTED — galaxy
- `uqm/misc.c` (407 lines) — NOT STARTED — misc utilities
- `uqm/getchar.c` (442 lines) — NOT STARTED — character input
- `uqm/util.c` (312 lines) — NOT STARTED — utilities
- `uqm/confirm.c` (250 lines) — NOT STARTED — confirmation dialogs
- `uqm/demo.c` (141 lines) — NOT STARTED — demo mode
- `uqm/dummy.c` (207 lines) — NOT STARTED — dummy/stub
- `uqm/fmv.c` (134 lines) — NOT STARTED — FMV playback
- `uqm/trans.c` (154 lines) — NOT STARTED — transitions
- `uqm/border.c` (200 lines) — NOT STARTED — border drawing
- `uqm/cnctdlg.c` (630 lines) — NOT STARTED — connection dialog
- `uqm/cleanup.c` (99 lines) — NOT STARTED
- `uqm/cons_res.c` (112 lines) — NOT STARTED
- `uqm/gendef.c` (137 lines) — NOT STARTED
- `uqm/intel.c` (76 lines) — NOT STARTED
- `uqm/starmap.c` (125 lines) — NOT STARTED
- `uqm/sounds.c` (199 lines) — NOT STARTED
- `uqm/pickship.c` (501 lines) — NOT STARTED
- `uqm/uqmdebug.c` (1926 lines) — NOT STARTED — debug utilities

### Rust bridge files (C side)
- `rust_bridge_macros.c` (135 lines) — provides real function symbols for C macros/static-inlines that Rust FFI declares as extern "C" fn
- `rust_bridge_main2.c` (241 lines), `rust_bridge_mainloop.c` (204 lines) — main loop bridges
- `rust_comm.c` (2156 lines) — comm system bridge, ACTIVE

---

## Summary by Status

### FULLY PORTED (C compiled out or thin shim only)
1. Memory management (`USE_RUST_MEM`)
2. File I/O (`USE_RUST_FILE`)
3. Clock/Time (`USE_RUST_CLOCK`)
4. Video playback (`USE_RUST_VIDEO`, `USE_RUST_VIDPLAYER`)
5. Main entry point (`RUST_OWNS_MAIN`)
6. Restart menu (`USE_RUST_RESTART`)

### BRIDGED (Rust active, C still partially runs)
1. Threading (`USE_RUST_THREADS`) — SDL thread backend still C
2. UIO (`USE_RUST_UIO`) — large C codebase still active
3. Resource loading (`USE_RUST_RESOURCE`) — direct.c, stringbank.c still C
4. Sound (`USE_RUST_AUDIO` + sub-flags) — stream, trackplayer, sfx partially C
5. Input (`USE_RUST_INPUT`) — input.c, keynames.c still C
6. Graphics (`USE_RUST_GFX`) — Rust SDL backend active, but DCQ/tfb_draw/context all C
7. Comm framework (`USE_RUST_COMM`) — framework in Rust, alien scripts in C
8. Ships framework (`USE_RUST_SHIPS`) — framework in Rust, race implementations dual
9. Super Melee (`USE_RUST_SUPERMELEE`) — partially active
10. State (`USE_RUST_STATE`) — partially active

### PARTIAL (Rust module exists, not wired in)
1. Graphics core (dcqueue.rs, tfb_draw.rs exist but dormant)
2. Math (mod.rs exists, not wired)

### NOT STARTED (No Rust module)
1. Strings (1703 lines)
2. MikMod (16,418 lines)
3. Network (2,943 lines)
4. Callback (426 lines)
5. Heap (197 lines)
6. MD5 (452 lines)
7. Decompression (998 lines)
8. CDP (1377 lines)
9. ABX audio (638 lines)
10. Regex (99 lines)
11. Planets system (~20,000 lines)
12. Alien dialog scripts (~17,000 lines)
13. Ship race implementations (~13,000 lines, though Rust races/ exists)
14. Super Melee netplay (~4,000 lines)
15. Most game logic modules (hyper, cyborg, tactrans, credits, shipyard, outfit, sis, etc.)

---

## Unported C Line Count by Subsystem (approximate)

| Subsystem | C Lines | Priority | Dependencies |
|-----------|---------|----------|--------------|
| Graphics core (DCQ, tfb_draw, context, drawable, cmap, frame, pixmap) | ~3,600 | HIGH | Everything visual |
| Graphics SDL backend (canvas, scalers, primitives, rotozoom, hq2x) | ~10,000 | HIGH | Graphics core |
| Strings | 1,703 | HIGH | Used everywhere |
| Sound (remaining C parts) | ~3,000 | MED | Sound heart active |
| MikMod | 16,418 | LOW | Mod music only |
| UIO (remaining C parts) | ~10,000 | MED | IO bridge active |
| Network | 2,943 | LOW | Netplay only |
| Input (remaining input.c, keynames.c) | 867 | MED | Input bridge active |
| Threading (SDL backend) | 706 | MED | Everything |
| Planets | ~20,000 | MED | Game logic |
| Alien dialog scripts | ~17,000 | LOW | Comm framework active |
| Ship race implementations | ~13,000 | LOW | Ship framework active |
| Game logic (hyper, cyborg, setup, etc.) | ~30,000 | MED | Game loop |
| Callback | 426 | LOW | Timer system |
| Misc (heap, md5, decomp, cdp, regex, abxadec) | ~3,000 | LOW | Various |