# Phase 04: ProcessCollisions

## Phase ID
`PLAN-20260320-BATTLEPT2.P04`

## Prerequisites
- Required: Phase 03a (Process Loop PrePost Verification) completed with PASS
- Expected files: `rust/src/battle/process_loop.rs` with P03 functions
- Expected artifacts: P03 pre_process/post_process/untarget verified

## Requirements Implemented (Expanded)

### REQ: Collision detection (battle/requirements.md §Collision detection)
**Requirement text**: Collision detection shall use pixel-accurate intersection testing between element trajectories within a single frame. Detection in preprocess pass checks each element only against successors (forward iteration). Newly-added elements in postprocess check against entire list from head.

Behavior contract:
- GIVEN: Two collidable elements with overlapping trajectories
- WHEN: ProcessCollisions checks the current element against its successors
- THEN: DrawablesIntersect is called and returns the collision time point. Return value semantics: 0 = no intersection; 1 = special "stuck overlap" sentinel (elements are overlapping at max time, NOT a valid collision time); >1 = collision time value.

### REQ: Collision dispatch ordering (battle/requirements.md §Collision dispatch)
**Requirement text**: If the test element (found by forward iteration) is a PLAYER_SHIP, the test element's collision_func is called first. Otherwise, the current element's collision_func is called first.

Behavior contract:
- GIVEN: A collision between element A (current) and element B (test/successor)
- WHEN: B has PLAYER_SHIP flag
- THEN: B.collision_func is called first, then A.collision_func

- GIVEN: A collision between element A (current) and element B (test/successor)
- WHEN: B does NOT have PLAYER_SHIP flag
- THEN: A.collision_func is called first, then B.collision_func

### REQ: Stuck object handling (battle/requirements.md §Stuck object handling)
**Requirement text**: When two elements are intersecting at maximum time with identical frames (stuck overlap): APPEARING elements are destroyed via do_damage(hit_points), untargeted, marked COLLISION|DISAPPEARING, and death_func invoked; non-APPEARING elements undergo a multi-step resolution sequence (frame normalization, intersect reinit, ship facing update), falling back to time_val=0/break when still stuck.

Behavior contract:
- GIVEN: Two overlapping elements where DrawablesIntersect returns sentinel `1` (stuck overlap — not a normal collision time), one or both APPEARING, both frames identical (NextFrame == CurFrame for both)
- WHEN: ProcessCollisions detects stuck overlap via the sentinel `1` return value (process.c:397-398)
- THEN: For each APPEARING element: call do_damage(element, element.hit_points), untarget if has pParent, set state_flags |= (COLLISION | DISAPPEARING), invoke death_func if present. If the current element is APPEARING, return COLLISION immediately. For non-APPEARING stuck pairs with identical frames, force time_val = 0 (process.c:451).

- GIVEN: Two overlapping non-APPEARING elements where DrawablesIntersect returns sentinel `1` with differing current/next frames
- WHEN: ProcessCollisions enters the sentinel resolution while-loop (process.c:397-516)
- THEN: Multi-step resolution: (1) If the current element already has COLLISION set, try alternate intersection: reinit test element's end point, set stamp origin to end point, re-call DrawablesIntersect with min_time=1, reinit start point (process.c:405-413). (2) If time_val still == sentinel `1`: check frame identity. (2a) If frames ARE identical (NextFrame == CurFrame for both): destroy APPEARING elements (do_damage, untarget, COLLISION|DISAPPEARING, death_func); for non-APPEARING elements force time_val = 0 (process.c:451). (2b) If frames DIFFER: normalize frames via SetEquFrameIndex (or copy current.image + clamp life_span to NORMAL_LIFE), reinitialize intersect start/end/frame for both, update ShipFacing if PLAYER_SHIP; then the while loop retries DrawablesIntersect from the top. (3) The while loop terminates when DrawablesIntersect returns non-sentinel OR when time_val has been forced to 0. (4) After loop exit, if time_val == 0: reinitialize end points for both elements and break out of the collision check for this pair (process.c:509-515). This is the ultimate fallback for irrecoverably stuck elements.


### REQ: Post-collision position snapping (battle/requirements.md §Post-collision position and physics)
**Requirement text**: When COLLISION flag is newly set (not already set before dispatch), snap next.location to collision point.

Behavior contract:
- GIVEN: A collision dispatched at a time point with a specific intersection location
- WHEN: Collision handlers have been called and COLLISION flag is newly set (was not set in pre-dispatch state_flags)
- THEN: element.IntersectControl.IntersectStamp.origin is set to the saved collision point, element.next.location is set to DISPLAY_TO_WORLD of that point, and InitIntersectEndPoint is called. If the element already had COLLISION set before dispatch, position snapping is skipped (process.c:572-583, 586-596).

### REQ: Post-bounce collision rechecks (battle/requirements.md §Post-bounce collision rechecks)
**Requirement text**: After elastic_collide alters velocity, recheck both elements against the entire display list from the head.

Behavior contract:
- GIVEN: Two non-FINITE_LIFE elements that just collided
- WHEN: elastic_collide() updates their velocities
- THEN: ProcessCollisions is re-called from head for BOTH elements independently

### REQ: Recursive earlier-time checks (battle/requirements.md §Collision detection, deeper-collision)
**Requirement text**: Before dispatching a collision, verify whether either element would intersect something earlier in time.

Behavior contract:
- GIVEN: Elements A and B colliding at time T
- WHEN: ProcessCollisions checks for earlier intersections
- THEN: If A intersects C at time T' < T, the A-C collision is dispatched first

## Implementation Tasks

### Files to modify

- `rust/src/battle/process_loop.rs` — Add ProcessCollisions
  - marker: `@plan PLAN-20260320-BATTLEPT2.P04`
  - marker: `@requirement REQ-COLLISION-DETECTION, REQ-COLLISION-DISPATCH, REQ-STUCK-OVERLAP`
  - Contents to add:
    - `pub fn process_collisions(current_handle: ElementHandle, test_handle: ElementHandle) -> CollisionResult` — Full recursive collision orchestration matching process.c:362-628
    - Private `collision_bridge(a: &IntersectControl, b: &IntersectControl) -> Option<CollisionPoint>` — Bridge to C DrawablesIntersect (temporary; moves to c_bridge.rs in P06)
    - Recursive structure:
      1. Walk successors from test_handle
      2. For each successor: check eligibility via Phase 1 collision_possible()
      3. **APPEARING+FINITE_LIFE prefilter** (process.c:389-394): Before calling DrawablesIntersect, check if `(state_flags | test_state_flags) & FINITE_LIFE` AND either element has `APPEARING` with `life_span > 1`. If so, set `time_val = 0` and skip the DrawablesIntersect loop entirely. This prevents newly-appearing finite-life elements from registering collisions before they've fully materialized.
      4. Call DrawablesIntersect via collision_bridge. Return value `1` is a special **sentinel** meaning "stuck overlap" — not a valid collision time. The while loop continues as long as DrawablesIntersect returns sentinel `1` AND neither element has FINITE_LIFE.
      5. Handle stuck overlap (sentinel `1` + identical frames): APPEARING → do_damage/untarget/COLLISION|DISAPPEARING/death_func; non-APPEARING → frame normalization (SetEquFrameIndex or copy+clamp), intersect reinit, ship facing update. If DrawablesIntersect still returns sentinel `1` after normalization/reinit AND frames were identical (couldn't normalize), forces time_val=0 and breaks (process.c:509-515).
      5. Check for earlier-time collisions recursively
      6. Dispatch collision handlers in PLAYER_SHIP-aware order
      7. Set COLLISION flag on both elements
      8. Snap next.location to collision point
      9. For non-FINITE_LIFE pairs: call Phase 1 elastic_collide()
      10. Post-bounce: re-call ProcessCollisions from head for both elements
    - COLLISION flag as alternate-check guard: elements with COLLISION set are NOT globally skipped; instead, when `state_flags & COLLISION`, an alternate intersection check is performed: reinitialize test element's end point, set intersect stamp origin to end point, re-call DrawablesIntersect with min_time=1, then reinitialize start point (process.c:405-413). The recursive earlier-time check also uses COLLISION as a skip guard for that specific recursive sub-call only (process.c:531-540).
    - PreProcess called on unprocessed elements encountered during successor walk

### C reference functions ported

| C Function | C File | C Lines | Rust Function | Rust Module |
|-----------|--------|---------|---------------|-------------|
| `ProcessCollisions()` | process.c | :362-628 | `process_collisions()` | `process_loop.rs` |

### C branches to handle

| Branch | Source Sites | Handling |
|--------|-------------|----------|
| `DEBUG_PROCESS` | process.c:401-404, 523-526, 545-548 | Debug logging with distinct semantic categories: (1) **Stuck overlap warning** (L401-404): "BAD NEWS" — logged when DrawablesIntersect returns sentinel `1` indicating two elements are overlapping at max time, triggering normalization; (2) **Collision candidate** (L523-526): logged when a valid collision time > 0 is found between two elements (pre-dispatch); (3) **Collision dispatch** (L545-548): "PROCESSING" — logged when collision handlers are actually being invoked after recursive earlier-time checks pass. Rust equivalent: `tracing::debug!()` behind a feature flag (e.g., `debug-process`), with distinct message prefixes for each category |

### Integration points
- Phase 1 `collision.rs`: collision_possible(), elastic_collide()
- Phase 1 `element.rs`: Element struct, ElementFlags, is_collidable()
- Phase 1 `display_list.rs`: iteration, handles
- P03 `process_loop.rs`: pre_process() called on unprocessed elements
- C FFI (temporary): DrawablesIntersect — private bridge function until P06

### Pseudocode traceability (if impl phase)
- Uses pseudocode lines from `analysis/pseudocode/process-collisions.md`: full ProcessCollisions

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `process_collisions()` implemented in `process_loop.rs`
- [ ] `collision_bridge()` private helper present (will move to c_bridge.rs in P06)
- [ ] Plan/requirement traceability markers present
- [ ] Tests compile and run
- [ ] No new module files created (all in existing process_loop.rs)

## Semantic Verification Checklist (Mandatory)
- [ ] Recursive structure matches process.c:362-628
- [ ] PLAYER_SHIP dispatch ordering verified: test element PLAYER_SHIP → test first
- [ ] COLLISION flag set on both elements after dispatch
- [ ] Stuck overlap APPEARING: do_damage(hit_points), untarget, COLLISION|DISAPPEARING, death_func invoked
- [ ] Position snapping: conditional on COLLISION being newly set (not already set before dispatch); snaps origin + next.location + reinit end point
- [ ] Post-bounce: elastic_collide → re-call from head for both elements
- [ ] Recursive earlier-time: both elements checked against earlier list before dispatch
- [ ] PreProcess called on unprocessed elements during successor walk
- [ ] APPEARING+FINITE_LIFE prefilter: time_val forced to 0 when (FINITE_LIFE on either) AND (APPEARING with life_span > 1 on either), skipping DrawablesIntersect loop entirely
- [ ] DrawablesIntersect sentinel: return value `1` is a stuck-overlap sentinel, not a valid collision time; drives the while loop for normalization/reinit retry
- [ ] Stuck overlap non-APPEARING: frame normalization (SetEquFrameIndex or copy image + clamp life_span), intersect reinit, ship facing update. If STILL sentinel `1` after normalization with identical frames → force time_val=0 and break (process.c:509-515)
- [ ] COLLISION flag triggers alternate intersection check (reinit end point, recheck with min_time=1) rather than global skip
- [ ] Handle-based traversal — no mutable borrows survive across callbacks
- [ ] No placeholder/deferred implementation patterns remain

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/process_loop.rs
```

## Success Criteria
- [ ] ProcessCollisions fully implemented and tested
- [ ] Recursive collision behavior matches C exactly
- [ ] All Phase 1 tests (2,151) still pass
- [ ] Verification commands pass
- [ ] Semantic checks pass

## Failure Recovery
- rollback: `git checkout -- rust/src/battle/process_loop.rs`
- blocking issues: DrawablesIntersect FFI wrapper complexity, recursive borrow conflicts

## Phase Completion Marker
Create: `project-plans/20260311/battlept2/.completed/P04.md`

Contents:
- phase ID: PLAN-20260320-BATTLEPT2.P04
- timestamp
- files changed: process_loop.rs
- tests added/updated
- verification outputs
- semantic verification summary
