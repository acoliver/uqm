# Phase 02a: Verify Pure Scheduler and Watchdog

Phase ID: `PLAN-20260723-RUNTIME-AUTOMATION.P02.VERIFY`

Require P01 marker and P02 handoff. Independently verify `REQ-SCHED-001..003`, `REQ-DET-001`, `REQ-WATCH-001..003` against execution-contract §2 and current pseudocode 002/003.

Run focused tests, proptest with recorded seed on failure, and all strict gates. Require explicit tables/timelines for max=1 and max=3 from both callback kinds, simultaneous limits, applicable overflow, terminal no-increment, and ordinary boundary update after release. Mutation checks must fail when post-increment moves after comparison, exact-limit becomes `>`, max M admits M callbacks, tap release is delayed, settle is off by one, state advances before matching commit, capture generation is zero/stale/duplicate/arms twice, priority changes, checked add wraps, or semantic event accepts wrong target.

FAIL if the phase contains C/SDL/filesystem/global runtime/activity/FFI/lifecycle integration or claims those later requirements. FAIL on untested table row, flaky wall clock, arbitrary sleep, placeholder, unsafe, user-edit loss, or any gate failure.

On PASS emit `Phase 02: PASS`, update tracker, create `.completed/P02.md` with table coverage, property/mutation evidence, commands/exits, file scope, and preservation result. Otherwise no marker.
