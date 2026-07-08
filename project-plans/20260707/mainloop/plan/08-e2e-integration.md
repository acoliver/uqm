# Phase 08: End-to-End Integration Verification

## Phase ID
`PLAN-20260707-MAINLOOP.P08`

## Prerequisites
- Phase 07 complete (all code written, C wiring in place)

## Requirements Verified (holistic)

All 10 requirements (REQ-ML-001 through REQ-ML-010) verified end-to-end
through a real binary that boots and reaches the main menu via the Rust
entry point.

---

## Integration Tasks

### 1. Enable the flag
- Set `USE_RUST_MAINLOOP=1` in `sc2/build.vars`
- Rebuild: `cd sc2 && ./build.sh uqm`

### 3. Verify Rust entry symbol
```bash
nm sc2/uqm | grep rust_game_loop
nm sc2/uqm | grep rust_dispatch_activity
```
Must show `rust_game_loop` and `rust_dispatch_activity` symbols present.

### 4. Boot the binary (full integration test)
```bash
cd sc2 && ./uqm -o -f &
PID=$!
sleep 10
kill $PID
```
Verify: no crash, no NULL frame errors, reaches splash or menu. This
proves `rust_game_loop()` → Starcon2Main init → game loop all
work end-to-end. (Note: `--help` exits before init and does NOT prove
graphics init — the full boot test is required.)

---

## E2E Boundary Test Suite

### Integration test file: `rust/tests/mainloop_e2e.rs`

```rust
/// @plan PLAN-20260707-MAINLOOP.P08
/// @requirement REQ-ML-001
/// Verify rust_game_loop symbol exists and is callable
#[test]
fn test_rust_game_loop_symbol_exists() {
    // Signature matches starcon.c wiring: extern int rust_game_loop(void)
    extern "C" { fn rust_game_loop() -> i32; }
    let ptr = rust_game_loop as *const ();
    assert!(!ptr.is_null());
}

/// @plan PLAN-20260707-MAINLOOP.P08
/// @requirement REQ-ML-005
/// Full boundary round-trip: activity + named game-state both directions
/// Uses NAMED accessors (uqm_get_chmmr_bomb_state), NOT byte offsets
#[test]
fn test_full_boundary_round_trip() {
    // Activity round-trip
    set_current_activity(ActivityValue(0x0403));
    assert_eq!(get_current_activity().0, 0x0403);

    // Named game-state round-trip (uses GET_GAME_STATE internally)
    uqm_set_chmmr_bomb_state(2);
    assert_eq!(uqm_get_chmmr_bomb_state(), 2);
}
```

---

## Verification Commands

```bash
# Rust
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# C+Rust build with flag enabled
cd sc2 && ./build.sh uqm

# Symbol verification
nm sc2/uqm | grep rust_game_loop
nm sc2/uqm | grep rust_dispatch_activity

# Full boot test (NOT --help, which exits before init/Starcon2Main)
cd sc2 && ./uqm -o -f &
PID=$!
sleep 10
kill $PID
# Verify: no crash, no NULL frame errors, reaches splash/menu
```

## Semantic Verification Checklist

### REQ-ML-001: Rust Game Loop Body
- [ ] `rust_game_loop` symbol present in binary
- [ ] Full boot test reaches splash/menu (proves rust_game_loop → init → loop)

### REQ-ML-002: Init Sequence Orchestration
- [ ] Binary boots (C main() startup + Rust game loop both work)

### REQ-ML-003: CurrentActivity Accessors
- [ ] Round-trip tests pass in both directions

### REQ-ML-004: Activity State Machine
- [ ] All 4 dispatch branch tests pass

### REQ-ML-005: Boundary Test Suite
- [ ] `test_full_boundary_round_trip` passes
- [ ] No NULL frame errors in boot log

### REQ-ML-006: Init Ordering Observable
- [ ] `test_rust_game_loop_symbol_exists` passes

### REQ-ML-007: Loop Structure
- [ ] Outer/inner loop tests pass

### REQ-ML-008: Shutdown Sequence
- [ ] `test_shutdown_calls_in_order` passes

### REQ-ML-009: C-to-Rust Callback
- [ ] `rust_dispatch_activity` symbol present
- [ ] Callback test passes

### REQ-ML-010: Game State Round-Trip
- [ ] Game state byte round-trip test passes

## Deferred Implementation Detection
```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/mainloop/
```
Must return zero hits across ALL mainloop files.

## Success Criteria (Definition of Done from overview)
- [ ] All 10 REQ-ML-* requirements have a passing test
- [ ] `rust_game_loop()` is the real entry point when `USE_RUST_MAINLOOP=1`
- [ ] Init sequence runs in exact order
- [ ] Activity state machine dispatches all branches
- [ ] Shutdown sequence runs in order
- [ ] C can call `rust_dispatch_activity()`
- [ ] No `unwrap()`/`expect()` in any mainloop FFI bridge file
- [ ] `cargo fmt`, `cargo clippy -D warnings`, `cargo test` all green
- [ ] Zero hits from deferred-implementation grep
- [ ] RaceDesc-class defense: `ActivityValue` is `#[repr(transparent)]`,
      accessed only through FFI function calls, with round-trip tests

## Failure Recovery
- Set `USE_RUST_MAINLOOP=0` in `sc2/build.vars`
- Rebuild: `cd sc2 && ./build.sh uqm`
- Original C path runs (backward compat verified)

## Phase Completion Marker
Create: `project-plans/20260707/mainloop/.completed/P08.md`
