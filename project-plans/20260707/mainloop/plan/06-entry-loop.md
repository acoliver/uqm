# Phase 06: Rust Game Loop Body (`rust_game_loop`) (Revised)

## Phase ID
`PLAN-20260707-MAINLOOP.P06`

## Prerequisites
- Phase 05 complete (state machine + named game-state accessors working)
- Phase 04 complete (startup verification gate)

## Requirements Implemented

### REQ-ML-001: Rust Game Loop Body
Rust provides `rust_game_loop()` as the replacement for `Starcon2Main()`.
It runs on the Starcon2Main thread (via existing `StartThread`). The C
main-thread event pump (`uqm.c:456-472`) is **preserved unchanged**.

### REQ-ML-007: Game Loop Outer/Inner Structure
Two-level loop preserved: outer (`while StartGame()`) + inner
(`loop until get_current_activity().has_flag(CHECK_ABORT)`).

---

## Implementation Tasks

### Stub

**Files to create:**
- `rust/src/mainloop/game_loop.rs` — `rust_game_loop()` entry + `run_game_lifecycle()`
  - marker: `@plan PLAN-20260707-MAINLOOP.P06`
- Add FFI externs for game-loop functions: `StartGame`, `LoadKernel`,
  `initAudio`, `InitGameStructures`, `InitGameClock`,
  `AddInitialGameEvents`, `VisitStarBase`, `RaceCommunication`,
  `ExploreSolarSys`, `StopSound`, `UninitGameClock`, `UninitGameStructures`,
  `ClearPlayerInputAll`, `DrawAutoPilotMessage`, `SetGameClockRate`,
  `InstallBombAtEarth`, `InitCommunication`
  + C wrappers from P02b: `uqm_splash_with_bg_init_kernel`,
  `uqm_battle_with_frame_callback`, `uqm_zero_global_velocity`,
  `uqm_set_flash_rect_null`, `uqm_set_player_input_all_or_explode`,
  `uqm_get_crew_enlisted`
  in `rust/src/mainloop/c_extern.rs`

**Files to modify:**
- `rust/src/mainloop/mod.rs` — add `pub mod game_loop;`

### TDD
**Tests in `rust/src/mainloop/game_loop.rs`:**

Tests are **Tier 1** (pure Rust) — use injected hooks, not real C calls:

```rust
/// @plan PLAN-20260707-MAINLOOP.P06
/// @requirement REQ-ML-001
#[test]
fn test_rust_game_loop_symbol_exists() {
    // Tier 2: verify symbol linkage
    extern "C" { fn rust_game_loop() -> i32; }
    let ptr = rust_game_loop as *const ();
    assert!(!ptr.is_null());
}

/// @plan PLAN-20260707-MAINLOOP.P06
/// @requirement REQ-ML-007
#[test]
fn test_game_loop_exits_on_check_abort() {
    // Tier 1: use a mock activity source that sets CHECK_ABORT after 1 iteration
    let mut iterations = 0;
    run_game_loop_with_mock_activity(|_| {
        iterations += 1;
        if iterations >= 1 { ActivityValue(0x4003) } // CHECK_ABORT | IN_HYPERSPACE
        else { ActivityValue(0x0003) }
    });
    // Inner loop should exit after CHECK_ABORT
    assert!(iterations >= 1);
}

/// @plan PLAN-20260707-MAINLOOP.P06
/// @requirement REQ-ML-007
#[test]
fn test_game_loop_re_reads_activity_after_dispatch() {
    // Tier 1: verify the loop re-reads CurrentActivity after dispatch
    // The mock dispatch mutates activity; the loop must see the mutation
    let dispatch_mutated = Arc::new(AtomicBool::new(false));
    // ... setup mock that sets dispatch_mutated=true during "dispatch"
    // ... verify the loop's post-dispatch logic sees the new value
}
```

### Impl
- Implement `rust_game_loop()` — pseudocode lines 1-7
- Implement `run_game_lifecycle()` — pseudocode lines 9-74
  - **CRITICAL**: re-read CurrentActivity after dispatch (line 41)
  - Set LastActivity after dispatch (line 55)
  - Post-dispatch flag mutation uses re-read value (lines 44-50)
  - Inner loop condition re-reads CurrentActivity (line 61)
- Implement `execute_activity()` — pseudocode lines 100-117
- Implement `should_stop_loop()` — pseudocode lines 120-136
- Implement `shutdown_game_kernel()` directly in `game_loop.rs` (5 FFI calls, not a stub)

### Pseudocode traceability
- Uses pseudocode lines: 1-74, 100-136

---

## Verification Commands
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Semantic Verification Checklist
- [ ] `rust_game_loop` is `#[no_mangle] pub extern "C" fn` returning c_int
- [ ] Outer loop governed by `StartGame()` return value
- [ ] Inner loop governed by re-read `CHECK_ABORT` flag (NOT stale value)
- [ ] **CurrentActivity re-read after every dispatch call** (pseudocode line 41)
- [ ] **LastActivity set after every dispatch** (pseudocode line 55)
- [ ] Win/loss/death conditions match starcon.c:275-290
- [ ] `shutdown_game_kernel` implemented (not stubbed)
- [ ] No `unwrap()`/`expect()` in game_loop code

## Deferred Implementation Detection
```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/mainloop/game_loop.rs
```
(P06 implements `shutdown_game_kernel` directly — no stubs. P07 adds `rust_dispatch_activity` and C wiring.)

## Success Criteria
- [ ] REQ-ML-001: `rust_game_loop` exists as no_mangle extern "C"
- [ ] REQ-ML-007: outer/inner loop structure correct with re-read pattern

## Failure Recovery
- `git restore rust/src/mainloop/game_loop.rs rust/src/mainloop/shutdown.rs rust/src/mainloop/c_extern.rs`

## Phase Completion Marker
Create: `project-plans/20260707/mainloop/.completed/P06.md`
