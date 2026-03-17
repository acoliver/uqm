# Phase 04: ShipBehavior Trait & Registry

## Phase ID
`PLAN-20260314-SHIPS.P04`

## Prerequisites
- Required: Phase 03.5a (FFI Boundary & Ownership Contract Verification) completed and PASS
- Expected files: `types.rs` with all core types, `ffi_contract.rs` with canonical ABI/ownership rules

## Requirements Implemented (Expanded)

### Behavioral Hook Registration
**Requirement text**: The subsystem shall support registration of per-descriptor-instance behavioral hooks: preprocess, postprocess, weapon initialization, AI intelligence, and teardown.

Behavior contract:
- GIVEN: A race implementation struct
- WHEN: It implements `ShipBehavior`
- THEN: All hooks are available through trait dispatch, null hooks default to no-op

### Hook Dispatch Properties
**Requirement text**: Hooks are per-descriptor-instance. A race may change its own hooks during the ship's lifetime. A null/absent hook shall be treated as a no-op. Hook calls are serialized.

Behavior contract:
- GIVEN: A `Box<dyn ShipBehavior>`
- WHEN: The trait default methods are not overridden
- THEN: They behave as no-ops, and the ship functions normally

### Race Dispatch and Full Template Coverage
**Requirement text**: The subsystem shall provide a dispatch mechanism that maps a ship's species identity to the race implementation that produces its descriptor template.

Behavior contract:
- GIVEN: Any valid `SpeciesId`
- WHEN: registry/template construction is requested
- THEN: Complete template metadata exists for all 28 species, even when full combat behavior for some races is deferred to later batches

## Implementation Tasks

### Files to create

- `rust/src/ships/traits.rs` — ShipBehavior trait definition
  - marker: `@plan PLAN-20260314-SHIPS.P04`
  - marker: `@requirement REQ-HOOKS-REGISTRATION, REQ-AI-HOOK, REQ-NULL-HOOK-NOOP, REQ-HOOK-CHANGE`
  - Contents:
    - `ShipState` struct (mutable view into ship runtime state for hooks)
    - `BattleContext` struct (read-only battle environment context)
    - `WeaponElement` struct (weapon/projectile description returned by init_weapon)
    - `CollisionHandler` type (callback for collision override)
    - `ShipBehavior` trait:
      - `fn descriptor_template(&self) -> RaceDescTemplate` (static data for this race)
      - `fn preprocess(&mut self, ship: &mut ShipState, ctx: &BattleContext) -> Result<()>` (default: Ok(()))
      - `fn postprocess(&mut self, ship: &mut ShipState, ctx: &BattleContext) -> Result<()>` (default: Ok(()))
      - `fn init_weapon(&mut self, ship: &ShipState, ctx: &BattleContext) -> Result<Vec<WeaponElement>>` (default: Ok(vec![]))
      - `fn intelligence(&mut self, ship: &ShipState, ctx: &BattleContext) -> StatusFlags` (default: empty)
      - `fn uninit(&mut self)` (default: no-op)
      - `fn collision_override(&self) -> Option<CollisionHandler>` (default: None)

- `rust/src/ships/registry.rs` — Ship dispatch/registration
  - marker: `@plan PLAN-20260314-SHIPS.P04`
  - marker: `@requirement REQ-NONMELEE-SAME-RUNTIME, REQ-ROSTER-PRESERVE, REQ-MUTATION-PRESERVE`
  - Contents:
    - `RaceDescTemplate` remains data-only and safe to construct before full race behavior is complete
    - `descriptor_template_for_species(species: SpeciesId) -> Result<RaceDescTemplate, ShipError>` — **mandatory complete table covering all 28 species before P05 begins**
    - The template table includes baseline metadata/characteristics/resource identifiers for all 25 melee ships plus SIS, Sa-Matra, and Probe
    - `create_ship_behavior(species: SpeciesId) -> Result<Box<dyn ShipBehavior>, ShipError>` — match on all 28 species, returning live behavior only for phases/races already implemented
    - `create_race_desc(species: SpeciesId) -> Result<RaceDesc, ShipError>` — creates descriptor instance from the mandatory full-species template table plus a live behavior object when available
    - `create_metadata_only_desc(species: SpeciesId) -> Result<RaceDesc, ShipError>` — **mandatory metadata-safe path** used by P05/P06; must succeed for all 28 species without depending on later race batches
    - `TemplateOnlyShip` or equivalent metadata-safe behavior object is **required**, not optional, for species whose full combat behavior has not yet been ported
    - The metadata-only path must provide safe no-op hooks and accurate template data without panic, hidden combat claims, or fake runtime parity
    - Live battle spawn/runtime paths may return `ShipError::UnimplementedSpecies(species)` until the corresponding race batch is complete, but metadata-only creation may not
    - **No `StubShip`, `unimplemented!()`, or panic-on-use placeholder registry path is allowed**

- `rust/src/ships/races/mod.rs` — Race module root (empty initially, populated in P11-P13)
  - marker: `@plan PLAN-20260314-SHIPS.P04`

### Files to modify

- `rust/src/ships/mod.rs`
  - Add `pub mod traits;` and `pub mod registry;` and `pub mod races;`
  - Re-export key types

### Pseudocode traceability
- Uses pseudocode component 1, lines 01-24

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust/src/ships/traits.rs` created with `ShipBehavior` trait
- [ ] `rust/src/ships/registry.rs` created with dispatch function
- [ ] `rust/src/ships/races/mod.rs` created
- [ ] All 28 species have a template-table entry before P05 begins
- [ ] All 28 species have a match arm or equivalent coverage in behavior dispatch
- [ ] Trait has default implementations for all optional hooks
- [ ] Metadata-only creation path is explicit and independent of later race batches
- [ ] No panic-on-use placeholder registry implementation exists
- [ ] Plan/requirement traceability markers present

## Semantic Verification Checklist (Mandatory)
- [ ] `descriptor_template_for_species()` succeeds for all 28 valid species IDs
- [ ] `create_metadata_only_desc()` succeeds for all 28 valid species IDs and is safe for catalog/analysis work
- [ ] `create_ship_behavior()` returns Ok for implemented SpeciesId values
- [ ] Live battle/runtime creation returns explicit Err for not-yet-implemented species only where combat behavior is actually required
- [ ] Each descriptor instance has an AI hook registration surface via `ShipBehavior::intelligence()`
- [ ] Default trait methods are no-ops (don't panic)
- [ ] Default `intelligence()` returns empty/no-input flags and is safe for metadata/template-only behavior objects
- [ ] `create_race_desc()` produces a RaceDesc with all fields populated from template for implemented species
- [ ] Template-only fallback cannot panic if touched by tests or metadata-only callers
- [ ] No placeholder patterns in traits.rs or registry.rs dispatch logic

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|StubShip\|unimplemented!" rust/src/ships/traits.rs rust/src/ships/registry.rs
```

## Success Criteria
- [ ] Trait compiles and all default methods work
- [ ] Registry/template coverage spans all 28 species without panic stubs
- [ ] Metadata-only descriptor creation is available for all 28 species before P05/P06
- [ ] Unit tests for registry dispatch and full template coverage pass
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git checkout -- rust/src/ships/traits.rs rust/src/ships/registry.rs rust/src/ships/races/`
- blocking issues: trait object safety issues, Send/Sync requirements

## Phase Completion Marker
Create: `project-plans/20260311/ships/.completed/P04.md`
