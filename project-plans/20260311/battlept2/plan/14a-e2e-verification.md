# Phase 14a: E2E Verification — Final Gate

## Phase ID
`PLAN-20260320-BATTLEPT2.P14a`

## Prerequisites
- Required: Phase 14 (E2E Integration) completed
- Expected artifacts: All integration tests passing, all modules complete

## Structural Verification Checklist
- [ ] All 15 Rust battle modules exist and compile: `cargo check -p uqm`
- [ ] All C files have correct guards: process.c, battle.c, ship.c, tactrans.c, intel.c, init.c
- [ ] `rust_battle_wrappers.c` exists with all wrapper functions
- [ ] Build system includes wrappers
- [ ] All `.completed/` markers from P03–P14 exist

## Semantic Verification Checklist (Mandatory — Most Important)

### Module-level behavioral completeness

#### process_loop.rs (P03-P05)
- [ ] 16 functions: pre_process, post_process, alloc_element, free_element, setup_element, untarget, remove_element, process_collisions, redraw_queue, pre_process_queue, post_process_queue, calc_reduction, calc_view, init_display_list, insert_prim, calc_display_coord (+init_kernel)
- [ ] Flag transitions exactly match C
- [ ] Collision recursion structure matches C
- [ ] Zoom hysteresis matches C

#### c_bridge.rs (P06)
- [ ] 44 bridge wrappers present and correct
- [ ] All pointer args have appropriate safety checks
- [ ] Thread affinity documented

#### ship_runtime.rs (P07-P08)
- [ ] 8 functions: ship_preprocess, ship_postprocess, inertial_thrust, animation_preprocess, ship_collision, spawn_ship, get_next_starship, get_initial_starships
- [ ] 7-stage pipeline order matches C
- [ ] Inertial physics matches C for all 4 modes

#### tactical.rs (P09-P10)
- [ ] 25 functions (17 death/explosion + 8 flee/warp/winner)
- [ ] 4-phase death chain works end-to-end
- [ ] Simultaneous death handling correct
- [ ] Flee 20-color pulse correct
- [ ] Winner first-wins preservation correct

#### ai.rs (P11)
- [ ] computer_intelligence with all 4 dispatch paths
- [ ] Output feeds correctly into ship_preprocess

#### lifecycle.rs (P12)
- [ ] 13 functions: battle, init_ships, uninit_ships, init_space, uninit_space, process_input, count_crew_elements, run_away_allowed, setup_battle_input_order, battle_song, free_battle_song, select_all_ships, get_player_order
- [ ] Reference counting correct
- [ ] All cleanup paths release resources

#### ffi.rs (P06+P13)
- [ ] 17 Phase 1 exports unchanged
- [ ] All Phase 3 exports present with catch_unwind
- [ ] rust_battle_frame reproduces DoBattle per-frame sequence

### Cross-cutting verification

#### Branch-parity completeness (all 7 families)
- [ ] `NETPLAY` / `NETPLAY_CHECKSUM`: CRC computation, frame sync, battle-end readiness
- [ ] `DEMO_MODE` / `CREATE_JOURNAL`: deterministic RNG seed, readyForBattleEnd=true
- [ ] `SUPER_MELEE`: infinite fleet, flee always allowed, ship selection UI
- [ ] `CHECK_ABORT` / `CHECK_LOAD`: Battle() exit paths
- [ ] `IN_ENCOUNTER` / `IN_LAST_BATTLE`: flee rules, init differences
- [ ] `inHyperSpace()` / `inQuasiSpace()`: music, single-ship spawn
- [ ] Max-speed rendering skip: simulation always, rendering conditional

#### Frame determinism
- [ ] Element processing order identical to C for same state
- [ ] Integer arithmetic only (no float)
- [ ] RNG call order identical to C
- [ ] CRC-32 bit-identical for same inputs

#### Callback-slot integrity
- [ ] All 4 element callback families (preprocess, postprocess, collision, death) use extern "C" targets
- [ ] No Rust closures or trait objects stored in C callback fields
- [ ] Stale callback dispatch prevented (null check, validity check)

#### Symbol-provider completeness (spec §5.2)
- [ ] Every non-static battle function with external callers has a named provider
- [ ] No missing symbols in either build mode
- [ ] No duplicate symbols causing linker errors

### Regression gate
- [ ] All Phase 1 tests pass (229 battle-specific, ~2,151 total)
- [ ] All Phase 2/3 unit tests pass
- [ ] All Phase 2/3 integration tests pass
- [ ] Zero test regressions

### Definition of Done (from overview lines 719-734)
- [ ] 1. All 64 ported functions have Rust implementations
- [ ] 2. All 11 retained C boundaries documented and accessible
- [ ] 3. USE_RUST_BATTLE_LOOP toggle works (both modes compile)
- [ ] 4. DoBattle thin shell delegates to rust_battle_frame
- [ ] 5. All Phase 1 tests pass
- [ ] 6. All Phase 2/3 tests pass
- [ ] 7. CRC-32 determinism verified
- [ ] 8. No TODO/FIXME/HACK in battle modules
- [ ] 9. cargo fmt/clippy/test all pass
- [ ] 10. Branch-parity for all 7 families verified
- [ ] 11. Symbol-provider matrix complete
- [ ] 12. Callback-slot migration matrix complete
- [ ] 13. Reference counting correct (no leaks)
- [ ] 14. All phase completion markers exist (P03-P14)

## Branch-Parity Verification
All 7 branch families verified above.

## Verification Commands

```bash
# Full Rust verification
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Deferred implementation detection across ALL battle files
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/

# C verification (both modes)
# C-only: make clean && make
# Rust-enabled: make clean && CFLAGS=-DUSE_RUST_BATTLE_LOOP make

# Phase completion marker check
ls project-plans/20260311/battlept2/.completed/
```

## Pass/Fail Gate Criteria
- **PASS (ALL must be true):**
  - All 14 Definition of Done items verified
  - All 7 branch families verified
  - Frame determinism (CRC-32) verified
  - Zero test regressions
  - Both build modes compile and link
  - No TODO/FIXME/HACK in any battle module
  - All phase completion markers (P03-P14) exist

- **FAIL (ANY one is sufficient):**
  - Any Definition of Done item not met
  - Any branch family not verified
  - CRC mismatch
  - Any test regression
  - Either build mode fails
  - TODO/FIXME/HACK found
  - Missing phase completion marker
