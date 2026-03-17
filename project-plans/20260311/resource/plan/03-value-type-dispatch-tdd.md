# Phase 03: Value-Type Dispatch Fix — TDD

## Phase ID
`PLAN-20260314-RESOURCE.P03`

## Prerequisites
- Required: Phase 02/02a completed
- Verify previous phase markers/artifacts exist
- Expected files from previous phase: `02-pseudocode.md`, `02a-pseudocode-verification.md`

## Requirements Implemented (Expanded)

### REQ-RES-UNK-001: Unknown type fallback
**Requirement text**: When a resource index entry declares a type that is not registered in the type registry, the resource subsystem shall store the entry as the built-in unknown type with the descriptor string preserved, rather than discarding the entry.

Behavior contract:
- GIVEN: An index line `mykey = FAKETYPE:some/path.dat`
- WHEN: `LoadResourceIndex` parses this line and FAKETYPE is not registered
- THEN: The entry is stored as UNKNOWNRES with `str_ptr` pointing to "some/path.dat"

Why it matters: Content packs may reference types not yet registered; entries must not be silently lost.

### REQ-RES-UNK-003: Unknown type accessor behavior
**Requirement text**: When typed getters or type predicates are applied to an entry stored as the built-in unknown type, the resource subsystem shall treat the entry as a type mismatch and return the established default/false value for that accessor. When the general resource-access function is applied to an unknown-type entry, the resource subsystem shall return the stored descriptor string pointer and increment the reference count.

Behavior contract:
- GIVEN: An UNKNOWNRES entry with descriptor "some/path.dat"
- WHEN: `res_GetResource(key)` is called
- THEN: Returns the str_ptr value (pointer to "some/path.dat"), refcount increments

### REQ-RES-LOAD-011: Value-type access through the general resource accessor
**Requirement text**: When the general resource-access function is applied to a value-type entry (including entries of the built-in unknown type), the resource subsystem shall return the entry's current data union representation without invoking heap-style lazy-load semantics, and shall increment the reference count.

Behavior contract:
- GIVEN: A STRING entry with str_ptr = "hello"
- WHEN: `res_GetResource(key)` is called
- THEN: Returns str_ptr as `*mut c_void`, refcount = 1
- GIVEN: An INT32 entry with num = 42
- WHEN: `res_GetResource(key)` is called
- THEN: Returns 42 as `*mut c_void`, refcount = 1

### REQ-RES-LOAD-003: Reference acquisition
**Requirement text**: When get-style resource acquisition succeeds for a resource type that participates in retain/release lifetime management, the resource subsystem shall record the caller-visible acquisition in the resource's lifetime accounting.

Behavior contract:
- GIVEN: A value-type entry with refcount = 0
- WHEN: `get_resource` is called
- THEN: refcount becomes 1

## Implementation Tasks

### Tests to create in `rust/src/resource/dispatch.rs`

Add tests in the existing `#[cfg(test)] mod tests` block:

1. **`test_unknownres_registered_as_value_type`**
   - Create TypeRegistry, register built-in types
   - Verify UNKNOWNRES handler has `free_fun = None` (value type discriminator)
   - Verify UNKNOWNRES handler has `load_fun = Some(...)` (not None)
   - marker: `@plan PLAN-20260314-RESOURCE.P03`
   - marker: `@requirement REQ-RES-UNK-001`

2. **`test_process_resource_desc_unknown_type_stores_as_value`**
   - Create ResourceDispatch with registered types
   - Call `process_resource_desc("mykey", "FAKETYPE", "some/path.dat")`
   - Verify entry exists with `type_handler_key = "UNKNOWNRES"`
   - Verify entry's `data.str_ptr` is non-null (loadFun was called)
   - marker: `@plan PLAN-20260314-RESOURCE.P03`
   - marker: `@requirement REQ-RES-UNK-001`

3. **`test_get_resource_value_type_string_returns_str_ptr`**
   - Create dispatch, add a STRING entry via `process_resource_desc`
   - Call `get_resource("key")`
   - Verify returns `Some(ptr)` where ptr equals `data.str_ptr`
   - Verify refcount = 1
   - marker: `@plan PLAN-20260314-RESOURCE.P03`
   - marker: `@requirement REQ-RES-LOAD-011`

4. **`test_get_resource_value_type_int_returns_num_as_ptr`**
   - Create dispatch, add an INT32 entry via `process_resource_desc` with value "42"
   - Call `get_resource("key")`
   - Verify returns `Some(ptr)` where ptr as usize equals 42
   - Verify refcount = 1
   - marker: `@plan PLAN-20260314-RESOURCE.P03`
   - marker: `@requirement REQ-RES-LOAD-011`

5. **`test_get_resource_unknownres_returns_str_ptr`**
   - Create dispatch, add an entry with unregistered type "BOGUS"
   - Verify entry is stored as UNKNOWNRES
   - Call `get_resource("key")`
   - Verify returns `Some(ptr)` where ptr equals `data.str_ptr`
   - Verify refcount = 1
   - marker: `@plan PLAN-20260314-RESOURCE.P03`
   - marker: `@requirement REQ-RES-UNK-003`

6. **`test_get_resource_heap_type_still_lazy_loads`**
   - Create dispatch, register a mock heap type with loadFun and freeFun
   - Add a heap-type entry
   - Call `get_resource("key")`
   - Verify that loadFun was called (data.ptr is set by the mock)
   - Verify refcount = 1
   - marker: `@plan PLAN-20260314-RESOURCE.P03`
   - marker: `@requirement REQ-RES-LOAD-001`

### Pseudocode traceability
- Uses pseudocode lines: PC-1 (1-9), PC-2 (10-46), PC-3 (50-82)

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Targeted test run — these tests should FAIL because implementation isn't done yet
cargo test --lib -- resource::dispatch::tests::test_unknownres_registered_as_value_type
cargo test --lib -- resource::dispatch::tests::test_get_resource_value_type_string_returns_str_ptr
cargo test --lib -- resource::dispatch::tests::test_get_resource_value_type_int_returns_num_as_ptr
cargo test --lib -- resource::dispatch::tests::test_get_resource_unknownres_returns_str_ptr
```

## Structural Verification Checklist
- [ ] Test file compiles (tests can be `#[should_panic]` or `assert!` that will fail against current code)
- [ ] Tests are in `rust/src/resource/dispatch.rs` `#[cfg(test)]` block
- [ ] Each test has plan/requirement traceability markers
- [ ] No production code changed in this phase

## Semantic Verification Checklist (Mandatory)
- [ ] Tests exercise the actual gap (value-type returns from get_resource)
- [ ] Tests use real TypeRegistry and ResourceDispatch, not mocks
- [ ] Tests verify behavior (return value, refcount) not internals
- [ ] Tests for UNKNOWNRES verify the complete chain: unregistered type → stored as UNKNOWNRES → get_resource returns descriptor

## Deferred Implementation Detection (Mandatory)

```bash
# Should find nothing new — this phase only adds tests
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/resource/dispatch.rs
```

## Success Criteria
- [ ] All tests compile
- [ ] Tests that exercise the gap FAIL against current code (TDD: red phase)
- [ ] Existing tests still pass
- [ ] Verification commands pass (except targeted new tests)

## Failure Recovery
- rollback: `git checkout -- rust/src/resource/dispatch.rs`
- blocking: if TypeRegistry cannot be constructed in tests without global state, add a test helper first

## Phase Completion Marker
Create: `project-plans/20260311/resource/.completed/P03.md`
