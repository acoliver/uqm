# Phase 06: C Bridge — Phase 2 FFI Wiring

## Phase ID
`PLAN-20260320-BATTLEPT2.P06`

## Prerequisites
- Required: Phase 05a (Queue Orchestration Verification) completed with PASS
- Expected files: `process_loop.rs` with P03+P04+P05 functions
- Expected artifacts: Full process loop verified, collision_bridge helper exists in process_loop.rs

## Requirements Implemented (Expanded)

### REQ: FFI boundary safety (battlept2/requirements.md §FFI boundary safety)
**Requirement text**: No Rust panic shall cross an FFI boundary. Foreign pointers validated before dereference. Borrowed C pointers not cached across frame boundaries. Handle-based re-lookups after callbacks.

Behavior contract:
- GIVEN: A Rust function callable from C or calling into C
- WHEN: The bridge function is invoked
- THEN: Panic containment is in place; null checks performed; pointers not cached across callback boundaries

### REQ: Callback-slot safety (battlept2/requirements.md §Callback-slot safety)
**Requirement text**: Callback-bearing fields remain C-ABI function pointers. Rust closures/trait objects never stored in C callback fields. Stale callback dispatch prevented.

Behavior contract:
- GIVEN: A bridge wrapper that dispatches through a callback
- WHEN: The callback is invoked
- THEN: Non-null check performed; element validity verified; stale callbacks treated as no-op

### REQ: Build-mode coexistence (battlept2/requirements.md §Build-mode coexistence)
**Requirement text**: C-only baseline build works. Rust code presence doesn't alter C-only path. Guards are dark-code/test-only before P13.

Behavior contract:
- GIVEN: `USE_RUST_BATTLE_LOOP` is disabled
- WHEN: process.c compiles
- THEN: Original C function bodies remain active and unchanged

## Implementation Tasks

### Files to create

- `rust/src/battle/c_bridge.rs` — Canonical Rust→C bridge layer
  - marker: `@plan PLAN-20260320-BATTLEPT2.P06`
  - marker: `@requirement REQ-FFI-SAFETY, REQ-CALLBACK-SAFETY, REQ-BUILD-COEXISTENCE`
  - Contents:
    - Move `collision_bridge()` from process_loop.rs to c_bridge.rs as `pub fn drawables_intersect()`
    - All deferred bridge operations organized by trait family. The spec §6.2 counts 44 conceptual trait operations deferred from Phase 1. However, the ported functions in P03–P12 additionally call numerous C helper/utility functions that must also have bridge wrappers. The bridge must cover all C functions called from Rust, organized below into the trait families plus additional helper categories:
      - **GraphicsIntegration** (~17 conceptual wrappers): `set_context`, `batch_graphics`, `unbatch_graphics`, `clear_drawable`, `get_frame_index`, `set_prim_type`, `set_prim_color`, `set_prim_stamp_frame`, `get_equiv_frame`/`SetEquFrameIndex`, `get_frame_rect`, `get_frame_count`, `set_graphic_scale_mode`/`SetGraphicScale`, `screen_transition`, `drawables_intersect`/`DrawablesIntersect`, `init_intersect_start_point`/`InitIntersectStartPoint`, `init_intersect_end_point`/`InitIntersectEndPoint`, `set_trilinear_mipmap`/`TFB_DrawScreen_SetMipmap`
      - **AudioIntegration** (~11 conceptual wrappers): `play_sound_effect`, `stop_sound`/`StopSound`, `process_sound`/`ProcessSound`, `flush_sounds`/`FlushSounds`, `update_sound_positions`/`UpdateSoundPositions`, `play_music`/`PlayMusic`, `stop_music`/`StopMusic`, `music_playing`/`PLRPlaying`, `play_ditty_sfx`, `stop_ditty_sfx`, `set_menu_sounds`/`SetMenuSounds`
      - **ThreadingIntegration** (~3 conceptual wrappers): `sleep_thread`, `sleep_thread_until`/`SleepThreadUntil`, `task_switch`/`TaskSwitch`
      - **InputIntegration** (~4 conceptual wrappers): `current_input_to_battle_input`/`CurrentInputToBattleInput`, `get_player_input`, `do_input_wrapper`/`DoInput`, `flush_input`
      - **ResourceIntegration** (~5 conceptual wrappers): `capture_drawable`/`CaptureDrawable`, `release_drawable`/`ReleaseDrawable`, `destroy_drawable`/`DestroyDrawable`, `load_music_instance`/`LoadMusic`, `destroy_music`/`DestroyMusic`
      - **ShipRaceIntegration** (~6 conceptual wrappers per spec §6.2): `get_element_starship`/`GetElementStarShip`, `set_element_starship`/`SetElementStarShip`, `lock_element`/`LockElement`, `unlock_element`/`UnlockElement`, `lock_starship`/`LockStarShip`, `unlock_starship`/`UnlockStarShip`
      - **GlobalStateIntegration** (~4 conceptual wrappers per spec §6.2): `get_current_activity`, `set_current_activity`, `get_game_state`/`GET_GAME_STATE`, `in_hq_space`/`inHQSpace`
      - **Additional helper wrappers** needed by P03–P12 ported functions (not counted in the 44 conceptual operations but required for implementation):
        - **Display list traversal**: `GetHeadElement`, `GetSuccElement`, `PutElement`, `InsertElement`, `CountLinks`, `GetPredLink`/`GetSuccLink`, `MakeLinks`, `GetPrimLinks`/`SetPrimLinks`, `AllocDisplayPrim`/`FreeDisplayPrim`
        - **Frame/image manipulation**: `SetAbsFrameIndex`, `IncFrameIndex`, `GetFrameHot`, `SetAbsSoundIndex`
        - **Ship queue traversal**: `GetEncounterStarShip`, `GetHeadLink`, `_GetSuccLink`, `Build` (for BuildSIS)
        - **Status display**: `InitShipStatus`, `DrawCaptainsWindow`, `PreProcessStatus`, `PostProcessStatus`, `DrawStamp`
        - **Space/galaxy setup**: `InitGalaxy`, `LoadHyperspace`, `FreeHyperspace`, `spawn_asteroid`, `spawn_planet`, `free_gravity_well`, `CalculateGravity`, `TimeSpaceMatterConflict`
        - **Ship resource management**: `load_ship`, `free_ship`, `UpdateShipFragCrew`, `FleetIsInfinite`
        - **Combat/damage**: `do_damage`, `DeltaEnergy`, `GRAVITY_MASS` (macro), `collide` (elastic_collide entry)
        - **Velocity**: `SetVelocityVector`, `SetVelocityComponents`, `GetCurrentVelocityComponents`, `ZeroVelocityComponents`, `DeltaVelocityComponents`, `VelocitySquared`, `GetVelocityTravelAngle` (some may be Phase 1 pure-Rust; bridge only for C-implemented ones)
        - **RNG/timing**: `TFB_Random`, `TFB_SeedRandom`, `GetTimeCounter`
        - **Hyperspace/transitions**: `MoveSIS`, `MoveGalaxy`, `hyper_transition`, `ship_transition`
        - **Melee-specific**: `GetInitialMeleeStarShips`, `MeleeShipDeath`, `MeleeGameOver`, `GetPlayerOrder` (macro)
        - **Queue management**: `ReinitQueue` (for race_q), `RemoveQueue`/`AllocLink`/`FreeLink` (disp_q operations)
        - **Misc helpers**: `OBJECT_CLOAKED` (macro), `spawn_ion_trail`, `crew_preprocess` (callback comparison), `WRAP_X`/`WRAP_Y`/`WRAP_DELTA_X`/`WRAP_DELTA_Y` (macros), `DISPLAY_ALIGN`/`DISPLAY_ALIGN_X`/`DISPLAY_ALIGN_Y` (macros), `DISPLAY_TO_WORLD`/`WORLD_TO_VELOCITY` (macros), `NORMALIZE_FACING`/`FACING_TO_ANGLE` (macros), `COSINE`/`SINE` (macros)
      - **Note on macros**: Many of the "helpers" above are C preprocessor macros, not functions. These cannot be called via FFI directly. For macro-only operations, the bridge either: (a) wraps the macro in a thin C helper function, or (b) reimplements the macro logic in Rust. The choice depends on complexity — simple arithmetic macros (WRAP_X, DISPLAY_TO_WORLD, NORMALIZE_FACING, COSINE/SINE) should be reimplemented in Rust (many already are from Phase 1 `battle_types.rs`); complex macros that access global state should get thin C wrapper functions.
    - Each function wrapper: extern "C" declaration of C function → safe Rust wrapper with null checks
    - FFI safety per spec §10: panic containment not needed on Rust→C calls (only on C→Rust entries), but null/validity checks required
    - The final bridge wrapper count will exceed 44 because the 44 count is conceptual trait operations; the additional helpers above are concrete C API calls needed by the ported behavioral logic

### Files to modify

- `rust/src/battle/process_loop.rs`
  - Remove `collision_bridge()` private helper; replace with `c_bridge::drawables_intersect()` call
  - marker: `@plan PLAN-20260320-BATTLEPT2.P06`

- `rust/src/battle/ffi.rs` — Add `rust_battle_redraw_queue` export
  - marker: `@plan PLAN-20260320-BATTLEPT2.P06`
  - Adds: `#[no_mangle] pub extern "C" fn rust_battle_redraw_queue(force_redraw: i32)` → calls `process_loop::redraw_queue()`

- `rust/src/battle/mod.rs`
  - Add `pub mod c_bridge;`
  - marker: `@plan PLAN-20260320-BATTLEPT2.P06`

- `sc2/src/uqm/process.c` — Dark-code/test guard plumbing
  - marker: `@plan PLAN-20260320-BATTLEPT2.P06`
  - Add `#ifdef USE_RUST_BATTLE_LOOP` / `#else` / `#endif` structure around function bodies (NOT around declarations/prototypes)
  - **Important**: USE_RUST_BATTLE_LOOP is NOT enabled yet — this adds the guard structure only
  - Add `extern` declarations for Rust replacement functions

### Deliverables
- `c_bridge.rs` with all 44 bridge wrappers
- `ffi.rs` extended with `rust_battle_redraw_queue`
- `process.c` with guard plumbing (dark-code, not active)
- Pre-P13 build/link map artifact documenting which symbols exist in which mode

### C reference functions ported
P06 ports no C functions. It creates bridge infrastructure.

### C branches to handle
- `USE_RUST_BATTLE_LOOP` guard plumbing in process.c (dark-code, not enabled)

### Integration points
- All 7 Phase 1 integration traits → concrete C function wrappers
- P05 process_loop.rs: redraw_queue → ffi.rs export
- P04 process_loop.rs: collision_bridge → c_bridge.rs migration

### Pseudocode traceability (if impl phase)
- N/A (infrastructure phase, not algorithm)

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
# Verify process.c still compiles in C-only mode (USE_RUST_BATTLE_LOOP not defined)
# make -C sc2 (or equivalent C build verification)
```

## Structural Verification Checklist
- [ ] `c_bridge.rs` created with all 44 bridge wrappers
- [ ] `ffi.rs` has `rust_battle_redraw_queue` export
- [ ] `mod.rs` declares `pub mod c_bridge;`
- [ ] `collision_bridge()` removed from process_loop.rs, replaced with c_bridge call
- [ ] process.c has `#ifdef USE_RUST_BATTLE_LOOP` guards (dark-code, not enabled)
- [ ] process.c compiles normally (guards are inactive)
- [ ] Pre-P13 build/link map artifact produced

## Semantic Verification Checklist (Mandatory)
- [ ] All 44 deferred bridge operations have wrappers
- [ ] Each wrapper has correct C function extern declaration
- [ ] Null/validity checks on pointer arguments per spec §10.3 categories
- [ ] No Rust panic can reach C (C→Rust entry in ffi.rs has catch_unwind)
- [ ] collision_bridge migration: process_loop.rs now calls c_bridge::drawables_intersect()
- [ ] process.c original behavior preserved when USE_RUST_BATTLE_LOOP undefined
- [ ] All Phase 1 FFI adapters in ffi.rs are unchanged
- [ ] Callback-slot migration matrix documented (spec §8.1)
- [ ] Thread-affinity documented per wrapper family (spec §10.4)
- [ ] No placeholder/deferred implementation patterns

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/c_bridge.rs rust/src/battle/ffi.rs
```

## Success Criteria
- [ ] All 44 bridge wrappers complete
- [ ] FFI safety rules enforced
- [ ] C-only build unaffected
- [ ] All tests pass
- [ ] Callback-slot matrix produced

## Failure Recovery
- rollback: `git checkout -- rust/src/battle/c_bridge.rs rust/src/battle/ffi.rs rust/src/battle/mod.rs rust/src/battle/process_loop.rs sc2/src/uqm/process.c`
- blocking issues: C function signatures don't match Rust extern declarations

## Phase Completion Marker
Create: `project-plans/20260311/battlept2/.completed/P06.md`

Contents:
- phase ID: PLAN-20260320-BATTLEPT2.P06
- timestamp
- files created: c_bridge.rs
- files changed: ffi.rs, mod.rs, process_loop.rs, process.c
- tests added/updated
- verification outputs (including callback-slot matrix, build/link map)
- semantic verification summary
