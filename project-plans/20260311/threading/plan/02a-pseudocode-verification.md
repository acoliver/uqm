# Phase 02a: Pseudocode Verification

## Phase ID
`PLAN-20260314-THREADING.P02a`

## Prerequisites
- Required: Phase 02 (Pseudocode) completed
- Expected previous artifact: `project-plans/20260311/threading/.completed/P02.md`

## Verification Checklist

### Pseudocode completeness
- [ ] G1 spawn path (lines 01-10): covers closure return type change, func return capture
- [ ] G1 FFI spawn (lines 11-18): covers Box::into_raw with Thread<c_int>
- [ ] G1 join path (lines 28-41): covers null checks, out_status writes, success/failure
- [ ] G1 C adapter (lines 42-52): covers WaitThread with new out_status parameter
- [ ] G1 header (line 53): covers rust_threads.h declaration
- [ ] G1 local extern (line 54): covers rust_thrcommon.c local extern declaration update
- [ ] G2 async pump (lines 55-66): covers Async_process loop, time conversion, min sleep
- [ ] G3 StartThread decision (lines 67-79): explicitly keeps `rust_thread_spawn` with synchronous failure cleanup
- [ ] G4 detached helper scope note (lines 80-93): documents `match` cleanup style without claiming detached-failure contract closure

### Validation points present
- [ ] Null pointer checks in rust_thread_join (lines 29-31)
- [ ] Error/success branching in join (lines 34-40)
- [ ] Time bounds check in SleepThreadUntil (line 59)
- [ ] Clamp to wake time in SleepThreadUntil (line 63)
- [ ] StartThread failure cleanup in synchronous spawn path (lines 72-75)

### Error handling covered
- [ ] spawn failure → NULL return (line 17)
- [ ] join on null → return 0 (lines 29-31)
- [ ] join failure → write 0 to out_status (lines 39-40)
- [ ] detached helper failure explicitly documented as unresolved for adapter-owned cleanup under current ABI (lines 86-91)

### Ordering constraints
- [ ] ThreadLocalGuard attached BEFORE func call (line 05)
- [ ] Async_process called BEFORE time check (lines 57-59)
- [ ] t->native set to NULL AFTER join completes (line 51)

### Integration boundaries
- [ ] Pseudocode references real function names from mod.rs and rust_thrcommon.c
- [ ] Time conversion formula matches legacy (line 60-61)
- [ ] All pseudocode lines are referenced in the traceability summary

### G3/G4 decisions documented
- [ ] Analysis shows ProcessThreadLifecycles needs `rust_thread_join` → needs native handle
- [ ] Spec §2.5 reference design is distinguished from normative requirements
- [ ] Detached-thread creation failure mismatch is explicitly documented, not falsely marked solved

## Gate Decision
- [ ] PASS: proceed to Phase 03
- [ ] FAIL: revise pseudocode (list issues)

## Phase Completion Marker
Create: `project-plans/20260311/threading/.completed/P02a.md` with phase ID, timestamp, files changed, verification commands run, outputs summary, semantic findings, and follow-up notes.
