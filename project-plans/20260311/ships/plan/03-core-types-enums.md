# Phase 03: Core Types & Enums

## Phase ID
`PLAN-20260314-SHIPS.P03`

## Prerequisites
- Required: Phase 02a (Pseudocode Verification) completed and PASS
- Expected files from previous phase: pseudocode document with all 9 components

## Requirements Implemented (Expanded)

### Ship Identity
**Requirement text**: The subsystem shall assign a unique species identity to every ship in the game, spanning both melee-eligible and non-melee ships within a single identity space. The subsystem shall define a clear boundary between melee-eligible ships and non-melee ships.

Behavior contract:
- GIVEN: The full species roster
- WHEN: A species ID is queried
- THEN: It maps to exactly one ship, and `is_melee_eligible()` returns the correct status

### Ship Descriptor Model
**Requirement text**: Each ship's runtime behavior shall be expressed through a ship descriptor aggregating: ship info, fleet characteristics, movement/energy characteristics, battle data, AI parameters, and race-specific behavioral hooks. Each ship descriptor shall carry an opaque private-data slot.

Behavior contract:
- GIVEN: A species ID
- WHEN: A descriptor is created
- THEN: All sub-structures are present and correctly typed

### Capability Flags
**Requirement text**: The subsystem shall define capability flags characterizing externally observable ship combat properties.

Behavior contract:
- GIVEN: A ship info structure
- WHEN: Flags are queried
- THEN: All C-equivalent flags are representable and queryable

## Implementation Tasks

### Files to create

- `rust/src/ships/mod.rs` — Module root
  - marker: `@plan PLAN-20260314-SHIPS.P03`
  - Declares submodules: `types`, `traits`, `registry`, `loader`, `catalog`, `queue`, `runtime`, `lifecycle`, `writeback`, `ffi`, `c_bridge`, `races`

- `rust/src/ships/types.rs` — All core types
  - marker: `@plan PLAN-20260314-SHIPS.P03`
  - marker: `@requirement REQ-SHIP-IDENTITY, REQ-SHIP-DESCRIPTOR, REQ-CAPABILITY-FLAGS`
  - Contents:
    - `SpeciesId` enum (`#[repr(i32)]`): `Arilou = 1`, `Chmmr = 2`, ..., `Mmrnmhrm = 25` (LAST_MELEE), `SisShip = 26`, `SaMatra = 27`, `UrQuanProbe = 28`. With `NUM_SPECIES`, `LAST_MELEE_ID` constants. Methods: `is_melee_eligible()`, `from_i32()`.
    - `ShipFlags` bitflags: `SEEKING_WEAPON`, `SEEKING_SPECIAL`, `POINT_DEFENSE`, `IMMEDIATE_WEAPON`, `CREW_IMMUNE`, `FIRES_FORE`, `FIRES_RIGHT`, `FIRES_AFT`, `FIRES_LEFT`, `SHIELD_DEFENSE`, `DONT_CHASE`, `PLAYER_CAPTAIN`
    - `StatusFlags` bitflags: `LEFT`, `RIGHT`, `THRUST`, `WEAPON`, `SPECIAL`, `LOW_ON_ENERGY`, `SHIP_BEYOND_MAX_SPEED`, `SHIP_AT_MAX_SPEED`, `SHIP_IN_GRAVITY_WELL`, `PLAY_VICTORY_DITTY`
    - `ShipInfo` struct: `ship_flags: ShipFlags`, `ship_cost: u16`, `crew_level: u16`, `max_crew: u16`, `energy_level: u16`, `max_energy: u16`, `race_strings_res: ResourceId`, `icons_res: ResourceId`, `melee_icon_res: ResourceId`, `race_strings: Option<StringTableHandle>`, `icons: Option<FrameHandle>`, `melee_icon: Option<FrameHandle>`
    - `FleetStuff` struct: `sphere_radius: u32`, `known_loc: (i32, i32)`
    - `Characteristics` struct: `max_thrust: i32`, `thrust_increment: i32`, `energy_regeneration: i32`, `weapon_energy_cost: i32`, `special_energy_cost: i32`, `energy_wait: u8`, `turn_wait: u8`, `thrust_wait: u8`, `weapon_wait: u8`, `special_wait: u8`, `ship_mass: i32`
    - `CaptainStuff` struct: `captain_res: ResourceId`, `background: Option<FrameHandle>`, `turn: Option<FrameHandle>`, `thrust: Option<FrameHandle>`, `weapon: Option<FrameHandle>`, `special: Option<FrameHandle>`
    - `ShipData` struct: `ship: [Option<FrameHandle>; 3]`, `weapon: [Option<FrameHandle>; 3]`, `special: [Option<FrameHandle>; 3]`, `captain: CaptainStuff`, `victory_ditty: Option<MusicHandle>`, `ship_sounds: Option<SoundHandle>`
    - `IntelStuff` struct: `maneuverability_index: i32`, `weapon_range: i32`
    - `RaceDesc` struct: `ship_info: ShipInfo`, `fleet: FleetStuff`, `characteristics: Characteristics`, `ship_data: ShipData`, `intel: IntelStuff`, `behavior: Box<dyn ShipBehavior>`, internal private data managed by behavior
    - `Starship` struct: `species_id: SpeciesId`, `race_desc: Option<Box<RaceDesc>>`, `crew_level: u16`, `max_crew: u16`, `energy_level: u16`, `max_energy: u16`, `cur_status_flags: StatusFlags`, `old_status_flags: StatusFlags`, `ship_facing: u8`, `player_nr: u8`, `weapon_counter: u8`, `special_counter: u8`, `energy_counter: u8`, `turn_counter: u8`, `thrust_counter: u8`, `h_ship: usize` (opaque element handle), `ship_flags: ShipFlags`
    - `ShipFragment` struct: `species_id: SpeciesId`, `crew_level: u16`, `max_crew: u16`, `energy_level: u16`, `max_energy: u16`, `icons: Option<FrameHandle>`, `melee_icon: Option<FrameHandle>`, `race_strings: Option<StringTableHandle>`
    - `FleetInfo` struct: `species_id: SpeciesId`, `allied: bool`, `actual_strength: u16`, `known_strength: u16`, `growth: i16`, `max_fleet_size: u16`, `growth_err_term: i16`, `loc: (i32, i32)`, `known_loc: (i32, i32)`, `actual_fleet_composition: Vec<u8>`, `known_fleet_composition: Vec<u8>`
    - `MasterShipInfo` struct: `species_id: SpeciesId`, `ship_info: ShipInfo`, `fleet: FleetStuff`
    - `ResourceId` type alias (u32 or appropriate handle type)
    - `FrameHandle`, `MusicHandle`, `SoundHandle`, `StringTableHandle` type aliases (opaque u64/usize)
    - `RaceDescTemplate` struct (static template data without loaded assets or behavior)

### Files to modify

- `rust/src/lib.rs`
  - Add `pub mod ships;`
  - marker: `@plan PLAN-20260314-SHIPS.P03`

### Pseudocode traceability
- Uses pseudocode component 1, lines 11-23 (RaceDesc structure)
- Uses pseudocode component 4, lines 130-136 (Starship structure)
- Uses pseudocode component 3, lines 80-84 (MasterShipInfo structure)

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust/src/ships/mod.rs` created with all submodule declarations
- [ ] `rust/src/ships/types.rs` created with all types
- [ ] `rust/src/lib.rs` updated with `pub mod ships;`
- [ ] All types compile without errors
- [ ] SpeciesId has all 28 variants plus sentinels
- [ ] ShipFlags has all 12 capability flags matching C
- [ ] StatusFlags has all 10 status flags matching C
- [ ] Plan/requirement traceability markers present

## Semantic Verification Checklist (Mandatory)
- [ ] `SpeciesId::is_melee_eligible()` returns true for ARILOU..MMRNMHRM, false for SIS/SAMATRA/PROBE
- [ ] `SpeciesId::from_i32()` round-trips correctly for all valid values
- [ ] `ShipFlags` values match C `races.h` bit positions
- [ ] `StatusFlags` values match C `races.h` bit positions
- [ ] All struct fields match C struct field semantics from analysis
- [ ] No placeholder/deferred implementation patterns remain

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/ships/
```

## Success Criteria
- [ ] All types compile
- [ ] Unit tests for SpeciesId round-trip, flag values, melee eligibility pass
- [ ] Verification commands pass
- [ ] Semantic checks pass

## Failure Recovery
- rollback: `git checkout -- rust/src/ships/ rust/src/lib.rs`
- blocking issues: type mismatches with C layout discovered during preflight

## Phase Completion Marker
Create: `project-plans/20260311/ships/.completed/P03.md`
