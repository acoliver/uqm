# Phase 17: Resource Dispatch — Implementation

## Phase ID
`PLAN-20260224-RES-SWAP.P17`

## Prerequisites
- Required: Phase 16a (Resource Dispatch TDD Verification) completed

## Requirements Implemented (Expanded)

### REQ-RES-026-033: res_GetResource — Lazy load, refcount
### REQ-RES-034-040: res_FreeResource — Decrement, free at zero
### REQ-RES-041-046: res_DetachResource — Ownership transfer
### REQ-RES-083-085: res_Remove — Cleanup and removal
### REQ-RES-031-033: LoadResourceFromPath — File open, length, load

## Implementation Tasks

### Files to modify
- `rust/src/resource/dispatch.rs`
  - Implement `get_resource(key)`:
    - NULL key → log warning, return NULL
    - Lookup in HashMap → not found → log warning, return NULL
    - If resdata.ptr is NULL and type is heap: call loadFun via unsafe
    - If still NULL after load → return NULL
    - Increment refcount, return ptr
    - marker: `@plan PLAN-20260224-RES-SWAP.P17`
  
  - Implement `free_resource(key)`:
    - Lookup → not found → warning
    - Not heap (freeFun is None) → warning
    - Not loaded (ptr is NULL) → warning
    - refcount == 0 → warning
    - refcount > 0 → decrement
    - If refcount reaches 0 → call freeFun, set ptr=NULL
    - marker: `@requirement REQ-RES-034-040`
  
  - Implement `detach_resource(key)`:
    - Guards: not found, non-heap, not loaded, refcount > 1
    - Save ptr, set ptr=NULL, set refcount=0
    - Return saved ptr
    - marker: `@requirement REQ-RES-041-046`
  
  - Implement `remove_resource(key)`:
    - If loaded and has freeFun → call freeFun
    - If refcount > 0 → log warning
    - Remove from HashMap
    - Return success
    - marker: `@requirement REQ-RES-083-085`
  
  - Implement `load_resource_from_path(path, load_fn)`:
    - Initially: stub that sets up for UIO integration (Phase 18)
    - Will call `res_OpenResFile(contentDir, path, "rb")`
    - Get length, set _cur_resfile_name, call load_fn, clear _cur_resfile_name
    - Close file
    - Note: Full UIO integration is in Phase 18. This phase implements
      the dispatch logic with a placeholder for file I/O.
  
  - Implement `get_int_resource`, `get_boolean_resource`, `get_resource_type`

### Pseudocode traceability
- get_resource: component-003.md lines 28-50
- load_resource_desc: component-003.md lines 51-56
- free_resource: component-003.md lines 57-83
- detach_resource: component-003.md lines 84-108
- remove_resource: component-003.md lines 109-124
- load_resource_from_path: component-003.md lines 125-145

### Critical: unsafe boundaries
All C function pointer calls (loadFun, freeFun) are `unsafe`. Each call
must be:
1. Inside an `unsafe` block
2. Preceded by an `Option` check (function pointer is Some)
3. Parameters validated (non-null fname_ptr, valid resdata pointer)

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] All dispatch functions implemented
- [ ] No `todo!()` markers remain
- [ ] All unsafe blocks documented
- [ ] Plan markers present

## Semantic Verification Checklist
- [ ] All P16 dispatch tests pass (GREEN)
- [ ] Lazy loading works (loadFun called once, cached thereafter)
- [ ] Refcount lifecycle correct
- [ ] Detach transfers ownership and forces reload
- [ ] Remove calls freeFun before dropping
- [ ] All error/warning paths handled

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder" rust/src/resource/dispatch.rs
# Expected: 0 matches (except LoadResourceFromPath UIO note, which is addressed in P18)
```

## Success Criteria
- [ ] All P16 tests pass
- [ ] Lint/format/test gates pass

## Failure Recovery
- rollback: `git checkout -- rust/src/resource/dispatch.rs`

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P17.md`
