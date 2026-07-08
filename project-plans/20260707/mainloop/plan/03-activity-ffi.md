# Phase 03: Activity Types + CurrentActivity FFI Accessors

## Phase ID
`PLAN-20260707-MAINLOOP.P03`

## Prerequisites
- Phase 02a pseudocode verification PASS
- Phase 02b (C wrapper functions) complete — all accessors exist in C
- Preflight (P0.5) confirms toolchain and existing FFI infrastructure

## Requirements Implemented

### REQ-ML-003: CurrentActivity FFI Accessors
**Requirement text**: Rust can read and write `GLOBAL(CurrentActivity)`
through FFI accessor functions, never by direct memory offset.

Behavior contract:
- GIVEN: C global `GlobData.Game_state.CurrentActivity` is set to value X
- WHEN: Rust calls `get_current_activity()`
- THEN: Rust receives X
- AND: when Rust calls `set_current_activity(Y)`, C reads Y

### REQ-ML-005: FFI Boundary Test Suite (foundation)
**Requirement text**: Every FFI bridge has integration tests proving data
crosses the boundary correctly.

### REQ-ML-010: Game State Round-Trip
**Requirement text**: Game state flags read from Rust match what C wrote.

**Note**: Game state is **bit-packed** via `getGameState(state, startBit,
endBit)`. Rust MUST use named C wrappers (from P02b) that call
`GET_GAME_STATE`/`SET_GAME_STATE` internally. Raw byte-offset access is
**unsafe** (wrong bit range) and forbidden.

**FFI ABI Rules** (critical — verified against `libs/compiler.h`):
- `BOOLEAN` is a C `enum` → ABI is `int` (4 bytes). Rust MUST use
  `type CBoolean = libc::c_int;`, NOT Rust `bool`.
- `ACTIVITY` / `UWORD` / `COUNT` are `uint16` → Rust `u16`.
- `LoadKernel` signature is `BOOLEAN LoadKernel(int argc, char *argv[])`,
  NOT `(COUNT, COUNT)`. Rust: `fn LoadKernel(argc: c_int, argv: *mut *mut c_char) -> c_int`.
- Add C-side `_Static_assert(sizeof(BOOLEAN) == sizeof(int))`.
- Add C-side `_Static_assert(sizeof(ACTIVITY) == 2)`.

---

## Implementation Tasks

### This phase follows Stub → TDD → Impl internally.

#### Stub (compile-safe skeletons)

**Files to create:**
- `rust/src/mainloop/mod.rs` — module declaration, `MainLoopError` enum
  - marker: `@plan PLAN-20260707-MAINLOOP.P03`
- `rust/src/mainloop/activity.rs` — `ActivityValue`, `ActivityKind`, `ActivityFlags` type defs
  - marker: `@plan PLAN-20260707-MAINLOOP.P03 @requirement REQ-ML-003`
- `rust/src/mainloop/c_extern.rs` — `extern "C"` declarations for accessors
  - marker: `@plan PLAN-20260707-MAINLOOP.P03`
- `rust/src/mainloop/bridge.rs` — safe wrappers (initially `todo!()`)
  - marker: `@plan PLAN-20260707-MAINLOOP.P03`
- `rust/src/mainloop/rust_test_bridge.c` — C test helper exposing
  `test_set_activity()` / `test_get_activity()` that directly write/read
  `GlobData.Game_state.CurrentActivity`
  - marker: `@plan PLAN-20260707-MAINLOOP.P03`

**Files to modify:**
- `rust/src/lib.rs` — add `pub mod mainloop;`
  - marker: `@plan PLAN-20260707-MAINLOOP.P03`
- `rust/build.rs` — compile `rust_test_bridge.c` into `libuqm_test_bridge.a`
  - marker: `@plan PLAN-20260707-MAINLOOP.P03`

#### TDD (write failing tests first)

**Test files to create:**
- `rust/src/mainloop/bridge.rs` (#[cfg(test)] module) — boundary tests:

```rust
/// @plan PLAN-20260707-MAINLOOP.P03
/// @requirement REQ-ML-003
#[test]
fn test_current_activity_round_trip_rust_to_c() {
    // GIVEN: C global is at a known state
    // WHEN: Rust sets activity to 0x0403 (IN_ENCOUNTER | START_ENCOUNTER)
    set_current_activity(ActivityValue(0x0403));
    // THEN: C reads the same value
    assert_eq!(unsafe { test_get_activity() }, 0x0403);
}

/// @plan PLAN-20260707-MAINLOOP.P03
/// @requirement REQ-ML-003
#[test]
fn test_current_activity_round_trip_c_to_rust() {
    // GIVEN: C sets activity
    unsafe { test_set_activity(0x0804); }  // START_INTERPLANETARY | IN_INTERPLANETARY
    // WHEN: Rust reads it
    let activity = get_current_activity();
    // THEN: Rust sees the same value
    assert_eq!(activity.0, 0x0804);
}

/// @plan PLAN-20260707-MAINLOOP.P03
/// @requirement REQ-ML-010
#[test]
fn test_game_state_round_trip() {
    // GIVEN: set CHMMR_BOMB_STATE from Rust via NAMED accessor (not byte offset)
    uqm_set_chmmr_bomb_state(2);
    // THEN: C reads 2 via GET_GAME_STATE macro internally
    assert_eq!(uqm_get_chmmr_bomb_state(), 2);
    // AND: Rust reads 2 back
    assert_eq!(uqm_get_chmmr_bomb_state(), 2);
}

/// @plan PLAN-20260707-MAINLOOP.P03
/// @requirement REQ-ML-003
#[test]
fn test_next_activity_round_trip() {
    // NextActivity is a standalone global (save.h:66)
    // Used by load/restart path: CurrentActivity | NextActivity & CHECK_LOAD
    set_next_activity(0x1000);  // CHECK_LOAD = MAKE_WORD(0, 1<<4)
    assert_eq!(get_next_activity(), 0x1000);
}

/// @plan PLAN-20260707-MAINLOOP.P03
/// @requirement REQ-ML-003
#[test]
fn test_activity_flags_decomposition() {
    let av = ActivityValue(0x0403);
    assert_eq!(av.kind(), ActivityKind::InEncounter);
    assert!(av.has_flag(ActivityFlags::START_ENCOUNTER));
    assert!(!av.has_flag(ActivityFlags::CHECK_ABORT));
}
```

#### Impl (production code to pass tests)

**C-side accessors are created in P02b** (`rust_bridge_mainloop.c`).
This phase only writes the **Rust side**.

**Files to implement (Rust side):**
- `rust/src/mainloop/c_extern.rs` — fill in extern declarations (matching
  the C wrappers from P02b):
  ```rust
  // C ABI type aliases (verified against libs/compiler.h)
  type CBoolean = libc::c_int;  // BOOLEAN is C enum = int
  type CActivity = u16;         // ACTIVITY = UWORD = uint16

  extern "C" {
      // Activity accessors (from P02b)
      fn get_current_activity() -> u16;
      fn set_current_activity(val: u16);
      fn get_next_activity() -> u16;   // save.h:66 — for load/restart path
      fn set_next_activity(val: u16);
      fn get_last_activity() -> u16;   // setup.h:60 — standalone global
      fn set_last_activity(val: u16);
      // Named game-state accessors (from P02b — NOT byte offsets)
      fn uqm_get_chmmr_bomb_state() -> u8;
      fn uqm_set_chmmr_bomb_state(val: u8);
      fn uqm_get_starbase_available() -> u8;
      fn uqm_get_global_flags_and_data() -> u8;
      fn uqm_get_kohr_ah_killed_all() -> u8;
      // SIS crew count (for death detection: starcon.c:295)
      fn uqm_get_crew_enlisted() -> u16;  // GLOBAL_SIS(CrewEnlisted) is COUNT
  }
  ```
- `rust/src/mainloop/bridge.rs` — implement safe wrappers calling the externs
- `rust/src/mainloop/activity.rs` — implement `kind()`, `has_flag()`,
  `set_flag()`, `clear_flag()` on `ActivityValue`

### Pseudocode traceability
- Uses pseudocode lines: 31, 36 (get/set_current_activity), 71-72 (game state)

---

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Structural Verification Checklist
- [ ] `mainloop/mod.rs`, `activity.rs`, `c_extern.rs`, `bridge.rs` created
- [ ] `lib.rs` exports `pub mod mainloop`
- [ ] `rust_bridge_mainloop.c/.h` created with accessor implementations
- [ ] `rust_test_bridge.c` compiled by build.rs
- [ ] Tests compile and run

## Semantic Verification Checklist
- [ ] `test_current_activity_round_trip_rust_to_c` passes (Rust writes, C reads)
- [ ] `test_current_activity_round_trip_c_to_rust` passes (C writes, Rust reads)
- [ ] `test_game_state_round_trip` passes (both directions, via NAMED accessors)
- [ ] `test_activity_flags_decomposition` passes
- [ ] No `unwrap()`/`expect()` in bridge code
- [ ] No raw byte-offset game-state access anywhere

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" \
  rust/src/mainloop/
```
Must return zero hits (stub `todo!()` removed during Impl step).

## Success Criteria
- [ ] REQ-ML-003 demonstrated by round-trip tests
- [ ] REQ-ML-010 demonstrated by game-state round-trip test
- [ ] REQ-ML-005 boundary test pattern established
- [ ] All verification commands pass

## Failure Recovery
- `git restore rust/src/lib.rs rust/build.rs`
- `rm -rf rust/src/mainloop/`
- `git restore sc2/src/uqm/rust_bridge_mainloop.c sc2/src/uqm/rust_bridge_mainloop.h`

## Phase Completion Marker
Create: `project-plans/20260707/mainloop/.completed/P03.md`
