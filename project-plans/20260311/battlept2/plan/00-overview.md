# Plan: Battle Engine — Phase 2/3 Full Logic Port

Plan ID: PLAN-20260320-BATTLEPT2
Generated: 2026-03-20
Predecessor: PLAN-20260320-BATTLE (Phase 1 — types + leaf functions, COMPLETE, 2139 tests)
Total Phases: 29 (P00.5 through P14a, with verification sub-phases)
Requirements: All deferred "Phase 2+" items from PLAN-20260320-BATTLE requirements traceability
Specification: `project-plans/20260311/battle/specification.md` §1–§18

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 0.5)
2. Integration points are explicitly listed
3. TDD cycle is defined per slice
4. Lint/test/coverage gates are declared
5. Phase 1 types and leaf functions (7,591 lines of existing Rust code, 2,139 tests total / 246 battle-specific) are the foundation — use them, do not redefine them

## Context

Phase 1 (PLAN-20260320-BATTLE) created Rust types and leaf math functions that C calls via FFI. All 37 phases passed. 2,139 tests. Phase 1 created these modules in `rust/src/battle/`:

| Module | Lines | Contents |
|--------|------:|----------|
| `battle_types.rs` | 398 | Coords, angles, trig, SINE_TABLE |
| `element.rs` | 943 | Element #[repr(C)], ElementFlags, lifecycle helpers |
| `velocity.rs` | 682 | VelocityDesc, Bresenham accumulation suite |
| `display_list.rs` | 899 | Pool allocator, generational handles, linked-list ops |
| `collision.rs` | 558 | elastic_collide(), isqrt, eligibility checks |
| `weapon.rs` | 949 | LaserBlock, MissileBlock, weapon_collision, blast creation, tracking |
| `process_types.rs` *(renamed to `process_loop.rs` in P03)* | 95 | ViewState, ZoomMode, zoom/camera constants |
| `lifecycle.rs` | 123 | BattleState type, frame rate constants |
| `ship_runtime_types.rs` *(renamed to `ship_runtime.rs` in P07)* | 128 | ShipPipelineStage, spawn constants |
| `tactical.rs` | 187 | Death pipeline enum, explosion/flee/warp constants |
| `ai_types.rs` *(renamed to `ai.rs` in P11)* | 141 | EvaluateDesc, AI constants, control flags |
| `netplay.rs` | 586 | CRC-32, crc_process_element(), protocol type defs |
| `integration.rs` | 860 | 7 trait interfaces (Graphics, Audio, Threading, Input, Resource, ShipRace, GlobalState) |
| `ffi.rs` | 510 | 15 Phase 1 FFI adapters |
| `mod.rs` | 532 | Module declarations, re-exports, integration tests |
| **Total** | **7,591** | |

### Phase 1 FFI Adapters (Extended or Replaced by This Plan)

The following 15 Phase 1 FFI adapters exist in `ffi.rs` and will continue to function unchanged. New Phase 2/3 FFI exports (P06, P13) are additive — they do not replace these:

| FFI Adapter | Phase 1 Purpose |
|-------------|----------------|
| `rust_velocity_get_current_components` | Velocity query |
| `rust_velocity_get_next_components` | Velocity stepping |
| `rust_velocity_set_vector` | Velocity set by facing/magnitude |
| `rust_velocity_set_components` | Velocity set by dx/dy |
| `rust_velocity_delta_components` | Velocity delta accumulation |
| `rust_velocity_zero` | Velocity zero |
| `rust_velocity_is_zero` | Velocity zero check |
| `rust_battle_collide` | Elastic collision dispatch |
| `rust_battle_weapon_collision` | Weapon collision dispatch |
| `rust_battle_compute_blast_direction` | Blast direction from target facing |
| `rust_battle_track_facing` | Weapon tracking |
| `rust_battle_crc_init` | CRC state init |
| `rust_battle_crc_process_element` | CRC element processing |
| `rust_battle_crc_finish` | CRC finalize |
| `rust_battle_sine` / `rust_battle_cosine` / `rust_battle_arctan` | Trig lookups |

## Permanent C Boundary — Functions Not Ported

The following C functions are **explicitly NOT ported** by this plan. Each remains in C with justification. This section, combined with the per-phase assignment tables below, ensures every C function in scope is either "ported in Phase X" or "permanent C boundary."

> **Note:** "Permanent C Boundary" means the C function *body* stays in C — Rust does not reimplement the logic. However, Rust may still *call* these functions via FFI (e.g., Rust's `ready_for_battle_end()` calls into C-side netplay callbacks). The "Rust Calls Instead" column in each table documents these FFI call paths.

### DoInput Framework Callbacks

These functions are thin callbacks registered via C function pointers in the `DoInput()` cooperative scheduling framework. The `DoInput` framework is not battle-specific — it is the engine-wide cooperative polling loop. Porting these callbacks provides no value until the `DoInput` framework itself moves to Rust (which is outside the scope of any current plan).

| C Function | Source File | Lines | Justification | Rust Calls Instead |
|-----------|------------|------:|---------------|-------------------|
| `frameInputHuman()` | `battle.c` | :137-142 | DoInput framework callback — one-liner that delegates to `CurrentInputToBattleInput()` through the existing input subsystem. Porting it provides no value until the DoInput framework itself moves to Rust. | Not called from Rust; C calls directly via `PlayerInput[]->handlers->frameInput` function pointer. |
| `DoBattle()` | `battle.c` | :258-354 | DoInput `InputFunc` callback — must match C function pointer layout. Becomes a 5-line thin shell that calls `rust_battle_frame()`. The frame callback architecture (§9.2) requires a C function pointer for the cooperative polling loop. | C shell calls `rust_battle_frame()` (Rust FFI export, P13). |
| `battleEndReadyHuman()` | `tactrans.c` | :231-236 | DoInput-pattern callback registered in `PlayerInput[]->handlers->battleEndReady`. One-liner that blocks until readiness conditions are met via `DoInput()` cooperative scheduling. Stays in C until the DoInput framework moves to Rust. | Not called from Rust; C calls directly via input handler vtable. |
| `battleEndReadyComputer()` | `tactrans.c` | :239-243 | DoInput-pattern callback — same as `battleEndReadyHuman()` but for AI players. One-liner delegate. | Not called from Rust; C calls directly via input handler vtable. |
| `battleEndReadyNetwork()` | `tactrans.c` | :246-250 | DoInput-pattern callback — same pattern for network players. One-liner delegate. | Not called from Rust; C calls directly via input handler vtable. |

### Netplay Transport Callbacks

These functions implement the netplay battle-end synchronization protocol. They are registered as C function pointers in the netplay connection subsystem and called from the netplay transport layer, which is entirely out of scope. They use `DoInput()` cooperative scheduling internally.

| C Function | Source File | Lines | Justification | Rust Calls Instead |
|-----------|------------|------:|---------------|-------------------|
| `readyToEnd2Callback()` | `tactrans.c` | :109-114 | Netplay end-phase-2 callback — registered via C function pointer in the netplay connection subsystem. Called from netplay transport layer (out of scope). Uses `DoInput()` cooperative scheduling. | Rust's `ready_for_battle_end()` (P09) calls through to C-side `readyForBattleEnd()` which dispatches to these callbacks for netplay builds. |
| `readyToEndCallback()` | `tactrans.c` | :117-150 | Netplay end-phase-1 callback — same pattern. Implements the `inBattle → endingBattle` protocol transition state machine with `DoInput()` polling. | Same as above — reached via Rust→C FFI dispatch chain. |
| `readyForBattleEndPlayer()` | `tactrans.c` | :169-228 | Netplay per-player battle-end readiness check — orchestrates the multi-phase protocol (`inBattle → endingBattle → endingBattle2 → interBattle`) using `DoInput()` cooperative scheduling and netplay connection state. | Same as above — reached via Rust→C FFI dispatch chain. |

### Resource / Ships Subsystem Internals

These functions belong to subsystems outside the battle engine's scope. They are called by battle lifecycle code (which IS ported) via FFI.

| C Function | Source File | Lines | Justification | Rust Calls Instead |
|-----------|------------|------:|---------------|-------------------|
| `load_animation()` | `init.c` | :49-77 | Resource subsystem internal — uses `LoadGraphic`/`CaptureDrawable` which are deep in the graphics resource layer (USE_RUST_RESOURCE scope, not USE_RUST_BATTLE_LOOP scope). | Rust `init_space()` / `init_ships()` (P12) calls `load_animation()` via FFI through `ResourceIntegration` trait. |
| `free_image()` | `init.c` | :81-113 | Resource subsystem internal — uses `DestroyDrawable`/`ReleaseDrawable` which are graphics resource layer internals. Same scope boundary as `load_animation()`. | Rust `uninit_space()` / `uninit_ships()` (P12) calls `free_image()` via FFI through `ResourceIntegration` trait. |
| `BuildSIS()` | `init.c` | :164-179 | Ships subsystem concern — uses `Build()` from `build.c` to set up SIS-specific player info (flagship fleet construction). This is fleet management, not battle logic. | Rust `init_ships()` (P12) calls `BuildSIS()` via FFI for the hyperspace initialization path. |

### Boundary Summary

| Category | Count | Functions |
|----------|------:|-----------|
| DoInput framework callbacks | 5 | `frameInputHuman`, `DoBattle`, `battleEndReadyHuman/Computer/Network` |
| Netplay transport callbacks | 3 | `readyToEnd2Callback`, `readyToEndCallback`, `readyForBattleEndPlayer` |
| Resource/ships subsystem internals | 3 | `load_animation`, `free_image`, `BuildSIS` |
| **Total** | **11** | |

After this plan completes, C retains only these 11 functions — everything inside the battle frame is Rust-owned.

## What This Plan Ports

This plan ports **ALL remaining battle engine logic** from C to Rust. After completion, C retains only the 11 functions listed in the Permanent C Boundary section above — everything inside the frame is Rust-owned.

### Complete C Function Inventory — battle.c

Every function in `battle.c` is explicitly assigned below.

| C Function | Lines | Status | Rust Target | Phase |
|-----------|------:|--------|-------------|-------|
| `RunAwayAllowed()` | :63-70 | **Port** | `lifecycle.rs` | P12 |
| `DoRunAway()` | :72-105 | **Port** | `tactical.rs` | P10 |
| `setupBattleInputOrder()` | :107-135 | **Port** | `lifecycle.rs` | P12 |
| `frameInputHuman()` | :137-142 | **Stays in C** | — | — |
| `ProcessInput()` | :144-226 | **Port** | `lifecycle.rs` | P12 |
| `BattleSong()` | :234-249 | **Port** | `lifecycle.rs` | P12 |
| `FreeBattleSong()` | :251-256 | **Port** | `lifecycle.rs` | P12 |
| `DoBattle()` | :258-354 | **Stays in C** | — (thin shell) | P13 |
| `GetPlayerOrder()` | :357-372 | **Port** | `lifecycle.rs` | P12 |
| `selectAllShips()` | :375-394 | **Port** | `lifecycle.rs` | P12 |
| `Battle()` | :396-516 | **Port** | `lifecycle.rs` | P12 |

**Justifications for functions staying in C:** See "Permanent C Boundary" section above for full justifications. Both are DoInput framework callbacks that must match C function pointer layout.

### Complete C Function Inventory — tactrans.c

Every function in `tactrans.c` is explicitly assigned below.

| C Function | Lines | Status | Rust Target | Phase |
|-----------|------:|--------|-------------|-------|
| `OpponentAlive()` | :54-75 | **Port** | `tactical.rs` | P10 |
| `PlayDitty()` | :77-82 | **Port** | `tactical.rs` | P09 |
| `StopDitty()` | :84-90 | **Port** | `tactical.rs` | P09 |
| `DittyPlaying()` | :92-100 | **Port** | `tactical.rs` | P09 |
| `ResetWinnerStarShip()` | :102-106 | **Port** | `tactical.rs` | P10 |
| `readyToEnd2Callback()` | :109-114 | **Stays in C** | — | — |
| `readyToEndCallback()` | :117-150 | **Stays in C** | — | — |
| `readyForBattleEndPlayer()` | :169-228 | **Stays in C** | — | — |
| `battleEndReadyHuman()` | :231-236 | **Stays in C** | — | — |
| `battleEndReadyComputer()` | :239-243 | **Stays in C** | — | — |
| `battleEndReadyNetwork()` | :246-250 | **Stays in C** | — | — |
| `readyForBattleEnd()` | :254-278 | **Port** | `tactical.rs` | P09 |
| `preprocess_dead_ship()` | :280-285 | **Port** | `tactical.rs` | P09 |
| `cleanup_dead_ship()` | :287-374 | **Port** | `tactical.rs` | P09 |
| `setMinShipLifeSpan()` | :376-387 | **Port** | `tactical.rs` | P09 |
| `setMinStarShipLifeSpan()` | :389-397 | **Port** | `tactical.rs` | P09 |
| `checkOtherShipLifeSpan()` | :399-437 | **Port** | `tactical.rs` | P09 |
| `new_ship()` | :441-540 | **Port** | `tactical.rs` | P09 |
| `explosion_preprocess()` | :542-616 | **Port** | `tactical.rs` | P09 |
| `StopAllBattleMusic()` | :618-623 | **Port** | `tactical.rs` | P09 |
| `FindAliveStarShip()` | :625-659 | **Port** | `tactical.rs` | P10 |
| `GetWinnerStarShip()` | :661-665 | **Port** | `tactical.rs` | P10 |
| `SetWinnerStarShip()` | :667-680 | **Port** | `tactical.rs` | P10 |
| `RecordShipDeath()` | :682-700 | **Port** | `tactical.rs` | P09 |
| `StartShipExplosion()` | :702-727 | **Port** | `tactical.rs` | P09 |
| `ship_death()` | :729-749 | **Port** | `tactical.rs` | P09 |
| `cycle_ion_trail()` | :755-789 | **Port** | `tactical.rs` | P09 |
| `spawn_ion_trail()` | :791-849 | **Port** | `tactical.rs` | P09 |
| `ship_transition()` | :854-961 | **Port** | `tactical.rs` | P10 |
| `flee_preprocess()` | :963-1033 | **Port** | `tactical.rs` | P10 |

**Justifications for functions staying in C:** See "Permanent C Boundary" section above for full justifications. Netplay transport callbacks and DoInput-pattern battle-end readiness callbacks — all registered via C function pointers in subsystems outside battle scope.

### Complete C Function Inventory — ship.c

Every function in `ship.c` is explicitly assigned below.

| C Function | Lines | Status | Rust Target | Phase |
|-----------|------:|--------|-------------|-------|
| `animation_preprocess()` | :46-58 | **Port** | `ship_runtime.rs` | P07 |
| `inertial_thrust()` | :61-153 | **Port** | `ship_runtime.rs` | P07 |
| `ship_preprocess()` | :155-290 | **Port** | `ship_runtime.rs` | P07 |
| `ship_postprocess()` | :292-364 | **Port** | `ship_runtime.rs` | P07 |
| `collision()` (ship) | :366-391 | **Port** | `ship_runtime.rs` | P07 |
| `spawn_ship()` | :393-515 | **Port** | `ship_runtime.rs` | P08 |
| `GetNextStarShip()` | :518-552 | **Port** | `ship_runtime.rs` | P08 |
| `GetInitialStarShips()` | :554-591 | **Port** | `ship_runtime.rs` | P08 |

**Dependency note:** `animation_preprocess()` (ship.c:46-58) is used as the explosion debris animation callback in `explosion_preprocess()` (tactrans.c:606). This creates a dependency: P09 (explosion) depends on P07 (ship runtime) for `animation_preprocess`. The phase ordering (P07 before P09) already satisfies this.

### Complete C Function Inventory — init.c

Every function in `init.c` is explicitly assigned below.

| C Function | Lines | Status | Rust Target | Phase |
|-----------|------:|--------|-------------|-------|
| `load_animation()` | :49-77 | **Stays in C** | — | — |
| `free_image()` | :81-113 | **Stays in C** | — | — |
| `InitSpace()` | :117-148 | **Port** | `lifecycle.rs` | P12 |
| `UninitSpace()` | :150-162 | **Port** | `lifecycle.rs` | P12 |
| `BuildSIS()` | :164-179 | **Stays in C** | — | — |
| `InitShips()` | :181-250 | **Port** | `lifecycle.rs` | P12 |
| `CountCrewElements()` | :252-274 | **Port** | `lifecycle.rs` | P12 |
| `UninitShips()` | :277-361 | **Port** | `lifecycle.rs` | P12 |

**Justifications for functions staying in C:** See "Permanent C Boundary" section above for full justifications. `load_animation()`/`free_image()` are resource subsystem internals (USE_RUST_RESOURCE scope). `BuildSIS()` is ships subsystem scope (fleet construction via `Build()` from `build.c`).

**Shared asset loading detail:** `InitSpace()` (init.c:117-148) loads shared battle assets with reference counting (`space_ini_cnt`):
- `stars_in_space` — star field (STAR_MASK_PMAP_ANIM)
- `explosion[NUM_VIEWS]` — explosion animation at 3 zoom levels (BOOM_BIG/MED/SML_MASK_PMAP_ANIM)
- `blast[NUM_VIEWS]` — blast animation at 3 zoom levels (BLAST_BIG/MED/SML_MASK_PMAP_ANIM)
- `asteroid[NUM_VIEWS]` — asteroid animation at 3 zoom levels (ASTEROID_BIG/MED/SML_MASK_PMAP_ANIM)

`UninitSpace()` (init.c:150-162) frees these in reverse order when reference count reaches zero. Both are ported in P12, with the actual `load_animation()`/`free_image()` calls going through FFI to the C resource subsystem.

### Complete C Function Inventory — process.c

Every function in `process.c` is explicitly assigned below.

| C Function | Lines | Status | Rust Target | Phase |
|-----------|------:|--------|-------------|-------|
| `CALC_ZOOM_STUFF()` | :49-74 | **Port** | `process_loop.rs` | P05 |
| `AllocElement()` | :77-100 | **Port** | `process_loop.rs` | P03 |
| `FreeElement()` | :102-115 | **Port** | `process_loop.rs` | P03 |
| `SetUpElement()` | :117-127 | **Port** | `process_loop.rs` | P03 |
| `PreProcess()` | :129-187 | **Port** | `process_loop.rs` | P03 |
| `PostProcess()` | :189-205 | **Port** | `process_loop.rs` | P03 |
| `CalcReduction()` | :207-282 | **Port** | `process_loop.rs` | P05 |
| `CalcView()` | :284-360 | **Port** | `process_loop.rs` | P05 |
| `ProcessCollisions()` | :362-628 | **Port** | `process_loop.rs` | P04 |
| `PreProcessQueue()` | :630-747 | **Port** | `process_loop.rs` | P05 |
| `InsertPrim()` | :749-784 | **Port** | `process_loop.rs` | P05 |
| `CalcDisplayCoord()` | :786-797 | **Port** | `process_loop.rs` | P05 |
| `PostProcessQueue()` | :799-984 | **Port** | `process_loop.rs` | P05 |
| `InitDisplayList()` | :986-1011 | **Port** | `process_loop.rs` | P05 |
| `RedrawQueue()` | :1013-1064 | **Port** | `process_loop.rs` | P05 |
| `Untarget()` | :1066-1092 | **Port** | `process_loop.rs` | P03 |
| `RemoveElement()` | :1094-end | **Port** | `process_loop.rs` | P03 |

### Complete C Function Inventory — intel.c

| C Function | Lines | Status | Rust Target | Phase |
|-----------|------:|--------|-------------|-------|
| `computer_intelligence()` | :31-end | **Port** | `ai.rs` | P11 |

### Netplay Functions — Scope Boundary

| C File | C Function | Status |
|--------|-----------|--------|
| `checksum.c` | `crc_processState()` | Phase 1 already provides `crc_process_element()`. Phase 2 provides `compute_frame_checksum()` using Rust-owned display list. |
| `checksum.c` | `crc_processDispQueue()` | Replaced by Rust `compute_frame_checksum()` iterating Rust-owned display list. |
| `tactrans.c` | `readyToEnd2Callback()` | Stays in C — netplay transport callback (see "Permanent C Boundary" section). |
| `tactrans.c` | `readyToEndCallback()` | Stays in C — netplay transport callback (see "Permanent C Boundary" section). |
| `tactrans.c` | `readyForBattleEndPlayer()` | Stays in C — netplay transport state machine (see "Permanent C Boundary" section). |
| `tactrans.c` | `battleEndReady{Human,Computer,Network}()` | Stay in C — DoInput-pattern callbacks (see "Permanent C Boundary" section). |
| `netplay/*.c` | All netplay transport | OUT OF SCOPE — separate netplay subsystem. This plan provides the integration hooks only. |

### Netplay Integration — Phase Assignment

The netplay integration is handled across specific phases in this plan:

| Netplay Concern | Phase | Implementation |
|----------------|-------|----------------|
| Frame checksum computation (`compute_frame_checksum`) | P13 | `netplay.rs` — iterates Rust-owned display list, produces CRC bit-identical to C `crc_processState()` |
| Frame-sync verification loop (compare CRCs, abort on mismatch) | P13 | `lifecycle.rs` — in `DoBattle` thin shell integration: compute checksum → `Netplay_NotifyAll_checksum` via FFI → `verifyChecksums` via FFI → `CHECK_ABORT` on mismatch + `resetConnections` |
| Input buffering (`BattleInputBuffer` push/pop) | P12 | `lifecycle.rs` — `process_input()` calls `getBattleInputBuffer`/`BattleInputBuffer_push`/`BattleInputBuffer_pop` via FFI |
| Battle-end multi-phase protocol (inBattle → endingBattle → endingBattle2 → interBattle) | P13 | The 5-step protocol (§13.5) is orchestrated by C-side `readyForBattleEnd()` which calls the C-side `readyToEndCallback`/`readyToEnd2Callback` and `readyForBattleEndPlayer`. Rust's `readyForBattleEnd()` port calls the C-side netplay functions via FFI. The Rust port of `new_ship()` (P09) calls `ready_for_battle_end()` which dispatches to C for netplay builds. |
| `battleFrameCount` increment | P13 | `lifecycle.rs` — incremented in `DoBattle` thin shell integration after frame completes |
| `ResetWinnerStarShip()` at battle start | P10 | `tactical.rs` — called from `Battle()` (P12) before ship selection |
| `initBattleInputBuffers` / `uninitBattleInputBuffers` | P12 | `lifecycle.rs` — called via FFI in `Battle()` setup/teardown |
| `initChecksumBuffers` / `uninitChecksumBuffers` | P12 | `lifecycle.rs` — called via FFI in `Battle()` setup/teardown |
| `setBattleStateConnections` | P12 | `lifecycle.rs` — called via FFI in `Battle()` setup/teardown (lifecycle netplay setup) |
| `initBattleStateDataConnections` | P12 | `lifecycle.rs` — called via FFI in `Battle()` setup and `new_ship()` (lifecycle netplay data setup) |
| `negotiateReadyConnections` | P12 | `lifecycle.rs` — called via FFI in `Battle()` setup and `new_ship()` (lifecycle netplay negotiation) |

> **Netplay ownership split:** P12 handles lifecycle netplay setup/teardown calls inside `Battle()`: `initBattleInputBuffers`, `uninitBattleInputBuffers`, `setBattleStateConnections`, `initBattleStateDataConnections`, `negotiateReadyConnections`. P13 handles per-frame netplay integration inside `DoBattle`: checksum computation, frame sync, `battleFrameCount`.

## Port Strategy

### Phase 2: Process Loop Moves to Rust

The process loop (PreProcessQueue, PostProcessQueue, ProcessCollisions, RedrawQueue) moves to Rust. C's `DoBattle()` becomes a thin shell that calls `rust_battle_redraw_queue()`. This is the foundation — everything else depends on the process loop being Rust-owned.

Key insight: `ProcessCollisions()` is deeply entangled with the process loop (spec §6.4). It calls `PreProcess()`, walks the display list, does recursive earlier-time checks, stuck-overlap resolution, and post-bounce full-list rescans. It MUST move as a unit with its callers.

### Phase 3: Full Battle Loop Moves to Rust

Battle lifecycle (Battle, InitShips, UninitShips, InitSpace, UninitSpace, BattleSong, FreeBattleSong, setupBattleInputOrder, selectAllShips, GetPlayerOrder, RunAwayAllowed), ship runtime (ship_preprocess, ship_postprocess, spawn_ship, animation_preprocess, inertial_thrust), tactical transitions (ship_death, explosion, cleanup, new_ship, flee, warp, winner tracking, ion trail, all helper functions), and AI dispatch all move to Rust. C's `DoBattle()` thin shell now calls `rust_battle_frame()` which orchestrates everything.

### Integration Model

All subsystem interactions go through the trait interfaces already defined in `battle/integration.rs` (Phase 1):
- `GraphicsIntegration` — 17 operations (3 declared in Phase 1, 14 added here)
- `AudioIntegration` — 11 operations (1 declared in Phase 1, 10 added here)
- `ThreadingIntegration` — 3 operations (all added here)
- `InputIntegration` — 4 operations (all added here)
- `ResourceIntegration` — 5 operations (all added here)
- `ShipRaceIntegration` — 6 operations (1 declared in Phase 1, 5 added here)
- `GlobalStateIntegration` — 4 operations (1 declared in Phase 1, 3 added here)

Rust orchestration calls these traits. FFI implementations in `c_bridge.rs` call the actual C functions.

### Existing Phase 1 Artifacts Used (NOT Re-Implemented)

| Phase 1 Module | Used By This Plan |
|---------------|-------------------|
| `element.rs` — Element struct, ElementFlags, lifecycle helpers, `commit_state()`, `is_collidable()` | Process loop, ship runtime, tactical transitions |
| `velocity.rs` — VelocityDesc, `get_next_components()`, `set_vector()`, `set_components()`, `delta_components()`, `zero()` | Process loop (velocity stepping), ship runtime (thrust) |
| `collision.rs` — `elastic_collide()`, `collision_possible()`, `isqrt()` | ProcessCollisions (post-dispatch elastic response) |
| `weapon.rs` — `weapon_collision()`, `track_ship()`, `do_damage()`, LaserBlock, MissileBlock | Ship postprocess (weapon firing), collision callbacks |
| `display_list.rs` — DisplayList pool, alloc/free/push_back/remove/iter, GenerationalHandle | Process loop (element management), all display list traversals |
| `battle_types.rs` — coords, angles, trig, SINE_TABLE | Process loop (coordinate transforms), ship runtime (facing/thrust) |
| `netplay.rs` — CrcState, `crc_process_element()`, CRC table | Frame checksum computation |
| `process_loop.rs` — ViewState, ZoomMode, zoom/camera constants (renamed from `process_types.rs` in P03) | Process loop (zoom/camera calculation) |
| `lifecycle.rs` — BattleState, BATTLE_FRAME_RATE | Battle lifecycle |
| `tactical.rs` — DeathPipelinePhase, explosion/flee/warp constants | Tactical transitions |
| `ai.rs` — EvaluateDesc, control flags, AI constants (renamed from `ai_types.rs` in P11) | AI dispatch |
| `ship_runtime.rs` — ShipPipelineStage, spawn constants (renamed from `ship_runtime_types.rs` in P07) | Ship runtime |
| `integration.rs` — All 7 trait interfaces | All orchestration modules |

## Rust Module Structure (Phase 2/3 additions)

Existing Phase 1 type-only modules are **extended** with behavioral logic. New modules are created only where justified.

```text
rust/src/battle/
  ## EXTENDED (Phase 1 types + Phase 2/3 orchestration logic added)
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

  ## EXTENDED (Phase 1 contracts + Phase 2/3 FFI implementations added)
  integration.rs            # ADDS: FFI implementations for all 43 Phase 2+ operations
                            #   (P06 creates the bridge wrappers; consuming phases
                            #    P07-P14 call them for actual feature integration)
  c_bridge.rs               # NEW: Rust-to-C FFI call wrappers (Phase 2+ operations)
  ffi.rs                    # EXTENDS: adds Phase 2/3 FFI exports
                            #   rust_battle_redraw_queue, rust_battle_frame,
                            #   rust_battle_init_ships, rust_battle_uninit_ships,
                            #   rust_battle_compute_checksum
                            # PRESERVES: all 15 Phase 1 FFI adapters unchanged

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

### C Files Modified (guarded)

| C File | Change |
|--------|--------|
| `sc2/src/uqm/process.c` | Guard ALL function bodies behind `#ifndef USE_RUST_BATTLE_LOOP`; add `extern` declarations for Rust replacements |
| `sc2/src/uqm/battle.c` | Guard `Battle()`, `ProcessInput()`, `RunAwayAllowed()`, `DoRunAway()`, `setupBattleInputOrder()`, `BattleSong()`, `FreeBattleSong()`, `selectAllShips()`, `GetPlayerOrder()` behind `#ifndef USE_RUST_BATTLE_LOOP`; thin `DoBattle()` calls `rust_battle_frame()` |
| `sc2/src/uqm/tactrans.c` | Guard ALL function bodies behind `#ifndef USE_RUST_BATTLE_LOOP` (except netplay callbacks and battleEndReady* which stay) |
| `sc2/src/uqm/intel.c` | Guard `computer_intelligence()` behind `#ifndef USE_RUST_BATTLE_LOOP` |
| `sc2/src/uqm/ship.c` | Guard ship runtime functions behind `#ifndef USE_RUST_BATTLE_LOOP` (additional to existing `USE_RUST_SHIPS`) |
| `sc2/src/uqm/init.c` | Guard init/uninit functions behind `#ifndef USE_RUST_BATTLE_LOOP` (additional to existing `USE_RUST_SHIPS`) |
| `sc2/build/unix/build.config` | Add `USE_RUST_BATTLE_LOOP` toggle |
| `sc2/config_unix.h` | Add `#define USE_RUST_BATTLE_LOOP` |

> **Guard staging between P06 and P13:** P06 creates the Rust-side bridge functions (`c_bridge.rs`) and adds `#ifndef USE_RUST_BATTLE_LOOP` guards to `process.c`. P13 adds the C-side `#ifdef` guards to the remaining C files (`battle.c`, `tactrans.c`, `intel.c`, `ship.c`, `init.c`) when the Phase 3 FFI exports are ready to receive those call paths.

## Phase Structure

| Phase | Title | Subagent | Est. LoC | TDD Summary |
|------:|-------|----------|---------|-------------|
| P00.5 | Preflight Verification | deepthinker | 0 | Verify Phase 1 artifacts, toolchain, existing tests |
| P01 | Analysis | deepthinker | 0 | Domain model, dependency graph, integration touchpoints |
| P01a | Analysis Verification | rustreviewer | 0 | Confirm all C functions covered, no gaps |
| P02 | Pseudocode | deepthinker | 0 | Algorithmic pseudocode for all 6 core algorithms |
| P02a | Pseudocode Verification | rustreviewer | 0 | Confirm pseudocode matches C behavior |
| P03 | Process Loop — PreProcess + PostProcess | rustcoder | ~800 | RED: test per-element PreProcess flag transitions and velocity stepping. GREEN: implement PreProcess/PostProcess matching C behavior. REFACTOR: extract shared helpers. |
| P03a | Process Loop Verification | rustreviewer | 0 | Verify flag transitions, velocity stepping, APPEARING handling |
| P04 | Process Loop — ProcessCollisions | rustcoder | ~700 | RED: test collision dispatch ordering, stuck-overlap resolution, recursive rechecks. GREEN: implement full ProcessCollisions with recursive earlier-time checks. REFACTOR: reduce recursive complexity where safe. |
| P04a | ProcessCollisions Verification | rustreviewer | 0 | Verify recursive behavior, dispatch ordering, post-bounce rescans |
| P05 | Process Loop — Queue Orchestration + Zoom/Camera | rustcoder | ~600 | RED: test PreProcessQueue camera tracking, PostProcessQueue cascading, zoom hysteresis. GREEN: implement queue orchestration with coordinate transforms and zoom. REFACTOR: consolidate transform logic. |
| P05a | Queue Orchestration Verification | rustreviewer | 0 | Verify cascading, scroll offsets, zoom modes, camera clamping |
| P06 | C Bridge — Phase 2 FFI Wiring | rustcoder | ~500 | RED: test FFI round-trip for DrawablesIntersect, primitive ops, sound, context. GREEN: implement c_bridge.rs FFI wrappers and process.c guards. REFACTOR: consolidate FFI patterns. |
| P06a | C Bridge Verification | rustreviewer | 0 | Verify all 43 FFI declarations, C guards compile, round-trip safety |
| P07 | Ship Runtime Pipeline | rustcoder | ~700 | RED: test ship_preprocess pipeline order (7 stages), inertial_thrust physics, energy regen. GREEN: implement ship_preprocess/postprocess matching C exactly. REFACTOR: extract pipeline stages. |
| P07a | Ship Runtime Verification | rustreviewer | 0 | Verify pipeline order, thrust physics, weapon firing sequence |
| P08 | Ship Spawn + Init | rustcoder | ~400 | RED: test spawn_ship placement (random avoiding gravity, Sa-Matra center), element initialization. GREEN: implement spawn_ship, GetNextStarShip, GetInitialStarShips. REFACTOR: consolidate ship initialization. |
| P08a | Ship Spawn Verification | rustreviewer | 0 | Verify placement, element field initialization, queue binding |
| P09 | Tactical Transitions — Death + Explosion | rustcoder | ~800 | RED: test ship_death 4-phase callback chain, explosion 36-frame animation with debris, winner tracking, battle music helpers, readiness check, death recording. GREEN: implement ship_death, StartShipExplosion, explosion_preprocess, cleanup_dead_ship, new_ship, plus all helper functions. REFACTOR: extract debris spawning. |
| P09a | Death Pipeline Verification | rustreviewer | 0 | Verify callback replacement chain, frame milestones, crew preservation, all helper functions |
| P10 | Tactical Transitions — Flee + Warp + Winner | rustcoder | ~600 | RED: test flee eligibility (5 conditions), 20-color pulse, warp ghost images, winner determination. GREEN: implement flee_preprocess, DoRunAway, ship_transition, find_alive_starship, OpponentAlive, winner state management. REFACTOR: consolidate transition helpers. |
| P10a | Flee/Warp/Winner Verification | rustreviewer | 0 | Verify flee eligibility, pulse timing, warp materialization, display-list-order winner |
| P11 | AI Dispatch | rustcoder | ~200 | RED: test computer_intelligence with Sa-Matra disabled, CYBORG dispatch, PSYTRON random, RPG overlay merge. GREEN: implement computer_intelligence(). REFACTOR: simplify control flow. |
| P11a | AI Dispatch Verification | rustreviewer | 0 | Verify all 4 dispatch paths |
| P12 | Battle Lifecycle — Init/Uninit/Input | rustcoder | ~800 | RED: test InitShips asset loading sequence, UninitShips crew writeback, ProcessInput bit mapping, RunAwayAllowed eligibility, BattleSong loading, setupBattleInputOrder netplay ordering. GREEN: implement Battle(), InitShips, UninitShips, InitSpace, UninitSpace, ProcessInput, CountCrewElements, RunAwayAllowed, setupBattleInputOrder, BattleSong, FreeBattleSong, selectAllShips, GetPlayerOrder. REFACTOR: consolidate asset management. |
| P12a | Lifecycle Verification | rustreviewer | 0 | Verify init sequence, reference counting, teardown robustness, all lifecycle helpers |
| P13 | FFI Layer — Phase 3 Exports + C Bridge Wiring | rustcoder | ~500 | RED: test rust_battle_frame FFI entry, rust_battle_init_ships/uninit_ships round-trip, compute_frame_checksum CRC, netplay frame-sync verification loop. GREEN: implement Phase 3 FFI exports, wire DoBattle thin shell, implement netplay CRC verification + battle-end sync hooks. REFACTOR: consolidate FFI error handling. |
| P13a | FFI Layer Verification | rustreviewer | 0 | Verify DoBattle→rust_battle_frame call path, C guard compilation, netplay integration |
| P14 | End-to-End Integration + Regression | rustcoder | ~200 | RED: test full battle frame cycle (init→preprocess→collision→postprocess→render), multi-frame sequences. GREEN: wire everything together, verify 2139+ Phase 1 tests still pass. REFACTOR: final cleanup. |
| P14a | E2E Verification | rustreviewer | 0 | Final gate: all tests pass, no TODO/HACK, cargo fmt/clippy/test clean |

**Total estimated new/modified LoC: ~6,800 Rust + ~400 C**

## Execution Order

```text
P00.5 → P01 → P01a → P02 → P02a
      → P03 → P03a → P04 → P04a → P05 → P05a
      → P06 → P06a
      → P07 → P07a → P08 → P08a
      → P09 → P09a → P10 → P10a
      → P11 → P11a
      → P12 → P12a → P13 → P13a
      → P14 → P14a
```

Phases execute strictly in order. Each phase MUST be completed and verified before the next begins. No skipping, no batching.

## Phase Details

---

### Phase P00.5: Preflight Verification

**Phase ID:** `PLAN-20260320-BATTLEPT2.P00.5`

**Subagent:** deepthinker

**Purpose:** Verify all assumptions before implementation.

**Toolchain Verification:**
- [ ] `cargo --version` (1.75+ required)
- [ ] `rustc --version`
- [ ] `cargo clippy --version`
- [ ] `cargo fmt --version`

**Codebase Verification:**
- [ ] All 14 Phase 1 battle modules exist and compile: `cargo check -p uqm`
- [ ] All 246 Phase 1 battle tests pass: `cargo test --lib battle`
- [ ] Phase 1 FFI functions are exported: verify `ffi.rs` contains all 15 adapters
- [ ] Phase 1 integration traits exist: verify `integration.rs` has 7 trait definitions
- [ ] `process_types.rs` (renamed to `process_loop.rs` in P03), `ship_runtime_types.rs` (renamed to `ship_runtime.rs` in P07), `ai_types.rs` (renamed to `ai.rs` in P11), `lifecycle.rs`, `tactical.rs` all contain type-only definitions ready for extension

**C Source Verification:**
- [ ] `process.c` is unmodified (no `USE_RUST_BATTLE_LOOP` guards yet)
- [ ] `battle.c` is unmodified
- [ ] `tactrans.c` is unmodified
- [ ] `intel.c` is unmodified
- [ ] `ship.c` has `USE_RUST_SHIPS` guards but no `USE_RUST_BATTLE_LOOP`
- [ ] `init.c` has `USE_RUST_SHIPS` guards but no `USE_RUST_BATTLE_LOOP`

**Integration Point Verification:**
- [ ] `DrawablesIntersect` C function exists and is callable
- [ ] Display primitive array (`DisplayArray`, `DisplayLinks`) globals exist
- [ ] `SetContext`, `BatchGraphics`, `UnbatchGraphics` C functions exist
- [ ] `DoInput` framework is intact
- [ ] `PlayerInput`, `CurrentInputToBattleInput` C functions exist

**Blocking Issues:** If any check fails, update this plan before proceeding.

**Gate Decision:**
- [ ] PASS: proceed to P01
- [ ] FAIL: revise plan

---

### Phase P01: Analysis

**Phase ID:** `PLAN-20260320-BATTLEPT2.P01`

**Subagent:** deepthinker

**Prerequisites:** P00.5 PASS

**Deliverables:**
1. **Dependency graph** — which Rust modules depend on which, and the C→Rust call chain
2. **Function-by-function mapping** — every C function mapped to its Rust target module, with exact Phase 1 types/functions it uses
3. **Integration touchpoint inventory** — every C function that Rust needs to call via FFI (the 43 Phase 2+ operations from integration.rs)
4. **State management analysis** — how display list ownership transfers from C to Rust (global state, DisplayArray, DisplayLinks, zoom_out, opt_max_zoom_out)
5. **Callback function pointer analysis** — how C function pointers in Element.preprocess_func/postprocess_func/collision_func/death_func are dispatched when Rust owns the process loop
6. **Display primitive coupling analysis** — how Rust-owned process loop manages C-owned DisplayArray and DisplayLinks globals

**Output:** `project-plans/20260311/battlept2/analysis/domain-model.md`

---

### Phase P01a: Analysis Verification

**Phase ID:** `PLAN-20260320-BATTLEPT2.P01a`

**Subagent:** rustreviewer

**Checklist:**
- [ ] Every C function from process.c (17 functions), ship.c (8 functions), tactrans.c (30 functions), intel.c (1 function), battle.c (11 functions), init.c (8 functions) is mapped
- [ ] No C function is left unmapped (either ported to Rust or explicitly stays in C with justification)
- [ ] All 43 Phase 2+ integration operations are listed
- [ ] Display list ownership transfer is explicitly described
- [ ] Callback dispatch mechanism is explicitly described

---

### Phase P02: Pseudocode

**Phase ID:** `PLAN-20260320-BATTLEPT2.P02`

**Subagent:** deepthinker

**Prerequisites:** P01a PASS

**Deliverables:** Algorithmic pseudocode for the 6 core algorithms, plus integration flow pseudocode.

**Pseudocode Files:**
1. `pseudocode/process-loop.md` — PreProcess, PostProcess, PreProcessQueue, PostProcessQueue, RedrawQueue, InitDisplayList, AllocElement, FreeElement, SetUpElement, InsertPrim, Untarget, RemoveElement, CALC_ZOOM_STUFF, CalcDisplayCoord
2. `pseudocode/process-collisions.md` — ProcessCollisions (recursive), stuck-overlap resolution, post-dispatch snapping, post-bounce rescans
3. `pseudocode/zoom-camera.md` — CalcReduction (step + continuous), CalcView (midpoint, clamping, zoom transition)
4. `pseudocode/ship-runtime.md` — ship_preprocess (7-stage pipeline), ship_postprocess (weapon firing), inertial_thrust, spawn_ship, ship collision, animation_preprocess
5. `pseudocode/tactical-transitions.md` — ship_death, StartShipExplosion, explosion_preprocess (note: spawns debris using animation_preprocess from P07), cleanup_dead_ship, new_ship, find_alive_starship, OpponentAlive, flee_preprocess, ship_transition, DoRunAway, cycle_ion_trail, spawn_ion_trail, PlayDitty, StopDitty, DittyPlaying, StopAllBattleMusic, preprocess_dead_ship, RecordShipDeath, ResetWinnerStarShip, GetWinnerStarShip, SetWinnerStarShip, setMinShipLifeSpan, setMinStarShipLifeSpan, checkOtherShipLifeSpan, readyForBattleEnd
6. `pseudocode/battle-lifecycle.md` — Battle(), InitShips, UninitShips, InitSpace/UninitSpace, ProcessInput, CountCrewElements, computer_intelligence, RunAwayAllowed, setupBattleInputOrder, BattleSong, FreeBattleSong, selectAllShips, GetPlayerOrder

Each pseudocode file uses numbered lines and includes:
- Validation points
- Error handling
- Ordering constraints
- Integration boundaries (FFI calls marked explicitly)
- Side effects
- Phase 1 type references (e.g., "uses Element from element.rs line 42")

---

### Phase P02a: Pseudocode Verification

**Phase ID:** `PLAN-20260320-BATTLEPT2.P02a`

**Subagent:** rustreviewer

**Checklist:**
- [ ] Every C function has corresponding pseudocode
- [ ] Pseudocode matches C reference behavior exactly (no simplifications that change semantics)
- [ ] ProcessCollisions recursion is correctly captured
- [ ] Flag transitions match specification §4.2 table
- [ ] Callback replacement chains are correctly sequenced
- [ ] FFI calls are explicitly marked at integration boundaries
- [ ] Phase 1 type references are accurate
- [ ] explosion_preprocess pseudocode explicitly notes animation_preprocess dependency (tactrans.c:606 → ship.c:46)

---

### Phase P03: Process Loop — PreProcess + PostProcess

**Phase ID:** `PLAN-20260320-BATTLEPT2.P03`

**Subagent:** rustcoder

**Prerequisites:** P02a PASS

**Requirements Implemented:**

| Requirement | Source |
|------------|--------|
| PreProcess per-element: life=0→death, APPEARING→init, velocity, flags | Spec §8.3 |
| PostProcess per-element: callback, commit state, init intersection, flags | Spec §8.4 |
| Asymmetric DEFY_PHYSICS clearing | Spec §4.2, Req lifecycle-flag-3/4 |
| FINITE_LIFE decrement | Spec §8.3 step 7 |
| APPEARING handling (PLAYER_SHIP local-copy-only clear) | Spec §8.3 step 2 |
| CHANGING + collidable → reinit intersection | Spec §8.3 step 4 |
| Untarget on element death | Spec §4.1, Req lifecycle-4 |
| SetUpElement intersection initialization | process.c:117-127 SetUpElement() |
| AllocElement/FreeElement with display prim coupling | process.c:77-115 |
| RemoveElement with sound cleanup | process.c:1094-end |

**TDD Cycle:**
- **RED:** Write tests for PreProcess flag transitions (PRE_PROCESS set, POST_PROCESS+COLLISION cleared), velocity stepping integration (uses VelocityDesc from velocity.rs), APPEARING handling for PLAYER_SHIP vs non-PLAYER_SHIP, life_span=0 death sequence, FINITE_LIFE decrement, CHANGING intersection reinit. Write tests for PostProcess flag transitions (POST_PROCESS set, PRE_PROCESS+CHANGING+APPEARING cleared), asymmetric DEFY_PHYSICS clearing, commit_state() invocation.
- **GREEN:** Implement `pre_process()` and `post_process()` in `process_loop.rs`, using Phase 1 Element methods and VelocityDesc.
- **REFACTOR:** Extract shared flag-transition logic into helper methods.

**Files to Create/Modify:**
- Rename `process_types.rs` → `process_loop.rs` (keep all Phase 1 types, add orchestration)
- Update `mod.rs` to reflect rename
- Add `pre_process()`, `post_process()`, `setup_element()`, `alloc_element()`, `free_element()`, `untarget()`, `remove_element()` functions

**Pseudocode Traceability:** `pseudocode/process-loop.md` lines covering PreProcess and PostProcess

**Verification Commands:**
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

---

### Phase P03a: Process Loop PreProcess/PostProcess Verification

**Phase ID:** `PLAN-20260320-BATTLEPT2.P03a`

**Subagent:** rustreviewer

**Checklist:**
- [ ] PreProcess flag transitions match specification §4.2 table exactly
- [ ] PostProcess flag transitions match specification §4.2 table exactly
- [ ] Asymmetric DEFY_PHYSICS clearing: COLLISION set → clear COLLISION keep DEFY; no COLLISION → clear DEFY
- [ ] APPEARING handling: PLAYER_SHIP clears in local copy only; non-PLAYER_SHIP skips preprocess callback
- [ ] Velocity stepping uses Phase 1 `get_next_components()` from velocity.rs
- [ ] FINITE_LIFE decrement occurs after velocity stepping
- [ ] Death callback invoked when life_span==0, DISAPPEARING set, Untarget called
- [ ] No TODO/FIXME/HACK in implementation code
- [ ] All Phase 1 tests still pass (246 tests)
- [ ] `cargo fmt/clippy/test` clean

---

### Phase P04: Process Loop — ProcessCollisions

**Phase ID:** `PLAN-20260320-BATTLEPT2.P04`

**Subagent:** rustcoder

**Prerequisites:** P03a PASS

**Requirements Implemented:**

| Requirement | Source |
|------------|--------|
| ProcessCollisions recursive orchestration | Spec §6.4, process.c:362-628 |
| Collision dispatch ordering (PLAYER_SHIP first) | Spec §6.2, Req collision-8 |
| Recursive earlier-time checks | Spec §6.4 entanglement #1 |
| Stuck-overlap handling (APPEARING kill, position revert) | Spec §6.4 entanglement #3, Req collision-12 |
| Collision-point snapping (snap next to intersection) | Spec §6.4 entanglement #4, Req collision-10 |
| Post-bounce full-list rescans | Spec §6.4 entanglement #5, Req collision-21 |
| COLLISION flag as re-entry guard | Spec §6.4 entanglement #6 |
| Direct PreProcess invocation on unprocessed elements | Spec §6.4 entanglement #2 |
| Pixel-accurate intersection testing via DrawablesIntersect FFI | Spec §6.1, Req collision-3 |
| Forward-only scan (preprocess) vs full-list scan (postprocess) | Spec §6.1, Req collision-5 |

**TDD Cycle:**
- **RED:** Write tests for collision dispatch ordering (PLAYER_SHIP test element → call test first; else current first), stuck-overlap resolution (APPEARING elements killed, non-APPEARING reverted), recursive earlier-time check (if A↔B collide at T, check A and B for earlier collisions at T'<T), collision-point snapping, post-bounce rescan triggering. Use mock `DrawablesIntersect` that returns controlled time_val results.
- **GREEN:** Implement `process_collisions()` in `process_loop.rs` with full recursive structure matching C exactly. Uses Phase 1 `collision_possible()`, `elastic_collide()`.
- **REFACTOR:** Reduce stack depth where safe without changing semantics.

**Files to Modify:**
- `process_loop.rs` — add `process_collisions()` function
- `c_bridge.rs` — add FFI wrapper for `DrawablesIntersect`

**Pseudocode Traceability:** `pseudocode/process-collisions.md` all lines

---

### Phase P04a: ProcessCollisions Verification

**Phase ID:** `PLAN-20260320-BATTLEPT2.P04a`

**Subagent:** rustreviewer

**Checklist:**
- [ ] Recursive structure matches C process.c:362-628
- [ ] Dispatch ordering correct: PLAYER_SHIP test element's handler called first
- [ ] COLLISION flag set on both elements after dispatch
- [ ] Stuck overlap: APPEARING killed (DISAPPEARING set, life=0), non-APPEARING positions reverted
- [ ] Position snapping: after dispatch, next.location = collision point, InitIntersectEndPoint called
- [ ] Post-bounce: elastic_collide changes velocity → ProcessCollisions re-called from head for both elements
- [ ] Recursive earlier-time: before dispatching A↔B, recursively check both for earlier intersections
- [ ] PreProcess called on unprocessed elements encountered during successor walk
- [ ] Uses Phase 1 `collision_possible()` from collision.rs
- [ ] Uses Phase 1 `elastic_collide()` from collision.rs
- [ ] DrawablesIntersect called via c_bridge.rs FFI
- [ ] No TODO/FIXME/HACK

---

### Phase P05: Process Loop — Queue Orchestration + Zoom/Camera

**Phase ID:** `PLAN-20260320-BATTLEPT2.P05`

**Subagent:** rustcoder

**Prerequisites:** P04a PASS

**Requirements Implemented:**

| Requirement | Source |
|------------|--------|
| PreProcessQueue: head-to-tail, PreProcess, collision vs successors, camera | Spec §8.2, Req process-3/4 |
| PostProcessQueue: flag clearing, scroll, newly-added cascading, removal, render | Spec §8.4, Req process-13-23 |
| Newly-added element cascading (tail-chasing) | Spec §8.5, Req process-14/15/16 |
| Scroll offset application (PRE/POST_PROCESS dependent) | Spec §8.4, Req process-17/18 |
| DISAPPEARING removal + rendering setup | Spec §8.4, Req process-19/20/21/22 |
| Zoom calculation (step + continuous modes, hysteresis) | Spec §8.6, Req process-24/25 |
| Camera calculation (midpoint, single-ship clamping) | Spec §8.7, Req process-26/27/28 |
| World-to-screen coordinate conversion | Spec §8.7, Req process-30/31 |
| RedrawQueue frame dispatch | Spec §8.1, Req process-1/2 |
| InitDisplayList | process.c:986-1011 InitDisplayList() |
| CALC_ZOOM_STUFF | process.c:49-74 |
| CalcDisplayCoord | process.c:786-797 |

**TDD Cycle:**
- **RED:** Write tests for PreProcessQueue (iterates all elements, tracks ship positions, computes zoom and camera), PostProcessQueue (newly-added element cascading with tail-chasing, DISAPPEARING removal, coordinate transforms, scroll offset application), zoom calculation (step mode 3-level hysteresis, continuous mode smooth interpolation, edge cases), camera calculation (midpoint, single-ship clamping, zoom change recalculation).
- **GREEN:** Implement `pre_process_queue()`, `post_process_queue()`, `redraw_queue()`, `calc_reduction()`, `calc_view()`, `init_display_list()`, `insert_prim()`, `calc_display_coord()`, `calc_zoom_stuff()` in `process_loop.rs`.
- **REFACTOR:** Consolidate coordinate transform logic into a single path.

**Files to Modify:**
- `process_loop.rs` — add queue orchestration, zoom/camera, frame dispatch
- `c_bridge.rs` — add FFI wrappers for SetContext, DrawBatch, ClearDrawable, SetGraphicScale, FlushSounds, UpdateSoundPositions, SetEquFrameIndex, CalcDisplayCoord

**Pseudocode Traceability:** `pseudocode/process-loop.md` queue orchestration lines + `pseudocode/zoom-camera.md` all lines

---

### Phase P05a: Queue Orchestration Verification

**Phase ID:** `PLAN-20260320-BATTLEPT2.P05a`

**Subagent:** rustreviewer

**Checklist:**
- [ ] PreProcessQueue iterates head-to-tail, calls PreProcess for unprocessed elements
- [ ] PreProcessQueue runs ProcessCollisions against successors for collidable elements
- [ ] PreProcessQueue tracks PLAYER_SHIP positions for camera/zoom
- [ ] PostProcessQueue handles newly-added elements (no PRE_PROCESS flag) with inner cascading loop
- [ ] Cascading loop continues until no new unprocessed elements remain (tail-chasing)
- [ ] After cascading: scroll offsets zeroed
- [ ] Scroll offsets: PRE_PROCESS+not POST_PROCESS → apply scroll; both → zero scroll
- [ ] DISAPPEARING elements removed and deallocated
- [ ] Surviving elements: world→screen transform, zoom frame selection, postprocess callback, render list insertion
- [ ] Line prims: both endpoints transformed with wrap handling
- [ ] Stamp/stamp-fill: zoom-level frame from farray via SetEquFrameIndex
- [ ] CalcReduction: step mode 3-level hysteresis, continuous mode smooth interpolation
- [ ] CalcView: midpoint camera, single-ship clamping, zoom transition
- [ ] RedrawQueue: PreProcessQueue → PostProcessQueue → UpdateSoundPositions → conditional render
- [ ] Simulation always executes; only rendering conditionally skipped
- [ ] No TODO/FIXME/HACK

---

### Phase P06: C Bridge — Phase 2 FFI Wiring

**Phase ID:** `PLAN-20260320-BATTLEPT2.P06`

**Subagent:** rustcoder

**Prerequisites:** P05a PASS

**Requirements Implemented:**

| Requirement | Source |
|------------|--------|
| All 43 Phase 2+ integration FFI bridge wrappers (availability; actual feature integration happens in consuming phases P07-P14) | Spec §14, plan overview integration inventory |
| C-side guards for process.c | Build toggle USE_RUST_BATTLE_LOOP |
| Display primitive array access from Rust | Req G1/G2 |
| Batch rendering entry | Req G5 |
| Drawing context management | Req G9 |
| All audio operations | Req A1-A11 |
| All threading operations | Req T1-T3 |
| All input operations | Req I1-I4 |
| All resource operations | Req R1-R5 |
| All ship/race operations | Req S1-S7 |
| All global state operations | Req GS1-GS4 |

**TDD Cycle:**
- **RED:** Write tests that FFI wrappers are declared with correct signatures, C guard macros produce correct preprocessor output, round-trip safety for pointer types.
- **GREEN:** Implement `c_bridge.rs` with all FFI wrappers. Add `#ifndef USE_RUST_BATTLE_LOOP` guards to `process.c`. Add `extern` declarations for Rust functions.
- **REFACTOR:** Consolidate FFI wrapper patterns (null-pointer checks, error conversion).

**Files to Create/Modify:**
- `c_bridge.rs` — NEW: Rust→C FFI call wrappers for all 43 integration operations (the reverse direction from ffi.rs’s C→Rust exports)
- `ffi.rs` — EXTEND: add `rust_battle_redraw_queue` export alongside the 15 existing Phase 1 adapters (which remain unchanged — see “Phase 1 FFI Adapters” table above)
- `process.c` — add `#ifndef USE_RUST_BATTLE_LOOP` guards around ALL function bodies
- `build.config` — add `USE_RUST_BATTLE_LOOP` toggle
- `config_unix.h` — add `#define USE_RUST_BATTLE_LOOP`

---

### Phase P06a: C Bridge Verification

**Phase ID:** `PLAN-20260320-BATTLEPT2.P06a`

**Subagent:** rustreviewer

**Checklist:**
- [ ] `c_bridge.rs` declares all 43 Phase 2+ FFI operations
- [ ] All FFI signatures match C function declarations exactly (types, calling convention)
- [ ] Null-pointer safety: all pointer parameters validated before dereference
- [ ] `process.c` compiles with `USE_RUST_BATTLE_LOOP` defined (C functions guarded out)
- [ ] `process.c` compiles without `USE_RUST_BATTLE_LOOP` (original behavior preserved)
- [ ] `build.config` toggle follows established pattern (`USE_RUST_SHIPS`, `USE_RUST_BATTLE`)
- [ ] `rust_battle_redraw_queue` FFI export declared with correct signature
- [ ] All 15 Phase 1 FFI adapters still present and unchanged in `ffi.rs`
- [ ] `cargo fmt/clippy/test` clean

---

### Phase P07: Ship Runtime Pipeline

**Phase ID:** `PLAN-20260320-BATTLEPT2.P07`

**Subagent:** rustcoder

**Prerequisites:** P06a PASS

**Requirements Implemented:**

| Requirement | Source |
|------------|--------|
| Ship per-frame pipeline (7 stages in exact order) | Spec §Ship runtime, Req ship-runtime-7 |
| First-frame initialization (APPEARING: suppress inputs, init crew, invoke preprocess, warp-in) | Spec §Ship runtime, Req ship-runtime-8 |
| Energy regeneration | Spec §Ship runtime, Req ship-runtime-9 |
| Turn processing (facing ±1, turn_wait cooldown) | Spec §Ship runtime, Req ship-runtime-10 |
| Thrust processing (inertial_thrust, ion trail spawn) | Spec §Ship runtime, Req ship-runtime-11 |
| Inertial movement model (inertialess, normal, gravity well, at-max-speed turning) | Spec §Ship runtime, Req ship-runtime-12-17 |
| Ship collision (planet damage = hp/4 min 1) | Spec §Ship runtime, Req ship-runtime-18 |
| Weapon firing pipeline (cooldown, energy, callback, bind, sound, wait) | Spec §Ship runtime, Req ship-runtime-20/21 |
| animation_preprocess (frame advance, turn_wait cooldown) | ship.c:46-58 |

**TDD Cycle:**
- **RED:** Write tests for the 7-stage pipeline order (input → APPEARING → energy → preprocess → turn → thrust → status), inertial_thrust physics (inertialess instant velocity, normal acceleration, gravity-well max speed, at-max-speed half-thrust turning), energy regeneration counter, weapon firing sequence (cooldown check → energy deduct → weapon callback → element bind → sound → wait), ship collision damage calculation.
- **GREEN:** Implement `ship_preprocess()`, `ship_postprocess()`, `ship_collision()`, `inertial_thrust()`, `animation_preprocess()` in `ship_runtime.rs`.
- **REFACTOR:** Extract pipeline stages into individual helper functions.

**Files to Modify:**
- Rename `ship_runtime_types.rs` → `ship_runtime.rs` (keep all Phase 1 types, add orchestration)
- Update `mod.rs` to reflect rename

**Pseudocode Traceability:** `pseudocode/ship-runtime.md` all lines

**Critical dependency:** `animation_preprocess()` (ship.c:46-58) is also used as the explosion debris animation callback by `explosion_preprocess()` (tactrans.c:606). P09 depends on P07 providing this function.

---

### Phase P07a: Ship Runtime Verification

**Phase ID:** `PLAN-20260320-BATTLEPT2.P07a`

**Subagent:** rustreviewer

**Checklist:**
- [ ] Pipeline order matches exactly: input → APPEARING → energy → preprocess → turn → thrust → status
- [ ] APPEARING first-frame: inputs suppressed, crew init, race preprocess invoked, warp-in started, early return
- [ ] PLAYER_SHIP APPEARING: APPEARING cleared in local copy only (actual flags retain APPEARING)
- [ ] Energy regen: counter countdown → DeltaEnergy when elapsed
- [ ] Turn: NORMALIZE_FACING ±1, update image frame, apply turn_wait
- [ ] Thrust: inertial_thrust() → ion trail spawn (if not cloaked) → apply thrust_wait
- [ ] inertial_thrust: MAX_ALLOWED_SPEED=WORLD_TO_VELOCITY(DISPLAY_TO_WORLD(18)), inertialess check, gravity well, at-max-speed half-thrust
- [ ] Weapon firing: weapon_counter check → energy cost → init_weapon_func (up to 6) → SetElementStarShip → sound → weapon_wait
- [ ] Ship collision: GRAVITY_MASS check → damage = hp/4 (min 1), sound scaled by damage
- [ ] animation_preprocess: frame advance via IncFrameIndex, CHANGING flag, turn_wait cooldown
- [ ] animation_preprocess is public/accessible for P09 explosion debris callback
- [ ] Uses Phase 1 velocity operations from velocity.rs
- [ ] Uses Phase 1 weapon types from weapon.rs
- [ ] No TODO/FIXME/HACK

---

### Phase P08: Ship Spawn + Init

**Phase ID:** `PLAN-20260320-BATTLEPT2.P08`

**Subagent:** rustcoder

**Prerequisites:** P07a PASS

**Requirements Implemented:**

| Requirement | Source |
|------------|--------|
| spawn_ship placement (random avoiding gravity, Sa-Matra center) | Spec §Ship runtime, Req ship-runtime-1/3/5 |
| Element initialization (flags, callbacks, velocity, mass, life) | Spec §Ship runtime, Req ship-runtime-2 |
| Bidirectional binding (element↔queue entry) | Spec §Ship runtime, Req ship-runtime-4 |
| Queue entry reuse (reinitialize in place) | Spec §Ship runtime, Req ship-runtime-6 |
| GetNextStarShip (encounter queue, infinite fleet recycling) | Spec §Tactical, Req tactical-15/16/17 |
| GetInitialStarShips (SuperMelee vs encounter) | ship.c:554-591 GetInitialStarShips() |

**TDD Cycle:**
- **RED:** Write tests for spawn_ship element initialization (APPEARING|PLAYER_SHIP|IGNORE_SIMILAR flags, NORMAL_LIFE, ship_mass, zero velocity, callbacks set), random placement avoiding gravity wells, Sa-Matra center placement, crew patching from queue entry, element reuse (existing hShip → reinitialize vs new alloc), GetNextStarShip ship selection (encounter queue traversal, infinite fleet recycling with crew reset).
- **GREEN:** Implement `spawn_ship()`, `get_next_starship()`, `get_initial_starships()` in `ship_runtime.rs`.
- **REFACTOR:** Consolidate element initialization into a builder pattern.

**Files to Modify:**
- `ship_runtime.rs` — add spawn functions

**Pseudocode Traceability:** `pseudocode/ship-runtime.md` spawn-related lines

---

### Phase P08a: Ship Spawn Verification

**Phase ID:** `PLAN-20260320-BATTLEPT2.P08a`

**Subagent:** rustreviewer

**Checklist:**
- [ ] Element flags: APPEARING | PLAYER_SHIP | IGNORE_SIMILAR set on spawn
- [ ] Life span = NORMAL_LIFE (1)
- [ ] Mass = ship_mass from descriptor
- [ ] Velocity = zero (ZeroVelocityComponents)
- [ ] Callbacks = ship_preprocess/postprocess/ship_death/collision
- [ ] Crew patched from queue entry (cap at max_crew for encounters)
- [ ] Random position: avoids gravity wells and existing matter
- [ ] Sa-Matra: defending ship at center, facing=0
- [ ] Element reuse: if queue entry has hShip, reinitialize; else alloc new
- [ ] Bidirectional binding: element.p_parent↔queue entry.hShip
- [ ] GetNextStarShip: encounter queue traversal, infinite fleet recycling
- [ ] No TODO/FIXME/HACK

---

### Phase P09: Tactical Transitions — Death + Explosion

**Phase ID:** `PLAN-20260320-BATTLEPT2.P09`

**Subagent:** rustcoder

**Prerequisites:** P08a PASS

**Requirements Implemented:**

| Requirement | Source |
|------------|--------|
| ship_death 4-phase pipeline (callback replacement chain) | Spec §10.1, Req tactical-1-3 |
| StartShipExplosion (zero velocity, drain energy, set life=36, FINITE_LIFE+NONSOLID) | Spec §10.2, Req tactical-4/5 |
| explosion_preprocess 36-frame animation (1-3 debris, hide frame 15, clear frame 25) | Spec §10.2, Req tactical-6/7/8 |
| cleanup_dead_ship (record crew, clear ownership, preserve CREW_OBJECT, victory music) | Spec §10.3, Req tactical-9/10/11 |
| new_ship (readiness wait, free descriptor, persist crew, deactivate, request replacement) | Spec §10.3, Req tactical-13/14 |
| Ship death recording (decrement counter, melee notification) | Spec §10.5, Req tactical-26 |
| Winner kept alive one frame longer than loser | Spec §10.3, Req tactical-12 |
| Ion trail spawning (12-color, POINT_PRIM, head-insert, pre-processed) | Spec §Tactical, Req tactical-27a-27e |
| StopAllBattleMusic — audio cleanup (stops ditty + music) | tactrans.c:618-623 |
| PlayDitty / StopDitty / DittyPlaying — victory ditty lifecycle | tactrans.c:77-100 |
| preprocess_dead_ship — sound processing during death wait | tactrans.c:280-285 |
| RecordShipDeath — battle_counter decrement + melee notification | tactrans.c:682-700 |
| readyForBattleEnd — ditty-done + netplay readiness check | tactrans.c:254-278 |
| setMinShipLifeSpan / setMinStarShipLifeSpan / checkOtherShipLifeSpan — winner lifespan coordination | tactrans.c:376-437 |
| explosion_preprocess spawns debris using animation_preprocess from P07 | tactrans.c:606 → ship.c:46 |

**TDD Cycle:**
- **RED:** Write tests for the complete ship_death→explosion→cleanup→new_ship callback chain (verify each phase replaces the correct callback), explosion 36-frame animation (verify debris count per frame, frame 15 hide, frame 25 clear), cleanup_dead_ship (verify CREW_OBJECT preserved while other owned elements marked for deletion), new_ship readiness wait and ship selection, ion trail spawning with correct color cycle and display list head insertion, StopAllBattleMusic, PlayDitty/StopDitty/DittyPlaying lifecycle, RecordShipDeath counter management, readyForBattleEnd ditty check, checkOtherShipLifeSpan winner coordination.
- **GREEN:** Implement `ship_death()`, `start_ship_explosion()`, `explosion_preprocess()`, `cleanup_dead_ship()`, `new_ship()`, `spawn_ion_trail()`, `cycle_ion_trail()`, `play_ditty()`, `stop_ditty()`, `ditty_playing()`, `stop_all_battle_music()`, `preprocess_dead_ship()`, `record_ship_death()`, `ready_for_battle_end()`, `set_min_ship_life_span()`, `set_min_starship_life_span()`, `check_other_ship_life_span()` in `tactical.rs`.
- **REFACTOR:** Extract debris spawning into a parameterized helper.

**Files to Modify:**
- `tactical.rs` — add orchestration functions to existing type-only module

**Pseudocode Traceability:** `pseudocode/tactical-transitions.md` death/explosion lines

---

### Phase P09a: Death Pipeline Verification

**Phase ID:** `PLAN-20260320-BATTLEPT2.P09a`

**Subagent:** rustreviewer

**Checklist:**
- [ ] ship_death: stops all battle music, clears victory-ditty, starts explosion, finds winner, records death
- [ ] StartShipExplosion: zero velocity, drain energy, life=36, FINITE_LIFE+NONSOLID, replace preprocess=explosion+death=cleanup+postprocess=PostProcessStatus, play sound
- [ ] explosion_preprocess: 36 frames, debris count varies by frame (1-3), frame 15 hides prim, frame 25 clears preprocess. Debris elements use animation_preprocess from P07 (ship.c:46).
- [ ] cleanup_dead_ship: records crew, iterates elements, clears ownership (NONSOLID|DISAPPEARING|FINITE_LIFE, clear callbacks), preserves CREW_OBJECT, plays victory ditty, sets death=new_ship+preprocess=preprocess_dead_ship
- [ ] new_ship: waits for readiness (ditty done, netplay sync via readyForBattleEnd), stops audio, frees descriptor, persists crew, deactivates queue entry, requests replacement via GetNextStarShip
- [ ] Winner kept alive one frame longer than loser via checkOtherShipLifeSpan
- [ ] Callback replacement chain correct: ship_death sets death=cleanup+preprocess=explosion; cleanup sets death=new_ship+preprocess=preprocess_dead_ship
- [ ] Ion trail: 12-color cycle, POINT_PRIM, inserted at display list head, PRE_PROCESS set, life pre-decremented
- [ ] StopAllBattleMusic: calls StopDitty + StopMusic
- [ ] PlayDitty: plays victory_ditty via PlayMusic, sets dittyIsPlaying
- [ ] StopDitty: conditionally stops music if ditty is playing
- [ ] DittyPlaying: checks PLRPlaying status
- [ ] preprocess_dead_ship: only calls ProcessSound
- [ ] RecordShipDeath: decrements battle_counter[playerNr], calls MeleeShipDeath for SuperMelee, handles flee (mass > MAX_SHIP_MASS) skip
- [ ] readyForBattleEnd: non-netplay returns !DittyPlaying; netplay checks all player handlers
- [ ] setMinShipLifeSpan / setMinStarShipLifeSpan: extends life_span of finished-exploding ship
- [ ] checkOtherShipLifeSpan: keeps winner alive longer than loser; handles simultaneous death
- [ ] Uses Phase 1 tactical constants from tactical.rs
- [ ] No TODO/FIXME/HACK

---

### Phase P10: Tactical Transitions — Flee + Warp + Winner

**Phase ID:** `PLAN-20260320-BATTLEPT2.P10`

**Subagent:** rustcoder

**Prerequisites:** P09a PASS

**Requirements Implemented:**

| Requirement | Source |
|------------|--------|
| Flee eligibility (5 conditions) | Spec §10.3 flee, Req tactical-33 |
| Flee initiation (mass=FLEE_MASS, dark red, timing, suppress input) | Spec §10.3 flee, Req tactical-34a-f |
| Flee animation (20-color pulse, accelerating timing, warp-out trigger) | Spec §10.3 flee, Req tactical-35/36a-f |
| DoRunAway (flee entry from escape input) | battle.c:72-105 DoRunAway() |
| RunAwayAllowed (flee eligibility check — IN_ENCOUNTER/IN_LAST_BATTLE + STARBASE_AVAILABLE + !BOMB_CARRIER) | battle.c:63-70 RunAwayAllowed() — used by DoRunAway call site in ProcessInput |
| ship_transition (15 frames, ghost images, materialization) | Spec §10.3 warp, Req tactical-28-31 |
| cycle_ion_trail (12-color ghost image fade) | tactrans.c:755-789 cycle_ion_trail() |
| find_alive_starship (display-list-order, PLAYER_SHIP, Pkunk mass+1) | Spec §10.2, Req tactical-20a-e |
| OpponentAlive (display list iteration, crew check, 3 return cases) | Spec §10.2, Req tactical-25a-d |
| Winner recorded once; victory-ditty set each death | Spec §10.2, Req tactical-22/23 |
| ResetWinnerStarShip — reset winner tracking at battle start | tactrans.c:102-106 |
| GetWinnerStarShip / SetWinnerStarShip — winner state accessors | tactrans.c:661-680 |

**TDD Cycle:**
- **RED:** Write tests for flee eligibility (stamp prim, NORMAL_LIFE, no FINITE_LIFE, not FLEE_MASS, no APPEARING — test each condition independently), 20-color pulse cycle with accelerating timing, warp-out trigger conditions (timing=0 AND cycle=midpoint), DoRunAway initiation (sets mass, color, timing, replaces preprocess, suppresses input), ship_transition 15-frame ghost image spawning with correct positioning along facing vector, materialization steps (show prim, clear NONSOLID|FINITE_LIFE, restore callbacks), find_alive_starship display-list-order traversal with Pkunk reincarnation (mass=11+crew=0→alive), OpponentAlive 3 return cases, ResetWinnerStarShip/GetWinnerStarShip/SetWinnerStarShip lifecycle.
- **GREEN:** Implement `flee_preprocess()`, `do_run_away()`, `ship_transition()`, `find_alive_starship()`, `opponent_alive()`, `reset_winner_starship()`, `get_winner_starship()`, `set_winner_starship()` in `tactical.rs`.
- **REFACTOR:** Consolidate transition helper logic.

**Files to Modify:**
- `tactical.rs` — add flee/warp/winner functions

**Pseudocode Traceability:** `pseudocode/tactical-transitions.md` flee/warp/winner lines

---

### Phase P10a: Flee/Warp/Winner Verification

**Phase ID:** `PLAN-20260320-BATTLEPT2.P10a`

**Subagent:** rustreviewer

**Checklist:**
- [ ] Flee eligibility: all 5 conditions checked independently, silently ignores if any fails
- [ ] Flee initiation (DoRunAway): mass=100 (MAX_SHIP_MASS*10), replace preprocess=flee_preprocess, zero velocity, clear max-speed flags, dark red stamp-fill (0x0B,0x00,0x00), clear colorCycleIndex, set turn_wait=3+thrust_wait=4, suppress inputs, decrement battle_counter[0]
- [ ] Flee animation: 20-color cycle, timing accelerates with each full cycle, all inputs suppressed every frame
- [ ] Flee warp-out: timing=0 AND cycle=midpoint → death=cleanup, crew=0, trigger warp-out (life=HYPERJUMP_LIFE+1, preprocess=ship_transition, hide prim, NONSOLID+FINITE_LIFE+CHANGING)
- [ ] ship_transition: life=HYPERJUMP_LIFE, hide prim, NONSOLID+FINITE_LIFE+CHANGING, clear postprocess
- [ ] Ghost images: one per frame along facing vector, ion-trail color cycle, STAMPFILL_PRIM
- [ ] Materialization (life==NORMAL_LIFE, crew>0): show prim, select zoom frame, init intersection, zero velocity, clear NONSOLID|FINITE_LIFE, restore callbacks
- [ ] Warp-out (life==NORMAL_LIFE, crew==0): proceed to cleanup/new-ship
- [ ] find_alive_starship: display-list head-to-tail, first PLAYER_SHIP not dead + not fleeing
- [ ] Winner: zero crew + not reincarnating → null (mutual destruction)
- [ ] Winner: recorded once per battle (SetWinnerStarShip no-ops if already set); victory-ditty set each death event
- [ ] Pkunk: mass==MAX_SHIP_MASS+1 + crew==0 → treated as alive
- [ ] OpponentAlive: iterates ALL elements, checks non-null owning ship crew_level
- [ ] Winner depends on display list order (not side index)
- [ ] ResetWinnerStarShip: sets winnerStarShip = null
- [ ] No TODO/FIXME/HACK

---

### Phase P11: AI Dispatch

**Phase ID:** `PLAN-20260320-BATTLEPT2.P11`

**Subagent:** rustcoder

**Prerequisites:** P10a PASS

**Requirements Implemented:**

| Requirement | Source |
|------------|--------|
| computer_intelligence entry point | Spec §11.1, Req ai-1 |
| RPG overlay merge (human escape + AI battle input) | Spec §11.1, Req ai-2 |
| Sa-Matra disabled AI (IN_LAST_BATTLE → return 0) | Spec §11.1, Req ai-3 |
| PSYTRON random ship selection (sleep + BATTLE_WEAPON) | Spec §11.1, Req ai-4 |
| Race-specific intelligence dispatch (CYBORG → tactical_intelligence) | Spec §11.1, Req ai-1 |

**TDD Cycle:**
- **RED:** Write tests for all 4 dispatch paths: Sa-Matra (returns 0), CYBORG (calls race intelligence + merges RPG escape), PSYTRON (returns BATTLE_WEAPON after sleep), non-cyborg (returns human input directly).
- **GREEN:** Implement `computer_intelligence()` in `ai.rs`.
- **REFACTOR:** Simplify control flow.

**Files to Modify:**
- Rename `ai_types.rs` → `ai.rs` (keep all Phase 1 types, add dispatch function)
- Update `mod.rs` to reflect rename

**Pseudocode Traceability:** `pseudocode/battle-lifecycle.md` AI dispatch lines

---

### Phase P11a: AI Dispatch Verification

**Phase ID:** `PLAN-20260320-BATTLEPT2.P11a`

**Subagent:** rustreviewer

**Checklist:**
- [ ] IN_LAST_BATTLE: returns 0 (AI disabled)
- [ ] CYBORG_CONTROL: calls tactical_intelligence() via race descriptor callback
- [ ] RPG player overlay: merges BATTLE_ESCAPE from human input with AI battle input
- [ ] Non-CYBORG in battle: returns CurrentInputToBattleInput (direct human input)
- [ ] PSYTRON_CONTROL selecting ship: sleeps 0.5s, returns BATTLE_WEAPON
- [ ] Uses Phase 1 AI constants from ai.rs (renamed from ai_types.rs in P11)
- [ ] Uses Phase 1 control flags (HUMAN_CONTROL, CYBORG_CONTROL, PSYTRON_CONTROL)
- [ ] No TODO/FIXME/HACK

---

### Phase P12: Battle Lifecycle — Init/Uninit/Input

**Phase ID:** `PLAN-20260320-BATTLEPT2.P12`

**Subagent:** rustcoder

**Prerequisites:** P11a PASS

**Requirements Implemented:**

| Requirement | Source |
|------------|--------|
| Battle() entry sequence (RNG seed, music, InitShips, activity flag, spawn, music start) | Spec §9.1, Req lifecycle-1a-4g |
| InitShips (load shared assets, contexts, display list, stars, environment) | Spec §9.1, Req lifecycle-2 |
| Shared asset reference counting (InitSpace/UninitSpace) | Spec §9.1, Req lifecycle-3a-d |
| InitSpace: loads stars_in_space, explosion[3], blast[3], asteroid[3] at all zoom levels | init.c:117-148 |
| UninitSpace: frees blast, explosion, asteroid, stars_in_space when refcount=0 | init.c:150-162 |
| UninitShips teardown (stop audio, free assets, count crew, writeback, clear activity) | Spec §9.1, Req lifecycle-16a-19 |
| ProcessInput (side iteration, bit mapping, escape detection) | Spec §9.3, Req lifecycle-14/15 |
| RunAwayAllowed (flee eligibility: IN_ENCOUNTER or IN_LAST_BATTLE + STARBASE_AVAILABLE + !BOMB_CARRIER) | battle.c:63-70 |
| setupBattleInputOrder (local players first, network players last) | battle.c:107-135 |
| BattleSong / FreeBattleSong (load/play/free battle music by space type) | battle.c:234-256 |
| selectAllShips (HyperSpace: single ship; normal: GetInitialStarShips) | battle.c:375-394 |
| GetPlayerOrder (netplay discriminant-based player ordering) | battle.c:357-372 |
| Frame timing (24 fps normal, max-speed skip) | Spec §9.4, Req lifecycle-11-13 |
| CountCrewElements | init.c:252-274 CountCrewElements() |
| Instant-victory skip | Spec §9.1, Req lifecycle-5 |
| Teardown returns hyperspace exit (negative ship count) | Spec §9.1, Req lifecycle-20 |

**TDD Cycle:**
- **RED:** Write tests for InitShips sequence (InitSpace reference counting, display list reset, environment object spawning for normal vs final-battle), UninitShips teardown (stop sounds, free assets, count crew, add floating crew to survivor capped at max, record results, free descriptors, clear activity), ProcessInput bit mapping (BATTLE_LEFT→LEFT, BATTLE_RIGHT→RIGHT, etc.), escape detection triggering DoRunAway, RunAwayAllowed 3-condition check, setupBattleInputOrder local-first ordering, BattleSong space-type selection, selectAllShips dispatch, frame timing calculation.
- **GREEN:** Implement `battle()`, `init_ships()`, `uninit_ships()`, `init_space()`, `uninit_space()`, `process_input()`, `count_crew_elements()`, `run_away_allowed()`, `setup_battle_input_order()`, `battle_song()`, `free_battle_song()`, `select_all_ships()`, `get_player_order()` in `lifecycle.rs`.
- **REFACTOR:** Consolidate asset management into an RAII guard pattern.

**Files to Modify:**
- `lifecycle.rs` — add all lifecycle orchestration to existing type-only module

**Pseudocode Traceability:** `pseudocode/battle-lifecycle.md` all lines

---

### Phase P12a: Lifecycle Verification

**Phase ID:** `PLAN-20260320-BATTLEPT2.P12a`

**Subagent:** rustreviewer

**Checklist:**
- [ ] Battle(): seed RNG, load music (BattleSong), InitShips, count ships, instant-victory check, set activity, configure scale/input order, spawn ships (selectAllShips), start music, enter DoInput
- [ ] Battle() cleanup: SuperMelee abort handling, netplay buffer cleanup, StopDitty+StopMusic+StopSound, UninitShips, FreeBattleSong
- [ ] InitShips: InitSpace → contexts → InitDisplayList → InitGalaxy → hyperspace path (ReinitQueues, BuildSIS via FFI, return 1) or encounter path (5 asteroids + 1 planet, return NUM_SIDES)
- [ ] InitSpace/UninitSpace: reference-counted (space_ini_cnt), loads stars/explosion/blast/asteroid at all zoom levels via load_animation FFI
- [ ] UninitShips: StopSound → UninitSpace → CountCrew → iterate elements → find survivor → add crew (cap at max) → record crew_level → free_ship → clear IN_BATTLE
- [ ] Encounter: persist crew via writeback (UpdateShipFragCrew)
- [ ] Non-encounter: ReinitQueue both sides + FreeHyperspace
- [ ] ProcessInput: iterate sides in battleInputOrder, for each active ship: get input, map bits, check escape (RunAwayAllowed + BATTLE_ESCAPE → DoRunAway)
- [ ] RunAwayAllowed: (IN_ENCOUNTER or IN_LAST_BATTLE) AND STARBASE_AVAILABLE AND !BOMB_CARRIER
- [ ] setupBattleInputOrder: non-netplay sequential; netplay local-first then network
- [ ] BattleSong: loads HYPERSPACE_MUSIC/QUASISPACE_MUSIC/BATTLE_MUSIC based on space type; conditionally plays
- [ ] FreeBattleSong: DestroyMusic, clear reference
- [ ] selectAllShips: num_ships==1 → GetNextStarShip(NULL,0); else → GetInitialStarShips
- [ ] GetPlayerOrder: netplay discriminant-based ordering; non-netplay returns i
- [ ] Frame timing: normal speed = SleepThreadUntil(next_time + BATTLE_FRAME_RATE / (speed + 1))
- [ ] Max speed: skip sleep, Async_process, TaskSwitch, suppress rendering
- [ ] InitShips returns i16 (negative for hyperspace exit)
- [ ] Instant-victory: positive ship count but zero ships for one side → skip frame loop
- [ ] Teardown robust: ships never fully spawned, absent hooks, already-freed descriptors
- [ ] No TODO/FIXME/HACK

---

### Phase P13: FFI Layer — Phase 3 Exports + C Bridge Wiring

**Phase ID:** `PLAN-20260320-BATTLEPT2.P13`

**Subagent:** rustcoder

**Prerequisites:** P12a PASS

**Requirements Implemented:**

| Requirement | Source |
|------------|--------|
| rust_battle_frame FFI export (DoBattle thin shell target) | Spec §9.2/§14 |
| rust_battle_init_ships / rust_battle_uninit_ships FFI exports | Spec §14 |
| C-side guards for battle.c, tactrans.c, intel.c, ship.c, init.c | Build toggle USE_RUST_BATTLE_LOOP |
| DoBattle thin shell wiring | Spec §9.2 (C retains DoBattle as callback, calls Rust) |
| compute_frame_checksum using Rust-owned display list | Spec §13.1 |
| Netplay frame-sync verification loop (compute CRC → notify → verify → abort on mismatch) | Spec §13.4 — integrated into DoBattle thin shell |
| Netplay input buffer hooks | Spec §13.3 |
| Netplay battle-end sync hooks (inBattle → endingBattle → endingBattle2 → interBattle) | Spec §13.5 — C-side callbacks (readyToEndCallback etc.) called via FFI from Rust readyForBattleEnd |

**TDD Cycle:**
- **RED:** Write tests for `rust_battle_frame` FFI round-trip (correct return type, null-safety), `rust_battle_init_ships` return value preservation (positive and negative i16), `compute_frame_checksum` against known display list states (bit-identical to C `crc_processState`), netplay frame-sync CRC verification integration.
- **GREEN:** Implement Phase 3 FFI exports in `ffi.rs`, wire `DoBattle` thin shell in `battle.c`, add C guards to all remaining C files, implement `compute_frame_checksum()` in `netplay.rs`.
- **REFACTOR:** Consolidate FFI error handling with catch_unwind.

**Files to Modify:**
- `ffi.rs` — EXTEND Phase 1's 15 FFI adapters with 4 Phase 3 exports: `rust_battle_frame`, `rust_battle_init_ships`, `rust_battle_uninit_ships`, `rust_battle_compute_checksum` (all 15 existing + P06's `rust_battle_redraw_queue` remain unchanged)
- `netplay.rs` — EXTEND Phase 1's CRC types with `compute_frame_checksum()` using Rust-owned display list (uses Phase 1's `crc_process_element()`)
- `battle.c` — thin `DoBattle` shell, guard `Battle()`, `ProcessInput()`, `RunAwayAllowed()`, `DoRunAway()`, `setupBattleInputOrder()`, `BattleSong()`, `FreeBattleSong()`, `selectAllShips()`, `GetPlayerOrder()` (keep `frameInputHuman` and `DoBattle` shell unguarded — see Permanent C Boundary)
- `tactrans.c` — guard all ported function bodies (keep 6 functions unguarded — see Permanent C Boundary)
- `intel.c` — guard `computer_intelligence()`
- `ship.c` — guard ship runtime functions (additional to `USE_RUST_SHIPS`)
- `init.c` — guard init/uninit functions (additional to `USE_RUST_SHIPS`; keep `load_animation`/`free_image`/`BuildSIS` unguarded — see Permanent C Boundary)

---

### Phase P13a: FFI Layer Verification

**Phase ID:** `PLAN-20260320-BATTLEPT2.P13a`

**Subagent:** rustreviewer

**Checklist:**
- [ ] `rust_battle_frame` declared with correct signature, returns i32 (C BOOLEAN)
- [ ] `rust_battle_init_ships` returns i16 (preserves negative hyperspace exit)
- [ ] `rust_battle_uninit_ships` declared void
- [ ] DoBattle thin shell: calls rust_battle_frame, returns result
- [ ] All C guards compile: battle.c, tactrans.c, intel.c, ship.c, init.c
- [ ] All C files compile WITHOUT USE_RUST_BATTLE_LOOP (original behavior preserved)
- [ ] All C files compile WITH USE_RUST_BATTLE_LOOP (Rust functions linked)
- [ ] compute_frame_checksum: bit-identical CRC to C crc_processState for same display list
- [ ] Netplay frame-sync: CRC computed → Netplay_NotifyAll_checksum via FFI → verifyChecksums via FFI → CHECK_ABORT + resetConnections on mismatch
- [ ] Netplay battle-end: readyForBattleEnd dispatches to C-side callbacks for netplay builds
- [ ] catch_unwind at all FFI boundaries
- [ ] All 15 Phase 1 FFI adapters still functional and unchanged
- [ ] No TODO/FIXME/HACK

---

### Phase P14: End-to-End Integration + Regression

**Phase ID:** `PLAN-20260320-BATTLEPT2.P14`

**Subagent:** rustcoder

**Prerequisites:** P13a PASS

**Requirements Implemented:**

| Requirement | Source |
|------------|--------|
| Full battle frame cycle (init→preprocess→collision→postprocess→render) | Spec §8.1 |
| Multi-frame sequences (ship spawn → combat → death → replacement) | Spec §10.1-10.3 |
| Determinism (bit-identical given same state+input) | Spec §13.4, Req netplay-10 |
| All Phase 1 tests regression | PLAN-20260320-BATTLE |

**TDD Cycle:**
- **RED:** Write integration tests for: full single-frame cycle (verify all stages execute), multi-frame battle (2 ships, one takes damage, dies, replacement spawned), flee sequence (initiate→pulse→warp-out→cleanup), collision scenario (two non-PLAYER_SHIP elements collide, elastic response applied, post-bounce rescans occur), determinism test (same initial state + inputs → same final state across 100 frames).
- **GREEN:** Wire all modules together. Verify all 2139+ Phase 1 tests still pass. Run new integration tests.
- **REFACTOR:** Final cleanup pass.

**Files to Modify:**
- `mod.rs` — update re-exports, add integration tests

**Verification Commands:**
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/
```

---

### Phase P14a: E2E Verification (Final Gate)

**Phase ID:** `PLAN-20260320-BATTLEPT2.P14a`

**Subagent:** rustreviewer

**Final Checklist:**
- [ ] All Phase 1 tests pass (2139+ tests, 246 battle-specific)
- [ ] All new Phase 2/3 tests pass
- [ ] `cargo fmt --all --check` clean
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean
- [ ] `cargo test --workspace --all-features` clean
- [ ] No TODO/FIXME/HACK/placeholder in any Phase 2/3 implementation code
- [ ] No `unwrap()` / `expect()` in production paths
- [ ] `catch_unwind` at all FFI boundaries
- [ ] All C files compile with USE_RUST_BATTLE_LOOP defined
- [ ] All C files compile without USE_RUST_BATTLE_LOOP defined
- [ ] Every C function from scope is either ported in a phase or listed in the "Permanent C Boundary" section (75 functions total: 17 process.c + 8 ship.c + 30 tactrans.c + 1 intel.c + 11 battle.c + 8 init.c = 64 ported + 11 permanent C boundary)
- [ ] Every deferred Phase 2+ requirement from Phase 1 plan is covered
- [ ] Integration traits fully implemented via c_bridge.rs FFI
- [ ] Display list ownership transferred (Rust-owned pool, C-owned display primitives accessed via FFI)
- [ ] Callback function pointers dispatched correctly when Rust owns the process loop
- [ ] Determinism verified (same state+input → same output across frames)

## Requirements Traceability Matrix — Phase 1 Deferred Items

Every item deferred to "Phase 2+" in PLAN-20260320-BATTLE is mapped to a specific phase in this plan. **Nothing remains unassigned.**

### Process Loop Orchestration (all deferred from Phase 1)

| Phase 1 Deferred Item | This Plan Phase | Notes |
|----------------------|----------------|-------|
| PreProcessQueue execution | P05 | Full queue orchestration |
| PostProcessQueue execution | P05 | Full queue orchestration |
| Asymmetric DEFY_PHYSICS clearing | P03 | Part of PostProcess implementation |
| Newly-added element cascading | P05 | Inner loop in PostProcessQueue |
| Scroll offset application | P05 | PostProcessQueue coordinate transforms |
| Scroll/transform/render insertion timing | P05 | PostProcessQueue rendering setup |
| Zoom calculation (step/continuous, hysteresis) | P05 | CalcReduction |
| Zoom hysteresis | P05 | CalcReduction |
| Camera calculation (midpoint, scroll clamping) | P05 | CalcView |
| Camera single-ship clamping | P05 | CalcView |
| World-to-screen conversion | P05 | PostProcessQueue coordinate transforms |

### Collision Orchestration (all deferred from Phase 1)

| Phase 1 Deferred Item | This Plan Phase | Notes |
|----------------------|----------------|-------|
| ProcessCollisions orchestration | P04 | Full recursive implementation |
| Pixel-accurate trajectory detection | P04 | Via DrawablesIntersect FFI |
| Forward-only vs full-list scan | P04 (forward), P05 (full-list) | Preprocess: successors; postprocess cascading: head |
| Recursive earlier-time checks | P04 | Recursive ProcessCollisions |
| Player-ship-first dispatch ordering | P04 | Dispatch logic |
| Collision-point snapping | P04 | Post-dispatch in ProcessCollisions |
| Stuck overlap handling | P04 | Stuck-overlap resolution loop |
| Post-bounce rechecks | P04 | Full-list rescan after elastic_collide |

### Battle Lifecycle Orchestration (all deferred from Phase 1)

| Phase 1 Deferred Item | This Plan Phase | Notes |
|----------------------|----------------|-------|
| Battle entry sequence | P12 | Battle() |
| Frame callback architecture | P13 | DoBattle thin shell → rust_battle_frame |
| Per-frame processing | P12, P13 | ProcessInput + frame dispatch |
| Frame timing (24 fps, max-speed) | P12 | Battle() timing logic |
| Input processing loop | P12 | ProcessInput() |
| Battle teardown sequencing | P12 | UninitShips() |
| Shared asset reference counting | P12 | InitSpace/UninitSpace |

### Tactical Transition Orchestration (all deferred from Phase 1)

| Phase 1 Deferred Item | This Plan Phase | Notes |
|----------------------|----------------|-------|
| Ship death 4-phase pipeline | P09 | ship_death callback chain |
| Explosion animation (36 frames, debris) | P09 | explosion_preprocess |
| Cleanup crew-pickup preservation | P09 | cleanup_dead_ship |
| New-ship readiness wait | P09 | new_ship handler |
| Ship replacement selection order | P08 | GetNextStarShip implements selection logic |
| Winner determination iteration | P10 | find_alive_starship |
| OpponentAlive semantics | P10 | OpponentAlive() |
| Ship death recording | P09 | RecordShipDeath() |
| Flee eligibility (5 conditions) | P10 | flee_preprocess eligibility check |
| Flee initiation (mass/color/timing) | P10 | DoRunAway() |
| Flee animation (20-color pulse) | P10 | flee_preprocess animation loop |
| Warp transition (15 frames, ghosts) | P10 | ship_transition |
| Warp ghost spawning | P10 | ship_transition ghost loop |
| Ion trail (12-color, head-insert, pre-processed) | P09 | spawn_ion_trail |

### Ship Runtime Pipeline Orchestration (all deferred from Phase 1)

| Phase 1 Deferred Item | This Plan Phase | Notes |
|----------------------|----------------|-------|
| Ship per-frame pipeline order | P07 | ship_preprocess 7-stage pipeline |
| Ship spawn placement | P08 | spawn_ship random/center placement |
| Ship first-frame initialization | P07 | APPEARING handling in ship_preprocess |
| Energy regeneration dispatch | P07 | ship_preprocess energy stage |
| Turn/thrust processing dispatch | P07 | ship_preprocess turn/thrust stages |
| Weapon firing pipeline | P07 | ship_postprocess weapon firing |

### AI Dispatch Orchestration (all deferred from Phase 1)

| Phase 1 Deferred Item | This Plan Phase | Notes |
|----------------------|----------------|-------|
| Computer intelligence entry point | P11 | computer_intelligence() |
| AI behavioral dispatch | P11 | CYBORG → tactical_intelligence |
| Object tracking system | P11 | Uses Phase 1 tracking indices |

### Netplay Orchestration (all deferred from Phase 1)

| Phase 1 Deferred Item | This Plan Phase | Notes |
|----------------------|----------------|-------|
| Input buffering | P12 | ProcessInput calls netplay buffer ops via FFI |
| Frame synchronization | P13 | compute_frame_checksum + CRC verification loop in DoBattle integration |
| Frame-sync verification loop (compare CRCs, abort on mismatch) | P13 | DoBattle thin shell: compute CRC → Netplay_NotifyAll_checksum → verifyChecksums → CHECK_ABORT + resetConnections |
| Battle-end multi-phase protocol (inBattle → endingBattle → endingBattle2 → interBattle) | P13 | C-side callbacks stay in C (see "Permanent C Boundary" section); Rust `ready_for_battle_end()` (P09) calls them via FFI |

### Display List Rendering Order (deferred from Phase 1)

| Phase 1 Deferred Item | This Plan Phase | Notes |
|----------------------|----------------|-------|
| Rendering-order linked list | P05 | InsertPrim in PostProcessQueue |

### Teardown/Double-Buffer Robustness (deferred from Phase 1)

| Phase 1 Deferred Item | This Plan Phase | Notes |
|----------------------|----------------|-------|
| Teardown robustness | P12 | UninitShips robustness |
| Double-buffer invariant consistency | P03 | PostProcess commit_state enforcement |

### Integration Operations (43 Phase 2+ operations from Phase 1)

> **Count note:** 43 operations are enumerated in the table below and wired in P06. A 44th Phase 2+ operation — `DrawablesIntersect` (GraphicsIntegration) — is wired earlier in P04's `c_bridge.rs` since ProcessCollisions needs it immediately. Phase 1 already declared 6 trait operations (3 Graphics + 1 Audio + 1 ShipRace + 1 GlobalState) which are excluded from both counts. The per-trait totals in the Integration Model section include both Phase 1 and Phase 2+ operations (50 total = 6 existing + 44 new).

| Phase 1 Deferred FFI Op | This Plan Phase | Notes |
|------------------------|----------------|-------|
| G1: Display primitive array access | P06 | c_bridge.rs |
| G2: Primitive free list management | P06 | c_bridge.rs |
| G4: Primitive property operations | P06 | c_bridge.rs |
| G5: Batch rendering entry | P06 | c_bridge.rs |
| G6: Graphic scale get/set | P06 | c_bridge.rs |
| G7: Scale mode operations | P06 | c_bridge.rs |
| G8: Drawable clear operations | P06 | c_bridge.rs |
| G9: Drawing context management | P06 | c_bridge.rs |
| G10: Clip rectangle operations | P06 | c_bridge.rs |
| G11: Background color/foreground frames | P06 | c_bridge.rs |
| G12: Screen transition operations | P06 | c_bridge.rs |
| G17: Trilinear mipmap setup | P06 | c_bridge.rs |
| G18: Primitive link management | P06 | c_bridge.rs |
| A2: Sound stopping | P06 | c_bridge.rs |
| A3: Element-positioned sound processing | P06 | c_bridge.rs |
| A4: Music playback | P06 | c_bridge.rs |
| A5: Music stopping | P06 | c_bridge.rs |
| A6: Stereo position calculation | P06 | c_bridge.rs |
| A7: Stereo position updating | P06 | c_bridge.rs |
| A8: Sound position removal | P06 | c_bridge.rs |
| A9: Sound flushing | P06 | c_bridge.rs |
| A10: Music-playing status query | P06 | c_bridge.rs |
| A11: Menu sound suppression | P06 | c_bridge.rs |
| T1: Cooperative yield | P06 | c_bridge.rs |
| T2: Timed sleep | P06 | c_bridge.rs |
| T3: Cooperative input loop | P06 | c_bridge.rs |
| I1: Per-player input handlers | P06 | c_bridge.rs |
| I2: Control flags | P06 | c_bridge.rs |
| I3: Frame-input polling | P06 | c_bridge.rs |
| I4: Raw-to-battle input conversion | P06 | c_bridge.rs |
| R1: Graphic asset loading | P06 | c_bridge.rs |
| R2: Drawable capture | P06 | c_bridge.rs |
| R3: Drawable release | P06 | c_bridge.rs |
| R4: Drawable destruction | P06 | c_bridge.rs |
| R5: Music destruction | P06 | c_bridge.rs |
| S2: Ship queue management | P06 | c_bridge.rs |
| S3: Ship loading/freeing | P06 | c_bridge.rs |
| S4: Energy management operations | P06 | c_bridge.rs |
| S5: Status bar initialization | P06 | c_bridge.rs |
| S6: Status bar update | P06 | c_bridge.rs |
| GS1: CurrentActivity flags | P06 | c_bridge.rs |
| GS2: Game state variables | P06 | c_bridge.rs |
| GS4: Space type detection | P06 | c_bridge.rs |

### Netplay CRC Serialization (Phase 1 complete, integrated here)

| Phase 1 Deferred Item | This Plan Phase | Notes |
|----------------------|----------------|-------|
| CRC field serialization (35-byte schema, field order, LE encoding, RNG seed inclusion) | Already implemented in Phase 1 P15 (`battle/netplay.rs`) | This plan's P13 integrates `crc_process_element()` into the frame-sync verification loop. Phase 1 provides the complete streaming CRC-32 over 19 fields in exact C byte order. P13 wires it into the `DoBattle` thin shell for per-frame checksum computation and mismatch abort. |

### First-Frame Screen Transition

| Phase 1 Deferred Item | This Plan Phase | Notes |
|----------------------|----------------|-------|
| Screen transition effect on first battle frame | P12 | Battle lifecycle (`Battle()` init and first-frame setup) handles the screen transition via `ScreenTransition()` FFI call during battle initialization, before the first frame callback fires. |

### Thread/Timing Behavioral Items

| Phase 1 Deferred Item | This Plan Phase | Notes |
|----------------------|----------------|-------|
| Max-speed skip mode (skip rendering, process faster) | P12 + P13 | P12 (`Battle()`) sets max-speed flags based on `ActivityFlags`. P13 (`DoBattle` thin shell) checks those flags to skip rendering while still running simulation. |
| Cooperative yield via `TaskSwitch()` | Permanent C Boundary | `TaskSwitch` is the threading subsystem's cooperative yield, not battle-specific. Called via `ThreadingIntegration` trait FFI wrapper (P06 `c_bridge.rs`), but scheduling policy stays in C's `DoInput()` framework. |
| Async task pump during blocking waits | Permanent C Boundary | `DoInput()` framework handles async task pumping during blocking waits (e.g., `new_ship` readiness wait). Battle code calls `DoInput()` via `ThreadingIntegration` trait; the framework itself is not ported. |

### Display Primitive Type-Specific Handling

| Phase 1 Deferred Item | This Plan Phase | Notes |
|----------------------|----------------|-------|
| Line endpoint wrapping | P05 | PostProcessQueue coordinate transform — line primitives have both endpoints transformed with toroidal wrap handling via `CalcDisplayCoord`. |
| Stamp/stamp-fill equivalent-frame selection | P05 | PostProcessQueue zoom-frame selection — stamp/stamp-fill primitives select the zoom-appropriate frame via `SetEquFrameIndex` based on current reduction level. |
| Trilinear/mipmap setup | P05 | PostProcessQueue zoom-frame selection — uses zoom constants from `process_loop.rs` (renamed from `process_types.rs` in P03). Setup is part of the render-prep logic in `PostProcessQueue`, not a separate subsystem. |

### Additional Phase 1 Deferred Items

| Phase 1 Deferred Item | This Plan Phase | Notes |
|----------------------|----------------|-------|
| Display primitive allocation coupling (AllocElement allocates element+prim as unit) | P03 | `alloc_element()` allocates both an element and its bound display primitive via FFI. `free_element()` frees both. |
| Element reuse (queue entry already has allocated handle → reinitialize in place) | P08 | `spawn_ship()` checks for existing `hShip` handle on queue entry and reinitializes in place instead of allocating new. |
| Sa-Matra center placement (defending ship) | P08 | `spawn_ship()` places defending ship at center (0,0) with facing=0 for Sa-Matra final battle. |
| Random position avoiding gravity wells | P08 | `spawn_ship()` rejection-samples random positions to avoid gravity well overlap. |
| Ship collision orchestration (gravity damage = hp/4, non-gravity elastic) | P07 | `ship_collision()` dispatches to Phase 1 `elastic_collide()` for non-gravity, computes hp/4 (min 1) damage for gravity wells. |
| Crew and energy management (regeneration, deduction, capping) | P07 | `ship_preprocess()` energy stage handles regeneration counter and `DeltaEnergy` dispatch. |
| Pkunk reincarnation (mass == MAX_SHIP_MASS + 1, treated as alive) | P10 | `find_alive_starship()` treats mass == MAX_SHIP_MASS + 1 as "alive" for winner determination. |
| Determinism obligations (bit-identical, no floating-point) | P14 | End-to-end verification confirms same RNG seed + same inputs → same frame checksum. Integer-only arithmetic enforced throughout. |
| Pool exhaustion robustness (no corruption, deterministic order) | P03 | `alloc_element()` returns `NULL_HANDLE` on exhaustion without corrupting pool state. |
| Top-level frame dispatch (SetContext → simulate → sounds → render) | P05 | `redraw_queue()` orchestrates the full frame: `SetContext` → `PreProcessQueue` → `PostProcessQueue` → `UpdateSoundPositions` → conditional render. |

### Other Deferred Items

| Phase 1 Deferred Item | This Plan Phase | Notes |
|----------------------|----------------|-------|
| InitShips return type mismatch fix | P13 | FFI layer uses correct i16 |
| Damage silhouette rendering | P07 (`ship_runtime.rs`) | Ship damage silhouette rendering is part of ship runtime postprocess in P07 |

## Integration Contract

### Existing Callers (C → Rust)

After this plan completes:
- `DoBattle()` (C) calls `rust_battle_frame()` (Rust) — per-frame processing
- `Battle()` path calls `rust_battle_init_ships()` / `rust_battle_uninit_ships()` (Rust) — lifecycle
- All Phase 1 callers continue unchanged (velocity, collision, weapon, CRC functions via the 15 existing FFI adapters)

### Existing Code Replaced

- `process.c` — ALL 17 function bodies guarded behind `USE_RUST_BATTLE_LOOP`
- `ship.c` — 8 ship runtime functions guarded (additional to `USE_RUST_SHIPS`)
- `tactrans.c` — 24 function bodies guarded (6 functions stay unguarded — see "Permanent C Boundary" section)
- `intel.c` — `computer_intelligence()` guarded
- `init.c` — 5 init/uninit functions guarded (additional to `USE_RUST_SHIPS`; 3 functions stay — see "Permanent C Boundary" section)
- `battle.c` — 9 functions guarded; 2 functions stay — see "Permanent C Boundary" section

### User Access Path

No change to user experience. SuperMelee battle runs identically. The toggle `USE_RUST_BATTLE_LOOP` controls whether C or Rust executes the battle logic. Both paths produce identical behavior.

### Data/State Migration

- Display list ownership: Rust-owned `DisplayList` pool. C-owned `DisplayArray`/`DisplayLinks` accessed via FFI.
- Global zoom state (`zoom_out`, `opt_max_zoom_out`): migrated to Rust process loop state.
- BattleState: `#[repr(C)]` struct, shared between C `DoInput()` and Rust.
- Callback function pointers: remain as C function pointers in `Element` struct. Rust dispatches them via `unsafe` calls.
- Winner tracking (`winnerStarShip`): migrated to Rust tactical module state.
- Space asset reference count (`space_ini_cnt`): migrated to Rust lifecycle module state.
- Battle music reference (`BattleRef`): migrated to Rust lifecycle module state.
- Battle input order (`battleInputOrder`): migrated to Rust lifecycle module state.
- Ditty state (`dittyIsPlaying`): migrated to Rust tactical module state.

### End-to-End Verification

- `cargo test --workspace --all-features` — all tests pass
- SuperMelee battle runs to completion with `USE_RUST_BATTLE_LOOP` enabled
- Determinism: same RNG seed + same inputs → same frame checksum

## Definition of Done

### Structural Gates
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test --workspace --all-features` passes
- [ ] No TODO/FIXME/HACK in any Phase 2/3 implementation code
- [ ] No `unwrap()` / `expect()` in production paths
- [ ] `catch_unwind` at all FFI boundaries

### Behavioral Gates
- [ ] All 2139+ Phase 1 tests still pass (regression)
- [ ] Process loop: PreProcessQueue → PostProcessQueue → render produces correct element states
- [ ] ProcessCollisions: recursive earlier-time checks, stuck-overlap resolution, post-bounce rescans
- [ ] Ship runtime: 7-stage pipeline in exact order, inertial thrust physics, weapon firing
- [ ] Tactical: 4-phase death pipeline, 36-frame explosion, flee pulse, warp ghost images, winner determination
- [ ] AI: all 4 dispatch paths (Sa-Matra, CYBORG, PSYTRON, direct)
- [ ] Lifecycle: InitShips/UninitShips reference counting, crew writeback
- [ ] Netplay: CRC bit-identical to C for same display list
- [ ] Netplay: frame-sync verification loop (CRC compare → abort on mismatch)
- [ ] Netplay: battle-end multi-phase protocol dispatches correctly
- [ ] Determinism: same state+input → same output

### ABI Gates
- [ ] All C files compile with `USE_RUST_BATTLE_LOOP` defined
- [ ] All C files compile without `USE_RUST_BATTLE_LOOP` defined
- [ ] `DoBattle` thin shell correctly calls `rust_battle_frame` and returns result
- [ ] `BattleState` layout matches C (InputFunc at offset 0)
- [ ] Element `#[repr(C)]` layout unchanged from Phase 1

### Migration Gates
- [ ] Ships subsystem tests pass (47 ships tests)
- [ ] Battle/ships integration: ship callbacks dispatch correctly through Rust process loop
- [ ] Phase 1 FFI adapters still functional (velocity, collision, weapon, CRC)

## Cross-Plan Dependencies

| Dependency | Direction | Notes |
|-----------|-----------|-------|
| PLAN-20260314-SHIPS P14 (C-Side Bridge Wiring) | Ships depends on Battle P04 (Element type) and P09 (weapon types) — already satisfied by Phase 1 | No new dependency |
| Ships `USE_RUST_SHIPS` guards | Battle `USE_RUST_BATTLE_LOOP` is independent but additive | Both toggles can be active simultaneously |
| Graphics subsystem (Phase 2 port) | Battle depends on graphics for DrawablesIntersect, primitives, contexts | Via FFI in c_bridge.rs |
| Netplay subsystem | Battle provides hooks; netplay owns transport | Hooks defined in P13 |

## Plan Files

```text
project-plans/20260311/battlept2/
  plan/
    00-overview.md                                  (this file)
    00a-preflight-verification.md                   P00.5
    01-analysis.md                                  P01
    01a-analysis-verification.md                    P01a
    02-pseudocode.md                                P02
    02a-pseudocode-verification.md                  P02a
    03-process-prepost.md                           P03
    03a-process-prepost-verification.md             P03a
    04-process-collisions.md                        P04
    04a-process-collisions-verification.md          P04a
    05-queue-orchestration-zoom-camera.md           P05
    05a-queue-orchestration-verification.md         P05a
    06-c-bridge-ffi.md                              P06
    06a-c-bridge-ffi-verification.md                P06a
    07-ship-runtime-pipeline.md                     P07
    07a-ship-runtime-verification.md                P07a
    08-ship-spawn-init.md                           P08
    08a-ship-spawn-verification.md                  P08a
    09-death-explosion.md                           P09
    09a-death-explosion-verification.md             P09a
    10-flee-warp-winner.md                          P10
    10a-flee-warp-winner-verification.md            P10a
    11-ai-dispatch.md                               P11
    11a-ai-dispatch-verification.md                 P11a
    12-battle-lifecycle.md                           P12
    12a-battle-lifecycle-verification.md             P12a
    13-ffi-layer-phase3.md                          P13
    13a-ffi-layer-verification.md                   P13a
    14-e2e-integration.md                           P14
    14a-e2e-verification.md                         P14a
    execution-tracker.md
  analysis/
    domain-model.md                                 (P01 output)
  pseudocode/
    process-loop.md                                 (P02 output)
    process-collisions.md                           (P02 output)
    zoom-camera.md                                  (P02 output)
    ship-runtime.md                                 (P02 output)
    tactical-transitions.md                         (P02 output)
    battle-lifecycle.md                             (P02 output)
  .completed/
    (phase completion markers)
```

## Execution Tracker

29 phases total. 14 implementation phases + 14 verification phases + 1 preflight. Builds on Phase 1's 7,591 lines of existing Rust code and 2,139 tests.

| Phase | Title | Status | Verified | Semantic Verified | Est. LoC | C Functions Ported | Notes |
|------:|-------|--------|----------|-------------------|---------|-------------------|-------|
| P00.5 | Preflight Verification | ⬜ | ⬜ | N/A | 0 | 0 | Verify Phase 1 artifacts + toolchain |
| P01 | Analysis | ⬜ | ⬜ | ⬜ | 0 | 0 | Domain model + dependency graph |
| P01a | Analysis Verification | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P02 | Pseudocode | ⬜ | ⬜ | ⬜ | 0 | 0 | 6 algorithm files |
| P02a | Pseudocode Verification | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P03 | Process PreProcess/PostProcess | ⬜ | ⬜ | ⬜ | ~800 | 7 | PreProcess, PostProcess, AllocElement, FreeElement, SetUpElement, Untarget, RemoveElement |
| P03a | Process PrePost Verification | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P04 | ProcessCollisions | ⬜ | ⬜ | ⬜ | ~700 | 1 | ProcessCollisions (recursive) |
| P04a | ProcessCollisions Verification | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P05 | Queue Orchestration + Zoom/Camera | ⬜ | ⬜ | ⬜ | ~600 | 9 | PreProcessQueue, PostProcessQueue, RedrawQueue, CalcReduction, CalcView, InitDisplayList, InsertPrim, CalcDisplayCoord, CALC_ZOOM_STUFF |
| P05a | Queue Orchestration Verification | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P06 | C Bridge FFI Wiring | ⬜ | ⬜ | ⬜ | ~500 | 0 | 43 FFI declarations + C guards (extends Phase 1 ffi.rs) |
| P06a | C Bridge Verification | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P07 | Ship Runtime Pipeline | ⬜ | ⬜ | ⬜ | ~700 | 5 | animation_preprocess, inertial_thrust, ship_preprocess, ship_postprocess, collision(ship) |
| P07a | Ship Runtime Verification | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P08 | Ship Spawn + Init | ⬜ | ⬜ | ⬜ | ~400 | 3 | spawn_ship, GetNextStarShip, GetInitialStarShips |
| P08a | Ship Spawn Verification | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P09 | Death + Explosion | ⬜ | ⬜ | ⬜ | ~800 | 17 | ship_death, StartShipExplosion, explosion_preprocess, cleanup_dead_ship, new_ship, plus 12 helpers (all from tactrans.c) |
| P09a | Death Pipeline Verification | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P10 | Flee + Warp + Winner | ⬜ | ⬜ | ⬜ | ~600 | 8 | flee_preprocess, ship_transition, find_alive_starship, OpponentAlive, ResetWinnerStarShip, GetWinnerStarShip, SetWinnerStarShip (7 from tactrans.c) + DoRunAway (1 from battle.c) |
| P10a | Flee/Warp/Winner Verification | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P11 | AI Dispatch | ⬜ | ⬜ | ⬜ | ~200 | 1 | computer_intelligence |
| P11a | AI Dispatch Verification | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P12 | Battle Lifecycle | ⬜ | ⬜ | ⬜ | ~800 | 13 | Battle, InitShips, UninitShips, InitSpace, UninitSpace, ProcessInput, CountCrewElements, + 6 helpers |
| P12a | Lifecycle Verification | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P13 | FFI Layer Phase 3 | ⬜ | ⬜ | ⬜ | ~500 | 0 | FFI exports + C shell wiring + netplay CRC (extends Phase 1 ffi.rs) |
| P13a | FFI Layer Verification | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P14 | E2E Integration | ⬜ | ⬜ | ⬜ | ~200 | 0 | Wire all modules, regression test |
| P14a | E2E Verification (Final Gate) | ⬜ | ⬜ | ⬜ | 0 | 0 | All gates must pass |
| | | | | | | | |
| | **Totals** | | | | **~6,800** | **64 ported** | **+ 11 permanent C boundary = 75 total** |

### Execution Rules

1. Phases execute in strict order: P00.5 → P01 → P01a → ... → P14 → P14a
2. Each phase MUST be completed and verified before the next begins
3. No skipping phases
4. No multi-phase batching
5. Phase completion requires creating `project-plans/20260311/battlept2/.completed/PNN.md`
6. Failed verification triggers remediation loop (fix → re-verify)
