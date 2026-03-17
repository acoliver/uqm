# Phase 05: Two-Tier Ship Loader

## Phase ID
`PLAN-20260314-SHIPS.P05`

## Prerequisites
- Required: Phase 04a (Trait & Registry Verification) completed and PASS
- Expected files: `traits.rs` with ShipBehavior, `registry.rs` with full 28-species template coverage and `create_metadata_only_desc()`

## Requirements Implemented (Expanded)

### Metadata-Only Loading
**Requirement text**: When a ship is loaded at the metadata-only tier, the subsystem shall allocate and initialize the ship descriptor, load icon assets, melee icons, and race name strings, but shall not load battle frame arrays, captain graphics, victory audio, or ship sounds.

Behavior contract:
- GIVEN: A valid SpeciesId and `LoadTier::MetadataOnly`
- WHEN: `load_ship()` is called
- THEN: Descriptor has icons and strings loaded, but ship_data battle fields remain None

### Battle-Ready Loading
**Requirement text**: When a ship is loaded at the battle-ready tier, the subsystem shall perform all metadata-only loading and additionally load ship body frames, weapon frames, special-ability frames, captain background graphics, victory music, and ship sounds.

Behavior contract:
- GIVEN: A valid SpeciesId and `LoadTier::BattleReady`
- WHEN: `load_ship()` is called
- THEN: All metadata AND battle assets are loaded

### Descriptor Free
**Requirement text**: When a ship descriptor is freed, the subsystem shall release whichever assets were loaded according to the tier, and shall invoke the race-specific teardown hook if registered.

Behavior contract:
- GIVEN: A loaded descriptor (either tier)
- WHEN: `free_ship()` is called
- THEN: All loaded assets are freed, teardown hook is invoked, no leaks

### Load Failure
**Requirement text**: If a ship descriptor fails to load, the subsystem shall free any resources that were successfully loaded before the failure, shall not leave a partially initialized descriptor reachable.

Behavior contract:
- GIVEN: A load attempt where one resource is missing
- WHEN: `load_ship()` fails midway
- THEN: Previously loaded resources are freed, Err is returned, no partial descriptor escapes

## Implementation Tasks

### Files to create

- `rust/src/ships/loader.rs` — Two-tier ship loading
  - marker: `@plan PLAN-20260314-SHIPS.P05`
  - marker: `@requirement REQ-METADATA-LOAD, REQ-BATTLE-LOAD, REQ-DESCRIPTOR-FREE, REQ-LOAD-FAILURE`
  - Contents:
    - `LoadTier` enum: `MetadataOnly`, `BattleReady`
    - `load_ship(species_id: SpeciesId, tier: LoadTier) -> Result<RaceDesc, ShipError>`:
      - Uses `registry::create_metadata_only_desc()` for the metadata-only baseline so P05/P06 do not depend on later live race behavior batches
      - Metadata tier: loads icons, melee_icon, race_strings via resource FFI
      - Battle tier: additionally upgrades the descriptor with ship/weapon/special frames (3 resolutions each), captain graphics, victory music, ship sounds, and any battle-only runtime behavior object required for implemented species
      - Metadata-only loading must succeed for all 28 species because catalog/special-path consumers depend on full template coverage before P11-P13
      - Battle-ready loading may return `ShipError::UnimplementedSpecies(species)` only when a species' live combat behavior has not yet reached its assigned race batch
      - On failure: cleans up any already-loaded resources before returning Err
    - `free_ship(desc: &mut RaceDesc, free_battle: bool, free_metadata: bool)`:
      - Calls `desc.behavior.uninit()` (teardown hook)
      - If free_battle: frees ship_data frames, captain, victory, sounds
      - If free_metadata: frees icons, melee_icon, race_strings
    - `ShipError` enum: `UnknownSpecies`, `UnimplementedSpecies(SpeciesId)`, `ResourceLoadFailed(String)`, `AllocationFailed`

- `rust/src/ships/c_bridge.rs` — Rust-to-C bridge for resource loading
  - marker: `@plan PLAN-20260314-SHIPS.P05`
  - Contents:
    - `load_graphic(res: ResourceId) -> Result<FrameHandle, ShipError>` — wraps C `LoadGraphic()`
    - `load_music(res: ResourceId) -> Result<MusicHandle, ShipError>` — wraps C `LoadMusic()`
    - `load_sound(res: ResourceId) -> Result<SoundHandle, ShipError>` — wraps C `LoadSound()`
    - `load_string_table(res: ResourceId) -> Result<StringTableHandle, ShipError>` — wraps C `LoadStringTable()`
    - `free_graphic(handle: FrameHandle)` — wraps C `DestroyDrawable()`
    - `free_music(handle: MusicHandle)` — wraps C `DestroyMusic()`
    - `free_sound(handle: SoundHandle)` — wraps C `DestroySound()`
    - `free_string_table(handle: StringTableHandle)` — wraps C `DestroyStringTable()`
    - All functions use `extern "C"` bindings to call into C

### Files to modify

- `rust/src/ships/mod.rs`
  - Add `pub mod loader;` and `pub mod c_bridge;`

### Pseudocode traceability
- Uses pseudocode component 2, lines 30-76

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust/src/ships/loader.rs` created with `load_ship()` and `free_ship()`
- [ ] `rust/src/ships/c_bridge.rs` created with resource bridge functions
- [ ] `LoadTier` enum has exactly two variants
- [ ] `ShipError` covers unknown species, unimplemented live behavior, and resource/allocation failure modes
- [ ] Loader explicitly separates metadata-complete coverage from live combat-behavior coverage
- [ ] Plan/requirement traceability markers present

## Semantic Verification Checklist (Mandatory)
- [ ] `load_ship(_, MetadataOnly)` does NOT load battle assets (verified by test)
- [ ] `load_ship(_, MetadataOnly)` succeeds for all 28 species (verified by test)
- [ ] `load_ship(_, BattleReady)` loads ALL assets for implemented species (verified by test)
- [ ] Battle-ready loads for not-yet-implemented species fail explicitly with `UnimplementedSpecies` rather than panic/placeholder behavior
- [ ] `free_ship()` calls `uninit()` on the behavior (verified by mock test)
- [ ] Load failure cleans up partial resources (verified by simulated failure test)
- [ ] Unknown species returns Err (verified by test)
- [ ] No placeholder/deferred implementation patterns remain

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/ships/loader.rs rust/src/ships/c_bridge.rs
```

## Success Criteria
- [ ] Two-tier loading compiles and passes tests
- [ ] Full metadata coverage exists before catalog work
- [ ] Failure cleanup verified
- [ ] Teardown hook invocation verified
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git checkout -- rust/src/ships/loader.rs rust/src/ships/c_bridge.rs`
- blocking issues: C resource API not accessible via FFI (requires bridge creation first)

## Phase Completion Marker
Create: `project-plans/20260311/ships/.completed/P05.md`
