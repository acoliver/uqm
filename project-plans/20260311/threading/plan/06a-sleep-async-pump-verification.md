# Phase 06a: SleepThreadUntil Async Pumping — Verification

## Phase ID
`PLAN-20260314-THREADING.P06a`

## Prerequisites
- Required: Phase 06 completed
- Expected previous artifact: `project-plans/20260311/threading/.completed/P06.md`

## Code Review

```bash
# View the updated function
grep -A 25 "SleepThreadUntil" /Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c
```

- [ ] Function contains `Async_process()` call
- [ ] Function contains `Async_timeBeforeNextMs()` call
- [ ] Function has `for (;;)` loop structure
- [ ] `Async_process()` is called BEFORE `GetTimeCounter()` check
- [ ] Early return on `wakeTime <= now`
- [ ] Sleep interval clamped to min(wakeTime, nextAsyncTime)
- [ ] `SleepThread(nextTime - now)` for sub-interval sleep

## C Build Gate

```bash
cd /Users/acoliver/projects/uqm/sc2
make -f Makefile.build
```
- [ ] Compiles without errors
- [ ] No warnings in `rust_thrcommon.c`

## Rust Test Gate (No Regressions)

```bash
cd /Users/acoliver/projects/uqm/rust
cargo test --workspace --all-features 2>&1
```
- [ ] All tests pass (no Rust files changed)

## Behavioral Validation Gate

One of the following must be completed and captured in the completion artifact:
- [ ] Automated harness/test demonstrates repeated `Async_process()` pumping and wake-time clamping behavior
- [ ] Manual validation demonstrates repeated `Async_process()` pumping and wake-time clamping behavior

Minimum facts that must be evidenced:
- [ ] `Async_process()` runs before the first sleep interval
- [ ] `SleepThreadUntil` performs multiple loop iterations when async work is due before `wakeTime`
- [ ] The effective sleep interval is clamped to the earlier async deadline rather than always sleeping until final `wakeTime`
- [ ] `wakeTime <= now` returns without sleeping

## Legacy Comparison

Compare new implementation against legacy `thrcommon.c:333-362`:
- [ ] Same loop structure
- [ ] Same `Async_process()` call placement (before time check)
- [ ] Same `Async_timeBeforeNextMs()` usage
- [ ] Same time conversion formula
- [ ] Same clamp logic (`wakeTime < nextTime`)

## Gate Decision
- [ ] PASS: proceed to Phase 07
- [ ] FAIL: fix issues before proceeding

## Phase Completion Marker
Create: `project-plans/20260311/threading/.completed/P06a.md` with phase ID, timestamp, files changed, verification commands run, outputs summary, semantic findings, and follow-up notes.
