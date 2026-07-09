# UQM C→Rust Porting Strategy: Dependency-Driven Testable Clusters

**Generated:** 2026-07-09
**Method:** Static analysis (grep, find, obj/ inspection, #include graph, USE_RUST_* build flags)
**Goal:** ZERO C code. Every C file is either in a porting cluster or marked dead-to-delete.

---

## 0. Ground Truth: What's Actually Compiled

The authoritative "live" set is the set of `.c.o` files in `sc2/obj/release/`. I compared this against all 383 `.c` files under `sc2/src`.

| Category | Files | Lines |
|---|---|---|
| **Live** (compiled into current binary) | 339 | **171,684** |
| **Dead** (never compiled) | 44 | **18,970** |
| **Total** | 383 | ~190,654 |

The build is driven by `sc2/build.vars`, whose `USE_RUST_*` flags reveal the bridging state:
`USE_RUST_BRIDGE, FILE, CLOCK, UIO, OGG, AUDIO, AUDIO_HEART, COMM, INPUT, VIDEO, VIDPLAYER, GFX, RESOURCE, MOD, WAV, DUKAUD, AIFF, THREADS, MIXER, MEM, STATE, SHIPS, SUPERMELEE, RESTART` and `RUST_OWNS_MAIN`.

**Critical correction to prior analysis:**
- **Decomp is NOT dead.** `decomp/lzdecode.c`, `lzencode.c`, `update.c` ARE compiled and their public API (`copen/cread/cwrite/cclose/cfilelength`) is called by `uqm/load_legacy.c` (legacy save decompression) and `uqm/demo.c` (input journaling). Decomp must be ported, not deleted.
- **"Bridged" does not mean the C file is gone.** Ships (USE_RUST_SHIPS=1), state, comm, etc. still compile their C files as thin `#ifdef USE_RUST_*` shims OR as data/dispatch tables. e.g. `state.c` has `#ifdef USE_RUST_STATE` bodies; all 28 ship `.c` files still compile because `dummy.c` dispatches `ARILOU_CODE_RES → &init_arilou` to build `RACE_DESC` tables. The 28 Rust race files mirror the AI logic 1:1, but the C data tables + dispatch are still the live entry point.

---

## 1. Dead Code — DELETE IMMEDIATELY (Cluster D0)

These files are never compiled and have no live callers. Deleting them is the cheapest possible progress and shrinks the surface before any real porting.

| File/Dir | Lines | Why dead | Verification |
|---|---|---|---|
| `libs/md5/md5.c` | 452 | Only self-refs (header guards). No callers. | `grep md5_` → only `md5.h` |
| `libs/cdp/cdp.c cdpapi.c windl.c` | 1,377 | Plugin system only referenced by `abxadec/abxaud.c`, itself dead | `grep cdp_` → only abxaud |
| `abxadec/abxaud.c` | 638 | Not compiled; depends on dead cdp | not in obj/ |
| `getopt/getopt.c getopt1.c` | 1,250 | Not compiled (options.c handles CLI) | not in obj/ |
| `regex/regex.c` | 99 | Not compiled | not in obj/ |
| `mem_wrapper.c` | (small) | Not compiled (USE_RUST_MEM) | not in obj/ |
| `src/setup.c` | — | Dead duplicate of `uqm/setup.c` | not in obj/ |
| **UIO C internals** replaced by Rust `uio_bridge.rs` (250KB): `io.c, ioaux.c, mount.c, mounttree.c, physical.c, fstypes.c, gphys.c, hashtable.c, match.c, memdebug.c, defaultfs.c, fileblock.c, utils.c, uiostream.c, stdio/stdio.c, uio_fread_shim.c` | ~9,000 | USE_RUST_UIO=1; not compiled | not in obj/ |
| **Platform-Win**: `network/*_win.c, socket_win.c, netmanager_win.c, wspiapiwrap.c, log/msgbox_win.c` | ~1,400 | darwin build; not compiled | not in obj/ |
| **Superseded drivers**: `sound/audiocore.c, mixer/mixer.c, openal/audiodrv_openal.c, decoders/oggaud.c` (Rust owns these), `libs/file/files.c, file/temp.c`, `libs/memory/w_memlib.c`, `threads/pthread/posixthreads.c`, `input/sdl/vcontrol.c`, `resource/rust_resource.c`(dup), `uqm/clock.c`(→clock_rust), `uqm/uqm.c`(0 lines), `uqm/rust_battle_wrappers.c` | ~5,000 | Not compiled | not in obj/ |

**Total dead: ~18,970 lines / 44 files.**
**Test:** `rm` the files, rebuild (`./build.sh uqm`), run smoke test. Binary must build and launch identically. FFI cost: **0**.

> WARNING: Before deleting each, run one confirmation grep for the file's exported symbols across the *live* set to guard against a symbol used through a macro. The greps above already did this for md5/cdp/decomp.

---

## 2. Dependency Graph of Live C (the real work)

### Leaf libraries (called by many, call almost nothing)

| Lib | Lines | Calls (deps) | Called-by (dependents) | Rust exists? |
|---|---|---|---|---|
| `libs/heap` | 197 | (none) | **only** `callback/alarm.c` | No |
| `libs/callback` (callback+alarm+async) | 426 | heap, list | uqm.c, gameinp, battle, sis, pickmele, netmelee, network/*, threads/*thrcommon | No |
| `libs/math` (random,random2,sqrt) | 287 | (none) | **67 files** — pure functions | Partial (RandomContext in ships) |
| `libs/list` | 132 | (none) | callback, resource/direct, options.c (+dead uio) | No |
| `libs/log` | 432 | (thread lock) | **116 files** | No |
| `libs/task` | 139 | threads | uqm.c, intro, util, gameinp, battle, starcon, video/vidplayer, input | No |
| `libs/strings` (getstr,unicode,strings,hashtable,sfileins,sresins) | 1,703 | resource, uio | **55 files** | No |

### Mid-tier libraries

| Lib | Lines | Depends on | Called-by | Rust status |
|---|---|---|---|---|
| `libs/decomp` | 998 | (self) | load_legacy.c, demo.c | No |
| `libs/resource` | 1,663 | strings, uio, list | **20 files** | Bridged (`rust_resource.c` shim + Rust) |
| `libs/network` | 2,943 | callback, socket, threads | **only netplay/** + battle/tactrans/melee integration | No |
| `libs/mikmod` | 16,418 | (self-contained MOD codec) | **only** `sound/decoders/modaud.c` | No |
| `libs/sound` | 9,623 live | mixer, decoders, resource | game-wide (music/sfx) | Bridged (audiocore_rust.c + Rust mixer/decoders) |
| `libs/video` | 2,240 | sound, graphics, mikmod | fmv.c, intro, comm | Bridged (rust_video.c + Rust video) |
| `libs/graphics` (+sdl) | 18,206 | resource, math, threads | **58 files** — biggest FFI surface | Bridged (23 Rust files + ffi shims) |
| `libs/input` | 2,270 | vcontrol, threads | gameinp, controls | Bridged (Rust input) |
| `libs/threads` | 941 | (SDL) | task, callback, everything | Bridged (rust_thrcommon.c + Rust) |

### Game logic (`uqm/`), grouped by feature area (largely independent)

| Feature area | Key files | Lines | Notes |
|---|---|---|---|
| **Ships (data+dispatch)** | 28× `ships/*/*.c` + `dummy.c`, `loadship.c`, `cyborg.c` | ~15,600 | Rust AI exists 1:1; migrate RACE_DESC tables + `dummy.c` dispatch |
| **Comm conversations** | 27× `comm/*/*.c` | 23,346 | Each alien is one self-contained file; comm engine already Rust |
| **Planets/surface** | `planets/*` (45 files) | 19,380 | solarsys, lander, plangen, scan, pstarmap, orbits, generate/* |
| **Supermelee** | `supermelee/melee.c, pickmele, meleesetup, loadmele, buildpick` | 5,187 | Rust supermelee/setup exists; finish + wire |
| **Netplay** | `supermelee/netplay/*` (20 files) | 4,248 | Tightly coupled to libs/network |
| **Hyperspace/starmap** | `hyper.c, galaxy.c, starmap.c, encount.c, grpinfo.c` | ~4,000 | |
| **Starbase/outfit/shipyard** | `starbase.c, outfit.c, shipyard.c, cnctdlg.c` | ~4,200 | |
| **SIS/status/HUD** | `sis.c, status.c, shipstat.c, gameopt.c, setupmenu.c` | ~5,700 | |
| **Save/load** | `save.c, load.c, load_legacy.c, state.c` | ~2,900 | load_legacy needs decomp |
| **Battle/tactical** | `battle.c, tactrans.c, process.c, gameinp.c, battlecontrols.c` | ~3,300 | Rust battle/ exists |
| **Main/init/core** | `src/uqm.c(1726)`, `init.c, setup.c, globdata.c, starcon.c, gameev.c` | ~4,000 | Last to port (owns C init) |

---

## 3. Caller-First Opportunities (avoid FFI shims)

The strategy's biggest lever: **port the callers of a library before the library itself, so the library has zero C callers and can be deleted outright** — no `extern "C"` shim, no bidirectional bridge.

1. **Network ← Netplay (STRONGEST).** `libs/network` (2,943) is called *only* by `supermelee/netplay/*` (4,248) plus a handful of netplay integration points in battle/tactrans/melee. **Port netplay + network as ONE cluster.** When done, zero C calls network → delete it with no shim. This single cluster removes 7,191 lines and needs **0 FFI shims** to the rest of the game (netplay integrates through already-bridged supermelee).

2. **Mikmod ← modaud.** `libs/mikmod` (16,418) is called *only* by `sound/decoders/modaud.c`. Rust already has `USE_RUST_MOD=1`. Confirm the Rust MOD decoder fully replaces modaud, then **mikmod + modaud delete together, 0 shims.** (16K lines gone for the price of finishing one decoder.)

3. **Heap ← Alarm.** `libs/heap` (197) is called *only* by `callback/alarm.c`. Port them as one unit; heap never needs an FFI boundary.

4. **Decomp ← load_legacy + demo.** Decomp (998) has exactly two callers. Port decomp *inside* the save/load cluster; it never needs a standalone shim.

5. **Comm conversations.** Each `comm/<race>/<race>c.c` is called only via `commglue.c`'s dispatch. The comm *engine* is already Rust (`rust/src/comm/*`). Porting conversations means moving data-driven dialogue into Rust behind the existing engine — the C dispatch shrinks per-race with **no new game-wide shims**.

6. **Ships.** 28 C ship files are reached only through `dummy.c`'s `CodeResToInitFunc` table. Rust AI already exists. Port the RACE_DESC data + move dispatch into Rust ships registry; delete `dummy.c` + all 28 C files together. Shims already exist (`rust_bridge_ships.c`) and get deleted at the end.

**Anti-pattern to avoid:** Do NOT port `math`, `log`, or `strings` early as standalone Rust libs with C callers — they have 67/116/55 C callers respectively. Porting them first forces you to build and maintain a huge FFI surface. Instead, port them **near the end**, after most callers are already Rust, so the residual C-caller shim is tiny.

---

## 4. Final Ordered Strategy (clusters)

Ordering principle: **(a) delete dead code, (b) do caller-first self-contained clusters that delete whole libs with 0 shims, (c) port game features top-down so leaf libs lose C callers, (d) port leaf libs last when their caller shim is minimal, (e) port core/init last.**

Each cluster is 500–5,000 lines except where a tightly-coupled unit (mikmod, comm) is naturally larger — splitting those would break coupling.

---

### Cluster D0 — Delete Dead Code
- **Files:** md5, cdp(+abxaud), getopt, regex, mem_wrapper, src/setup.c, uio C internals, Win platform files, superseded sound/file/thread/input drivers, clock.c, empty uqm/uqm.c. (§1)
- **Lines removed:** ~18,970
- **Depends on:** nothing
- **Depended on by:** nothing (verified)
- **Test:** rebuild + smoke test (`node scripts/start.js` / launch binary), byte-identical behavior.
- **FFI shims:** 0

---

### Cluster 1 — Heap + Callback/Alarm/Async
- **Files:** `libs/heap/heap.c`, `libs/callback/{callback,alarm,async}.c` (623 lines)
- **Depends on:** `list` (still C — call via FFI, or inline the tiny list usage), threads (bridged)
- **Depended on by:** uqm.c, gameinp, battle, sis, pickmele, netmelee, network/*, thrcommon → **~8 C caller files need shims** (small: `Alarm_add*`, `Callback_init/uninit`, `Async_process`)
- **Test:** unit-test the alarm min-heap ordering + timer expiry in Rust; integration: flash alarm in SIS, netplay keepalive still fires.
- **FFI shims:** ~10 functions (Alarm/Callback/Async public API), all trivial
- **Note:** heap needs 0 shims (internal to this cluster).

---

### Cluster 2 — Network + Netplay (caller-first, 0 net shims)
- **Files:** `libs/network/*` (2,943) + `uqm/supermelee/netplay/*` (4,248) = **7,191 lines**
- **Depends on:** callback (Cluster 1 [OK]), threads (bridged), sockets (BSD — port with it), Rust supermelee/setup (exists)
- **Depended on by:** battle/tactrans/melee netplay hooks (already partly Rust) → tiny integration shim
- **Test:** two-instance netplay melee handshake (connect, ready, reset, packet exchange, checksum); the `checksum.c`/`crc.c` give deterministic unit tests.
- **FFI shims:** ~3 (netplay entry points called from melee.c) — network lib itself needs **0** because netplay is its only caller and is ported simultaneously.

---

### Cluster 3 — Mikmod + MOD decoder (caller-first, 0 shims)
- **Files:** confirm `USE_RUST_MOD` Rust decoder fully covers `sound/decoders/modaud.c`, then delete `libs/mikmod/*` (16,418) + `modaud.c` (430) = **16,848 lines**
- **Depends on:** Rust sound mixer (bridged [OK])
- **Depended on by:** only modaud (deleted with it)
- **Test:** play a `.mod`/`.s3m`/`.xm` track, compare PCM output hash against C reference; regression on in-game module music.
- **FFI shims:** 0 (self-contained codec; if Rust MOD decoder incomplete, this cluster = finish it, then delete)

---

### Cluster 4 — Decomp + Legacy Save/Load
- **Files:** `libs/decomp/{lzdecode,lzencode,update}.c` (998) + `uqm/{load_legacy,save,load,state}.c` + `demo.c` (~3,500) = **~4,500 lines**
- **Depends on:** strings/resource (bridged/upcoming), globdata (core)
- **Depended on by:** menu save/load, demo journaling
- **Test:** load a known legacy `.sav`, verify game state hash; round-trip save→load; demo replay determinism.
- **FFI shims:** ~4 (save/load entry points from menu). Decomp needs 0 (internal to cluster).

---

### Clusters 5–7 — Comm Conversations (3 batches × ~9 aliens)
- **Files:** 27× `comm/<race>/*.c` (23,346), grouped into 3 clusters of ~7,800 lines each (e.g. batch A: arilou, blackur, chmmr, comandr, druuge, ilwrath, melnorm, mycon, orz).
- **Depends on:** Rust comm engine (`rust/src/comm/*` — exists, 20 files), strings (upcoming), graphics (bridged)
- **Depended on by:** `commglue.c` dispatch only
- **Test:** per-alien conversation script playthrough (dialogue tree, responses, animations) against C reference transcript.
- **FFI shims:** shrinks `commglue.c` per batch; 0 new game-wide shims (engine already Rust). Delete `commglue.c` after batch 3.

---

### Cluster 8 — Ships (data + dispatch)
- **Files:** 28× `ships/*/*.c` (13,943) + `dummy.c` (207) + `loadship.c`, `cyborg.c` = **~15,700 lines**
- **Depends on:** Rust ships registry + 28 Rust race files (exist 1:1), battle (Rust)
- **Depended on by:** `dummy.c` code-resource dispatch (ported into Rust ships loader)
- **Test:** each ship's RACE_DESC values (stats, weapon, special) match C; melee AI behavior parity per ship.
- **FFI shims:** delete `rust_bridge_ships.c` (673) at end. Net shim delta: negative.
- **Split option:** if 15.7K is too big, split into 3 clusters of ~9 ships each (each ship file is independent).

---

### Clusters 9–11 — Planets (3 batches)
- **Files:** `planets/*` (45 files, 19,380), split by sub-feature:
  - **9. Solar system + orbits + gen** (solarsys, orbits, planets, calc, generate/*, gensol) ~7,000
  - **10. Lander + surface + scan** (lander, scan, surface, pl_stuff, cargo, devices, roster) ~5,600
  - **11. Planet gen graphics + starmap** (plangen, pstarmap, oval, gentopo, report) ~4,500
- **Depends on:** graphics (bridged), math (upcoming), resource (bridged)
- **Test:** generate a known seed's solar system, verify planet types/positions; lander mineral pickup; starmap navigation.
- **FFI shims:** integration points to hyperspace/game (~5 each).

---

### Cluster 12 — Supermelee front-end
- **Files:** `supermelee/{melee,pickmele,meleesetup,loadmele,buildpick}.c` (5,187)
- **Depends on:** netplay (Cluster 2 [OK]), ships (Cluster 8 [OK]), Rust supermelee/setup (exists)
- **Test:** build a team, save/load melee config, pick ships, start a match.
- **FFI shims:** ~4; retires `rust` supermelee bridge shims.

---

### Cluster 13 — Hyperspace / Galaxy / Encounters
- **Files:** `hyper.c, galaxy.c, starmap.c, encount.c, grpinfo.c` (~4,000)
- **Depends on:** planets (Clusters 9–11 [OK]), comm (5–7 [OK]), math
- **Test:** hyperspace navigation, fuel use, encounter triggering, fleet groups.

---

### Cluster 14 — Starbase / Outfit / Shipyard
- **Files:** `starbase.c, outfit.c, shipyard.c, cnctdlg.c` (~4,200)
- **Test:** buy/sell modules, crew, fuel; ship refit.

---

### Cluster 15 — SIS / Status / HUD / Menus
- **Files:** `sis.c, status.c, shipstat.c, gameopt.c, setupmenu.c, menu.c, gameinp.c` (~6,500)
- **Depends on:** input (bridged), graphics (bridged), callback (Cluster 1 [OK])
- **Test:** HUD rendering, options menu, in-game menu navigation.

---

### Cluster 16 — Battle / Tactical runtime
- **Files:** `battle.c, tactrans.c, process.c, battlecontrols.c, weapon.c, gendef.c, collide.c, gravity.c, velocity.c` (~3,500)
- **Depends on:** ships (8 [OK]), Rust battle/ (exists)
- **Test:** full melee combat determinism vs C; collision, gravity, weapon fire.
- **FFI shims:** retires `rust_battle_wrappers.c`, `rust_bridge_ships.c` remnants.

---

### Cluster 17 — Residual bridged subsystems (finish + delete C shims)
- **Files:** finish Rust for graphics leaf C (`libs/graphics/*`, `sdl/*` — 18K), sound (`libs/sound/*` — 9.6K), video (`libs/video/*` — 2.2K), input (`libs/input/*` — 2.3K), resource (`libs/resource/*` — 1.7K), threads (`libs/threads/*`), then delete each C file and its `*_rust.c`/`rust_*.c` shim.
- **Ordering within:** graphics is the biggest FFI surface (58 callers) — do it AFTER Clusters 8–16 have converted most callers to Rust, so the residual C-caller shim is small.
- **Test:** full-screen render parity, audio playback, video (intro/comm FMV), keybindings.

---

### Cluster 18 — Leaf libs LAST (minimal-shim window)
- **Files:** `libs/math` (287), `libs/strings` (1,703), `libs/log` (432), `libs/list` (132), `libs/task` (139), `libs/time`, residual `libs/uio/*` (charhashtable, paths, uioutils, zip, debug — the Rust uio_bridge's remaining C helpers)
- **Why last:** these have 67/55/116 C callers *today*. By this point nearly all callers are Rust, so the caller shim is tiny or zero.
- **Test:** RNG determinism (seed→sequence hash), UTF-8 string handling, log output, task scheduling.
- **FFI shims:** near-0 (most callers already Rust).

---

### Cluster 19 — Core / Init / Main (the keystone)
- **Files:** `src/uqm.c` (1,726), `uqm/{init,setup,globdata,starcon,gameev,cleanup,intro,credits,fmv,misc,util}.c`, and all remaining `rust_bridge_*.c` shims (rust_bridge_mainloop, main2, macros, restart, clock_rust, rust_comm) (~6,000)
- **Depends on:** everything else ported
- **Test:** full game boot→main menu→new game→save→load→melee→exit; the smoke test profile.
- **FFI shims:** DELETE ALL. `RUST_OWNS_MAIN` already true; this removes the last C.
- **End state:** `find sc2/src -name '*.c'` returns nothing. Remove all `USE_RUST_*` flags from build.vars; the C toolchain step is deleted.

---

## 5. Summary Ledger

| Phase | Clusters | ~C Lines Removed | FFI Shims Added | Net Shim Trend |
|---|---|---|---|---|
| Dead code | D0 | 18,970 | 0 | — |
| Self-contained caller-first | 1–4 | ~29,000 | ~17 | low |
| Feature areas (top-down) | 5–16 | ~85,000 | ~40 (mostly transient) | shims retire as we go |
| Bridged leaf subsystems | 17 | ~34,000 | negative (delete shims) | ↓ |
| Leaf libs (deferred) | 18 | ~2,800 | ~0 | ↓ |
| Core/init | 19 | ~6,000 | delete all remaining | **0** |
| **TOTAL** | | **~190K → 0** | | |

**Key wins from static analysis:**
- **Network (7.2K) and Mikmod (16.8K) delete with ~0 shims** via caller-first ordering — 24K lines, near-free.
- **Decomp is live** (corrected) — bundled with save/load, needs 0 standalone shim.
- **Ships/comm C files still compile as data/dispatch** despite "bridged" — the Rust logic exists; these clusters are data-migration + dispatch-move, low risk.
- **Math/log/strings deferred to the end** to avoid building a 67/116/55-caller FFI surface prematurely.
- Every phase ends buildable + smoke-testable; no phase leaves an un-runnable binary.