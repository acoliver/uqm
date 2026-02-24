# Phase 11: Config API — Implementation

## Phase ID
`PLAN-20260224-RES-SWAP.P11`

## Prerequisites
- Required: Phase 10a (Config TDD Verification) completed
- Expected: All config API tests exist and fail (RED)

## Requirements Implemented (Expanded)

### REQ-RES-047-059: Config Get/Put
### REQ-RES-060-065: SaveResourceIndex
### REQ-RES-R007: Case-Sensitive Keys
### REQ-RES-R012: Stable CString for STRING descriptors

## Implementation Tasks

### Core data structure: ResourceState (internal)
This is the Rust-side global state container. It holds:
- `entries: HashMap<String, ResourceDescInternal>`
  - Key: resource key (case-sensitive, original case)
  - Value: descriptor with fname, type, resdata, refcount
- Interior mutability via `Mutex<Option<ResourceState>>` global

### ResourceDescInternal fields
- `fname: String` — path or value string
- `fname_cstr: CString` — cached CString for FFI return
- `type_name: String` — "STRING", "INT32", "BOOLEAN", "COLOR", etc.
- `resdata_num: u32` — for INT32/BOOLEAN/COLOR
- `resdata_ptr: *mut c_void` — for heap types (NULL until loaded)
- `refcount: u32` — reference count
- `handlers: Option<TypeHandlerRef>` — reference to type handler

### Files to modify
- `rust/src/resource/resource_system.rs` (or new module)
  - Implement `process_resource_desc(key, value)`:
    - Split TYPE:path via `parse_type_path`
    - Look up type handler
    - Create ResourceDescInternal
    - For value types (freeFun==NULL): parse immediately
    - For heap types: set ptr=NULL
    - Insert (replace if exists)
  - Implement `res_PutString`:
    - Auto-create via `process_resource_desc(key, "STRING:undefined")` if needed
    - Update fname and fname_cstr
  - Implement `res_PutInteger`:
    - Auto-create via `process_resource_desc(key, "INT32:0")` if needed
    - Set resdata_num
  - Implement `res_PutBoolean`:
    - Auto-create via `process_resource_desc(key, "BOOLEAN:false")` if needed
    - Set resdata_num
  - Implement `res_PutColor`:
    - Auto-create via `process_resource_desc(key, "COLOR:rgb(0, 0, 0)")` if needed
    - Set resdata_num to packed RGBA
  - Implement `SaveResourceIndex`:
    - Open file for writing (initially to Rust std::fs, UIO wiring in Phase 18)
    - Iterate entries filtered by root prefix
    - For each with toString: serialize `key = TYPE:value\n`
    - If strip_root: remove prefix from key
  - marker: `@plan PLAN-20260224-RES-SWAP.P11`
  - marker: `@requirement REQ-RES-047-065`

### Pseudocode traceability
- process_resource_desc: component-002.md lines 1-44
- res_PutString: component-002.md PutString lines 1-14
- res_PutInteger: component-002.md PutInteger lines 1-10
- res_PutBoolean: component-002.md PutBoolean lines 1-10
- res_PutColor: component-002.md PutColor lines 1-10
- SaveResourceIndex: component-002.md SaveResourceIndex lines 1-34

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] All Put functions implemented
- [ ] process_resource_desc implemented
- [ ] SaveResourceIndex implemented
- [ ] ResourceDescInternal type defined
- [ ] No `todo!()` markers remain

## Semantic Verification Checklist
- [ ] All P10 config API tests pass (GREEN)
- [ ] Put+Get roundtrip works for all 4 types
- [ ] Auto-creation works for missing keys
- [ ] SaveResourceIndex produces correct file format
- [ ] Save/load roundtrip works
- [ ] Type checking (IsString, IsInteger, etc.) works

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/resource/resource_system.rs
# Expected: 0 matches
```

## Success Criteria
- [ ] All P10 tests pass
- [ ] Lint/format/test gates pass

## Failure Recovery
- rollback: `git checkout -- rust/src/resource/`

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P11.md`
