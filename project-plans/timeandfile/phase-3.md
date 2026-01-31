# Phase 3 - Integrated run + proof log

## Objective
Prove the C build is using Rust implementations for both file and time modules, with symbol resolution and runtime logs showing unique markers.

## Test-First Methodology
1. Define symbol + log marker checklist first.
2. Run a single build with both `USE_RUST_FILE` and `USE_RUST_CLOCK` enabled.
3. Validate symbols, call sites, and runtime markers in one run.

## Project Context (Self-Contained)
- Build entry: `sc2/build.sh`
- Rust crate: `rust/`
- Binary: `sc2/build/uqm`
- Log file: `rust-bridge.log`

## Scope
- Build with `USE_RUST_FILE=1` and `USE_RUST_CLOCK=1`.
- Run the game to trigger file operations and clock ticks.
- Verify log + symbols + call sites.

## Subagent Prompt (Implementation)
You are implementing Phase 3. Do the following:

1) Add a build config option that allows enabling both `USE_RUST_FILE` and `USE_RUST_CLOCK` simultaneously and ensure both variables are exported for Makeinfo.
2) Ensure `rust-bridge.log` is truncated on each run (`rust_bridge_init` should do this).
3) Document the exact run sequence that triggers:
   - File operations (e.g., load menu or resource access).
   - Clock ticks (hyperspace for ~5 seconds).

## Verification Prompt
You are verifying Phase 3. Run commands and confirm symbol resolution + call sites + runtime markers. If any step fails, report failure.

Pass/Fail commands:
1) `cd rust && cargo build --release && cd ..`
2) `cd sc2 && ./build.sh uqm config` (enable both Rust toggles)
3) `cd sc2 && ./build.sh uqm`
4) Symbol resolution: `nm sc2/build/uqm | rg "T fileExists|T copyFile|T InitGameClock|T GameClockTick"`
5) Call-site verification (Linux): `objdump -d sc2/build/uqm | rg "call.*fileExists|call.*InitGameClock"`
6) Call-site verification (macOS fallback): `otool -tV sc2/build/uqm | rg "fileExists|InitGameClock"`
7) Run the game and follow the documented run sequence.
8) Log markers:
   - `rg -n "RUST_BRIDGE_PHASE0_OK" rust-bridge.log`
   - `rg -n "RUST_FILE_EXISTS_CALLED" rust-bridge.log`
   - `rg -n "RUST_COPY_FILE_CALLED" rust-bridge.log`
   - `rg -n "RUST_CLOCK_INIT" rust-bridge.log`
   - `rg -n "RUST_CLOCK_TICK" rust-bridge.log`

Success criteria:
- Build succeeds.
- Symbols defined in binary.
- Call sites exist in binary.
- Log contains all markers in a single run.

User action required:
- Follow run sequence and exit cleanly.
