# Phase 03a: Process Loop — PreProcess + PostProcess Verification

## Phase ID
`PLAN-20260320-BATTLEPT2.P03a`

## Prerequisites
- Required: Phase 03 (Process Loop — PreProcess + PostProcess) completed
- Expected artifacts: `rust/src/battle/process_loop.rs` with 7 functions implemented, `mod.rs` updated

## Structural Verification Checklist
- [ ] `process_types.rs` no longer exists (renamed to `process_loop.rs`)
- [ ] `process_loop.rs` contains all Phase 1 type definitions (ViewState, ZoomMode, constants) plus new functions
- [ ] `mod.rs` declares `pub mod process_loop;` (not `process_types`)
- [ ] All 7 functions present: `pre_process`, `post_process`, `alloc_element`, `free_element`, `setup_element`, `untarget`, `remove_element`
- [ ] Plan/requirement traceability markers (`@plan`, `@requirement`) present
- [ ] Git history: commit 1 is rename-only, commit 2+ adds logic

## Semantic Verification Checklist (Mandatory — Most Important)

### PreProcess behavioral equivalence with C (process.c:180-270)
- [ ] **Death handling**: life_span == 0 → Untarget(handle) called → DISAPPEARING set on state_flags → death_func invoked (if non-null) → if death_func extends life and clears DISAPPEARING, element survives
- [ ] **APPEARING + PLAYER_SHIP**: APPEARING cleared in local `state_flags` copy only; element's actual `state_flags` retain APPEARING; preprocess_func IS called; intersection geometry initialized
- [ ] **APPEARING + non-PLAYER_SHIP**: preprocess_func NOT called; only intersection geometry init; element not velocity-stepped
- [ ] **CHANGING + collidable**: intersection frame reinitialized from updated image after preprocess_func callback
- [ ] **Velocity stepping**: !IGNORE_VELOCITY → Phase 1 `get_next_components()` used to compute next.location from velocity descriptor; error accumulator mutated correctly
- [ ] **FINITE_LIFE decrement**: happens AFTER velocity stepping, not before
- [ ] **Flag output**: PRE_PROCESS set, POST_PROCESS cleared, COLLISION cleared — exactly matching process.c:267-269
- [ ] **Collidable intersection init**: intersection end point set from next position converted to display coordinates

### PostProcess behavioral equivalence with C (process.c:274-285)
- [ ] Calls postprocess_func callback (if non-null)
- [ ] Copies next visual state to current via Phase 1 `commit_state()`
- [ ] Reinitializes intersection start/end points from current position
- [ ] Sets POST_PROCESS, clears PRE_PROCESS + CHANGING + APPEARING

### Asymmetric DEFY_PHYSICS clearing (spec §4.2, battle/requirements.md §Element lifecycle flag transitions)
- [ ] COLLISION set → clear COLLISION, retain DEFY_PHYSICS
- [ ] COLLISION not set → clear DEFY_PHYSICS
- [ ] Test verifies: stuck objects (both DEFY + COLLISION) retain DEFY after one pass; free-moving objects (DEFY without COLLISION) lose DEFY

### Untarget behavioral equivalence with C (process.c:93-115)
- [ ] Iterates entire display list
- [ ] For each element where hTarget == dying_handle: clears hTarget to null/zero
- [ ] Correctly handles: no elements targeting, multiple elements targeting, element targeting itself

### RemoveElement behavioral equivalence with C (process.c:118-140)
- [ ] Removes element's sound via bridge (StopElement sound)
- [ ] Calls Untarget with the element's handle
- [ ] Removes element from disp_q
- [ ] Element is deallocated (FreeElement)

### AllocElement / FreeElement behavioral equivalence with C (process.c:143-177)
- [ ] AllocElement: allocates from element pool + allocates from display primitive free list
- [ ] AllocElement: returns None when pool exhausted (no corruption)
- [ ] FreeElement: returns element to pool + returns display primitive to free list
- [ ] AllocElement + FreeElement round-trip: pool capacity preserved

### SetUpElement behavioral equivalence with C
- [ ] Initializes all fields matching C SetUpElement behavior

## Branch-Parity Verification
P03 does not directly contain compile-time branch families. No branch-parity entries to verify.

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
# Deferred implementation detection
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/process_loop.rs
```

## Pass/Fail Gate Criteria
- **PASS:** All 7 functions implemented with correct behavioral equivalence to C. Asymmetric DEFY_PHYSICS verified. All Phase 1 tests pass. No TODO/FIXME/HACK. Rename commit is clean (rename-only).
- **FAIL:** Any behavioral discrepancy with C reference (especially: APPEARING handling, DEFY_PHYSICS asymmetry, velocity stepping order, FINITE_LIFE decrement timing). Any Phase 1 test regression. Any TODO/FIXME/HACK in implementation.
