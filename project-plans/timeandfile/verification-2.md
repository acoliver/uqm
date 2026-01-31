# Phase 2 Verification - Time module C→Rust swap

## Purpose
Confirm that the game clock functions are served by Rust and invoked during runtime.

## Qualitative Checks
- `USE_RUST_CLOCK` enabled in `sc2/build/unix/build.config` and adds `-DUSE_RUST_CLOCK`.
- `USE_RUST_CLOCK` is exported for Makeinfo evaluation.
- `clock.c` excluded when `USE_RUST_CLOCK=1`.
- Rust exports `InitGameClock`, `GameClockTick`, etc.

## Pass/Fail Commands
1) `cd rust && cargo build --release && cd ..`
2) `cd sc2 && ./build.sh uqm config` (enable `USE_RUST_CLOCK`).
3) `cd sc2 && ./build.sh uqm`
4) Symbol resolution: `nm sc2/build/uqm | rg "T InitGameClock|T GameClockTick|T MoveGameClockDays"`
5) Exclusion check: `test ! -f sc2/build/obj*/uqm/clock.o`
6) Call-site check (Linux): `objdump -d sc2/build/uqm | rg "call.*InitGameClock|call.*GameClockTick"`
7) Call-site check (macOS fallback): `otool -tV sc2/build/uqm | rg "InitGameClock|GameClockTick"`
8) Run the game and enter hyperspace for ~5 seconds.
9) Log markers:
   - `rg -n "RUST_CLOCK_INIT" rust-bridge.log`
   - `rg -n "RUST_CLOCK_TICK" rust-bridge.log`

## Stronger Functional Test (Behavior Divergence)
- Temporarily hardcode Rust `GameClockTick` to increment day index by 2 and log `RUST_CLOCK_DOUBLE_TICK`.
- Verify after a few seconds of hyperspace:
  - `rg -n "RUST_CLOCK_DOUBLE_TICK" rust-bridge.log`
- Revert the divergence after verification.

## Expected Results
- All commands succeed.
- Rust symbols defined in binary.
- `clock.c` not compiled.
- Log markers present during runtime.

## Failure Conditions
- Build fails, symbols undefined, missing log markers.

## User Action Needed
- Document exact menu steps to enter hyperspace and wait ~5 seconds.
