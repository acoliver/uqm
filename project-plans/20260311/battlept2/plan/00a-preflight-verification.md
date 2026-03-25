# Phase 0.5: Preflight Verification

## Phase ID
`PLAN-20260320-BATTLEPT2.P00.5`

## Purpose
Verify all assumptions before Phase 2/3 implementation begins. This phase ensures the Phase 1 foundation is intact, the toolchain is ready, and all planned integration points are reachable.

## Toolchain Verification
- [ ] `cargo --version` (1.75+ required)
- [ ] `rustc --version`
- [ ] `cargo clippy --version`
- [ ] `cargo fmt --version`

## Dependency Verification
- [ ] Required crates present in `Cargo.toml` (no new crates needed for Phase 2/3)
- [ ] `#[repr(C)]` types compile correctly
- [ ] FFI linkage to C battle code works (Phase 1 FFI adapters pass tests)

## Type/Interface Verification

### Phase 1 Modules Exist and Compile
- [ ] `cargo check -p uqm` succeeds
- [ ] All 15 Phase 1 battle modules exist in `rust/src/battle/`:
  - `battle_types.rs` (398 lines — coords, angles, trig, SINE_TABLE)
  - `element.rs` (943 lines — Element #[repr(C)], ElementFlags, lifecycle helpers)
  - `velocity.rs` (682 lines — VelocityDesc, Bresenham accumulation)
  - `display_list.rs` (899 lines — pool allocator, generational handles, linked-list ops)
  - `collision.rs` (558 lines — elastic_collide, isqrt, eligibility)
  - `weapon.rs` (949 lines — LaserBlock, MissileBlock, weapon_collision, blast creation)
  - `process_types.rs` (95 lines — ViewState, ZoomMode, zoom/camera constants)
  - `lifecycle.rs` (123 lines — BattleState, frame rate constants)
  - `ship_runtime_types.rs` (128 lines — ShipPipelineStage, spawn constants)
  - `tactical.rs` (187 lines — DeathPipelinePhase, explosion/flee/warp constants)
  - `ai_types.rs` (141 lines — EvaluateDesc, AI constants, control flags)
  - `netplay.rs` (586 lines — CRC-32, crc_process_element, protocol defs)
  - `integration.rs` (860 lines — 7 trait interfaces)
  - `ffi.rs` (510 lines — 17 Phase 1 FFI adapters)
  - `mod.rs` (532 lines — module declarations, re-exports, integration tests)

### Phase 1 Tests Pass
- [ ] All Phase 1 battle tests pass: `cargo test --lib battle::` (229 battle-specific tests, 2,151 total)
- [ ] Phase 1 FFI adapters verified: `ffi.rs` contains all 17 adapters listed in overview §Phase 1 FFI Adapters

### Phase 1 Integration Traits Exist
- [ ] `integration.rs` has 7 trait definitions:
  - `GraphicsIntegration` (17 conceptual operations)
  - `AudioIntegration` (11 operations)
  - `ThreadingIntegration` (3 operations)
  - `InputIntegration` (4 operations)
  - `ResourceIntegration` (5 operations)
  - `ShipRaceIntegration` (6 operations)
  - `GlobalStateIntegration` (4 operations)

### Rename-Target Modules Are Type-Only
- [ ] `process_types.rs` contains only type definitions (ViewState, ZoomMode, constants) — ready for rename to `process_loop.rs` in P03
- [ ] `ship_runtime_types.rs` contains only type definitions (ShipPipelineStage, spawn constants) — ready for rename to `ship_runtime.rs` in P07
- [ ] `ai_types.rs` contains only type definitions (EvaluateDesc, AI constants) — ready for rename to `ai.rs` in P11

## Test Infrastructure Verification
- [ ] Existing test harness for battle modules confirmed
- [ ] `#[cfg(test)]` module patterns are established in existing Phase 1 files
- [ ] Integration test patterns in `mod.rs` are usable for Phase 2/3 tests

## C Source Verification
- [ ] `sc2/src/uqm/process.c` — unmodified (no `USE_RUST_BATTLE_LOOP` guards yet)
- [ ] `sc2/src/uqm/battle.c` — unmodified (no `USE_RUST_BATTLE_LOOP` guards yet)
- [ ] `sc2/src/uqm/tactrans.c` — unmodified (no `USE_RUST_BATTLE_LOOP` guards yet)
- [ ] `sc2/src/uqm/intel.c` — unmodified (no `USE_RUST_BATTLE_LOOP` guards yet)
- [ ] `sc2/src/uqm/ship.c` — has `USE_RUST_SHIPS` guards only, no `USE_RUST_BATTLE_LOOP`
- [ ] `sc2/src/uqm/init.c` — has `USE_RUST_SHIPS` guards only, no `USE_RUST_BATTLE_LOOP`

## Integration Point Verification
- [ ] `DrawablesIntersect` exists and is callable from Rust via FFI
- [ ] Display primitive array globals (`DisplayArray`, `DisplayLinks`) are accessible
- [ ] `SetContext`/`BatchGraphics`/`UnbatchGraphics` are declared and callable
- [ ] `DoInput` framework functions exist
- [ ] `PlayerInput[]`/`CurrentInputToBattleInput` are accessible
- [ ] `race_q[]` (ship queues) accessible for iteration
- [ ] `GLOBAL()` macro equivalents are reachable for activity flags

## Blocking Issues
[List any blockers discovered during verification. If non-empty, stop and revise plan first.]

## Gate Decision
- [ ] **PASS:** All checks above pass — proceed to P01
- [ ] **FAIL:** One or more checks failed — revise plan before continuing

## Verification Commands

```bash
cargo --version
rustc --version
cargo clippy --version
cargo fmt --version
cargo check -p uqm
cargo test --lib battle:: --workspace --all-features
```
