# Phase 07: C Wiring — Starcon2Main Delegation (Revised)

## Phase ID
`PLAN-20260707-MAINLOOP.P07`

## Prerequisites
- Phase 06 complete (rust_game_loop + game loop body + shutdown_game_kernel working)

## Requirements Implemented

### REQ-ML-008: Game-Kernel Cleanup Verification
P06 implements `shutdown_game_kernel()` (starcon.c:313-318 only).
This phase verifies it via the full binary build. C `main()` owns
subsystem teardown (`uqm.c:479-507`) after `MainExited`.

**Note**: REQ-ML-009 (`rust_dispatch_activity`) was **removed** — there
is no C call site for it. With `Starcon2Main` delegating to
`rust_game_loop()`, the activity dispatch lives entirely in Rust.
Adding a separate C-to-Rust dispatch callback would create an
unnecessary and potentially dangerous second entry point.

---

## Implementation

**This phase is C wiring only.** No new Rust code.

### C Wiring: Starcon2Main delegates to rust_game_loop

- `sc2/src/uqm/starcon.c` — add at the TOP of `Starcon2Main` body:
  ```c
  int
  Starcon2Main (int argc, char *argv[])
  {
  #ifdef USE_RUST_MAINLOOP
      extern int rust_game_loop(void);
      return rust_game_loop();
  #else
      // ... original Starcon2Main body unchanged ...
  #endif
  }
  ```
- `sc2/build.vars` — set `USE_RUST_MAINLOOP=1` for testing (P08)

C `main()` remains unchanged — it calls `StartThread(Starcon2Main, ...)`,
the main-thread pump runs as before, and after `MainExited` it does
subsystem teardown (`uqm.c:479-507`).

### Shutdown ownership (final):
- **P06** implements `shutdown_game_kernel()` in `game_loop.rs`:
  `UninitGameKernel` → `FreeMasterShipList` → `FreeKernel` →
  `log_showBox` → `set_main_exited(true)`. NO subsystem teardown.
- **C main()** owns subsystem teardown after `MainExited` (`uqm.c:479-507`).
- **P07** does NOT add `shutdown.rs` or modify shutdown logic.

---

## TDD

```rust
/// @plan PLAN-20260707-MAINLOOP.P07
/// @requirement REQ-ML-008
/// Tier 1: verify game-kernel cleanup ordering via injected hooks
#[test]
fn test_shutdown_game_kernel_order() {
    let mut calls = Vec::new();
    shutdown_game_kernel_with_hooks(&mut calls);
    assert_eq!(calls[0], "UninitGameKernel");
    assert_eq!(calls[1], "FreeMasterShipList");
    assert_eq!(calls[2], "FreeKernel");
    assert_eq!(calls[3], "log_showBox");
    assert_eq!(calls[4], "MainExited");
    // Must NOT contain subsystem teardown functions
    assert!(!calls.contains(&"TFB_UninitInput"));
    assert!(!calls.contains(&"DestroyColorMaps"));
}
```

### C binary verification:
```bash
# Build with USE_RUST_MAINLOOP=1
cd sc2 && ./build.sh uqm

# Verify Starcon2Main delegates to Rust
nm sc2/uqm | grep rust_game_loop

# Verify backward compat (flag OFF)
cd sc2 && USE_RUST_MAINLOOP= ./build.sh uqm
nm sc2/uqm | grep Starcon2Main
```

---

## Verification Commands
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd sc2 && ./build.sh uqm
```

## Semantic Verification Checklist
- [ ] Game-kernel cleanup verified (from P06, no subsystem teardown)
- [ ] `Starcon2Main` delegates to `rust_game_loop` when `USE_RUST_MAINLOOP`
- [ ] Binary builds with flag OFF (backward compat)
- [ ] C main-thread pump unchanged (uqm.c:456-472)
- [ ] No `rust_dispatch_activity` (removed — no C caller)
- [ ] No `unwrap()`/`expect()` in mainloop code

## Deferred Implementation Detection
```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/mainloop/
```

## Success Criteria
- [ ] REQ-ML-008: game-kernel cleanup only (starcon.c:313-318)
- [ ] C wiring: Starcon2Main delegates to rust_game_loop
- [ ] Backward compatible (flag OFF works)

## Failure Recovery
- `git restore sc2/src/uqm/starcon.c`

## Phase Completion Marker
Create: `project-plans/20260707/mainloop/.completed/P07.md`
