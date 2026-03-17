# Phase 09: Ship Spawn & Lifecycle

## Phase ID
`PLAN-20260314-SHIPS.P09`

## Prerequisites
- Required: Phase 08a (Runtime Pipeline Verification) completed and PASS
- Expected files: `loader.rs`, `runtime.rs`, `queue.rs`, `ffi_contract.rs`, all bridge-safe types

## Requirements Implemented (Expanded)

### Spawn Sequence
**Requirement text**: When an external system hands a chosen queue entry to the subsystem for combat, the subsystem shall: load descriptor at battle-ready tier, bind to queue entry, patch crew, allocate element, configure element, bind callbacks, register race hooks, mark active.

Behavior contract:
- GIVEN: A canonical queue entry with species_id and crew_level set
- WHEN: `spawn_ship()` is called
- THEN: Descriptor is loaded, element is created, callbacks bound, ship is active

### Spawn Idempotency
**Requirement text**: Spawn shall be idempotent per queue entry within a single battle.

Behavior contract:
- GIVEN: A queue entry that has already been spawned and destroyed
- WHEN: Spawn is attempted again on the same entry
- THEN: It is not respawned (entry is marked destroyed)

### Battle Initialization
**Requirement text**: When battle begins, the subsystem shall initialize ship-runtime resources and state required for combat.

Behavior contract:
- GIVEN: Battle is starting
- WHEN: `init_ships()` is called
- THEN: Ship-runtime assets loaded and battle state prepared, while broader battle-environment orchestration stays with the C battle engine

### Battle Teardown
**Requirement text**: When battle ends, the subsystem shall stop audio, free assets, enumerate remaining ships, free descriptors (invoking teardown hooks), write back crew, clear state.

Behavior contract:
- GIVEN: Battle is ending
- WHEN: `uninit_ships()` is called
- THEN: All ship resources freed, crew written back, state cleared

### Spawn Failure
**Requirement text**: When a ship cannot be spawned, the side is treated as having no replacement. Failure shall not corrupt other active ship state.

Behavior contract:
- GIVEN: A spawn attempt that fails (load error, element allocation failure)
- WHEN: Spawn returns Err
- THEN: No partial state left, other ships unaffected

## Implementation Tasks

### Files to create

- `rust/src/ships/lifecycle.rs` — Battle lifecycle: init/uninit/spawn
  - marker: `@plan PLAN-20260314-SHIPS.P09`
  - marker: `@requirement REQ-SPAWN-SEQUENCE, REQ-SPAWN-IDEMPOTENT, REQ-BATTLE-INIT, REQ-BATTLE-TEARDOWN, REQ-SPAWN-FAILURE, REQ-FAILURE-ISOLATION`
  - Contents:
    - `spawn_ship(starship: *mut STARSHIP) -> Result<(), ShipError>` or exact typed equivalent from P03.5:
      - Check if already spawned/destroyed → return early
      - `loader::load_ship(species_id, BattleReady)?`
      - Patch descriptor crew_level from canonical queue entry
      - Allocate element via `c_bridge::alloc_element()`
      - Configure element state (position, facing, frames, mass, life_span)
      - Bind shared callbacks (preprocess, postprocess, death, collision) on element
      - Register collision override if race provides one
      - Set `hShip` and bind starship↔element association using the established C battle-engine path
      - Attach Rust-owned descriptor handle to the C-owned queue entry according to the boundary contract
      - Mark active
    - `init_ships() -> Result<u32, ShipError>`:
      - Initialize only ship-runtime resources and state that belong to the ships subsystem boundary
      - Load shared ship-runtime assets such as explosion/blast assets used by ship death/runtime behavior
      - Participate in mode-specific setup where ship-runtime behavior requires it (for example, hyperspace flagship handling)
      - Do **not** absorb broader C-owned battle-environment orchestration such as display-list ownership, galaxy/background rendering policy, or planet/asteroid placement policy beyond the battle-engine integration hooks already defined by the spec
      - Return NUM_SIDES
    - `uninit_ships() -> Result<(), ShipError>`:
      - Stop all ship-related audio
      - Uninit only ship-runtime assets/resources owned by the ships boundary
      - Count floating crew elements
      - For each side's canonical race queue: free active descriptors, write back crew
      - Clear IN_BATTLE state through the existing owner boundary
      - Reinitialize/reset queue-facing state only where the ships subsystem is the owner or explicit participant
    - `init_space_runtime_assets() -> Result<(), ShipError>`:
      - Load only the subset of `init.c` space/explosion/blast assets that are ship-runtime dependencies
      - Document which `init.c` responsibilities remain C battle/environment orchestration and are therefore out of scope for migration into Rust ships
    - `uninit_space_runtime_assets()`:
      - Free only the shared ship-runtime assets loaded by `init_space_runtime_assets()`
    - `get_next_starship(side: u8) -> Option<*mut STARSHIP>` or typed equivalent:
      - Find next available (not dead, not spawned) starship in side's canonical queue
    - `get_initial_starships() -> Result<(), ShipError>`:
      - For each side, spawn the first available ship

### Boundary clarification (mandatory)
- This phase must explicitly split ship-runtime initialization from broader battle/environment orchestration.
- `InitSpace()` / `UninitSpace()` from C may only be ported to Rust ships for the subset that is truly a ship-runtime dependency.
- Any remaining environment/display-list/background/planet setup owned by the battle engine stays in C and is merely integrated with from ships.
- P09 deliverables must name which `init.c` responsibilities move to `lifecycle.rs` and which remain in C battle/environment code.

### Files to modify

- `rust/src/ships/mod.rs`
  - Add `pub mod lifecycle;`

- `rust/src/game_init/init.rs`
  - Replace stub `init_ships()` / `uninit_ships()` with delegation to `ships::lifecycle`

### Pseudocode traceability
- Uses pseudocode component 6, lines 270-303
- Uses pseudocode component 8, lines 360-396
- Adjusts spawn signature/model to the canonical C-owned `STARSHIP` integration decided in P03.5

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `lifecycle.rs` created with spawn, init, uninit functions
- [ ] `game_init/init.rs` delegates to ships::lifecycle
- [ ] Spawn entrypoint signature matches the Phase 03.5 ABI contract
- [ ] Ship-runtime versus battle-environment ownership split is documented concretely
- [ ] Plan/requirement traceability markers present

## Semantic Verification Checklist (Mandatory)
- [ ] `spawn_ship()` loads descriptor, patches crew, creates element, binds callbacks
- [ ] `spawn_ship()` operates on canonical C-owned queue entries rather than Rust-only queue objects
- [ ] `spawn_ship()` on already-spawned entry returns early (idempotent)
- [ ] `spawn_ship()` on destroyed entry returns early
- [ ] Spawn failure cleans up: no partial descriptor, no dangling element
- [ ] `init_ships()` loads only ship-runtime dependencies and returns NUM_SIDES
- [ ] Remaining broader environment orchestration responsibilities are explicitly left in C where required by the boundary contract
- [ ] `uninit_ships()` frees all active descriptors with teardown hooks
- [ ] `uninit_ships()` writes back crew to fragments
- [ ] init/uninit round-trip leaves clean state
- [ ] Spawn failure does not affect other active ships
- [ ] Mixed C/Rust smoke test exercises real spawn entrypoint against canonical queue storage and callback registration
- [ ] No placeholder/deferred implementation patterns remain

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/ships/lifecycle.rs
```

## Success Criteria
- [ ] Spawn, init, uninit all compile and pass tests
- [ ] Idempotency verified
- [ ] Failure isolation verified
- [ ] Boundary ownership split with C battle/environment orchestration is explicit and consistent
- [ ] game_init integration working
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git checkout -- rust/src/ships/lifecycle.rs rust/src/game_init/init.rs`

## Phase Completion Marker
Create: `project-plans/20260311/ships/.completed/P09.md`
