# Phase 14: Type Registration — Implementation

## Phase ID
`PLAN-20260224-RES-SWAP.P14`

## Prerequisites
- Required: Phase 13a (Type Registration TDD Verification) completed
- Expected: All type registration tests exist and fail (RED)

## Requirements Implemented (Expanded)

### REQ-RES-014: InstallResTypeVectors Creates Handler
### REQ-RES-015: Stored in Same Map with "sys." Prefix
### REQ-RES-016: Four Fields in ResourceHandlers
### REQ-RES-017: Returns FALSE on Failure
### REQ-RES-004: 14 Built-in Types
### REQ-RES-R008: C Function Pointer Safety via Option<fn>

**Note**: REQ-RES-090-095 (CONVERSATION, 3DOVID, SHIP special types) are satisfied
by C loaders calling through the Rust-stored function pointers registered here.

## Implementation Tasks

### Files to modify
- `rust/src/resource/type_registry.rs`
  - Implement `install_res_type_vectors()`:
    - Create `ResourceHandlers` struct (Box'd for stable pointer)
    - Store type name as `CString` for FFI returns
    - Insert into HashMap under `"sys.<type_name>"` key
    - Return true on success
  - Implement `lookup_type_handler()`:
    - Look up `"sys.<type_name>"` in HashMap
    - Return reference to ResourceHandlers
  - Implement `count_resource_types()`:
    - Count entries with `"sys."` prefix
  - Implement 5 built-in value type loaders (Rust functions matching C signatures):
    - `use_descriptor_as_res(descriptor, resdata)` — set str = descriptor
    - `descriptor_to_int(descriptor, resdata)` — parse via atoi semantics
    - `descriptor_to_boolean(descriptor, resdata)` — case-insensitive "true"
    - `descriptor_to_color(descriptor, resdata)` — delegate to parse_c_color
    - Note: These are Rust functions, not C function pointers. They will be
      wrapped as `extern "C"` in the FFI layer (Phase 18).
  - Implement 4 built-in toString functions:
    - `raw_descriptor(resdata, buf, size)` — copy str to buf
    - `int_to_string(resdata, buf, size)` — format num as decimal
    - `boolean_to_string(resdata, buf, size)` — "true"/"false"
    - `color_to_string(resdata, buf, size)` — delegate to serialize_color
  - marker: `@plan PLAN-20260224-RES-SWAP.P14`
  - marker: `@requirement REQ-RES-014-017, REQ-RES-004`

### Pseudocode traceability
- component-003.md InstallResTypeVectors lines 1-27

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `install_res_type_vectors()` fully implemented
- [ ] `lookup_type_handler()` fully implemented
- [ ] 5 built-in loaders implemented
- [ ] 4 built-in toString functions implemented
- [ ] No `todo!()` markers remain

## Semantic Verification Checklist
- [ ] All P13 type registration tests pass (GREEN)
- [ ] Type stored under "sys." prefix
- [ ] Built-in STRING loader aliases str to descriptor
- [ ] Built-in INT32 loader parses decimal
- [ ] Built-in BOOLEAN loader is case-insensitive
- [ ] Built-in COLOR loader parses rgb()/rgba()/rgb15()
- [ ] toString functions produce correct format

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder" rust/src/resource/type_registry.rs
# Expected: 0 matches
```

## Success Criteria
- [ ] All P13 tests pass
- [ ] Lint/format/test gates pass

## Failure Recovery
- rollback: `git checkout -- rust/src/resource/type_registry.rs`

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P14.md`
