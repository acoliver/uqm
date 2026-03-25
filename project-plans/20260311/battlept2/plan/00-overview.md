# Plan: Battle Engine — Phase 2/3 Full Logic Port

Plan ID: PLAN-20260320-BATTLEPT2
Generated: 2026-03-20
Predecessor: PLAN-20260320-BATTLE (Phase 1 — types + leaf functions, COMPLETE, 2,151 tests)
Total Phases: 29 (1 preflight P00.5 + P01-P14 with verification sub-phases P01a-P14a)
Phase breakdown: 1 preflight (P00.5) + 2 preparatory (P01-P02) + 12 implementation (P03-P14) + 14 verification (P01a-P14a)
Requirements: `battle/requirements.md` (shared) + `battlept2/requirements.md` (addendum)
Specification: `battle/specification.md` (shared, §1–§18) + `battlept2/specification.md` (Phase 2/3 addendum)

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 0.5)
2. Integration points are explicitly listed
3. TDD cycle is defined per slice
4. Lint/test/coverage gates are declared
5. Phase 1 types and leaf functions (7,591 lines, 2,151 tests / 229 battle-specific) are the foundation — use them, do not redefine them

## Context

Phase 1 (PLAN-20260320-BATTLE) created Rust types and leaf math functions that C calls via FFI. All 37 phases passed. Phase 1 created these modules in `rust/src/battle/`:

| Module | Lines | Contents |
|--------|------:|----------|
| `battle_types.rs` | 398 | Coords, angles, trig, SINE_TABLE |
| `element.rs` | 943 | Element #[repr(C)], ElementFlags, lifecycle helpers |
| `velocity.rs` | 682 | VelocityDesc, Bresenham accumulation suite |
| `display_list.rs` | 899 | Pool allocator, generational handles, linked-list ops |
| `collision.rs` | 558 | elastic_collide(), isqrt, eligibility checks |
| `weapon.rs` | 949 | LaserBlock, MissileBlock, weapon_collision, blast creation, tracking |
| `process_types.rs` *(current name; renamed to `process_loop.rs` as the first commit of P03)* | 95 | ViewState, ZoomMode, zoom/camera constants |
| `lifecycle.rs` | 123 | BattleState type, frame rate constants |
| `ship_runtime_types.rs` *(current name; renamed to `ship_runtime.rs` as the first commit of P07)* | 128 | ShipPipelineStage, spawn constants |
| `tactical.rs` | 187 | Death pipeline enum, explosion/flee/warp constants |
| `ai_types.rs` *(current name; renamed to `ai.rs` as the first commit of P11)* | 141 | EvaluateDesc, AI constants, control flags |

| `netplay.rs` | 586 | CRC-32, crc_process_element(), protocol type defs |
| `integration.rs` | 860 | 7 trait interfaces (Graphics, Audio, Threading, Input, Resource, ShipRace, GlobalState) |
| `ffi.rs` | 510 | 17 Phase 1 FFI adapters |
| `mod.rs` | 532 | Module declarations, re-exports, integration tests |
| **Total** | **7,591** | |

### Phase 1 FFI Adapters (Retained Unchanged)

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
| `rust_battle_sine` | Trig sine lookup |
| `rust_battle_cosine` | Trig cosine lookup |
| `rust_battle_arctan` | Trig arctan lookup |

## What This Plan Ports

This plan ports 64 C functions across 6 source files. 11 functions remain as permanent C boundary surfaces (see `battlept2/specification.md` §3). After completion, Rust owns all battle logic except those 11 retained surfaces.

> **Authoritative inventory:** The complete 75-function inventory with per-function status, phase assignment, and Rust target module is maintained in `battlept2/specification.md` §12 (Master Function Inventory). The tables below are per-phase assignment summaries for implementation sequencing — the spec table governs in case of discrepancy.

> **Design decisions:** The ownership transfer model, symbol preservation strategy, boundary function selection criteria, callback-slot migration rules, and guard/toggle staging rationale are defined in `battlept2/specification.md` §2–§9. This plan does not restate those decisions — it references them.



### Per-Phase Function Assignments

The complete 75-function inventory (all 64 ported + 11 retained) with per-function C file, status, phase, Rust target, and notes is in `battlept2/specification.md` §12.1. The tables below summarize phase assignments for implementation sequencing.

#### P03: Process Loop — PreProcess + PostProcess (7 functions → `process_loop.rs`)

`PreProcess()`, `PostProcess()`, `AllocElement()`, `FreeElement()`, `SetUpElement()`, `Untarget()`, `RemoveElement()`

#### P04: ProcessCollisions (1 function → `process_loop.rs`)

`ProcessCollisions()`

#### P05: Queue Orchestration + Zoom/Camera (9 functions → `process_loop.rs`)

`CalcReduction()`, `CalcView()`, `InsertPrim()`, `CalcDisplayCoord()`, `PreProcessQueue()`, `PostProcessQueue()`, `InitDisplayList()`, `RedrawQueue()`, `InitKernel()`

#### P07: Ship Runtime Pipeline (5 functions → `ship_runtime.rs`)

`animation_preprocess()`, `inertial_thrust()`, `ship_preprocess()`, `ship_postprocess()`, `collision()` (ship)

#### P08: Ship Spawn + Init (3 functions → `ship_runtime.rs`)

`spawn_ship()`, `GetNextStarShip()`, `GetInitialStarShips()`

#### P09: Death + Explosion (17 functions → `tactical.rs`)

`ship_death()`, `StartShipExplosion()`, `explosion_preprocess()`, `cleanup_dead_ship()`, `new_ship()`, `spawn_ion_trail()`, `cycle_ion_trail()`, `PlayDitty()`, `StopDitty()`, `DittyPlaying()`, `StopAllBattleMusic()`, `preprocess_dead_ship()`, `RecordShipDeath()`, `readyForBattleEnd()`, `setMinShipLifeSpan()`, `setMinStarShipLifeSpan()`, `checkOtherShipLifeSpan()`

#### P10: Flee + Warp + Winner (8 functions → `tactical.rs`)

`flee_preprocess()`, `ship_transition()`, `DoRunAway()`, `FindAliveStarShip()`, `OpponentAlive()`, `ResetWinnerStarShip()`, `GetWinnerStarShip()`, `SetWinnerStarShip()`

#### P11: AI Dispatch (1 function → `ai.rs`)

`computer_intelligence()`

#### P12: Battle Lifecycle (13 functions → `lifecycle.rs`)

`Battle()`, `InitShips()`, `UninitShips()`, `InitSpace()`, `UninitSpace()`, `ProcessInput()`, `CountCrewElements()`, `RunAwayAllowed()`, `setupBattleInputOrder()`, `BattleSong()`, `FreeBattleSong()`, `selectAllShips()`, `GetPlayerOrder()`

#### Retained permanent C boundary (11 functions — not ported)

See `battlept2/specification.md` §3.1 for the complete list and justifications: `frameInputHuman()`, `DoBattle()` (thin shell in P13), `battleEndReadyHuman()`, `battleEndReadyComputer()`, `battleEndReadyNetwork()`, `readyToEnd2Callback()`, `readyToEndCallback()`, `readyForBattleEndPlayer()`, `load_animation()`, `free_image()`, `BuildSIS()`.



## Phase Structure

| Phase | Title | Est. LoC | TDD Summary |
|------:|-------|---------|-------------|
| P00.5 | Preflight Verification | 0 | Verify Phase 1 artifacts, toolchain, existing tests |
| P01 | Analysis | 0 | Domain model, dependency graph, integration touchpoints |
| P01a | Analysis Verification | 0 | Confirm all C functions covered, no gaps |
| P02 | Pseudocode | 0 | Algorithmic pseudocode for all 6 core algorithms |
| P02a | Pseudocode Verification | 0 | Confirm pseudocode matches C behavior |
| P03 | Process Loop — PreProcess + PostProcess | ~800 | RED: per-element flag transitions, velocity stepping. GREEN: PreProcess/PostProcess. REFACTOR: extract helpers. Rename `process_types.rs` → `process_loop.rs` first. |
| P03a | Process Loop Verification | 0 | Verify flag transitions, velocity stepping, APPEARING handling |
| P04 | Process Loop — ProcessCollisions | ~900 | RED: collision dispatch ordering, stuck-overlap, recursive rechecks. GREEN: full ProcessCollisions. REFACTOR: reduce recursive complexity. |
| P04a | ProcessCollisions Verification | 0 | Verify recursive behavior, dispatch ordering, post-bounce rescans |
| P05 | Process Loop — Queue Orchestration + Zoom/Camera | ~600 | RED: camera tracking, cascading, zoom hysteresis. GREEN: queue orchestration with transforms and zoom. REFACTOR: consolidate transforms. |
| P05a | Queue Orchestration Verification | 0 | Verify cascading, scroll offsets, zoom modes, camera clamping |
| P06 | C Bridge — Phase 2 FFI Wiring | ~500 | RED: FFI round-trip tests. GREEN: c_bridge.rs with 44 deferred bridge wrappers + process.c guards. REFACTOR: consolidate FFI patterns. |
| P06a | C Bridge Verification | 0 | Verify all 44 bridge operations covered, FFI safety, C guards compile |
| P07 | Ship Runtime Pipeline | ~700 | RED: 7-stage pipeline order, thrust physics, energy regen. GREEN: ship_preprocess/postprocess. REFACTOR: extract stages. Rename `ship_runtime_types.rs` → `ship_runtime.rs` first. |
| P07a | Ship Runtime Verification | 0 | Verify pipeline order, thrust physics, weapon firing sequence |
| P08 | Ship Spawn + Init | ~400 | RED: spawn placement, element init. GREEN: spawn_ship, GetNextStarShip. REFACTOR: consolidate init. |
| P08a | Ship Spawn Verification | 0 | Verify placement, element fields, queue binding |
| P09 | Death + Explosion | ~1000 | RED: 4-phase callback chain, 36-frame animation. GREEN: all 17 tactrans.c death/explosion functions. REFACTOR: extract debris spawning. |
| P09a | Death Pipeline Verification | 0 | Verify callback chain, frame milestones, crew preservation |
| P10 | Flee + Warp + Winner | ~600 | RED: flee eligibility, 20-color pulse, warp ghosts. GREEN: 8 flee/warp/winner functions. REFACTOR: consolidate transitions. |
| P10a | Flee/Warp/Winner Verification | 0 | Verify flee eligibility, pulse timing, warp materialization |
| P11 | AI Dispatch | ~200 | RED: all 4 dispatch paths. GREEN: computer_intelligence(). REFACTOR: simplify flow. Rename `ai_types.rs` → `ai.rs` first. |
| P11a | AI Dispatch Verification | 0 | Verify all 4 dispatch paths |
| P12 | Battle Lifecycle | ~800 | RED: InitShips sequence, UninitShips teardown, ProcessInput mapping. GREEN: 13 lifecycle functions. REFACTOR: consolidate asset management. |
| P12a | Lifecycle Verification | 0 | Verify init sequence, reference counting, teardown robustness |
| P13 | FFI Layer Phase 3 | ~500 | RED: rust_battle_frame entry, init/uninit round-trip. GREEN: FFI exports + DoBattle shell + C guards. REFACTOR: consolidate error handling. |
| P13a | FFI Layer Verification | 0 | Verify DoBattle→rust_battle_frame path, C guard compilation |
| P14 | E2E Integration | ~200 | Wire all modules, regression test |
| P14a | E2E Verification (Final Gate) | 0 | All gates must pass |

**Total estimated new/modified LoC: ~7,200 Rust + ~400 C**

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

**Subagent:** deepthinker

**Toolchain Verification:**
- [ ] `cargo --version` (1.75+ required)
- [ ] `rustc --version`
- [ ] `cargo clippy --version`
- [ ] `cargo fmt --version`

**Codebase Verification:**
- [ ] All 15 Phase 1 battle modules exist and compile: `cargo check -p uqm`
- [ ] All 229 Phase 1 battle tests pass: `cargo test --lib battle::`
- [ ] Phase 1 FFI functions are exported: verify `ffi.rs` contains all 17 adapters
- [ ] Phase 1 integration traits exist: verify `integration.rs` has 7 trait definitions
- [ ] `process_types.rs`, `ship_runtime_types.rs`, `ai_types.rs`, `lifecycle.rs`, `tactical.rs` contain type-only definitions ready for extension

**C Source Verification:**
- [ ] `process.c`, `battle.c`, `tactrans.c`, `intel.c` are unmodified (no `USE_RUST_BATTLE_LOOP` guards)
- [ ] `ship.c` has `USE_RUST_SHIPS` guards but no `USE_RUST_BATTLE_LOOP`
- [ ] `init.c` has `USE_RUST_SHIPS` guards but no `USE_RUST_BATTLE_LOOP`

**Integration Point Verification:**
- [ ] `DrawablesIntersect`, display primitive array globals, `SetContext`/`BatchGraphics`/`UnbatchGraphics`, `DoInput` framework, `PlayerInput`/`CurrentInputToBattleInput` all exist and are callable

**Gate:** PASS → proceed to P01. FAIL → revise plan.

---

### Phase P01: Analysis

**Subagent:** deepthinker | **Prerequisites:** P00.5 PASS

**Deliverables:**
1. Dependency graph — Rust module dependencies and C→Rust call chain
2. Function-by-function mapping — every C function → Rust target + Phase 1 types used
3. Integration touchpoint inventory — all 44 deferred bridge operations
4. State management analysis — display list ownership transfer model
5. Callback function pointer analysis — dispatch when Rust owns process loop
6. Display primitive coupling analysis — Rust process loop + C DisplayArray globals
7. Branch-parity inventory — all compile-time/runtime branches (spec §13)
8. FFI safety matrix — ownership, lifetime, thread-affinity, panic, reentrancy per boundary

**Output:** `battlept2/analysis/domain-model.md`

---

### Phase P01a: Analysis Verification

**Subagent:** rustreviewer

**Checklist:**
- [ ] All 75 C functions mapped (64 ported + 11 permanent C boundary)
- [ ] All 44 deferred bridge operations accounted for
- [ ] Display list ownership transfer explicitly described
- [ ] Callback dispatch mechanism explicitly described
- [ ] Branch-parity inventory covers all families (spec §13.1)
- [ ] DoBattle thin-shell contract is explicit (spec §4)
- [ ] FFI safety matrix complete (spec §10)

---

### Phase P02: Pseudocode

**Subagent:** deepthinker | **Prerequisites:** P01a PASS

**Pseudocode Files:**
1. `pseudocode/process-loop.md` — PreProcess, PostProcess, PreProcessQueue, PostProcessQueue, RedrawQueue, InitDisplayList, AllocElement, FreeElement, SetUpElement, InsertPrim, Untarget, RemoveElement, CALC_ZOOM_STUFF, CalcDisplayCoord
2. `pseudocode/process-collisions.md` — ProcessCollisions (recursive), stuck-overlap, post-dispatch snapping, post-bounce rescans
3. `pseudocode/zoom-camera.md` — CalcReduction (step + continuous), CalcView (midpoint, clamping, zoom transition)
4. `pseudocode/ship-runtime.md` — ship_preprocess (7-stage pipeline), ship_postprocess, inertial_thrust, spawn_ship, ship collision, animation_preprocess
5. `pseudocode/tactical-transitions.md` — All 24 ported tactrans.c functions
6. `pseudocode/battle-lifecycle.md` — Battle(), InitShips, UninitShips, InitSpace/UninitSpace, ProcessInput, CountCrewElements, computer_intelligence, helpers

Each file includes: validation points, error handling, ordering constraints, FFI call markers, Phase 1 type references.

---

### Phase P02a: Pseudocode Verification

**Subagent:** rustreviewer

**Checklist:**
- [ ] Every C function has corresponding pseudocode
- [ ] Pseudocode matches C reference behavior exactly
- [ ] ProcessCollisions recursion correctly captured
- [ ] Flag transitions match specification §4.2 table
- [ ] Callback replacement chains correctly sequenced
- [ ] FFI calls explicitly marked
- [ ] Phase 1 type references accurate
- [ ] explosion_preprocess notes animation_preprocess dependency (tactrans.c:606 → ship.c:46)

---

### Phase P03: Process Loop — PreProcess + PostProcess

**Subagent:** rustcoder | **Prerequisites:** P02a PASS | **Est. LoC:** ~800

**Requirements:** Spec §8.3/§8.4 (PreProcess/PostProcess per-element), Spec §4.2 (DEFY_PHYSICS asymmetric clearing), all element lifecycle flag transitions.

**Functions:** `pre_process()`, `post_process()`, `setup_element()`, `alloc_element()`, `free_element()`, `untarget()`, `remove_element()`

**TDD Cycle:**
- **RED:** Tests for PreProcess flag transitions, velocity stepping, APPEARING handling (PLAYER_SHIP vs non-PLAYER_SHIP), life_span=0 death, FINITE_LIFE decrement, CHANGING intersection reinit. Tests for PostProcess asymmetric DEFY_PHYSICS clearing, commit_state().
- **GREEN:** Implement in `process_loop.rs` using Phase 1 Element methods and VelocityDesc.
- **REFACTOR:** Extract shared flag-transition helpers.

**Commit 1 (rename-only):** Rename `process_types.rs` → `process_loop.rs` and update `mod.rs` imports. This commit contains NO logic changes — only the file rename and import path updates. This ensures `git log --follow` tracks the file history correctly.

**Commit 2+:** Add logic to `process_loop.rs` as described in the TDD cycle above.


---

### Phase P03a: Process Loop Verification

**Subagent:** rustreviewer

**Checklist:**
- [ ] PreProcess/PostProcess flag transitions match spec §4.2 exactly
- [ ] Asymmetric DEFY_PHYSICS: COLLISION set → clear COLLISION keep DEFY; no COLLISION → clear DEFY
- [ ] APPEARING: PLAYER_SHIP clears in local copy only; non-PLAYER_SHIP skips preprocess callback
- [ ] Velocity stepping uses Phase 1 `get_next_components()`
- [ ] FINITE_LIFE decrement after velocity stepping
- [ ] Death callback on life_span==0, DISAPPEARING set, Untarget called
- [ ] All Phase 1 tests still pass (229 battle-specific tests)
- [ ] `cargo fmt/clippy/test` clean

---

### Phase P04: Process Loop — ProcessCollisions

**Subagent:** rustcoder | **Prerequisites:** P03a PASS | **Est. LoC:** ~900

**Requirements:** Spec §6.4 (recursive orchestration), collision dispatch ordering, stuck-overlap, snapping, post-bounce rescans, COLLISION flag as re-entry guard, DrawablesIntersect FFI.

**Functions:** `process_collisions()` + private `collision_bridge` helper for DrawablesIntersect FFI

**TDD Cycle:**
- **RED:** Tests for dispatch ordering (PLAYER_SHIP first), stuck-overlap (APPEARING kill, position revert), recursive earlier-time checks, collision-point snapping, post-bounce rescan. Mock DrawablesIntersect.
- **GREEN:** Full ProcessCollisions with recursive structure matching C exactly. Uses Phase 1 `collision_possible()`, `elastic_collide()`.
- **REFACTOR:** Reduce stack depth where safe.

**Implementation note:** Handle-based traversal and staged re-lookups — no mutable borrows across recursive descent, callback dispatch, or list mutation.

---

### Phase P04a: ProcessCollisions Verification

**Subagent:** rustreviewer

**Checklist:**
- [ ] Recursive structure matches C process.c:362-628
- [ ] PLAYER_SHIP test element's handler called first
- [ ] COLLISION flag set on both elements after dispatch
- [ ] Stuck overlap: APPEARING killed, non-APPEARING reverted
- [ ] Position snapping: next.location = collision point
- [ ] Post-bounce: elastic_collide → ProcessCollisions re-called from head
- [ ] Recursive earlier-time: check both elements before dispatching
- [ ] PreProcess called on unprocessed elements during successor walk
- [ ] DrawablesIntersect called via private collision_bridge helper (moves to c_bridge.rs in P06)
- [ ] No TODO/FIXME/HACK

---

### Phase P05: Queue Orchestration + Zoom/Camera

**Subagent:** rustcoder | **Prerequisites:** P04a PASS | **Est. LoC:** ~600

**Requirements:** Spec §8.2/§8.4/§8.5/§8.6/§8.7 (queue orchestration, cascading, zoom, camera, coordinate transforms).

**Functions:** `pre_process_queue()`, `post_process_queue()`, `redraw_queue()`, `calc_reduction()`, `calc_view()`, `init_display_list()`, `insert_prim()`, `calc_display_coord()`, `calc_zoom_stuff()`, `init_kernel()`

**TDD Cycle:**
- **RED:** Tests for PreProcessQueue iteration/camera, PostProcessQueue cascading/removal/transforms, zoom hysteresis (step 3-level, continuous smooth), camera midpoint/clamping.
- **GREEN:** Implement all queue orchestration in `process_loop.rs`.
- **REFACTOR:** Consolidate coordinate transform logic.

---

### Phase P05a: Queue Orchestration Verification

**Subagent:** rustreviewer

**Checklist:**
- [ ] PreProcessQueue: head-to-tail, PreProcess, collisions vs successors, camera tracking
- [ ] PostProcessQueue: newly-added cascading (tail-chasing), scroll offsets, DISAPPEARING removal, coordinate transforms, render insertion
- [ ] Scroll: PRE_PROCESS+not POST_PROCESS → apply; both → zero
- [ ] CalcReduction: step mode 3-level hysteresis, continuous smooth interpolation
- [ ] CalcView: midpoint camera, single-ship clamping
- [ ] RedrawQueue: PreProcessQueue → PostProcessQueue → UpdateSoundPositions → conditional render
- [ ] Simulation always executes; rendering conditionally skipped
- [ ] No TODO/FIXME/HACK

---

### Phase P06: C Bridge — Phase 2 FFI Wiring

**Subagent:** rustcoder | **Prerequisites:** P05a PASS | **Est. LoC:** ~500

**Requirements:** 44 deferred bridge operations (1 DrawablesIntersect from P04 moved to canonical location + 43 new). Process.c dark-code/test guards (public toggle deferred to P13).

**Deliverables:**
- `c_bridge.rs` — canonical Rust→C bridge layer with all 44 deferred bridge wrappers
- `ffi.rs` extension — add `rust_battle_redraw_queue` export
- `process.c` — dark-code/test guard plumbing
- Pre-P13 build/link map artifact

**TDD Cycle:**
- **RED:** FFI signature audits, round-trip safety tests, wrapper-behavior harnesses for highest-risk families.
- **GREEN:** Create c_bridge.rs, add process.c guards.
- **REFACTOR:** Consolidate FFI patterns.

**FFI safety:** See spec §10 for mandatory pointer-family safety categories, panic containment, callback-slot rules, and thread-affinity documentation requirements.

**Callback-slot migration matrix:** P06 verification must produce a callback-slot matrix (spec §8.1) covering preprocess/postprocess/collision/death + handler/vtable families.

---

### Phase P06a: C Bridge Verification

**Subagent:** rustreviewer

**Checklist:**
- [ ] All 44 deferred bridge operations covered across P04+P06
- [ ] FFI signatures match C declarations exactly
- [ ] Pointer-family safety rules enforced per spec §10.3
- [ ] Wrapper-family verification doc: callback/reentrancy/thread-affinity/nullability per family
- [ ] Callback-slot migration matrix produced (spec §8.1)
- [ ] Pre-P13 build/link map produced
- [ ] No Rust panic can cross any FFI boundary
- [ ] process.c original behavior preserved in pre-P13 config
- [ ] All Phase 1 FFI adapters unchanged in `ffi.rs`
- [ ] `cargo fmt/clippy/test` clean

---

### Phase P07: Ship Runtime Pipeline

**Subagent:** rustcoder | **Prerequisites:** P06a PASS | **Est. LoC:** ~700

**Requirements:** Spec Ship runtime §7-stage pipeline, energy regen, turn/thrust, inertial movement, weapon firing, ship collision.

**Functions:** `ship_preprocess()`, `ship_postprocess()`, `ship_collision()`, `inertial_thrust()`, `animation_preprocess()`

**TDD Cycle:**
- **RED:** Tests for 7-stage pipeline order, inertial_thrust physics (inertialess/normal/gravity/at-max-speed), energy regen, weapon firing sequence, ship collision damage (hp/4 min 1).
- **GREEN:** Implement in `ship_runtime.rs`.
- **REFACTOR:** Extract pipeline stages.

**Commit 1 (rename-only):** Rename `ship_runtime_types.rs` → `ship_runtime.rs` and update `mod.rs` imports. This commit contains NO logic changes — only the file rename and import path updates. This ensures `git log --follow` tracks the file history correctly.

**Commit 2+:** Add logic to `ship_runtime.rs` as described in the TDD cycle above.


**Critical dependency:** `animation_preprocess()` is also used by P09's `explosion_preprocess()` (tactrans.c:606 → ship.c:46).

---

### Phase P07a: Ship Runtime Verification

**Subagent:** rustreviewer

**Checklist:**
- [ ] Pipeline order: input → APPEARING → energy → preprocess → turn → thrust → status
- [ ] APPEARING first-frame: inputs suppressed, crew init, race preprocess, warp-in, early return
- [ ] Energy regen: counter countdown → DeltaEnergy
- [ ] Turn: NORMALIZE_FACING ±1, turn_wait
- [ ] Thrust: inertial_thrust → ion trail (if not cloaked) → thrust_wait
- [ ] inertial_thrust: MAX_ALLOWED_SPEED, inertialess check, gravity well, at-max-speed half-thrust
- [ ] Weapon: weapon_counter → energy cost → init_weapon_func (up to 6) → bind → sound → wait
- [ ] Ship collision: GRAVITY_MASS → damage = hp/4 (min 1)
- [ ] animation_preprocess: frame advance, CHANGING flag, turn_wait cooldown, public for P09
- [ ] No TODO/FIXME/HACK

---

### Phase P08: Ship Spawn + Init

**Subagent:** rustcoder | **Prerequisites:** P07a PASS | **Est. LoC:** ~400

**Functions:** `spawn_ship()`, `get_next_starship()`, `get_initial_starships()`

**TDD Cycle:**
- **RED:** Tests for element initialization (APPEARING|PLAYER_SHIP|IGNORE_SIMILAR, NORMAL_LIFE, ship_mass, zero velocity, callbacks), random placement avoiding gravity, Sa-Matra center placement, crew patching, element reuse, GetNextStarShip queue traversal and infinite fleet recycling.
- **GREEN:** Implement in `ship_runtime.rs`.
- **REFACTOR:** Consolidate initialization.

---

### Phase P08a: Ship Spawn Verification

**Subagent:** rustreviewer

**Checklist:**
- [ ] Flags: APPEARING | PLAYER_SHIP | IGNORE_SIMILAR; life=NORMAL_LIFE; mass=ship_mass; velocity=zero
- [ ] Callbacks: ship_preprocess/postprocess/ship_death/collision
- [ ] Crew patched from queue entry (cap at max_crew for encounters)
- [ ] Random position avoids gravity wells; Sa-Matra defending at center
- [ ] Element reuse: existing hShip → reinitialize; else alloc new
- [ ] Bidirectional binding: element.p_parent ↔ queue entry.hShip
- [ ] GetNextStarShip: encounter queue traversal, infinite fleet recycling
- [ ] No TODO/FIXME/HACK

---

### Phase P09: Death + Explosion

**Subagent:** rustcoder | **Prerequisites:** P08a PASS | **Est. LoC:** ~1000

**Functions (17):** `ship_death()`, `start_ship_explosion()`, `explosion_preprocess()`, `cleanup_dead_ship()`, `new_ship()`, `spawn_ion_trail()`, `cycle_ion_trail()`, `play_ditty()`, `stop_ditty()`, `ditty_playing()`, `stop_all_battle_music()`, `preprocess_dead_ship()`, `record_ship_death()`, `ready_for_battle_end()`, `set_min_ship_life_span()`, `set_min_starship_life_span()`, `check_other_ship_life_span()`

**TDD Cycle:**
- **RED:** Tests for ship_death→explosion→cleanup→new_ship callback chain, 36-frame explosion animation (debris count, frame 15 hide, frame 25 clear), cleanup_dead_ship (CREW_OBJECT preserved), new_ship readiness, ion trail colors, all helper functions.
- **GREEN:** Implement all 17 functions in `tactical.rs`. explosion_preprocess spawns debris using `animation_preprocess` from P07.
- **REFACTOR:** Extract debris spawning helper.

---

### Phase P09a: Death Pipeline Verification

**Subagent:** rustreviewer

**Checklist:**
- [ ] ship_death: stops music, clears ditty, starts explosion, finds winner, records death
- [ ] StartShipExplosion: zero velocity, drain energy, life=36, FINITE_LIFE+NONSOLID, replace callbacks
- [ ] explosion_preprocess: 36 frames, debris varies (1-3), frame 15 hide, frame 25 clear, uses animation_preprocess from P07
- [ ] cleanup_dead_ship: records crew, clears ownership, preserves CREW_OBJECT, plays ditty, sets death=new_ship
- [ ] new_ship: waits readiness, frees descriptor, persists crew, deactivates, requests replacement
- [ ] Callback chain: ship_death→explosion+cleanup; cleanup→new_ship+preprocess_dead_ship
- [ ] Winner kept alive one frame longer (checkOtherShipLifeSpan)
- [ ] Ion trail: 12-color, POINT_PRIM, head-insert, PRE_PROCESS set
- [ ] All helper functions verified
- [ ] Branch-parity cases exercised and recorded
- [ ] No TODO/FIXME/HACK

---

### Phase P10: Flee + Warp + Winner

**Subagent:** rustcoder | **Prerequisites:** P09a PASS | **Est. LoC:** ~600

**Functions (8):** `flee_preprocess()`, `do_run_away()`, `ship_transition()`, `find_alive_starship()`, `opponent_alive()`, `reset_winner_starship()`, `get_winner_starship()`, `set_winner_starship()`

**TDD Cycle:**
- **RED:** Tests for flee eligibility (5 conditions), 20-color pulse cycle, warp-out trigger, DoRunAway initiation, ship_transition 15-frame ghost images, find_alive_starship with Pkunk reincarnation (mass=11+crew=0→alive), OpponentAlive 3 return cases, winner state lifecycle.
- **GREEN:** Implement in `tactical.rs`.
- **REFACTOR:** Consolidate transition helpers.

---

### Phase P10a: Flee/Warp/Winner Verification

**Subagent:** rustreviewer

**Checklist:**
- [ ] Flee eligibility: all 5 conditions checked independently
- [ ] DoRunAway: mass=100, replace preprocess=flee, dark red, timing, suppress inputs
- [ ] Flee animation: 20-color cycle, accelerating timing, warp-out trigger (timing=0 AND cycle=midpoint)
- [ ] ship_transition: 15 frames, ghost images along facing vector, materialization restores callbacks
- [ ] find_alive_starship: display-list order, PLAYER_SHIP, Pkunk mass=11+crew=0 → alive
- [ ] OpponentAlive: 3 return cases
- [ ] Winner: zero crew + not reincarnating → null; recorded once; ditty set each death
- [ ] Branch-parity cases exercised and recorded
- [ ] No TODO/FIXME/HACK

---

### Phase P11: AI Dispatch

**Subagent:** rustcoder | **Prerequisites:** P10a PASS | **Est. LoC:** ~200

**Functions (1):** `computer_intelligence()`

**TDD Cycle:**
- **RED:** Tests for all 4 dispatch paths: Sa-Matra (returns 0), CYBORG (race intelligence + RPG escape merge), PSYTRON (sleep + BATTLE_WEAPON), non-cyborg (human input).
- **GREEN:** Implement in `ai.rs`.
- **REFACTOR:** Simplify control flow.

**Commit 1 (rename-only):** Rename `ai_types.rs` → `ai.rs` and update `mod.rs` imports. This commit contains NO logic changes — only the file rename and import path updates. This ensures `git log --follow` tracks the file history correctly.

**Commit 2+:** Add logic to `ai.rs` as described in the TDD cycle above.


---

### Phase P11a: AI Dispatch Verification

**Subagent:** rustreviewer

**Checklist:**
- [ ] IN_LAST_BATTLE: returns 0
- [ ] CYBORG_CONTROL: calls tactical_intelligence() + merges RPG BATTLE_ESCAPE
- [ ] PSYTRON_CONTROL: sleeps 0.5s, returns BATTLE_WEAPON
- [ ] Non-CYBORG: returns CurrentInputToBattleInput
- [ ] Uses Phase 1 AI constants
- [ ] No TODO/FIXME/HACK

---

### Phase P12: Battle Lifecycle

**Subagent:** rustcoder | **Prerequisites:** P11a PASS | **Est. LoC:** ~800

**Functions (13):** `battle()`, `init_ships()`, `uninit_ships()`, `init_space()`, `uninit_space()`, `process_input()`, `count_crew_elements()`, `run_away_allowed()`, `setup_battle_input_order()`, `battle_song()`, `free_battle_song()`, `select_all_ships()`, `get_player_order()`

**TDD Cycle:**
- **RED:** Tests for InitShips sequence (InitSpace refcounting, display list reset, environment spawning), UninitShips teardown (stop sounds, free assets, count crew, writeback), ProcessInput bit mapping + escape detection, RunAwayAllowed 3-condition check, all helpers.
- **GREEN:** Implement in `lifecycle.rs`. process_input() calls P10's do_run_away() for flee entry.
- **REFACTOR:** RAII guard pattern for asset management.

**Dark-code constraint:** Before P13, Rust lifecycle/input ownership is integration-harness-only. No supported runtime ownership flip.

**Mandatory branch-parity tests:** hyperspace/quasispace music selection, encounter vs final-battle init/teardown, SUPER_MELEE abort, CHECK_ABORT/CHECK_LOAD cleanup, max-speed skip.

---

### Phase P12a: Lifecycle Verification

**Subagent:** rustreviewer

**Checklist:**
- [ ] Battle(): seed RNG, BattleSong, InitShips, instant-victory check, spawn, music start, DoInput
- [ ] Battle() cleanup: SuperMelee abort, netplay buffer, StopDitty+StopMusic+StopSound, UninitShips, FreeBattleSong
- [ ] InitShips: InitSpace → contexts → InitDisplayList → hyperspace path (1 ship) or encounter path (asteroids + planet, NUM_SIDES)
- [ ] InitSpace/UninitSpace: reference-counted (space_ini_cnt)
- [ ] UninitShips: StopSound → UninitSpace → CountCrew → iterate → survivor → add crew (cap) → record → free_ship → clear IN_BATTLE
- [ ] ProcessInput: bit mapping, escape → do_run_away() (no duplication of flee logic)
- [ ] Branch parity verified for all mandatory families
- [ ] Dark-code: no supported runtime ownership flip before P13
- [ ] InitShips returns i16 (negative for hyperspace exit)
- [ ] No TODO/FIXME/HACK

---

### Phase P13: FFI Layer — Phase 3 Exports + C Bridge Wiring

**Subagent:** rustcoder | **Prerequisites:** P12a PASS | **Est. LoC:** ~500

**Deliverables:**
- Phase 3 FFI exports: `rust_battle_frame`, `rust_battle_entry`, `rust_battle_init_ships`, `rust_battle_uninit_ships`, `rust_battle_compute_checksum`, `rust_battle_computer_intelligence`, `rust_battle_song`, `rust_battle_free_song`, `rust_battle_get_player_order`
- DoBattle thin shell rewrite under `#ifdef USE_RUST_BATTLE_LOOP`
- `rust_battle_wrappers.c` — C wrapper file preserving external symbol ABI (spec §5.4)
- C guards on battle.c, tactrans.c, intel.c, ship.c, init.c
- `USE_RUST_BATTLE_LOOP` toggle in build.config + config_unix.h
- Symbol-provider table/link-map artifact matching spec §5.2

**TDD Cycle:**
- **RED:** Tests for rust_battle_frame FFI entry, init/uninit round-trip, CRC verification, netplay frame-sync.
- **GREEN:** Implement exports, wire shell, verify guards.
- **REFACTOR:** Consolidate FFI error handling.

**Must include:** Cross-phase P12↔P13 integration tests covering battle setup, frame-sync, mismatch abort, battle-end readiness.

---

### Phase P13a: FFI Layer Verification

**Subagent:** rustreviewer

**Checklist:**
- [ ] DoBattle→rust_battle_frame call path verified
- [ ] All C guards compile correctly
- [ ] Symbol-provider table matches spec §5.2
- [ ] Wrapper→Rust mapping matches spec §5.4
- [ ] C-only baseline build unaffected (USE_RUST_BATTLE_LOOP disabled)
- [ ] Netplay CRC integration verified
- [ ] No TODO/FIXME/HACK

---

### Phase P14: E2E Integration + Regression

**Subagent:** rustcoder | **Prerequisites:** P13a PASS | **Est. LoC:** ~200

**TDD Cycle:**
- **RED:** Full battle frame cycle (init→preprocess→collision→postprocess→render), multi-frame sequences.
- **GREEN:** Wire all modules, verify 2,151+ Phase 1 tests still pass.
- **REFACTOR:** Final cleanup.

---

### Phase P14a: E2E Verification (Final Gate)

**Subagent:** rustreviewer

**Checklist:**
- [ ] All Rust tests pass: `cargo test --workspace --all-features`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean
- [ ] `cargo fmt --all --check` clean
- [ ] C-only baseline build works
- [ ] Rust-enabled build works
- [ ] Phase 1 tests (2,151) all still pass
- [ ] No TODO/FIXME/HACK in implementation code
- [ ] All 64 ported functions verified
- [ ] All 11 permanent C boundary functions verified in place

## Execution Tracker

| Phase | Tests | Clippy | Fmt | Est. LoC | Ported Fns | Key Functions |
|-------|-------|--------|-----|----------|-----------|---------------|
| P00.5 | ⬜ | ⬜ | ⬜ | 0 | 0 | preflight |
| P01 | ⬜ | ⬜ | ⬜ | 0 | 0 | analysis |
| P01a | ⬜ | ⬜ | ⬜ | 0 | 0 | verify analysis |
| P02 | ⬜ | ⬜ | ⬜ | 0 | 0 | pseudocode |
| P02a | ⬜ | ⬜ | ⬜ | 0 | 0 | verify pseudocode |
| P03 | ⬜ | ⬜ | ⬜ | ~800 | 7 | PreProcess, PostProcess, AllocElement, FreeElement, SetUpElement, Untarget, RemoveElement |
| P03a | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P04 | ⬜ | ⬜ | ⬜ | ~900 | 1 | ProcessCollisions |
| P04a | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P05 | ⬜ | ⬜ | ⬜ | ~600 | 9 | PreProcessQueue, PostProcessQueue, RedrawQueue, CalcReduction, CalcView, InitDisplayList, InsertPrim, CalcDisplayCoord, InitKernel |
| P05a | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P06 | ⬜ | ⬜ | ⬜ | ~500 | 0 | c_bridge.rs (44 bridge wrappers), process.c guards |
| P06a | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P07 | ⬜ | ⬜ | ⬜ | ~700 | 5 | animation_preprocess, inertial_thrust, ship_preprocess, ship_postprocess, collision(ship) |
| P07a | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P08 | ⬜ | ⬜ | ⬜ | ~400 | 3 | spawn_ship, GetNextStarShip, GetInitialStarShips |
| P08a | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P09 | ⬜ | ⬜ | ⬜ | ~1000 | 17 | ship_death, StartShipExplosion, explosion_preprocess, cleanup_dead_ship, new_ship, + 12 helpers |
| P09a | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P10 | ⬜ | ⬜ | ⬜ | ~600 | 8 | flee_preprocess, ship_transition, DoRunAway, FindAliveStarShip, OpponentAlive, + 3 winner helpers |
| P10a | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P11 | ⬜ | ⬜ | ⬜ | ~200 | 1 | computer_intelligence |
| P11a | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P12 | ⬜ | ⬜ | ⬜ | ~800 | 13 | Battle, InitShips, UninitShips, InitSpace, UninitSpace, ProcessInput, CountCrewElements, + 6 helpers |
| P12a | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P13 | ⬜ | ⬜ | ⬜ | ~500 | 0 | FFI exports + C shell wiring + wrappers.c |
| P13a | ⬜ | ⬜ | ⬜ | 0 | 0 | |
| P14 | ⬜ | ⬜ | ⬜ | ~200 | 0 | Wire all modules, regression test |
| P14a | ⬜ | ⬜ | ⬜ | 0 | 0 | All gates must pass |
| | | | | | | |
| | **Totals** | | | **~7,200** | **64 ported** | **+ 11 permanent C boundary = 75 total** |

## Execution Rules

1. Phases execute in strict order: P00.5 → P01 → P01a → ... → P14 → P14a
2. Each phase MUST be completed and verified before the next begins
3. No skipping phases
4. No multi-phase batching
5. Phase completion requires creating `project-plans/20260311/battlept2/.completed/PNN.md`
6. Failed verification triggers remediation loop (fix → re-verify)

## Definition of Done

1. All `cargo test --workspace --all-features` pass
2. All `cargo clippy --workspace --all-targets --all-features -- -D warnings` pass
3. `cargo fmt --all --check` passes
4. C-only baseline build (`USE_RUST_BATTLE_LOOP` disabled) works identically to pre-port
5. Rust-enabled build (`USE_RUST_BATTLE_LOOP` enabled) passes all tests
6. All 64 ported C functions have equivalent Rust implementations
7. All 11 permanent C boundary functions are verified in place
8. Symbol-provider table matches spec §5.2
9. DoBattle thin shell matches spec §4
10. All Phase 1 tests (2,151) continue to pass
11. No TODO/FIXME/HACK in implementation code
12. Branch-parity verified for all families (spec §13)
13. Netplay CRC bit-identical to C reference
14. Callback-slot migration matrix produced and verified (spec §8)

## Plan Files

```text
plan/
  00-overview.md                                (this file)
  00a-preflight-verification.md                 P00.5
  01-analysis.md                                P01
  01a-analysis-verification.md                  P01a
  02-pseudocode.md                              P02
  02a-pseudocode-verification.md                P02a
  03-process-prepost.md                         P03
  03a-process-prepost-verification.md           P03a
  04-process-collisions.md                      P04
  04a-process-collisions-verification.md        P04a
  05-queue-zoom-camera.md                       P05
  05a-queue-zoom-camera-verification.md         P05a
  06-c-bridge-ffi.md                            P06
  06a-c-bridge-ffi-verification.md              P06a
  07-ship-runtime.md                            P07
  07a-ship-runtime-verification.md              P07a
  08-ship-spawn-init.md                         P08
  08a-ship-spawn-init-verification.md           P08a
  09-death-explosion.md                         P09
  09a-death-explosion-verification.md           P09a
  10-flee-warp-winner.md                        P10
  10a-flee-warp-winner-verification.md          P10a
  11-ai-dispatch.md                             P11
  11a-ai-dispatch-verification.md               P11a
  12-battle-lifecycle.md                        P12
  12a-battle-lifecycle-verification.md          P12a
  13-ffi-layer-phase3.md                        P13
  13a-ffi-layer-phase3-verification.md          P13a
  14-e2e-integration.md                         P14
  14a-e2e-verification.md                       P14a
  execution-tracker.md
```

## Deferred Items

- **DoInput framework port** — `DoInput()` is engine-wide and not battle-specific. All DoInput callbacks stay in C.
- **Netplay transport layer** — The network protocol is out of scope. Battle provides integration hooks only.
- **Graphics/audio subsystem internals** — Called via bridge, not owned.
- **Ships subsystem internals** — Per-race behavior stays in ships subsystem.
- **Display primitive array ownership** — Stays C-owned. May move to Rust in a future graphics subsystem port.
.
bsystem port.
