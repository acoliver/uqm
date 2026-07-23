# Phase 02: Pure Scheduler and Watchdog Reducers

Phase ID: `PLAN-20260723-RUNTIME-AUTOMATION.P02`

## Prerequisite/ownership

Require `.completed/P01.md`. Own `REQ-SCHED-001..003`, `REQ-DET-001`, `REQ-WATCH-001..003`, and pure property-test portion of `REQ-TEST-001`. This phase is pure: no C setter/getter, global runtime, activity shutdown, FFI export, trace writer, filesystem, SDL, capture, or lifecycle integration.

## Files

Create `rust/src/automation/{scheduler,watchdog}.rs`; modify module/error/script only for typed integration.

## Normative reducers

Implement execution-contract §2 exactly. Applicable counter checked-add/store precedes comparison; equality is terminal and performs no scheduler work. `M` admits at most `M-1`; max=3 timeline must be 1/admit, 2/admit, 3/timeout. Applicable overflow is typed; then priority is input >= max, presentation >= max, wall >= max, clock regression. Terminal callbacks do not increment.

Implement every scheduler table row, including zero-wait chaining, exact admitted Hold/ReleasePending/Settle semantics, and the fact that a boundary C callback performs its ordinary update only after terminal release. Reducer output is typed proposed state plus `EffectPlan`; advancement is represented only by matching sequence/state-version commit. Model checked nonzero atomic capture generation/request metadata, one arm, WaitingCapture, and stale/duplicate/zero/future generation rejection. This remains pure: model atomics/transactions as inputs/outputs, not globals or side effects.

## TDD

1. Table-driven test every scheduler row and no-callback action chaining.
2. Tap hold 1/many, settle 0/many, multiple input callbacks without presents, unowned ownership outputs.
3. Capture and typed main-menu assertion blocking/match/mismatch.
4. Every watchdog boundary from input and present callback kinds.
5. `proptest` arbitrary valid scripts/callback sequences: no counter wrap, one action, capture single-arm, terminal absorbing, deterministic replay after removing elapsed observations.
6. Refactor reducer functions under project complexity limits.

Commands: focused scheduler/watchdog tests then all four strict shared gates. Handoff evidence only; P02a creates marker.
