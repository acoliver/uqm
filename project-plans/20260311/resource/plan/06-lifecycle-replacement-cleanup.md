# Phase 06: Lifecycle & Replacement Cleanup — TDD + Implementation

## Phase ID
`PLAN-20260314-RESOURCE.P06`

## Prerequisites
- Required: Phase 05/05a completed
- Verify previous phase markers/artifacts exist
- Expected files from previous phase: `05-res-getstring-parity.md`, `05a-res-getstring-parity-verification.md`

## Requirements Implemented (Expanded)

### REQ-RES-LIFE-004: Clean shutdown
**Requirement text**: When the resource subsystem is uninitialized, the resource subsystem shall release subsystem-owned global state and resource-index state in a way that leaves the process able to shut down cleanly.

Behavior contract:
- GIVEN: Resource system is initialized, entries loaded, some heap resources materialized
- WHEN: `UninitResourceSystem()` is called
- THEN: Every loaded heap resource has its `freeFun` called; then all Rust state is released

Why it matters:
- Without this, C-allocated graphics/sound/string resources leak on shutdown.

### REQ-RES-OWN-009: Entry replacement invalidation
**Requirement text**: When a loaded heap-type resource entry is replaced by a subsequent index load or removed via the public API while outstanding references exist, the resource subsystem shall log a warning and proceed with freeing the old resource.

Behavior contract:
- GIVEN: Entry "comm.foo.gfx" is loaded (ptr != null, refcount = 1, handler has freeFun)
- WHEN: `LoadResourceIndex` inserts a new entry for "comm.foo.gfx"
- THEN: Old entry's freeFun is called with old ptr, warning logged about refcount > 0

Why it matters:
- Replacement must not leave stale authoritative entries or leak the replaced object.

### REQ-RES-LOAD-007: Value-type free safety
**Requirement text**: When a caller frees or detaches a built-in scalar/config value resource that does not require external destructor logic, the resource subsystem shall handle that operation safely without attempting an incompatible destructor call.

Behavior contract:
- GIVEN: A STRING, INT32, BOOLEAN, COLOR, or UNKNOWNRES entry exists
- WHEN: `res_FreeResource` or `res_DetachResource` is called for that key
- THEN: the call returns through the value-type safety path, logs or reports mismatch as established, and never invokes a heap destructor

Why it matters:
- This requirement is about direct free/detach behavior, not just UNKNOWNRES load classification.

### REQ-RES-LOAD-008: Remove behavior for materialized resources
**Requirement text**: When a caller removes an entry whose resource is currently materialized, the resource subsystem shall update the authoritative resource map and lifetime state according to the established remove contract so that later operations do not observe a stale authoritative entry.

Behavior contract:
- GIVEN: A heap-type resource is materialized and present in the map
- WHEN: `res_Remove(key)` is called
- THEN: its type-specific free path runs if needed, the key disappears from the authoritative map, and later lookups do not observe the old entry

Why it matters:
- Replacement cleanup and remove semantics are related but not interchangeable; both must be verified directly.

### REQ-RES-OWN-005: Destructor use only for compatible types
**Requirement text**: When the subsystem destroys a materialized resource, the resource subsystem shall invoke type-specific destructor logic only for resource types whose registrations define such destructor behavior.

Behavior contract:
- GIVEN: a mix of heap and value-type entries
- WHEN: cleanup or removal happens
- THEN: only heap types with `freeFun` receive destructor callbacks

Why it matters:
- Value types must never be passed through heap-destruction logic.

### REQ-RES-OWN-010: Destruction path matches allocation domain
**Requirement text**: When the subsystem destroys a materialized heap resource, the resource subsystem shall invoke the type-specific freeFun registered for that resource's type rather than assuming a single universal deallocation function.

Behavior contract:
- GIVEN: a materialized heap resource with a registered type-specific freeFun
- WHEN: teardown, replacement, or removal destroys it
- THEN: the registered freeFun for that resource type is invoked

Why it matters:
- Correct destruction depends on matching the registered allocation/free domain.

## Implementation Tasks

### TDD: Tests to add

#### In `rust/src/resource/dispatch.rs` tests:

1. **`test_process_resource_desc_replacement_calls_free_fun`**
   - Register a heap type with a mock freeFun that sets an atomic flag
   - Add a heap entry, simulate it being loaded (set data.ptr to a known value)
   - Call `process_resource_desc` again for the same key with a new value
   - Assert: the mock freeFun was called with the old ptr
   - marker: `@plan PLAN-20260314-RESOURCE.P06`
   - marker: `@requirement REQ-RES-OWN-009`

2. **`test_process_resource_desc_replacement_warns_on_refcount`**
   - Same as above but set refcount > 0 on the old entry
   - Assert: freeFun still called (warning logged but not blocking)
   - marker: `@plan PLAN-20260314-RESOURCE.P06`
   - marker: `@requirement REQ-RES-OWN-009`

3. **`test_process_resource_desc_replacement_value_type_no_free`**
   - Add a STRING entry, then overwrite with a new STRING entry
   - Assert: no crash (no freeFun to call for value types)
   - marker: `@plan PLAN-20260314-RESOURCE.P06`
   - marker: `@requirement REQ-RES-OWN-005`

4. **`test_uninit_frees_loaded_heap_resources`**
   - Create a ResourceDispatch with a loaded heap entry (mock freeFun with atomic flag)
   - Call the teardown/cleanup method
   - Assert: freeFun was called
   - marker: `@plan PLAN-20260314-RESOURCE.P06`
   - marker: `@requirement REQ-RES-LIFE-004`

5. **`test_uninit_skips_value_types`**
   - Create a ResourceDispatch with STRING and INT32 entries
   - Call teardown
   - Assert: no crash (value types have no freeFun)
   - marker: `@plan PLAN-20260314-RESOURCE.P06`
   - marker: `@requirement REQ-RES-OWN-005`

6. **`test_uninit_skips_unloaded_heap_entries`**
   - Create a heap-type entry with `data.ptr = null` (not yet loaded)
   - Call teardown
   - Assert: freeFun NOT called (nothing to free)
   - marker: `@plan PLAN-20260314-RESOURCE.P06`
   - marker: `@requirement REQ-RES-OWN-005`

7. **`test_free_resource_on_value_type_never_calls_free_fun`**
   - Exercise `free_resource` on STRING / INT32 / UNKNOWNRES entries
   - Assert: operation is handled safely and no destructor callback is invoked
   - marker: `@plan PLAN-20260314-RESOURCE.P06`
   - marker: `@requirement REQ-RES-LOAD-007`

8. **`test_detach_resource_on_value_type_returns_null_without_destructor`**
   - Exercise `detach_resource` on STRING / BOOLEAN / UNKNOWNRES entries
   - Assert: returns null or mismatch result per contract and never invokes a destructor callback
   - marker: `@plan PLAN-20260314-RESOURCE.P06`
   - marker: `@requirement REQ-RES-LOAD-007`

9. **`test_remove_materialized_heap_entry_frees_and_erases_key`**
   - Materialize a heap entry with a mock freeFun
   - Call `remove_resource`
   - Assert: freeFun called, function reports success, and subsequent lookup does not find the key
   - marker: `@plan PLAN-20260314-RESOURCE.P06`
   - marker: `@requirement REQ-RES-LOAD-008`

10. **`test_remove_value_type_erases_key_without_heap_destructor`**
   - Insert a STRING or INT32 entry
   - Call `remove_resource`
   - Assert: key is removed and no destructor callback occurs
   - marker: `@plan PLAN-20260314-RESOURCE.P06`
   - marker: `@requirement REQ-RES-LOAD-008`

### Implementation

#### 1. `rust/src/resource/dispatch.rs` — Add `cleanup_all_entries` method

```rust
/// @plan PLAN-20260314-RESOURCE.P06
/// @requirement REQ-RES-LIFE-004, REQ-RES-OWN-010
pub fn cleanup_all_entries(&mut self) {
    for (key, entry) in self.entries.iter_mut() {
        unsafe {
            if !entry.data.ptr.is_null() {
                if let Some(handler) = self.type_registry.lookup(&entry.type_handler_key) {
                    if let Some(free_fun) = handler.free_fun {
                        free_fun(entry.data.ptr);
                        entry.data.ptr = std::ptr::null_mut();
                    }
                }
            }
        }
    }
}
```

#### 2. `rust/src/resource/dispatch.rs` — Fix `process_resource_desc` entry replacement

Before `self.entries.insert(key, desc)` at line 117, add:

```rust
// @plan PLAN-20260314-RESOURCE.P06
// @requirement REQ-RES-OWN-009
if let Some(old) = self.entries.get(key) {
    unsafe {
        if !old.data.ptr.is_null() {
            if let Some(handler) = self.type_registry.lookup(&old.type_handler_key) {
                if let Some(free_fun) = handler.free_fun {
                    if old.refcount > 0 {
                        log::warn!(
                            "Replacing resource '{}' with outstanding refcount={}",
                            key, old.refcount
                        );
                    }
                    free_fun(old.data.ptr);
                }
            }
        }
    }
}
```

#### 3. `rust/src/resource/ffi_bridge.rs` — Fix `UninitResourceSystem`

Replace the current `*guard = None` with:

```rust
// @plan PLAN-20260314-RESOURCE.P06
// @requirement REQ-RES-LIFE-004
if let Some(ref mut state) = *guard {
    state.dispatch.cleanup_all_entries();
}
*guard = None;
```

#### 4. `rust/src/resource/dispatch.rs` — Verify/remove path safety if needed
- Inspect `free_resource`, `detach_resource`, and `remove_resource` against the requirements above.
- If tests 7–10 expose a mismatch, make the minimal behavior-preserving fix in this phase rather than creating a new gap.
- marker: `@plan PLAN-20260314-RESOURCE.P06`
- marker: `@requirement REQ-RES-LOAD-007`
- marker: `@requirement REQ-RES-LOAD-008`

### Pseudocode traceability
- Uses pseudocode lines: PC-2 (31-43), PC-5 (120-139)

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Targeted tests
cargo test --lib -- resource::dispatch::tests::test_process_resource_desc_replacement_calls_free_fun
cargo test --lib -- resource::dispatch::tests::test_uninit_frees_loaded_heap_resources
cargo test --lib -- resource::dispatch::tests::test_free_resource_on_value_type_never_calls_free_fun
cargo test --lib -- resource::dispatch::tests::test_remove_materialized_heap_entry_frees_and_erases_key
```

## Structural Verification Checklist
- [ ] `cleanup_all_entries` method added to `ResourceDispatch`
- [ ] `process_resource_desc` checks for existing entry before insert
- [ ] `UninitResourceSystem` calls `cleanup_all_entries` before dropping state
- [ ] New direct-verification tests added for `res_FreeResource`, `res_DetachResource`, and `res_Remove`
- [ ] Plan/requirement traceability present

## Semantic Verification Checklist (Mandatory)
- [ ] Replacement of loaded heap entries calls freeFun
- [ ] Replacement warns when refcount > 0
- [ ] Replacement of value-type entries doesn't crash
- [ ] UninitResourceSystem calls freeFun on all loaded heap entries
- [ ] UninitResourceSystem skips value types and unloaded entries
- [ ] `free_resource` on value types is safe and never attempts incompatible destruction
- [ ] `detach_resource` on value types is safe and never attempts incompatible destruction
- [ ] `remove_resource` on materialized heap entries frees the object and removes the key
- [ ] `remove_resource` on value types removes the key without heap destruction
- [ ] No memory leaks detectable via test mock
- [ ] Integration points validated end-to-end for cleanup and removal semantics

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/resource/dispatch.rs rust/src/resource/ffi_bridge.rs
```

## Success Criteria
- [ ] All new tests pass
- [ ] All existing tests pass
- [ ] Verification commands pass
- [ ] Requirement behavior demonstrated for shutdown, replacement, value-type free/detach safety, and remove semantics

## Failure Recovery
- rollback steps: `git checkout -- rust/src/resource/dispatch.rs rust/src/resource/ffi_bridge.rs`
- blocking issues to resolve before next phase: mock freeFun calling convention issues or missing direct test access to remove/free/detach paths

## Phase Completion Marker
Create: `project-plans/20260311/resource/.completed/P06.md`

Contents:
- phase ID
- timestamp
- files changed
- tests added/updated
- verification outputs
- semantic verification summary
