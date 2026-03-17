# Phase 04: RNG & World Classification (TDD + Impl)

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P04`

## Prerequisites
- Required: Phase 03a (Core Types Verification) completed
- Expected files: all P03 stub files compiling

## Requirements Implemented (Expanded)

### REQ-PSS-RNG-001: Deterministic RNG seeding
**Requirement text**: The star seed shall be derived deterministically from the star descriptor. The same star shall always produce the same seed, which shall always produce the same generated outputs.

Behavior contract:
- GIVEN: A star descriptor with known Type, Index, Prefix, Postfix, star_pt
- WHEN: `get_random_seed_for_star(star)` is called
- THEN: The returned seed is deterministic and matches the C implementation

### REQ-PSS-RNG-002: RNG isolation
**Requirement text**: Procedural generation for solar-system content shall use a dedicated RNG context that is isolated from unrelated game activity.

Behavior contract:
- GIVEN: SysGenRng is seeded
- WHEN: Generation functions draw random values
- THEN: The sequence is independent of combat, UI, or other RNG usage

### REQ-PSS-WORLD-001: World classification — planet vs. moon
**Requirement text**: The subsystem shall distinguish planets from moons and provide classification and indexing operations.

Behavior contract:
- GIVEN: A `WorldRef` or equivalent identity for a populated system
- WHEN: classification helpers are called
- THEN: Planet vs. moon identity is returned correctly

### REQ-PSS-WORLD-002: Planet/moon index consistency with persistence
**Requirement text**: Planet indices and moon indices produced by the subsystem's classification operations shall be consistent with the indices used by the persistence subsystem for scan-retrieval mask addressing.

Behavior contract:
- GIVEN: A world at planet_index=2, moon_index=1
- WHEN: Classification returns indices, and those are used for get_planet_info
- THEN: The same scan-retrieval record is accessed

### REQ-PSS-WORLD-003: World matching
**Requirement text**: Matching a world descriptor against a specific planet-index/moon-index pair.

## Implementation Tasks

### Files to modify

- `rust/src/planets/rng.rs` — Full RNG implementation
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P04`
  - marker: `@requirement REQ-PSS-RNG-001, REQ-PSS-RNG-002`
  - Implement `SysGenRng` with:
    - `new() -> Self`
    - `seed(seed: u32)` — seed the RNG context
    - `random() -> u32` — next random value (must match C RandomContext behavior)
    - `random_range(low: u32, high: u32) -> u32`
  - Implement `get_random_seed_for_star(star: &StarDesc) -> u32`
  - The RNG algorithm must exactly reproduce the C `RandomContext` sequence for the same seed
  - Uses pseudocode lines: 002, 210

- `rust/src/planets/world_class.rs` — Full classification implementation
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P04`
  - marker: `@requirement REQ-PSS-WORLD-001, REQ-PSS-WORLD-002, REQ-PSS-WORLD-003`
  - Implement helpers using explicit identity types:
    - `world_is_planet(world: &WorldRef) -> bool`
    - `world_is_moon(world: &WorldRef) -> bool`
    - `planet_index(world: &WorldRef) -> usize`
    - `moon_index(world: &WorldRef) -> Option<usize>`
    - `match_world(world: &WorldRef, planet_i: u8, moon_i: u8) -> bool`
    - `classify_world_for_persistence(state: &SolarSysState, world: &WorldRef) -> PersistenceAddress`
    - `player_in_solar_system() -> bool`
    - `player_in_planet_orbit() -> bool`
    - `player_in_inner_system() -> bool`
  - Design note: do not collapse planet/moon identity into a single raw `usize` where that loses category information
  - Uses pseudocode lines: 340-369

### Files to create

- `rust/src/planets/tests/rng_tests.rs` — RNG tests
  - Test seed reproducibility (same seed produces same sequence)
  - Test `get_random_seed_for_star` with known star descriptors
  - Test RNG isolation (two separate `SysGenRng` instances don't interfere)
  - Fixture tests against C reference values (captured from C runtime)

- `rust/src/planets/tests/world_class_tests.rs` — Classification tests
  - Test planet/moon discrimination with populated state
  - Test index computation correctness
  - Test `match_world` with planet-only and planet+moon combinations
  - Test edge cases: first planet, last planet, first moon, last moon
  - Test consistency with `ScanRetrieveMask` addressing (cross-reference with `state::planet_info`)
  - Test save/orbit identity helpers preserve planet vs. moon distinction

### Files to modify

- `rust/src/planets/tests/mod.rs`
  - Add `mod rng_tests;` and `mod world_class_tests;`

## Pseudocode Traceability
- Uses pseudocode lines: 002, 210, 340-369

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rng.rs` contains full `SysGenRng` implementation (no todo!())
- [ ] `world_class.rs` contains full classification implementation (no todo!())
- [ ] Test files created and included in test module
- [ ] Plan/requirement traceability present

## Semantic Verification Checklist
- [ ] RNG produces deterministic output for same seed
- [ ] `get_random_seed_for_star` matches C output for known stars
- [ ] World classification correctly distinguishes planets from moons
- [ ] Index computation is consistent with persistence addressing
- [ ] `match_world` correctly handles MATCH_PLANET sentinel
- [ ] Save/orbit helpers retain explicit planet/moon identity without ambiguous raw-index conventions

## Deferred Implementation Detection

```bash
grep -RIn "todo!()\|unimplemented!()" rust/src/planets/rng.rs rust/src/planets/world_class.rs
# Should return 0 matches — these modules are fully implemented
```

## Success Criteria
- [ ] All RNG tests pass
- [ ] All world classification tests pass
- [ ] RNG output matches C reference values
- [ ] No placeholder code in `rng.rs` or `world_class.rs`

## Failure Recovery
- rollback: `git checkout -- rust/src/planets/rng.rs rust/src/planets/world_class.rs rust/src/planets/tests/`

## Phase Completion Marker
Create: `project-plans/20260311/planet-solarsys/.completed/P04.md`
