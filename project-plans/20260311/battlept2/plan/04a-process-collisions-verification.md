# Phase 04a: ProcessCollisions Verification

## Phase ID
`PLAN-20260320-BATTLEPT2.P04a`

## Prerequisites
- Required: Phase 04 (ProcessCollisions) completed
- Expected artifacts: `process_collisions()` + `collision_bridge()` in `process_loop.rs`

## Structural Verification Checklist
- [ ] `process_collisions()` is a public function in `process_loop.rs`
- [ ] `collision_bridge()` is a private helper in `process_loop.rs`
- [ ] Plan/requirement traceability markers present
- [ ] No new module files created

## Semantic Verification Checklist (Mandatory — Most Important)

### Recursive structure equivalence with C (process.c:362-628)
- [ ] **Successor walk**: iterates from test_handle forward to end of display list
- [ ] **Eligibility check**: calls Phase 1 `collision_possible()` before intersection test
- [ ] **DrawablesIntersect**: called via collision_bridge for each eligible pair
- [ ] **Recursion**: when collision found at time T, recursively checks both elements against earlier list entries for collisions at T' < T before dispatching T

### Dispatch ordering (process.c ~line 500-530)
- [ ] Test element has PLAYER_SHIP → test element's collision_func called FIRST
- [ ] Test element lacks PLAYER_SHIP → current element's collision_func called FIRST
- [ ] Both handlers receive correct element pair references

### Stuck overlap handling (process.c ~line 440-470)
- [ ] Detection: max_time_value AND same intersection frame
- [ ] APPEARING element: life_span set to 0 (immediate death)
- [ ] Non-APPEARING elements: next.location reverted to current.location for both
- [ ] Test covers: one APPEARING, both APPEARING, neither APPEARING

### Position snapping
- [ ] After collision dispatch: element.next.location = collision point for both elements
- [ ] Snap happens AFTER collision_func calls, not before

### Post-bounce rechecks
- [ ] After elastic_collide: ProcessCollisions re-called from list HEAD for element A
- [ ] After elastic_collide: ProcessCollisions re-called from list HEAD for element B
- [ ] Only applies to non-FINITE_LIFE element pairs
- [ ] FINITE_LIFE pairs: no elastic_collide, no recheck

### COLLISION flag semantics
- [ ] Set on BOTH elements after collision dispatch
- [ ] Acts as re-entry guard: elements with COLLISION already set are skipped
- [ ] Flag is cleared by PreProcess (verified in P03)

### Unprocessed element handling
- [ ] Elements encountered during successor walk that lack PRE_PROCESS flag → PreProcess is called first
- [ ] This handles elements added during the current frame's preprocess pass

### Handle-based traversal safety
- [ ] No mutable borrows held across collision_func callbacks
- [ ] Re-lookup from handle after each callback returns
- [ ] List mutations during callbacks (element add/remove) handled safely

## Branch-Parity Verification
P04 does not directly contain compile-time branch families. No branch-parity entries.

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/process_loop.rs
```

## Pass/Fail Gate Criteria
- **PASS:** Recursive collision structure matches C. Dispatch ordering correct. Stuck-overlap, position snapping, and post-bounce rechecks all verified. Handle-based traversal safe. No TODO/FIXME/HACK.
- **FAIL:** Any dispatch ordering discrepancy. Stuck-overlap not handled. Position snapping missing or misplaced. Elastic_collide not re-checking from head. Mutable borrows survive across callbacks.
