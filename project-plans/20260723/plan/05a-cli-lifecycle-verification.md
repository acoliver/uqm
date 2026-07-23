# Phase 05a: Verify CLI and Lifecycle

Phase ID: `PLAN-20260723-RUNTIME-AUTOMATION.P05.VERIFY`

Require P04 marker/P05 evidence. Independently test `REQ-MODE-001..003`, `REQ-BUILD-001`, `REQ-EXIT-006/008/009`, finalization. Run focused lifecycle/process-status tests and strict gates.

Require ordered evidence for success and every failure class:

```text
setup -> C init -> game -> run_end attempt -> automation output closed
-> teardown_subsystems returned -> teardown_complete -> status mapping
```

Mutation checks must fail when inactive direct callback allocates/touches TLS/locks/does external work, active gate is entered inactive, finalization does not clear capture/gate or drain active shells/reservations, a cancelled sequence blocks, run_end duplicates/precedes prior records, any output/I/O lock remains during teardown, active receipt is before teardown, inactive receipt is emitted here, unsupported build enters game, incomplete CLI enters game, terminal outer guard allows retry, or failure maps zero.

FAIL if P05 edits C input/menu/graphics, claims their later requirements, uses callback process exit, loses current options/user work, emits false teardown marker, lacks panic/finalization tests, or strict gate fails.

On PASS emit `Phase 05: PASS`, update tracker, create `.completed/P05.md` with lifecycle order evidence and deferred P06/P07 integrations. Otherwise no marker.
