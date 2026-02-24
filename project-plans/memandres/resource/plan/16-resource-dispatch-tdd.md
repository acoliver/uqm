# Phase 16: Resource Dispatch — TDD

## Phase ID
`PLAN-20260224-RES-SWAP.P16`

## Prerequisites
- Required: Phase 15a (Resource Dispatch Stub Verification) completed

## Requirements Implemented (Expanded)

### REQ-RES-026-046: Resource Get/Free/Detach — Tests
### REQ-RES-083-085: res_Remove — Tests
### REQ-RES-101-103: Additional Value Access — Tests

## Implementation Tasks

### Tests to write

#### res_GetResource tests
```
test_get_resource_null_key_returns_null
  get_resource(NULL) → NULL

test_get_resource_undefined_key_returns_null
  get_resource("no.such.key") → NULL

test_get_resource_value_type_returns_data
  Register STRING type, add entry "test = STRING:hello"
  get_resource("test") → non-NULL (but value types don't use get_resource in C)

test_get_resource_heap_type_lazy_loads
  Register mock heap type with tracking load function
  Add entry "test = MOCK:path"
  Verify loadFun NOT called yet
  get_resource("test") → calls loadFun, returns ptr

test_get_resource_increments_refcount
  get_resource("test") → refcount becomes 1
  get_resource("test") → refcount becomes 2

test_get_resource_load_failure_returns_null
  Register type with loadFun that leaves ptr=NULL
  get_resource("test") → NULL, refcount stays 0

test_get_resource_cached_after_first_load
  get_resource("test") → loads
  get_resource("test") → does NOT load again (ptr already set)
```

#### res_FreeResource tests
```
test_free_resource_decrements_refcount
  get_resource → refcount=1
  free_resource → refcount=0, freeFun called, ptr=NULL

test_free_resource_multiple_refs
  get_resource x3 → refcount=3
  free_resource → refcount=2, ptr still valid
  free_resource x2 → refcount=0, freeFun called

test_free_resource_unreferenced_logs_warning
  Create loaded resource with refcount=0
  free_resource → warning logged

test_free_resource_non_heap_logs_warning
  Register value type (freeFun=NULL)
  free_resource("value.key") → warning "non-heap resource"

test_free_resource_not_loaded_logs_warning
  Register heap type, add entry (ptr=NULL, not loaded)
  free_resource("test") → warning "not loaded"

test_free_resource_unknown_key_logs_warning
  free_resource("no.such.key") → warning
```

#### res_DetachResource tests
```
test_detach_resource_transfers_ownership
  get_resource → refcount=1, ptr=X
  detach_resource → returns X, ptr=NULL, refcount=0

test_detach_resource_unknown_key_returns_null
  detach_resource("no.such.key") → NULL

test_detach_resource_non_heap_returns_null
  Register value type
  detach_resource → NULL

test_detach_resource_not_loaded_returns_null
  Register heap type, don't load
  detach_resource → NULL

test_detach_resource_multi_ref_returns_null
  get_resource x2 → refcount=2
  detach_resource → NULL (can't detach multi-referenced)

test_detach_resource_triggers_reload
  get_resource → load
  detach_resource → ptr=NULL
  get_resource → loads AGAIN (fresh copy)
```

#### res_Remove tests
```
test_remove_existing_entry
  Add entry, remove_resource("key") → true

test_remove_nonexistent_entry
  remove_resource("no.such.key") → false

test_remove_calls_free_fun
  Add loaded heap entry
  remove_resource → freeFun called

test_remove_live_resource_logs_warning
  get_resource → refcount=1
  remove_resource → warning "replacing while live", still removes
```

#### Additional value access tests
```
test_get_int_resource_no_type_check
  Add INT32 entry with num=42
  get_int_resource → 42

test_get_boolean_resource
  Add BOOLEAN entry with num=1
  get_boolean_resource → true

test_get_resource_type
  Add GFXRES entry
  get_resource_type("key") → "GFXRES"

test_get_resource_type_null_returns_null
  get_resource_type(NULL) → NULL

test_get_resource_type_undefined_returns_null
  get_resource_type("no.such.key") → NULL
```

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features 2>&1 | grep "FAILED"
```

## Structural Verification Checklist
- [ ] All dispatch tests exist
- [ ] Tests compile
- [ ] Plan markers present

## Semantic Verification Checklist
- [ ] Tests fail with stubs (RED)
- [ ] Tests cover lazy loading trigger
- [ ] Tests cover refcount lifecycle
- [ ] Tests cover detach semantics (ownership transfer)
- [ ] Tests cover all error/warning paths

## Success Criteria
- [ ] All tests compile and FAIL (RED)

## Failure Recovery
- rollback: `git checkout -- rust/src/resource/`

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P16.md`
