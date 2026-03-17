# Phase 06: SleepThreadUntil Async Pumping Restoration

## Phase ID
`PLAN-20260314-THREADING.P06`

## Prerequisites
- Required: Phase 05a (Return Value Implementation Verification) completed
- Expected previous artifact: `project-plans/20260311/threading/.completed/P05a.md`
- All tests pass
- C project builds cleanly

## Requirements Implemented (Expanded)

### SleepThreadUntil async pumping
**Requirement text**: “While SleepThreadUntil is waiting for a future wake time, the threading subsystem shall preserve the legacy behavior of servicing pending asynchronous engine work required by the main-thread sleep path before and between blocking intervals.”

Behavior contract:
- GIVEN: `SleepThreadUntil` called with a future wake time
- WHEN: Async callbacks are scheduled before the wake time
- THEN: `Async_process()` is called to service them before and between sleep intervals
- AND: The function wakes early enough to service async work scheduled before the final wake time
- AND: The function returns promptly when `wakeTime <= now`

Why it matters:
- The main-thread sleep path services timer-driven operations (audio callbacks, input processing, deferred operations)
- Without async pumping, these operations stall until the full sleep completes
- Spec §6.5 explicitly states this is a parity gap

## Implementation Tasks

This is a **C-only change**. No Rust modifications needed.

### Files to modify

#### `sc2/src/libs/threads/rust_thrcommon.c` — Replace `SleepThreadUntil`

**Current** (lines 192-200):
```c
void
SleepThreadUntil (TimeCount wakeTime)
{
    TimeCount now = GetTimeCounter();
    if (wakeTime > now)
    {
        SleepThread(wakeTime - now);
    }
}
```

**Target** (pseudocode lines 54-65, matches legacy `thrcommon.c:333-362`):
```c
void
SleepThreadUntil (TimeCount wakeTime)
{
    for (;;)
    {
        uint32 nextTimeMs;
        TimeCount nextTime;
        TimeCount now;

        Async_process ();

        now = GetTimeCounter ();
        if (wakeTime <= now)
            return;

        nextTimeMs = Async_timeBeforeNextMs ();
        nextTime = (nextTimeMs / 1000) * ONE_SECOND +
                ((nextTimeMs % 1000) * ONE_SECOND / 1000);
        if (wakeTime < nextTime)
            nextTime = wakeTime;

        SleepThread (nextTime - now);
    }
}
```

### Behavioral validation requirement

Because this fix is behavior-sensitive, execution must include more than grep/build review. At least one of the following must be completed and recorded in the phase completion marker:

1. **Automated harness/test** proving `Async_process()` is invoked before and between sleep intervals and that the sleep duration clamps to the next async deadline.
2. **Manual behavioral validation** using temporary instrumentation or an existing debug hook to show:
   - `Async_process()` runs before the first sleep interval,
   - `SleepThreadUntil` loops more than once when async work is scheduled earlier than `wakeTime`,
   - the function returns promptly when `wakeTime <= now`.

If automated testing is impractical in the current infrastructure, the manual validation procedure is mandatory rather than optional.

### Pseudocode traceability
- Uses pseudocode lines: 54-65

## Verification Commands

```bash
# Rust tests should still pass (no Rust changes in this phase)
cd /Users/acoliver/projects/uqm/rust
cargo test --workspace --all-features

# C project build verification
cd /Users/acoliver/projects/uqm/sc2
make -f Makefile.build
```

## Structural Verification Checklist
- [ ] `SleepThreadUntil` contains `Async_process()` call
- [ ] `SleepThreadUntil` contains `Async_timeBeforeNextMs()` call
- [ ] `SleepThreadUntil` uses `for (;;)` loop (not single-shot)
- [ ] `SleepThreadUntil` returns when `wakeTime <= now`
- [ ] Time conversion uses overflow-safe formula matching legacy
- [ ] `SleepThread(nextTime - now)` used for sub-interval sleeps
- [ ] `wakeTime < nextTime` clamp present
- [ ] `libs/async.h` is included

## Semantic Verification Checklist (Mandatory)
- [ ] `Async_process()` is called BEFORE the time check (not after) — this is critical for servicing work that's already due
- [ ] Sleep interval is bounded by BOTH wake time AND next async event
- [ ] Function returns immediately when `wakeTime <= now` (no unnecessary sleep)
- [ ] Loop continues after each sub-interval sleep (not single return)
- [ ] Implementation matches legacy `thrcommon.c:333-362` exactly
- [ ] Behavioral validation is executed, not replaced by grep/build-only review
- [ ] No Rust code modified
- [ ] All Rust tests pass
- [ ] C project compiles cleanly

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder" sc2/src/libs/threads/rust_thrcommon.c
```

No deferred patterns should exist in the modified function.

## Success Criteria
- [ ] `SleepThreadUntil` pumps async queue matching legacy behavior
- [ ] Behavioral validation demonstrates repeated pumping/clamping semantics
- [ ] C project builds without warnings
- [ ] All Rust tests pass (no Rust changes)
- [ ] Async callbacks will be serviced during main-thread sleep

## Failure Recovery
- rollback: `git checkout -- sc2/src/libs/threads/rust_thrcommon.c`
- blocking: if `Async_process` or `Async_timeBeforeNextMs` are not accessible, verify include paths and linkage

## Phase Completion Marker
Create: `project-plans/20260311/threading/.completed/P06.md` with phase ID, timestamp, files changed, verification commands run, outputs summary, semantic findings, and follow-up notes.
