# Phase 15: Resource Dispatch — Stub

## Phase ID
`PLAN-20260224-RES-SWAP.P15`

## Prerequisites
- Required: Phase 14a (Type Registration Implementation Verification) completed
- Expected: Working type registry, config API, parser, color parser

## Requirements Implemented (Expanded)

### REQ-RES-026-033: res_GetResource
**Requirement text**: Lazy-load via function pointer dispatch. Look up
descriptor, if ptr is NULL call loadFun, increment refcount, return ptr.

### REQ-RES-034-040: res_FreeResource
**Requirement text**: Decrement refcount, if zero call freeFun and NULL ptr.

### REQ-RES-041-046: res_DetachResource
**Requirement text**: Transfer ownership — return ptr, NULL descriptor's ptr,
set refcount to 0. Fail if refcount > 1.

### REQ-RES-083-085: res_Remove
**Requirement text**: Free loaded data via freeFun, remove from HashMap.

### REQ-RES-031-033: LoadResourceFromPath
**Requirement text**: Open file via UIO, get length, call load function,
set _cur_resfile_name during load.

### REQ-RES-101-103: Additional Value Access
`res_GetIntResource`, `res_GetBooleanResource`, `res_GetResourceType`, `CountResourceTypes`

## Implementation Tasks

### Files to create/modify
- `rust/src/resource/dispatch.rs` (new) — Resource dispatch logic
  - Stub `get_resource(key) -> *mut c_void` — `todo!()`
  - Stub `free_resource(key)` — `todo!()`
  - Stub `detach_resource(key) -> *mut c_void` — `todo!()`
  - Stub `remove_resource(key) -> bool` — `todo!()`
  - Stub `load_resource_from_path(path, load_fn) -> *mut c_void` — `todo!()`
  - Stub `get_int_resource(key) -> u32` — `todo!()`
  - Stub `get_boolean_resource(key) -> bool` — `todo!()`
  - Stub `get_resource_type(key) -> Option<&str>` — `todo!()`
  - marker: `@plan PLAN-20260224-RES-SWAP.P15`
  - marker: `@requirement REQ-RES-026-046, REQ-RES-083-085, REQ-RES-101-103`

### Design note: C function pointer dispatch
When `get_resource()` triggers a lazy load for a heap type:
1. Get the descriptor's `loadFun` from the type handler
2. Call it: `unsafe { (load_fun)(fname_ptr, &mut resdata) }`
3. The C function (e.g., `GetCelFileData`) reads the file via UIO
   and sets `resdata.ptr` to the loaded data
4. Rust stores the opaque pointer — it never dereferences it

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `dispatch.rs` with all stubs
- [ ] Module registered in `mod.rs`
- [ ] Plan markers present

## Semantic Verification Checklist
- [ ] Compilation succeeds
- [ ] Existing tests pass

## Success Criteria
- [ ] All stubs compile

## Failure Recovery
- rollback: `git checkout -- rust/src/resource/`

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P15.md`
