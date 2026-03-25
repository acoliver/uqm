# Battle Engine Phase 2/3 — Functional & Technical Specification

This document specifies the Phase 2/3-specific design decisions for porting the battle engine's full orchestration logic from C to Rust. It is an addendum to the shared battle engine specification (`battle/specification.md`), which defines the end-state types, contracts, and integration model that all three phases target.

Sections marked **[Normative]** define required contracts. Sections marked **[Reference design]** describe one acceptable approach.

---

## 1. Scope

### 1.1 What Phase 2/3 owns

Phase 2/3 ports the **orchestration logic** that Phase 1 left in C:

1. the process loop: `PreProcessQueue`, `PostProcessQueue`, `ProcessCollisions`, `RedrawQueue`, zoom/camera calculation, coordinate transforms, element allocation/deallocation coupling,
2. the ship runtime pipeline: `ship_preprocess`, `ship_postprocess`, `inertial_thrust`, `animation_preprocess`, `ship_collision`,
3. ship spawn and initialization: `spawn_ship`, `GetNextStarShip`, `GetInitialStarShips`,
4. tactical transitions: the complete death→explosion→cleanup→new_ship callback chain, flee/warp sequences, winner determination, ion trail management, battle music helpers,
5. AI dispatch: `computer_intelligence` entry point with all four dispatch paths,
6. battle lifecycle: `Battle()` entry/exit, `InitShips`/`UninitShips`, `InitSpace`/`UninitSpace`, `ProcessInput`, shared-asset reference counting, frame timing,
7. the C bridge layer: Rust→C FFI wrappers for all subsystem integration operations, and
8. the Phase 3 FFI exports and C-side guard/wrapper wiring that enables the runtime ownership flip.

### 1.2 What Phase 2/3 does not own

- **Core types and leaf functions.** Phase 1 (7,591 lines, 2,151 tests) created the `#[repr(C)]` types (`Element`, `VelocityDesc`, `DisplayList`, `IntersectControl`), leaf math/physics functions (`elastic_collide`, `weapon_collision`, `track_ship`, velocity operations, trig), CRC-32, and integration trait definitions. Phase 2/3 uses these as-is.
- **Individual ship race implementations.** Per-race behavior remains in the ships subsystem.
- **DoInput framework.** The engine-wide cooperative polling loop remains in C.
- **Netplay transport.** The network protocol layer remains out of scope.
- **Graphics/audio/resource internals.** Phase 2/3 calls these via the bridge layer.

### 1.3 Relationship to the shared specification

The shared specification (`battle/specification.md`) defines the end-state contracts in §1–§18:

- §2.1 describes the three-phase migration strategy and explicitly defines Phase 2 and Phase 3 scope.
- §2.2–§2.5 define the Element struct layout, display list ownership model, process loop location, and callback function pointer design — all of which Phase 2/3 implements.
- §3–§13 define type contracts (Element, DisplayList, VelocityDesc, collision, weapon, process loop, lifecycle, tactical, AI, netplay) that Phase 1 implemented and Phase 2/3 extends with behavioral logic.
- §14–§15 define integration boundary operations and the ships↔battle weapon adapter.

This addendum covers design decisions specific to the Phase 2/3 port that are not in the shared spec: the ownership transfer model, symbol preservation strategy, boundary function inventory, callback-slot migration rules, module evolution, and guard/toggle staging.

Where this addendum provides more specific guidance than the shared specification for Phase 2/3 implementation, this addendum governs.



---

## 2. Port strategy — ownership transfer model

### 2.1 Incremental ownership transfer

**[Normative]** Phase 2/3 transfers ownership of battle orchestration from C to Rust incrementally. Each implementation phase ports a coherent slice of logic to Rust. Until the final FFI wiring phase activates the `USE_RUST_BATTLE_LOOP` toggle, the C runtime path remains the supported execution path and ported Rust code exists as dark-code for testing only.

The transfer proceeds through three stages:

1. **Dark-code stage** (P03–P12): Rust implementations exist and are tested, but the live runtime uses C. Rust functions may be exercised in test harnesses but are not installed as live callbacks or entry points.
2. **Wiring stage** (P13): The `USE_RUST_BATTLE_LOOP` toggle is created. C function bodies are guarded out. The `DoBattle()` thin shell is activated. Rust FFI exports are wired to C wrapper surfaces.
3. **Integration stage** (P14): End-to-end verification with all modules connected.

### 2.2 No parallel canonical state

**[Normative]** Rust does not introduce parallel canonical global state (display lists, queues, battle counters) that could diverge from C state. During Phase 2/3, the Rust code operates on the same backing storage as C — accessed via `#[repr(C)]` types and FFI — not on independent Rust copies.

---

## 3. Permanent C boundary

### 3.1 Functions not ported

**[Normative]** The following C functions are explicitly NOT ported by Phase 2/3. Each remains in C with the stated justification. This section, combined with the per-phase function assignment tables in the implementation plan, ensures every C function in scope is either "ported in Phase X" or "permanent C boundary."

#### 3.1.1 Retained boundary taxonomy

To avoid ambiguity, three distinct boundary categories are used:

- **Semantically C-owned boundary body:** The C symbol and its battle semantics both remain in C in all supported modes.
- **Retained ABI shell with Rust semantics behind it:** The C symbol remains for ABI/callback registration, but in Rust-enabled mode its body delegates semantics to Rust.
- **C-ABI callback slot:** A function-pointer field stored in a foreign C struct remains a C-ABI dispatch slot; the installed target may be a retained C callback, a Rust `extern "C"` callback, or a documented boundary shim depending on build mode.

#### 3.1.2 DoInput framework callbacks

These functions are thin callbacks registered via C function pointers in the `DoInput()` cooperative scheduling framework. The `DoInput` framework is engine-wide and not battle-specific. Porting these provides no value until `DoInput` itself moves to Rust.

| C Function | Source File | Lines | Justification |
|-----------|------------|------:|---------------|
| `frameInputHuman()` | `battle.c` | :137-142 | One-liner delegate to `CurrentInputToBattleInput()`. Not called from Rust. |
| `DoBattle()` | `battle.c` | :258-354 | DoInput `InputFunc` callback — must match C ABI. Becomes a thin shell calling `rust_battle_frame()`. |
| `battleEndReadyHuman()` | `tactrans.c` | :231-236 | DoInput-pattern callback for human players. One-liner delegate. |
| `battleEndReadyComputer()` | `tactrans.c` | :239-243 | DoInput-pattern callback for AI players. One-liner delegate. |
| `battleEndReadyNetwork()` | `tactrans.c` | :246-250 | DoInput-pattern callback for network players. One-liner delegate. |

#### 3.1.3 Netplay transport callbacks

These implement netplay battle-end synchronization. They are registered as C function pointers in the netplay connection subsystem (out of scope) and use `DoInput()` cooperative scheduling.

| C Function | Source File | Lines | Justification |
|-----------|------------|------:|---------------|
| `readyToEnd2Callback()` | `tactrans.c` | :109-114 | Netplay end-phase-2 callback in the netplay connection subsystem. |
| `readyToEndCallback()` | `tactrans.c` | :117-150 | Netplay end-phase-1 callback with `inBattle → endingBattle` protocol transition. |
| `readyForBattleEndPlayer()` | `tactrans.c` | :169-228 | Multi-phase netplay readiness orchestration using `DoInput()` and netplay connection state. |

#### 3.1.4 Resource / ships subsystem internals

These belong to subsystems outside the battle engine's scope. They are called by battle lifecycle code via FFI.

| C Function | Source File | Lines | Justification |
|-----------|------------|------:|---------------|
| `load_animation()` | `init.c` | :49-77 | Resource subsystem internal — deep in the graphics resource layer. |
| `free_image()` | `init.c` | :81-113 | Resource subsystem internal — graphics resource layer. |
| `BuildSIS()` | `init.c` | :164-179 | Ships subsystem concern — fleet management, not battle logic. |

#### 3.1.5 Boundary summary

| Category | Count | Functions |
|----------|------:|-----------|
| Semantically C-owned retained boundary bodies | 10 | `frameInputHuman`, `battleEndReadyHuman/Computer/Network`, `readyToEnd2Callback`, `readyToEndCallback`, `readyForBattleEndPlayer`, `load_animation`, `free_image`, `BuildSIS` |
| Retained C ABI shell with Rust semantics behind it | 1 | `DoBattle` |
| **Total retained named C boundary surfaces** | **11** | |

After Phase 2/3 completes, exactly these 11 retained named C boundary surfaces remain. Of those, 10 remain semantically C-owned; `DoBattle()` is the one retained ABI shell. The battle implementation is otherwise Rust-owned, with supporting C wrapper/guard/ABI infrastructure around those boundaries.

---

## 4. DoBattle thin-shell contract

### 4.1 Design

**[Normative]** `DoBattle()` remains a C function symbol because the engine-wide `DoInput()` framework requires the existing C callback ABI. Phase 2/3 does not introduce a parallel `DoBattle()` symbol; it rewrites the existing `DoBattle()` body in place under `#ifdef USE_RUST_BATTLE_LOOP` so the callback registration and symbol identity stay unchanged while the enabled body becomes a thin shell delegating to `rust_battle_frame()`.

### 4.2 Invariant

With `USE_RUST_BATTLE_LOOP` disabled, the original C `DoBattle()` body remains active. With `USE_RUST_BATTLE_LOOP` enabled, that same C symbol compiles only the thin-shell body.

### 4.3 Responsibility split

**C-only responsibilities (thin shell):**
- Preserve the existing callback ABI and registration path expected by `DoInput()`.
- Marshal the existing `BATTLE_STATE` / callback arguments into `rust_battle_frame()`.
- Return Rust's `BOOLEAN` / status result back to `DoInput()` unchanged.
- Keep only the minimum preprocessor wiring needed for build compatibility.

**Rust-owned responsibilities (via `rust_battle_frame()`):**
- Per-frame simulation, preprocessing, collisions, postprocessing, and rendering decisions.
- Netplay checksum computation/verification, `battleFrameCount` evolution, and mismatch handling.
- Activity-flag mutation, max-speed render skipping, audio updates, and battle-end readiness orchestration.

**Prohibited in the C shell:**
- No direct display-list or element mutation.
- No duplicate frame logic, rendering logic, or lifecycle branching already owned by Rust.
- No netplay state-machine logic beyond invoking the Rust frame entrypoint and returning its result.

### 4.4 Supported runtime path

Once Phase 2/3 enables the handoff, external C callers still call the original `Battle()` symbol. That Rust-owned `Battle()` replacement prepares `BATTLE_STATE`, assigns the retained C `DoBattle()` shell to `BATTLE_STATE.InputFunc`, and then enters the existing C `DoInput()` loop. `DoInput()` continues to call the retained C `DoBattle()` callback surface, and that shell delegates per-frame semantics to `rust_battle_frame()`.

---

## 5. Public symbol provider model

### 5.1 Symbol-replacement rules

**[Normative]** Unless a function is explicitly listed as a retained permanent-boundary body or as the retained `DoBattle()` shell, any cross-file public battle symbol that must keep its original C name for callers is preserved by a documented C shim/wrapper surface rather than relying on same-name direct Rust symbol replacement.

**Pre-FFI-wiring export rule:** Before the FFI wiring phase, the only supported C-callable Rust battle exports are explicitly documented test/integration surfaces. Rust functions may exist as ordinary module functions for unit/integration harnesses but are not public C-callable provider surfaces.

### 5.2 Public symbol provider matrix

**[Normative]** Every non-static battle symbol that has current-tree external C callers must have a named final provider in Rust-enabled builds.

| Symbol / group | External C callers? | Final provider (Rust-enabled) | Notes | Req. linkage |
|---|---|---|---|---|
| `DoBattle()` | callback ABI only via `DoInput()` | existing C symbol rewritten to thin shell | Never replaced by wrapper file or parallel Rust export | §External symbol ABI preservation; §Build-mode coexistence |
| `Battle()` | yes (`encount.c`, `starcon.c`, `melee.c`) | `rust_battle_wrappers.c` | Original body guarded out | §External symbol ABI preservation |
| `computer_intelligence()` | yes (`battlecontrols.c`) | `rust_battle_wrappers.c` | Original body guarded out | §External symbol ABI preservation |
| `InitShips()` / `UninitShips()` | yes (`battle.c`) | `rust_battle_wrappers.c` | Original bodies guarded out | §External symbol ABI preservation |
| `InitSpace()` / `UninitSpace()` | yes (`setup.c`, `cleanup.c`, `melee.c`) | `rust_battle_wrappers.c` | External callers need wrappers | §External symbol ABI preservation |
| `BattleSong()` / `FreeBattleSong()` | yes / paired | `rust_battle_wrappers.c` | `BattleSong()` has external callers | §External symbol ABI preservation |
| `GetPlayerOrder()` | yes (`ship.c`, `pickmele.c`) | `rust_battle_wrappers.c` | Preserves caller-facing symbol | §External symbol ABI preservation |
| `ProcessInput()`, `RunAwayAllowed()`, `DoRunAway()`, `setupBattleInputOrder()`, `selectAllShips()`, `CountCrewElements()` | no external callers | Rust-owned internal replacement | No preserving wrapper needed | §Build-mode coexistence |
| Most `process.c` / `tactrans.c` helpers | no external callers | Rust-owned internal replacement | No preserving wrapper | §Build-mode coexistence |
| 10 retained boundary bodies | retained by design | retained C implementation | Always compiled in C | §Callback-slot safety; §Build-mode coexistence |

> *Req. linkage column:* References are to `battlept2/requirements.md` section headings that each symbol row satisfies.


### 5.3 Supported symbol/link modes

| Mode | `USE_RUST_BATTLE_LOOP` | `Battle()` provider | `DoBattle()` provider | Other ported symbols |
|------|------------------------|---------------------|----------------------|---------------------|
| C-only baseline | disabled | original C body | original C body | original C bodies |
| Pre-wiring mixed build | disabled | original C body | original C body | C authoritative; Rust test-only |
| Rust-enabled build | enabled | `rust_battle_wrappers.c` | retained C thin shell | Rust-owned under guard |

### 5.4 Wrapper-target export family

**[Normative]** The preserving-wrapper compilation unit (`rust_battle_wrappers.c`) calls a fixed C-callable export family:

| C wrapper symbol | Rust FFI export | Rust implementation |
|-----------------|----------------|-------------------|
| `Battle()` | `rust_battle_entry()` | `lifecycle.rs::battle()` |
| `InitShips()` | `rust_battle_init_ships()` | `lifecycle.rs::init_ships()` |
| `UninitShips()` | `rust_battle_uninit_ships()` | `lifecycle.rs::uninit_ships()` |
| `computer_intelligence()` | `rust_battle_computer_intelligence()` | `ai.rs::computer_intelligence()` |
| `BattleSong()` | `rust_battle_song()` | `lifecycle.rs::battle_song()` |
| `FreeBattleSong()` | `rust_battle_free_song()` | `lifecycle.rs::free_battle_song()` |
| `GetPlayerOrder()` | `rust_battle_get_player_order()` | `lifecycle.rs::get_player_order()` |

These export names are fixed by this specification and must be used consistently by `rust_battle_wrappers.c`, `ffi.rs`, and verification artifacts.

---

## 6. Integration model

### 6.1 Trait-based subsystem integration

All subsystem interactions go through the trait interfaces defined in `battle/integration.rs` (Phase 1). This specification uses two counting schemes:

1. **Conceptual integration operations** — requirement-level capabilities.
2. **Deferred bridge operations** — concrete Rust→C bridge wrappers completed by Phase 2/3.

### 6.2 Conceptual integration inventory

| Trait | Conceptual operations |
|-------|---------------------:|
| `GraphicsIntegration` | 17 |
| `AudioIntegration` | 11 |
| `ThreadingIntegration` | 3 |
| `InputIntegration` | 4 |
| `ResourceIntegration` | 5 |
| `ShipRaceIntegration` | 6 |
| `GlobalStateIntegration` | 4 |
| **Total** | **50** |

Of those, 6 were used by Phase 1 leaf functions. The remaining **44 deferred bridge operations** are completed by Phase 2/3:
- **1 early bridge**: `DrawablesIntersect`, needed immediately by `ProcessCollisions`.
- **43 remaining bridge operations**: wired in the C bridge phase.

---

## 7. Rust module structure (Phase 2/3 evolution)

### 7.1 Extended modules

Existing Phase 1 type-only modules are extended with behavioral logic. New modules are created only where justified.

> **Current vs. target names:** Three Phase 1 modules are renamed as Phase 2/3 begins. The current Phase 1 names are `process_types.rs`, `ship_runtime_types.rs`, and `ai_types.rs`. Each rename occurs as a dedicated rename-only commit at the start of its owning phase (P03, P07, P11 respectively) before any logic changes. The tree below shows the **Phase 2/3 target names** after rename.



```text
rust/src/battle/
  ## EXTENDED (Phase 1 types + Phase 2/3 orchestration logic)
  process_loop.rs           # WAS: process_types.rs (renamed)
                            # ADDS: PreProcess, PostProcess, ProcessCollisions,
                            #   PreProcessQueue, PostProcessQueue, RedrawQueue,
                            #   CalcReduction, CalcView, AllocElement, FreeElement,
                            #   SetUpElement, InsertPrim, Untarget, RemoveElement,
                            #   InitDisplayList, CALC_ZOOM_STUFF, CalcDisplayCoord

  ship_runtime.rs           # WAS: ship_runtime_types.rs (renamed)
                            # ADDS: ship_preprocess, ship_postprocess, ship_collision,
                            #   spawn_ship, inertial_thrust, GetNextStarShip,
                            #   GetInitialStarShips, animation_preprocess

  tactical.rs               # EXTENDS existing tactical.rs
                            # ADDS: ship_death, explosion_preprocess, cleanup_dead_ship,
                            #   new_ship, find_alive_starship, OpponentAlive,
                            #   flee_preprocess, ship_transition, cycle_ion_trail,
                            #   spawn_ion_trail, StartShipExplosion, DoRunAway,
                            #   PlayDitty, StopDitty, DittyPlaying, StopAllBattleMusic,
                            #   preprocess_dead_ship, RecordShipDeath,
                            #   ResetWinnerStarShip, GetWinnerStarShip, SetWinnerStarShip,
                            #   setMinShipLifeSpan, setMinStarShipLifeSpan,
                            #   checkOtherShipLifeSpan, readyForBattleEnd

  ai.rs                     # WAS: ai_types.rs (renamed)
                            # ADDS: computer_intelligence()

  lifecycle.rs              # EXTENDS existing lifecycle.rs
                            # ADDS: Battle(), ProcessInput, InitShips, UninitShips,
                            #   InitSpace/UninitSpace, CountCrewElements,
                            #   RunAwayAllowed, setupBattleInputOrder,
                            #   BattleSong, FreeBattleSong, selectAllShips,
                            #   GetPlayerOrder

  ## NEW
  c_bridge.rs               # Canonical Rust-to-C bridge layer — all 44 deferred
                            #   bridge operation wrappers

  ## EXTENDED (Phase 1 retained; additive)
  integration.rs            # RETAINS trait contracts
  ffi.rs                    # EXTENDS: retains 17 Phase 1 adapters, adds Phase 2/3
                            #   exports (rust_battle_redraw_queue, rust_battle_frame,
                            #   rust_battle_entry, rust_battle_init_ships, etc.)

  ## UNCHANGED (Phase 1 types used as-is)
  element.rs                # Element struct, ElementFlags, lifecycle helpers
  velocity.rs               # VelocityDesc, Bresenham accumulation
  collision.rs              # elastic_collide(), collision eligibility
  weapon.rs                 # weapon_collision, blast creation, tracking
  display_list.rs           # Pool allocator, generational handles, linked-list ops
  battle_types.rs           # Coords, angles, trig
  netplay.rs                # CRC-32, crc_process_element
  mod.rs                    # Module declarations, re-exports
```

### 7.2 C files modified (guarded)

| C File | Change |
|--------|--------|
| `sc2/src/uqm/process.c` | Guard function bodies behind `#ifndef USE_RUST_BATTLE_LOOP`; add `extern` declarations for Rust replacements |
| `sc2/src/uqm/battle.c` | Guard `Battle()`, `ProcessInput()`, `RunAwayAllowed()`, `DoRunAway()`, `setupBattleInputOrder()`, `BattleSong()`, `FreeBattleSong()`, `selectAllShips()`, `GetPlayerOrder()` behind `#ifndef USE_RUST_BATTLE_LOOP`; rewrite `DoBattle()` to thin shell |
| `sc2/src/uqm/tactrans.c` | Guard 24 ported function bodies behind `#ifndef USE_RUST_BATTLE_LOOP`; leave 6 permanent-C-boundary functions compiled |
| `sc2/src/uqm/intel.c` | Guard `computer_intelligence()` behind `#ifndef USE_RUST_BATTLE_LOOP` |
| `sc2/src/uqm/ship.c` | Guard ship runtime functions behind `#ifndef USE_RUST_BATTLE_LOOP` (additional to `USE_RUST_SHIPS`) |
| `sc2/src/uqm/init.c` | Guard init/uninit functions behind `#ifndef USE_RUST_BATTLE_LOOP` (additional to `USE_RUST_SHIPS`) |
| `sc2/build/unix/build.config` | Add `USE_RUST_BATTLE_LOOP` toggle |
| `sc2/config_unix.h` | Add `#define USE_RUST_BATTLE_LOOP` |

---

## 8. Callback-slot migration model

### 8.1 Callback-slot migration matrix

**[Normative]** Every callback family touched by battle code has explicit migration rules:

| Callback family | Slot owner | C-only target | Rust-owned target | Earliest live replacement |
|----------------|-----------|--------------|------------------|--------------------------|
| `Element.preprocess_func` | C `ELEMENT` struct field | retained C callback | Rust `extern "C"` or boundary shim | Supported live runtime only after FFI wiring |
| `Element.postprocess_func` | C `ELEMENT` struct field | retained C callback | Rust `extern "C"` or boundary shim | Supported live runtime only after FFI wiring |
| `Element.collision_func` | C `ELEMENT` struct field | retained C callback | Rust `extern "C"` or boundary shim | Supported live runtime only after FFI wiring |
| `Element.death_func` | C `ELEMENT` struct field | retained C callback | Rust `extern "C"` or boundary shim | Supported live runtime only after FFI wiring |
| `PlayerInput[]->handlers->frameInput` | C handler/vtable slot | retained C callback | retained C callback | Never replaced by this plan |
| `PlayerInput[]->handlers->battleEndReady` | C handler/vtable slot | retained C callback | retained C callback | Never replaced by this plan |

### 8.2 Callback installation rules

**[Normative]**

- In C-only builds, callback-bearing slots keep their original retained C targets.
- In Rust-owned callback paths, the slot is updated to either a Rust `extern "C"` callback or a documented boundary shim that immediately dispatches into Rust-owned logic.
- Rust-only closures, enums, or trait objects may NOT be hidden behind C-ABI callback-pointer fields.
- Any path that can invoke C callbacks or Rust-owned callback replacements must use handle-based traversal and staged re-lookups after every callback/re-entrant boundary. No mutable borrow, raw pointer assumption, or cached element/queue/display-list location may survive across a callback that can mutate battle state.
- No lock may be held across a callback into C if C may re-enter the battle loop or mutate the same intrusive list.

### 8.3 Reuse/free callback-slot protocol

**[Normative]** When an element or queue entry is reused, rebound, cleared, or freed, all callback-bearing fields and back-references touched by that transition must be rewritten or cleared in the same state transition before the object can be observed again through foreign storage.

---

## 9. Guard/toggle staging strategy

### 9.1 Dark-code stage

**[Normative]** Phases before the FFI wiring phase create Rust-side implementations and may add local/test-only plumbing needed for targeted verification, but do not expose a supported global `USE_RUST_BATTLE_LOOP` runtime switch. Until the FFI wiring phase completes, normal battle execution remains on the original C-owned runtime path, and any Rust process-loop or lifecycle/input entrypoints are dark-code/test/integration surfaces only.

### 9.2 Wiring stage

**[Normative]** The FFI wiring phase is the first phase that adds the public `USE_RUST_BATTLE_LOOP` toggle, guards the remaining C files, rewrites the existing `DoBattle()` body into the thin shell, and makes the full runtime ownership flip buildable and supported.

### 9.3 Pre-wiring execution-surface rule

**[Normative]** Before the FFI wiring phase, Rust battle entrypoints are allowed only for targeted tests, fixtures, or integration harnesses that explicitly keep the normal runtime battle loop C-owned. They are not a supported partial-runtime owner, may not replace live callback registration, and must run only under a documented harness/integration ownership model.

---

## 10. FFI safety contract

### 10.1 Panic containment

**[Normative]** No Rust panic may cross an FFI boundary. All Rust exports use `catch_unwind` or equivalent containment, converting failure to deterministic error/abort behavior.

### 10.2 Pointer safety

**[Normative]** `ELEMENT*`, `STARSHIP*`, display-primitive pointers, and queue-entry pointers are borrowed only unless an API explicitly transfers ownership. Borrowed C pointers may not be cached across frame boundaries unless the plan explicitly documents a stable backing allocation.

### 10.3 Pointer-family safety categories

**[Normative]** All FFI wrapper pointer arguments must be classified into one of these categories:

- **Always-nonnull borrowed battle-state pointers:** Active `BATTLE_STATE*`, `ELEMENT*`, `STARSHIP*`, and queue-entry pointers from verified internal call paths. May be treated as preconditions after wrapper-boundary assertion, but may not outlive the documented borrow/relookup scope.
- **Nullable foreign handles/optional references:** Any pointer/handle documented by the C API as optional, sentinel-capable, or absent-by-state must be checked at the wrapper boundary before dereference.
- **Callback/re-entrant invalidation-sensitive pointers:** Any foreign pointer that can be invalidated by callback dispatch, free/reuse, unlink/relink, or owner-transition paths must be re-looked-up from stable handle identity after each such boundary.

### 10.4 Thread affinity

**[Normative]** Battle-loop state is single-thread-affine to the `DoInput` execution thread. Battle-state and display-list internals are not required to be `Send`/`Sync` for this port. FFI wrappers must document thread-affinity assumptions: frame-loop calls are main-thread / DoInput-thread only unless a specific function is declared thread-safe.

### 10.5 Stable identity model

**[Normative]** Rust-owned battle logic treats Rust display-list handles / queue handles as the only stable identities across callback/re-entrant boundaries. Foreign raw pointers into C-managed elements, queue entries, display primitives, or intrusive links are transient views only. Any callback, wrapper, or C-side helper that can free, reuse, detach, retarget, or relink foreign objects invalidates previously observed raw-pointer/list-position assumptions; subsequent access must restart from stable handle identity plus a fresh staged lookup.

---

## 11. Phase 1 artifacts used (not re-implemented)

**[Reference design]** Phase 2/3 builds on the following Phase 1 modules:

| Phase 1 Module | Used By |
|---------------|---------|
| `element.rs` — Element struct, ElementFlags, lifecycle helpers, `commit_state()`, `is_collidable()` | Process loop, ship runtime, tactical transitions |
| `velocity.rs` — VelocityDesc, `get_next_components()`, `set_vector()`, `set_components()`, `delta_components()`, `zero()` | Process loop (velocity stepping), ship runtime (thrust) |
| `collision.rs` — `elastic_collide()`, `collision_possible()`, `isqrt()` | ProcessCollisions (post-dispatch elastic response) |
| `weapon.rs` — `weapon_collision()`, `track_ship()`, `do_damage()`, LaserBlock, MissileBlock | Ship postprocess (weapon firing), collision callbacks |
| `display_list.rs` — DisplayList pool, alloc/free/push_back/remove/iter, GenerationalHandle | Process loop (element management), all display list traversals |
| `battle_types.rs` — coords, angles, trig, SINE_TABLE | Process loop (coordinate transforms), ship runtime (facing/thrust) |
| `netplay.rs` — CrcState, `crc_process_element()`, CRC table | Frame checksum computation |
| `process_loop.rs` — ViewState, ZoomMode, zoom/camera constants | Process loop (zoom/camera calculation) |
| `lifecycle.rs` — BattleState, BATTLE_FRAME_RATE | Battle lifecycle |
| `tactical.rs` — DeathPipelinePhase, explosion/flee/warp constants | Tactical transitions |
| `ai.rs` — EvaluateDesc, control flags, AI constants | AI dispatch |
| `ship_runtime.rs` — ShipPipelineStage, spawn constants | Ship runtime |
| `integration.rs` — All 7 trait interfaces | All orchestration modules |

---

## 12. Master function inventory

**[Normative]** This section is the authoritative reconciliation of all 75 C functions in Phase 2/3 scope. Every function appears exactly once. Combined with the permanent C boundary list in §3.1, a reader can confirm that all functions are either "Ported in Phase X" or "Retained as permanent C boundary." The implementation plan references this table and provides per-phase assignment summaries for sequencing.

### 12.1 Master function inventory table

| # | C File | Function | Status | Phase | Rust Target Module | Notes |
|--:|--------|----------|--------|------:|-------------------|-------|
| 1 | `battle.c` | `RunAwayAllowed()` | Ported | P12 | `lifecycle.rs` | 3-condition eligibility check |
| 2 | `battle.c` | `DoRunAway()` | Ported | P10 | `tactical.rs` | Initiates flee sequence |
| 3 | `battle.c` | `setupBattleInputOrder()` | Ported | P12 | `lifecycle.rs` | Configures per-side input processing order |
| 4 | `battle.c` | `frameInputHuman()` | Retained | — | — | DoInput callback; one-liner delegate (§3.1.2) |
| 5 | `battle.c` | `ProcessInput()` | Ported | P12 | `lifecycle.rs` | Per-frame input mapping + escape detection |
| 6 | `battle.c` | `BattleSong()` | Ported | P12 | `lifecycle.rs` | Loads battle music |
| 7 | `battle.c` | `FreeBattleSong()` | Ported | P12 | `lifecycle.rs` | Frees battle music |
| 8 | `battle.c` | `DoBattle()` | Retained | P13 | — | Retained ABI shell; rewritten to thin shell calling `rust_battle_frame()` (§4) |
| 9 | `battle.c` | `GetPlayerOrder()` | Ported | P12 | `lifecycle.rs` | Determines player turn order |
| 10 | `battle.c` | `selectAllShips()` | Ported | P12 | `lifecycle.rs` | Auto-selects all ships for a side |
| 11 | `battle.c` | `Battle()` | Ported | P12 | `lifecycle.rs` | Top-level battle entry/exit |
| 12 | `process.c` | `CalcReduction()` | Ported | P05 | `process_loop.rs` | Step/continuous zoom calculation |
| 13 | `process.c` | `CalcView()` | Ported | P05 | `process_loop.rs` | CALC_ZOOM_STUFF — camera midpoint, clamping |
| 14 | `process.c` | `AllocElement()` | Ported | P03 | `process_loop.rs` | Element + display prim allocation |
| 15 | `process.c` | `FreeElement()` | Ported | P03 | `process_loop.rs` | Element + display prim deallocation |
| 16 | `process.c` | `SetUpElement()` | Ported | P03 | `process_loop.rs` | Element field initialization after alloc |
| 17 | `process.c` | `InsertPrim()` | Ported | P05 | `process_loop.rs` | Rendering-order insertion |
| 18 | `process.c` | `Untarget()` | Ported | P03 | `process_loop.rs` | Clears tracking target refs to removed element |
| 19 | `process.c` | `RemoveElement()` | Ported | P03 | `process_loop.rs` | Display list removal + Untarget |
| 20 | `process.c` | `PreProcess()` | Ported | P03 | `process_loop.rs` | Per-element preprocess pass |
| 21 | `process.c` | `ProcessCollisions()` | Ported | P04 | `process_loop.rs` | Recursive collision orchestration |
| 22 | `process.c` | `PostProcess()` | Ported | P03 | `process_loop.rs` | Per-element postprocess pass |
| 23 | `process.c` | `CalcDisplayCoord()` | Ported | P05 | `process_loop.rs` | World→screen coordinate conversion |
| 24 | `process.c` | `PreProcessQueue()` | Ported | P05 | `process_loop.rs` | Top-level preprocess queue iteration |
| 25 | `process.c` | `PostProcessQueue()` | Ported | P05 | `process_loop.rs` | Top-level postprocess queue iteration |
| 26 | `process.c` | `InitDisplayList()` | Ported | P05 | `process_loop.rs` | Display list reset at battle start |
| 27 | `process.c` | `RedrawQueue()` | Ported | P05 | `process_loop.rs` | Top-level frame: preprocess→postprocess→render |
| 28 | `process.c` | `InitKernel()` | Ported | P05 | `process_loop.rs` | Graphics kernel initialization |
| 29 | `tactrans.c` | `OpponentAlive()` | Ported | P10 | `tactical.rs` | Display-list iteration alive check (3 return cases) |
| 30 | `tactrans.c` | `PlayDitty()` | Ported | P09 | `tactical.rs` | Victory music start |
| 31 | `tactrans.c` | `StopDitty()` | Ported | P09 | `tactical.rs` | Victory music stop |
| 32 | `tactrans.c` | `DittyPlaying()` | Ported | P09 | `tactical.rs` | Victory music playing check |
| 33 | `tactrans.c` | `ResetWinnerStarShip()` | Ported | P10 | `tactical.rs` | Clears winner state |
| 34 | `tactrans.c` | `readyToEnd2Callback()` | Retained | — | — | Netplay end-phase-2 callback (§3.1.3) |
| 35 | `tactrans.c` | `readyToEndCallback()` | Retained | — | — | Netplay end-phase-1 callback (§3.1.3) |
| 36 | `tactrans.c` | `readyForBattleEndPlayer()` | Retained | — | — | Multi-phase netplay readiness (§3.1.3) |
| 37 | `tactrans.c` | `battleEndReadyHuman()` | Retained | — | — | DoInput callback for human end-ready (§3.1.2) |
| 38 | `tactrans.c` | `battleEndReadyComputer()` | Retained | — | — | DoInput callback for AI end-ready (§3.1.2) |
| 39 | `tactrans.c` | `battleEndReadyNetwork()` | Retained | — | — | DoInput callback for network end-ready (§3.1.2) |
| 40 | `tactrans.c` | `readyForBattleEnd()` | Ported | P09 | `tactical.rs` | Battle-end readiness orchestration |
| 41 | `tactrans.c` | `preprocess_dead_ship()` | Ported | P09 | `tactical.rs` | Dead ship preprocess stub |
| 42 | `tactrans.c` | `cleanup_dead_ship()` | Ported | P09 | `tactical.rs` | Explosion cleanup + crew preservation |
| 43 | `tactrans.c` | `setMinShipLifeSpan()` | Ported | P09 | `tactical.rs` | Minimum lifespan enforcement |
| 44 | `tactrans.c` | `setMinStarShipLifeSpan()` | Ported | P09 | `tactical.rs` | Minimum starship lifespan |
| 45 | `tactrans.c` | `checkOtherShipLifeSpan()` | Ported | P09 | `tactical.rs` | Winner kept alive one frame longer |
| 46 | `tactrans.c` | `new_ship()` | Ported | P09 | `tactical.rs` | Death→new ship replacement handler |
| 47 | `tactrans.c` | `explosion_preprocess()` | Ported | P09 | `tactical.rs` | 36-frame explosion animation |
| 48 | `tactrans.c` | `StopAllBattleMusic()` | Ported | P09 | `tactical.rs` | Stops all battle audio |
| 49 | `tactrans.c` | `FindAliveStarShip()` | Ported | P10 | `tactical.rs` | Display-list search for alive ship |
| 50 | `tactrans.c` | `GetWinnerStarShip()` | Ported | P10 | `tactical.rs` | Returns recorded winner |
| 51 | `tactrans.c` | `SetWinnerStarShip()` | Ported | P10 | `tactical.rs` | Records winner (once only) |
| 52 | `tactrans.c` | `RecordShipDeath()` | Ported | P09 | `tactical.rs` | Decrements battle counter |
| 53 | `tactrans.c` | `StartShipExplosion()` | Ported | P09 | `tactical.rs` | Initiates explosion: zero vel, drain energy, set life=36 |
| 54 | `tactrans.c` | `ship_death()` | Ported | P09 | `tactical.rs` | Top-level death entry: stops music, finds winner |
| 55 | `tactrans.c` | `cycle_ion_trail()` | Ported | P09 | `tactical.rs` | 12-color ion trail fade |
| 56 | `tactrans.c` | `spawn_ion_trail()` | Ported | P09 | `tactical.rs` | Creates ion trail point element |
| 57 | `tactrans.c` | `ship_transition()` | Ported | P10 | `tactical.rs` | 15-frame warp ghost animation |
| 58 | `tactrans.c` | `flee_preprocess()` | Ported | P10 | `tactical.rs` | Flee 20-color pulse + warp-out |
| 59 | `ship.c` | `animation_preprocess()` | Ported | P07 | `ship_runtime.rs` | Frame advance, CHANGING flag |
| 60 | `ship.c` | `inertial_thrust()` | Ported | P07 | `ship_runtime.rs` | Inertial/inertialess/gravity movement |
| 61 | `ship.c` | `ship_preprocess()` | Ported | P07 | `ship_runtime.rs` | 7-stage per-frame pipeline |
| 62 | `ship.c` | `ship_postprocess()` | Ported | P07 | `ship_runtime.rs` | Weapon firing + race postprocess |
| 63 | `ship.c` | `collision()` (ship) | Ported | P07 | `ship_runtime.rs` | Ship-specific collision (gravity damage) |
| 64 | `ship.c` | `spawn_ship()` | Ported | P08 | `ship_runtime.rs` | Ship element allocation + placement |
| 65 | `ship.c` | `GetNextStarShip()` | Ported | P08 | `ship_runtime.rs` | Queue traversal + infinite fleet recycling |
| 66 | `ship.c` | `GetInitialStarShips()` | Ported | P08 | `ship_runtime.rs` | Initial ship selection for all sides |
| 67 | `init.c` | `load_animation()` | Retained | — | — | Resource subsystem internal (§3.1.4) |
| 68 | `init.c` | `free_image()` | Retained | — | — | Resource subsystem internal (§3.1.4) |
| 69 | `init.c` | `InitSpace()` | Ported | P12 | `lifecycle.rs` | Ref-counted space asset init |
| 70 | `init.c` | `UninitSpace()` | Ported | P12 | `lifecycle.rs` | Ref-counted space asset teardown |
| 71 | `init.c` | `BuildSIS()` | Retained | — | — | Ships subsystem (fleet management) (§3.1.4) |
| 72 | `init.c` | `InitShips()` | Ported | P12 | `lifecycle.rs` | Full ship initialization sequence |
| 73 | `init.c` | `CountCrewElements()` | Ported | P12 | `lifecycle.rs` | Counts floating crew pickups |
| 74 | `init.c` | `UninitShips()` | Ported | P12 | `lifecycle.rs` | Full ship teardown sequence |
| 75 | `intel.c` | `computer_intelligence()` | Ported | P11 | `ai.rs` | 4-path AI dispatch entry point |

### 12.2 Inventory summary

| Category | Count |
|----------|------:|
| Ported to Rust | 64 |
| Retained as permanent C boundary | 11 |
| **Total** | **75** |

| Retained boundary category (from §3.1) | Count |
|----------------------------------------|------:|
| Semantically C-owned retained boundary bodies | 10 |
| Retained C ABI shell with Rust semantics behind it (`DoBattle`) | 1 |
| **Total retained** | **11** |

### 12.3 Per-phase porting summary

| Phase | Ported functions | Rust target module(s) |
|------:|-----------------:|----------------------|
| P03 | 7 | `process_loop.rs` — PreProcess, PostProcess, AllocElement, FreeElement, SetUpElement, Untarget, RemoveElement |
| P04 | 1 | `process_loop.rs` — ProcessCollisions |
| P05 | 9 | `process_loop.rs` — CalcReduction, CalcView, InsertPrim, CalcDisplayCoord, PreProcessQueue, PostProcessQueue, InitDisplayList, RedrawQueue, InitKernel |
| P07 | 5 | `ship_runtime.rs` — animation_preprocess, inertial_thrust, ship_preprocess, ship_postprocess, collision(ship) |
| P08 | 3 | `ship_runtime.rs` — spawn_ship, GetNextStarShip, GetInitialStarShips |
| P09 | 17 | `tactical.rs` — ship_death, StartShipExplosion, explosion_preprocess, cleanup_dead_ship, new_ship, + 12 helpers |
| P10 | 8 | `tactical.rs` — flee_preprocess, ship_transition, DoRunAway, FindAliveStarShip, OpponentAlive, + 3 winner helpers |
| P11 | 1 | `ai.rs` — computer_intelligence |
| P12 | 13 | `lifecycle.rs` — Battle, InitShips, UninitShips, InitSpace, UninitSpace, ProcessInput, + 7 helpers |
| **Total** | **64** | |



## 13. Branch-parity obligations

### 13.1 Required branch families

**[Normative]** The following compile-time and runtime branches in the six C source files must remain behaviorally identical in the Rust port. See `battlept2/requirements.md` for the EARS-format behavioral requirements.

| Branch family | Source sites | Behavioral impact |
|--------------|-------------|-------------------|
| `NETPLAY` / `NETPLAY_CHECKSUM` | battle.c, tactrans.c, process.c | Frame sync, CRC, battle-end protocol |
| `DEMO_MODE` / `CREATE_JOURNAL` | battle.c, process.c | Demo recording/playback |
| `SUPER_MELEE` | battle.c, tactrans.c | Abort handling, ship-death notification |
| `CHECK_ABORT` / `CHECK_LOAD` | battle.c, init.c | Cleanup paths from lifecycle code |
| `IN_ENCOUNTER` / `IN_LAST_BATTLE` | init.c, tactrans.c, battle.c | Environment setup, flee eligibility, teardown |
| `inHyperSpace()` / `inQuasiSpace()` | init.c, battle.c | Music selection, init paths, single-ship spawn |
| Max-speed rendering skip | battle.c, process.c | Conditional rendering suppression |

### 13.2 Verification obligation

Each implementation phase that touches a branch family must record which compile/config cases were exercised in its verification output, identifying each branch side or configuration tested, or explicitly marking it as permanent-C-boundary-owned.
