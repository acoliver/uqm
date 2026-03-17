# Phase 08: Shared Runtime Pipeline

## Phase ID
`PLAN-20260314-SHIPS.P08`

## Prerequisites
- Required: Phase 07a (Queue Verification) completed and PASS
- Expected files: `types.rs`, `traits.rs`, `queue.rs`, `ffi_contract.rs` with runtime callback and element semantics documented

## Requirements Implemented (Expanded)

### Pipeline Ordering
**Requirement text**: For each active ship element per battle frame, the shared runtime shall execute a pipeline of: input/status normalization, first-frame setup, race preprocess dispatch, energy regeneration, turn and thrust, status coordination, weapon fire, special activation, race postprocess dispatch, and cooldown updates. The relative ordering shall be preserved.

Behavior contract:
- GIVEN: An active ship element
- WHEN: A battle frame is processed
- THEN: All pipeline steps execute in the specified order

### Movement Model
**Requirement text**: The movement model shall be inertial: thrust applies acceleration in the ship's facing direction, ships coast at current velocity when not thrusting, turn rate is governed by the ship's characteristics, and maximum speed is enforced. The movement model shall be deterministic.

Behavior contract:
- GIVEN: A ship with known thrust/turn characteristics and input
- WHEN: Movement is processed
- THEN: Position/velocity change is deterministic and inertial

### Energy Model
**Requirement text**: Energy shall regenerate at a rate and interval defined by the ship's characteristics, weapon and special use shall deduct energy, and energy shall not exceed maximum.

Behavior contract:
- GIVEN: A ship with energy_wait=6, energy_regeneration=1
- WHEN: 6 frames pass
- THEN: Energy increases by 1 (if below max)

### Weapon Fire
**Requirement text**: When the weapon input is active, energy is sufficient, and weapon cooldown has elapsed, the subsystem shall invoke the weapon initialization hook and deduct energy.

Behavior contract:
- GIVEN: WEAPON status flag set, sufficient energy, cooldown at 0
- WHEN: Weapon fire step executes
- THEN: `init_weapon()` is called, energy deducted, cooldown set

### Collision
**Requirement text**: Ships shall exhibit correct collision behavior. When a race overrides collision behavior, the subsystem shall dispatch through the override.

Behavior contract:
- GIVEN: A ship with collision override registered
- WHEN: Collision occurs
- THEN: Race-specific collision handler is called

## Implementation Tasks

### Files to create

- `rust/src/ships/runtime.rs` — Shared ship runtime pipeline
  - marker: `@plan PLAN-20260314-SHIPS.P08`
  - marker: `@requirement REQ-PIPELINE-ORDER, REQ-MOVEMENT-INERTIAL, REQ-MOVEMENT-DETERMINISTIC, REQ-ENERGY-REGEN, REQ-WEAPON-FIRE, REQ-SPECIAL-ACTIVATION, REQ-AI-HOOK, REQ-HOOK-SERIALIZED, REQ-COLLISION-CORRECT, REQ-COLLISION-OVERRIDE`
  - Contents:
    - `ship_preprocess(ship: &mut Starship, element: &mut ElementState) -> Result<(), ShipError>`:
      - Input/status normalization from element to ship
      - First-frame setup (APPEARING flag check)
      - AI-input path for computer-controlled ships: invoke `intelligence()` at the correct point relative to status normalization and before shared movement/weapon processing
      - Race preprocess dispatch: `ship.race_desc.behavior.preprocess()`
      - Energy regeneration: counter-based with energy_wait interval
      - Turn handling: LEFT/RIGHT with turn_wait counter
      - Thrust handling: THRUST with thrust_wait counter
      - Gravity-well / planet influence integration using battle-engine-provided state
      - Status coordination (LOW_ON_ENERGY, SHIP_AT_MAX_SPEED flags, status display synchronization)
    - `ship_postprocess(ship: &mut Starship, element: &mut ElementState) -> Result<(), ShipError>`:
      - Weapon fire: check WEAPON flag, energy, cooldown → call `init_weapon()`
      - Sound trigger for weapon
      - Special activation: check SPECIAL flag, energy, cooldown
      - Race postprocess dispatch: `ship.race_desc.behavior.postprocess()`
      - Cooldown decrements
    - `inertial_thrust(ship: &mut Starship, element: &mut ElementState, characteristics: &Characteristics)`:
      - Apply thrust_increment in facing direction
      - Cap velocity at max_thrust
      - Thrust counter management
    - `animation_preprocess(element: &mut ElementState)`:
      - Standard frame animation cycling
    - `default_ship_collision(ship_element: &ElementState, other: &ElementState) -> CollisionResult`:
      - Ship-vs-planet: gravity well / landing behavior
      - Ship-vs-crew: crew pickup
      - Ship-vs-projectile: damage application and ownership attribution
      - Ship-vs-ship / element-category interactions routed according to battle-engine categories/flags, not just a generic callback swap
    - `ElementState` struct — Rust-side view of element state for pipeline (position, velocity, state_flags, category/owner info, image, life_span, etc.)
    - `CollisionResult` enum — outcome of collision (damage, pickup, bounce, etc.)
    - Constants: `NORMAL_LIFE`, `NUM_FACINGS`, `NORMALIZE_FACING()`

### Required battle-engine coupling analysis captured in this phase
- Exact gravity-well / planet influence data path and how it reaches runtime processing
- AI-control timing relative to input normalization and race preprocess/postprocess hooks
- Element category/flag mapping used for collision compatibility
- Projectile ownership / damage attribution path
- Crew pickup/writeback-relevant runtime semantics

### Files to modify

- `rust/src/ships/mod.rs`
  - Add `pub mod runtime;`

### Pseudocode traceability
- Uses pseudocode component 5, lines 170-265
- Uses pseudocode component 6, lines 270-303 (ElementState)
- Augments pseudocode with gravity-well, AI timing, and element-category coupling that the specification requires

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `runtime.rs` created with preprocess, postprocess, thrust, collision functions
- [ ] Pipeline step ordering matches specification
- [ ] ElementState captures all needed element and category fields
- [ ] AI hook invocation path is explicit for computer-controlled ships and placed at a single documented stage of preprocess
- [ ] Hook invocation on a descriptor instance occurs through serialized `&mut self` access with no concurrent same-instance dispatch path in runtime design
- [ ] Gravity-well / AI / projectile-ownership hooks are represented in the runtime design
- [ ] Plan/requirement traceability markers present

## Semantic Verification Checklist (Mandatory)
- [ ] Pipeline ordering test: preprocess steps execute in documented order
- [ ] Energy regeneration test: counter cycles correctly, energy increases, caps at max
- [ ] Turn test: LEFT/RIGHT decrease/increase facing with turn_wait delay
- [ ] Thrust test: THRUST applies acceleration, velocity capped at max_thrust
- [ ] Inertia test: ship coasts at current velocity when not thrusting
- [ ] Gravity-well test: ship velocity is influenced correctly when in a gravity well
- [ ] AI-hook invocation test: computer-controlled ships invoke `intelligence()` through the descriptor instance behavior object
- [ ] AI-input timing test: computer-controlled status flags are computed at the intended stage of the pipeline relative to normalization and before shared movement/weapon processing
- [ ] AI hook is not invoked for non-computer-controlled ships unless explicitly required by the runtime contract
- [ ] Hook serialization test: repeated preprocess/runtime dispatch never concurrently invokes hooks on the same descriptor instance
- [ ] Weapon fire test: fires when WEAPON + energy + cooldown conditions met
- [ ] Weapon fire test: does NOT fire when energy insufficient
- [ ] Cooldown test: counters decrement each frame
- [ ] First-frame test: APPEARING flag triggers setup, then clears
- [ ] Movement determinism: same inputs produce same outputs
- [ ] Collision override: race override is used when present
- [ ] Default collision: planet/crew/projectile/category cases handled
- [ ] Mixed C/Rust smoke test validates callback trampoline registration and one minimal runtime step through real C-owned element state
- [ ] No placeholder/deferred implementation patterns remain

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/ships/runtime.rs
```

## Success Criteria
- [ ] Full pipeline compiles and passes tests
- [ ] All movement/energy/firing mechanics verified
- [ ] Battle-engine coupling points are explicit and tested early
- [ ] Determinism verified
- [ ] Collision dispatch verified
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git checkout -- rust/src/ships/runtime.rs`

## Phase Completion Marker
Create: `project-plans/20260311/ships/.completed/P08.md`
