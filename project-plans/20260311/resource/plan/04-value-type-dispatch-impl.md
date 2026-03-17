# Phase 04: Value-Type Dispatch Fix — Implementation

## Phase ID
`PLAN-20260314-RESOURCE.P04`

## Prerequisites
- Required: Phase 03/03a completed
- All TDD tests from Phase 03 compile
- Expected files from previous phase: tests added to `dispatch.rs`

## Requirements Implemented (Expanded)

### REQ-RES-UNK-001: Unknown type fallback
(See Phase 03 for full text)

### REQ-RES-UNK-003: Unknown type accessor behavior
(See Phase 03 for full text)

### REQ-RES-LOAD-011: Value-type access through the general resource accessor
(See Phase 03 for full text)

### REQ-RES-LOAD-003: Reference acquisition
(See Phase 03 for full text)

## Implementation Tasks

### Files to modify

#### 1. `rust/src/resource/ffi_bridge.rs` — UNKNOWNRES registration
- **Location:** `register_builtin_types()` function (~line 190)
- **Change:** Replace `(None, None, None)` for UNKNOWNRES with a proper loadFun that stores the descriptor as `data.str_ptr`
- **Implementation:**
  ```rust
  // @plan PLAN-20260314-RESOURCE.P04
  // @requirement REQ-RES-UNK-001
  extern "C" fn unknownres_load_fun(descriptor: *const c_char, data: *mut ResourceData) {
      unsafe {
          (*data).str_ptr = descriptor;
      }
  }
  // Then register: ("UNKNOWNRES", Some(unknownres_load_fun), None, None)
  ```
- marker: `@plan PLAN-20260314-RESOURCE.P04`
- marker: `@requirement REQ-RES-UNK-001`

#### 2. `rust/src/resource/dispatch.rs` — `process_resource_desc` UNKNOWNRES handling
- **Location:** Line 89 (`is_value_type = false` for unknown types)
- **Change:** Set `is_value_type = true` for UNKNOWNRES fallback
- **Also:** Fix line 97 — when loading value types for UNKNOWNRES, use `handler_key` (which is "UNKNOWNRES") to look up the handler, not the original `type_name` (which was the unregistered type)
- marker: `@plan PLAN-20260314-RESOURCE.P04`
- marker: `@requirement REQ-RES-UNK-001`
- Uses pseudocode lines: PC-2 (10-46)

#### 3. `rust/src/resource/dispatch.rs` — `get_resource` value-type path
- **Location:** `get_resource()` method (~lines 128-168)
- **Change:** Before the lazy-load check, determine if the entry's type is a value type (handler has no `freeFun`). If value type:
  - Increment refcount
  - Return `str_ptr` as `*mut c_void` if `str_ptr` is non-null
  - Else return `num` as `*mut c_void`
  - Do NOT enter the lazy-load path
- **Keep existing heap-type logic unchanged**
- marker: `@plan PLAN-20260314-RESOURCE.P04`
- marker: `@requirement REQ-RES-LOAD-011`
- Uses pseudocode lines: PC-3 (50-82)

### Pseudocode traceability
- PC-1 lines 1-9: UNKNOWNRES loadFun implementation
- PC-2 lines 10-46: process_resource_desc fix
- PC-3 lines 50-82: get_resource value-type path

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Targeted tests — ALL should now PASS
cargo test --lib -- resource::dispatch::tests::test_unknownres_registered_as_value_type
cargo test --lib -- resource::dispatch::tests::test_process_resource_desc_unknown_type_stores_as_value
cargo test --lib -- resource::dispatch::tests::test_get_resource_value_type_string_returns_str_ptr
cargo test --lib -- resource::dispatch::tests::test_get_resource_value_type_int_returns_num_as_ptr
cargo test --lib -- resource::dispatch::tests::test_get_resource_unknownres_returns_str_ptr
cargo test --lib -- resource::dispatch::tests::test_get_resource_heap_type_still_lazy_loads
```

## Structural Verification Checklist
- [ ] `ffi_bridge.rs` — `unknownres_load_fun` added
- [ ] `ffi_bridge.rs` — UNKNOWNRES registered with `Some(unknownres_load_fun)`
- [ ] `dispatch.rs` — `process_resource_desc` sets `is_value_type = true` for UNKNOWNRES
- [ ] `dispatch.rs` — `process_resource_desc` uses `handler_key` for value-type loadFun lookup
- [ ] `dispatch.rs` — `get_resource` has new value-type code path
- [ ] Plan/requirement traceability present in changed code

## Semantic Verification Checklist (Mandatory)
- [ ] All 6 TDD tests from Phase 03 now PASS
- [ ] All pre-existing tests still PASS
- [ ] `res_GetResource("config.somebool")` on a BOOLEAN entry returns `num` as pointer (not null)
- [ ] `res_GetResource("unknown.key")` on an UNKNOWNRES entry returns str_ptr (not null)
- [ ] Heap-type lazy loading still works (no regression)
- [ ] No placeholder/deferred implementation patterns remain

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/resource/dispatch.rs rust/src/resource/ffi_bridge.rs
```

## Success Criteria
- [ ] All Phase 03 TDD tests pass
- [ ] All existing tests pass
- [ ] Verification commands pass
- [ ] No new TODOs/FIXMEs in modified files

## Failure Recovery
- rollback: `git checkout -- rust/src/resource/dispatch.rs rust/src/resource/ffi_bridge.rs`
- blocking: if `unknownres_load_fun` signature doesn't match `ResourceLoadFun`, adjust extern "C" signature

## Phase Completion Marker
Create: `project-plans/20260311/resource/.completed/P04.md`
