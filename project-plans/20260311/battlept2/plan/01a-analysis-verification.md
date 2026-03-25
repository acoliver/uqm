# Phase 01a: Analysis Verification

## Phase ID
`PLAN-20260320-BATTLEPT2.P01a`

## Prerequisites
- Required: Phase 01 (Analysis) completed
- Expected artifact: `project-plans/20260311/battlept2/analysis/domain-model.md`

## Structural Verification Checklist
- [ ] `analysis/domain-model.md` exists and is non-empty
- [ ] Contains all 8 required sections (dependency graph, function mapping, integration touchpoints, state management, callback analysis, display primitive coupling, branch-parity inventory, FFI safety matrix)
- [ ] Function mapping table has exactly 75 rows
- [ ] Integration touchpoint table has exactly 44 deferred operations + 6 Phase 1 operations = 50 total
- [ ] Branch-parity table has all 7 families from spec §13.1

## Semantic Verification Checklist (Mandatory — Most Important)
- [ ] **All 64 ported functions** have a Rust target module and implementation phase assigned
- [ ] **All 11 retained functions** match spec §3.1 exactly: `frameInputHuman`, `DoBattle`, `battleEndReadyHuman/Computer/Network`, `readyToEnd2Callback`, `readyToEndCallback`, `readyForBattleEndPlayer`, `load_animation`, `free_image`, `BuildSIS`
- [ ] **No C function in scope is missing** — cross-check against process.c, battle.c, tactrans.c, intel.c, ship.c, init.c
- [ ] **Integration touchpoints trace to actual trait methods** in `integration.rs`
- [ ] **Callback-slot analysis** covers all 4 element callback families (preprocess, postprocess, collision, death) + 2 handler/vtable families (frameInput, battleEndReady)
- [ ] **Display primitive coupling** documents that Rust process loop reads/writes C-owned `DisplayArray[]` and `DisplayLinks[]` via FFI during PostProcessQueue
- [ ] **Branch-parity inventory** lists specific C source lines for each branch family:
  - `NETPLAY/NETPLAY_CHECKSUM` sites in battle.c, tactrans.c, process.c
  - `DEMO_MODE/CREATE_JOURNAL` sites in battle.c, process.c
  - `SUPER_MELEE` sites in battle.c, tactrans.c
  - `CHECK_ABORT/CHECK_LOAD` sites in battle.c, init.c
  - `IN_ENCOUNTER/IN_LAST_BATTLE` sites in init.c, tactrans.c, battle.c
  - `inHyperSpace()/inQuasiSpace()` sites in init.c, battle.c
  - Max-speed rendering skip sites in battle.c, process.c
- [ ] **FFI safety matrix** covers: pointer-family categories (spec §10.3), panic containment (spec §10.1), thread affinity (spec §10.4), stable identity model (spec §10.5)
- [ ] **DoBattle thin-shell contract** from spec §4 is explicitly documented in the state management or callback analysis

## Branch-Parity Verification
This phase touches all branch families at the analysis level. The inventory must identify which phases implement each branch:

| Branch Family | Source Sites | Implementing Phases |
|--------------|-------------|-------------------|
| NETPLAY / NETPLAY_CHECKSUM | battle.c, tactrans.c, process.c | P05, P09, P10, P12, P13 |
| DEMO_MODE / CREATE_JOURNAL | battle.c, process.c | P05, P12 |
| SUPER_MELEE | battle.c, tactrans.c | P09, P12 |
| CHECK_ABORT / CHECK_LOAD | battle.c, init.c | P12 |
| IN_ENCOUNTER / IN_LAST_BATTLE | init.c, tactrans.c, battle.c | P08, P09, P10, P12 |
| inHyperSpace() / inQuasiSpace() | init.c, battle.c | P08, P12 |
| Max-speed rendering skip | battle.c, process.c | P05, P12 |

## Verification Commands

```bash
# Phase 1 tests still pass
cargo test --workspace --all-features
```

## Pass/Fail Gate Criteria
- **PASS:** All structural and semantic checklist items verified. All 75 functions accounted for. All 44 bridge operations mapped. All 7 branch families inventoried with source sites.
- **FAIL:** Any function missing from inventory, any bridge operation unaccounted, any branch family missing source sites, or DoBattle thin-shell contract not documented.
