# UQM target architecture: from C-shaped Rust to idiomatic Rust

Scope: the `uqm` crate under `rust/` at `main` 0ba2845ad, and the Cargo workspace it
becomes (section 2.1). Companion documents: [quality-gates.md](quality-gates.md)
(checks), [plan.md](plan.md) (phases), [work-items.md](work-items.md) (one issue per
PR) and [AGENTS.md](AGENTS.md) (rules for implementing agents). The dependency counts
cited here came from one-off line-regex scans of production code at 0ba2845ad (test
code skipped); the scan scripts are not kept. The Phase 0 `quality layers` check
(P0-04) replaces them with syn-based counts.

## 1. Starting point

Measured on production code only: `#[cfg(test)]` items, `*_tests.rs`, `tests.rs`,
blank and `//` lines are excluded. The counters are line-regex approximations; the
Phase 0 xtask baseline replaces them with syn-based numbers.

| Measure | Value |
|---|---|
| Production lines (non-blank, non-comment) | 217.7k in `rust/src` |
| Lines inside `unsafe` blocks or `unsafe fn` bodies | 23% |
| `static mut` definitions (incl. 24 from the `global!` macro, `game_init/kernel_abi.rs:17-46`) | 375 |
| `extern "C" fn` definitions | 1,922 |
| `#[no_mangle]` / `#[export_name]` | 1,257 |
| Foreign declarations in `extern "C" {}` blocks | 641 fns, 340 statics, in 123 files |
| `#[repr(C)]` | 390 |
| Functions with raw-pointer parameters | 1,907 |
| `#[allow(..)]` attributes (176 are `non_snake_case`) | 269 |
| `.unwrap()` / `.expect(` | 716 |
| Files over 1,000 / 700 production lines | 29 / 58 |
| Functions over 80 / 150 lines | 263 / 67 |

Per-subsystem figures are in plan.md section 1. Facts that shape the design:

1. **Rust modules call each other through the linker.** Almost every foreign
   declaration resolves to a `#[no_mangle]` Rust definition in this same crate. The
   only external native symbols are SDL2 (via `sdl2`/`sdl2-sys`), libc sockets
   (`network.rs`), and macOS frameworks (`graphics/ffi_video_macos.rs:25,34`,
   `automation/native_window.rs:1776,1786`, `automation/native_isolation.rs:67`).
2. **Six ABI type universes.** `include!` of generated bindings at `c_bindings.rs:15,31`,
   `navigation/mod.rs:14`, `supermelee/netplay/mod.rs:13`, `graphics/native_abi.rs:11`
   and `ships/native.rs:12` creates separate Rust types for one C record:
   `ELEMENT` is defined at `abi/ship_abi.rs:1280`, `abi/navigation_abi.rs:3479` and
   `abi/netplay_abi.rs:248`, and `RACE_DESC` is an opaque `[u128; 24]` at
   `abi/global_state_abi.rs:188`. They are reconciled by 164 layout assertions, 74
   `link_name` aliases and 7 `clashing_extern_declarations` allows.
3. **One symbol, several types.** `DisplayArray` is defined as `[PRIMITIVE; 330]`
   (`battle/process_native.rs:259`) and declared as `[CPrimitive; 150]`
   (`planet_side/orbit_scan.rs:79`). `CurStarDescPtr` (defined `planets/starmap.rs:12`)
   is declared with other pointee types at `state/c_api.rs:97`,
   `savegame/live/load.rs:16`, `comm/dispatch.rs:532`, `battle/encount_native.rs:92`,
   `planets/solarsys/lifecycle.rs:127`, `planets/pstarmap.rs:138`. `GlobData` is
   declared both as `GLOBDATA` and as `comm::locdata::CGlobData`
   (`savegame/runtime/live.rs:30`, `comm/commglue_abi.rs:73`).
4. **Duplicate authority for story state.** `GAME_STATE.GameState: [u8; 155]`
   (`abi/ship_abi.rs:3500`) is passed to `getGameState`, which ignores it
   (`navigation/globals.rs:19-21`); the real bits live in `GLOBAL_GAME_STATE`
   (`state/ffi.rs:15`). Callers also address bits by raw index
   (`planets/pstarmap.rs:202-213` uses 42..43 and 41) or by string
   (`state/game_state_keys.rs:477`, returns 0 for an unknown name).
5. **Dormant parallel models.** `ships/traits.rs` (`ShipBehavior`) and the
   non-`_native` halves of `ships/races/*.rs` (3.6k production lines) are reached only
   through `ships/registry.rs:1012` from tests. `battle/process_loop.rs` (899 lines),
   `battle/display_list.rs` (316), `battle/tactical.rs` (165) and
   `battle/integration.rs` (130) have no production caller. This breaks
   `dev-docs/project-standards.md` section 3 (one implementation per subsystem).
6. **Untestable modules.** 16 modules are `#![cfg(not(test))]` (all of `ui/`,
   `navigation/hyper_*`, `front_end/cnctdlg.rs`) because they need link-time symbols.
7. **One game thread.** Game logic runs on one thread. `CreateThread_Core`
   (`threading/legacy_abi.rs:50`) has no caller. Other threads are the audio thread
   (`sound/rodio_backend.rs:731`) and automation I/O. An owned `&mut Game` therefore
   needs no locks.
8. **Direct record access.** Unconverted race code dereferences element and ship
   records, follows `pred`/`succ` links and reads queue statics itself
   (`ships/races/orz_native.rs:107-111`, `chmmr_native.rs:406-460`); 27 race files,
   13 battle files, 5 navigation files, `planet_side/collision_adapter.rs`,
   `front_end/uqmdebug.rs` and 3 savegame files name `ELEMENT`/`HELEMENT`.
9. **Parts already idiomatic:** `planet_side/` (session, generational
   `SurfaceWorld`), `savegame/` (`decode.rs` over `Read`, owned `SaveFile`), dialogue
   scripts (`Dialogue<A: OrzApi>`), generator logic (`planets/generate/orz.rs` over
   `Story`/site traits), `mainloop/state_machine.rs`, the sound command enum
   (`sound/rodio_audio.rs:20`), `threading/pacing.rs` (`PacingEnv`).
10. **One package with a cyclic module graph.** Everything builds as one `uqm`
    package (404k lines in `rust/src` with tests). The only other workspace member,
    `xtask`, depends on it (`xtask/Cargo.toml`, `uqm-rust`), so every xtask command
    compiles the game, and `incremental = false` (`rust/.cargo/config.toml:2`) means
    any edit recompiles the whole package. Through `crate::` paths alone, 25 of the 37
    top-level modules form one strongly connected component. The `abi/*.rs` hubs
    widen it: `ships::native::abi` re-exports battle, Super Melee and ui functions
    (`abi/ship_abi.rs:1184-1195`, `:1618-1630`) and is imported by battle, mainloop,
    game_init and graphics. The modules also link through foreign statics: 364 of the
    371 cross-module foreign declarations the scan resolves are statics, defined in
    `game_init` (112, the `global!` kernel set), `mainloop` (76, the option globals of
    `mainloop/options_abi.rs:12-22`), `graphics` (57), `navigation` (52, `GlobData`)
    and `planets` (21, `CurStarDescPtr`). The exception is
    the layer that was `libs/` in C (graphics, sound, resource, video, strings,
    threading, input, io, network, collections, math, logging): it reaches the game
    through 13 `crate::` paths and 7 foreign static declarations only (section 2.2).

## 2. Layers

```
app/          composition root: main, CLI, config, builds Platform + Game, runs the
              loop; automation (scenarios, scheduler, Observer impl, native harness)
modes/        melee_setup, battle, hyperspace, solar_system, orbit, planet_side,
              comm, starbase, menus (front_end, ui, restart)
content/      races (descriptors + behaviour), system generators, dialogue scripts,
              data tables
engine/       fixed-point math, RNG, ordered pools, element/ship records and the
              battle world (Battle, Cx), process loop, collision, typed World and
              story state, clock, Observer and host-port traits, resource cache,
              canvas, DCQ, snapshot/apply between Game and SaveFile
persistence/  byte formats only: save file, state files, melee team files
platform/     SDL2 window/texture/events/audio device, sockets, OS clock, file
              system (Vfs), threads and QoS, macOS FFI
```

Dependency direction: `app` → `modes` → `content` → `engine` → `persistence`;
`platform` → `engine`, and only `app` names `platform`. A lower layer never names a
higher layer's type. Where content must call back into a mode (a generator starting a
conversation), `content` declares a narrow host trait and `modes` implements it
(section 5.3). Modes reach the platform only through the `engine::host::Platform`
trait object in `Io` (section 3). `persistence` holds no game types: it encodes and
decodes `SaveFile`, and `engine::save` maps `Game` to and from it (section 7).
`engine`, `content`, `modes` and `persistence` do not read wall-clock time, spawn
threads, read environment variables or touch the filesystem.

| Layer | Current modules |
|---|---|
| platform | `graphics/sdl`, `graphics/ffi_video*`, `input/sdl_input.rs`, `sound/rodio_*`, `network*`, `io` (VFS backends), `threading/qos.rs`, `time/counter_abi.rs` (OS clock), `platform.rs`, the process, descriptor and fd-relative file calls of `automation/child_session.rs`, `native_runner.rs` and `interrupt.rs`, the CoreGraphics and sandbox parts of `automation/native_window.rs` and `automation/native_isolation.rs` |
| engine | `math`, `collections`, `battle/{process_native,velocity,gravity,world_elements,weapon_native,ship_native,sounds}`, the combat AI `battle/{cyborg_native,ai,ai_entry}` (the race files call `ship_intelligence`, `cyborg_native.rs:512`, as `vux_native.rs:190` does), `state` (except state-file bytes), `time/clock_*` (game clock), `graphics/{drawable,frame,context,dcqueue,tfb_draw,native_canvas,software_*}`, `resource`, `strings`, `sound` (command queue, decoders), `savegame/{live,runtime}` |
| content | `ships/races/*_native.rs`, ship descriptors and loader, `planets/generate`, `planets/tables.rs`, `comm/races` |
| modes | `battle/{battle_native,tactrans_native,encount_native}`, `supermelee`, `pickship`, `navigation`, `planets/{solarsys,orbit,scan,pstarmap}`, `planet_side`, `comm`, `starbase`, `ui`, `front_end`, `mainloop/restart_menu` |
| persistence | `savegame/{format,types,encode,decode,error,lzh}`, the byte layout in `state/state_file.rs` and `state/planet_info.rs`, `supermelee/setup/persistence.rs` |
| app | `main.rs`, `cli.rs`, `config.rs`, `mainloop` (game loop), `automation`, `bin/`; `c_bindings.rs` and `game_init` until Phase 4 |

A phase creates a new module only where it splits a mixed one (for example
`battle/process_native.rs` becomes `engine::battle` + `modes::battle`). Code whose
crate does not exist yet, or cannot move yet, is staged in a root module named after
its layer (`rust/src/engine/` from Phase 1); `quality layers` applies the target
crate's rules to the staging module from the day it is created.

### 2.1 Crates

The layers become Cargo packages so the compiler enforces the direction. Package
names are `uqm-<name>` under `rust/crates/<name>/`; the root package stays at
`rust/` as `uqm` (library `uqm_rust`) and becomes the app.

| Crate | Holds | Workspace deps | `unsafe` |
|---|---|---|---|
| `uqm-save` | `SaveFile`, encode/decode, LZH, state-file and `.mle` byte layouts, `SaveFormatError` | none | `#![forbid(unsafe_code)]` |
| `uqm-engine` | the engine layer; until Phase 7 also the SDL/OS backends it was seeded with | `uqm-save` | baseline debt, `forbid` from Phase 7 exit |
| `uqm-content` | `Race` impls and descriptors, `SystemGenerator` impls, data tables, dialogue scripts | `uqm-engine` | `forbid` |
| `uqm-modes` | every mode session and its UI | `uqm-engine`, `uqm-content`, `uqm-save` | `forbid` |
| `uqm-platform` | `SdlPlatform`, rodio audio, sockets, Vfs, OS clock, QoS, macOS FFI | `uqm-engine` | `#![deny(unsafe_code)]`, `#[allow(unsafe_code)]` on the modules in `UNSAFE_ALLOWED` |
| `uqm` (root) | app and automation; bins `uqm`, `uqm-gameplay-proof`, `uqm-native-acceptance` | all | `forbid` from Phase 8 |
| `uqm-xtask` | gates and the native harness CLI | `uqm` only with feature `native` | n/a |

Rules:
- Only `uqm-platform` depends on `sdl2`, `sdl2-sys`, `gl`, `libc`, `windows-sys`,
  `rodio` or `cpal`. Until Phase 7 `uqm-engine` and the root carry these as baseline
  entries of `quality layers` (quality-gates.md section 3.5).
- `engine::host` declares the ports: `trait Platform` (present a frame, poll input,
  queue audio commands, read the clock, `files()`, `net()`), `trait Files` (what
  callers do today on VFS paths: open for read, open for write, remove, list a
  directory, stat) and `trait Net` (the six netplay socket operations). The
  replay clock and `ReplayAudio` are engine types that `uqm-platform` composes when
  `--replay` is set. Each port has two implementations from the start (real and
  replay; real and in-memory loopback for `Net`), which is what justifies the trait.
- A mode that must call a C-style graphics or sound function before Phase 7 calls a
  safe wrapper added to `uqm-engine`; the wrapper's `unsafe` is engine debt, which is
  what lets `uqm-modes` and `uqm-content` start with `forbid(unsafe_code)`.
- A move keeps module paths: `git mv rust/src/graphics rust/crates/engine/src/graphics`,
  the new crate root declares `pub mod graphics`, and the root re-exports it
  (`pub use uqm_engine::graphics;`), so `crate::graphics::…` in root code and
  `uqm_rust::graphics::…` in `rust/tests` keep resolving. The `time` module splits:
  the root keeps `time/mod.rs`, `clock_abi.rs` and `clock_bridge.rs` (the game
  clock) and re-exports `counter_abi` from the engine. Phase 8 replaces the
  re-exports with crate paths.
- Features are declared on the root and forwarded: `bundled-sdl`, `debug-process`
  and `audio_heart` (`bundled-sdl = ["uqm-engine/bundled-sdl"]`, later
  `uqm-platform/bundled-sdl`). `rust/build.rs` (the pkg-config probe) moves to the
  crate that depends on `sdl2`.
- Release builds keep `codegen-units = 1` and `lto = "off"`, so a non-generic function
  is not inlined across crates. Small engine functions on the element loop (pool
  index, `ElementTable` accessors, fixed-point helpers) are `#[inline]`. Integer
  results do not depend on inlining, so traces are unaffected.

### 2.2 When each crate is split

A crate can be cut only when no module in it names a module outside it and above it.
Cargo rejects cyclic packages, and a foreign declaration against the dependency
direction is unsafe to rely on: a crate's own test binary does not link the crates
above it, and GNU ld, the default linker on the Linux targets, searches archives in
command-line order, which rustc derives from Cargo dependencies. `static mut` shared
across crates compiles, but every access needs `unsafe`, so it blocks the crate's
`forbid(unsafe_code)` instead of the split.

| Crate | Split in | First contents | Blockers cleared first |
|---|---|---|---|
| `uqm-save` | Phase 0 (P0-05) | `savegame/{format,types,encode,decode,error}.rs` | the codec names `state::planet_info::CannedNode` and `state::game_state::NUM_GAME_STATE_BYTES` (`savegame/types.rs:8,313`, `encode.rs:182`, `decode.rs:452`); both move into the crate. `.mle` joins in Phase 3, state-file layouts in Phase 4, LZH in Phase 7 (`savegame/lzh/mod.rs:14` reads through `resource::ffi_bridge`) |
| `uqm-engine` | Phase 1 (P1-06) | the former `libs/` layer: graphics, sound, resource, video, strings, threading, input, io, network, callback, collections, math, memory, logging, log_abi, bridge_log, `platform.rs`, `time/counter_abi.rs` (about 50k production lines by the section 1 counts) | 13 upward paths: `graphics/ffi.rs:137,254,565,566,574,596`, `graphics/ffi_video.rs:25` and `graphics/sdl/lifecycle_abi.rs:45,47` call automation; `graphics/geometry_ffi.rs:438` re-exports a ships ABI type; `input/sdl_input.rs:14` reads `mainloop::dir_prep`; `resource/ffi_bridge.rs:60` installs the ship code resource type; `threading/critical_timer.rs:53` takes an automation guard. 7 foreign statics defined in the game: `repository` (`graphics/gfxload.rs:12`, defined `game_init/kernel_abi.rs:44`), `input_templates` (`input/sdl_input.rs:57`) and `contentDir` (`resource/ffi_bridge.rs:67`, `sound/heart_ffi.rs:77`, `strings/getstr.rs:27`, `video/control.rs:21`, and `abi/graphics_abi.rs:998`, which `graphics/native_abi.rs:11` includes), all defined in `mainloop/options_abi.rs:12-22`. The eighth, `GlobData` at `time/clock_abi.rs:98`, is in the game clock, which stays in the root |
| `uqm-engine`, core | Phase 4 exit (P4-07) | `src/engine/` staging (records, `Battle`, `Cx`, pools), `World`, `Game`, `Kernel`, `StoryState`, `GameClock`, `Observer`, `engine::save` | the battle core reads `GlobData` (`battle/process_native.rs:723,896,1164`, `battle/ship_native.rs:189-434`) and calls main-loop wrap helpers (`battle/gravity.rs:31-45`); the game clock calls comm, navigation and ui (`time/clock_abi.rs:87-111`) |
| `uqm-content`, `uqm-modes` | Phase 6 exit (P6-05) | content and modes rows of the layer table | the generator vtable and `*_exports.rs` shims (Phase 5); dialogue scripts that name mode types (`LiveScript` calling comm and ui, `comm/races/script.rs:65-89`; `CLocData` descriptors and dispatch constants, `comm/races/orz.rs:3-4,793-795`), split into content traits and modes adapters by P6-02; `COMM_STATE` (`comm/state.rs:23`, P6-01); mode code calling automation directly (`crate::automation` paths: comm 21, planets 10, front_end 9, ui 7; replaced by the `Observer` in Phase 4); `app::bridge` calls (budget 0 at Phase 6 exit) |
| `uqm-platform` | Phase 7 exit (P7-10) | the platform row, carved out of `uqm-engine` | engine code reaching SDL state directly (`graphics/ffi.rs` `get_gfx_state`, also used by `automation/native_window.rs:1952`); `SDL_Surface` canvases, including video's (`video/ffi.rs:18,42-45`); the `uio_*` API and its callers; POSIX process and descriptor code in automation and `platform.rs` (P7-09) |

Not adopted:
- **A crate per mode** (`uqm-battle`, `uqm-comm`, …). Modes call each other
  synchronously (`Site::talk` calls `comm::race_communication`,
  `home_exports.rs:96-98`; the main loop enters every mode), so there is no direction
  between them for the compiler to enforce.
- **`uqm-automation`.** Automation sits only under the app binary; a separate crate
  enforces nothing new, and xtask would still compile modes through it.
- **Separate graphics or sound crates.** They would enforce no rule beyond the
  engine/platform split.
- **A port-trait crate** so that `uqm-platform` does not rebuild on engine edits: a
  compile-time measure only, added if the measured rebuild cost warrants it.
- **Moving `rust/src` to `rust/crates/uqm`.** Path churn in every document and open
  PR, with no enforcement gained.

Compile time: today any edit rebuilds the whole package and relinks all 61
integration-test binaries. After each split, an edit above a crate reuses that
crate's rlib; `cargo test -p <crate>` builds only that crate and its dependencies;
and the release profile's single codegen unit per crate lets independent crates
compile in parallel. Each split PR records `cargo build --timings` before and after
(plan.md section 3.1).
## 3. Ownership: an owned `Game` passed explicitly

```rust
/// Everything the original kept in globals, owned by the game thread.
pub struct Game {
    pub world: World,      // was GlobData (GLOBDATA, abi/ship_abi.rs:3504)
    pub kernel: Kernel,    // was the setup.c globals (game_init/kernel_abi.rs:23-46)
    pub content: Content,  // master ship list, star catalog, string tables
    pub options: Options,  // was opt* (abi/ship_abi.rs:1093-1117)
    pub rng: RandomContext,// was the TFB_Random context (one stream, same call order)
}

pub struct World {           // engine type; content may take &mut World
    pub sis: SisState,
    pub activity: Activity,  // CurrentActivity, NextActivity, LastActivity
    pub clock: GameClock,
    pub story: StoryState,   // typed bits; replaces GameState[155] and GLOBAL_GAME_STATE
    pub current_star: Option<StarId>, // was CurStarDescPtr; see below
    // Every C queue keeps its InitQueue capacity (navigation/globals.rs:101,151-170):
    // a full queue is a game rule (for example the shipyard limit), not an error.
    pub avail_races: OrderedPool<FleetInfo, NUM_FLEET_ENTRIES>,
    pub built_ships: OrderedPool<ShipFragment, MAX_BUILT_SHIPS>,
    pub npc_built_ships: OrderedPool<ShipFragment, MAX_SHIPS_PER_SIDE>,
    pub ip_groups: OrderedPool<IpGroup, MAX_BATTLE_GROUPS>,
    pub encounters: OrderedPool<Encounter, MAX_ENCOUNTERS>,
    pub autopilot: Point, pub ip_location: Point, pub ship_stamp: Stamp, // ...rest of GAME_STATE
}
```

Mode state lives in a session value created on entry and dropped on exit, as
`PlanetSideSession` already does:

| Global today | Owner in target |
|---|---|
| `disp_q`, `DisplayArray`, `DisplayLinks`, `zoom_out`, `nth_frame`, `OPT_MAX_ZOOM_OUT` (`process_native.rs:259-266`), `race_q` (`kernel_abi.rs:36`), `battle_counter` (`battle_native.rs:79`) | `engine::battle::Battle`, used by every element-loop mode (melee, campaign battle, hyperspace, IP flight); section 4.1 gives the stages |
| `CurStarDescPtr` | `World::current_star: Option<StarId>`. It outlives the solar system: hyperspace star contact sets it and suppresses repeat contact, star death clears it (`navigation/hyper_transition.rs:191-229`), load restores it before entry (`savegame/live/load.rs:28-35`), and it is the save's Star chunk (`savegame/runtime/live.rs:89-91`) |
| `pSolarSysState` | `SolarSystem` session, holding `sys: SolarSys` (planets, moons, orbit) with a required `star: StarId` |
| `COMM_STATE` (`comm/state.rs:23`, `LazyLock<RwLock>`), `COMM_DATA` (`comm/locdata.rs:15`) | `Comm` session |
| `ScreenContext`, `SpaceContext`, `Screen`, fonts, frames (`kernel_abi.rs:23-46`) | `Kernel`, loaded once by `LoadKernel` |
| `AUDIO_SENDER` (`sound/rodio_audio.rs:54`), SDL window, input state | `uqm_platform::SdlPlatform`, behind `engine::host::Platform` |
| `Coordinator` statics (automation) | `automation` observer, owned by `app` |

Entry points take what they use: `fn battle(game: &mut Game, io: &mut Io<'_>, kind:
BattleKind) -> Result<BattleOutcome, GameError>`, where `Io<'a> { platform: &'a mut dyn
Platform, observer: &'a mut dyn Observer }`. `Platform` is the engine port trait of
section 2.1, so `uqm-modes` compiles without `uqm-platform`. `Game`, `World`, `Kernel`
and `Options` are engine types; `Game::content` holds the loaded tables (ship catalog,
star catalog, string tables) as engine data that `uqm-content` fills.

**Ordering.** The call graph and statement order stay the same; only the way state
is reached changes. RNG draws, element traversal and sound queueing happen in the
order they happen today, so V1 traces stay identical (plan.md section 2).

**Rejected alternatives.**
- `thread_local! { RefCell<Game> }`: collision callbacks re-enter (collision → death
  → spawn), so `borrow_mut` would fail at run time; it is a global with a lock.
- Keeping statics behind an `UnsafeCell` wrapper with a "single thread" invariant:
  the compiler still cannot check aliasing, which is the problem being solved.

## 4. Entity storage: ordered pools that keep C order

`collections/queue.rs` implements `QUEUE_TABLE` lists as raw links in a
`HMalloc`ed table. The target is one generic container for every queue: battle
elements (`disp_q`), starships (`race_q[2]`), the `GAME_STATE` queues and the clock's
`event_q` (`time/clock_abi.rs:170`):

```rust
pub struct OrderedPool<T, const N: usize> {
    slots: Box<[Slot<T>; N]>, // fixed capacity, never reallocated
    head: Option<SlotIx>, tail: Option<SlotIx>,
    free: Option<SlotIx>,     // LIFO free list, exactly as FreeLink
}
struct Slot<T> { pred: Option<SlotIx>, succ: Option<SlotIx>, gen: u16, value: Option<T> }
#[derive(Clone, Copy, PartialEq, Eq)] pub struct Id<T> { ix: SlotIx, gen: u16, .. }
```

How it reproduces C order exactly:
1. **Allocation.** `InitQueue` frees slots N..1 (`collections/queue.rs:110-114`), so
   allocation returns slots 1, 2, 3... and frees push to the front. The pool does the
   same, so slot reuse and exhaustion (`MAX_DISPLAY_ELEMENTS = 150`,
   `abi/ship_abi.rs:142`) behave as before. `alloc` returns `Option<Id<T>>`: a full
   pool means a weapon does not fire, which is game behaviour, not an error.
2. **Links.** `put` appends at the tail (`PutQueue`), `insert_before` matches
   `InsertQueue`, `remove` unlinks without freeing (`process_native.rs:1034-1035`).
   Links hold slot indices without generations, so traversal follows C's links.
3. **Traversal.** There is no snapshot iterator. Loops use `head()` and `succ(id)` and
   read the successor at the same point as today: after `preprocess` and before
   `process_collisions` (`process_native.rs:746-752`, `:494`). Elements appended by
   callbacks during a pass are visited in that pass, as in C.
4. **Collision order.** `process_collisions` keeps its recursion, its `next_test` and
   `head()` restarts and its pairwise callback order (player ship first,
   `process_native.rs:642-668`). No element borrow lives across a hook call
   (section 5.3).
5. **Long-lived references** (`hTarget`, weapon handles, `Encounter::element`) are
   generational `Id`s. A stale id is a bug and panics. During parity runs the pool
   counts stale lookups; if the corpus shows the original relied on a reused slot,
   that case is modelled explicitly.
6. **Display primitives** (`DisplayArray`, 330 entries, with `DisplayLinks`) use the
   same pool shape, since primitive link order is drawing order.

The element record becomes a plain struct. The five bindgen unions
(`abi/ship_abi.rs:1338-1366`) hold same-typed members, so each becomes one field
with named accessors (`hit_points()` and `crew_level()` read the same `u16`).
`pParent: *mut c_void` becomes `parent: Parent` with
`enum Parent { None, Ship(ShipId), IpGroup(Id<IpGroup>) }`; the 57 `pParent` writes
are to starships and IP groups (`navigation/interplanetary.rs:428`), and the test
`test.pParent == element.pParent` (`process_native.rs:502`) becomes enum equality.

**Frame sets.** `image.farray` points at a frame array: a race's ship, weapon or
special array inside one ship's descriptor copy, or a shared array such as
`stars_in_space` (`battle/weapon_native.rs:79-80`) or `blast` (`:188`). The process
loop indexes it by resolution (`process_native.rs:1112-1123`), and Arilou compares a
threat's array with its owner's descriptor (`arilou_native.rs:216-224`). The target:

```rust
pub struct FrameSetRef { pub owner: FrameOwner, frames: Handle<FrameArray> } // one Frame per resolution
pub enum FrameOwner { Ship(ShipId, Role), Shared(SharedFrames) }              // Role::{Ship, Weapon, Special}
pub struct Image { pub set: FrameSetRef, pub frame: FrameRef }
```

`farray == &desc.ship_data.special` becomes `set.owner == FrameOwner::Ship(ship,
Role::Special)`; two ships of one species differ, as their descriptor copies do in C.
`FrameOwner` is a plain value, not a reference to the ship. The `Handle` keeps the
frames loaded, so a parentless blast that copied a weapon's set
(`weapon_native.rs:196`) draws correctly after its ship is torn down. Resolution
indexing becomes `set.view(index)`. The PLAN fixture `frame-sets` checks both
comparisons and the indexing.

### 4.1 Transition sequence

Each static authority that unconverted code dereferences directly (the element and
ship records with their queues; `GlobData` in Phase 4) changes in three stages. No
stage keeps a second copy of any state.

**Stage A: one authority, current representation.** `engine::battle::record` holds
the only definition of `ELEMENT`, `STARSHIP`, `RACE_DESC`, `QUEUE` and `PRIMITIVE`
(from `abi/ship_abi.rs`), and `engine::battle::legacy` the only definition of
`disp_q`, `race_q`, `DisplayArray`, `DisplayLinks` and `DisplayFreeList`. Both live in
the root staging module `rust/src/engine/` until Phase 4 exit, when they move into the
`uqm-engine` crate with the rest of the battle core (section 2.2). The copies in
`navigation_abi.rs`, `netplay_abi.rs` and `global_state_abi.rs` become re-exports.
Layout, raw links, `pParent`, `hTarget`, the unions and the hook fields are
unchanged, so unconverted races keep their code. The Stage A PR (P1-01) moves
definitions only.

The hook fields change in the first Stage B PR (P1-02), together with `ElementId`,
`Cx` and the dispatcher they refer to, so no PR names a type that does not exist yet:

```rust
pub enum ElementHook { Legacy(unsafe extern "C" fn(*mut ELEMENT)), Native(fn(&mut Cx, ElementId)) }
pub enum CollisionHook { Legacy(LegacyCollisionFn), Native(CollisionFn) }
```

The `Legacy` variants are together called `Hook::Legacy`. Legacy writes become
`Some(CollisionHook::Legacy(f))`, a mechanical edit; Androsynth's save and restore
(`androsynth_native.rs:389-397`) still copies the whole value.
Legacy code that calls a hook field calls `invoke_*` instead, the one dispatcher.
The same PR adds `LinkQueue<T>`, a typed view of the same kind over any other `QUEUE`
(`race_q`, the GAME_STATE queues, `event_q`); later units move queue consumers to it.

**Stage B: convert consumers on that representation.** Converted code reaches records
only through `ElementTable`, a view over the authority, and names them by
`ElementId { slot, gen }`. Generations sit in a side array, so the layout is
unchanged; `HELEMENT` and `ElementId` convert by slot arithmetic. Accessors decode C
fields (`parent()` maps `pParent` to `Parent` by address range). Rules:

1. *Pointer lifetime.* Storage keeps today's lifetime: `InitQueue` at kernel init,
   `UninitQueue` at cleanup (`game_init/cleanup.rs:33`), never moved. A raw element
   pointer is valid until its slot is freed. `ReinitQueue` in `InitDisplayList`
   (`process_native.rs:926-934`) frees every slot and bumps every generation.
2. *Provenance.* Pointers handed to legacy code derive from the static's address,
   never from a Rust reference. `ElementTable` creates a reference to one record at a
   time, scoped to its own borrow; it never references the whole array.
3. *No reference across a call.* Converted code holds no record reference across a
   hook call or any call into legacy code (section 5.3).
4. *Nested entry.* `app::bridge` holds at most one published `NonNull<Cx>`. The
   dispatcher publishes the pointer derived from its own `&mut Cx` before calling a
   legacy hook and clears it on return. A Rust entry point called from legacy code
   (`AllocElement`, `do_damage`, `ProcessSound`) takes the pointer out of the slot
   for its body (`bridge::enter(|cx| ..)`), so one `&mut Cx` is live at a time; a
   nested legacy call publishes again through the dispatcher. Entering with an empty
   slot panics. Nesting is strictly LIFO.
5. *Write visibility.* There is one copy, so every write is visible to the next read.
   No synchronisation step exists, and none may be added.

Until Phase 4, `Cx` reaches world state through the single `GlobData` definition
under the same rules.

**Stage C: replace the representation.** When no legacy reader of the records
remains (Phase 2 exit: all races, the AI, the element callbacks in `navigation/`, the
ship pickers and the battle sound tracker; the `raw_records` counter is 0),
a PR swaps the storage for `OrderedPool` (in `uqm-engine`, `collections::pool`), links for slot indices, `pParent`
for `Parent`, `hTarget` for `Option<ElementId>`, unions for fields, `farray` for
`FrameSetRef`, and deletes the authority statics and the record types
(`Hook::Legacy` is already gone with the last race, P2-15). The pools move into `Battle`, which each element-loop mode creates on entry,
if the migration trace shows no element survives a mode switch (`InitDisplayList`
reinitialises the queue at mode entry); otherwise they stay in `Kernel` and `Battle` borrows them. Order is
unchanged, so V1 traces stay identical.

`GlobData` follows the same stages: Phase 0 leaves one definition (Stage A); Phase 4
converts readers to `World` accessors over it (B), then replaces it with the owned
`World` (C). The GAME_STATE queues and `event_q` do too: every consumer moves to
`LinkQueue` views (P4-03, `queue_calls` = 0), then the storage becomes `OrderedPool`
and `collections/queue.rs` is deleted (P4-04).

## 5. Behaviour dispatch

### 5.1 Races: a trait object per species

The 28 initialisers in `ships/code_resources.rs:75-104` return pointers to
`static mut <RACE>_DESC` templates whose fields hold `Option<unsafe extern "C" fn>`.

```rust
pub trait Race: Sync {                                   // declared in engine, implemented in content
    fn descriptor(&self) -> RaceDesc;                    // plain data template
    fn preprocess(&self, _cx: &mut Cx, _ship: ShipId) {}
    fn postprocess(&self, _cx: &mut Cx, _ship: ShipId) {}
    fn init_weapon(&self, cx: &mut Cx, ship: ShipId) -> WeaponSpawn;
    fn intelligence(&self, cx: &mut Cx, ship: ShipId, concerns: &mut Concerns);
    fn uninit(&self, _ship: &mut Starship) {}
}
pub fn race(species: Species) -> &'static dyn Race { match species { Species::Arilou => &Arilou, /* ... */ } } // content
```

The engine stores the `&'static dyn Race` in each `Starship` and never names a species
type, so `uqm-engine` does not depend on `uqm-content`. `SystemGenerator` and
`SiteHost` (section 5.3) are declared in content, since only modes call them;
`SolarSys` is an engine type because `engine::save` reads it.

`Cx { b: &mut Battle, world: &mut World, rng: &mut RandomContext }` is the engine's
battle context. `Starship` owns its copy of `RaceDesc` (C copies the template per
ship), so VUX limpets edit that ship's own characteristics. Per-race runtime data
(`race_desc.data: *mut c_void`) becomes `enum RaceState { None, Pkunk(..), .. }`.

### 5.2 Element hooks: safe `fn` pointers

Weapons, fighters, debris and ship elements get hooks at creation, and some are
swapped and restored at run time (Androsynth, Pkunk `pkunk_native.rs:401-442`). About
150 distinct callbacks become

```rust
pub type ElementFn = fn(&mut Cx, ElementId);
pub type CollisionFn = fn(&mut Cx, ElementId, &mut Point, ElementId, &mut Point);
pub struct ElementHooks { pub pre: Option<ElementFn>, pub post: Option<ElementFn>,
                          pub collide: Option<CollisionFn>, pub death: Option<ElementFn> }
```

Rejected: one enum of all callbacks (every race's weapons in one match) and
`Box<dyn>` per missile (allocates; `next = current` stops being a copy).

### 5.3 Re-entry

**Collision → death → spawn.** `process_collisions` copies the hook it is about to
call, ends all element borrows, calls it, then rereads:

```rust
let (first, second) = if cx.b.elements[test].flags.contains(Flags::PLAYER_SHIP) { (test, el) } else { (el, test) };
let hook = cx.b.elements[first].hooks.collide;          // Copy; borrow ends here
invoke_collision(cx, hook, first, second);              // may do_damage → death hook → alloc
let hook = cx.b.elements[second].hooks.collide;         // reread: the first call may replace it
invoke_collision(cx, hook, second, first);
```

A death hook that spawns calls `cx.b.elements.alloc()`. A full pool returns `None`, as
`AllocElement` returns null. The new element is appended and visited later in the
same pass. After the recursive calls the loop rereads both elements' flags, as
`process_native.rs:640-642` does.

**Generator → communication → return.** `content` declares the host and `modes`
implements it:

```rust
pub trait SystemGenerator: Sync {
    fn generate_orbital(&self, s: &mut SolarSys, host: &mut dyn SiteHost, body: Body) -> bool {
        default::generate_orbital(s, host, body)
    }
    // the other 12 GenerateFunctions hooks (planets/generate/vtable.rs:29-43): same pattern
}
pub trait SiteHost: Story + HomeContactSite + OrzSite + MyconSite /* + the other site traits */ {}
```

`modes::solar_system::Site { game: &mut Game, io: &mut Io }` implements `SiteHost`,
as `RaceHome` does today (`home_exports.rs:34-140`). The call is
`generator(star).generate_orbital(&mut session.sys, &mut Site { game, io }, body)`;
`session.sys` and `game` are disjoint. `Site::talk` calls
`comm::race_communication(game, io)` synchronously (`home_exports.rs:96-98`). During
that call the `SolarSystem` session stays on `solar_system::explore`'s stack and comm
cannot reach it (comm reads no `pSolarSysState` today). `Story` becomes a supertrait
of the site traits, so the logic functions drop their separate `story` parameter
(`orz.rs:65-72`) and reread story through the host after `talk` returns. `LiveStory`
becomes `impl Story for StoryState`, and the `*_exports.rs` shims go.

## 6. Errors and `unsafe`

- Typed errors with `thiserror` (already a dependency): `ContentError` (with the
  resource name), `SaveFormatError` (exists, `savegame/error.rs`), `PlatformError`,
  `NetError`, combined in `GameError` at `app`.
- C status returns go: `LoadSC2Data` returning 0 on a missing frame
  (`navigation/globals.rs:59-77`) becomes `Result<Kernel, ContentError>`; `InitQueue`
  allocation failure disappears with fixed-capacity pools; `AllocElement` null becomes
  `Option<ElementId>`; string-keyed `get_game_state(name)` becomes typed `StoryKey`
  constants, so an unknown key is a compile error.
- Internal invariant violations (stale id, a `PLAYER_SHIP` element without a ship
  parent, bridge entry with no published context) panic with context: fail fast.
  Recoverable conditions use `Result`. No `unwrap`/`expect` in production paths.
- `unsafe` is allowed only in five modules of `uqm-platform`: `sdl` (texture upload,
  `sdl2-sys` calls), `macos` (today `graphics/ffi_video_macos.rs`, the CoreFoundation
  and CoreGraphics parts of `automation/native_window.rs`,
  `automation/native_isolation.rs`), `net` unless replaced by `std::net`, `qos`
  (today `threading/qos.rs`) and `os` (process, descriptor, fd-relative file and
  errno calls: today `automation/child_session.rs`, `native_runner.rs`,
  `interrupt.rs` and `platform.rs`, gathered behind `engine::platform::os` by P7-09). Every other crate carries `#![forbid(unsafe_code)]` from
  the point given in section 2.2. Software canvases become Rust pixel buffers
  (`Vec<u32>` RGBA8888, which the scalers already take); only the final present
  touches SDL. Every block carries `// SAFETY:`.

## 7. Interfaces with the rest of the system

**Content loading.** `io/uio_bridge.rs` becomes `uqm_platform::vfs::Vfs`, keeping
mount order and precedence (`UIO_MOUNT_TOP/BOTTOM/ABOVE/BELOW`, `uio_bridge.rs:20-27`)
and zip packages. It lives in the platform because it reads the filesystem, which the
engine may not. `resource/` becomes a typed `engine::ResourceCache` that reads through
the `Files` port and returns `Handle<Frame>`, `Handle<StringTable>`, `Handle<Sound>`;
dropping the owner releases the resource, replacing
`CaptureDrawable`/`ReleaseDrawable`/`DestroyDrawable` pairs. Content tests use an
in-memory `Files`.

**Savegame.** The byte format and `encode.rs`/`decode.rs`/`lzh/` stay, in `uqm-save`.
The live connection is `engine::save`; it names engine types only, so the solar view
is the `SolarSys` data that the `SolarSystem` session owns:

```rust
pub struct SessionView<'a> { pub battle: Option<&'a Battle>, pub solar: Option<&'a SolarSys> }
fn snapshot(game: &Game, view: SessionView<'_>) -> SaveFile;          // as capture() in live/save.rs:16-50
fn apply(save: SaveFile, game: &mut Game) -> Result<Activity, LoadError>;
```

`snapshot` keeps the current preparation steps (hyper or solar location, `PutGroupInfo`,
summary from live solar state, `in_orbit` cleared after capture). `apply` leaves the
current activity alone and returns the saved one for the main loop, as `LoadGame` does
(`savegame/runtime/global.rs:79-85,112`). The writer's element-amount quirk
(`live/save.rs:65-69`) stays at the writer. plan.md V3 has the tests.

**Automation.** Three hooks, threaded into each running mode through `Io`, keep the
scheduler's contract (`automation/scheduler.rs:93-113`):

```rust
pub trait Observer {                                      // engine::observe
    fn input_admitted(&mut self, v: &InputView) -> Injected; // once per admitted input callback
    fn checkpoint(&mut self, v: &CheckpointView);          // once per simulation step
    fn presented(&mut self, p: &Presented);                // once per committed present
}
```

In one element-loop frame (`DoBattle`, `battle/battle_native.rs:306-375`) the order is:
netplay CRC sample; `ProcessInput` calls `input_admitted`, which applies injected keys
and advances hold, release and settle counts; `RedrawQueue` runs the element loop and
queues drawing; sleep and DCQ flush call `presented` once per committed present, zero
or more times per frame (`threading/pacing.rs:122-146`); the capture acknowledgement
follows the present whose generation matches; `checkpoint` runs where
`rust_battle_frame_advance` runs today. Other modes place the same three hooks in
their `DoInput` loop. Campaign keys (`automation/campaign_state.rs`) read typed fields.
Seeds (`automation/trace.rs:49` `SeedDomain`) set `Game::rng` and system RNGs through
the same entry points as today. Scenario JSON in `rust/scripts/` does not change.

**Netplay.** The wire format stays byte-identical and `NETPLAY_PROTOCOL_VERSION`
(`supermelee/netplay/packet.rs:238`) does not change. `#[repr(C)]` packet structs and
`packetTypeData` (`packet.rs:29`) become `enum Packet` with explicit
`encode`/`decode`, tested against byte fixtures. Sockets are reached through the
`engine::host::Net` port; an in-memory loopback `Net` runs the two-peer test. The sync checksum stays byte-for-byte
(`supermelee/netplay/checksum.rs:38-77`): the RNG seed, then, in link order, each
element without `BACKGROUND_OBJECT` (bit 15), hashing flags, life span, crew, mass,
turn and thrust waits, velocity and both locations. It omits images, targets,
parents, hooks and ship counters, so it is one field of the migration trace (plan.md
V1), not the whole oracle.

## 8. Before and after

### 8.1 Ship preprocess: Arilou teleport

Before (`ships/races/arilou_native.rs:260-290`):

```rust
unsafe extern "C" fn arilou_preprocess(ship: *mut ELEMENT) {
    let ss = starship(ship);
    let special_frames = (&raw mut (*(*ss).RaceDescPtr).ship_data.special).cast::<FRAME>();
    if (*ship).state_flags & NONSOLID as u16 == 0 {
        if (*ship).__bindgen_anon_5.thrust_wait == 0 {
            zero_velocity_components(&raw mut (*ship).velocity);
            (*ss).cur_status_flags &= !(SHIP_AT_MAX_SPEED as u16);
        }
        if ((*ss).cur_status_flags & SPECIAL as u16 != 0) && (*ss).special_counter == 0
            && DeltaEnergy(ship, -(SPECIAL_ENERGY_COST as SIZE)) != 0
        { /* start teleport ... */ }
    } else if (*ship).next.image.farray == special_frames { /* ... TFB_Random() ... */ }
}
```

After:

```rust
impl Race for Arilou {
    fn descriptor(&self) -> RaceDesc { descriptor() } // same constants, plain data
    fn preprocess(&self, cx: &mut Cx, ship: ShipId) {
        let el = cx.b.ships[ship].element;
        if !cx.b.elements[el].flags.contains(Flags::NONSOLID) {
            if cx.b.elements[el].thrust_wait() == 0 {
                cx.b.elements[el].velocity = VelocityDesc::ZERO;
                cx.b.ships[ship].status.remove(Status::AT_MAX_SPEED);
            }
            if cx.b.ships[ship].status.contains(Status::SPECIAL)
                && cx.b.ships[ship].special_counter == 0
                && cx.b.delta_energy(ship, -SPECIAL_ENERGY_COST)
            { start_teleport(cx, ship, el) }
        } else if cx.b.elements[el].next.image.set.owner == FrameOwner::Ship(ship, Role::Special) {
            continue_teleport(cx, ship, el) // cx.rng.next() calls in the original order
        }
    }
}
```

### 8.2 Collision callback: VUX limpet

Before (`ships/races/vux_native.rs:77-120`): the same steps through
`(*(*starship(b)).RaceDescPtr).characteristics`, `wrapping_add` followed by an undo at
zero, `TFB_Random()` and `__bindgen_anon_*` union writes. After:

```rust
fn limpet_collision(cx: &mut Cx, limpet: ElementId, _: &mut Point, hit: ElementId, _: &mut Point) {
    if cx.b.elements[hit].flags.contains(Flags::PLAYER_SHIP) {
        let target = cx.b.ship_of(hit);
        let c = &mut cx.b.ships[target].race.characteristics;
        c.turn_wait = c.turn_wait.saturating_add(1);   // wrapping_add then undo at 0 == saturating
        c.thrust_wait = c.thrust_wait.saturating_add(1);
        slow_thrust(c);
        cx.b.ships[target].race.intel.maneuverability_index = 0;
        let owner = cx.b.ship_of(limpet);
        let sounds = cx.b.ships[owner].race.data.ship_sounds;
        cx.b.sounds.process(sounds.index(2), hit);      // same order: sound, then RNG draw
        let frame = cx.b.ships[owner].race.data.weapon[0].with_index(cx.rng.next() as u16);
        cx.b.modify_silhouette(hit, Stamp::from(frame), Modify::Image);
    }
    let e = &mut cx.b.elements[limpet];
    e.set_hit_points(0);
    e.life_span = 0;
    e.flags |= Flags::COLLISION | Flags::DISAPPEARING;
}
```

### 8.3 Main loop

Before: the loop calls through a 40-method `GameLoopOps` trait whose production impl
forwards to `extern "C"` symbols (`mainloop/game_loop.rs:290-345`) that reach
`GlobData` (`mainloop/ffi.rs:289-318`). After: `app::run(game, io)` keeps
`state_machine::evaluate`/`check_break` and calls `starbase::visit(game, io)`,
`solar_system::explore(game, io)` and the other modes directly; `World::new_game`
replaces `InitGameStructures` (`navigation/globals.rs:89-140`), putting
`FleetInfo::initial` in the order of `globals.rs:102-112`. `GameLoopOps` stays only
if a test needs it; the mock exists because the real calls were link-time symbols.

### 8.4 Planet generator: Orz

Before: `battle/gendef.rs:74` returns `&home_exports::GENERATE_ORZ_FUNCTIONS` as a
`*const GenerateFunctions` of `unsafe extern "C"` shims over raw `SOLARSYS_STATE`
pointers (`home_exports.rs:518-535, 650-656`), called as
`((*(*state).genFuncs).generatePlanets.unwrap())(state)`
(`planets/solarsys/lifecycle.rs:234`). After:

```rust
impl SystemGenerator for Orz {
    fn generate_orbital(&self, s: &mut SolarSys, host: &mut dyn SiteHost, body: Body) -> bool {
        let star = s.star_index;
        s.with_orbit(body, |orbit| orz::generate_orbital(orbit, body, star, host, load_default_surface))
    }
    // generate_planets / generate_energy / pickup_energy likewise; the rest use the defaults
}
```

## 9. Kept as they are, on purpose

- **Fixed-point arithmetic and C integer widths.** `VELOCITY_DESC` fractions, `COUNT`/
  `SIZE`/`COORD` widths, `wrapping_*` operations and narrowing `as` casts reproduce
  C results (for example `scale_sine`, `arilou_native.rs:47`). Type aliases may be
  renamed (`Count = u16`) but not widened.
- **The packed save format**, LZH compression and chunk order.
- **Procedural dialogue.** `comm/races/*` stays code (`Dialogue<A: Api>`); only the
  `Api` implementation changes from globals to `&mut Game` + `Comm`.
- **Data tables.** `planets/tables.rs`, `graphics/software_scalers/hq_rules.rs`,
  race descriptor constants, string-table indices and resource names.
- **The netplay wire format and checksum.**
- **Statement order in ported functions**, including where an idiomatic rewrite
  would reorder RNG draws or sound queueing.
- **C names in docs.** Functions drop CamelCase when they stop being exported, and keep
  `/// C: <name> (<file>.c)` for traceability.
