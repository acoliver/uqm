# Plan: Battle Engine Subsystem — Phase 1 Rust Port

Plan ID: PLAN-20260320-BATTLE
Generated: 2026-03-20
Total Phases: 37 (P00.5 through P18a, with verification sub-phases)
Requirements: Canonical REQ IDs defined in `plan/01-analysis.md` from `battle/requirements.md`; later phases must use those exact IDs only

## Context

The battle engine subsystem is **entirely C-owned**. There is no `USE_RUST_BATTLE` build toggle, no `battle` module in `rust/src/lib.rs`, and no Rust code anywhere in the codebase for the battle loop, element processing, collision detection, velocity computation, weapon mechanics, display list management, tactical transitions, or AI dispatch.

The following C files compose the battle engine (total ~5,640 lines of C):

| C File | Lines | Purpose |
|--------|-------|---------|
| `sc2/src/uqm/battle.c` | 517 | Battle entry, per-frame callback, input processing |
| `sc2/src/uqm/process.c` | 1,108 | Element PreProcess/PostProcess pipeline, collision orchestration, camera/zoom, rendering dispatch |
| `sc2/src/uqm/collide.c` | 183 | Elastic collision physics |
| `sc2/src/uqm/element.h` | 242 | ELEMENT struct definition and flags |
| `sc2/src/uqm/velocity.c` | 153 | Bresenham-style velocity system |
| `sc2/src/uqm/velocity.h` | 76 | Velocity types and macros |
| `sc2/src/uqm/weapon.c` | 414 | Weapon spawning, damage, projectile tracking |
| `sc2/src/uqm/weapon.h` | 68 | Weapon descriptor types |
| `sc2/src/uqm/displist.c` | 274 | Doubly-linked list pool allocator |
| `sc2/src/uqm/displist.h` | 131 | Queue/link types |
| `sc2/src/uqm/tactrans.c` | 1,032 | Ship death, explosion, transition, flee, winner tracking |
| `sc2/src/uqm/tactrans.h` | 59 | Tactical transition declarations |
| `sc2/src/uqm/intel.c` | 76 | AI dispatch |
| `sc2/src/uqm/intel.h` | 85 | AI constants and types |
| `sc2/src/uqm/ship.c` | 592 | Ship spawn, per-frame preprocess/postprocess, collision handler |
| `sc2/src/uqm/ship.h` | 43 | Ship runtime declarations |
| `sc2/src/uqm/init.c` | 363 | InitShips/UninitShips, space initialization |
| `sc2/src/uqm/units.h` | 227 | Coordinate/angle/trig systems |

The ships subsystem (`rust/src/ships/`) already exists with traits, types, loader, catalog, registry, runtime, and 25+ race implementations. The ships spec explicitly states: "The battle engine owns the overall battle loop, frame timing, element display list management, collision dispatch." The battle engine *uses* ships — it does not *own* them.

### Existing Rust ships types that will be relocated

The following types and functions currently live in `ships/runtime.rs` and must be extracted to a shared `battle_types` module (the battle engine's foundation):

- **Angle/facing constants:** `FACING_SHIFT`, `NUM_FACINGS`, `CIRCLE_SHIFT`, `FULL_CIRCLE`, `HALF_CIRCLE`, `QUADRANT`, `OCTANT`
- **Coordinate constants:** `VELOCITY_SHIFT`, `ONE_SHIFT`
- **Element constants:** `NORMAL_LIFE`, `MAX_SHIP_MASS`, `GRAVITY_THRESHOLD`, `PLAYER_SHIP`, `APPEARING`, `DISAPPEARING`, `CHANGING`, `COLLISION_FLAG`, `IGNORE_SIMILAR`, `FINITE_LIFE`
- **Trigonometry:** `sine()`, `cosine()`, `arctan()`, `SINE_TABLE`
- **Velocity:** `VelocityState` type and all methods
- **Conversions:** `normalize_facing()`, `facing_to_angle()`, `angle_to_facing()`, `normalize_angle()`, `display_to_world()`, `world_to_velocity()`, `velocity_to_world()`, `gravity_mass()`

After extraction, `ships/runtime.rs` re-exports from `battle_types` so existing race files require zero changes.

### Known bug — VelocityState.incr byte order (PREREQUISITE — already fixed)

The `VelocityState` implementation in `ships/runtime.rs` had **swapped byte order** in the `incr` field relative to C's `MAKE_WORD(lo, hi)` encoding, causing `get_current_components()` to produce incorrect results for negative velocities. This bug has been fixed. The battle engine's `VelocityDesc` will use the correct C byte order from the start.

### ships/runtime.rs impact note

The file `rust/src/ships/runtime.rs` currently has **1,503 lines** and **~47 tests**. The type relocation in P03 (extracting constants, trig, angles, velocity types to `battle_types/`) will touch a large surface area of this file. All 47 existing tests must continue to pass after relocation — verified by the re-export strategy (race files require zero changes).

## Port Strategy

### Phase 1 — Rust types and leaf operations, C-owned loop

This plan implements **Phase 1** of the three-phase migration strategy defined in the specification (§2.1):

- **Rust provides** the core `#[repr(C)]` types (`Element`, `VelocityDesc`, `IntersectControl`, `ElementVisualState`, `ElementFlags`) and **leaf** math/physics functions as a library.
- **C retains** the battle loop (`DoBattle`), process loop (`RedrawQueue`, `PreProcessQueue`, `PostProcessQueue`), collision orchestration (`ProcessCollisions`), and all frame dispatch.
- **C calls into Rust** for velocity computation, elastic collision response (the `collide()` physics math), weapon collision handling, homing weapon tracking, and netplay CRC per-element processing.
- **`ProcessCollisions` stays entirely in C** because it is deeply entangled with the process loop: it recursively calls `PreProcess`, walks the display list, and mutates element state that the loop depends on.
- **Rust defines types and constants** for all requirements (process loop, tactical transitions, battle lifecycle, ship runtime, AI dispatch, netplay integration, integration points) even though the corresponding logic stays in C for Phase 1. This ensures Phase 2/3 have a complete type foundation.

This matches the pattern used by the ships subsystem: C owns the loop, Rust provides behavior.

### What "types and constants" means for C-owned logic

Phase 1 does not port the process loop, battle lifecycle, tactical transitions, ship runtime pipeline, AI dispatch, or netplay synchronization. However, it **must** define:

1. **Type definitions** — Rust structs/enums that represent the data these systems operate on (e.g., `BattleState`, `ViewState`, `ZoomMode`, death pipeline phase enums, flee/warp constants)
2. **Constants** — All numeric constants these systems depend on (e.g., `BATTLE_FRAME_RATE`, `EXPLOSION_LIFE`, `FLEE_MASS`, zoom thresholds, AI range constants)
3. **Integration contracts** — Typed function signatures documenting how Rust will call into C subsystems (graphics, audio, threading, input, resource, ships) and vice versa
4. **Test infrastructure** — Tests verifying the correctness of all types, constant values, and contracts

This ensures that when Phase 2/3 ports the actual logic, the type foundation is already verified and stable.

### Initialization return value semantics

The C function `InitShips()` returns `SIZE` (i16, signed). The caller `Battle()` at `battle.c:515` tests `num_ships < 0` to detect hyperspace exit — **the sign is semantically meaningful**. Any cross-language binding must preserve the ability to return negative error values without silent reinterpretation.

The existing `USE_RUST_SHIPS` bridge declares `rust_ships_init()` as returning `COUNT` (u16, unsigned) — this is an ABI-level type mismatch that is latent today (the implementation only returns 0 or 2). The FFI adapter phase (P17) must use the correct `SIZE` (i16) return type, and a fix for the existing ships-side mismatch should be coordinated.

### Types designed for Phase 2/3

All types are designed to support the full migration path:

- **Phase 2 (future):** Rust owns the process loop. `PreProcessQueue`, `PostProcessQueue`, `ProcessCollisions` move to Rust. C retains `DoBattle` as a thin frame callback.
- **Phase 3 (future):** Full Rust battle loop. `Battle()`, `DoBattle()`, input processing, and frame timing move to Rust.

The `DisplayList` type includes generation counters and callback registry infrastructure that Phase 2/3 will need, even though Phase 1 doesn't use them (C owns the display list in Phase 1).

### Callback registry with generational handles

Phase 2+ will need a callback registry that maps element handles to Rust closures. The `Element` struct's callback fields remain C function pointers in all phases (`Option<unsafe extern "C" fn(...)>`). A separate registry (§2.5.1 of the spec) uses generational handles to prevent stale dispatch after element pool reuse. Phase 1 defines the types; Phase 2 activates them.

### Integration boundary with ships subsystem

The battle engine and ships subsystem share types through a new `battle_types` module:

```
battle_types ← battle   (battle imports shared types)
battle_types ← ships    (ships imports shared types via re-exports)
```

Neither `battle` nor `ships` directly depends on the other for types. Both depend on `battle_types`. This avoids bidirectional module coupling.

### Battle↔Ships weapon initialization adapter

When a ship fires, the ships subsystem creates weapon descriptors (`Vec<WeaponElement>` from `ShipBehavior::init_weapon()`). The battle engine must accept these and create actual elements. In Phase 1, the `rust_ships_init_weapon` bridge function (installed as the C `init_weapon_func` callback) handles this:

1. Calls `ShipBehavior::init_weapon()` to get `Vec<WeaponElement>` (high-level intent)
2. For each `WeaponElement`, builds a `MISSILE_BLOCK` or `LASER_BLOCK` from `WeaponElement` fields + `RaceDesc` weapon data
3. Calls C's `initialize_missile()`/`initialize_laser()` to do actual element allocation and field setup
4. Returns the spawned element handles

This adapter path is covered in P09 (weapon types including `LaserBlock`/`MissileBlock`) and P17 (FFI bridge with the `rust_ships_init_weapon` signature). The `WeaponElement → MISSILE_BLOCK/LASER_BLOCK` conversion must be explicitly implemented and tested.

### CElement interoperability / ffi_contract migration

Spec §3.4 defines conversion between ships' `ElementState` (Rust-native, not `#[repr(C)]`) and battle's `Element` (`#[repr(C)}`). Spec §15.8.5 decides that `CElement` in `ships/ffi_contract.rs` should be a non-opaque `#[repr(C)]` type aliased to `battle::Element`:

```rust
pub use crate::battle::element::Element as CElement;
```

This migration is covered in P04 (define `Element`) and P17 (update `ffi_contract.rs` to alias `CElement = Element`). Until `battle::Element` exists, the existing opaque `CElement` remains. The type relocation strategy (spec §3.5) ensures this is done atomically.

### Display primitive allocation/deallocation coupling

Requirements say "When an element is allocated, the battle engine shall also allocate a display primitive." In Phase 1, **C owns both allocations** — C's `AllocElement()` allocates from `disp_q` and also allocates a display prim from `DisplayArray`. Rust types merely need the `prim_index: u16` field to hold the binding. The Rust `DisplayList` type (P06) includes the field but does not manage primitive allocation in Phase 1. Phase 2+ will need `DisplayList::alloc_element()` that allocates both element slot and display primitive (via FFI callback to C's graphics subsystem).

## Phase 1 vs Phase 2+ Requirement Boundary

This plan implements **Phase 1 only**. The fundamental boundary is: **Phase 1 implements Rust types and leaf functions; C owns all orchestration loops.** Many requirements in `requirements.md` describe behavioral orchestration that is NOT implementable in Rust in Phase 1 because C owns those loops. This is not "uncovered" — it is correctly deferred to Phase 2+ because the orchestration code stays in C.

### Phase 1 implements (Rust types + leaf operations)

The following requirement areas are fully or partially implementable in Phase 1 as Rust types, constants, and leaf functions that C calls via FFI:

| Requirement Area | What Phase 1 Implements | Phases |
|-----------------|------------------------|--------|
| **Element types & flags** | `Element` struct, `ElementFlags`, union types, safe accessors, lifecycle helpers | P03–P05 |
| **Element constants** | `NORMAL_LIFE`, `MAX_CREW_SIZE`, `MAX_SHIP_MASS`, `GRAVITY_THRESHOLD`, all element flag bit positions | P03, P04 |
| **Display list pool types** | `DisplayList` struct, pool alloc/free, linked-list ops, `GenerationalHandle`, `CallbackRegistry` types | P06 |
| **Coordinate/precision math** | Three-tier conversions, toroidal wrapping, display↔world↔velocity shifts | P03 |
| **Angle/facing math** | 64-step angles, 16 facings, normalization, `facing_to_angle`, `angle_to_facing` | P03 |
| **Trigonometry** | `sine()`, `cosine()`, `arctan()`, `SINE_TABLE` (14-bit precision) | P03 |
| **Velocity operations** | `VelocityDesc` with full method suite: `get_current`, `get_next`, `set_vector`, `set_components`, `delta`, `zero`, `is_zero` — all bit-identical to C | P07 |
| **Elastic collision physics** | `elastic_collide()` — mass-based elastic response, impact angle, momentum transfer, min velocity, DEFY_PHYSICS, gravity mass exemption, player ship penalty | P08 |
| **Collision eligibility checks** | `is_collidable()`, `collision_possible()` — NONSOLID/DISAPPEARING/COLLISION/IGNORE_SIMILAR/mass checks | P05, P08 |
| **Weapon collision handling** | `weapon_collision()` — guard, damage, blast creation (8 directional bins, standard vs custom), sound dispatch, `do_damage()` | P09 |
| **Homing/tracking** | `track_ship()` — fast-path h_target, display list scan fallback, cloaking check, Manhattan distance, random turn on 180° | P09 |
| **Weapon block types** | `LaserBlock`, `MissileBlock`, initialization constants, `WeaponElement→LaserBlock/MissileBlock` conversion | P09 |
| **Netplay CRC** | `crc_process_element()` — streaming CRC-32, 35 bytes/element, exact field order, LE, bit-identical to C | P15 |
| **Checksum types** | `CrcState`, CRC-32 table, `BACKGROUND_OBJECT` exclusion | P15 |
| **Battle/process/lifecycle/tactical/AI/netplay types & constants** | Type definitions, enums, constants for all requirement areas (see P10–P16) — these define the Phase 2+ type foundation | P10–P16 |
| **Integration contracts** | Typed function signatures for graphics, audio, threading, input, resource, ship/race, global state interfaces | P16 |
| **FFI exports** | `rust_velocity_*`, `rust_battle_collide`, `rust_battle_weapon_collision`, `rust_battle_track_ship`, `rust_battle_crc_process_element` | P17 |

### Phase 2+ implements (orchestration that C owns in Phase 1)

The following requirement areas describe **behavioral orchestration** that stays entirely in C during Phase 1. Phase 1 defines types and constants for these areas to provide a verified type foundation, but the actual logic is NOT ported. These will be implemented when the corresponding C loops move to Rust.

#### Process loop orchestration (Phase 2)

| Requirement Area | Why Phase 2+ | Phase 1 Provides |
|-----------------|-------------|-----------------|
| **PreProcessQueue execution** (head-to-tail iteration, per-element PreProcess dispatch, collision detection against successors) | C owns `PreProcessQueue()` in `process.c` | PreProcess flag transition helpers, element lifecycle methods |
| **PostProcessQueue execution** (flag clearing, scroll application, removal, render list insertion) | C owns `PostProcessQueue()` in `process.c` | Flag transition helpers, element lifecycle methods |
| **Asymmetric DEFY_PHYSICS clearing** (COLLISION set → clear COLLISION keep DEFY_PHYSICS; COLLISION clear → clear DEFY_PHYSICS) | Implemented inside `PostProcessQueue()` in C | Flag transition helper methods on `Element` |
| **Newly-added element cascading** (inner loop for elements lacking PRE_PROCESS, full-list collision detection, tail-chasing for spawned elements) | Implemented inside `PostProcessQueue()` in C | Type definitions and constants |
| **Scroll offset application** (PRE_PROCESS/POST_PROCESS flag-dependent scroll delta) | Applied inside `PostProcessQueue()` in C | Type definitions |
| **Scroll/transform/render insertion timing** (coordinate transform, zoom-frame selection, postprocess callback, primitive insertion) | Implemented inside `PostProcessQueue()` in C | Coordinate conversion constants, zoom types |
| **Zoom calculation** (step/continuous modes, hysteresis to prevent oscillation) | Computed inside `PreProcessQueue()` in C | `ZoomMode` enum, `ViewState` enum, zoom threshold constants |
| **Zoom hysteresis** (different thresholds for zoom-in vs zoom-out transitions) | Implemented inside zoom calculation in C | Hysteresis threshold constants |
| **Camera calculation** (midpoint between ships, scroll clamping) | Computed inside `PreProcessQueue()` in C | Camera type definitions |
| **Camera single-ship clamping** (max per-frame jump distance when one ship active) | Implemented inside camera calculation in C | Clamping constants |
| **World-to-screen conversion** (step/continuous formulas with zoom) | Applied inside `PostProcessQueue()` in C | Conversion constants and `battle_types` functions |

#### Collision orchestration (Phase 2)

| Requirement Area | Why Phase 2+ | Phase 1 Provides |
|-----------------|-------------|-----------------|
| **ProcessCollisions orchestration** (display list walk, pair-wise dispatch, recursive earlier-time checks) | `ProcessCollisions()` in `process.c` is entangled with the process loop | Collision eligibility helpers |
| **Pixel-accurate trajectory detection** (intersection testing between current→next positions) | Calls C's `DrawablesIntersect()` inside `ProcessCollisions()` | `IntersectControl` type, `DrawablesIntersect` FFI declaration |
| **Forward-only vs full-list scan** (preprocess: successors only; postprocess cascading: full list from head) | Iteration logic inside `ProcessCollisions()` | Constants documenting scan modes |
| **Recursive earlier-time checks** (verify neither element intersects something earlier before dispatching) | Implemented inside `ProcessCollisions()` | — |
| **Player-ship-first dispatch ordering** (test element is PLAYER_SHIP → call test first, else current first) | Dispatch ordering inside `ProcessCollisions()` | Dispatch order type definitions |
| **Collision-point snapping** (snap next position to intersection location when COLLISION set) | Applied inside `ProcessCollisions()` | — |
| **Stuck overlap handling** (APPEARING kill, non-APPEARING position revert) | Applied inside `ProcessCollisions()` | Types and constants |
| **Post-bounce rechecks** (full-list rescan after velocity change) | Implemented inside `ProcessCollisions()` | Constants |

#### Battle lifecycle orchestration (Phase 3)

| Requirement Area | Why Phase 2+ | Phase 1 Provides |
|-----------------|-------------|-----------------|
| **Battle entry sequence** (RNG seed, music load, InitShips, ship count, activity flag, graphics scale, input order, ship spawn, music start) | `Battle()` in `battle.c` owns this sequence | `BattleState` type, lifecycle constants, `i16` return type |
| **Frame callback architecture** (DoInput pattern, InputFunc at offset 0) | `DoBattle()` in `battle.c` | `BattleState` `#[repr(C)]` type with offset-0 InputFunc |
| **Per-frame processing** (input → batch → simulate → render → exit check) | `DoBattle()` in `battle.c` | Frame processing stage enum, type definitions |
| **Frame timing** (24 fps normal, max-speed skip) | Timing logic in `battle.c` | `BATTLE_FRAME_RATE` constant, timing types |
| **Input processing loop** (side iteration, bit mapping, escape detection) | `ProcessInput()` in `battle.c` | Input type definitions, `BATTLE_INPUT_STATE` mapping |
| **Battle teardown sequencing** (stop audio, free assets, count crew, writeback, clear activity) | `UninitShips()` in `init.c` | Teardown sequence type definitions |
| **Shared asset reference counting** (nested init/deinit, load/free) | `init.c` manages reference counts | Reference counting type definitions |

#### Tactical transition orchestration (Phase 2/3)

| Requirement Area | Why Phase 2+ | Phase 1 Provides |
|-----------------|-------------|-----------------|
| **Ship death 4-phase pipeline** (ship_death → explosion → cleanup → new_ship callback chain) | `ship_death()` and callback replacements in `tactrans.c` | Death pipeline phase enum, transition constants |
| **Explosion animation** (36 frames, 1-3 debris/frame, frame 15 hide prim, frame 25 clear preprocess) | `explosion_preprocess()` in `tactrans.c` | `EXPLOSION_LIFE`, `NUM_EXPLOSION_FRAMES`, frame milestone constants |
| **Cleanup crew-pickup preservation** (iterate elements, clear ownership, mark deletion, preserve CREW_OBJECT) | `cleanup_dead_ship()` in `tactrans.c` | Cleanup type definitions, `CREW_OBJECT` flag |
| **New-ship readiness wait** (ditty playback finished, netplay sync complete) | `new_ship()` handler in `tactrans.c` | Spawn readiness type definitions |
| **Ship replacement selection order** (SuperMelee picker, encounter queue, infinite fleet recycling) | `tactrans.c` + melee UI | Selection order type definitions |
| **Winner determination iteration** (display-list-order, PLAYER_SHIP, break-first, Pkunk mass+1) | `find_alive_starship()` in `tactrans.c` | Winner determination type definitions, `MAX_SHIP_MASS` constant |
| **OpponentAlive semantics** (display list iteration, crew check, 3 return cases) | `OpponentAlive()` in `tactrans.c` | OpponentAlive type definitions |
| **Ship death recording** (decrement battle counter, melee notification) | `ship_death()` in `tactrans.c` | Type definitions |
| **Flee eligibility** (5 conditions: stamp prim, NORMAL_LIFE, no FINITE_LIFE, not FLEE_MASS, no APPEARING) | `flee_preprocess()` in `tactrans.c` | Flee eligibility condition type definitions, `FLEE_MASS` constant |
| **Flee initiation** (mass=10×MAX_SHIP_MASS, dark red stamp-fill, timing counters, input suppression) | `flee_preprocess()` in `tactrans.c` | Flee initiation step constants |
| **Flee animation** (20-color red pulse, accelerating timing, warp-out trigger) | `flee_preprocess()` in `tactrans.c` | 20-color palette constant array, timing type definitions |
| **Warp transition** (15 frames, ghost images, ion trail colors, materialization steps) | `ship_transition()` in `tactrans.c` | `HYPERJUMP_LIFE` constant, warp transition type definitions |
| **Warp ghost spawning** (per-frame ghost image positioning along facing vector) | `ship_transition()` in `tactrans.c` | Ghost image type definitions, ion trail color cycle |

#### Ship runtime orchestration (Phase 2/3)

| Requirement Area | Why Phase 2+ | Phase 1 Provides |
|-----------------|-------------|-----------------|
| **Ship per-frame pipeline order** (input → APPEARING → energy → preprocess → turn → thrust → status) | `ship_preprocess()` in `ship.c` | Pipeline stage enum/documentation, exact order contract |
| **Ship spawn placement** (random position avoiding gravity wells, Sa-Matra center) | `spawn_ship()` in `ship.c`/`init.c` | Spawn position types, constants |
| **Ship first-frame initialization** (suppress inputs, init crew display, invoke race preprocess, start warp-in) | `ship_preprocess()` APPEARING handling in `ship.c` | Type definitions |
| **Energy regeneration dispatch** | `ship_preprocess()` in `ship.c` | Energy regeneration constants |
| **Turn/thrust processing dispatch** | `ship_preprocess()` in `ship.c` | Turn/thrust constants |
| **Weapon firing pipeline** (postprocess: cooldown check, energy deduct, weapon callback, bind elements) | `ship_postprocess()` in `ship.c` | Weapon block types, firing constants |

#### AI dispatch orchestration (Phase 2/3)

| Requirement Area | Why Phase 2+ | Phase 1 Provides |
|-----------------|-------------|-----------------|
| **Computer intelligence entry point** (RPG overlay merge, Sa-Matra disabled, PSYTRON random picker) | `computer_intelligence()` in `intel.c` | AI dispatch type definitions |
| **AI behavioral dispatch** (call race-specific `intelligence()` callback) | `computer_intelligence()` in `intel.c` | `EvaluateDesc` type, AI constants |
| **Object tracking system** (concern-type indexed array) | `intel.c` tracking infrastructure | Tracking index constants |

#### Netplay orchestration (Phase 2/3)

| Requirement Area | Why Phase 2+ | Phase 1 Provides |
|-----------------|-------------|-----------------|
| **Input buffering** (configurable delay, push/pop per side) | Netplay input loop integration | Input buffer hook type definitions |
| **Frame synchronization** (CRC verification interval, mismatch → abort) | Frame sync logic in battle loop | Frame sync type definitions, `crc_process_element()` function |
| **Battle-end multi-phase protocol** (in-battle → ending → phase-2 → inter-battle) | Multi-phase state machine in battle/tactrans | Protocol phase enum |

#### Display list rendering order (Phase 2)

| Requirement Area | Why Phase 2+ | Phase 1 Provides |
|-----------------|-------------|-----------------|
| **Rendering-order linked list** (separate linked list of display primitives ordered by display position for visual layering) | Maintained by `PostProcessQueue()` in `process.c`; display primitives are C-owned | Type definitions in P06; `prim_index` field on Element for binding |

#### Teardown/double-buffer robustness (Phase 2/3)

| Requirement Area | Why Phase 2+ | Phase 1 Provides |
|-----------------|-------------|-----------------|
| **Teardown robustness** (ships never fully spawned, absent hooks, already-freed descriptors) | Orchestration-level robustness in `UninitShips()` | Types are correct and exhaustion-safe |
| **Double-buffer invariant consistency** (current/next maintained across frames) | Enforced by process loop in C | `commit_state()` method, documentation |

## C Files Replaced

Phase 1 does **not** replace any C files. C retains all battle loop, process loop, and dispatch code. Rust provides leaf functions that C calls conditionally.

### C Files Modified (guarded, not deleted)

| C File | Change |
|--------|--------|
| `sc2/src/uqm/velocity.c` | Guard velocity function bodies behind `#ifndef USE_RUST_BATTLE`; add `extern` declarations for Rust replacements |
| `sc2/src/uqm/collide.c` | Guard `collide()` body behind `#ifndef USE_RUST_BATTLE`; add `extern` for `rust_battle_collide` |
| `sc2/src/uqm/weapon.c` | Guard `weapon_collision()` and `TrackShip()` bodies behind `#ifndef USE_RUST_BATTLE`; add `extern` declarations |
| `sc2/build/unix/build.config` | Add `USE_RUST_BATTLE` toggle (symbol + substitution variable) |
| `sc2/config_unix.h` | Add `#define USE_RUST_BATTLE` (when Rust bridge enabled) |

### C Files NOT modified (remain pure C in Phase 1)

| C File | Reason |
|--------|--------|
| `battle.c` | Owns the battle loop — Phase 3 concern |
| `process.c` | Owns `PreProcessQueue`/`PostProcessQueue`/`ProcessCollisions` — Phase 2 concern |
| `displist.c` | Owns display list allocation — Phase 2 concern |
| `tactrans.c` | Owns tactical transitions — Phase 2/3 concern |
| `intel.c` | Owns AI dispatch — Phase 2/3 concern |
| `ship.c` | Already has `USE_RUST_SHIPS` guards — ships subsystem concern |
| `init.c` | Already has `USE_RUST_SHIPS` guards — ships subsystem concern |

## Rust Module Structure

```text
rust/src/battle_types/
  mod.rs                    # Shared foundation module — no deps on battle or ships
  coords.rs                 # VELOCITY_SHIFT, ONE_SHIFT, display_to_world, toroidal wrapping, etc.
  trig.rs                   # sine, cosine, arctan, SINE_TABLE
  angles.rs                 # FACING_SHIFT, NUM_FACINGS, normalize_facing, etc.

rust/src/battle/
  mod.rs                    # Module root, pub exports
  types.rs                  # Element, VelocityDesc, IntersectControl, ElementVisualState,
                            #   ElementFlags, ElementImage, Stamp, Point, Extent, Color,
                            #   union types, handle types, BattleState
  element.rs                # Element methods: accessors, commit_state, is_collidable,
                            #   collision_possible, lifecycle helpers
  display_list.rs           # DisplayList struct, pool ops (alloc/free/push_back/remove/iter),
                            #   GenerationalHandle, CallbackRegistry (Phase 2+ ready)
  velocity.rs               # VelocityDesc methods: get_current_components, get_next_components,
                            #   set_vector, set_components, delta_components, zero, is_zero
  collision.rs              # elastic_collide() — mass-based elastic collision response,
                            #   collision eligibility helpers
  weapon.rs                 # LaserBlock, MissileBlock, weapon_collision(), blast creation,
                            #   damage application, TrackShip/homing logic
  process_loop.rs           # Process loop types for Phase 2: ViewState, ZoomMode,
                            #   PreProcess/PostProcess flag transition helpers,
                            #   zoom calculation constants, camera types,
                            #   world-to-screen conversion constants
  lifecycle.rs              # Battle lifecycle types: BattleState (#[repr(C)]),
                            #   battle entry/teardown type definitions,
                            #   frame callback types, input processing types,
                            #   frame timing constants, reference counting types
  tactical.rs              # Tactical transition types: death pipeline phase enum,
                            #   explosion constants (36 frames, debris, frame 15/25),
                            #   cleanup types (crew pickup preservation),
                            #   new ship spawn types (readiness conditions, selection order),
                            #   winner determination types (display-list-order, Pkunk),
                            #   OpponentAlive semantics, ship death recording,
                            #   ion trail constants (12-color fade),
                            #   warp transition types (15 frames, ghost images),
                            #   flee sequence types (eligibility, 20-color pulse, warp-out)
  ship_runtime.rs           # Ship runtime within battle types: spawn position types,
                            #   ship per-frame pipeline order documentation,
                            #   inertial movement model constants,
                            #   ship collision types, weapon firing types,
                            #   crew/energy management constants
  ai.rs                     # AI dispatch types: computer intelligence entry point types,
                            #   EvaluateDesc, control flags, AI constants (range thresholds,
                            #   maneuverability indices), object tracking indices,
                            #   Sa-Matra disabled AI, PSYTRON random picker
  netplay.rs                # CrcState, crc_process_element, CRC table,
                            #   per-frame checksum computation,
                            #   input buffering hook types,
                            #   frame synchronization types (checksum verification),
                            #   battle-end synchronization protocol types
                            #   (in-battle → ending-battle → phase-2 → inter-battle)
  integration.rs            # Integration contract types: graphics subsystem interface
                            #   (primitive types, draw batch, scale, contexts),
                            #   audio subsystem interface (positioned sound, music),
                            #   threading subsystem interface (TaskSwitch, SleepThreadUntil),
                            #   input subsystem interface (PlayerInput, frameInput),
                            #   resource subsystem interface (LoadGraphic, capture/release),
                            #   ship/race subsystem interface (ShipBehavior dispatch),
                            #   global state interface (CurrentActivity, RNG)
  ffi.rs                    # All extern "C" FFI exports: rust_velocity_*, rust_battle_collide,
                            #   rust_battle_weapon_collision, rust_battle_track_ship,
                            #   rust_battle_crc_process_element
  c_bridge.rs               # Rust-to-C calls: DrawablesIntersect, sound/graphics APIs,
                            #   GetElementStarShip, element/prim allocation (for Phase 2+)
  constants.rs              # Battle-specific constants: MAX_DISPLAY_ELEMENTS, MAX_DISPLAY_PRIMS,
                            #   HYPERJUMP_LIFE, zoom/explosion/weapon/flee/timing constants
```

## Requirements Traceability

Every requirement from `requirements.md` maps to at least one implementation phase. The tables below trace each requirements section to the phase(s) that implement it.

**Column conventions for orchestration-heavy sections:** Where requirements span both Phase 1 (types/leaf functions) and Phase 2+ (behavioral orchestration), tables use "Impl Phase" to indicate when the behavioral logic will be ported to Rust, and "Types Phase" for the Phase 1 phases that define types and constants. An "Impl Phase" of **Phase 2** or **Phase 2/3** or **Phase 3** (bold) means the orchestration logic stays in C for Phase 1 and will be ported in a future plan. An "Impl Phase" of "Phase 1" means the logic is fully implemented in Rust by this plan.

### Entity Model and Element System

| Requirement Area | Primary Phase | Supporting Phase(s) | What Phase 1 Delivers |
|-----------------|---------------|--------------------|-----------------------|
| **Entity model** (element struct, fields, unions, lifecycle) | P04, P05 | — | `Element` struct, union types, accessors, `ElementFlags` |
| **Element state flags** (all 14 flags, bit positions) | P04 | — | `ElementFlags` bitflags type with compile-time verified bit positions |
| **Element callbacks** (4 callbacks, null=no-op, self-replacement) | P04, P05 | — | Callback function pointer types, lifecycle helpers |
| **Element lifecycle** (life_span→death, DISAPPEARING, flag transitions) | P05 | P10 | `commit_state`, `is_collidable`, flag transition methods |
| **Element union fields** (crew/hp, turn/blast, color cycle) | P04 | P05 | `#[repr(C)]` union types with safe accessors |
| **Element union-field lifecycle semantics** (ship vs weapon interpretation) | P05 | — | Documentation + accessors context-aware for PLAYER_SHIP |
| **Element constants** (NORMAL_LIFE, MAX_CREW_SIZE, MAX_SHIP_MASS, GRAVITY_THRESHOLD) | P03, P04 | — | `battle_types` shared constants + `battle/constants.rs` |

### Display List Management

| Requirement Area | Primary Phase | Supporting Phase(s) | What Phase 1 Delivers |
|-----------------|---------------|--------------------|-----------------------|
| **Pool allocation** (150 cap, free chain, exhaustion handling) | P06 | — | `DisplayList` struct with full pool semantics |
| **Display list operations** (alloc/free/push_back/insert/remove/count/iter) | P06 | — | All operations matching C `QUEUE` API |
| **Display primitive management** (330 cap, 5 prim types, independent free list) | P06 | P04 | Constants and type definitions; C owns prims in Phase 1; `prim_index` field on Element binds element↔prim |
| **Display primitive allocation coupling** ("When an element is allocated, the battle engine shall also allocate a display primitive") | P06 | P04 | In Phase 1, C's `AllocElement()` performs both allocations. Rust `Element` carries `prim_index: u16` for the binding. Rust `DisplayList::alloc_element()` in Phase 2+ will allocate both via FFI callback. |
| **Rendering-order linked list** (separate display-primitive linked list for visual layering) | **Phase 2** (P06 types) | — | Type definitions only; C owns the rendering-order list entirely in Phase 1. Phase 2+ will port this when `PostProcessQueue` moves to Rust. |
| **GenerationalHandle and CallbackRegistry** (spec §2.5.1) | P06 | — | Types defined and tested with full lifecycle (alloc→increment generation, register on spawn, unregister on DISAPPEARING removal, reinit clears registry); activated in Phase 2+ |

### Coordinate and Precision System

| Requirement Area | Primary Phase | Supporting Phase(s) | What Phase 1 Delivers |
|-----------------|---------------|--------------------|-----------------------|
| **Three-tier precision** (display/world/velocity coords, bit-shift conversion) | P03 | — | `battle_types/coords.rs` — extracted from ships/runtime |
| **Logical space dimensions** (LOG_SPACE_WIDTH/HEIGHT, zoom levels) | P04 | P10 | `constants.rs` |
| **Toroidal wrapping** (WRAP_X/Y, shortest-path delta) | P03 | — | `battle_types/coords.rs` — wrapping functions |
| **Angle and facing systems** (64-step angles, 16 facings, normalization) | P03 | — | `battle_types/angles.rs` — extracted from ships/runtime |
| **Trigonometry** (sine/cosine/arctan lookup tables, 14-bit precision) | P03 | — | `battle_types/trig.rs` — extracted from ships/runtime |
| **Screen layout** (STATUS_WIDTH, SPACE_WIDTH, universe coords) | P04 | — | `constants.rs` |

### Velocity System

| Requirement Area | Primary Phase | Supporting Phase(s) | What Phase 1 Delivers |
|-----------------|---------------|--------------------|-----------------------|
| **Velocity descriptor** (Bresenham-style, 5 fields, accumulation) | P07 | — | `VelocityDesc` with full method suite |
| **Increment encoding** (MAKE_WORD packed format, FFI-critical) | P07 | — | Exact C byte-order replication |
| **Velocity operations** (get_current, get_next, set_vector, set_components, delta, zero) | P07 | — | All 6 operations on `VelocityDesc`, bit-identical to C |

### Collision System

Phase 1 implements **collision physics** (the math that computes velocity changes). Phase 2+ implements **collision orchestration** (the process loop code that detects intersections, walks the display list, dispatches handlers, and manages post-collision state).

| Requirement Area | Impl Phase | Types Phase | What Phase 1 Delivers |
|-----------------|-----------|------------|----------------------|
| **Collision eligibility** (NONSOLID, DISAPPEARING, COLLISION flag, IGNORE_SIMILAR, mass) | Phase 1 | P05, P08 | `collision_possible()`, `is_collidable()` |
| **Elastic collision response** (impact angle, momentum, mass-based, min velocity) | Phase 1 | P08 | `elastic_collide()` — the leaf physics function |
| **Gravity mass exemption** (mass_points ≥ 100 immovable) | Phase 1 | P08, P03 | `gravity_mass()` function (in `battle_types`) |
| **Player ship collision penalty** (clear max-speed, add wait counters) | Phase 1 | P08 | Implemented within `elastic_collide()` |
| **Collision detection** (pixel-accurate intersection, trajectory-based) | **Phase 2** | P08 | `IntersectControl` type, `DrawablesIntersect` FFI declaration |
| **Collision dispatch** (pair-wise, PLAYER_SHIP ordering, forward-only vs full-list) | **Phase 2** | P08 | Dispatch order types (orchestration stays in C) |
| **Recursive earlier-time checks** (verify no earlier intersection before dispatch) | **Phase 2** | P08 | — |
| **Collision-point snapping** (snap next position to intersection location) | **Phase 2** | P08 | — |
| **Stuck object handling** (APPEARING kill, position revert) | **Phase 2** | P08 | Types and constants (orchestration stays in C) |
| **Post-collision position/physics** (position snap, COLLISION flag) | **Phase 2** | P08 | Helper types (orchestration stays in C) |
| **Post-bounce collision rechecks** (full-list rescan after velocity change) | **Phase 2** | P08 | Constants (rescan stays in C) |

### Weapon System

| Requirement Area | Primary Phase | Supporting Phase(s) | What Phase 1 Delivers |
|-----------------|---------------|--------------------|-----------------------|
| **Laser initialization** (LINE_PRIM, life=1, start position from ship+offset, velocity=endpoint−startpoint, register weapon collision callback) | P09 | — | `LaserBlock` type with all initialization fields, initialization constants; exact behavior documented per requirements |
| **Missile initialization** (STAMP_PRIM, configurable hp/damage/life_span/speed/optional preprocess, spawn position from ship+offset, velocity from speed+facing, back up position by one velocity step) | P09 | — | `MissileBlock` type with all initialization fields, initialization constants; back-up-by-one-step documented |
| **Weapon collision** (guard, damage, sound, destroy, blast) | P09 | — | `weapon_collision()` leaf function |
| **Blast effect creation** (8 directional bins via velocity angle quantized to 16÷2 with even/odd rounding, standard 2-frame blast when frame count ≤ 16 from shared array, custom multi-frame blast when count > 16 from weapon farray with animation preprocess callback) | P09 | — | Blast logic within `weapon_collision()` with both standard and custom paths |
| **Damage application** (decrement hit_points/crew, zero→death) | P09 | — | `do_damage()` helper |
| **Damage silhouette** (rejection-sampling intersection within ship's silhouette for status panel indicators) | P09 | P16 | Type definitions for damage silhouette API; Phase 1 rendering stays in C; requires `DrawablesIntersect` from graphics integration |
| **Homing and tracking** (h_target fast path, display list scan, cloaking) | P09 | — | `track_ship()` leaf function |
| **Weapon firing from ships** (cooldown, energy, weapon array) | P12 | P09 | Weapon block types + ship postprocess pipeline types |
| **WeaponElement→MISSILE_BLOCK/LASER_BLOCK adapter** (ships' `init_weapon()` returns `Vec<WeaponElement>`, bridge builds MISSILE_BLOCK/LASER_BLOCK, calls C `initialize_missile()`/`initialize_laser()`) | P09, P17 | P12 | `LaserBlock`/`MissileBlock` types in P09; `rust_ships_init_weapon` FFI bridge in P17 |

### Ion Trail

| Requirement Area | Primary Phase | Supporting Phase(s) | What Phase 1 Delivers |
|-----------------|---------------|--------------------|-----------------------|
| **Ion trail** (12-color orange→red fade cycle, one color per frame; POINT_PRIM type; inserted at head of display list (drawn behind everything); marked as already preprocessed with life span pre-decremented because head-inserted elements skip normal preprocessing) | P13 | P06 | 12-color palette constant array, ion trail type definitions including display list head-insertion semantics, pre-processed flag and pre-decremented life span documentation |

### Process Loop

| Requirement Area | Impl Phase | Types Phase | What Phase 1 Delivers |
|-----------------|-----------|------------|----------------------|
| **Top-level frame dispatch** (SetContext, PreProcess, PostProcess, sounds, render) | **Phase 2** | P10 | Frame dispatch sequence documentation + type definitions |
| **PreProcessQueue** (head-to-tail iteration, PreProcess, collision, camera) | **Phase 2** | P10, P05 | PreProcess flag transition helpers + camera types |
| **PreProcess per-element** (life_span check, APPEARING, velocity, flags) | **Phase 2** | P10, P05, P07 | Lifecycle methods on Element, velocity operations |
| **PostProcessQueue** (flag clearing, scroll, removal, rendering) | **Phase 2** | P10, P05 | Flag transition helpers in Element |
| **Newly-added element cascading** (inner loop, full-list collision) | **Phase 2** | P10 | Type definitions + constants |
| **Scroll offset application** (PRE/POST_PROCESS flag-dependent) | **Phase 2** | P10 | Type definitions |
| **Asymmetric DEFY_PHYSICS clearing** (COLLISION→keep DEFY; no COLLISION→clear DEFY) | **Phase 2** | P10, P05 | Flag transition helper methods |
| **Element removal and rendering setup** (DISAPPEARING removal, coordinate transform) | **Phase 2** | P10, P05 | Element lifecycle helpers |
| **Zoom calculation** (step/continuous modes, hysteresis) | **Phase 2** | P10 | `ZoomMode` enum, `ViewState` enum, zoom constants |
| **Zoom hysteresis** (different thresholds for zoom-in vs zoom-out) | **Phase 2** | P10 | Hysteresis threshold constants |
| **Camera calculation** (midpoint, scroll clamping, view states) | **Phase 2** | P10 | Camera type definitions |
| **Camera single-ship clamping** (max per-frame jump when one ship active) | **Phase 2** | P10 | Clamping constants |
| **World-to-screen conversion** (step/continuous formulas) | **Phase 2** | P10, P03 | Coordinate conversion constants + `battle_types` functions |

### Battle Lifecycle

| Requirement Area | Impl Phase | Types Phase | What Phase 1 Delivers |
|-----------------|-----------|------------|----------------------|
| **Battle entry** (RNG seed, music, InitShips, ship count) | **Phase 3** | P11 | `BattleState` type, lifecycle constants |
| **InitShips return type** (returns `SIZE`/i16; negative = hyperspace exit; caller tests `num_ships < 0`) | **Phase 3** | P11, P17 | Correct `i16` return type in `BattleState` types; FFI adapter uses `i16` not `u16` |
| **Frame callback architecture** (DoInput pattern, InputFunc at offset 0) | **Phase 3** | P11 | `BattleState` `#[repr(C)]` type with offset-0 InputFunc |
| **Per-frame processing** (input, batch, simulate, render, exit check) | **Phase 3** | P11, P10 | Type definitions + frame dispatch documentation |
| **Frame timing** (24 fps, max-speed mode) | **Phase 3** | P11 | `BATTLE_FRAME_RATE` constant, timing types |
| **Input processing** (side iteration, bit mapping, escape) | **Phase 3** | P11 | Input type definitions, `BATTLE_INPUT_STATE` mapping |
| **Battle teardown** (stop audio, free assets, crew count, writeback) | **Phase 3** | P11 | Teardown sequence type definitions |
| **Shared asset initialization** (reference counting, load/free) | **Phase 3** | P11 | Reference counting type definitions |

### Ship Runtime Within Battle

| Requirement Area | Impl Phase | Types Phase | What Phase 1 Delivers |
|-----------------|-----------|------------|----------------------|
| **Ship spawn** (load descriptor, patch crew, set flags, random position) | **Phase 2/3** | P12 | Spawn type definitions (ships subsystem concern for logic) |
| **Ship per-frame pipeline** (input → APPEARING → energy → preprocess → turn → thrust → status) | **Phase 2/3** | P12 | Pipeline stage enum/documentation, exact order contract |
| **Inertial movement model** (thrust, coast, max speed, gravity) | Phase 1 (math) | P12, P07 | Inertial movement constants + velocity operations (already in ships/runtime.rs) |
| **Ship collision** (gravity damage, non-gravity elastic) | Phase 1 (physics) | P08, P12 | `elastic_collide()` + ship-specific collision constants |
| **Crew and energy** (regeneration, deduction, capping) | **Phase 2/3** | P12 | Crew/energy management constants |
| **Element reuse** (queue entry already has allocated handle → reinitialize in place) | **Phase 2/3** | P12, P06 | DisplayList alloc/reinit types |
| **Final battle center placement** (Sa-Matra defending ship) | **Phase 2/3** | P12 | Constant + type definition |
| **Random position avoiding gravity wells** | **Phase 2/3** | P12 | Type definitions |

### Tactical Transitions

| Requirement Area | Impl Phase | Types Phase | What Phase 1 Delivers |
|-----------------|-----------|------------|----------------------|
| **Ship death sequence** (exact 4-phase ordering, callback replacement) | **Phase 2/3** | P13 | Death pipeline phase enum + transition constants |
| **Ship explosion** (36 frames, debris spawning, frame 15/25 milestones) | **Phase 2/3** | P13 | `EXPLOSION_LIFE`, `NUM_EXPLOSION_FRAMES`, frame milestone constants |
| **Cleanup after explosion** (crew pickup preservation, winner, victory music) | **Phase 2/3** | P13 | Cleanup type definitions, CREW_OBJECT flag usage |
| **New ship spawning** (readiness conditions, descriptor free, queue deactivation) | **Phase 2/3** | P13 | Spawn readiness type definitions |
| **Ship replacement selection order** (SuperMelee picker, encounter queue, infinite fleet recycling) | **Phase 2/3** | P13 | Selection order type definitions |
| **Winner determination** (display-list-order, PLAYER_SHIP, break-first, mutual destruction) | **Phase 2/3** | P13 | Winner determination type definitions + constants |
| **Pkunk reincarnation** (mass == MAX_SHIP_MASS + 1, treated as alive) | **Phase 2/3** | P13 | `MAX_SHIP_MASS` constant, reincarnation documentation |
| **OpponentAlive semantics** (display list iteration, crew check, 3 return cases) | **Phase 2/3** | P13 | OpponentAlive type definitions |
| **Ship death recording** (decrement battle counter, melee notification) | **Phase 2/3** | P13 | Type definitions |
| **Ion trail** (POINT_PRIM, head-insert, 12-color fade, pre-processed, life span pre-decremented) | **Phase 2/3** | P13, P06 | 12-color palette constant, ion trail type definitions with head-insertion semantics |
| **Ship warp transition** (15 frames, ghost images per frame along facing vector, materialization: show prim, select zoom frame, init intersection, zero velocity, clear NONSOLID/FINITE_LIFE, restore callbacks) | **Phase 2/3** | P13 | `HYPERJUMP_LIFE` constant, warp transition type definitions including all 6 materialization steps |
| **Flee sequence — eligibility** (5 conditions: stamp prim, life_span=NORMAL_LIFE, no FINITE_LIFE, mass≠FLEE_MASS, no APPEARING; silent reject on any failure) | **Phase 2/3** | P13 | Flee eligibility condition type definitions |
| **Flee sequence — initiation** (decrement battle counter, replace preprocess with flee handler, mass=10×MAX_SHIP_MASS, zero velocity, clear max-speed flags, dark red stamp-fill, clear color cycle index, set initial timing counters, suppress input) | **Phase 2/3** | P13 | `FLEE_MASS` constant, flee initiation step constants |
| **Flee sequence — animation** (20-color red pulse dark→bright→dark, timing counters accelerate each cycle, all inputs suppressed; at timing=0 + color=midpoint: crew=0, set death callback to cleanup, trigger warp-out) | **Phase 2/3** | P13 | 20-color palette constant array, flee animation timing type definitions |

### AI Dispatch

| Requirement Area | Impl Phase | Types Phase | What Phase 1 Delivers |
|-----------------|-----------|------------|----------------------|
| **Computer intelligence entry point** (RPG overlay, Sa-Matra disabled, PSYTRON picker) | **Phase 2/3** | P14 | AI dispatch type definitions |
| **AI constants** (range thresholds, maneuverability indices) | Phase 1 | P14 | `CLOSE_RANGE_WEAPON`, `LONG_RANGE_WEAPON`, `FAST_SHIP`, etc. |
| **Object tracking system** (concern-type indexing) | **Phase 2/3** | P14 | `EvaluateDesc` type, tracking index constants |
| **Control flags** (HUMAN/CYBORG/PSYTRON/NETWORK, AI ratings) | Phase 1 | P14 | Control flag constants |

### Netplay Integration

| Requirement Area | Impl Phase | Types Phase | What Phase 1 Delivers |
|-----------------|-----------|------------|----------------------|
| **Checksum-critical fields** (35 bytes, 19 fields, exact order, LE) | Phase 1 | P15 | `crc_process_element()` — streaming CRC, bit-identical to C |
| **Excluded fields** (player_nr, prim_index, image, pointers) | Phase 1 | P15 | Implemented by omission in CRC function |
| **Input buffering** (configurable delay, push/pop) | **Phase 2/3** | P15 | Input buffer hook type definitions |
| **Frame synchronization** (CRC verification, mismatch → abort) | **Phase 2/3** | P15 | Frame sync type definitions, CRC computation function |
| **Battle-end synchronization** (multi-phase protocol) | **Phase 2/3** | P15 | Protocol phase enum (in-battle → ending → phase-2 → inter-battle) |
| **Determinism obligations** (bit-identical, no floating-point) | Phase 1 | P07, P08, P15 | Enforced by integer-only arithmetic across all leaf functions |

### Integration Points

| Requirement Area | Primary Phase | Supporting Phase(s) | What Phase 1 Delivers |
|-----------------|---------------|--------------------|-----------------------|
| **Graphics integration** (primitive types, draw batch, scale, contexts) | P16 | P04, P06 | Integration contract type definitions in `integration.rs` |
| **Audio integration** (positioned sound, music, stereo, flush) | P16 | — | Integration contract type definitions |
| **Threading integration** (TaskSwitch, SleepThreadUntil, DoInput) | P16 | — | Integration contract type definitions |
| **Input integration** (PlayerInput, frameInput, raw-to-battle) | P16 | — | Integration contract type definitions |
| **Resource integration** (LoadGraphic, capture/release/destroy) | P16 | — | Integration contract type definitions |
| **Ship/race integration** (ShipBehavior trait, race queues, load/free) | P16 | — | Integration contract type definitions |
| **Global state** (CurrentActivity flags, RNG, space type detection) | P16 | — | Global state interface type definitions |

### Cross-Language Boundary, Error Handling, Determinism

| Requirement Area | Primary Phase | Supporting Phase(s) | What Phase 1 Delivers |
|-----------------|---------------|--------------------|-----------------------|
| **Cross-language boundary** (InitShips return type including negative values, element field ordering) | P17 | P04, P11 | FFI function signatures with correct i16 return type, ABI-verified types |
| **Element structure interoperability** (CElement = Element alias, field order, link-first layout) | P04 | P17 | Compile-time size/offset assertions; P17 migrates ffi_contract.rs CElement |
| **Behavioral hooks via callbacks** (4 registered callbacks) | P04, P05 | — | Callback types in Element |
| **Error handling** (pool exhaustion robust, no corruption, deterministic order) | P06, P08 | — | Pool exhaustion returns NULL_HANDLE |
| **Double-buffer invariant** (current/next consistency) | P05 | — | `commit_state()` method |
| **Cooperative scheduling** (DoInput pattern, frame timing, batching) | P11 | — | Type definitions |
| **Frame rate and speed control** (24 fps, max-speed suppression) | P11 | — | `BATTLE_FRAME_RATE` constant |

## Phase Structure

| Phase | Title | Requirements Covered | Est. LoC | Subagent |
|-------|-------|---------------------|----------|----------|
| P00.5 | Preflight Verification | Toolchain, dependencies, existing ships module structure; **Prerequisite: VelocityState byte-order fix** already applied (verify `ships/runtime.rs` `incr` field uses correct C byte order) | 0 | deepthinker |
| P01 | Analysis | Canonical requirement index, subsystem inventory, C code survey, resolve spec §18 open design decisions | 0 | rustreviewer |
| P01a | Analysis Verification | — | 0 | deepthinker |
| P02 | Pseudocode | Algorithm pseudocode for Phase 1 leaf functions: **elastic collision** (impact angle, momentum transfer, min velocity, DEFY_PHYSICS), **weapon collision** (guard, damage, blast 8-bin direction, standard vs custom path), **homing/tracking** (h_target fast path, display list scan, Manhattan distance, 180° random turn), **CRC serialization** (35-byte field order, streaming CRC-32), **velocity operations** (Bresenham accumulation, set_vector trig decomposition, set_components arctangent, delta recompose) | 0 | rustreviewer |
| P02a | Pseudocode Verification | — | 0 | deepthinker |
| P03 | Shared Foundation — `battle_types` Module | Extract coords, trig, angles from `ships/runtime.rs` (1,503 lines, ~47 tests) to `battle_types/`; re-exports in runtime.rs; VelocityState.incr byte order already fixed (prerequisite complete); toroidal wrapping functions. **TDD:** RED: test `battle_types::sine(16)` == `ships::runtime::sine(16)` for 10 known angles; test `wrap_x`/`wrap_y` for boundary cases; test `shortest_path_delta` for both-direction wrapping. GREEN: implement extraction. REFACTOR: verify all 47 existing ships tests still pass unchanged via re-exports. | ~400 | rustcoder |
| P03a | Shared Foundation Verification | — | 0 | deepthinker |
| P04 | Core Types & Constants | `Element`, `ElementFlags`, `VelocityDesc`, `IntersectControl`, `ElementVisualState`, `Point`, `Extent`, `Color`, `BattleState`, union types, handle types; all battle constants; `battle/mod.rs` module structure. `prim_index` field included for element↔primitive binding. **TDD:** RED: write compile-time `assert_eq!(size_of::<Element>(), C_ELEMENT_SIZE)` and `assert_eq!(offset_of!(Element, state_flags), C_STATE_FLAGS_OFFSET)` for all fields — these fail against stubs. RED: test all 14 `ElementFlags` bit positions match C `element.h`. GREEN: implement `#[repr(C)]` types to pass assertions. REFACTOR: extract shared constants to `constants.rs`. | ~600 | rustcoder |
| P04a | Core Types Verification | Compile-time size/offset assertions matching C layout | 0 | deepthinker |
| P05 | Element Methods & Lifecycle | Element accessors (safe union wrappers), `commit_state()`, `is_collidable()`, `collision_possible()`, flag transition helpers, lifecycle documentation. **TDD:** RED: test `is_collidable` returns false for NONSOLID and DISAPPEARING elements. RED: test `collision_possible` enforces IGNORE_SIMILAR when same parent. RED: test `commit_state` copies next→current exactly. RED: test asymmetric DEFY_PHYSICS flag clearing helper. GREEN: implement methods. REFACTOR: consolidate union accessor patterns. | ~300 | rustcoder |
| P05a | Element Methods Verification | — | 0 | deepthinker |
| P06 | Display List, Pool & Callback Registry | `DisplayList` struct, pool alloc/free/push_back/insert_before/remove/count/iter, `GenerationalHandle`, `CallbackRegistry` type definitions with full lifecycle (alloc increments generation, register on spawn, unregister on DISAPPEARING removal, reinit clears registry). Display primitive coupling documented (Phase 1: C does both allocations). **TDD:** RED: test alloc returns valid handle, 151st alloc returns NULL_HANDLE (pool exhaustion at capacity 150). RED: test alloc→free→alloc reuses slot with incremented generation. RED: test push_back/remove maintains count and iteration order. GREEN: implement pool with linked-list free chain. REFACTOR: extract handle validation helpers. | ~600 | rustcoder |
| P06a | Display List Verification | — | 0 | deepthinker |
| P07 | Velocity System | `VelocityDesc` full method suite: `get_current_components`, `get_next_components`, `set_vector`, `set_components`, `delta_components`, `zero`, `is_zero`; exact C byte-order `incr` encoding. **TDD:** RED: test `get_current_components` returns bit-identical values to C for 20 known input vectors (including negative velocities and edge cases from the byte-order bug fix). RED: test `set_vector` round-trips through `get_current_components` for all 16 facings at 3 magnitudes. GREEN: implement Bresenham accumulation. REFACTOR: extract common incr encoding/decoding helpers. | ~400 | rustcoder |
| P07a | Velocity Verification | Bit-identical verification against C test vectors | 0 | deepthinker |
| P08 | Collision System | `elastic_collide()` implementing mass-based elastic response; collision eligibility helpers; gravity mass exemption; player ship penalty; DEFY_PHYSICS handling; minimum velocity enforcement. **TDD:** RED: test head-on collision between equal-mass ships produces symmetric velocity reversal. RED: test scraping collision (within one quadrant) fudges directness to half-circle. RED: test gravity-mass object (mass≥100) is immovable. RED: test zero-velocity stuck overlap sets DEFY_PHYSICS on both. RED: test player ship penalty clears max-speed flags and adds wait counters. GREEN: implement elastic response. REFACTOR: extract impact angle and momentum transfer helpers. | ~400 | rustcoder |
| P08a | Collision Verification | — | 0 | deepthinker |
| P09 | Weapon System | `LaserBlock` type definition with exact initialization fields per requirements: (1) allocate element with LINE_PRIM, (2) set life=1 (single frame), (3) compute start position from ship location + offset along firing facing, (4) set velocity = endpoint − startpoint displacement, (5) register weapon collision callback. `MissileBlock` type definition with exact initialization fields per requirements: (1) allocate element with STAMP_PRIM, (2) set configurable hp/damage/life_span/speed and optional per-frame preprocess, (3) compute spawn position from ship location + offset along firing facing, (4) set velocity from speed + facing via trig decomposition, (5) back up initial position by one velocity step (so missile doesn't visually start one frame ahead). **Phase 1 provides the TYPE DEFINITIONS for `LaserBlock`/`MissileBlock` and the `WeaponElement→LaserBlock/MissileBlock` conversion helpers, but the actual `initialize_laser()`/`initialize_missile()` orchestration functions (which allocate elements and set all fields) stay in C until Phase 2+.** `weapon_collision()` leaf function. Blast effect creation with 8 directional bins (16÷2 with rounding), standard (≤16 frames, shared array) vs custom (>16 frames, weapon farray + animation preprocess). `do_damage()` helper. `track_ship()` homing logic. Damage silhouette type definitions (rejection-sampling API). `DrawImageFlags`. **Note: P09 covers laser blocks, missile blocks, weapon collision, blast effects, homing/tracking, damage silhouette, and conversion helpers — this is a large phase (~500 LoC). If implementation reveals the scope is too broad, it may be split into P09-weapons (LaserBlock/MissileBlock/collision/blast) and P09-tracking (track_ship/damage silhouette) at the implementer's discretion.** **TDD:** RED: test `LaserBlock` construction produces correct field values for 5 known ship+offset+facing combinations. RED: test `MissileBlock` back-up-by-one-step adjusts position correctly. RED: test `weapon_collision()` damage, blast direction (8-bin quantization), standard vs custom path selection. RED: test `track_ship()` returns correct facing adjustment for 10 target positions including 180° case. GREEN: implement. REFACTOR: extract shared weapon geometry helpers. | ~500 | rustcoder |
| P09a | Weapon Verification | — | 0 | deepthinker |
| P10 | Process Loop Types & Contracts (types only — orchestration is Phase 2) | `ViewState`, `ZoomMode` enums; zoom calculation constants + hysteresis thresholds; camera types; PreProcess per-element semantics documentation; PostProcessQueue newly-added cascading documentation; scroll offset types; world-to-screen conversion constants; flag transition helpers verified against process loop requirements. **No behavioral orchestration** — PreProcessQueue/PostProcessQueue/ProcessCollisions execution stays in C. **TDD:** RED: test zoom threshold constants match C values exactly. RED: test world-to-screen conversion for discrete mode at all 3 reduction levels. RED: test hysteresis thresholds are correctly ordered (zoom-in < zoom-out). GREEN: implement type definitions. REFACTOR: consolidate related constants. | ~350 | rustcoder |
| P10a | Process Loop Types Verification | — | 0 | deepthinker |
| P11 | Battle Lifecycle Types & Contracts (types only — orchestration is Phase 3) | `BattleState` `#[repr(C)]` type (InputFunc at offset 0); battle entry sequencing types; frame callback types; per-frame processing stage enum; frame timing constants; input processing types (BATTLE_INPUT_STATE mapping); battle teardown sequence types; shared asset reference counting types; max-speed mode types. InitShips return type as `i16` (negative = hyperspace exit). **No behavioral orchestration** — Battle()/DoBattle()/ProcessInput()/InitShips()/UninitShips() stay in C. **TDD:** RED: test `BattleState` size matches C `BATTLE_STATE`, InputFunc field is at offset 0. RED: test `BATTLE_FRAME_RATE == 24`. RED: test InitShips return type preserves negative values (i16 round-trip through FFI). GREEN: implement type definitions. REFACTOR: group lifecycle stage enums. | ~350 | rustcoder |
| P11a | Battle Lifecycle Verification | BattleState layout matches C BATTLE_STATE (compile-time assertions) | 0 | deepthinker |
| P12 | Ship Runtime Within Battle Types (types only — orchestration is Phase 2/3) | Ship spawn position types (random avoiding gravity wells, Sa-Matra center); ship per-frame pipeline exact order contract (input → APPEARING → energy → preprocess → turn → thrust → status); inertial movement model constants (inertialess mode, gravity well override, at-max-speed turning); ship collision constants (quarter-HP gravity well damage); weapon firing types (postprocess order, energy deduction, weapon array); crew/energy management constants; element reuse types. **No behavioral orchestration** — ship_preprocess()/ship_postprocess()/spawn stay in C. Phase 1 provides inertial thrust math (already in ships/runtime.rs), energy math, velocity operations. **TDD:** RED: test `MAX_CREW_SIZE == 42`, `MAX_ENERGY_SIZE == 42`, gravity-well max velocity == 2304. RED: test pipeline stage enum has exactly 7 stages in correct order. GREEN: implement type definitions and constants. REFACTOR: align constant naming with ships/runtime.rs. | ~300 | rustcoder |
| P12a | Ship Runtime Verification | — | 0 | deepthinker |
| P13 | Tactical Transition Types & Constants (types only — orchestration is Phase 2/3) | Death pipeline phase enum (ship_death → explosion → cleanup → new_ship); explosion constants (36 frames, debris 1-3/frame, frame 15 hide, frame 25 clear); cleanup types (crew pickup CREW_OBJECT preservation, ownership clearing); new ship readiness types (ditty wait, netplay sync); ship replacement selection order types (SuperMelee picker, encounter queue, infinite fleet recycling); winner determination types (display-list-order, break-first, mutual destruction); Pkunk reincarnation constant (MAX_SHIP_MASS + 1); OpponentAlive semantics types (3 return cases); ship death recording types. Ion trail: 12-color fade palette constant array, POINT_PRIM, head-of-list insertion, marked PRE_PROCESS with life span pre-decremented. Warp transition: HYPERJUMP_LIFE=15, ghost image per frame along facing vector, ion trail color cycle, materialization steps. Flee: eligibility conditions, initiation constants, 20-color pulse palette, timing types. **No behavioral orchestration** — ship_death()/explosion_preprocess()/cleanup/new_ship/flee_preprocess()/ship_transition() all stay in C. **TDD:** RED: test `EXPLOSION_LIFE == 36`, `HYPERJUMP_LIFE == 15`, ion trail palette has exactly 12 entries, flee pulse palette has exactly 20 entries. RED: test death pipeline enum has 4 phases in correct order. RED: test Pkunk reincarnation mass == `MAX_SHIP_MASS + 1`. GREEN: implement type definitions and constant arrays. REFACTOR: group related constants by transition type. | ~450 | rustcoder |
| P13a | Tactical Transition Verification | — | 0 | deepthinker |
| P14 | AI Dispatch Types & Constants (types only — orchestration is Phase 2/3) | Computer intelligence entry point types (RPG overlay merge, Sa-Matra disabled, PSYTRON random picker); `EvaluateDesc` type; AI range constants (CLOSE_RANGE_WEAPON, LONG_RANGE_WEAPON); maneuverability indices (FAST_SHIP, MEDIUM_SHIP, SLOW_SHIP); object tracking indices (ENEMY_SHIP_INDEX through FIRST_EMPTY_INDEX); control flags (HUMAN/CYBORG/PSYTRON/NETWORK/ratings). **No behavioral orchestration** — computer_intelligence() stays in C. **TDD:** RED: test `CLOSE_RANGE_WEAPON == 200`, `LONG_RANGE_WEAPON == 4000`, `FAST_SHIP == 150`. RED: test tracking indices are contiguous and FIRST_EMPTY_INDEX follows last named index. GREEN: implement type definitions and constants. REFACTOR: derive Debug/Clone on EvaluateDesc. | ~200 | rustcoder |
| P14a | AI Dispatch Verification | — | 0 | deepthinker |
| P15 | Netplay Integration Types & CRC | `CrcState` and `crc_process_element()` (streaming CRC-32, bit-identical to C); CRC-32 table; input buffering hook types (configurable delay, push/pop per side); frame synchronization types (CRC verification interval, mismatch abort); battle-end synchronization protocol phase enum (in-battle → ending-battle → ending-battle-phase-2 → inter-battle); determinism contract documentation. **TDD:** RED: test `crc_process_element` produces exact CRC for 5 hand-constructed elements with known C output (including BACKGROUND_OBJECT skip, negative velocity fields, all 35 bytes in exact order). RED: test CRC table[0..5] matches C CRC-32 lookup table values. GREEN: implement streaming CRC. REFACTOR: extract LE serialization helpers. | ~400 | rustcoder |
| P15a | Netplay Verification | CRC bit-identical verification against C test vectors | 0 | deepthinker |
| P16 | Integration Point Contracts | `integration.rs` defining typed contracts with **per-operation Phase 1 vs Phase 2+ bucketing** (see Integration Operation Inventory below). Graphics (17 operations): Phase 1 needs FFI declarations for `DrawablesIntersect` (used by collision P08), primitive type constants (used by P04/P06), and frame rectangle queries (used by weapon blast P09); remaining 14 graphics ops are Phase 2+ (C owns all draw calls). Audio (11 operations): Phase 1 needs FFI declaration for damage sound dispatch (used by `weapon_collision` P09); remaining 10 audio ops are Phase 2+ (C owns sound positioning, music, flush). Threading (3 operations): all Phase 2+ (C owns frame loop). Input (4 operations): all Phase 2+ (C owns input dispatch). Resource (5 operations): all Phase 2+ (C owns asset lifecycle). Ship/race (6 operations): Phase 1 needs `ShipBehavior` trait reference for weapon callback dispatch (P09/P17); remaining 5 are Phase 2+. Global state (4 operations): Phase 1 needs `TFB_Random` FFI declaration (used by CRC P15, tracking P09); remaining 3 are Phase 2+. `c_bridge.rs` Rust-to-C call declarations for all Phase 1-needed operations. **TDD:** write type construction and trait bound tests first, then implement contract types. | ~400 | rustcoder |
| P16a | Integration Contracts Verification | — | 0 | deepthinker |
| P17 | FFI Layer & C-Side Bridge | All `extern "C"` FFI exports (`rust_velocity_*`, `rust_battle_collide`, `rust_battle_weapon_collision`, `rust_battle_track_ship`, `rust_battle_crc_process_element`); `USE_RUST_BATTLE` C-side guards in `velocity.c`, `collide.c`, `weapon.c`; build config toggle; FFI adapter functions matching spec §14.3 signatures exactly (including `rust_velocity_get_current(vel, dx, dy)`, `rust_battle_collide(e0, e1)`, `rust_battle_weapon_collision(weapon, w_pt, target, t_pt) -> HELEMENT`, `rust_battle_track_ship(tracker, pfacing) -> SIZE`); InitShips return type adapter preserving negative values (i16, not u16); `CElement = Element` migration in `ships/ffi_contract.rs`; `WeaponElement→MISSILE_BLOCK/LASER_BLOCK` bridge adapter in `rust_ships_init_weapon`. **TDD:** RED: test all 10 FFI symbols resolve at link time (symbol presence check). RED: test `rust_velocity_get_current` with null pointer returns error (null-pointer rejection). RED: test InitShips return value round-trips negative values through i16 FFI (−1, −127). GREEN: implement FFI exports and C-side `#ifndef` guards. REFACTOR: consolidate null-pointer checks into a shared FFI guard macro. | ~400 Rust + ~200 C | rustporter |
| P17a | FFI Verification | Symbol resolution, mixed C/Rust compilation, negative return value roundtrip | 0 | deepthinker |
| P18 | End-to-End Integration | Full build with `USE_RUST_BATTLE=1` + `USE_RUST_SHIPS=1`. **E2E scope for Phase 1:** the game compiles with both toggles enabled; C calls Rust leaf functions successfully via FFI. **Specific verification:** (1) game boots to main menu, (2) enters SuperMelee, (3) ships fire weapons and collide during a battle, (4) Rust velocity functions (`rust_velocity_*`) produce correct ship movement, (5) Rust collision (`rust_battle_collide`) produces correct bounce behavior, (6) Rust weapon collision (`rust_battle_weapon_collision`) correctly applies damage and creates blasts, (7) Rust CRC (`rust_battle_crc_process_element`) produces matching checksums. The test is: a SuperMelee battle runs to completion with one ship destroyed, and all Rust leaf functions return bit-identical results to the C baseline. **TDD:** write E2E smoke test script first, then verify all paths. | ~100 | rustcoder |
| P18a | End-to-End Verification | Final verification: all cargo gates pass, SuperMelee boot+run+complete with ship destruction, behavioral parity confirmed for all 10 FFI-exported leaf functions | 0 | deepthinker |

Total estimated new/modified LoC: ~5,150 (Rust) + ~200 (C) = **~5,350 total**

## Execution Order

```text
P00.5 → P01 → P01a → P02 → P02a
      → P03 → P03a → P04 → P04a
      → P05 → P05a → P06 → P06a
      → P07 → P07a → P08 → P08a
      → P09 → P09a → P10 → P10a
      → P11 → P11a → P12 → P12a
      → P13 → P13a → P14 → P14a
      → P15 → P15a → P16 → P16a
      → P17 → P17a → P18 → P18a
```

Each phase MUST be completed and verified before the next begins. No skipping.

## TDD Discipline Per Phase

Per PLAN.md and PLAN-TEMPLATE.md, every implementation phase follows the stub→TDD→impl pattern:

1. **Stub:** Create compile-safe skeletons with `todo!()` where needed. Wire module structure.
2. **TDD:** Write behavior-driven tests for the phase's deliverables. Tests should fail against stubs.
3. **Impl:** Implement to satisfy tests and pseudocode. Remove all stubs. Pass all cargo gates.

Each phase description above includes a **TDD** note specifying what tests to write first. Verification phases confirm both structural correctness (cargo gates) and semantic correctness (behavior tests actually test behavior, not just internals).

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 0.5)
2. Integration points are explicitly listed
3. TDD cycle is defined per slice (see per-phase TDD notes above)
4. Lint/test/coverage gates are declared (see Definition of Done below)
5. `Element` struct field ordering MUST match C `element.h` exactly — compile-time assertions via `core::mem::size_of` and `memoffset::offset_of` are mandatory (P04a)
6. All velocity operations MUST produce bit-identical results to C for netplay determinism
7. The `VelocityState.incr` byte-order bug in `ships/runtime.rs` has been fixed (prerequisite complete)
8. `ProcessCollisions` stays in C for Phase 1 — do NOT attempt to port it
9. The `USE_RUST_BATTLE` toggle is independent of `USE_RUST_SHIPS` — both can be enabled simultaneously
10. No `unwrap()`/`expect()` in production paths; `catch_unwind` at FFI boundaries
11. `unsafe` blocks must be minimized and documented with safety comments
12. Union field access requires `unsafe` — safe accessor methods on `Element` are mandatory
13. FFI adapter functions must preserve negative return values (i16, not u16) for InitShips

## Definition of Done

### Structural verification (from RULES.md) — mandatory cargo gates
1. `cargo fmt --all --check` passes
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
3. `cargo test --workspace --all-features` passes
4. No `TODO`/`FIXME`/`HACK`/placeholder markers in implementation code (per anti-placeholder rule)
5. No `unwrap()`/`expect()` in production paths

### Semantic verification (from PLAN-TEMPLATE.md)
6. Feature behavior is present and reachable from real app flow
7. Tests verify behavior, not only internals
8. Error handling behavior matches requirements
9. No placeholder/deferred implementation patterns remain
10. Integration points validated end-to-end

### ABI verification
11. Mixed C/Rust build succeeds with both `USE_RUST_BATTLE` and `USE_RUST_SHIPS` enabled
12. `Element` struct size and field offsets match C `ELEMENT` (compile-time assertions)
13. `BattleState` struct size and field offsets match C `BATTLE_STATE` (compile-time assertions)
14. All 14 element state flag bit positions match C `element.h` exactly
15. Every FFI function validates non-null pointers before dereferencing
16. FFI functions returning `SIZE` (i16) preserve negative values without reinterpretation

### Behavioral verification
17. SuperMelee battle boots and runs to completion with `USE_RUST_BATTLE=1`
18. Velocity computation produces bit-identical results to C (verified by test vectors)
19. Elastic collision response produces bit-identical results to C (verified by test vectors)
20. Weapon collision behavior matches C (damage, blast creation, tracking)
21. Netplay CRC per-element produces bit-identical results to C (verified by test vectors)

### Migration verification
22. `VelocityState.incr` byte-order bug no longer present in `ships/runtime.rs` (prerequisite already fixed)
23. `ships/runtime.rs` re-exports from `battle_types` — existing race files require zero changes
24. Display list pool operations handle exhaustion gracefully (return `NULL_HANDLE`)
25. `CElement` in `ships/ffi_contract.rs` aliased to `battle::Element` (shared layout)
26. **No regression in ships subsystem tests after type relocation** — all ~47 existing tests in `ships/runtime.rs` pass after P03 extraction

### Traceability verification
27. Every requirement in `requirements.md` maps to at least one phase (see Requirements Traceability Matrix)
28. Every phase references the requirement IDs it implements

## Open Design Decisions from Specification §18

The specification notes 8 open design decisions (§18.1–§18.8). The analysis phase (P01) must explicitly enumerate each, document the resolution, and trace it to the implementation phase. This plan's preliminary approach for each:

1. **§18.1 Union field layout verification** — Phase 1 uses compile-time `size_of`/`offset_of` assertions (P04a) to verify ABI compatibility. If unions introduce unexpected padding, the assertion fails and the layout is adjusted before proceeding. **Assigned: P04a (deepthinker).**

2. **§18.2 Callback function pointer ABI compatibility** — Verify that `Option<unsafe extern "C" fn(...)>` is ABI-compatible with a nullable C function pointer. This is guaranteed by the Rust reference but should be verified with a cross-compilation test. **Assigned: P04a (deepthinker).**

3. **§18.3 `p_parent` void pointer semantics** — `Element.p_parent` stays as `*mut core::ffi::c_void` in Phase 1. C code accesses it via `GetElementStarShip`/`SetElementStarShip` macros. Rust code uses `c_bridge.rs` helpers. **Assigned: P16 (integration contracts).**

4. **§18.4 Frame and drawable handles** — `Frame` is `*mut c_void` (opaque handle to C's `FRAME_DESC*`). Rust passes through without interpretation. **Assigned: P04 (core types).**

5. **§18.5 Display primitive array ownership timeline** — `DisplayArray[330]` stays C-owned in all phases. Primitive management may move to graphics module in Phase 2+. Phase 1 documents the coupling: element allocation must be paired with primitive allocation (C does both in Phase 1). **Assigned: P06 (display list) for documentation, P16 (integration contracts) for Phase 2+ planning.**

6. **§18.6 `DrawablesIntersect` replacement** — Phase 1 calls C's `DrawablesIntersect()` via FFI (declared in `c_bridge.rs`). No Rust reimplementation. **Assigned: P16 (integration contracts).**

7. **§18.7 Existing ships/runtime.rs migration timing** — Step A (extract constants) in P03, Step B (add VelocityDesc, ElementFlags) in P04/P07. Race files require zero changes. **Assigned: P03 (rustcoder).**

8. **§18.8 VelocityState.incr byte order** — Bug has been fixed as a prerequisite. `VelocityDesc` uses correct C byte order from the start. `VelocityState ↔ VelocityDesc` conversion implemented in P07. **Assigned: P03 (already complete) + P07 (rustcoder).**

**P01 (Analysis) must explicitly revisit all 8 decisions, confirm or revise these resolutions, and document any additional constraints discovered during C code survey.**

## Leaf Operations vs C-Owned Logic (Phase 1 Clarity)

Per spec §2.1 and §2.4, the following functions are **Rust leaf operations** that Phase 1 implements:

| Rust Function | C Replacement | Phase |
|--------------|---------------|-------|
| `rust_velocity_get_current` | `GetCurrentVelocityComponents` | P07, P17 |
| `rust_velocity_get_next` | `GetNextVelocityComponents` | P07, P17 |
| `rust_velocity_set_vector` | `SetVelocityVector` | P07, P17 |
| `rust_velocity_set_components` | `SetVelocityComponents` | P07, P17 |
| `rust_velocity_delta_components` | `DeltaVelocityComponents` | P07, P17 |
| `rust_velocity_zero` | `ZeroVelocityComponents` | P07, P17 |
| `rust_battle_collide` | `collide()` | P08, P17 |
| `rust_battle_weapon_collision` | `weapon_collision()` | P09, P17 |
| `rust_battle_track_ship` | `TrackShip()` | P09, P17 |
| `rust_battle_crc_process_element` | `crc_processELEMENT()` | P15, P17 |

The following **stay entirely in C** for Phase 1:

| C Function | Reason | Future Phase |
|-----------|--------|-------------|
| `Battle()` | Battle loop entry, frame timing | Phase 3 |
| `DoBattle()` | Per-frame callback | Phase 3 |
| `ProcessInput()` | Input dispatch | Phase 3 |
| `RedrawQueue()` | Top-level frame dispatch | Phase 2 |
| `PreProcessQueue()` | Element iteration + collision orchestration | Phase 2 |
| `PostProcessQueue()` | Render list + dead element removal | Phase 2 |
| `ProcessCollisions()` | Recursive collision orchestration | Phase 2 |
| `PreProcess()` | Per-element preprocessing | Phase 2 |
| `InitShips()` / `UninitShips()` | Battle lifecycle | Phase 3 |
| `ship_death()` / explosion / cleanup / new_ship | Death pipeline | Phase 2/3 |
| `flee_preprocess()` / `ship_transition()` | Flee/warp | Phase 2/3 |
| `find_alive_starship()` | Winner determination | Phase 2/3 |
| `computer_intelligence()` | AI dispatch | Phase 2/3 |
| `crc_processState()` / `crc_processDispQueue()` | Display list iteration for CRC | Phase 2 |

## Type Relocation Plan (spec §3.5)

The following types and functions currently in `ships/runtime.rs` must be relocated to `battle_types/` in P03:

**Step A (P03, atomic commit):**
1. Create `rust/src/battle_types/mod.rs`, `coords.rs`, `trig.rs`, `angles.rs`
2. Move constants: `VELOCITY_SHIFT`, `ONE_SHIFT`, `FACING_SHIFT`, `NUM_FACINGS`, `CIRCLE_SHIFT`, `FULL_CIRCLE`, `HALF_CIRCLE`, `QUADRANT`, `OCTANT`
3. Move functions: `sine()`, `cosine()`, `arctan()`, `SINE_TABLE`, `normalize_facing()`, `facing_to_angle()`, `angle_to_facing()`, `normalize_angle()`, `display_to_world()`, `world_to_velocity()`, `velocity_to_world()`, `gravity_mass()`
4. Move element flag constants: `NORMAL_LIFE`, `MAX_SHIP_MASS`, `GRAVITY_THRESHOLD`, `PLAYER_SHIP`, `APPEARING`, `DISAPPEARING`, `CHANGING`, `COLLISION_FLAG`, `IGNORE_SIMILAR`, `FINITE_LIFE`
5. Add toroidal wrapping functions: `wrap_x()`, `wrap_y()`, `shortest_path_delta()`
6. Update `ships/runtime.rs` to `pub use crate::battle_types::*` — zero changes to race files
7. VelocityState.incr byte-order bug already fixed (prerequisite complete)

**Step B (P04/P07, incremental):**
1. Add `VelocityDesc` (`#[repr(C)]`) to `battle_types/`
2. Add `ElementFlags` bitflags type
3. Add `VelocityState ↔ VelocityDesc` `From`/`Into` conversions

## Cross-Reference: Ships Plan Dependencies

The ships plan (PLAN-20260314-SHIPS) has deferred items that depend on battle types:

- **Ships P14-WIRE (C-Side Bridge Wiring)** stubs depend on battle types like `Element`, `LaserBlock`, `MissileBlock` being defined. Those types are created in battle P04 and P09. The ships P14 phase should not execute until battle P09 is complete.
- **Ships `CElement`** in `ffi_contract.rs` is currently an opaque zero-sized struct. Battle P17 migrates it to `pub use crate::battle::element::Element as CElement`, giving ships bridge code direct field access.
- **`rust_ships_init_weapon`** bridge function needs `LaserBlock`/`MissileBlock` from battle P09 and the FFI adapter from battle P17.

These cross-plan dependencies should be tracked in the execution tracker.

## Deferred Items

The following are explicitly out of scope for this plan. Phase 1 defines types, constants, and contracts for all of these areas (providing a verified type foundation), but the behavioral orchestration stays in C.

### Process loop orchestration (Phase 2)

- **PreProcessQueue/PostProcessQueue execution** (`PreProcessQueue`, `PostProcessQueue` in `process.c`): The iteration loops, per-element dispatch, and render list construction. Types and contracts defined in P10.
- **ProcessCollisions orchestration** (`ProcessCollisions` in `process.c`): Display list walk, pair-wise dispatch, recursive earlier-time checks, collision-point snapping, stuck overlap handling, post-bounce rechecks. Too entangled with the process loop for Phase 1. Rust provides eligibility checks and elastic response math via FFI; C owns the orchestration.
- **Asymmetric DEFY_PHYSICS clearing**: The PostProcessQueue logic that clears COLLISION but retains DEFY_PHYSICS when COLLISION is set, or clears DEFY_PHYSICS when COLLISION is not set. Flag transition helpers defined on `Element` in P05/P10.
- **Newly-added element cascading**: The inner loop in PostProcessQueue for elements lacking PRE_PROCESS, including tail-chasing for spawned elements and full-list collision detection. Types defined in P10.
- **Scroll offset application**: PRE_PROCESS/POST_PROCESS flag-dependent scroll delta application. Types defined in P10.
- **Scroll/transform/render insertion timing**: Coordinate transform, zoom-frame selection, postprocess callback dispatch, primitive insertion into render list.
- **Display list ownership transfer**: Phase 1 defines `DisplayList` types but C owns `disp_q`.
- **Display primitive array management**: Remains C-owned in all phases. Battle engine sets `prim_index` on elements; C manages `DisplayArray[]`.
- **Rendering-order linked list**: The separate display-primitive linked list ordered by display position for visual layering, maintained by `PostProcessQueue()`. Phase 1 documents the type; C owns it entirely.

### Camera/zoom computation (Phase 2)

- **Zoom calculation** (`CalcReduction`): Step/continuous modes, hysteresis to prevent oscillation between adjacent zoom levels. Types defined in P10.
- **Zoom hysteresis**: Different thresholds for zoom-in vs zoom-out transitions.
- **Camera calculation** (`CalcView`): Midpoint between ships, scroll clamping, view state transitions.
- **Camera single-ship clamping**: Max per-frame jump distance when only one ship is active.
- **World-to-screen conversion** (`CalcDisplayCoord`): Step/continuous formulas with zoom. Conversion constants defined in P03/P10.

### Battle lifecycle orchestration (Phase 3)

- **Battle loop porting** (`Battle()`, `DoBattle()`, `ProcessInput()` in `battle.c`): Battle entry sequencing, per-frame callback, input processing loop. Types defined in P11.
- **Battle entry sequencing**: RNG seed, music load, InitShips, ship count validation, activity flag, graphics scale, input order, ship spawn, music start — the entire `Battle()` orchestration.
- **Input processing loop**: Side iteration, bit mapping to ship status flags, escape detection.
- **Battle teardown sequencing** (`UninitShips()` in `init.c`): Stop audio, free assets, count crew, writeback, clear activity flag.
- **Shared asset reference counting**: Nested init/deinit for space assets (star field, explosions, blasts, asteroids).
- **Frame timing**: 24 fps sleep, max-speed skip logic.

### Tactical transition orchestration (Phase 2/3)

- **Ship death 4-phase pipeline** (`ship_death` → `explosion_preprocess` → `cleanup_dead_ship` → `new_ship` in `tactrans.c`): The callback-replacement state machine that drives the death sequence. Types and constants defined in P13.
- **Explosion animation**: 36-frame sequence with 1-3 debris particles/frame, frame 15 hide primitive, frame 25 clear preprocess. Constants defined in P13.
- **Cleanup crew-pickup preservation**: Iterating elements to clear ownership, marking deletion, preserving CREW_OBJECT elements.
- **New-ship readiness wait**: Waiting for ditty playback to finish and netplay sync to complete before ship replacement.
- **Ship replacement selection order**: SuperMelee picker, encounter queue traversal, infinite fleet recycling.
- **Winner determination iteration**: Display-list-order traversal, PLAYER_SHIP check, break-first, mutual destruction detection, Pkunk reincarnation (mass == MAX_SHIP_MASS + 1).
- **OpponentAlive semantics**: Display list iteration with crew check, 3 return cases.
- **Flee sequence orchestration**: Eligibility check (5 conditions), initiation (mass/color/timing setup), animation (20-color pulse with accelerating timing), warp-out trigger.
- **Warp transition orchestration** (`ship_transition` in `tactrans.c`): 15-frame ghost image spawning along facing vector, materialization steps (show prim, select zoom frame, init intersection, zero velocity, clear NONSOLID/FINITE_LIFE, restore callbacks).
- **Warp ghost spawning**: Per-frame ghost image positioning, ion trail color cycle application.

### Ship runtime pipeline orchestration (Phase 2/3)

- **Ship per-frame pipeline order** (`ship_preprocess` in `ship.c`): The exact sequence input → APPEARING → energy → preprocess → turn → thrust → status. Phase 1 provides the math (inertial thrust, velocity operations, energy constants) but not the dispatch order.
- **Ship spawn placement** (`spawn_ship` in `ship.c`/`init.c`): Random position avoiding gravity wells, Sa-Matra center placement.
- **Ship first-frame initialization**: Input suppression during APPEARING, crew display init, race preprocess invocation, warp-in start.
- **Energy regeneration dispatch**: Counter-based energy regeneration timing.
- **Turn/thrust processing dispatch**: Cooldown-based turn and thrust application.
- **Weapon firing pipeline** (`ship_postprocess` in `ship.c`): Cooldown check, energy deduction, weapon callback dispatch, element binding.
- **Ship spawn porting**: Already covered by `USE_RUST_SHIPS` in the ships subsystem plan. Types defined in P12.

### AI dispatch orchestration (Phase 2/3)

- **Computer intelligence entry point** (`computer_intelligence` in `intel.c`): RPG overlay merge, Sa-Matra disabled AI, PSYTRON random picker. Types and constants defined in P14.
- **AI behavioral dispatch**: Calling race-specific `intelligence()` callbacks through the dispatch entry point.
- **Object tracking system orchestration**: Concern-type indexed tracking array management.

### Netplay orchestration (Phase 2/3)

- **Input buffering**: Configurable delay, push/pop per side for netplay input synchronization. Hook types defined in P15.
- **Frame synchronization**: CRC verification at configurable intervals, mismatch → abort. Phase 1 provides `crc_process_element()` but not the verification loop.
- **Battle-end multi-phase protocol**: in-battle → ending-battle → ending-battle-phase-2 → inter-battle synchronization state machine.
- **Netplay transport**: Out of scope entirely — belongs to a separate netplay subsystem.

### Teardown/double-buffer robustness (Phase 2/3)

- **Teardown robustness**: Handling ships never fully spawned, absent teardown hooks, already-freed descriptors, queue entries with no associated descriptor. Phase 1 ensures types are correct and exhaustion-safe.
- **Double-buffer invariant enforcement**: Ensuring current/next consistency is maintained across frames by the process loop.

### Other deferred items

- **Advanced collision features**: HQxx scalers, advanced frame operations — graphics subsystem concerns.
- **InitShips return type mismatch fix**: The existing `rust_ships_init()` returns `COUNT` (u16) but `InitShips()` returns `SIZE` (i16). The FFI adapter in P17 uses the correct `i16` type. A coordinated fix for the ships-side declaration should be tracked but is not blocking for this plan.

## Plan Files

```text
plan/
  00-overview.md                                  (this file)
  00a-preflight-verification.md                   P00.5
  01-analysis.md                                  P01
  01a-analysis-verification.md                    P01a
  02-pseudocode.md                                P02
  02a-pseudocode-verification.md                  P02a
  03-shared-foundation.md                         P03
  03a-shared-foundation-verification.md           P03a
  04-core-types-constants.md                      P04
  04a-core-types-constants-verification.md        P04a
  05-element-methods-lifecycle.md                  P05
  05a-element-methods-lifecycle-verification.md    P05a
  06-display-list-pool-registry.md                 P06
  06a-display-list-pool-registry-verification.md   P06a
  07-velocity-system.md                            P07
  07a-velocity-system-verification.md              P07a
  08-collision-system.md                           P08
  08a-collision-system-verification.md             P08a
  09-weapon-system.md                              P09
  09a-weapon-system-verification.md                P09a
  10-process-loop-types.md                         P10
  10a-process-loop-types-verification.md           P10a
  11-battle-lifecycle-types.md                     P11
  11a-battle-lifecycle-types-verification.md       P11a
  12-ship-runtime-types.md                         P12
  12a-ship-runtime-types-verification.md           P12a
  13-tactical-transition-types.md                  P13
  13a-tactical-transition-types-verification.md    P13a
  14-ai-dispatch-types.md                          P14
  14a-ai-dispatch-types-verification.md            P14a
  15-netplay-integration.md                        P15
  15a-netplay-integration-verification.md          P15a
  16-integration-contracts.md                      P16
  16a-integration-contracts-verification.md        P16a
  17-ffi-c-bridge.md                               P17
  17a-ffi-c-bridge-verification.md                 P17a
  18-e2e-integration.md                            P18
  18a-e2e-integration-verification.md              P18a
  execution-tracker.md
```

## Execution Tracker

The execution tracker for this plan is maintained at `plan/execution-tracker.md`. It follows the format defined in `dev-docs/PLAN-TEMPLATE.md`.

| Phase | Title | Status | Verified | Semantic Verified | Notes |
|------:|-------|--------|----------|-------------------|-------|
| P00.5 | Preflight Verification | ⬜ | ⬜ | N/A | Includes VelocityState byte-order fix verification |
| P01 | Analysis | ⬜ | ⬜ | ⬜ | |
| P01a | Analysis Verification | ⬜ | ⬜ | ⬜ | |
| P02 | Pseudocode | ⬜ | ⬜ | ⬜ | 5 algorithms |
| P02a | Pseudocode Verification | ⬜ | ⬜ | ⬜ | |
| P03 | Shared Foundation | ⬜ | ⬜ | ⬜ | ~400 LoC |
| P03a | Shared Foundation Verification | ⬜ | ⬜ | ⬜ | Must verify 47 ships tests pass |
| P04 | Core Types & Constants | ⬜ | ⬜ | ⬜ | ~600 LoC |
| P04a | Core Types Verification | ⬜ | ⬜ | ⬜ | |
| P05 | Element Methods & Lifecycle | ⬜ | ⬜ | ⬜ | ~300 LoC |
| P05a | Element Methods Verification | ⬜ | ⬜ | ⬜ | |
| P06 | Display List & Pool | ⬜ | ⬜ | ⬜ | ~600 LoC |
| P06a | Display List Verification | ⬜ | ⬜ | ⬜ | |
| P07 | Velocity System | ⬜ | ⬜ | ⬜ | ~400 LoC |
| P07a | Velocity Verification | ⬜ | ⬜ | ⬜ | Bit-identical to C |
| P08 | Collision System | ⬜ | ⬜ | ⬜ | ~400 LoC |
| P08a | Collision Verification | ⬜ | ⬜ | ⬜ | |
| P09 | Weapon System | ⬜ | ⬜ | ⬜ | ~500 LoC; may split |
| P09a | Weapon Verification | ⬜ | ⬜ | ⬜ | |
| P10 | Process Loop Types | ⬜ | ⬜ | ⬜ | ~350 LoC |
| P10a | Process Loop Types Verification | ⬜ | ⬜ | ⬜ | |
| P11 | Battle Lifecycle Types | ⬜ | ⬜ | ⬜ | ~350 LoC |
| P11a | Battle Lifecycle Verification | ⬜ | ⬜ | ⬜ | |
| P12 | Ship Runtime Types | ⬜ | ⬜ | ⬜ | ~300 LoC |
| P12a | Ship Runtime Verification | ⬜ | ⬜ | ⬜ | |
| P13 | Tactical Transition Types | ⬜ | ⬜ | ⬜ | ~450 LoC |
| P13a | Tactical Transition Verification | ⬜ | ⬜ | ⬜ | |
| P14 | AI Dispatch Types | ⬜ | ⬜ | ⬜ | ~200 LoC |
| P14a | AI Dispatch Verification | ⬜ | ⬜ | ⬜ | |
| P15 | Netplay Integration | ⬜ | ⬜ | ⬜ | ~400 LoC |
| P15a | Netplay Verification | ⬜ | ⬜ | ⬜ | CRC bit-identical |
| P16 | Integration Contracts | ⬜ | ⬜ | ⬜ | ~400 LoC |
| P16a | Integration Contracts Verification | ⬜ | ⬜ | ⬜ | |
| P17 | FFI & C Bridge | ⬜ | ⬜ | ⬜ | ~400 Rust + ~200 C |
| P17a | FFI Verification | ⬜ | ⬜ | ⬜ | |
| P18 | E2E Integration | ⬜ | ⬜ | ⬜ | ~100 LoC |
| P18a | E2E Verification | ⬜ | ⬜ | ⬜ | Final gate |

### Execution Rules

1. Phases execute in strict order: P00.5 → P01 → P01a → ... → P18 → P18a
2. Each phase MUST be completed and verified before the next begins
3. No skipping phases
4. Phase completion requires creating `project-plans/20260311/battle/.completed/PNN.md`

## Integration Operation Inventory (P16 Detail)

This section enumerates every specific integration operation from `requirements.md` §Integration points, with explicit Phase 1 vs Phase 2+ bucketing. Phase 1 provides FFI declarations in `c_bridge.rs` for operations Rust needs to call; Phase 2+ provides the remaining declarations when orchestration moves to Rust.

### Graphics Subsystem (17 operations)

| # | Operation | Phase 1 FFI? | Used By |
|---|-----------|-------------|---------|
| 1 | Display primitive array access | Phase 2+ | C owns prims |
| 2 | Primitive free list management | Phase 2+ | C owns alloc |
| 3 | Primitive type get/set | **Phase 1** | P04 (type constants), P09 (LINE_PRIM check) |
| 4 | Primitive property operations | Phase 2+ | C owns prim state |
| 5 | Batch rendering entry (DrawBatch) | Phase 2+ | C owns frame dispatch |
| 6 | Graphic scale operations | Phase 2+ | C owns zoom |
| 7 | Scale mode operations | Phase 2+ | C owns zoom |
| 8 | Drawable clear operations | Phase 2+ | C owns frame dispatch |
| 9 | Drawing context management (SetContext) | Phase 2+ | C owns contexts |
| 10 | Clip rectangle operations | Phase 2+ | C owns contexts |
| 11 | Background color / foreground frames | Phase 2+ | C owns rendering |
| 12 | Screen transition operations | Phase 2+ | C owns transitions |
| 13 | Frame index / equivalent frame queries | **Phase 1** | P09 (blast frame selection) |
| 14 | Frame rectangle queries | **Phase 1** | P09 (blast positioning) |
| 15 | Frame count queries | **Phase 1** | P09 (standard vs custom blast) |
| 16 | Pixel-accurate intersection testing (DrawablesIntersect) | **Phase 1** | P08 (`IntersectControl` type), P09 (damage silhouette) |
| 17 | Trilinear mipmap setup | Phase 2+ | C owns rendering |

### Audio Subsystem (11 operations)

| # | Operation | Phase 1 FFI? | Used By |
|---|-----------|-------------|---------|
| 1 | Positioned sound playback (PlaySound) | **Phase 1** | P09 (damage sound in `weapon_collision`) |
| 2 | Sound stopping (StopSound) | Phase 2+ | C owns lifecycle |
| 3 | Element-positioned sound processing | Phase 2+ | C owns frame dispatch |
| 4 | Music playback (PlayMusic) | Phase 2+ | C owns lifecycle |
| 5 | Music stopping (StopMusic) | Phase 2+ | C owns lifecycle |
| 6 | Stereo position calculation | Phase 2+ | C owns frame dispatch |
| 7 | Stereo position updating | Phase 2+ | C owns frame dispatch |
| 8 | Sound position removal on death | Phase 2+ | C owns lifecycle |
| 9 | Sound flushing (FlushSounds) | Phase 2+ | C owns frame dispatch |
| 10 | Music-playing status query | Phase 2+ | C owns lifecycle |
| 11 | Menu sound suppression | Phase 2+ | C owns lifecycle |

### Threading Subsystem (3 operations)

| # | Operation | Phase 1 FFI? | Used By |
|---|-----------|-------------|---------|
| 1 | Cooperative yield (TaskSwitch) | Phase 2+ | C owns frame loop |
| 2 | Timed sleep (SleepThreadUntil) | Phase 2+ | C owns frame loop |
| 3 | Cooperative input loop (DoInput) | Phase 2+ | C owns frame loop |

### Input Subsystem (4 operations)

| # | Operation | Phase 1 FFI? | Used By |
|---|-----------|-------------|---------|
| 1 | Per-player input handlers (PlayerInput) | Phase 2+ | C owns input dispatch |
| 2 | Control flags (PlayerControl) | Phase 2+ | C owns input dispatch |
| 3 | Frame-input polling (frameInput) | Phase 2+ | C owns input dispatch |
| 4 | Raw-to-battle conversion (CurrentInputToBattleInput) | Phase 2+ | C owns input dispatch |

### Resource Subsystem (5 operations)

| # | Operation | Phase 1 FFI? | Used By |
|---|-----------|-------------|---------|
| 1 | Graphic asset loading (LoadGraphic) | Phase 2+ | C owns asset lifecycle |
| 2 | Drawable capture (CaptureDrawable) | Phase 2+ | C owns asset lifecycle |
| 3 | Drawable release (ReleaseDrawable) | Phase 2+ | C owns asset lifecycle |
| 4 | Drawable destruction (DestroyDrawable) | Phase 2+ | C owns asset lifecycle |
| 5 | Music destruction (DestroyMusic) | Phase 2+ | C owns asset lifecycle |

### Ship/Race Subsystem (6 operations)

| # | Operation | Phase 1 FFI? | Used By |
|---|-----------|-------------|---------|
| 1 | Race descriptor behavioral callbacks | **Phase 1** | P09/P17 (weapon init adapter) |
| 2 | Ship queue management (per side) | Phase 2+ | C owns queues |
| 3 | Ship loading/freeing | Phase 2+ | C owns lifecycle |
| 4 | Energy management operations | Phase 2+ | C owns ship runtime |
| 5 | Status bar initialization | Phase 2+ | C owns UI |
| 6 | Status bar update | Phase 2+ | C owns UI |

### Global State (4 operations)

| # | Operation | Phase 1 FFI? | Used By |
|---|-----------|-------------|---------|
| 1 | CurrentActivity flags | Phase 2+ | C owns activity state |
| 2 | Game state variables (GET_GAME_STATE) | Phase 2+ | C owns game state |
| 3 | Pseudo-random number generator (TFB_Random) | **Phase 1** | P09 (tracking random turn), P15 (CRC includes RNG state) |
| 4 | Space type detection (hyperspace/quasispace) | Phase 2+ | C owns navigation state |

**Phase 1 total:** 8 FFI declarations needed in `c_bridge.rs` (primitive type, frame queries ×3, DrawablesIntersect, PlaySound, race callbacks, TFB_Random).
**Phase 2+ total:** 42 FFI declarations deferred until orchestration moves to Rust.

## Cross-Reference: Ships Plan P14-WIRE Dependencies

The ships plan (PLAN-20260314-SHIPS) Phase 14 (C-Side Bridge Wiring) has deferred stubs that depend on battle types:

| Ships P14 Stub | Battle Dependency | Battle Phase | Notes |
|---------------|-------------------|-------------|-------|
| `CElement` type (opaque → real) | `battle::element::Element` | P04 | Ships P14 should not execute until battle P04 is complete |
| `LaserBlock` / `MissileBlock` in weapon bridge | `battle::weapon::{LaserBlock, MissileBlock}` | P09 | Ships P14 should not execute until battle P09 is complete |
| `rust_ships_init_weapon` FFI adapter | `WeaponElement→LaserBlock/MissileBlock` conversion | P09 + P17 | Full adapter requires both battle P09 (types) and P17 (FFI bridge) |
| `CElement = Element` alias migration | `pub use crate::battle::element::Element as CElement` | P17 | Battle P17 performs the actual alias in `ffi_contract.rs` |

**Coordination rule:** Ships P14-WIRE must not begin until battle P09a verification is complete.

## Comprehensive Requirements Traceability Matrix

This matrix maps **every individual requirement bullet** from `requirements.md` to its Phase 1 implementation phase or "Phase 2+" deferral. Requirements are grouped by `requirements.md` section. Where a requirement is partially addressed in Phase 1 (types/constants defined but behavioral logic deferred), the "Phase" column indicates the Phase 1 phase that provides the types, and the "Notes" column clarifies.

**Annotations used in Phase column:**
- **P03–P17** — Phase 1 phases that implement types, constants, or leaf functions
- **Phase 2+** — Behavioral orchestration deferred; C owns the logic in Phase 1
- **P17 FFI decl** — Phase 1 provides an FFI declaration in `c_bridge.rs` (Rust-callable wrapper for a C function)
- **Already in ships/runtime.rs → P03** — Already implemented in the ships subsystem; Phase 1 P03 relocates to `battle_types/` and re-exports

### Element System — Entity Model (§ Element system → Entity model)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1 | Every physical object represented as element in unified entity model | P04 | `Element` struct definition |
| 2 | Each element carries: linked-list, callbacks, owner, flags, life span, combat stats, timing, velocity, intersection, prim index, double-buffered visual, parent, tracking target | P04 | All fields in `#[repr(C)]` struct |
| 3 | Parent ownership reference associates element with owning ship; tracking target reference for homing | P04 | `p_parent`, `h_target` fields |
| 4a | Owner identity: bottom-side player (player_nr = 0) | P04 | `player_nr` field + constant |
| 4b | Owner identity: top-side player (player_nr = 1) | P04 | `player_nr` field + constant |
| 4c | Owner identity: neutral (player_nr sentinel) | P04 | `player_nr` field + constant |
| 5 | Display primitive index linking element to primitive array; prim alloc independent from element alloc | P04, P06 | `prim_index` field; coupling documented |

### Element System — Element State Flags (§ Element system → Element state flags)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1 | Define 14 semantic flag categories | P04 | `ElementFlags` bitflags type |
| 2 | APPEARING = newly spawned, not completed first cycle | P04 | Bit position matches C |
| 3 | DISAPPEARING = marked for removal, deallocated in cleanup | P04 | Bit position matches C |
| 4 | COLLISION = already processed, skip further checks until cleared | P04 | Bit position matches C |
| 5 | NONSOLID = exclude from all collision detection | P04 | Bit position matches C |
| 6 | IGNORE_SIMILAR = prevent collision with same-parent elements | P04 | Bit position matches C |
| 7 | FINITE_LIFE = decrement life span each frame | P04 | Bit position matches C |
| 8 | BACKGROUND_OBJECT = exclude from netplay checksum | P04 | Bit position matches C |
| 9 | PLAYER_SHIP = player-controlled, special treatment in collision/camera/winner | P04 | Bit position matches C |
| 10 | CHANGING = graphical representation changed, reinit intersection | P04 | Bit position matches C |
| 11 | DEFY_PHYSICS = overlapping stationary, asymmetric clearing | P04, P05 | Flag type + transition helpers |
| 12 | PRE_PROCESS = preprocessed this frame | P04 | Bit position matches C |
| 13 | POST_PROCESS = postprocessed this frame | P04 | Bit position matches C |
| 14 | IGNORE_VELOCITY = skip velocity application | P04 | Bit position matches C |
| 15 | CREW_OBJECT = floating crew pickup, preserved during cleanup | P04 | Bit position matches C |

### Element System — Element Union Fields (§ Element system → Element union fields)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1 | Crew-level and hit-points share storage (union) | P04 | `#[repr(C)]` union type |
| 2 | Turn-wait field; thrust-wait and blast-offset share storage | P04 | `#[repr(C)]` union type |
| 3 | Color-cycle index tracks animation position | P04 | Field in Element |

### Element System — Element Union-Field Lifecycle Semantics (§ Element system → Element union-field lifecycle semantics)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1 | Ship (PLAYER_SHIP): crew-level union = crew, thrust-wait union = thrust wait | P05 | Safe accessor methods context-aware |
| 2 | Weapon (FINITE_LIFE, no PLAYER_SHIP): hit points, blast offset | P05 | Safe accessor methods context-aware |
| 3 | Ship→explosion: crew-level field undefined during explosion | P05 | Documented in accessors |
| 4 | Ship→explosion: thrust-wait union may be repurposed | P05 | Documented in accessors |

### Element System — Element Callbacks (§ Element system → Element callbacks)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1 | Four behavioral callbacks: preprocess, postprocess, collision, death | P04 | Function pointer types |
| 2 | Null/absent callback treated as no-op | P04 (type), Phase 2+ (dispatch) | `Option<fn>` naturally represents null; Phase 2+ dispatch code checks for `None` |
| 3 | Callbacks can replace themselves or other callbacks on same element during execution (multi-phase state machines) | P04 (type), Phase 2+ (dispatch) | Types support mutation; callback registry dispatch supports replacement by design; actual replacement occurs in orchestration |

### Element System — Element Lifecycle (§ Element system → Element lifecycle)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1 | Life span zero → invoke death callback | Phase 2+ | P05 types; C owns PreProcess dispatch |
| 2 | Death callback sets DISAPPEARING → remove in postprocess | Phase 2+ | P05 types; C owns PostProcess |
| 3 | Death callback extends life span → keep active | Phase 2+ | P05 types; C owns dispatch |
| 4 | Removed element → clear all tracking target references | Phase 2+ | P06 types; C owns display list |

### Element System — Element Lifecycle Flag Transitions (§ Element system → Element lifecycle flag transitions)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1 | After preprocessing: set PRE_PROCESS, clear POST_PROCESS + COLLISION | P05 | Flag transition helper method |
| 2 | After postprocessing: set POST_PROCESS, clear PRE_PROCESS + CHANGING + APPEARING | P05 | Flag transition helper method |
| 3 | PostProcess + COLLISION not set → clear DEFY_PHYSICS | P05 | Flag transition helper method |
| 4 | PostProcess + COLLISION set → clear COLLISION, retain DEFY_PHYSICS | P05 | Flag transition helper method |

### Element System — Element Constants (§ Element system → Element constants)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1 | NORMAL_LIFE = 1 | P03 | `battle_types` constant |
| 2 | MAX_CREW_SIZE = 42, MAX_ENERGY_SIZE = 42 | P04 | `constants.rs` |
| 3 | MAX_SHIP_MASS = 10; gravity-mass threshold = mass_points ≥ 100 | P03 | `battle_types` constant + `gravity_mass()` |
| 4 | GRAVITY_THRESHOLD = 255 | P03 | `battle_types` constant |

### Display List Management — Pool Allocation (§ Display list management → Pool allocation)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1 | Preallocated fixed-capacity pool with ordered traversal | P06 | `DisplayList` struct |
| 2 | Element pool capacity = 150 | P06 | `MAX_DISPLAY_ELEMENTS` constant |
| 3 | Display primitive array capacity = 330 | P06 | `MAX_DISPLAY_PRIMS` constant |
| 4 | Pool exhaustion fails without corruption | P06 | Returns NULL_HANDLE |
| 5 | Display primitive exhaustion fails without corruption | P06 | Returns NULL_HANDLE |

### Display List Management — Display List Operations (§ Display list management → Display list operations)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1 | Alloc, dealloc, append tail, insert before, remove, count, iter with callback | P06 | All operations implemented |
| 2 | Null/empty sentinel for absent element | P06 | NULL_HANDLE constant |
| 3 | Pool allocated once during engine context init | P06 | Type supports this; C controls timing |
| 4 | Reset at battle start: empty active, rebuild free chain | P06 | `reset()` method |

### Display List Management — Display Primitive Management (§ Display list management → Display primitive management)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1a | Primitive type: stamp (sprites) | P04 | Enum variant / constant |
| 1b | Primitive type: stamp-fill (colored sprites) | P04 | Enum variant / constant |
| 1c | Primitive type: line (laser beams) | P04 | Enum variant / constant |
| 1d | Primitive type: point (particles) | P04 | Enum variant / constant |
| 1e | Primitive type: no-prim (hidden) | P04 | Enum variant / constant |
| 2 | Independent free list within display primitive array | P06 | Type definitions; C owns alloc in Phase 1 |
| 3 | Element alloc → also alloc display primitive, bind via prim index | P06 | `prim_index` field; C does both allocs Phase 1 |
| 4 | Element dealloc → return display primitive to free list | P06 | Types; C owns dealloc Phase 1 |

### Display List Management — Rendering Order (§ Display list management → Rendering order)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1 | Separate rendering-order linked list for visual layering | Phase 2+ | P06 type definitions; C owns list entirely |

### Coordinate and Precision System (§ Coordinate and precision system)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1 | Three-tier precision: display, world, velocity | P03 | `battle_types/coords.rs` |
| 2 | Bit-shift conversions (display↔world ×4, world↔velocity ×32) | P03 | Shift constants |
| 3 | No floating-point substitution for integer shift-and-accumulate | P03, P07 | Enforced across all implementations |
| 4 | Logical space dimensions = display × world_shift × max_reduction | P04 | `constants.rs` |
| 5 | Three discrete zoom levels | P10 | Zoom constants |
| 6 | Continuous zoom with 8-bit fractional, max zoom-out 4 | P10 | Zoom constants |
| 7 | Toroidal wrapping in both axes | P03 | `wrap_x()`, `wrap_y()` |
| 8 | Shortest-path delta (adjust if > half dimension) | P03 | `shortest_path_delta()` |
| 9 | Wrapping applied during postprocess, not velocity stepping | Phase 2+ | P03 types; C owns postprocess |
| 10 | Display alignment rounds to world-coordinate boundary | P03 | Shift constants |
| 11 | 64-step angle system with wraparound | P03 | `battle_types/angles.rs` |
| 12 | 16-direction facing from angles (add-half-then-shift) | P03 | `angle_to_facing()` |
| 13 | Angle normalization via bitmask | P03 | `normalize_angle()` |
| 14 | Facing normalization via bitmask | P03 | `normalize_facing()` |
| 15 | Sine/cosine lookup table, 14-bit precision (16384) | P03 | `battle_types/trig.rs` |
| 16 | SINE = table × magnitude >> 14 | P03 | `sine()` |
| 17 | COSINE = SINE(angle + quadrant) | P03 | `cosine()` |
| 18 | Arctangent lookup table, 0–63 range | P03 | `arctan()` |
| 19 | Battle viewport = screen width − 64-pixel status panel | P04 | `constants.rs` |
| 20 | Universe coordinates 0–9999 each axis | P04 | `constants.rs` |

### Velocity System (§ Velocity system)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1 | Velocity descriptor: angle, vector, fraction, error, increment | P07 | `VelocityDesc` struct |
| 2 | Bresenham-style accumulation, no floating-point | P07 | Integer-only implementation |
| 3 | Increment encoding: positive → lo=1 hi=0; negative → lo=0xFF hi=2×remainder | P07 | Exact C byte order |
| 4 | Increment preserved across language boundary and netplay CRC | P07, P15, P17 | FFI + CRC serialization |
| 5 | Operations: read current, compute delta N frames, set from magnitude+facing, set from components, add delta, zero, test zero | P07 | Full method suite |
| 6 | Set from magnitude+facing: trig decomposition, split into vector/fraction/increment | P07 | `set_vector()` |
| 7 | Set from components: compute travel angle via arctangent | P07 | `set_components()` |
| 8 | Delta velocity: read current + add delta + recompute | P07 | `delta_components()` |
| 9 | Compute delta N frames: accumulate fraction N times, mutate error | P07 | `get_next_components()` |

### Collision System (§ Collision system)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1 | Ineligible if NONSOLID or DISAPPEARING | P05 | `is_collidable()` |
| 2 | Collision possible: eligible, not both COLLISION, IGNORE_SIMILAR satisfied, mass > 0 | P05, P08 | `collision_possible()` |
| 3 | Pixel-accurate intersection testing (trajectory-based) | Phase 2+ | P08 `IntersectControl` type |
| 4 | Intersection start/end from current/next positions | Phase 2+ | P08 types |
| 5 | Preprocess: forward-only (successors); postprocess cascading: full list | Phase 2+ | P08 constants |
| 6 | Recursive deeper-collision check before dispatch | Phase 2+ | C owns `ProcessCollisions` |
| 7 | Collision handlers invoked in pairs | Phase 2+ | C owns dispatch |
| 8 | Dispatch order: PLAYER_SHIP test element first | Phase 2+ | P08 dispatch order types |
| 9 | Set COLLISION flag on both participants | Phase 2+ | C owns dispatch |
| 10 | Snap next position to collision point when COLLISION set | Phase 2+ | C owns dispatch |
| 11 | Non-finite-life collision → elastic response after handlers | P08 | `elastic_collide()` |
| 12 | Stuck overlap: APPEARING kill, non-APPEARING position revert | Phase 2+ | P08 types |
| 13 | Elastic response: impact angle via arctangent | P08 | Implemented in `elastic_collide()` |
| 14 | Elastic response: relative velocity, collision speed, directness | P08 | Implemented |
| 15 | Scraping: fudge directness to half-circle | P08 | Implemented |
| 16 | Momentum transfer: sine × speed × mass, inversely proportional to mass | P08 | Implemented |
| 17 | Minimum velocity enforcement | P08 | Implemented |
| 18 | Both stationary → DEFY_PHYSICS, fudge angles | P08 | Implemented |
| 19 | Gravity-mass immovable (mass_points ≥ 100) | P08 | Uses `gravity_mass()` from P03 |
| 20 | Player ship penalty: clear max-speed, add wait counters | P08 | Implemented |
| 21 | Post-bounce rechecks: full-list rescan after velocity change | Phase 2+ | C owns `ProcessCollisions` |

### Weapon System (§ Weapon system)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1a | Laser init TYPE DEFINITIONS: `LaserBlock` fields (LINE_PRIM, life=1, start position, velocity, callback) | P09 | `LaserBlock` type definition |
| 1b | Laser init ORCHESTRATION: allocate element, set callbacks, compute position from ship+offset | Phase 2+ | `initialize_laser()` stays in C |
| 2a | Missile init TYPE DEFINITIONS: `MissileBlock` fields (STAMP_PRIM, hp/damage/life/speed, preprocess, position, velocity, back-up) | P09 | `MissileBlock` type definition |
| 2b | Missile init ORCHESTRATION: allocate element, set callbacks, compute spawn position, back up position | Phase 2+ | `initialize_missile()` stays in C |
| 3 | Weapon collision: COLLISION guard prevents double-hit | P09 | `weapon_collision()` leaf function |
| 4 | Nonzero damage + FINITE_LIFE/NORMAL_LIFE target → apply damage; survivor sets COLLISION on weapon | P09 | Implemented |
| 5 | Weapon destroyed: zero hp/life, set COLLISION+NONSOLID, damage sound | P09 | Implemented |
| 6 | Non-line weapon destroyed → also set DISAPPEARING | P09 | Implemented |
| 7 | Weapon destroyed → create blast effect at collision point | P09 | Implemented |
| 8 | Line-type weapons: no DISAPPEARING on collision | P09 | Implemented |
| 9a | Blast: 8 directional bins (velocity angle quantized to 16÷2 with even/odd rounding) | P09 | Implemented in blast creation |
| 9b | Blast: standard path — ≤16 frames → 2-frame blast from shared array | P09 | Implemented |
| 9c | Blast: custom path — >16 frames → multi-frame from weapon farray with animation preprocess callback | P09 | Implemented |
| 10 | Damage: decrement hit_points/crew; zero + FINITE_LIFE → life=0 | P09 | `do_damage()` helper |
| 11 | Damage silhouette: rejection-sampling within ship silhouette | P09 | Type definitions; rendering stays in C |
| 12 | Homing: h_target fast path, display list scan fallback | P09 | `track_ship()` leaf function |
| 13 | Cloaked ships invisible to tracking (unless tracker has APPEARING) | P09 | Implemented |
| 14 | Tracking: Manhattan distance with toroidal shortest-path | P09 | Implemented |
| 15 | 180° target → random turn direction | P09 | Implemented |

### Process Loop (§ Process loop)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1 | Frame dispatch: SetContext → PreProcess → PostProcess → sounds → render | Phase 2+ | P10 type definitions |
| 2 | Simulation always executes; only rendering conditionally skipped | Phase 2+ | P10 documentation |
| 3 | PreProcessQueue: head-to-tail, PreProcess, collision vs successors, camera | Phase 2+ | P10 types |
| 4 | Zoom from ship separation, camera from ship midpoint | Phase 2+ | P10 types + constants |
| 5 | PreProcess per-element: life=0 → death; APPEARING → init intersection | Phase 2+ | P05 lifecycle methods |
| 6 | Non-PLAYER_SHIP APPEARING → skip preprocess callback | Phase 2+ | P10 documentation |
| 7 | PLAYER_SHIP APPEARING → clear in local copy only, invoke callback | Phase 2+ | P10 documentation |
| 8 | Non-IGNORE_VELOCITY → apply velocity via Bresenham | Phase 2+ | P07 velocity ops |
| 9 | CHANGING + collidable → reinit intersection | Phase 2+ | P10 documentation |
| 10 | Collidable → init intersection end from next position | Phase 2+ | P10 types |
| 11 | FINITE_LIFE → decrement life span | Phase 2+ | P05 types |
| 12 | After PreProcess: set PRE_PROCESS, clear POST_PROCESS + COLLISION | P05 | Flag transition helper |
| 13 | PostProcessQueue: iterate head-to-tail, asymmetric flag clearing | Phase 2+ | P05 flag helpers, P10 types |
| 14 | Newly-added cascading: inner loop, PreProcess, full-list collision | Phase 2+ | P10 types |
| 15 | Cascading continues until no new elements (tail-chasing) | Phase 2+ | P10 documentation |
| 16 | After cascading inner loop: zero scroll offsets | Phase 2+ | P10 types |
| 17 | Scroll: preprocessed+not-postprocessed → apply scroll offsets | Phase 2+ | P10 types |
| 18 | Scroll: preprocessed+postprocessed → zero scroll (already adjusted) | Phase 2+ | P10 types |
| 19 | DISAPPEARING → remove and deallocate | Phase 2+ | P05 lifecycle |
| 20 | Surviving element: world→screen transform, zoom frame, postprocess callback, insert prim into render list | Phase 2+ | P10 types |
| 21 | Line prims: both endpoints transformed, wrap handled | Phase 2+ | P10 documentation |
| 22 | Stamp/stamp-fill: zoom-level frame from farray, optional trilinear mipmap | Phase 2+ | P10 types |
| 23 | After PostProcess: copy next→current, reinit intersection, set POST_PROCESS, clear PRE_PROCESS+CHANGING+APPEARING | Phase 2+ | P05 flag helpers |
| 24 | Discrete zoom: 3 levels with hysteresis thresholds | P10 | Constants and type defs |
| 25 | Continuous zoom: linear interpolation, fractional, clamped | P10 | Constants and type defs |
| 26 | Camera: midpoint between ships, scroll delta | P10 | Type definitions |
| 27 | Single-ship camera: clamp scroll speed | P10 | Constants |
| 28 | Zoom change → recalculate space origin | Phase 2+ | P10 types |
| 29 | Camera view state: stable, scroll-only, zoom-changed | P10 | `ViewState` enum |
| 30 | Discrete world→screen: subtract origin, shift right by reduction | P10 | Conversion constants |
| 31 | Continuous world→screen: subtract origin, shift left by precision, divide by zoom factor | P10 | Conversion constants |

### Battle Lifecycle (§ Battle lifecycle)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1a | Battle entry: seed RNG | Phase 3 | P11 types |
| 1b | Battle entry: load music | Phase 3 | P11 types |
| 1c | Battle entry: InitShips | Phase 3 | P11 types |
| 1d | Battle entry: count ship sides | Phase 3 | P11 types |
| 2 | Init ships+space: load shared assets, set contexts, reset display list, init stars, spawn objects | Phase 3 | P11 types |
| 3a | Shared asset: star field mask (reference counted) | Phase 3 | P11 types |
| 3b | Shared asset: explosion sprites at all zoom levels (reference counted) | Phase 3 | P11 types |
| 3c | Shared asset: blast sprites at all zoom levels (reference counted) | Phase 3 | P11 types |
| 3d | Shared asset: asteroid sprites at all zoom levels (reference counted) | Phase 3 | P11 types |
| 4a | Valid battle: set activity flag | Phase 3 | P11 types |
| 4b | Valid battle: count ships per side | Phase 3 | P11 types |
| 4c | Valid battle: set graphics scale | Phase 3 | P11 types |
| 4d | Valid battle: determine input order | Phase 3 | P11 types |
| 4e | Valid battle: spawn ships | Phase 3 | P11 types |
| 4f | Valid battle: start music | Phase 3 | P11 types |
| 4g | Valid battle: enter frame loop | Phase 3 | P11 types |
| 5 | Instant-victory: skip frame loop | Phase 3 | P11 types |
| 6 | Frame callback: not own loop, callback invoked per frame, return true/false | Phase 3 | P11 `BattleState` type |
| 7 | BattleState first field = InputFunc (DoInput pattern) | P11 | Layout assertion |
| 8 | Per-frame: input → batch → callback → simulate → render → unbatch → exit check | Phase 3 | P11 types |
| 9 | First frame: screen transition effect | Phase 3 | P11 types |
| 10 | In-battle cleared or abort → return false | Phase 3 | P11 types |
| 11 | Frame rate = 24 fps | P11 | `BATTLE_FRAME_RATE` constant |
| 12 | Normal speed: sleep until next deadline | Phase 3 | P11 types |
| 13 | Max speed: skip sleep, process async, yield, suppress rendering | Phase 3 | P11 types |
| 14 | Input processing: iterate sides, input handler, map bits, escape check | Phase 3 | P11 types |
| 15 | Escape → flee sequence | Phase 3 | P11 types, P13 flee types |
| 16a | Teardown: stop ditty | Phase 3 | P11 types |
| 16b | Teardown: stop music | Phase 3 | P11 types |
| 16c | Teardown: stop sounds | Phase 3 | P11 types |
| 17a | Uninit ships: stop sounds | Phase 3 | P11 types |
| 17b | Uninit ships: free shared assets | Phase 3 | P11 types |
| 17c | Uninit ships: count crew | Phase 3 | P11 types |
| 17d | Uninit ships: find survivor | Phase 3 | P11 types |
| 17e | Uninit ships: cap crew to max | Phase 3 | P11 types |
| 17f | Uninit ships: record crew in queue entry | Phase 3 | P11 types |
| 17g | Uninit ships: free descriptors | Phase 3 | P11 types |
| 17h | Uninit ships: clear activity flag | Phase 3 | P11 types |
| 18 | Encounter: persist crew to fleet via writeback | Phase 3 | P11 types |
| 19 | Non-encounter: reinit queues, free hyperspace resources | Phase 3 | P11 types |
| 20 | Teardown returns: hyperspace exit (negative ship count) | P11 | i16 return type |

### Ship Runtime Within Battle (§ Ship runtime within battle)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1 | Ship spawn: load descriptor, patch crew from queue | Phase 2/3 | P12 types |
| 2 | Ship element flags: APPEARING, PLAYER_SHIP, IGNORE_SIMILAR, shared callbacks, zero velocity, mass, NORMAL_LIFE | Phase 2/3 | P12 types |
| 3 | Position: random avoiding gravity wells | Phase 2/3 | P12 types |
| 4 | Bidirectional binding: element↔queue entry | Phase 2/3 | P12 types |
| 5 | Final battle: defending ship at center | Phase 2/3 | P12 constant |
| 6 | Queue entry reuse: reinitialize in place | Phase 2/3 | P12 types |
| 7 | Per-frame pipeline: input → APPEARING → energy → preprocess → turn → thrust → status | Phase 2/3 | P12 pipeline enum |
| 8 | First frame (APPEARING): suppress inputs, init crew, invoke preprocess, start warp-in, return early | Phase 2/3 | P12 types |
| 9 | Energy regeneration when counter elapses | Phase 2/3 | P12 constants |
| 10 | Turn: facing ±1, update image, apply turn-wait cooldown | Phase 2/3 | P12 types |
| 11 | Thrust: inertial computation, ion trail spawn, thrust-wait cooldown | Already in ships/runtime.rs → P03 (math) | `inertial_thrust()` already implemented; relocated to `battle_types/` in P03 |
| 12 | Inertial: thrust = acceleration in facing, coast at current velocity, max speed enforced | Already in ships/runtime.rs → P03 | Already implemented; relocated via re-exports |
| 13 | Inertialess: velocity = max speed along facing | Already in ships/runtime.rs → P03 | Already implemented; relocated via re-exports |
| 14 | Normal inertial: compare v² vs max-thrust² threshold | Already in ships/runtime.rs → P03 | Already implemented; relocated via re-exports |
| 15 | Gravity well: permit up to 2304 velocity units | P12 | Constant (already used in `ships/runtime.rs`) |
| 16 | At-max-speed turning: half-thrust new − full-thrust old | Already in ships/runtime.rs → P03 | Already implemented; relocated via re-exports |
| 17 | Inertial thrust returns: at-max-speed, beyond-max-speed, in-gravity-well flags | Already in ships/runtime.rs → P03 | Already implemented; relocated via re-exports |
| 18 | Ship collision with gravity-mass: damage = hp/4 (min 1) | P12 | Constant |
| 19 | Ship collision with non-gravity, non-finite-life: elastic response only | P08 | `elastic_collide()` |
| 20 | Postprocess pipeline: exit if crew=0, weapon fire, special counter, race postprocess, status update | Phase 2/3 | P12 types |
| 21 | Weapon fire: cooldown + energy + callback + bind + sound + wait | Phase 2/3 | P12 types, P09 weapon types |
| 22 | Energy: regen rate/interval, weapon/special deduct, cap at max | Phase 2/3 | P12 constants |
| 23 | Crew: decremented by damage, min 0 | Phase 2/3 | P12 constants |

### Tactical Transitions (§ Tactical transitions)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1a | Ship death: stop music | Phase 2/3 | P13 types |
| 1b | Ship death: clear victory-ditty flag | Phase 2/3 | P13 types |
| 1c | Ship death: start explosion | Phase 2/3 | P13 types |
| 1d | Ship death: find winner | Phase 2/3 | P13 types |
| 1e | Ship death: record death (decrement counter, melee notification) | Phase 2/3 | P13 types |
| 2 | Death sequence as callback-replacement state machine | Phase 2/3 | P13 phase enum |
| 3 | 4 phases: ship_death → explosion → cleanup → new_ship | Phase 2/3 | P13 phase enum |
| 4 | Explosion: zero velocity, drain energy | Phase 2/3 | P13 types |
| 5 | Explosion: life=36, FINITE_LIFE+NONSOLID, replace preprocess+death, play sound | Phase 2/3 | P13 constants |
| 6 | Explosion: 1-3 debris/frame at random positions | Phase 2/3 | P13 types |
| 7 | Frame 15: hide display primitive | Phase 2/3 | P13 constant |
| 8 | Frame 25: clear explosion preprocess | Phase 2/3 | P13 constant |
| 9 | Cleanup: record crew, clear ownership, mark deletion, preserve CREW_OBJECT | Phase 2/3 | P13 types |
| 10 | Winner has play-victory-ditty → play victory music | Phase 2/3 | P13 types |
| 11 | Cleanup: replace death callback with new-ship, life=3sec of frames | Phase 2/3 | P13 constants |
| 12 | Winner kept alive one frame longer than loser | Phase 2/3 | P13 documentation |
| 13 | New-ship: wait for readiness (ditty done, netplay sync) | Phase 2/3 | P13 types |
| 14 | No replacement → clear in-battle flag | Phase 2/3 | P13 types |
| 15 | SuperMelee replacement: delegate to ship-picker | Phase 2/3 | P13 types |
| 16 | NPC finite fleet: next in queue (successor link) | Phase 2/3 | P13 types |
| 17 | NPC infinite fleet: recycle queue entry | Phase 2/3 | P13 types |
| 18 | Human RPG: armada picker or auto-select sole ship | Phase 2/3 | P13 types |
| 19 | Fleet persistence: SuperMelee deactivate; infinite NPC reuse | Phase 2/3 | P13 documentation |
| 20a | Winner determination: display-list head-to-tail iteration | Phase 2/3 | P13 types |
| 20b | Winner determination: zero-crew = null (mutual destruction) | Phase 2/3 | P13 types |
| 20c | Winner determination: first-call-only (recorded once) | Phase 2/3 | P13 documentation |
| 20d | Winner determination: victory-ditty set each death event | Phase 2/3 | P13 documentation |
| 20e | Winner determination: result depends on display list order (port must preserve) | Phase 2/3 | P13 documentation |
| 21 | Zero crew + not reincarnating → null winner (mutual destruction) | Phase 2/3 | P13 types |
| 22 | Winner recorded once; victory-ditty set each death | Phase 2/3 | P13 documentation |
| 23 | Winner depends on display list order (port must preserve) | Phase 2/3 | P13 documentation |
| 24 | Pkunk: mass = MAX_SHIP_MASS+1 + zero crew → alive (reincarnating) | P13 | Constant |
| 25a | OpponentAlive: iterate all elements in display list | Phase 2/3 | P13 types |
| 25b | OpponentAlive: check non-null owning ship for each element | Phase 2/3 | P13 types |
| 25c | OpponentAlive: check race_desc crew > 0 | Phase 2/3 | P13 types |
| 25d | OpponentAlive: return false (no opponent) / true (opponent alive) semantics | Phase 2/3 | P13 types |
| 26 | Ship death recording: decrement battle counter; SuperMelee notification | Phase 2/3 | P13 types |
| 27a | Ion trail: POINT_PRIM type | P13 | Constant (Phase 1 type) |
| 27b | Ion trail: 12-color orange→red fade cycle (one color per frame) | P13 | 12-color palette constant array (Phase 1 type) |
| 27c | Ion trail: insertion at head of display list (drawn behind everything) | Phase 2+ | Orchestration — display list insertion |
| 27d | Ion trail: marked as already preprocessed (PRE_PROCESS set) | Phase 2+ | Orchestration — flag management |
| 27e | Ion trail: life span pre-decremented (head-inserted elements skip normal preprocessing) | Phase 2+ | Orchestration — lifecycle management |
| 28 | Warp-in: life=15, replace preprocess, clear postprocess, hide prim, NONSOLID+FINITE_LIFE+CHANGING | Phase 2/3 | P13 constant + types |
| 29 | Warp ghost images: one per frame along facing, ion-trail colors | Phase 2/3 | P13 types |
| 30a | Warp-in materialize: show primitive | Phase 2/3 | P13 types |
| 30b | Warp-in materialize: select zoom frame | Phase 2/3 | P13 types |
| 30c | Warp-in materialize: init intersection | Phase 2/3 | P13 types |
| 30d | Warp-in materialize: zero velocity | Phase 2/3 | P13 types |
| 30e | Warp-in materialize: clear NONSOLID+FINITE_LIFE flags | Phase 2/3 | P13 types |
| 30f | Warp-in materialize: restore callbacks | Phase 2/3 | P13 types |
| 31 | Warp-out: zero crew → cleanup/new-ship phases | Phase 2/3 | P13 types |
| 32 | Flee eligibility: encounter/final-battle, starbase, not bomb carrier | Phase 2/3 | P13 types |
| 33 | Flee conditions: stamp prim, NORMAL_LIFE, no FINITE_LIFE, not FLEE_MASS, no APPEARING | P13 | Eligibility condition types |
| 34a | Flee initiation: mass=10×MAX_SHIP_MASS | Phase 2/3 | P13 `FLEE_MASS` constant |
| 34b | Flee initiation: replace preprocess with flee handler | Phase 2/3 | P13 types |
| 34c | Flee initiation: dark red stamp-fill | Phase 2/3 | P13 constants |
| 34d | Flee initiation: clear color cycle index | Phase 2/3 | P13 constants |
| 34e | Flee initiation: set initial timing counters | Phase 2/3 | P13 constants |
| 34f | Flee initiation: suppress input | Phase 2/3 | P13 types |
| 35 | Flee animation: 20-color pulse, accelerating timing, all inputs suppressed | Phase 2/3 | P13 20-color palette |
| 36a | Flee warp-out trigger: timing counter reaches zero | Phase 2/3 | P13 types |
| 36b | Flee warp-out trigger: color cycle reaches midpoint | Phase 2/3 | P13 types |
| 36c | Flee warp-out trigger conditions (timing=0 AND cycle=midpoint) documented as constants | P13 | Trigger condition constants (Phase 1 type definitions) |
| 36d | Flee warp-out action: set crew=0 | Phase 2/3 | P13 types |
| 36e | Flee warp-out action: set death callback to cleanup | Phase 2/3 | P13 types |
| 36f | Flee warp-out action: trigger warp-out transition | Phase 2/3 | P13 types |
| 37 | Flee warp-out complete → normal cleanup/new-ship, crew=0, deactivate entry | Phase 2/3 | P13 types |

### AI Dispatch (§ AI dispatch)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1 | AI input → invoke race-specific intelligence callback | Phase 2/3 | P14 types |
| 2 | RPG overlay: merge human escape with AI battle input | Phase 2/3 | P14 types |
| 3 | Final battle: return no AI input (disabled) | Phase 2/3 | P14 types |
| 4 | PSYTRON: pause + weapon-button for random ship selection | Phase 2/3 | P14 types |
| 5 | Range thresholds: close=200, long=4000 | P14 | Constants |
| 6 | Maneuverability indices: fast=150, medium=45, slow=25 | P14 | Constants |
| 7 | Object tracking: enemy ship, crew, enemy weapon, gravity mass, first-empty index | P14 | Tracking index constants |
| 8 | Control flags: HUMAN, CYBORG, PSYTRON, NETWORK, AI ratings | P14 | Constants |

### Thread and Timing (§ Thread and timing)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1 | Cooperative polling loop, no own loop in per-frame callback | Phase 3 | P11 `BattleState` type |
| 2 | Frame timing: timed sleep (normal), async+yield (max speed) | Phase 3 | P11 types |
| 3 | Graphics batching brackets rendering ops | Phase 3 | P11 types |
| 4 | Default frame rate = 24 fps | P11 | `BATTLE_FRAME_RATE` constant |
| 5 | Max speed: suppress rendering, continue simulation+flush sounds | Phase 3 | P11 types |

### Netplay Integration (§ Netplay integration)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1 | CRC: serialize 19 fields in exact order, 35 bytes per element, LE | P15 | `crc_process_element()` |
| 2 | BACKGROUND_OBJECT → skip entirely (zero bytes) | P15 | Implemented by omission |
| 3 | CRC includes RNG state as 32-bit value before element data | P15 | Implemented |
| 4 | CRC: little-endian regardless of platform | P15 | Implemented |
| 5 | CRC type: 32-bit unsigned | P15 | `CrcState` |
| 6 | Fields excluded: player_nr, prim_index, color_cycle, intersection, image, parent, target, links, callbacks | P15 | Implemented by omission |
| 7 | Input buffering: configurable delay, push/pop per side | Phase 2/3 | P15 hook types |
| 8 | Frame synchronization: CRC at intervals, mismatch → abort | Phase 2/3 | P15 types |
| 9 | Battle-end sync: multi-phase protocol (4 phases) | Phase 2/3 | P15 phase enum |
| 10 | Determinism: bit-identical given same state+input, no float | P07, P08, P15 | Enforced in all leaf functions |

### Integration Points — Graphics Subsystem (§ Integration points → Graphics subsystem)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| G1 | Display primitive array access | Phase 2+ | C owns prims |
| G2 | Primitive free list management | Phase 2+ | C owns alloc |
| G3 | Primitive type get/set | P17 FFI decl | P04 type constants, P09 LINE_PRIM check |
| G4 | Primitive property operations (stamp, line, point, fill properties) | Phase 2+ | C owns prim state |
| G5 | Batch rendering entry (DrawBatch) | Phase 2+ | C owns frame dispatch |
| G6 | Graphic scale get/set | Phase 2+ | C owns zoom |
| G7 | Scale mode operations | Phase 2+ | C owns zoom |
| G8 | Drawable clear operations | Phase 2+ | C owns frame dispatch |
| G9 | Drawing context management (SetContext, status/space contexts) | Phase 2+ | C owns contexts |
| G10 | Clip rectangle operations | Phase 2+ | C owns contexts |
| G11 | Background color / foreground frames | Phase 2+ | C owns rendering |
| G12 | Screen transition operations | Phase 2+ | C owns transitions |
| G13 | Frame index / equivalent frame queries | P17 FFI decl | P09 blast frame selection |
| G14 | Frame rectangle queries | P17 FFI decl | P09 blast positioning |
| G15 | Frame count queries | P17 FFI decl | P09 standard vs custom blast |
| G16 | Pixel-accurate intersection testing (DrawablesIntersect) | P17 FFI decl | P08 `IntersectControl`, P09 damage silhouette |
| G17 | Trilinear mipmap setup | Phase 2+ | C owns rendering |
| G18 | Primitive link management (link/unlink display primitives in render chain) | Phase 2+ | C owns rendering order |

### Integration Points — Audio Subsystem (§ Integration points → Audio subsystem)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| A1 | Positioned sound playback (PlaySound) | P17 FFI decl | P09 damage sound in `weapon_collision` |
| A2 | Sound stopping (StopSound) | Phase 2+ | C owns lifecycle |
| A3 | Element-positioned sound processing | Phase 2+ | C owns frame dispatch |
| A4 | Music playback (PlayMusic) | Phase 2+ | C owns lifecycle |
| A5 | Music stopping (StopMusic) | Phase 2+ | C owns lifecycle |
| A6 | Stereo position calculation | Phase 2+ | C owns frame dispatch |
| A7 | Stereo position updating | Phase 2+ | C owns frame dispatch |
| A8 | Sound position removal on element death | Phase 2+ | C owns lifecycle |
| A9 | Sound flushing (FlushSounds) | Phase 2+ | C owns frame dispatch |
| A10 | Music-playing status query | Phase 2+ | C owns lifecycle |
| A11 | Menu sound suppression during battle | Phase 2+ | C owns lifecycle |

### Integration Points — Threading Subsystem (§ Integration points → Threading subsystem)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| T1 | Cooperative yield (TaskSwitch) | Phase 2+ | C owns frame loop |
| T2 | Timed sleep (SleepThreadUntil) | Phase 2+ | C owns frame loop |
| T3 | Cooperative input loop framework (DoInput) | Phase 2+ | C owns frame loop |

### Integration Points — Input Subsystem (§ Integration points → Input subsystem)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| I1 | Per-player input handlers (PlayerInput) | Phase 2+ | C owns input dispatch |
| I2 | Control flags (PlayerControl) | Phase 2+ | C owns input dispatch |
| I3 | Frame-input polling (frameInput) | Phase 2+ | C owns input dispatch |
| I4 | Raw-to-battle input conversion (CurrentInputToBattleInput) | Phase 2+ | C owns input dispatch |

### Integration Points — Resource Subsystem (§ Integration points → Resource subsystem)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| R1 | Graphic asset loading (LoadGraphic) | Phase 2+ | C owns asset lifecycle |
| R2 | Drawable capture (CaptureDrawable) | Phase 2+ | C owns asset lifecycle |
| R3 | Drawable release (ReleaseDrawable) | Phase 2+ | C owns asset lifecycle |
| R4 | Drawable destruction (DestroyDrawable) | Phase 2+ | C owns asset lifecycle |
| R5 | Music destruction (DestroyMusic) | Phase 2+ | C owns asset lifecycle |

### Integration Points — Ship/Race Subsystem (§ Integration points → Ship/race subsystem)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| S1 | Race descriptor behavioral callbacks (preprocess, postprocess, weapon init, teardown) | P17 FFI decl | P09/P17 weapon init adapter |
| S2 | Ship queue management (per side) | Phase 2+ | C owns queues |
| S3 | Ship loading/freeing | Phase 2+ | C owns lifecycle |
| S4 | Energy management operations | Phase 2+ | C owns ship runtime |
| S5 | Status bar initialization | Phase 2+ | C owns UI |
| S6 | Status bar update | Phase 2+ | C owns UI |
| S7 | No race-specific logic in battle engine (design constraint) | All phases | Enforced by architecture — callbacks only |

### Integration Points — Global State (§ Integration points → Global state)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| GS1 | CurrentActivity flags (in-battle, check-abort, check-load, in-encounter, final-battle, super-melee) | Phase 2+ | C owns activity state |
| GS2 | Game state variables (GET_GAME_STATE) | Phase 2+ | C owns game state |
| GS3 | Pseudo-random number generator (TFB_Random) | P17 FFI decl | P09 tracking random turn, P15 CRC includes RNG state |
| GS4 | Space type detection (hyperspace, quasispace) | Phase 2+ | C owns navigation state |

### Cross-Language Boundary (§ Cross-language boundary considerations)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1 | Init return: support negative values (hyperspace exit) | P17 | i16 return type preserved |
| 2 | Element field order matches across language boundary, links first | P04 | Compile-time offset assertions |
| 3 | Behavioral hooks via 4 registered callbacks | P04, P05 | Callback function pointer types |

### Error Handling and Invariants (§ Error handling and invariants)

| # | Requirement (short text) | Phase | Notes |
|---|-------------------------|-------|-------|
| 1 | Robust against element pool exhaustion | P06 | NULL_HANDLE return |
| 2 | Robust against display primitive exhaustion | P06 | NULL_HANDLE return |
| 3 | Deterministic processing order preserved across frames | P06 | DisplayList ordering |
| 4 | Double-buffer pattern: next computed in preprocess, next→current in postprocess, collision uses current→next trajectory | P05 | `commit_state()` method |
| 5 | Teardown robust: ships never fully spawned, absent hooks, already-freed descriptors, no-descriptor queue entries | Phase 2/3 | P11 types; C owns teardown Phase 1 |
