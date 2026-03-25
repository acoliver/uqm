# Phase 07: Ship Runtime Pipeline

## Phase ID
`PLAN-20260320-BATTLEPT2.P07`

## Prerequisites
- Required: Phase 06a (C Bridge Verification) completed with PASS
- Expected files: `c_bridge.rs` with all 44 bridge wrappers, `ffi.rs` with redraw_queue export
- Expected artifacts: Full bridge layer verified, callback-slot matrix produced

## Requirements Implemented (Expanded)

### REQ: Ship per-frame pipeline (battle/requirements.md §Ship per-frame pipeline)
**Requirement text**: For each active ship per frame, execute: read input → handle APPEARING → energy regen → race preprocess → turn → thrust → status update. This exact order is mandatory.

Behavior contract:
- GIVEN: An active ship element with PLAYER_SHIP flag
- WHEN: ship_preprocess is called as the element's preprocess_func
- THEN: The 7-stage pipeline executes in exact order

### REQ: APPEARING first-frame (battle/requirements.md §Ship per-frame pipeline, APPEARING)
**Requirement text**: On first frame, suppress all control inputs, init crew, init status, invoke race preprocess. For encounter/last-battle path, either initiate warp-in via ship_transition OR handle Pkunk reincarnation (hTarget != 0). Return early.

Behavior contract:
- GIVEN: A ship element with APPEARING flag set in actual state_flags
- WHEN: ship_preprocess executes
- THEN: Inputs suppressed; crew initialized from descriptor. Then THREE mutually exclusive paths checked in this exact order (ship.c:180-230):
  1. **Sa-Matra** (checked FIRST): `playerNr == NPC_PLAYER_NUM && LOBYTE(CurrentActivity) == IN_LAST_BATTLE`. Draw captain background, destroy drawable. Falls through to normal processing (does NOT early return — Sa-Matra doesn't call ship_transition or InitShipStatus).
  2. **Generic encounter** (checked SECOND, as else-if): `LOBYTE(CurrentActivity) <= IN_ENCOUNTER`. Note: IN_LAST_BATTLE is NOT <= IN_ENCOUNTER, so this path does NOT include the Sa-Matra battle. InitShipStatus, DrawCaptainsWindow, invoke race preprocess, then branch on hTarget (ship.c:208-218):
    - hTarget == 0 (normal spawn): call ship_transition() for warp-in animation
    - hTarget != 0 (Pkunk reincarnation): clear hTarget to 0, check if music is not playing AND opponent is alive → if so, restart BattleSong(TRUE)
    - **Early return** in this sub-path (skip energy/turn/thrust/status)
  3. **HyperSpace** (else: activity > IN_ENCOUNTER, which includes IN_LAST_BATTLE for non-NPC): set position to center, InitIntersectStartPoint/EndPoint, call hyper_transition. **Early return** if hyper_transition succeeds.

### REQ: Inertial movement model (battle/requirements.md §Inertial movement model)
**Requirement text**: Inertialess = instant max. Normal = compare v² vs max²; accelerate if below. Gravity well = allow up to GRAVITY_MAX. At max speed + facing change = half-thrust new minus full old.

Behavior contract:
- GIVEN: A ship with thrust_increment == max_thrust (inertialess)
- WHEN: inertial_thrust is called
- THEN: Velocity set instantly to max_thrust along facing direction

- GIVEN: A ship within a gravity well (distance < GRAVITY_THRESHOLD)
- WHEN: inertial_thrust is called
- THEN: Speed allowed up to MAX_ALLOWED_SPEED (2304) even if > ship's max

- GIVEN: A ship at max speed that changes facing
- WHEN: inertial_thrust is called
- THEN: Apply half thrust in new direction minus full thrust in old direction

### REQ: Weapon firing from ships (battle/requirements.md §Weapon firing from ships)
**Requirement text**: Postprocess: exit if crew==0; weapon firing (counter→energy→init_weapon→bind→sound→wait); special counter decrement; race postprocess; status update.

Behavior contract:
- GIVEN: A ship with weapon input active, weapon_counter elapsed, sufficient energy
- WHEN: ship_postprocess executes
- THEN: Energy deducted, init_weapon_func called (up to 6 elements), elements bound to parent, weapon sound played, weapon_wait applied

### REQ: Ship collision (battle/requirements.md §Ship collision)
**Requirement text**: collision() is the ship's collision_func. It conditionally sets COLLISION and handles gravity-mass damage depending on the other element's flags. Elastic response (velocity changes) is NOT handled here — it is done externally by ProcessCollisions via collide()/elastic_collide() for non-FINITE_LIFE pairs.

Behavior contract:
- GIVEN: A ship collides with another element
- WHEN: collision() handler executes (ship.c:367-391)
- THEN: The entire body is gated on `!(ElementPtr1->state_flags & FINITE_LIFE)`. Only when the OTHER element (ElementPtr1) does NOT have FINITE_LIFE:
  1. COLLISION flag is set on the ship element (ElementPtr0->state_flags |= COLLISION)
  2. If the other element is a gravity-mass object (GRAVITY_MASS(mass_points)): ship takes damage = max(hit_points/4, 1) via do_damage, and a damage sound is played via ProcessSound
  3. If the other element is NOT gravity-mass: only COLLISION is set, no damage
  When the OTHER element HAS FINITE_LIFE (e.g., a projectile): collision() is a no-op (no COLLISION flag, no damage). Projectile-vs-ship damage is handled by the projectile's own collision_func.

- GIVEN: Two non-FINITE_LIFE elements collide (ship-ship or ship-planet)
- WHEN: ProcessCollisions detects the collision (process.c:598-607)
- THEN: AFTER both collision_funcs are called, ProcessCollisions externally calls collide() (elastic_collide) to compute velocity response, then rechecks both elements against the full list

## Implementation Tasks

### Commit 1 (rename-only)
- Rename `rust/src/battle/ship_runtime_types.rs` → `rust/src/battle/ship_runtime.rs`
- Update `rust/src/battle/mod.rs` to reference `ship_runtime` instead of `ship_runtime_types`
- marker: `@plan PLAN-20260320-BATTLEPT2.P07`
- **NO logic changes** — only file rename and import path updates

### Commit 2+: Files to modify

- `rust/src/battle/ship_runtime.rs` — Add ship pipeline logic
  - marker: `@plan PLAN-20260320-BATTLEPT2.P07`
  - marker: `@requirement REQ-SHIP-PIPELINE, REQ-INERTIAL-MOVEMENT, REQ-WEAPON-FIRING, REQ-SHIP-COLLISION`
  - Contents to add:
    - `pub fn ship_preprocess(element: &mut Element)` — 7-stage pipeline matching ship.c:156-290. Stages: (1) read input status from cur_status_flags (suppress inputs if APPEARING), (2) APPEARING first-frame handling: init crew from descriptor; then THREE mutually exclusive paths in exact order: (a) Sa-Matra check FIRST: `playerNr == NPC_PLAYER_NUM && activity == IN_LAST_BATTLE` — draw captain bg, destroy drawable, falls through to normal processing (no early return); (b) Generic encounter SECOND (else-if): `activity <= IN_ENCOUNTER` (excludes IN_LAST_BATTLE) — InitShipStatus, DrawCaptainsWindow, race preprocess, then hTarget==0 → ship_transition warp-in, OR hTarget!=0 → Pkunk reincarnation: clear hTarget, conditionally restart BattleSong; EARLY RETURN; (c) HyperSpace (else): center position, InitIntersectStartPoint/EndPoint, hyper_transition; early return if transition succeeds. (3) energy regen (energy_counter countdown → DeltaEnergy), (4) race-specific preprocess callback, (5) turning (NORMALIZE_FACING ±1, turn_wait), (6) thrust (inertial_thrust → ion trail if not cloaked → thrust_wait), (7) status display update (if activity <= IN_ENCOUNTER).
    - `pub fn ship_postprocess(element: &mut Element)` — Weapon firing + race post matching ship.c:375-470. Exit if crew==0; weapon firing sequence; special_counter decrement; race postprocess; status update.
    - `pub fn inertial_thrust(element: &mut Element, starship: &Starship) -> StatusFlags` — Movement physics matching ship.c:64-118. Returns status flags (AT_MAX_SPEED, BEYOND_MAX_SPEED, IN_GRAVITY_WELL). Handles: inertialess (thrust==max → instant), normal (v²<max² → accelerate), gravity well (allow up to 2304), at-max-speed (half new − full old).
    - `pub fn animation_preprocess(element: &mut Element)` — Frame advance matching ship.c:46-62. turn_wait decrement, frame advance, CHANGING flag set. **Public**: also used by P09 explosion_preprocess.
    - `pub fn ship_collision(element: &mut Element, other: &Element)` — Ship collision handler matching ship.c:367-391 collision(). Entire body is gated on `!(other.state_flags & FINITE_LIFE)`: when other is non-FINITE_LIFE, sets COLLISION flag on element; if other is GRAVITY_MASS, additionally applies damage = max(hit_points/4, 1) via do_damage and plays damage sound. When other HAS FINITE_LIFE, collision() is a no-op (projectile damage comes from the projectile's own collision_func). Note: elastic velocity response is NOT in this function — it is handled externally by ProcessCollisions calling collide()/elastic_collide().

### C reference functions ported

| C Function | C File | C Lines | Rust Function | Rust Module |
|-----------|--------|---------|---------------|-------------|
| `animation_preprocess()` | ship.c | :46-62 | `animation_preprocess()` | `ship_runtime.rs` |
| `inertial_thrust()` | ship.c | :64-118 | `inertial_thrust()` | `ship_runtime.rs` |
| `ship_preprocess()` | ship.c | :120-370 | `ship_preprocess()` | `ship_runtime.rs` |
| `ship_postprocess()` | ship.c | :375-470 | `ship_postprocess()` | `ship_runtime.rs` |
| `collision()` (ship) | ship.c | :475-520 | `ship_collision()` | `ship_runtime.rs` |

### C branches to handle

| Branch | Source Sites | Handling |
|--------|-------------|----------|
| `IN_ENCOUNTER` / `IN_LAST_BATTLE` | ship.c ship_preprocess APPEARING | Encounter-specific init vs SuperMelee init |
| Cloaking check | ship.c ship_preprocess thrust stage | Skip ion trail if cloaked |
| `USE_RUST_SHIPS` | ship.c:38-43,158-160,295-297,396-397 | ship_preprocess, ship_postprocess, collision, spawn_ship all wrapped with USE_RUST_SHIPS guards delegating to rust_ships_* extern functions |

### Integration points
- P06 `c_bridge.rs`: get_element_starship, set_element_starship, process_sound, play_sound_effect
- Phase 1 `element.rs`: Element struct, ElementFlags
- Phase 1 `velocity.rs`: set_vector(), get_current_components(), delta_components()
- Phase 1 `battle_types.rs`: SINE/COSINE macros, NORMALIZE_FACING, coordinate conversions
- Phase 1 `ship_runtime.rs` (types): ShipPipelineStage, spawn constants
- P09 uses `animation_preprocess()` — must be `pub`

### Pseudocode traceability (if impl phase)
- Uses pseudocode lines from `analysis/pseudocode/ship-runtime.md`: ship_preprocess, ship_postprocess, inertial_thrust, animation_preprocess, collision sections

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `ship_runtime_types.rs` renamed to `ship_runtime.rs` (commit 1 rename-only)
- [ ] `mod.rs` updated to reference `ship_runtime`
- [ ] All 5 functions implemented in `ship_runtime.rs`
- [ ] `animation_preprocess` is `pub` (used by P09)
- [ ] Plan/requirement traceability markers present
- [ ] Tests compile and run

## Semantic Verification Checklist (Mandatory)
- [ ] Pipeline 7-stage order verified: input → APPEARING → energy → race preprocess → turn → thrust → status
- [ ] APPEARING first-frame: inputs suppressed, crew init. Three mutually exclusive paths checked in order: (a) Sa-Matra first (NPC + IN_LAST_BATTLE) — draw captain bg, NO early return; (b) Generic encounter second (activity <= IN_ENCOUNTER, excludes IN_LAST_BATTLE) — InitShipStatus, DrawCaptainsWindow, race preprocess, hTarget==0 → ship_transition, hTarget!=0 → Pkunk reincarnation, EARLY RETURN; (c) HyperSpace (else) — center + hyper_transition, early return if succeeds
- [ ] Energy regen: counter countdown → DeltaEnergy when elapsed
- [ ] Turn: NORMALIZE_FACING ±1, turn_wait from characteristics
- [ ] Thrust: inertial_thrust → ion trail (if not cloaked) → thrust_wait
- [ ] inertial_thrust: inertialess (instant max), normal (v²<max²), gravity (allow 2304), at-max-speed (half new − full old)
- [ ] inertial_thrust returns correct status flags
- [ ] Weapon firing: counter → energy check → init_weapon_func (up to 6) → bind to parent → sound → wait
- [ ] Ship collision: entire body gated on other not having FINITE_LIFE; when non-FINITE_LIFE: sets COLLISION flag; gravity mass → damage = max(hit_points/4, 1) + sound; when FINITE_LIFE: no-op. Elastic response is external (ProcessCollisions calls collide())
- [ ] animation_preprocess: turn_wait decrement, frame advance, CHANGING flag
- [ ] No placeholder/deferred implementation patterns

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/ship_runtime.rs
```

## Success Criteria
- [ ] All 5 functions implemented and tested
- [ ] Pipeline order matches C exactly
- [ ] Inertial physics produces same results as C
- [ ] All Phase 1 tests pass
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git checkout -- rust/src/battle/ship_runtime.rs rust/src/battle/mod.rs`
- blocking issues: Race-specific callback integration, energy/velocity calculation precision

## Phase Completion Marker
Create: `project-plans/20260311/battlept2/.completed/P07.md`

Contents:
- phase ID: PLAN-20260320-BATTLEPT2.P07
- timestamp
- files changed: ship_runtime.rs (renamed + logic), mod.rs
- tests added/updated
- verification outputs
- semantic verification summary
