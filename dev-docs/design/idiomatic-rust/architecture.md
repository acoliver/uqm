# UQM target architecture: from C-shaped Rust to idiomatic Rust

Scope: the `uqm` crate under `rust/` at `main` 0ba2845ad. Companion documents:
[quality-gates.md](quality-gates.md) (checks) and [plan.md](plan.md) (phases).

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

## 2. Layers

```
app/          composition root: main, CLI, builds Platform + Game, runs the loop
automation/   implements engine::observe::Observer and the scripted input source
modes/        melee_setup, battle, hyperspace, solar_system, orbit, planet_side,
              comm, starbase, menus (front_end, ui, restart)
content/      races (descriptors + behaviour), system generators, dialogue scripts,
              data tables, resource index, VFS
engine/       fixed-point math, RNG, ordered pools, element/ship records and the
              battle world (Battle, Cx), process loop, collision, typed World and
              story state, clock, observer trait, resource handles, canvas, DCQ
persistence/  save format (savegame/), state files, melee team files, config
platform/     SDL2 window/texture/events/audio device, sockets, OS clock, macOS FFI
```

Dependency direction: `app` → `automation`, `modes` → `content` → `engine`;
`persistence` → `engine`. A lower layer never names a higher layer's type. Where
content must call back into a mode (a generator starting a conversation), `content`
declares a narrow host trait and `modes` implements it (section 5.3). `platform` is
reached only through the `Platform` value passed in by `app`. `engine`, `content` and
`modes` do not read wall-clock time, spawn threads, read environment variables or
touch the filesystem; `xtask quality layers` checks it (quality-gates.md section 3).

Existing top-level modules map onto layers in place; directories are not renamed in
bulk. A phase creates a new module only where it splits a mixed one (for example
`battle/process_native.rs` becomes `engine::battle` + `modes::battle`).

| Layer | Current modules |
|---|---|
| platform | `graphics/sdl`, `graphics/ffi_video*`, `input/sdl_input.rs`, `sound/rodio_*`, `network*`, `threading/qos.rs`, `time` (OS clock part), `platform.rs` |
| engine | `math`, `collections`, `battle/{process_native,velocity,gravity,world_elements,weapon_native,ship_native}`, `state`, `time` (game clock), `graphics/{drawable,frame,context,dcqueue,tfb_draw,native_canvas,software_*}`, `resource`, `strings` |
| content | `ships/races/*_native.rs`, `planets/generate`, `planets/tables.rs`, `comm/races`, `io` (VFS) |
| modes | `battle/{battle_native,tactrans_native,encount_native,cyborg_native}`, `supermelee`, `pickship`, `navigation`, `planets/{solarsys,orbit,scan,pstarmap}`, `planet_side`, `comm`, `starbase`, `ui`, `front_end`, `mainloop/restart_menu` |
| persistence | `savegame`, `state/state_file.rs`, `supermelee/setup/persistence.rs`, `config.rs` |

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
| `AUDIO_SENDER` (`sound/rodio_audio.rs:54`), SDL window, input state | `Platform` |
| `Coordinator` statics (automation) | `automation` observer, owned by `app` |

Entry points take what they use: `fn battle(game: &mut Game, io: &mut Io<'_>, kind:
BattleKind) -> Result<BattleOutcome, GameError>`, where `Io { platform: &mut Platform,
observer: &mut dyn Observer }`.

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
`disp_q`, `race_q`, `DisplayArray`, `DisplayLinks` and `DisplayFreeList`. The copies in
`navigation_abi.rs`, `netplay_abi.rs` and `global_state_abi.rs` become re-exports.
Layout, raw links, `pParent`, `hTarget` and the unions are unchanged, so
unconverted races keep their code. The one record change is the hook fields:

```rust
pub enum ElementHook { Legacy(unsafe extern "C" fn(*mut ELEMENT)), Native(fn(&mut Cx, ElementId)) }
pub enum CollisionHook { Legacy(LegacyCollisionFn), Native(CollisionFn) }
```

The `Legacy` variants are together called `Hook::Legacy`. Legacy writes become
`Some(CollisionHook::Legacy(f))`, a mechanical edit; Androsynth's save and restore
(`androsynth_native.rs:389-397`) still copies the whole value.
Legacy code that calls a hook field calls `invoke_*` instead, the one dispatcher.

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
remains (Phase 2 exit: all races, the AI and the element callbacks in `navigation/`),
a PR series swaps the storage for `OrderedPool`, links for slot indices, `pParent`
for `Parent`, `hTarget` for `Option<ElementId>`, unions for fields, `farray` for
`FrameSetRef`, and deletes `Hook::Legacy`, the authority statics and the record
types. The pools move into `Battle`, which each element-loop mode creates on entry,
if the migration trace shows no element survives a mode switch (`InitDisplayList`
reinitialises the queue at mode entry); otherwise they stay in `Kernel` and `Battle` borrows them. Order is
unchanged, so V1 traces stay identical.

`GlobData` follows the same stages: Phase 0 leaves one definition (Stage A); Phase 4
converts readers to `World` accessors over it (B), then replaces it with the owned
`World` (C).

## 5. Behaviour dispatch

### 5.1 Races: a trait object per species

The 28 initialisers in `ships/code_resources.rs:75-104` return pointers to
`static mut <RACE>_DESC` templates whose fields hold `Option<unsafe extern "C" fn>`.

```rust
pub trait Race: Sync {                                   // content; uses engine types only
    fn descriptor(&self) -> RaceDesc;                    // plain data template
    fn preprocess(&self, _cx: &mut Cx, _ship: ShipId) {}
    fn postprocess(&self, _cx: &mut Cx, _ship: ShipId) {}
    fn init_weapon(&self, cx: &mut Cx, ship: ShipId) -> WeaponSpawn;
    fn intelligence(&self, cx: &mut Cx, ship: ShipId, concerns: &mut Concerns);
    fn uninit(&self, _ship: &mut Starship) {}
}
pub fn race(species: Species) -> &'static dyn Race { match species { Species::Arilou => &Arilou, /* ... */ } }
```

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
- `unsafe` is allowed only in `platform::sdl` (texture upload, `sdl2-sys` calls),
  `platform::macos` (today `graphics/ffi_video_macos.rs`, the CoreFoundation and
  CoreGraphics parts of `automation/native_window.rs`, `automation/native_isolation.rs`),
  `platform::net` unless replaced by `std::net`, and `threading/qos.rs`. Software
  canvases become Rust pixel buffers (`Vec<u32>` RGBA8888, which the scalers already
  take); only the final present touches SDL. Every block carries `// SAFETY:`.

## 7. Interfaces with the rest of the system

**Content loading.** `io/uio_bridge.rs` becomes `content::Vfs`, keeping mount order
and precedence (`UIO_MOUNT_TOP/BOTTOM/ABOVE/BELOW`, `uio_bridge.rs:20-27`) and zip
packages. `resource/` becomes a typed `ResourceCache` returning `Handle<Frame>`,
`Handle<StringTable>`, `Handle<Sound>`; dropping the owner releases the resource,
replacing `CaptureDrawable`/`ReleaseDrawable`/`DestroyDrawable` pairs.

**Savegame.** The byte format and `encode.rs`/`decode.rs`/`lzh/` stay. The live
connection becomes

```rust
pub struct SessionView<'a> { pub battle: Option<&'a Battle>, pub solar: Option<&'a SolarSystem> }
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
`encode`/`decode`, tested against byte fixtures. The sync checksum stays byte-for-byte
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
