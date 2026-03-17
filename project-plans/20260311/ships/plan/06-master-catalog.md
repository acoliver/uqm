# Phase 06: Master Ship Catalog

## Phase ID
`PLAN-20260314-SHIPS.P06`

## Prerequisites
- Required: Phase 05a (Loader Verification) completed and PASS
- Expected files: `loader.rs` with `load_ship()` / `free_ship()`, `ffi_contract.rs` with catalog pointer/lifetime rules, and mandatory full-species template coverage already established in P04/P05

## Requirements Implemented (Expanded)

### Master Catalog
**Requirement text**: The subsystem shall maintain a master ship catalog enumerating all melee-eligible ships, providing for each entry: species identity, cost/value, display icons, melee icons, and race name strings. The master catalog shall be sorted by race name for stable enumeration order.

Behavior contract:
- GIVEN: The game is starting up
- WHEN: `load_master_ship_list()` is called
- THEN: A sorted catalog of all melee-eligible ships is available for lookup

### Catalog Startup/Shutdown
**Requirement text**: When the engine starts up, the subsystem shall load the master ship catalog using metadata-only loading. When the engine shuts down, the subsystem shall free all master catalog resources.

Behavior contract:
- GIVEN: Engine lifecycle
- WHEN: Startup completes
- THEN: Catalog is loaded, sorted, and available; on shutdown, all resources freed

### Catalog Exclusion
**Requirement text**: The master catalog shall not include non-melee ships.

Behavior contract:
- GIVEN: The loaded catalog
- WHEN: Queried for SIS_SHIP_ID, SA_MATRA_ID, or UR_QUAN_PROBE_ID
- THEN: Returns None / not found

### Catalog Lookups
**Requirement text**: The subsystem shall provide lookup accessors for catalog entries by species identity, by enumeration index, and for ship cost, display icons, and melee icons by index, without triggering battle-asset loading.

Behavior contract:
- GIVEN: A loaded catalog
- WHEN: Lookup by species or index
- THEN: Returns metadata without loading battle assets

## Implementation Tasks

### Files to create

- `rust/src/ships/catalog.rs` — Master ship catalog
  - marker: `@plan PLAN-20260314-SHIPS.P06`
  - marker: `@requirement REQ-CATALOG, REQ-CATALOG-SORT, REQ-CATALOG-STARTUP, REQ-CATALOG-SHUTDOWN, REQ-CATALOG-EXCLUSION, REQ-CATALOG-LOOKUP`
  - Contents:
    - `MASTER_CATALOG: Mutex<Option<Vec<MasterShipInfo>>>` or equivalent stable store whose returned entries remain valid for the documented catalog lifetime
    - `MasterShipInfo` entries are **metadata snapshots with explicit ownership**, not shallow copies from temporary descriptors
    - `load_master_ship_list() -> Result<(), ShipError>`:
      - Iterates only the 25 melee-eligible species
      - Calls `loader::load_ship(species, MetadataOnly)` for each
      - Builds a dedicated catalog-owned metadata snapshot from the descriptor
      - Sorts by race name string
      - Stores in `MASTER_CATALOG`
    - `free_master_ship_list()`:
      - Frees icons, melee_icon, race_strings for each entry using the catalog-owned metadata snapshot rules
      - Clears `MASTER_CATALOG`
    - `find_master_ship(species_id: SpeciesId) -> Option<&'static MasterShipInfo>` or an equivalent accessor model that preserves the lifetime rules documented in P03.5
    - `find_master_ship_by_index(index: usize) -> Option<&'static MasterShipInfo>` or equivalent
    - `get_ship_cost_from_index(index: usize) -> Option<u16>`
    - `get_ship_icons_from_index(index: usize) -> Option<FrameHandle>`
    - `get_ship_melee_icons_from_index(index: usize) -> Option<FrameHandle>`
    - `is_catalog_loaded() -> bool`
    - `catalog_count() -> usize`

### Metadata coverage dependency (mandatory)
- P06 must rely only on the metadata-complete template/loader path established in P04/P05.
- P06 must **not** depend on live race combat behavior being implemented for all species.
- The phase explicitly separates:
  - **full metadata coverage** — already required before catalog work, and
  - **full combat behavior coverage** — completed later in P11-P13.
- Catalog startup must therefore remain satisfiable even while some species still lack live battle behavior.

### Ownership requirements for metadata handles
- The phase must choose and document one explicit model:
  - `MasterShipInfo` owns dedicated metadata handles independent of `RaceDesc`, or
  - catalog entries hold shared/refcounted metadata handles, or
  - metadata-only loader returns a distinct catalog-owned metadata struct.
- The implementation may **not** rely on vague "transfer" semantics from a soon-to-be-freed temporary descriptor without an explicit type/API boundary that proves single ownership and exact cleanup behavior.

### Files to modify

- `rust/src/ships/mod.rs`
  - Add `pub mod catalog;`

- `rust/src/game_init/master.rs`
  - Replace placeholder `MasterShipList` with delegation to `ships::catalog`
  - `load_master_ship_list()` → `ships::catalog::load_master_ship_list()`
  - `free_master_ship_list()` → `ships::catalog::free_master_ship_list()`
  - Keep existing test-compatible API surface but backed by real catalog

### Pseudocode traceability
- Uses pseudocode component 3, lines 80-124, with ownership clarified by Phase 03.5 contracts

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust/src/ships/catalog.rs` created with all lookup functions
- [ ] `game_init/master.rs` delegates to `ships::catalog`
- [ ] Catalog uses a storage model compatible with the documented returned-pointer lifetime
- [ ] Metadata ownership model is explicit in types and comments
- [ ] Catalog implementation depends only on metadata-complete loading, not unfinished live race behavior
- [ ] Plan/requirement traceability markers present

## Semantic Verification Checklist (Mandatory)
- [ ] `load_master_ship_list()` loads exactly the melee-eligible ships (25 races)
- [ ] Non-melee ships are excluded from catalog
- [ ] Catalog is sorted by race name
- [ ] `find_master_ship()` returns correct entry for valid species
- [ ] `find_master_ship()` returns None for non-melee and invalid species
- [ ] `get_ship_cost_from_index()` returns correct cost values
- [ ] `free_master_ship_list()` frees all catalog-owned resources exactly once
- [ ] Double-load returns error (already loaded)
- [ ] Metadata handles are not leaked or double-freed when temporary descriptors are released
- [ ] Catalog loading remains available before P11-P13 completes
- [ ] No placeholder/deferred implementation patterns remain

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/ships/catalog.rs
```

## Success Criteria
- [ ] Catalog loads, sorts, and provides correct lookups
- [ ] Non-melee exclusion verified
- [ ] Resource cleanup verified
- [ ] game_init integration verified
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git checkout -- rust/src/ships/catalog.rs rust/src/game_init/master.rs`

## Phase Completion Marker
Create: `project-plans/20260311/ships/.completed/P06.md`
