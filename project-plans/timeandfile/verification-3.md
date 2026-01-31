# Phase 3 Verification - Integrated run + proof log

## Purpose
Verify that the C build uses Rust implementations for time and file modules and logs all required markers in a single run.

## Qualitative Checks
- Both `USE_RUST_FILE` and `USE_RUST_CLOCK` enabled in build config.
- Both variables are exported for Makeinfo evaluation.
- `files.c` and `clock.c` excluded from build.
- Rust symbols defined in binary.

## Pass/Fail Commands
1) `cd rust && cargo build --release && cd ..`
2) `cd sc2 && ./build.sh uqm config` (enable both Rust toggles)
3) `cd sc2 && ./build.sh uqm`
4) Symbol resolution: `nm sc2/build/uqm | rg "T fileExists|T copyFile|T InitGameClock|T GameClockTick"`
5) Exclusion checks:
   - `test ! -f sc2/build/obj*/libs/file/files.o`
   - `test ! -f sc2/build/obj*/uqm/clock.o`
6) Call-site check (Linux): `objdump -d sc2/build/uqm | rg "call.*fileExists|call.*InitGameClock"`
7) Call-site check (macOS fallback): `otool -tV sc2/build/uqm | rg "fileExists|InitGameClock"`
8) Run the game and follow the documented run sequence.
9) Log markers:
   - `rg -n "RUST_BRIDGE_PHASE0_OK" rust-bridge.log`
   - `rg -n "RUST_FILE_EXISTS_CALLED" rust-bridge.log`
   - `rg -n "RUST_COPY_FILE_CALLED" rust-bridge.log`
   - `rg -n "RUST_CLOCK_INIT" rust-bridge.log`
   - `rg -n "RUST_CLOCK_TICK" rust-bridge.log`

## Expected Results
- All commands succeed.
- Rust symbols defined in binary.
- Call sites exist.
- Log contains all markers.

## Failure Conditions
- Missing symbols, build failure, or missing log markers.

## User Action Needed
- Follow run sequence and exit cleanly.
