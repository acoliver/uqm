# Phase 05: Planetary Analysis (TDD)

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P05`

## Prerequisites
- Required: Phase 04a (RNG & World Classification Verification) completed
- Expected: `SysGenRng` fully functional with C-compatible output

## Requirements Implemented (Expanded)

### REQ-PSS-ANALYSIS-001: Analysis computation
**Requirement text**: Planetary analysis shall compute surface temperature, density, radius, gravity, rotation, tilt, tectonics, atmospheric density, weather, and life chance from the world's seed and parent star properties.

Behavior contract:
- GIVEN: A world with seed 0xABCD1234 orbiting a G-type star
- WHEN: `do_planetary_analysis()` is called
- THEN: All output fields are populated with deterministic values matching C baseline

### REQ-PSS-ANALYSIS-002: Output equivalence
**Requirement text**: Planetary analysis shall produce results identical to the established baseline for the same inputs, verified against seeded reference cases.

Behavior contract:
- GIVEN: Known seed/star pairs captured from C runtime
- WHEN: Rust `do_planetary_analysis()` processes the same inputs
- THEN: Every output field matches the C output exactly

### REQ-PSS-ANALYSIS-003: Temperature-color derivation
**Requirement text**: Planetary analysis results shall include a temperature-derived display color. The greenhouse quirk shall be preserved for initial parity.

Behavior contract:
- GIVEN: A planet with computed temperature
- WHEN: `compute_temp_color()` is called
- THEN: Color matches C `temp_color` assignment including the greenhouse mismatch

### REQ-PSS-ANALYSIS-004: Determinism
**Requirement text**: All derivations shall be deterministic functions of the world seed and star properties.

## Implementation Tasks

### Files to create

- `rust/src/planets/tests/calc_tests.rs` — Planetary analysis TDD tests
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P05`
  - marker: `@requirement REQ-PSS-ANALYSIS-001, REQ-PSS-ANALYSIS-002, REQ-PSS-ANALYSIS-003`
  - Test categories:
    1. **Fixture tests**: Capture C output for 10+ representative planet/star combinations, assert Rust matches
       - Sol system: Earth, Mars, Jupiter, Pluto (known planets with known properties)
       - Random generic systems: 3-5 different star types, various world types
       - Edge cases: hottest possible planet, coldest, gas giant, cratered world
    2. **Determinism tests**: Same seed always produces same output
    3. **Temperature-color tests**: Verify color mapping covers all temperature ranges
    4. **Greenhouse quirk test**: Verify the known temp/orbit-color mismatch is reproduced
    5. **Property tests** (proptest): For arbitrary seeds, analysis always produces values within valid ranges
       - Temperature: within plausible physical range
       - Radius: within MIN/MAX
       - Gravity: positive
       - Density: positive

### Files to modify

- `rust/src/planets/tests/mod.rs`
  - Add `mod calc_tests;`

## Pseudocode Traceability
- Tests target pseudocode lines: 001-044

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
# Tests should FAIL at this point (TDD: tests written before implementation)
cargo test -p uqm --lib planets::tests::calc_tests -- --nocapture 2>&1 | head -50
```

## Structural Verification Checklist
- [ ] `calc_tests.rs` created with comprehensive test cases
- [ ] Fixture data captured from C runtime for reference systems
- [ ] Tests reference specific requirement IDs in comments
- [ ] Tests call `do_planetary_analysis()` and `compute_temp_color()` from `calc.rs`

## Semantic Verification Checklist
- [ ] Tests cover all analysis output fields (not just a subset)
- [ ] Tests include the greenhouse quirk explicitly
- [ ] Property tests verify value ranges, not just specific fixtures
- [ ] Tests will fail against the current `todo!()` stubs (confirming TDD correctness)

## Success Criteria
- [ ] Test file compiles
- [ ] Tests are structured to verify real behavior once implemented
- [ ] Tests currently fail (expected — implementation is in P06)

## Failure Recovery
- rollback: `git checkout -- rust/src/planets/tests/calc_tests.rs`

## Phase Completion Marker
Create: `project-plans/20260311/planet-solarsys/.completed/P05.md`
