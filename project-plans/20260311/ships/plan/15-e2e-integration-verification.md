# Phase 15: End-to-End Integration & Verification

## Phase ID
`PLAN-20260314-SHIPS.P15`

## Prerequisites
- Required: Phase 14a (Bridge Wiring Verification) completed and PASS
- Expected state: All 28 races implemented, all shared infrastructure complete, FFI wired, build toggle active, earlier mixed C/Rust smoke tests already passing

## Purpose
Final end-to-end integration testing across the complete ships subsystem. This phase produces no new code — it verifies the complete system works together.

## Early integration gates that must already exist before P15
The following are not deferred to this final phase and must have been added to earlier phases:
- P05/P05a: loader/resource bridge smoke tests through real C resource APIs
- P08/P08a: callback trampoline registration and minimal runtime-step smoke tests through real C-owned element state
- P09/P09a: spawn smoke tests against canonical `STARSHIP`/display-list integration
- P10/P10a: teardown/writeback integration tests with real queue/fragment structures
- P14/P14a: symbol/link validation for the Rust-enabled path

P15 assumes those gates already passed and focuses on full gameplay/system validation.

## Verification Scenarios

### Scenario 1: Master Catalog Load
- Start game with `USE_RUST_SHIPS=1`
- Master ship list loads during startup
- Verify: 25 melee-eligible ships in catalog
- Verify: sorted by race name
- Verify: costs match C reference values
- Verify: icons and melee icons load correctly

### Scenario 2: SuperMelee Ship Selection
- Enter SuperMelee setup
- Select ships from catalog into team roster
- Verify: all 25 melee ships are selectable
- Verify: ship costs display correctly
- Verify: team value calculates correctly

### Scenario 3: SuperMelee Battle — Simple Ships
- Start SuperMelee battle with Batch 1 ships
- Verify: ships spawn correctly
- Verify: weapon fire works (projectiles appear, do damage)
- Verify: special abilities work
- Verify: energy regeneration occurs
- Verify: movement is correct (thrust, turn, inertia, gravity-well influence where applicable)
- Verify: ship death → explosion → crew scatter → next ship

### Scenario 4: SuperMelee Battle — Mode-Switching Ships
- Battle with Androsynth: verify blazer mode toggle, characteristics change, return to normal
- Battle with Mmrnmhrm: verify X↔Y transform, different weapons/movement
- Battle with Pkunk: verify resurrection on death (probabilistic — test multiple times)
- Battle with VUX: verify warp-in positioning near enemy

### Scenario 5: SuperMelee Battle — Complex Ships
- Battle with Chmmr: verify ZapSat satellites orbit and fire
- Battle with Ur-Quan: verify fighters launch and attack independently
- Battle with Kohr-Ah: verify blade boomerang return, FRIED ring expansion
- Battle with Chenjesu: verify crystal shard fragmentation

### Scenario 6: Non-Melee Ships
- Verify SIS Ship does NOT appear in SuperMelee catalog
- Verify SIS Ship spawns correctly in campaign encounter context
- Verify Sa-Matra spawns correctly in final battle context
- Verify Probe spawns correctly in encounter context

### Scenario 7: Crew Writeback (Campaign Context)
- Start campaign encounter
- Ship takes damage (loses some crew)
- Win encounter
- Verify surviving crew count is written back to persistent state
- Start another encounter with same ship
- Verify crew count reflects prior damage

### Scenario 8: Ship Death & Replacement
- Start battle with multiple ships per side
- Kill first ship
- Verify: death animation, sound stops, replacement ship spawns
- Kill all ships on one side
- Verify: battle ends correctly

### Scenario 9: Battle Teardown
- Start and end multiple battles
- Verify: no resource leaks (memory, graphics handles, sound handles)
- Verify: clean state between battles
- Verify: master catalog still valid after battles

### Scenario 10: AI Combat
- CPU vs CPU SuperMelee
- Verify: AI makes reasonable combat decisions for all 25 melee ships
- Verify: no stuck/frozen AI states
- Verify: AI uses specials appropriately
- Verify: AI input timing remains compatible with the shared pipeline

### Scenario 11: Descriptor Mutation Preservation
- Androsynth blazer → normal cycle: characteristics restored correctly
- Mmrnmhrm transform cycle: characteristics swap and restore
- Vux limpet: target ship max_thrust actually reduced
- Druuge furnace: crew actually decremented, energy actually gained

### Scenario 12: Error Resilience
- Verify game doesn't crash if ship resources are missing (graceful degradation)
- Verify double-free doesn't occur during battle teardown
- Verify init/uninit round-trip is clean

## Verification Commands

```bash
# Full Rust verification
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Full build verification
# Build with USE_RUST_SHIPS=1
# Build with USE_RUST_SHIPS=0 (C fallback still works)

# No placeholder code anywhere
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|unimplemented\|todo!" rust/src/ships/
```

## Integration Contract

### Existing Callers (C side, now routed to Rust)
- `starcon.c` → `LoadMasterShipList()` → typed Rust catalog exports
- `battle.c` → `InitShips()` / `UninitShips()` → typed Rust lifecycle exports
- `ship.c` / battle transition code → `spawn_ship()` → typed Rust spawn export
- `battle.c` → element callbacks → Rust preprocess/postprocess/death trampolines
- queue/catalog/build callers → typed Rust helper exports matching established C signatures

### Existing Code Replaced
- `dummy.c` body: ship dispatch (replaced by `ships::registry`)
- `loadship.c` body: two-tier loading (replaced by `ships::loader`)
- `master.c` body: catalog management (replaced by `ships::catalog`)
- `build.c` body: queue operations (replaced by `ships::queue`)
- `ship.c` body: runtime pipeline (replaced by `ships::runtime`)
- `init.c` ship functions: lifecycle (replaced by `ships::lifecycle`)
- All 28 `ships/*/*.c` `init_*()` functions (replaced by race implementations)

### User Access Path
- SuperMelee: pick ships → start battle → ships fight using Rust implementations
- Campaign: encounter → ships fight using Rust implementations
- Hyperspace: SIS ship uses Rust implementation

### End-to-End Verification
- SuperMelee battle with all 25 melee ships
- Campaign encounter with SIS ship
- Final battle with Sa-Matra

## Structural Verification Checklist
- [ ] No `todo!()`, `unimplemented!()`, or inappropriate `unreachable!()` in ships module
- [ ] No `TODO`, `FIXME`, `HACK` comments in ships module
- [ ] All 28 race files present and non-empty
- [ ] All shared infrastructure files present
- [ ] FFI exports complete and typed
- [ ] C guards in place

## Semantic Verification Checklist (Mandatory)
- [ ] All 12 scenarios above pass
- [ ] Game is playable with USE_RUST_SHIPS=1
- [ ] Game is still playable with USE_RUST_SHIPS=0 (C fallback)
- [ ] No regressions in non-ships subsystems
- [ ] Earlier mixed-path integration gates were executed and documented before this phase

## Success Criteria
- [ ] All verification scenarios pass
- [ ] All verification commands pass
- [ ] Game is fully playable with Rust ships
- [ ] C fallback still works
- [ ] No resource leaks
- [ ] No behavioral regressions

## Phase Completion Marker
Create: `project-plans/20260311/ships/.completed/P15.md`

Contents:
- Plan ID: PLAN-20260314-SHIPS
- Final phase completion timestamp
- Total files created/modified
- Total tests added
- All verification outputs
- Final PASS decision
- Summary of complete ships subsystem port
