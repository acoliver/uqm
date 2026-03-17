# Phase 09: Scan Flow & Node Materialization

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P09`

## Prerequisites
- Required: Phase 08a (Surface Gen Verification) completed
- Expected: Surface generation functional, rendering helpers implemented
- Required: State subsystem `PlanetInfoManager` accessible for scan mask operations
- Required: generation-handler signature inventory completed so data-provider node semantics are confirmed before implementation

## Requirements Implemented (Expanded)

### REQ-PSS-SCAN-001: Scan entry
**Requirement text**: Entering scan mode shall prepare scan context, determine initial scan-type based on world properties (shielded/gas giant restrictions), initialize planet location display, draw scanned objects, display coarse-scan information, and enter the scan input loop.

Behavior contract:
- GIVEN: Player is in orbit around a scannable planet
- WHEN: Scan action is selected from orbital menu
- THEN: Scan context is initialized, restrictions applied, display prepared, input loop entered

### REQ-PSS-SCAN-002: Scan types
**Requirement text**: Three scan types: mineral (0), energy (1), biological (2).

### REQ-PSS-SCAN-003: Gas giant scan restrictions
**Requirement text**: Gas-giant worlds restrict available scan types appropriately.

### REQ-PSS-SCAN-004: Shielded world restrictions
**Requirement text**: Shielded worlds skip node generation entirely.

### REQ-PSS-NODES-001: Surface-node population
**Requirement text**: When populating nodes, the subsystem shall: initialize display list, skip shielded worlds, iterate scan types (bio, energy, mineral order), query generation functions for counts and per-node info, filter retrieved nodes, allocate display elements.

Behavior contract:
- GIVEN: A world with 5 mineral nodes, 2 already retrieved (bits set in scan mask)
- WHEN: `generate_planet_side()` is called
- THEN: 3 mineral nodes are materialized (the 2 retrieved are skipped)

### REQ-PSS-NODES-002: Node-retrieval filtering
**Requirement text**: Each node shall be tested against the appropriate scan-retrieval mask bit. Nodes whose bits are set shall be skipped.

### REQ-PSS-NODES-003: Mineral node attributes
**Requirement text**: Each mineral node carries element type, gross deposit size (image), fine deposit size (quantity), and mineral-category frame indexing.

### REQ-PSS-NODES-004: Energy/biological node attributes
**Requirement text**: Energy/bio nodes carry animated scan-dot frames, bio nodes carry creature type and variation.

### REQ-PSS-SCAN-005: Scan-triggered encounters
**Requirement text**: When a scan triggers an encounter, set the encounter flag, save solar-system location, and exit scan mode.

### REQ-PSS-PERSIST-001: Persistence addressing parity during node materialization
**Requirement text**: Scan mask reads used for node suppression shall preserve baseline world addressing semantics for planets and moons.

### REQ-PSS-PERSIST-002: Persistence access limited to legal host window
**Requirement text**: Scan-side persistence access shall occur only while the host-guaranteed persistence window is active.

## Implementation Tasks

### Files to modify

- `rust/src/planets/scan.rs` — Full scan implementation
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P09`
  - marker: `@requirement REQ-PSS-SCAN-001 through REQ-PSS-SCAN-005, REQ-PSS-NODES-001 through REQ-PSS-NODES-004, REQ-PSS-PERSIST-001 through REQ-PSS-PERSIST-002`
  - Remove all `todo!()` stubs
  - Implement `scan_system(state: &mut SolarSysState, world: &WorldRef) -> ScanOutcome`:
    - Prepare scan context for current orbital target
    - Determine restrictions: `ScanRestriction::GasGiant` or `ScanRestriction::Shielded` or `ScanRestriction::None`
    - Initialize planet location display (coordinate mapping)
    - Draw any previously scanned objects
    - Print coarse-scan report (delegates to `print_coarse_scan()`)
    - Run scan input loop (`do_scan()`)
    - Clean up scan display
    - Return `ScanOutcome`
  - Implement `generate_planet_side(state: &mut SolarSysState, world: &WorldRef)`:
    - Initialize display list
    - Early return for shielded worlds
    - Iterate: BIOLOGICAL, ENERGY, MINERAL (this order matches C)
    - For each scan type:
      - Query the audited data-provider count API to get node count
      - For each node index:
        - Check `is_node_retrieved(scan_masks, scan_type, node_index)`
        - If not retrieved: query the audited per-node provider API, create display element
    - Populate mineral elements with: element_type, gross_size (image), fine_size (qty)
    - Populate bio elements with: creature_type, variation (capped by life_variation)
    - Populate energy elements with: location, animated frames
    - Treat the loaded scan masks as baseline-addressed world state; do not introduce alternate addressing logic in scan materialization
  - Implement `is_node_retrieved(masks: &[u32; 3], scan_type: ScanType, index: usize) -> bool`
  - Implement `draw_scanned_objects(reversed: bool)`
  - Implement `print_coarse_scan(planet_info: &PlanetInfo)` (the report display)
  - Implement scan encounter handling:
    - If encounter trigger (e.g., Fwiffo at Pluto), set `START_ENCOUNTER`
    - Call `save_solar_sys_location()`
    - Return `ScanOutcome::Encounter`
  - Uses pseudocode lines: 090-124
  - Integration: Uses `PlanetInfoManager` from `rust/src/state/planet_info.rs` for scan masks
  - Integration: Uses drawing primitives from `rust/src/graphics/` for scan display
  - Integration: Uses generation dispatch wrappers for data-provider node queries

### Files to create

- `rust/src/planets/tests/scan_tests.rs` — Scan and node materialization tests
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P09`
  - Test `is_node_retrieved` with various mask/index combinations
  - Test `generate_planet_side` with mock generation providers:
    - Verify correct node count
    - Verify retrieved nodes are filtered
    - Verify iteration order (bio, energy, mineral)
    - Verify mineral node attribute population
    - Verify bio node variation capping
  - Test shielded world produces 0 nodes
  - Test gas giant scan restrictions
  - Test scan encounter trigger path
  - Fixture tests: capture C node populations for reference worlds including at least one shielded world and one encounter-capable world

- `rust/src/planets/tests/persistence_tests.rs` — Scan persistence round-trip tests
  - marker: `@plan PLAN-20260314-PLANET-SOLARSYS.P09`
  - marker: `@requirement REQ-PSS-NODES-002, REQ-PSS-PERSIST-001`
  - Test: put scan mask with retrieved bits, then get, then materialize — retrieved nodes absent
  - Test: put mask for planet vs. moon — correct records addressed
  - Test: round-trip through `PlanetInfoManager` matches expected behavior
  - Test: node filtering parity for multiple moon-count layouts
  - Test: scan-side reads consume the same world-addressing identity later used by save/write paths

### Files to modify

- `rust/src/planets/tests/mod.rs`
  - Add `mod scan_tests;` and `mod persistence_tests;`

### C files being replaced
- `sc2/src/uqm/planets/scan.c` — scan flow and node materialization (~1345 lines)
- `sc2/src/uqm/planets/report.c` — coarse-scan report display (~150 lines)
- `sc2/src/uqm/planets/lander.c` — pickup hook dispatch only (lander gameplay stays C)

## Pseudocode Traceability
- Implements pseudocode lines: 090-124

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p uqm --lib planets::tests::scan_tests --all-features -- --nocapture
cargo test -p uqm --lib planets::tests::persistence_tests --all-features -- --nocapture
```

## Structural Verification Checklist
- [ ] `scan.rs` fully implemented, no `todo!()`
- [ ] Test files created: `scan_tests.rs`, `persistence_tests.rs`
- [ ] Plan/requirement traceability present

## Semantic Verification Checklist
- [ ] Node retrieval filtering works correctly (bit-level check)
- [ ] Shielded worlds produce 0 nodes
- [ ] Gas giant restrictions are enforced
- [ ] Iteration order is BIOLOGICAL, ENERGY, MINERAL
- [ ] Mineral nodes carry element_type, gross_size, fine_size
- [ ] Bio nodes carry creature_type, variation (capped)
- [ ] Encounter trigger path saves location and returns correct outcome
- [ ] Scan mask round-trip through `PlanetInfoManager` is correct
- [ ] Data-provider handlers are consumed according to audited count/per-node semantics, not handled/not-handled semantics
- [ ] Persistence-addressing behavior is covered across differing moon-count layouts
- [ ] Scan-side persistence access remains inside the host-guaranteed legal window

## Deferred Implementation Detection

```bash
grep -RIn "todo!()\|unimplemented!()\|FIXME\|HACK" rust/src/planets/scan.rs
# Must return 0
```

## Success Criteria
- [ ] All `scan_tests` pass
- [ ] All `persistence_tests` pass
- [ ] Node populations match C reference for fixture worlds
- [ ] No placeholder code remains

## Failure Recovery
- rollback: `git checkout -- rust/src/planets/scan.rs rust/src/planets/tests/`

## Phase Completion Marker
Create: `project-plans/20260311/planet-solarsys/.completed/P09.md`
