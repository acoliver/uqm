# Phase 0.5: Preflight Verification

## Phase ID
`PLAN-20260314-CAMPAIGN.P00.5`

## Purpose
Verify all assumptions about the codebase, toolchain, and dependency availability before any implementation work begins.

## Toolchain Verification

- [ ] `cargo --version` — confirm Rust toolchain available
- [ ] `rustc --version` — confirm compiler version
- [ ] `cargo clippy --version` — confirm linter available
- [ ] `cargo llvm-cov --version` — confirm coverage tool if coverage gate required

## Dependency Verification

### Rust Crate Dependencies
- [ ] `serde` and `serde_json` available in `Cargo.toml` for Campaign Canonical Export Document (JSON serialization)
- [ ] If not present, add `serde = { version = "1", features = ["derive"] }` and `serde_json = "1"` to `rust/Cargo.toml`
- [ ] Verify `rust/Cargo.toml` feature flags support conditional campaign compilation

### Existing Rust Module Dependencies
- [ ] `rust/src/time/game_clock.rs` — confirm `GameClock` struct and FFI exports exist and match expected signatures
- [ ] `rust/src/time/events.rs` — confirm event scheduling types exist
- [ ] `rust/src/time/game_date.rs` — confirm `GameDate` type exists
- [ ] `rust/src/state/game_state.rs` — confirm game-state bit access functions exist
- [ ] `rust/src/state/state_file.rs` — confirm state-file I/O functions exist
- [ ] `rust/src/state/ffi.rs` — confirm FFI exports for `rust_get_game_state_bits_from_bytes`, `rust_set_game_state_bits_in_bytes`, `rust_open_state_file`, etc.
- [ ] `rust/src/game_init/init.rs` — confirm initialization helpers exist
- [ ] `rust/src/io/` — confirm file I/O primitives available

## Type/Interface Verification

### C Types Referenced by Campaign
- [ ] `GAME_STATE` struct layout in `globdata.h` — confirm field offsets for `CurrentActivity`, `GameClock`, `GameState` bitfield
- [ ] `CurrentActivity`, `NextActivity`, `LastActivity` globals accessible
- [ ] Activity flags: `IN_LAST_BATTLE`, `IN_ENCOUNTER`, `IN_HYPERSPACE`, `IN_INTERPLANETARY`, `START_ENCOUNTER`, `START_INTERPLANETARY`, `CHECK_LOAD`, `CHECK_RESTART`, `CHECK_ABORT` — confirm values
- [ ] `GLOBAL_FLAGS_AND_DATA` usage for starbase-context marker
- [ ] `npc_built_ship_q`, `escort_q`, `race_q` queue types and access patterns
- [ ] `ENCOUNTER_Q` / `IP_GROUP_Q` / `SHIP_FRAGMENT` types for encounter/group data
- [ ] Event handler function signatures in `gameev.c` — confirm `EventHandler()` switch structure
- [ ] Save/load function signatures: `SaveGame()`, `LoadGame()`, `SaveGameState()`, `LoadGameState()`, `PrepareSummary()`

### Rust Types That Must Exist
- [ ] `rust/src/time/game_clock.rs` exports: `InitGameClock`, `UninitGameClock`, `SetGameClockRate`, `GameClockTick`, `MoveGameClockDays`, `LockGameClock`, `UnlockGameClock`, `GameClockRunning`
- [ ] `rust/src/time/events.rs` exports: event scheduling interface (add_event, remove_event, etc.)
- [ ] `rust/src/state/game_state.rs` exports: get/set game state bits interface

## Call-Path Feasibility

### Campaign Loop Integration Path
- [ ] `starcon.c:FreeKernel()` → teardown path exists and can be called from Rust
- [ ] `starcon.c:InitKernel()` → initialization path accessible
- [ ] `starcon.c` main loop → can be replaced by Rust FFI call when `USE_RUST_CAMPAIGN` defined
- [ ] `restart.c:TryStartGame()` → entry point accessible for replacement

### Clock Integration Path
- [ ] Campaign code can call `InitGameClock()` / `UninitGameClock()` through existing Rust clock
- [ ] Campaign code can call `SetGameClockRate()` with hyperspace/interplanetary rates
- [ ] Campaign code can call `MoveGameClockDays()` for starbase time skips
- [ ] Event registration path accessible: campaign can schedule events with the Rust clock

### State Integration Path
- [ ] Campaign code can call `GET_GAME_STATE()` / `SET_GAME_STATE()` through existing Rust state
- [ ] State-file helpers accessible: `OpenStateFile`, `ReadStateFile`, `WriteStateFile`

### Save/Load I/O Path
- [ ] File I/O for save files accessible through `rust/src/io/`
- [ ] Save slot management interface available or needs to be created

## Build System Verification

- [ ] `build.config` pattern for `USE_RUST_*` toggles understood — campaign toggle will follow same pattern
- [ ] `config_unix.h` template includes slot for `USE_RUST_CAMPAIGN`
- [ ] Makefile system can conditionally compile `starcon_rust.c` vs `starcon.c` (same pattern as `clock_rust.c` vs `clock.c`)
- [ ] Rust library linkage confirmed working for existing toggles

## Test Infrastructure Verification

- [ ] `cargo test --workspace --all-features` passes currently with no failures
- [ ] Test files can be created under `rust/src/campaign/` directory
- [ ] Integration test infrastructure exists (if any e2e test harness)
- [ ] C test compilation path available if C-side bridge tests needed

## Blocking Issues
[List any blockers discovered. If non-empty, stop and revise plan first.]

## Gate Decision
- [ ] PASS: proceed to Phase 01
- [ ] FAIL: revise plan — document specific blockers and required plan changes
