# Recommended Port Order — July 9, 2026

## Strategy

The user wants chunks that are neither too small (40 lines) nor too big (10k lines).
We go **bottom-up for leaf libraries**, then **top-down for game logic**, ensuring each
chunk has clear boundaries and can be independently verified.

The key principle: **port the things that unblock the most other ports first**.
Graphics and strings are depended on by virtually everything. MikMod and network
are leaf modules that few things depend on.

---

## Phase 1: Leaf Utility Libraries (unblock everything)

### 1.1 — libs/strings/ (1,703 lines → ~1,500 Rust)
**Why first**: Used by every alien dialog, every UI string, every menu. No
dependencies on other unported modules. Clean, self-contained.
**Chunk size**: ~1,700 lines C
**Scope**:
- `getstr.c` (643) — string table lookup
- `strings.c` (347) — string management
- `unicode.c` (541) — unicode conversion
- `stringhashtable.c` (67), `sfileins.c` (50), `sresins.c` (55)
**Verify**: String display in menus and dialogs works correctly.
**Rust target**: `rust/src/strings/` (new module)

### 1.2 — libs/math/ + libs/md5/ + libs/list/ (871 lines → ~800 Rust)
**Why**: Tiny leaf modules, unblock state/save, collections.
**Chunk size**: ~871 lines C
**Scope**:
- `random.c` (101), `random2.c` (89), `sqrt.c` (97) — RNG and math
- `md5.c` (452) — hashing for save games (use `md5` crate)
- `list.c` (132) — generic linked list
**Verify**: Save game checksums, RNG reproducibility.
**Rust target**: Expand `rust/src/math/mod.rs`, new `rust/src/md5.rs`, expand `rust/src/collections/`

### 1.3 — libs/decomp/ + libs/callback/ + libs/heap/ (1,621 lines → ~1,200 Rust)
**Why**: Leaf libraries needed by save/load and timer systems.
**Chunk size**: ~1,621 lines C
**Scope**:
- `lzdecode.c` (415), `lzencode.c` (468), `update.c` (115) — LZ compression
- `alarm.c` (177), `async.c` (56), `callback.c` (193) — timer callbacks
- `heap.c` (197) — memory-mapped heap
**Verify**: Save game compression, timer callbacks.
**Rust target**: `rust/src/decomp/`, `rust/src/callback/`, expand `rust/src/collections/`

---

## Phase 2: Graphics Core (the critical path)

This is the biggest blocker. The Rust graphics modules exist (23K lines) but are
dormant. We need to wire them in and compile out the C equivalents.

### 2.1 — Graphics primitives: boxint, clipline, intersec, bbox, pixmap (1,156 lines)
**Why**: Leaf geometry modules that everything else in graphics depends on.
**Chunk size**: ~1,156 lines C
**Scope**:
- `boxint.c` (183), `clipline.c` (241), `intersec.c` (415) — geometry
- `bbox.c` (133) — bounding box
- `pixmap.c` (170) — pixel map
**Rust target**: `rust/src/graphics/` — pixmap.rs exists, add intersection/clipline/boxint
**Verify**: No visual regressions in drawing.

### 2.2 — Graphics core: frame, drawable, context, font, cmap (2,368 lines)
**Why**: Core data structures that the DCQ and tfb_draw operate on.
**Chunk size**: ~2,368 lines C
**Scope**:
- `frame.c` (266), `drawable.c` (501), `context.c` (404) — core abstractions
- `font.c` (334) — font rendering
- `cmap.c` (663) — colormap
- `filegfx.c` (72), `resgfx.c` (54), `loaddisp.c` (65) — gfx helpers
**Rust target**: `rust/src/graphics/` — frame.rs, drawable.rs, context.rs, cmap.rs, font.rs exist
**Verify**: Images render, fonts display, colormaps work.

### 2.3 — DCQ + tfb_draw + gfx_common (1,399 lines) ⚡ CRITICAL
**Why**: This is the rendering pipeline. The Rust equivalents exist but are dormant.
Wiring this in eliminates the largest single block of actively-running C code.
**Chunk size**: ~1,399 lines C
**Scope**:
- `dcqueue.c` (685) — draw command queue
- `tfb_draw.c` (518) — draw screen operations
- `gfx_common.c` (196) — common graphics facade
**Rust target**: `rust/src/graphics/dcqueue.rs` (1362 lines), `rust/src/graphics/tfb_draw.rs` (3405 lines) — **EXIST, need wiring**
**Verify**: Full rendering works — menu, battle, planets, comm.
**Note**: The flicker fix we just made (FlushGraphicsEx skip_swap) must be replicated.

### 2.4 — Graphics loading + widgets (1,538 lines)
**Why**: gfxload loads images from resources; widgets are UI components.
**Chunk size**: ~1,538 lines C
**Scope**:
- `gfxload.c` (597) — graphics loading
- `widgets.c` (941) — UI widgets (buttons, scrollbars, etc.)
**Rust target**: New modules in `rust/src/graphics/`
**Verify**: UI widgets (buttons, sliders) work in menus.

### 2.5 — SDL backend: canvas + primitives + sdl_common (3,205 lines)
**Why**: The SDL drawing surface operations. Rust SDL backend exists but
canvas operations are still C.
**Chunk size**: ~3,205 lines C
**Scope**:
- `sdl/canvas.c` (2176) — canvas blit/fill operations
- `sdl/primitives.c` (633) — drawing primitives
- `sdl/sdl_common.c` (396) — SDL common (SwapBuffers etc.)
- `sdl/sdl2_common.c` (222), `sdl/sdl2_pure.c` (465) — SDL2 specific
**Rust target**: `rust/src/graphics/sdl/` — common.rs, sdl2.rs exist
**Verify**: All rendering backends work.

### 2.6 — SDL scalers + rotozoom + hq2x (5,583 lines)
**Why**: Image scaling and rotation. Can be deferred if scaling quality
isn't critical for initial port.
**Chunk size**: ~5,583 lines C (largest chunk — could split)
**Scope**:
- `sdl/hq2x.c` (2888), `sdl/rotozoom.c` (1038) — HQ scaler + rotation
- `sdl/biadv2x.c` (532), `sdl/bilinear2x.c` (112), `sdl/nearest2x.c` (207) — scalers
- `sdl/triscan2x.c` (155), `sdl/2xscalers.c` (260) — more scalers
- `sdl/pure.c` (474), `sdl/png2sdl.c` (300) — pure software + PNG
- `sdl/sdluio.c` (153), `sdl/palette.c` (47) — misc
- 3DNow/MMX/SSE variants (356 lines) — guarded by `USE_RUST_GFX` already
**Rust target**: `rust/src/graphics/scaling.rs` (3470 lines) exists
**Verify**: Scaling modes work (2x, hq2x, bilinear).

---

## Phase 3: Sound Cleanup

### 3.1 — Sound stream + trackplayer (1,704 lines)
**Why**: Partially guarded but still active C code.
**Chunk size**: ~1,704 lines C
**Scope**:
- `stream.c` (819) — audio stream
- `trackplayer.c` (885) — music track player
**Rust target**: `rust/src/sound/stream.rs` (1795 lines), `rust/src/sound/trackplayer.rs` (1637 lines) — EXIST
**Verify**: Music plays correctly during gameplay.

### 3.2 — Sound sfx + music + misc (803 lines)
**Why**: Remaining sound management.
**Chunk size**: ~803 lines C
**Scope**:
- `sfx.c` (316), `music.c` (237), `sound.c` (183), `fileinst.c` (89), `resinst.c` (65)
**Rust target**: `rust/src/sound/sfx.rs` (474 lines), `rust/src/sound/music.rs` (392 lines) — EXIST
**Verify**: SFX and music work.

### 3.3 — MikMod (16,418 lines) — OPTIONAL/LAST
**Why**: Module music player. Only used for .mod/.xm/.it files. Could
use Rust crate (e.g., `mod_player` or similar) or keep as external lib.
**Chunk size**: 16,418 lines C (very large — would need to split by module)
**Defer unless needed**: The `USE_RUST_MOD` flag routes mod loading to Rust,
but the mikmod player code is still compiled. Could potentially just drop
all mikmod C and use a pure Rust crate.

---

## Phase 4: Remaining I/O & Input

### 4.1 — UIO remaining C (10,000 lines)
**Why**: Large C codebase still active despite UIO bridge.
**Chunk size**: Split into 2-3 chunks of ~3,000-4,000 lines
**Scope (chunk A)**: zip.c (1680), ioaux.c (930), debug.c (914), stdio/stdio.c (854)
**Scope (chunk B)**: mounttree.c (814), gphys.c (620), paths.c (602), uiostream.c (608), match.c (569), utils.c (497)
**Scope (chunk C)**: fileblock.c (332), fstypes.c (272), memdebug.c (293), hashtable.c (374), mount.c (168), physical.c (174), charhashtable.c (77), defaultfs.c (41), uio_fread_shim.c (14)
**Rust target**: Expand `rust/src/io/`
**Verify**: File reading, zip extraction, path resolution work.

### 4.2 — Input remaining C (867 lines)
**Why**: input.c and keynames.c still C.
**Chunk size**: ~867 lines C
**Scope**:
- `input.c` (638) — SDL input handling
- `keynames.c` (229) — key name mapping
- `input_common.c` (20) — trivial
**Rust target**: `rust/src/input/` — keyboard.rs, joystick.rs exist
**Verify**: Keyboard, joystick input works.

### 4.3 — Threading SDL backend (706 lines)
**Why**: SDL thread implementation still C.
**Chunk size**: ~706 lines C
**Scope**: `sdl/sdlthreads.c` (706)
**Rust target**: `rust/src/threading/` (1427 lines) exists
**Verify**: Thread creation, mutex, semaphore work.

---

## Phase 5: Game Logic (Top-Down)

### 5.1 — Game state + save/load (2,108 lines)
**Why**: State management and save/load — partially ported, needs completion.
**Chunk size**: ~2,108 lines C
**Scope**:
- `state.c` (520), `save.c` (813), `load.c` (774)
**Rust target**: `rust/src/state/` (2365 lines) exists, expand
**Verify**: Save/load works.

### 5.2 — Flash + trans + menu + misc UI (2,287 lines)
**Why**: Flash overlay (we just fixed!), transitions, menus.
**Chunk size**: ~2,287 lines C
**Scope**:
- `flash.c` (805), `trans.c` (154), `menu.c` (603), `displist.c` (274), `confirm.c` (250), `border.c` (200)
**Rust target**: New modules in `rust/src/game/`
**Verify**: Flash overlays, transitions, menus work (including the flicker fix!).

### 5.3 — Setup + settings + gameopt (3,062 lines)
**Why**: Game configuration.
**Chunk size**: ~3,062 lines C
**Scope**: `setupmenu.c` (1613), `gameopt.c` (1347), `setup.c` (332), `settings.c` (102)
**Verify**: Setup menu works.

### 5.4 — Hyperspace + galaxy + encounter (3,055 lines)
**Why**: Core space travel gameplay.
**Chunk size**: ~3,055 lines C
**Scope**: `hyper.c` (1747), `encount.c` (844), `galaxy.c` (464)
**Verify**: Hyperspace travel, encounters work.

### 5.5 — SIS + status + starbase + shipyard + outfit (5,319 lines)
**Why**: Player ship management and station UI.
**Chunk size**: ~5,319 lines C (could split into 2)
**Scope**: `sis.c` (1741), `shipyard.c` (1495), `outfit.c` (795), `status.c` (582), `starbase.c` (602)
**Verify**: Ship management UI works.

### 5.6 — Planets system (~20,000 lines)
**Why**: Planetary exploration — largest game logic block.
**Chunk size**: Split into 3-4 chunks of ~5,000 lines
**Scope (A)**: solarsys.c (2021), plangen.c (1954), pstarmap.c (1631)
**Scope (B)**: lander.c (2101), scan.c (1385), plandata.c (1850)
**Scope (C)**: devices.c (690), orbits.c (629), planets.c (483), roster.c (428), cargo.c (356), report.c (271), surface.c (251), pl_stuff.c (318), oval.c (329), calc.c (530), gentopo.c (206)
**Scope (D)**: generate/ (all ~30 files, ~5,000 lines) — planet generation scripts
**Verify**: Planet exploration, scanning, lander, generation work.

### 5.7 — Alien dialog scripts (~17,000 lines)
**Why**: Individual alien conversations.
**Chunk size**: ~17,000 lines total, but each alien is ~500-2000 lines (port 3-5 at a time)
**Scope**: 25+ alien dialog files (arilouc.c, blackurc.c, chmmrc.c, etc.)
**Verify**: Each alien's dialog works.
**Note**: Comm framework is already Rust; these are data/script ports.

### 5.8 — Ship race implementations (~13,000 lines)
**Why**: Individual ship behavior.
**Chunk size**: Each ship is ~300-1000 lines (port 3-5 at a time)
**Scope**: 25+ ship files (androsyn.c, arilou.c, etc.)
**Verify**: Each ship plays correctly in battle.
**Note**: Ship framework is already Rust; Rust races/ already has implementations.

### 5.9 — Process loop + battle + remaining game logic (~5,000 lines)
**Why**: Core game processing, battle setup, remaining modules.
**Chunk size**: ~5,000 lines C
**Scope**: `process.c` (1108), `battle.c` (517), `battlecontrols.c` (100), `collide.c` (183), `velocity.c` (153), `weapon.c` (414), `gravity.c` (200), `tactrans.c` (1032), `gameinp.c` (525), `gameev.c` (729), `grpinfo.c` (867), `intro.c` (875), `ipdisp.c` (779), `credits.c` (839), `cyborg.c` (1339), `getchar.c` (442), `misc.c` (407), `util.c` (312), `demo.c` (141), `dummy.c` (207), `fmv.c` (134), `intel.c` (76), `starmap.c` (125), `sounds.c` (199), `pickship.c` (501), `cnctdlg.c` (630), `cleanup.c` (99), `cons_res.c` (112), `gendef.c` (137)
**Note**: Many of these are already partially Rust (battle/, etc.)

### 5.10 — Super Melee netplay (~4,000 lines) — LAST
**Why**: Netplay protocol, lowest priority.
**Scope**: All files in `uqm/supermelee/netplay/`
**Defer**: Until everything else is ported.

---

## The C→Rust Crossing Problem

The key issue identified: every time we port a C module to Rust, the *remaining*
C code that called that module now has to cross the FFI boundary. This means:

1. Write Rust replacement for the C module
2. Create `extern "C"` shims in Rust that expose the same API the C callers expect
3. Update the C headers so C code calls the Rust shims instead
4. Remove the C implementation (compile out with `#ifndef USE_RUST_XXX`)

The FFI crossing work is proportional to the **number of C callers** and the
**API surface** (number of exported functions).

### FFI Surface Analysis

| Module | C Lines | C Callers | API Functions | FFI Bridge Work |
|--------|---------|-----------|---------------|-----------------|
| Strings | 1,703 | 135 sites in ~20 files | 19 | LOW — small API, C side just calls `GAME_STRING()` |
| Math | 287 | 248 sites in ~20 files | 7 | LOW — tiny API, but many call sites |
| Graphics core | 3,600 | 1,260 sites in ~40 files | 78 | HIGH — huge API, many callers |
| MD5 | 452 | 4 sites | 5 | TRIVIAL |
| Decomp | 998 | 1 site | 3 | TRIVIAL |
| Callback | 426 | 2 sites | 6 | TRIVIAL |
| Heap | 197 | 15 sites | 5 | LOW |

### The "Callers First" Strategy

The user's insight: port the **callers** of a leaf library first, so that when
the leaf library is ported, there are no C callers left that need FFI shims.
This minimizes FFI bridge work.

However, this only works if the callers themselves are ready to be ported. Many
callers of strings are unported game logic (melee.c, scan.c, sis.c, gameopt.c)
that depend on graphics, sound, and many other C modules.

**Practical hybrid**: Port leaf libraries that have the FEWEST external callers
first (trivial FFI bridge), then port their callers, working upward.

### Revised Priority Based on FFI Cost

1. **MD5** (4 callers, 5 API funcs) — trivial FFI, zero risk
2. **Decomp** (1 caller, 3 API funcs) — trivial FFI
3. **Callback** (2 callers, 6 API funcs) — trivial FFI
4. **Heap** (15 callers, 5 API funcs) — low FFI
5. **Math** (248 callers, 7 API funcs) — moderate FFI, but many call sites
6. **Strings** (135 callers, 19 API funcs) — moderate FFI
7. **Graphics** (1260 callers, 78 API funcs) — HIGH FFI, defer until callers ported

## Recommended Next Port: Phase 1.0 — Trivial Leaf Libraries

Start with the modules where the FFI bridge is trivial:

### 1.0a — MD5 (452 lines, 4 callers)
- Port `libs/md5/md5.c` to Rust (use `md5` crate or hand-port)
- Create `#[no_mangle] extern "C" fn` shims for the 5 API functions
- Guard C code with `#ifndef USE_RUST_MD5`
- **FFI cost**: 5 shims, 4 call sites to redirect

### 1.0b — Decomp (998 lines, 1 caller)
- Port `libs/decomp/lzdecode.c`, `lzencode.c`, `update.c` to Rust
- Create FFI shims for 3 API functions
- **FFI cost**: 3 shims, 1 call site

### 1.0c — Callback (426 lines, 2 callers)
- Port `libs/callback/alarm.c`, `async.c`, `callback.c` to Rust
- Create FFI shims for 6 API functions
- **FFI cost**: 6 shims, 2 call sites

### 1.0d — Heap (197 lines, 15 callers)
- Port `libs/heap/heap.c` to Rust
- Create FFI shims for 5 API functions
- **FFI cost**: 5 shims, 15 call sites

Then proceed to math and strings (the moderate-FFI leaf libraries),
then graphics (the high-FFI core).

**Total for Phase 1.0**: ~2,073 lines C → ~1,500 lines Rust, with only 19 FFI
shims and 22 call site redirects. Low risk, high confidence.