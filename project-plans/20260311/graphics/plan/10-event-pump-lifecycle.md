# Phase 10: Event-Pump Lifecycle + Forwarding Revalidation

## Phase ID
`PLAN-20260314-GRAPHICS.P10`

## Prerequisites
- Required: Phase P09 completed
- Verify: C bridge wiring builds and links
- Expected: Reinit/lifecycle helper boundary identified in P07 and live event-processing path confirmed in P09

## Requirements Implemented (Expanded)

### REQ-INT-001: Existing API compatibility
**Requirement text**: The subsystem shall preserve the externally visible behavior required by the existing UQM graphics API surface.

Behavior contract:
- GIVEN: SDL events are available to the graphics backend
- WHEN: the Rust graphics path processes events
- THEN: event collection/forwarding behavior remains compatible with the established external contract

### REQ-INT-002: Backend-vtable / backend lifecycle compatibility
The graphics subsystem owns SDL event collection and event-pump execution. This phase verifies that ownership remains behaviorally correct across init, reinit, and uninit on the migrated path.

### REQ-RL-001 / REQ-RL-011
This phase verifies lifecycle-sensitive event-pump state preservation through initialization and reinitialization.

### REQ-ERR-002 / REQ-ERR-004
This phase verifies safe behavior and return-value compatibility when event processing is called before init or after uninit.

## Implementation Tasks

### Task 1: Identify and document the concrete event-pump owner and forwarding entry point

#### Files: `rust/src/graphics/ffi.rs` plus the driver-state owner identified in P07
- Record the exact state field(s) that hold SDL event-pump ownership
- Record where they are initialized, replaced during reinit, and destroyed during uninit
- Record the exact externally callable entry point(s) that poll and forward events

This task is not optional. The rest of this phase must cite the concrete owner/entry path, not an inferred one.

### Task 2: Verify normal init → process_events behavior

#### Files: `rust/src/graphics/ffi.rs` tests and/or migrated integration test harness
- Exercise the initialized event-processing path
- Confirm SDL events are polled in order
- Confirm forwarded output preserves the established external contract
- Confirm no spurious drops or reorderings are introduced by the migrated path

### Task 3: Verify reinit replaces or preserves event-pump state correctly

#### Files: `rust/src/graphics/ffi.rs`, `rust/src/graphics/dcqueue.rs`, migrated integration harness
- Trigger `ReinitVideo` through the actual migrated path
- Verify the event-processing entry point remains valid afterward
- Verify event collection/forwarding still works after reinit
- Verify old event-pump state is not reused after teardown if the backend contract requires replacement

### Task 4: Verify uninit / pre-init safety contract for event processing

#### Files: `rust/src/graphics/ffi.rs`
- Call event processing before initialization and after shutdown
- Verify safe failure / no-op behavior matches the established return convention
- Verify no invalid dereference or stale state use occurs

### Task 5: Feed results forward into later verification

Update the plan artifacts/checklists so P12 final verification includes explicit event-lifecycle checks, not just general runtime smoke confidence.

## TDD Test Plan

### Tests to add

1. `test_process_events_before_init_safe` — verifies pre-init safe behavior
2. `test_process_events_after_init_forwarding_contract` — verifies normal initialized event forwarding
3. `test_process_events_after_reinit_still_works` — verifies event path after `ReinitVideo`
4. `test_process_events_after_uninit_safe` — verifies post-uninit safe behavior
5. `test_process_events_order_preserved` — verifies ordered forwarding for multiple queued SDL events

If direct SDL event injection is difficult in a unit test, use the narrowest realistic integration harness that still exercises the actual migrated call path.

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] Concrete event-pump owner/state field identified
- [ ] Concrete externally callable event-processing entry point identified
- [ ] Init, reinit, and uninit ownership transitions documented against real code
- [ ] Tests exist for pre-init, post-init, post-reinit, and post-uninit event processing

## Semantic Verification Checklist (Mandatory)
- [ ] Initialized event processing polls/forwards events through the established contract
- [ ] Event ordering is preserved
- [ ] No dropped events are introduced by the migrated path in the verified scenarios
- [ ] Post-reinit event processing still functions correctly
- [ ] Pre-init and post-uninit calls fail safely / no-op per the established contract
- [ ] Findings are propagated into final end-to-end verification scope

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/ffi.rs rust/src/graphics/ --include="*.rs" | grep -i "event\|pump"
```

## Success Criteria
- [ ] Event-pump lifecycle and forwarding are explicitly verified on the migrated path
- [ ] Reinit does not silently break event processing
- [ ] Safe failure/no-op behavior is verified before init and after uninit
- [ ] Verification commands pass

## Failure Recovery
- Rollback: `git checkout -- rust/src/graphics/ffi.rs rust/src/graphics/dcqueue.rs rust/tests/`

## Phase Completion Marker
Create: `project-plans/20260311/graphics/.completed/P10.md`
