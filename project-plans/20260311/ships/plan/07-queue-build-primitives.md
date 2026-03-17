# Phase 07: Queue & Build Primitives

## Phase ID
`PLAN-20260314-SHIPS.P07`

## Prerequisites
- Required: Phase 06a (Catalog Verification) completed and PASS
- Expected files: `types.rs` with Starship/ShipFragment/FleetInfo bridge types, `catalog.rs` with lookup functions, `ffi_contract.rs` with canonical queue ownership rules

## Requirements Implemented (Expanded)

### Combat Queue Model
**Requirement text**: The subsystem shall define the shared combat queue data contracts and provide helper operations for: allocating a new queue entry for a given species identity, initializing the entry with catalog or campaign metadata, enqueuing the entry into a side's combat queue, and looking up queue entries by index.

Behavior contract:
- GIVEN: A species ID and a target queue
- WHEN: `build_ship()` is called
- THEN: A new zero-initialized Starship entry is created and enqueued according to the established queue-owner contract

### Ship Fragment Model
**Requirement text**: The subsystem shall support a persistent ship-fragment model carrying species identity, current crew level, and display metadata, sufficient to reconstruct a combat queue entry for battle.

Behavior contract:
- GIVEN: Fleet state or catalog entry
- WHEN: `clone_ship_fragment()` is called
- THEN: A ShipFragment with correct metadata is produced

### Fleet-Info Model
**Requirement text**: The subsystem shall support a fleet-info model for campaign-level fleet state, carrying allied/hostile status, fleet size and growth, encounter composition, known location, sphere-of-influence tracking, and actual fleet strength.

Behavior contract:
- GIVEN: Campaign fleet data
- WHEN: Fleet-info is queried or updated through ships helpers
- THEN: Allied/hostile status, fleet size/growth, encounter composition, known location, sphere tracking, and actual strength are available

### Queue Helpers
**Requirement text**: External systems decide which entries to create; the subsystem provides the data contracts and helpers they consume.

Behavior contract:
- GIVEN: A populated queue
- WHEN: `get_starship_from_index()` is called with valid index
- THEN: Returns the correct queue entry from canonical queue storage

## Implementation Tasks

### Files to create

- `rust/src/ships/queue.rs` — Queue & build primitives
  - marker: `@plan PLAN-20260314-SHIPS.P07`
  - marker: `@requirement REQ-QUEUE-MODEL, REQ-QUEUE-OWNER-BOUNDARY, REQ-FRAGMENT-MODEL, REQ-FRAGMENT-CLONE, REQ-FLEET-INFO`
  - Contents:
    - Bridge helpers that operate over canonical queue ownership established in P03.5
    - If local helper containers are needed for tests, they are explicitly test-only mirrors and not authoritative runtime queues
    - `build_ship(queue_head: *mut STARSHIP, species_id: SpeciesId) -> Result<StarshipHandle, ShipError>` or the exact typed equivalent established by the ABI contract:
      - Creates/initializes a queue entry in canonical queue storage
    - `get_starship_from_index(queue_head: *mut STARSHIP, index: usize) -> Option<*mut STARSHIP>` or typed equivalent
    - `clone_ship_fragment(src: &MasterShipInfo, dst: &mut ShipFragment)` or typed bridge equivalent:
      - Copies species_id, crew/energy metadata, icon/string handles per ownership contract
    - `add_escort_ships(...) -> usize`
    - `count_escort_ships(...) -> usize`
    - `have_escort_ship(...) -> bool`
    - `set_race_allied(fleet_info: &mut FleetInfo, allied: bool)`
    - `escort_feasibility_study(...) -> bool`
    - `start_sphere_tracking(fleet_info: &mut FleetInfo)`
    - Explicit support for `avail_race_q` / `built_ship_q`-style campaign queue conventions where those are part of the current subsystem boundary
    - Exact helper signatures and touched files/functions must be aligned with `build.c` parity, including campaign-facing helper behavior required by encounter composition and sphere tracking
    - `StarshipHandle` type only if the ABI contract explicitly uses handles; otherwise use typed pointers consistently

### Explicit non-goal for this phase
- Do **not** introduce new canonical Rust-owned global runtime queues such as `RACE_Q`, `BUILT_SHIP_Q`, or `AVAIL_RACE_Q` unless the ABI contract has explicitly migrated ownership for all callers. This plan does not make that migration.

### Files to modify

- `rust/src/ships/mod.rs`
  - Add `pub mod queue;`

### Pseudocode traceability
- Uses pseudocode component 4, lines 130-162, adjusted to operate on canonical boundary storage rather than parallel Rust-owned global queues

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `queue.rs` created with build/lookup/clone functions
- [ ] Queue helper signatures match the ownership model established in P03.5
- [ ] Campaign/build helper surface from `build.c` is explicitly covered, including escort feasibility and sphere tracking helpers
- [ ] No parallel canonical Rust-owned global queue state is introduced
- [ ] Plan/requirement traceability markers present

## Semantic Verification Checklist (Mandatory)
- [ ] `build_ship()` creates correctly zero-initialized Starship entries in canonical queue storage
- [ ] `get_starship_from_index()` returns the correct entry using canonical queue ordering
- [ ] `clone_ship_fragment()` copies all required metadata
- [ ] `add_escort_ships()` respects MAX_BUILT_SHIPS limit
- [ ] `count_escort_ships()` / `have_escort_ship()` preserve existing queue semantics
- [ ] `escort_feasibility_study()` preserves campaign helper behavior required by current encounter flow
- [ ] `start_sphere_tracking()` preserves sphere-of-influence tracking behavior
- [ ] Queue ordering is preserved (insertion order)
- [ ] Fragment cloning transfers or shares handle references according to the documented ownership contract
- [ ] Mixed C/Rust smoke test covers at least one real queue helper invocation against C-owned queue memory
- [ ] No placeholder/deferred implementation patterns remain

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/ships/queue.rs
```

## Success Criteria
- [ ] Queue operations compile and pass tests
- [ ] Build/lookup/clone all work correctly
- [ ] Campaign/build helper parity surface is covered concretely enough for encounter consumers
- [ ] Canonical queue ownership is preserved
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git checkout -- rust/src/ships/queue.rs`

## Phase Completion Marker
Create: `project-plans/20260311/ships/.completed/P07.md`
