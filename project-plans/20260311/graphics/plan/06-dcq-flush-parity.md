# Phase 06: DCQ Flush + Queue Semantics Parity

## Phase ID
`PLAN-20260314-GRAPHICS.P06`

## Prerequisites
- Required: Phase P05 completed
- Verify: All 16 command types pushable
- Expected files from previous phase: Modified `dcq_ffi.rs`, `dcqueue.rs`

## Requirements Implemented (Expanded)

### REQ-DQ-003: Batch visibility
**Requirement text**: When draw batching is active, the subsystem shall defer visibility of batched commands until the corresponding batch scope is exited.

Behavior contract:
- GIVEN: Batching is active
- WHEN: Commands are pushed and flush is called before unbatch
- THEN: Those commands are not visible to the consumer/flush path yet

### REQ-DQ-004: Nested batching
**Requirement text**: When nested batching scopes are used, the subsystem shall not expose queued commands for execution until the outermost active batch scope is exited.

Behavior contract:
- GIVEN: Nested batch scopes are active
- WHEN: The inner scope exits but the outer scope remains active
- THEN: Previously queued commands are still not visible
- THEN: Visibility is restored only when the outermost scope exits

### REQ-DQ-006: Flush completion signal
**Requirement text**: When a flush cycle completes, the subsystem shall notify waiting integration code through the established synchronization mechanism.

Behavior contract:
- GIVEN: Game threads are blocked waiting on the rendering condition variable
- WHEN: `process_commands()` finishes dispatching all queued commands
- THEN: The rendering condition variable is broadcast, unblocking waiting threads

### REQ-DQ-007: Empty-queue fade handling
**Requirement text**: When the queue is empty and a visible fade or transition update is required, the subsystem shall execute the presentation path without requiring a synthetic draw command.

Behavior contract:
- GIVEN: The DCQ is empty but a fade or transition is active
- WHEN: Flush is called
- THEN: `swap_buffers` is called with REDRAW_FADING flag to update the display

### REQ-RL-009: Idle behavior
**Requirement text**: When no draw commands are pending and no visible fade or transition update is required, the subsystem shall return from the flush/present cycle without altering visible output.

Behavior contract:
- GIVEN: The DCQ is empty and no fade/transition update is active
- WHEN: Flush is called
- THEN: The function returns without calling the presentation path and without changing visible output

### REQ-DQ-008: Queue backpressure
**Requirement text**: The subsystem shall apply queue backpressure sufficient to guarantee forward progress.

Behavior contract:
- GIVEN: The flush loop is processing commands
- WHEN: Producer threads add commands faster than they can be processed
- THEN: The livelock detection mechanism blocks producers (not just logs)

### REQ-DQ-012: Bounding update tracking
**Requirement text**: The subsystem shall track the modified region as a bounding box that is a correct superset of all main-screen pixels modified during the flush cycle.

Behavior contract:
- GIVEN: Draw commands target `TFB_SCREEN_MAIN` during flush
- WHEN: Commands are dispatched
- THEN: A bounding box is accumulated covering all affected pixels
- THEN: The bounding box is reset after the flush cycle completes

### REQ-OWN-006: Deferred free ordering
**Requirement text**: When destruction is deferred through the draw queue, the subsystem shall not release the targeted object before all earlier queued uses of that object have completed.

Behavior contract:
- GIVEN: Commands enqueue draw/copy work against an image or data pointer, followed by deletion of that same object
- WHEN: Flush executes
- THEN: all earlier queued uses complete before deletion occurs

### REQ-OWN-007: Image synchronization obligations
**Requirement text**: When externally visible image metadata may be observed concurrently with rendering activity, the subsystem shall preserve the locking or equivalent synchronization guarantees required by the existing ABI contract.

Behavior contract:
- GIVEN: Rendering activity and metadata access can overlap at the ABI boundary
- WHEN: image metadata is observed or mutated
- THEN: per-image mutex or equivalent synchronization semantics are preserved

## Implementation Tasks

### Task 1: Add BoundingBox tracking to DrawCommandQueue

#### File: `rust/src/graphics/dcqueue.rs`
- Add `BoundingBox` struct with `min_x, min_y, max_x, max_y, valid` fields
- Add `expand(x, y, w, h)` and `reset()` methods
- Add `flush_bbox` field to `DrawCommandQueue`
- In `handle_command`, for commands targeting `Screen::Main`, expand bbox by affected region
- After flush loop, reset bbox
- marker: `@plan PLAN-20260314-GRAPHICS.P06`
- marker: `@requirement REQ-DQ-012`

### Task 2: Fix livelock detection to actually block producers

#### File: `rust/src/graphics/dcqueue.rs`
- Current behavior: livelock counter increments and logs but never breaks the loop
- Required: When livelock threshold is exceeded, acquire a lock or equivalent gate that blocks producer `push()` calls until flush completes
- Use an approach consistent with the existing queue locking model; do not introduce busy-wait semantics unless the current codebase already uses them and they are demonstrably acceptable
- marker: `@requirement REQ-DQ-008`

### Task 3: Add flush completion signaling

#### File: `rust/src/graphics/dcqueue.rs`
- Drive the established rendering completion synchronization primitive used by UQM
- After flush loop completes: broadcast the condition variable / equivalent primitive
- marker: `@requirement REQ-DQ-006, REQ-INT-009`

### Task 4: Add explicit empty-queue split for redraw-vs-idle behavior

#### File: `rust/src/graphics/dcqueue.rs`
- At the top of `process_commands()`: if queue is empty, first check whether a visible fade/transition update is active
- If active: call swap_buffers equivalent with REDRAW_FADING
- If not active: return early without calling the presentation path
- This task must explicitly preserve both sides of the contract: REQ-DQ-007 / REQ-RL-008 redraw continuity and REQ-RL-009 idle/no-redraw behavior
- marker: `@requirement REQ-DQ-007, REQ-RL-008, REQ-RL-009`

### Task 5: Validate and, if needed, repair batch visibility semantics

#### File: `rust/src/graphics/dcqueue.rs`
- Audit current `batch`/`unbatch` implementation and queue-visibility rules
- If already correct, document the exact code path and add tests that prove it
- If incorrect or incomplete, repair it so commands remain hidden until batch depth returns to zero
- marker: `@requirement REQ-DQ-003, REQ-DQ-004`

### Task 6: Validate deferred free ordering

#### File: `rust/src/graphics/dcqueue.rs`
- Add ordering-focused tests around `DeleteImage` / `DeleteData`
- If the implementation relies on FIFO alone, prove that destruction happens only after all earlier queued uses have fully completed in the same flush path
- If any helper path can release resources early, fix it
- marker: `@requirement REQ-OWN-006, REQ-DQ-010`

### Task 7: Validate image synchronization obligations at the real ABI boundary

#### Files: `rust/src/graphics/dcqueue.rs` plus the actual image lifecycle / metadata access layer
- Identify where per-image mutex or equivalent synchronization is enforced today
- If the Rust migration path bypasses or weakens that contract, repair it
- Add tests or documented invariants proving metadata access remains synchronized relative to rendering activity
- marker: `@requirement REQ-OWN-007`

### Task 8: Wire flush signal in the actual bridge path

#### File: `rust/src/graphics/dcq_ffi.rs` or the real C/Rust synchronization bridge
- Ensure `rust_dcq_flush()` does not merely return after `process_commands()`; it must also preserve the externally visible completion signal semantics on the migrated path
- marker: `@requirement REQ-DQ-006, REQ-INT-009`

### Pseudocode traceability
- Uses pseudocode lines: PC-06, lines 140-178

## TDD Test Plan

### Tests to add in `dcqueue.rs`

1. `test_bbox_initial_state` — new bbox is invalid (no region)
2. `test_bbox_expand_single` — expand once → valid bbox matches region
3. `test_bbox_expand_union` — expand twice → bbox is union of both regions
4. `test_bbox_reset` — expand then reset → invalid again
5. `test_bbox_tracks_main_screen_only` — push commands to Main and Extra, verify bbox only covers Main
6. `test_flush_resets_bbox` — push commands, flush, verify bbox reset
7. `test_livelock_blocks_producers` — integration-style test proving backpressure is applied
8. `test_empty_queue_no_crash` — empty queue flush → no-op, no crash
9. `test_empty_queue_with_fade_triggers_redraw` — empty queue + active fade/transition redraws through presentation path
10. `test_empty_queue_without_fade_returns_without_present` — empty queue + no active fade/transition returns early and leaves visible output unchanged
11. `test_batch_hides_commands_until_unbatch` — flush while batched does not expose commands
12. `test_nested_batch_hides_until_outermost_unbatch` — inner unbatch still keeps commands hidden
13. `test_delete_image_occurs_after_prior_uses` — prior queued image uses complete before deletion
14. `test_delete_data_occurs_after_prior_uses` — prior queued data-dependent uses complete before free
15. `test_image_metadata_access_uses_required_sync` — verify mutex/locking contract or equivalent invariant

### Tests to add in `dcq_ffi.rs` or bridge layer

16. `test_flush_completion_signal` — init, push commands, flush, verify completion signal observed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `BoundingBox` struct defined with expand/reset
- [ ] `flush_bbox` field in `DrawCommandQueue`
- [ ] Livelock detection blocks producers (not just logs)
- [ ] Flush completion signaling mechanism present
- [ ] Empty-queue check explicitly distinguishes redraw-required and idle/no-redraw branches
- [ ] Batch-depth / visibility behavior is explicitly documented in code or repaired
- [ ] Deferred free ordering tests exist
- [ ] Image synchronization boundary is explicitly documented/tested
- [ ] Plan/requirement traceability markers present

## Semantic Verification Checklist (Mandatory)
- [ ] Bounding box correctly accumulates Main screen modifications
- [ ] Bounding box ignores Extra/Transition screen modifications
- [ ] Bounding box resets after flush
- [ ] Livelock counter triggers actual producer blocking
- [ ] Flush completion unblocks waiting threads through the established integration primitive
- [ ] Empty queue + active fade/transition triggers redraw continuity
- [ ] Empty queue + no active fade/transition preserves idle/no-redraw behavior and does not alter visible output
- [ ] Batched commands are invisible until unbatch exits the batch scope
- [ ] Nested batching preserves invisibility until the outermost unbatch
- [ ] Deferred destruction occurs only after all prior queued uses complete
- [ ] Image metadata synchronization obligations remain intact
- [ ] All existing DCQ tests pass
- [ ] P09/P10 revalidate the same semantics again through the actual C→Rust bridge path

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/dcqueue.rs rust/src/graphics/dcq_ffi.rs
```

## Success Criteria
- [ ] REQ-DQ-003, REQ-DQ-004, REQ-DQ-006, REQ-DQ-007, REQ-DQ-008, REQ-DQ-012 behavior demonstrated
- [ ] REQ-RL-009 idle/no-redraw behavior demonstrated as a distinct obligation
- [ ] REQ-OWN-006 and REQ-OWN-007 obligations demonstrated or explicitly repaired
- [ ] 16+ new tests pass
- [ ] Verification commands pass

## Failure Recovery
- Rollback: `git checkout -- rust/src/graphics/dcqueue.rs rust/src/graphics/dcq_ffi.rs`
- Blocking: Completion-signal integration requires the real synchronization primitive to be accessible from tests

## Phase Completion Marker
Create: `project-plans/20260311/graphics/.completed/P06.md`
