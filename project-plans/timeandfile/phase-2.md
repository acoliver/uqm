# Phase 2 - Time module (uqm/clock) C→Rust swap

## Objective
Replace UQM game clock functions with Rust implementations callable from C, and prove via runtime log + symbol resolution that the Rust versions are used.

## Test-First Methodology
1. Define exact C signatures and shared state layout.
2. Implement Rust externs exporting the same symbol names as C.
3. Exclude `clock.c` when Rust is enabled.
4. Verify with symbol checks, call-site verification, and runtime behavior.

## Project Context (Self-Contained)
- Header: `sc2/src/uqm/clock.h`
- C implementation: `sc2/src/uqm/clock.c`
- Game clock storage: `sc2/src/uqm/globdata.h` (contains `CLOCK_STATE GameClock;`).
- Build config: `sc2/build/unix/build.config`.
- Makeinfo file: `sc2/src/uqm/Makeinfo`.

## CLOCK_STATE Layout (from `clock.h`)
```
typedef struct
{
    BYTE day_index, month_index;
    COUNT year_index;
    SIZE tick_count, day_in_ticks;
    QUEUE event_q;
} CLOCK_STATE;
```

## C Function Signatures (from `clock.h`)
```
BOOLEAN InitGameClock(void);
BOOLEAN UninitGameClock(void);
void SetGameClockRate(COUNT seconds_per_day);
void GameClockTick(void);
void MoveGameClockDays(COUNT days);
void LockGameClock(void);
void UnlockGameClock(void);
BOOLEAN GameClockRunning(void);
```

## Scope
- Rust `extern "C"` replacements that export *the exact C symbol names* above.
- Log markers for each function:
  - `RUST_CLOCK_INIT`, `RUST_CLOCK_UNINIT`, `RUST_CLOCK_RATE`, `RUST_CLOCK_TICK`, `RUST_CLOCK_MOVE`, `RUST_CLOCK_LOCK`, `RUST_CLOCK_UNLOCK`, `RUST_CLOCK_RUNNING`.
- Exclude `clock.c` from compilation when `USE_RUST_CLOCK` is enabled.

## Subagent Prompt (Implementation)
You are implementing Phase 2. Do the following:

1) Implement Rust externs with the exact C names in a Rust module (e.g., `rust/src/time/clock_bridge.rs`).
2) In Rust, access the existing C `CLOCK_STATE GameClock` storage by declaring:
```
#[repr(C)]
pub struct ClockState { ... }
extern "C" { static mut GameClock: ClockState; }
```
3) Update fields on `GameClock` directly in Rust (e.g., `day_index`, `month_index`, `year_index`, `tick_count`, `day_in_ticks`).
4) Add log markers to `rust-bridge.log` for each function.
5) Exclude `clock.c` when `USE_RUST_CLOCK` is enabled:
   - In `sc2/src/uqm/Makeinfo`, remove `clock.c` from `uqm_CFILES` when flag is set.
6) Add `USE_RUST_CLOCK` toggle in `sc2/build/unix/build.config` (like Phase 0/1).
7) Add negative guard to `clock.c`:
```
#ifdef USE_RUST_CLOCK
#error "clock.c should not be compiled when USE_RUST_CLOCK is enabled"
#endif
```

## Verification Prompt
You are verifying Phase 2. Run commands and confirm symbol resolution + runtime markers + behavior. If any step fails, report failure.

Qualitative checks:
- `clock.c` excluded when `USE_RUST_CLOCK` enabled.
- `nm` shows `InitGameClock`, `GameClockTick`, etc. defined in binary.

Pass/Fail commands:
1) `cd rust && cargo build --release && cd ..`
2) `cd sc2 && ./build.sh uqm config` (enable `USE_RUST_CLOCK`).
3) `cd sc2 && ./build.sh uqm`
4) Verify symbols: `nm sc2/build/uqm | rg "T InitGameClock|T GameClockTick|T MoveGameClockDays"`
5) Verify `clock.c` excluded: `test ! -f sc2/build/obj*/uqm/clock.o`
6) Run the game and enter a mode that advances time (hyperspace). Exit after 5 seconds.
7) `rg -n "RUST_CLOCK_INIT" rust-bridge.log`
8) `rg -n "RUST_CLOCK_TICK" rust-bridge.log`

Success criteria:
- Build succeeds with `USE_RUST_CLOCK` enabled.
- Rust symbols are defined in binary.
- `clock.c` is not compiled.
- Log markers appear during runtime.

User action required:
- Document steps to enter hyperspace and wait ~5 seconds to tick clock.
