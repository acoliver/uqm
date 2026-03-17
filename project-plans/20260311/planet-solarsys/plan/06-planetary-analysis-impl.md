# Phase 06: Planetary Analysis (Implementation)

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P06`

## Prerequisites
- Required: Phase 05a (Analysis TDD Verification) completed
- Expected: calc_tests.rs compiling with failing tests

## Requirements Implemented (Expanded)

### REQ-PSS-ANALYSIS-001 through REQ-PSS-ANALYSIS-004
(See Phase 05 for full requirement text)

Implementation satisfies these by making all TDD tests pass.

## Implementation Tasks

### Files to modify

- `rust/src/planets/calc.rs` — Full planetary analysis implementation
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P06`
  - marker: `@requirement REQ-PSS-ANALYSIS-001, REQ-PSS-ANALYSIS-002, REQ-PSS-ANALYSIS-003, REQ-PSS-ANALYSIS-004`
  - Remove all `todo!()` stubs
  - Implement `do_planetary_analysis(sys_info: &mut SystemInfo, planet: &PlanetDesc, star: &StarDesc, rng: &mut SysGenRng)`:
    - Derive star energy from SUN_DATA lookup
    - Compute orbital distance
    - Compute base temperature from star energy and distance
    - Look up PlanData for planet's data_index
    - Derive density, radius, rotation_period using PlanData ranges and RNG
    - Compute gravity from density and radius
    - Derive axial tilt from RNG
    - Derive atmospheric density from PlanData and RNG
    - Compute greenhouse adjustment (if atmosphere present)
    - Compute surface temperature (base + greenhouse)
    - Derive tectonics from PlanData, density, RNG
    - Derive weather from atmosphere, temperature, RNG
    - Derive life chance from PlanData, temperature, atmosphere, RNG
    - Store all values in sys_info.planet_info
  - Implement `compute_temp_color(temperature: i16) -> Color`:
    - Map temperature ranges to colors matching C implementation exactly
    - Preserve the greenhouse quirk: use pre-greenhouse temperature for orbit-color
  - Uses pseudocode lines: 001-044

- `rust/src/planets/constants.rs` — Add PlanData and SunData arrays
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P06`
  - Add `PlanDataEntry` struct matching C `PlanData` fields (min/max ranges for density, radius, etc.)
  - Add `PLAN_DATA: [PlanDataEntry; N]` const array with values from `sc2/src/uqm/planets/plandata.h`
  - Add `SunDataEntry` struct matching C sun data
  - Add `SUN_DATA: [SunDataEntry; N]` const array with values from `sc2/src/uqm/planets/sundata.h`

### C files being replaced
- `sc2/src/uqm/planets/calc.c` — `DoPlanetaryAnalysis()` function (~530 lines)
  - Not yet guarded (C guards added in P12)

## Pseudocode Traceability
- Implements pseudocode lines: 001-044

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
# Focused verification:
cargo test -p uqm --lib planets::tests::calc_tests --all-features -- --nocapture
```

## Structural Verification Checklist
- [ ] `calc.rs` has no `todo!()` or `unimplemented!()`
- [ ] `constants.rs` contains PLAN_DATA and SUN_DATA arrays
- [ ] Plan/requirement traceability markers present
- [ ] All calc_tests pass

## Semantic Verification Checklist
- [ ] All fixture tests pass (Rust output matches C reference for every field)
- [ ] Temperature-color mapping matches C for all temperature ranges
- [ ] Greenhouse quirk is preserved (orbit color uses pre-greenhouse temp)
- [ ] Property tests confirm valid output ranges
- [ ] Determinism: same seed always produces same output

## Deferred Implementation Detection

```bash
grep -RIn "todo!()\|unimplemented!()\|FIXME\|HACK\|placeholder" rust/src/planets/calc.rs rust/src/planets/constants.rs
# Should return 0 matches
```

## Success Criteria
- [ ] All calc_tests pass (100%)
- [ ] Fixture outputs match C reference values exactly
- [ ] No placeholder code remains
- [ ] Verification commands all pass

## Failure Recovery
- rollback: `git checkout -- rust/src/planets/calc.rs rust/src/planets/constants.rs`

## Phase Completion Marker
Create: `project-plans/20260311/planet-solarsys/.completed/P06.md`
