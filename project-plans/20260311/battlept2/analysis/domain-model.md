# Battle Engine Phase 2/3 — Domain Model & Analysis

@plan PLAN-20260320-BATTLEPT2.P01

## 1. Dependency Graph

### 1.1 Rust Module Dependencies (Phase 2/3 target state)

```
process_loop.rs ──uses──► element.rs (Element, ElementFlags, commit_state)
                ──uses──► velocity.rs (get_next_components)
                ──uses──► collision.rs (collision_possible, elastic_collide)
                ──uses──► display_list.rs (alloc/free/iter/push_back/remove)
                ──uses──► battle_types.rs (coords, angles, trig)
                ──uses──► c_bridge.rs (DrawablesIntersect, display prim ops)

ship_runtime.rs ──uses──► element.rs (Element, ElementFlags)
                ──uses──► velocity.rs (set_vector, get_next_components)
                ──uses──► battle_types.rs (SINE/COSINE, facing)
                ──uses──► weapon.rs (weapon_collision, LaserBlock, MissileBlock)
                ──uses──► process_loop.rs (alloc_element, setup_element)
                ──uses──► c_bridge.rs (graphics/audio ops)

tactical.rs     ──uses──► element.rs (Element, ElementFlags)
                ──uses──► ship_runtime.rs (animation_preprocess)
                ──uses──► process_loop.rs (alloc_element, remove_element)
                ──uses──► c_bridge.rs (audio, graphics, resource ops)
                ──uses──► lifecycle.rs (BattleState constants)

ai.rs           ──uses──► c_bridge.rs (input ops)
                ──uses──► battle_types.rs (control flags)

lifecycle.rs    ──uses──► process_loop.rs (init_display_list, redraw_queue)
                ──uses──► ship_runtime.rs (spawn_ship, get_next_starship)
                ──uses──► tactical.rs (ship_death, flee helpers, music)
                ──uses──► ai.rs (computer_intelligence)
                ──uses──► c_bridge.rs (all subsystem ops)

c_bridge.rs     ──uses──► integration.rs (trait definitions)
                ──calls──► C subsystems via FFI

ffi.rs          ──exports──► 17 Phase 1 adapters (unchanged)
                ──exports──► Phase 3 entry points (rust_battle_frame, etc.)
```

### 1.2 C→Rust Call Chain (Rust-enabled mode)

```
DoInput() loop
  └► DoBattle() [retained C thin shell]
       └► rust_battle_frame() [FFI export in ffi.rs]
            └► lifecycle::battle_frame()
                 ├► process_loop::redraw_queue()
                 │    ├► pre_process_queue()
                 │    │    ├► pre_process() per element
                 │    │    │    └► callback dispatch (ship_preprocess, etc.)
                 │    │    └► process_collisions()
                 │    │         └► collision dispatch (ship_collision, etc.)
                 │    ├► post_process_queue()
                 │    │    └► post_process() per element
                 │    │         └► callback dispatch (ship_postprocess, etc.)
                 │    └► render (via c_bridge graphics ops)
                 ├► netplay::crc_process (if NETPLAY_CHECKSUM)
                 └► audio updates (via c_bridge)

Battle() [C wrapper → rust_battle_entry]
  └► lifecycle::battle()
       ├► init_ships()
       ├► DoInput(DoBattle) [C DoInput loop]
       └► uninit_ships()
```

### 1.3 Phase Execution Dependencies

```
P03 (PreProcess/PostProcess) ← foundation for all queue iteration
P04 (ProcessCollisions) ← depends on P03 element lifecycle
P05 (Queue Orchestration) ← depends on P03+P04, orchestrates full frame
P06 (C Bridge) ← provides FFI layer consumed by P03-P05 and all later phases
P07 (Ship Runtime) ← depends on P03 (element ops) + P06 (bridge)
P08 (Ship Spawn) ← depends on P07 (ship callbacks)
P09 (Death/Explosion) ← depends on P07 (animation_preprocess) + P08 (new_ship spawns)
P10 (Flee/Warp/Winner) ← depends on P09 (death triggers winner search)
P11 (AI Dispatch) ← depends on P06 (bridge for input)
P12 (Lifecycle) ← depends on all above (orchestrates full battle)
P13 (FFI Layer) ← depends on P12 (wires everything to C)
P14 (E2E) ← final integration gate
```

## 2. Function-by-Function Mapping

### 2.1 Complete 75-Function Inventory

See `battlept2/specification.md` §12.1 for the authoritative table. Summary by Rust target module:

#### `process_loop.rs` (17 functions, P03+P04+P05)

| # | C Function | C File:Lines | Phase | Phase 1 Types Used |
|--:|-----------|-------------|------:|-------------------|
| 14 | `AllocElement()` | process.c:91-134 | P03 | DisplayList::alloc, Element |
| 15 | `FreeElement()` | process.c:136-161 | P03 | DisplayList::free, Element |
| 16 | `SetUpElement()` | process.c:163-205 | P03 | Element, ElementFlags |
| 18 | `Untarget()` | process.c:60-87 | P03 | Element (hTarget field) |
| 19 | `RemoveElement()` | process.c:206-225 | P03 | DisplayList::remove |
| 20 | `PreProcess()` | process.c:227-361 | P03 | Element, ElementFlags, VelocityDesc |
| 22 | `PostProcess()` | process.c:596-734 | P03 | Element, ElementFlags, commit_state() |
| 21 | `ProcessCollisions()` | process.c:362-593 | P04 | collision_possible(), elastic_collide() |
| 12 | `CalcReduction()` | process.c:736-808 | P05 | ViewState, ZoomMode |
| 13 | `CalcView()` | process.c:810-948 | P05 | ViewState |
| 17 | `InsertPrim()` | process.c:950-990 | P05 | DisplayList prim ops |
| 23 | `CalcDisplayCoord()` | process.c:992-1010 | P05 | Coords |
| 24 | `PreProcessQueue()` | process.c:1012-1043 | P05 | — |
| 25 | `PostProcessQueue()` | process.c:1045-1098 | P05 | — |
| 26 | `InitDisplayList()` | process.c:1100-1108 | P05 | DisplayList |
| 27 | `RedrawQueue()` | process.c:1010-1098 | P05 | — |
| 28 | `InitKernel()` | process.c:44-58 | P05 | — |

#### `ship_runtime.rs` (8 functions, P07+P08)

| # | C Function | C File:Lines | Phase | Phase 1 Types Used |
|--:|-----------|-------------|------:|-------------------|
| 59 | `animation_preprocess()` | ship.c:46-87 | P07 | Element, CHANGING flag |
| 60 | `inertial_thrust()` | ship.c:89-157 | P07 | VelocityDesc, SINE/COSINE, MAX_ALLOWED_SPEED |
| 61 | `ship_preprocess()` | ship.c:159-293 | P07 | Element, ElementFlags, ShipPipelineStage |
| 62 | `ship_postprocess()` | ship.c:295-391 | P07 | Element, weapon.rs (init_weapon, do_damage) |
| 63 | `collision()` (ship) | ship.c:393-461 | P07 | Element, collision funcs |
| 64 | `spawn_ship()` | ship.c:463-515 | P08 | Element, ElementFlags, APPEARING|PLAYER_SHIP |
| 65 | `GetNextStarShip()` | ship.c:518-552 | P08 | Queue traversal |
| 66 | `GetInitialStarShips()` | ship.c:554-592 | P08 | — |

#### `tactical.rs` (25 functions, P09+P10)

| # | C Function | C File:Lines | Phase | Phase 1 Types Used |
|--:|-----------|-------------|------:|-------------------|
| 54 | `ship_death()` | tactrans.c:729-749 | P09 | DeathPipelinePhase |
| 53 | `StartShipExplosion()` | tactrans.c:612-660 | P09 | Element, FINITE_LIFE|NONSOLID |
| 47 | `explosion_preprocess()` | tactrans.c:542-610 | P09 | Element, animation_preprocess (P07) |
| 42 | `cleanup_dead_ship()` | tactrans.c:287-374 | P09 | Element |
| 46 | `new_ship()` | tactrans.c:376-540 | P09 | — |
| 56 | `spawn_ion_trail()` | tactrans.c:662-727 | P09 | Element, POINT_PRIM |
| 55 | `cycle_ion_trail()` | tactrans.c:755-783 | P09 | Color table |
| 30 | `PlayDitty()` | tactrans.c:78-96 | P09 | — |
| 31 | `StopDitty()` | tactrans.c:98-106 | P09 | — |
| 32 | `DittyPlaying()` | tactrans.c:68-76 | P09 | — |
| 48 | `StopAllBattleMusic()` | tactrans.c:623-628 | P09 | — |
| 41 | `preprocess_dead_ship()` | tactrans.c:281-285 | P09 | — |
| 52 | `RecordShipDeath()` | tactrans.c:683-727 | P09 | — |
| 40 | `readyForBattleEnd()` | tactrans.c:253-278 | P09 | — |
| 43 | `setMinShipLifeSpan()` | tactrans.c:56-62 | P09 | — |
| 44 | `setMinStarShipLifeSpan()` | tactrans.c:64-66 | P09 | — |
| 45 | `checkOtherShipLifeSpan()` | tactrans.c:42-54 | P09 | — |
| 58 | `flee_preprocess()` | tactrans.c:785-886 | P10 | Color table, timing |
| 57 | `ship_transition()` | tactrans.c:888-1032 | P10 | Element, ghost images |
| 2 | `DoRunAway()` | battle.c:68-135 | P10 | Element, APPEARING |
| 49 | `FindAliveStarShip()` | tactrans.c:630-660 | P10 | Element, PLAYER_SHIP |
| 29 | `OpponentAlive()` | tactrans.c:18-40 | P10 | Element |
| 33 | `ResetWinnerStarShip()` | tactrans.c:662-665 | P10 | — |
| 50 | `GetWinnerStarShip()` | tactrans.c:667-670 | P10 | — |
| 51 | `SetWinnerStarShip()` | tactrans.c:672-681 | P10 | — |

#### `ai.rs` (1 function, P11)

| # | C Function | C File:Lines | Phase | Phase 1 Types Used |
|--:|-----------|-------------|------:|-------------------|
| 75 | `computer_intelligence()` | intel.c:1-76 | P11 | EvaluateDesc, AI control flags |

#### `lifecycle.rs` (13 functions, P12)

| # | C Function | C File:Lines | Phase | Phase 1 Types Used |
|--:|-----------|-------------|------:|-------------------|
| 11 | `Battle()` | battle.c:396-516 | P12 | BattleState, BATTLE_FRAME_RATE |
| 72 | `InitShips()` | init.c:186-277 | P12 | — |
| 74 | `UninitShips()` | init.c:279-363 | P12 | — |
| 69 | `InitSpace()` | init.c:115-153 | P12 | — |
| 70 | `UninitSpace()` | init.c:155-182 | P12 | — |
| 5 | `ProcessInput()` | battle.c:145-177 | P12 | BattleInputState |
| 73 | `CountCrewElements()` | init.c:253-274 | P12 | Element |
| 1 | `RunAwayAllowed()` | battle.c:63-67 | P12 | ActivityFlags |
| 3 | `setupBattleInputOrder()` | battle.c:180-220 | P12 | — |
| 6 | `BattleSong()` | battle.c:222-262 | P12 | — |
| 7 | `FreeBattleSong()` | battle.c:264-280 | P12 | — |
| 10 | `selectAllShips()` | battle.c:282-320 | P12 | — |
| 9 | `GetPlayerOrder()` | battle.c:380-394 | P12 | — |

#### Retained C Boundary (11 functions, not ported)

| # | C Function | C File | Category |
|--:|-----------|--------|----------|
| 4 | `frameInputHuman()` | battle.c | DoInput callback |
| 8 | `DoBattle()` | battle.c | Retained ABI shell (→thin shell in P13) |
| 37 | `battleEndReadyHuman()` | tactrans.c | DoInput callback |
| 38 | `battleEndReadyComputer()` | tactrans.c | DoInput callback |
| 39 | `battleEndReadyNetwork()` | tactrans.c | DoInput callback |
| 34 | `readyToEnd2Callback()` | tactrans.c | Netplay transport |
| 35 | `readyToEndCallback()` | tactrans.c | Netplay transport |
| 36 | `readyForBattleEndPlayer()` | tactrans.c | Netplay transport |
| 67 | `load_animation()` | init.c | Resource subsystem |
| 68 | `free_image()` | init.c | Resource subsystem |
| 71 | `BuildSIS()` | init.c | Ships subsystem |

## 3. Integration Touchpoint Inventory

### 3.1 Bridge Operation Summary

Total conceptual integration operations: 50 (across 8 traits)
Phase 1 used: 6
Phase 2/3 deferred bridge operations: 44

### 3.2 Bridge Operations by Trait

#### BattleGraphics (17 operations, ~13 deferred)

| Operation | Consuming Phase | C Function(s) Called |
|-----------|:--------------:|---------------------|
| `get_primitive_type` | P05 | `GetPrimType()` |
| `get_frame_count` | P07 | `GetFrameCount()` |
| `get_frame_rect` | P05,P07 | frame rect queries |
| `drawables_intersect` | P04 | `DrawablesIntersect()` |
| `set_context_foreground_color` | P09,P10 | `SetContextForeGroundColor()` |
| `draw_stamp` | P05 | `DrawStamp()` |
| `draw_line` | P05 | `DrawLine()` |
| `draw_point` | P09 | `DrawPoint()` |
| `batch_graphics_begin` | P05 | `BatchGraphics()` |
| `batch_graphics_end` | P05 | `UnbatchGraphics()` |
| `set_graphics_scale` | P05 | scale configuration |
| `get/set_scale_mode` | P05 | `TFB_DrawScreen_Scale()` |
| `clear_drawable` | P12 | `ClearDrawable()` |
| `set_context` | P05,P12 | `SetContext()` |
| `set_clip_rect` | P05 | `SetContextClipRect()` |
| `get_background_color` | P05 | `GetContextBackgroundColor()` |
| `screen_transition` | P12 | `ScreenTransition()` |

#### BattleAudio (11 operations, ~10 deferred)

| Operation | Consuming Phase | C Function(s) Called |
|-----------|:--------------:|---------------------|
| `play_sound` | P07,P09 | `PlaySound()` |
| `stop_sound` | P12 | `StopSound()` |
| `process_sound_for_element` | P05 | `ProcessSound()` |
| `play_music` | P09,P12 | `PLRPlaySong()` |
| `stop_music` | P09,P12 | `PLRStop()` |
| `calculate_stereo_position` | P05 | position calc |
| `update_stereo_position` | P05 | `UpdateSoundPositions()` |
| `remove_sound_position` | P03 | `RemoveSoundEffect()` |
| `flush_sounds` | P05 | flush |
| `is_music_playing` | P09 | `PLRPlaying()` |
| `suppress_menu_sounds` | P12 | suppression flag |

#### BattleThreading (3 operations)

| Operation | Consuming Phase | C Function(s) Called |
|-----------|:--------------:|---------------------|
| `task_switch` | P09,P12 | `TaskSwitch()` |
| `sleep_thread_until` | P11 | `SleepThreadUntil()` |
| `do_input` | P12 | `DoInput()` |

#### BattleInput (4 operations)

| Operation | Consuming Phase | C Function(s) Called |
|-----------|:--------------:|---------------------|
| `get_input_state` | P12 | `PlayerInput[].state` |
| `get_player_control` | P07,P11 | `PlayerControl[]` |
| `poll_frame_input` | P12 | frame input poll |
| `raw_input_to_battle_input` | P11,P12 | `CurrentInputToBattleInput()` |

#### BattleResources (5 operations)

| Operation | Consuming Phase | C Function(s) Called |
|-----------|:--------------:|---------------------|
| `load_graphic` | P12 | `CaptureDrawable(LoadGraphic())` |
| `capture_drawable` | P12 | `CaptureDrawable()` |
| `release_drawable` | P12 | `ReleaseDrawable()` |
| `destroy_drawable` | P03 | `DestroyDrawable()` |
| `destroy_music` | P12 | `DestroyMusic()` |

#### BattleShipInterface (6+ operations)

| Operation | Consuming Phase | C Function(s) Called |
|-----------|:--------------:|---------------------|
| `get_race_preprocess` | P07 | Race-specific preprocess callback |
| `get_race_postprocess` | P07 | Race-specific postprocess callback |
| `get_race_intelligence` | P11 | Race-specific AI callback |
| `load/free_ship_descriptor` | P08,P09 | Ship descriptor management |
| `modify_ship_energy` | P07 | `DeltaEnergy()` |
| `init/update_status_bar` | P07,P08 | Status bar ops |

#### BattleGlobalState (4 operations)

| Operation | Consuming Phase | C Function(s) Called |
|-----------|:--------------:|---------------------|
| `get_random` | P08 | `TFB_Random()` |
| `get_activity_flags` | P10,P11,P12 | `LOBYTE(GLOBAL(CurrentActivity))` |
| `set_activity_flags` | P12 | activity flag mutation |
| `get_game_state` | P10,P12 | `GLOBAL_SIS()` / `GET_GAME_STATE()` |

## 4. State Management Analysis

### 4.1 Display List Ownership

The display list (element pool + linked list) is C-owned. Rust accesses it through `#[repr(C)]` Element structs via raw pointers obtained from C. Key invariants:

- **No parallel Rust copy.** Rust operates on the same backing storage as C (spec §2.2).
- **Pointer lifetime.** Element pointers borrowed from C are valid only within a single process-loop iteration. After any callback dispatch, list mutation, or element free/reuse, pointers must be re-looked-up from stable handle identity.
- **Allocation/free.** `AllocElement()`/`FreeElement()` in Rust wrap the C display list pool operations (in Phase 1 `display_list.rs`).

### 4.2 DisplayArray/DisplayLinks Globals

`DisplayArray[]` and `DisplayLinks[]` are C global arrays indexed by element's `PrimIndex`. Rust accesses them via FFI:
- `InsertPrim()` modifies `DisplayLinks[]` for render ordering.
- `PostProcess()` reads/writes `DisplayArray[]` for coordinate transforms.
- `SetPrimType()`, `GetPrimType()` operate on `DisplayArray[]` entries.

### 4.3 Queue Ownership

`race_q[NUM_SIDES]` (ship queues) are C-owned. Rust iterates them via FFI for ship selection, crew counting, and battle counter management. Queue entries contain `hShip` handles back to display list elements.

## 5. Callback Function Pointer Analysis

### 5.1 Callback Slots in Element Struct

| Slot | C Type | Current C Targets | Rust Replacement |
|------|--------|-------------------|-----------------|
| `preprocess_func` | `void (*)(ELEMENT*)` | `ship_preprocess`, `explosion_preprocess`, `flee_preprocess`, `preprocess_dead_ship`, race-specific | Rust `extern "C"` fn |
| `postprocess_func` | `void (*)(ELEMENT*)` | `ship_postprocess`, race-specific | Rust `extern "C"` fn |
| `collision_func` | `void (*)(ELEMENT*, POINT*, ELEMENT*, POINT*)` | `collision` (ship), `weapon_collision`, race-specific | Rust `extern "C"` fn |
| `death_func` | `void (*)(ELEMENT*)` | `ship_death`, `cycle_ion_trail`, race-specific | Rust `extern "C"` fn |

### 5.2 Dispatch Model

When Rust owns the process loop, callback dispatch works as:
1. Process loop reads `element.preprocess_func` (a C function pointer).
2. Calls it via FFI — the target may be a Rust `extern "C"` function or a retained C function.
3. After the callback returns, all element/list pointers are invalidated and must be re-looked-up.

### 5.3 Callback Installation

When Rust creates/reconfigures elements (spawn_ship, StartShipExplosion, etc.), it writes Rust `extern "C"` function pointers into the callback slots. The C struct stores them as raw function pointers — no trait objects or closures.

## 6. Display Primitive Coupling

Rust process loop interacts with C `DisplayArray[]` through:
1. **PostProcess coordinate transforms** — writes world→screen coords into `DisplayArray[PrimIndex]`
2. **InsertPrim render ordering** — inserts into `DisplayLinks[]` sorted list
3. **SetPrimType** — marks element visibility (NO_PRIM to hide)
4. **GetPrimType** — reads prim type for render decisions

All access goes through `c_bridge.rs` wrappers that call the C macros/functions.

## 7. Branch-Parity Inventory

### 7.1 Compile-Time Branch Families

| Family | Source Sites | Affected Phases | Verification Strategy |
|--------|-------------|:---------------:|----------------------|
| `NETPLAY` / `NETPLAY_CHECKSUM` | battle.c:30,58,112,150,182,220,268,287,350,356,383,438,440,457,482,498,500; tactrans.c:31,108,245,257,483,510,531 | P09,P12,P13 | Feature flag `netplay`; test both paths |
| `NETPLAY_DEBUG` | tactrans.c:127,144 | P09 | Debug logging; map to `tracing::debug!` |
| `DEMO_MODE` / `CREATE_JOURNAL` | battle.c:179,228,402 | P12 | Feature flag `demo`; test with/without |
| `USE_RUST_SHIPS` | ship.c:38,158,295,396; init.c:38,184,279 | P07,P08,P12 | Already exists; P13 adds `USE_RUST_BATTLE_LOOP` |
| `KDEBUG` | process.c:211,276,290,354,638,742,806,980 | P03,P04,P05 | Tracing behind feature flag |
| `DEBUG_PROCESS` | process.c:401,523,545 | P04 | Tracing behind feature flag |
| Dead code (`#if 0`) | process.c:47 | — | Not ported |

### 7.2 Runtime Branch Families

| Family | Condition | Affected Phases | Sites |
|--------|----------|:---------------:|-------|
| `SUPER_MELEE` | `activity == SUPER_MELEE` | P09,P12 | battle.c, tactrans.c |
| `CHECK_ABORT`/`CHECK_LOAD` | Activity flag test | P12 | battle.c cleanup |
| `IN_ENCOUNTER`/`IN_LAST_BATTLE` | `LOBYTE(CurrentActivity)` | P07,P08,P10,P12 | init.c, tactrans.c, battle.c |
| `inHyperSpace()`/`inQuasiSpace()` | Runtime space check | P12 | init.c, battle.c |
| Max-speed render skip | `maxSpeedTraveling()` | P05,P12 | battle.c, process.c |

## 8. FFI Safety Matrix

### 8.1 Phase 1 FFI Exports (17 — unchanged)

All 17 Phase 1 exports follow the pattern: receive raw pointers → validate non-null → operate on `#[repr(C)]` data → return i32 status. No panic can escape (pure computation, no unwinding paths).

### 8.2 Phase 2/3 FFI Boundaries

| Boundary | Direction | Ownership | Lifetime | Thread | Panic | Reentrant |
|----------|-----------|-----------|----------|--------|-------|-----------|
| `rust_battle_frame` | C→Rust | Borrowed BATTLE_STATE | Single frame | DoInput thread | catch_unwind | No |
| `rust_battle_entry` | C→Rust | Transfers battle ownership | Battle duration | DoInput thread | catch_unwind | No |
| `rust_battle_init_ships` | C→Rust | Borrowed state | Init scope | DoInput thread | catch_unwind | No |
| `rust_battle_uninit_ships` | C→Rust | Borrowed state | Cleanup scope | DoInput thread | catch_unwind | No |
| `rust_battle_computer_intelligence` | C→Rust | Borrowed ship state | Single call | DoInput thread | catch_unwind | No |
| `rust_battle_song` / `_free_song` | C→Rust | Borrowed resource | Asset lifetime | DoInput thread | catch_unwind | No |
| `rust_battle_get_player_order` | C→Rust | Query only | Instant | DoInput thread | catch_unwind | No |
| `rust_battle_compute_checksum` | C→Rust | Borrowed display list | Single frame | DoInput thread | catch_unwind | No |
| All 44 bridge wrappers | Rust→C | Borrowed C state | Per-call | DoInput thread | N/A (C side) | Some (callbacks) |

### 8.3 Pointer Safety Categories (per spec §10.3)

| Category | Pointers | Rule |
|----------|----------|------|
| Always-nonnull borrowed | `ELEMENT*`, `STARSHIP*`, `BATTLE_STATE*` from internal paths | Assert at boundary, borrow within scope |
| Nullable/optional | `hTarget`, `p_parent`, winner handles | Check before dereference |
| Callback-invalidation-sensitive | Any pointer across callback dispatch | Re-lookup from stable handle after callback |

### 8.4 Thread Affinity

All battle state is single-thread-affine to the DoInput execution thread. No `Send`/`Sync` bounds required. All FFI wrappers document: "DoInput-thread only."
