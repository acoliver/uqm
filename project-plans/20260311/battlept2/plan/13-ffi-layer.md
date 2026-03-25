# Phase 13: FFI Layer Phase 3 — Exports + C Shell Wiring + Netplay CRC

## Phase ID
`PLAN-20260320-BATTLEPT2.P13`

## Prerequisites
- Required: Phase 12a (Lifecycle Verification) completed with PASS
- Expected files: All Rust battle modules complete (process_loop.rs, ship_runtime.rs, tactical.rs, ai.rs, lifecycle.rs, c_bridge.rs)
- Expected artifacts: All 64 ported functions verified; dark-code guards in process.c from P06

## Requirements Implemented (Expanded)

### REQ: External symbol ABI preservation (battlept2/requirements.md §External symbol ABI preservation)
**Requirement text**: Every non-static battle function with external C callers shall preserve its original C symbol name in all build modes. In Rust-enabled builds, a C-linkage wrapper delegates to Rust.

Behavior contract:
- GIVEN: An external C caller (e.g., encount.c calling Battle())
- WHEN: `USE_RUST_BATTLE_LOOP` is enabled
- THEN: The call resolves to `rust_battle_wrappers.c` which delegates to Rust FFI export

### REQ: Build-mode coexistence (battlept2/requirements.md §Build-mode coexistence)
**Requirement text**: C-only baseline and Rust-enabled builds coexist. Guards control which bodies compile.

Behavior contract:
- GIVEN: `USE_RUST_BATTLE_LOOP` disabled
- WHEN: All C files compile
- THEN: Original C function bodies active; no Rust involvement

- GIVEN: `USE_RUST_BATTLE_LOOP` enabled
- WHEN: All C files compile
- THEN: Ported C function bodies guarded out; wrappers delegate to Rust

### REQ: DoBattle thin shell (battlept2/specification.md §4)
**Requirement text**: DoBattle remains a C function. In Rust-enabled builds, its body delegates to rust_battle_frame.

Behavior contract:
- GIVEN: `USE_RUST_BATTLE_LOOP` enabled
- WHEN: DoInput calls DoBattle
- THEN: DoBattle's thin-shell body calls rust_battle_frame() and returns its result

### REQ: Cross-language frame determinism (battlept2/requirements.md §Cross-language frame determinism)
**Requirement text**: Rust-owned logic produces bit-identical CRC-32 checksums as C reference for same inputs.

Behavior contract:
- GIVEN: A netplay-enabled build with identical input sequences
- WHEN: Rust path processes frames
- THEN: CRC-32 checksums match C reference exactly

### REQ: FFI boundary safety (battlept2/requirements.md §FFI boundary safety)
**Requirement text**: No Rust panic crosses FFI boundary. All C→Rust entry points have catch_unwind.

Behavior contract:
- GIVEN: Any C→Rust FFI export
- WHEN: Rust code panics
- THEN: Panic is caught; deterministic error/abort returned to C

## Implementation Tasks

### Files to create

- `sc2/src/uqm/rust_battle_wrappers.c` — C wrapper file
  - marker: `@plan PLAN-20260320-BATTLEPT2.P13`
  - marker: `@requirement REQ-SYMBOL-ABI, REQ-BUILD-COEXISTENCE`
  - Contents:
    - Compilation unit that preserves public C symbol names via wrappers
    - Only compiled when `USE_RUST_BATTLE_LOOP` is defined
    - Wrapper functions calling Rust FFI exports:
      - **battle.c symbols**: `Battle()` → `rust_battle_entry()`, `BattleSong()` → `rust_battle_song()`, `FreeBattleSong()` → `rust_free_battle_song()`, `GetPlayerOrder()` → `rust_get_player_order()`
      - **init.c symbols**: `InitShips()` / `UninitShips()` → `rust_init_ships()` / `rust_uninit_ships()`, `InitSpace()` / `UninitSpace()` → `rust_init_space()` / `rust_uninit_space()`
      - **intel.c symbols**: `computer_intelligence()` → `rust_computer_intelligence()`
    - Each wrapper: exact original signature → call Rust export → return result
    - **Symbols NOT wrapped** — these have external callers from non-battle code (solarsys, lander, hyper, ship race modules) and remain as C implementations even in Rust-enabled builds. Their Rust equivalents are internal module functions used only by the Rust battle loop:
      - **process.c shared utilities** (callers in solarsys.c, lander.c, scan.c, ipdisp.c, hyper.c, weapon.c, many ship race files): `AllocElement()`, `FreeElement()`, `SetUpElement()`, `Untarget()`, `RemoveElement()`, `RedrawQueue()`, `InitDisplayList()` — These C functions remain active in all build modes because non-battle code depends on them. Rust battle code calls equivalent internal functions that operate on the same shared display list.
      - **ship.c callback-referenced functions** (set as callbacks from hyper.c, tactrans.c, race modules): `animation_preprocess()`, `inertial_thrust()`, `ship_preprocess()`, `ship_postprocess()`, `ship_death()`, `collision()` — These are used as function-pointer values stored in element callback slots by both battle and non-battle code. They remain as C symbols. Under `USE_RUST_SHIPS`, ship_preprocess/ship_postprocess/spawn_ship already delegate to Rust. Under `USE_RUST_BATTLE_LOOP`, additional guards apply per ship.c section below.
      - **ship.c queue functions** (callers in battle.c, tactrans.c): `GetNextStarShip()`, `GetInitialStarShips()` — Called internally by battle code. Under `USE_RUST_BATTLE_LOOP`, the Rust battle entry replaces the call sites so these C functions are not reached. No wrapper needed; guarding call sites is sufficient.
      - **tactrans.c functions with race-module callers** (callers in pkunk.c, shofixti.c): `StopAllBattleMusic()`, `StartShipExplosion()`, `FindAliveStarShip()`, `SetWinnerStarShip()`, `GetWinnerStarShip()`, `RecordShipDeath()` — These are called from race-specific ship code (Pkunk reincarnation, Shofixti glory device) which runs under `USE_RUST_SHIPS` not `USE_RUST_BATTLE_LOOP`. They remain as active C functions. Rust equivalents are internal helpers called by the Rust death chain.
      - **tactrans.c internal battle functions** (callers only within battle loop): `OpponentAlive()` (ship.c), `StopDitty()` (battle.c), `ResetWinnerStarShip()` (battle.c), `new_ship()` (init.c callback comparison) — Under `USE_RUST_BATTLE_LOOP`, call sites are replaced by Rust. `new_ship` is referenced as a callback comparison value in UninitShips; when UninitShips is guarded, this reference moves to Rust. No wrapper needed.

### Files to modify

- `rust/src/battle/ffi.rs` — Add all Phase 3 FFI exports
  - marker: `@plan PLAN-20260320-BATTLEPT2.P13`
  - marker: `@requirement REQ-FFI-SAFETY, REQ-DETERMINISM, REQ-SYMBOL-ABI`
  - FFI exports to add (each with `#[no_mangle] pub extern "C"` + `catch_unwind`):
    - `rust_battle_entry()` → lifecycle::battle()
    - `rust_battle_frame()` → Per-frame logic (the core DoBattle replacement): SetMenuSounds → netplay checksum → ProcessInput → BatchGraphics → frame_cb → RedrawQueue(TRUE) → ScreenTransition → UnbatchGraphics → battle_speed timing
    - `rust_init_ships()` → lifecycle::init_ships()
    - `rust_uninit_ships()` → lifecycle::uninit_ships()
    - `rust_init_space()` → lifecycle::init_space()
    - `rust_uninit_space()` → lifecycle::uninit_space()
    - `rust_computer_intelligence()` → ai::computer_intelligence()
    - `rust_battle_song()` → lifecycle::battle_song()
    - `rust_free_battle_song()` → lifecycle::free_battle_song()
    - `rust_get_player_order()` → lifecycle::get_player_order()
  - All 17 Phase 1 FFI adapters remain unchanged

- `sc2/src/uqm/battle.c` — Add DoBattle thin-shell + C guards
  - marker: `@plan PLAN-20260320-BATTLEPT2.P13`
  - `#ifdef USE_RUST_BATTLE_LOOP` around DoBattle body → thin shell calling rust_battle_frame()
  - `#ifdef USE_RUST_BATTLE_LOOP` around Battle() body → guarded out (wrapper provides symbol)
  - extern declarations for Rust FFI exports

- `sc2/src/uqm/ship.c` — Add `USE_RUST_BATTLE_LOOP` guards
  - marker: `@plan PLAN-20260320-BATTLEPT2.P13`
  - Guard ported function bodies (ship_preprocess, ship_postprocess, collision, spawn_ship, GetNextStarShip, GetInitialStarShips)
  - Existing `USE_RUST_SHIPS` guards remain unchanged; `USE_RUST_BATTLE_LOOP` is a separate guard

- `sc2/src/uqm/tactrans.c` — Add `USE_RUST_BATTLE_LOOP` guards
  - marker: `@plan PLAN-20260320-BATTLEPT2.P13`
  - Guard ported function bodies (death, explosion, flee, winner functions)
  - Retained boundary functions (battleEndReadyHuman/Computer/Network, netplay callbacks) NOT guarded

- `sc2/src/uqm/intel.c` — Add `USE_RUST_BATTLE_LOOP` guard
  - marker: `@plan PLAN-20260320-BATTLEPT2.P13`
  - Guard computer_intelligence body

- `sc2/src/uqm/init.c` — Add `USE_RUST_BATTLE_LOOP` guards
  - marker: `@plan PLAN-20260320-BATTLEPT2.P13`
  - Guard InitShips, UninitShips, InitSpace, UninitSpace bodies
  - Note: CountCrewElements is `static` (init.c:253) — no external symbol, no guard needed. It is ported as an internal Rust helper (`count_crew_elements()` in lifecycle.rs), not as an FFI boundary function.
  - Retained boundary functions (load_animation, free_image, BuildSIS) NOT guarded

- `sc2/Makefile` (or build config) — Add rust_battle_wrappers.c to C build
  - marker: `@plan PLAN-20260320-BATTLEPT2.P13`

### C reference functions ported
P13 ports no new functions. It creates the FFI export/wrapper/guard layer.

### C branches to handle

| Branch | Source Sites | Handling |
|--------|-------------|----------|
| `USE_RUST_BATTLE_LOOP` | All 5 C files + wrappers | Master toggle: disabled = C-only, enabled = Rust |
| `NETPLAY_CHECKSUM` | rust_battle_frame | CRC computation on Rust side must match C |
| Max-speed rendering skip | rust_battle_frame | Inherit from DoBattle: simulation runs, render skips |

### Integration points
- All Rust battle modules: lifecycle.rs, process_loop.rs, ship_runtime.rs, tactical.rs, ai.rs, c_bridge.rs
- All C source files: process.c, battle.c, ship.c, tactrans.c, intel.c, init.c
- Build system: Makefile/CMake additions for wrappers
- Phase 1 `ffi.rs`: 17 adapters unchanged, new exports added alongside
- Phase 1 `netplay.rs`: CRC functions used by rust_battle_frame

### Pseudocode traceability (if impl phase)
- N/A (wiring/infrastructure phase)

## Verification Commands

```bash
# Rust side
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# C side — both modes must compile
# C-only mode (USE_RUST_BATTLE_LOOP not defined):
# make clean && make  (or equivalent)
# Rust-enabled mode (USE_RUST_BATTLE_LOOP defined):
# make clean && CFLAGS=-DUSE_RUST_BATTLE_LOOP make  (or equivalent)
```

## Structural Verification Checklist
- [ ] `rust_battle_wrappers.c` created with all wrapper functions
- [ ] `ffi.rs` has all Phase 3 exports (rust_battle_entry, rust_battle_frame, etc.)
- [ ] All 5 C files have `USE_RUST_BATTLE_LOOP` guards on ported functions
- [ ] Retained boundary functions NOT guarded (11 functions)
- [ ] DoBattle thin-shell body in battle.c
- [ ] Build system updated for wrappers
- [ ] All 17 Phase 1 FFI adapters unchanged

## Semantic Verification Checklist (Mandatory)
- [ ] C-only build (no USE_RUST_BATTLE_LOOP): compiles and original behavior unchanged
- [ ] Rust-enabled build (USE_RUST_BATTLE_LOOP defined): compiles with wrappers
- [ ] DoBattle thin shell: calls rust_battle_frame(), returns result to DoInput
- [ ] rust_battle_frame: reproduces DoBattle's per-frame sequence exactly
- [ ] All FFI exports have catch_unwind
- [ ] All wrapper functions have correct signatures matching originals
- [ ] CRC-32 computation: identical checksums in Rust path vs C path for same frame
- [ ] Symbol-provider matrix (spec §5.2) satisfied: every external symbol has provider
- [ ] Retained boundary functions accessible in both build modes
- [ ] No placeholder/deferred implementation patterns

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/ffi.rs sc2/src/uqm/rust_battle_wrappers.c
```

## Success Criteria
- [ ] Both build modes compile
- [ ] DoBattle thin shell works
- [ ] All FFI exports safe (catch_unwind)
- [ ] CRC-32 determinism verified
- [ ] Symbol-provider matrix complete
- [ ] All tests pass

## Failure Recovery
- rollback: `git checkout -- rust/src/battle/ffi.rs sc2/src/uqm/battle.c sc2/src/uqm/ship.c sc2/src/uqm/tactrans.c sc2/src/uqm/intel.c sc2/src/uqm/init.c sc2/src/uqm/process.c` + remove `rust_battle_wrappers.c`
- blocking issues: Linker symbol conflicts, CRC mismatch, build system integration

## Phase Completion Marker
Create: `project-plans/20260311/battlept2/.completed/P13.md`

Contents:
- phase ID: PLAN-20260320-BATTLEPT2.P13
- timestamp
- files created: rust_battle_wrappers.c
- files changed: ffi.rs, battle.c, ship.c, tactrans.c, intel.c, init.c, process.c, build config
- tests added/updated
- verification outputs (both build modes)
- CRC-32 determinism verification results
- semantic verification summary
