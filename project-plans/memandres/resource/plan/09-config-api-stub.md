# Phase 09: Config API — Stub

## Phase ID
`PLAN-20260224-RES-SWAP.P09`

## Prerequisites
- Required: Phase 08a (Color Parser Implementation Verification) completed
- Expected: Working color parser and property file parser

## Requirements Implemented (Expanded)

### REQ-RES-051-056: Put Functions
**Requirement text**: `res_PutString`, `res_PutInteger`, `res_PutBoolean`,
`res_PutColor` shall auto-create entries if missing or wrong type.

### REQ-RES-060-065: SaveResourceIndex
**Requirement text**: Iterate entries, filter by root prefix, serialize
via `toString`, write to file.

### REQ-RES-047-050: Get Functions (complete)
**Requirement text**: Typed getters with safe defaults.

### REQ-RES-R007: Case-Sensitive Keys (continued)

## Implementation Tasks

### Files to modify
- `rust/src/resource/ffi.rs` (or new `rust/src/resource/config_api.rs`)
  - Stub `res_PutString(key, value)` — `todo!()`
  - Stub `res_PutInteger(key, value)` — `todo!()`
  - Stub `res_PutBoolean(key, value)` — `todo!()`
  - Stub `res_PutColor(key, value)` — `todo!()`
  - Stub `SaveResourceIndex(dir, file, root, strip_root)` — `todo!()`
  - These will eventually be `#[no_mangle] pub extern "C"` but in stub
    phase they can be regular Rust functions
  - marker: `@plan PLAN-20260224-RES-SWAP.P09`
  - marker: `@requirement REQ-RES-051-056, REQ-RES-060-065`

### New internal types needed
- `ResourceDesc` struct (Rust-side, matching C layout conceptually):
  - `fname: CString`
  - `vtable_type: String` (type name, e.g., "STRING", "INT32")
  - `resdata_num: u32` (for INT32, BOOLEAN, COLOR)
  - `resdata_str: Option<CString>` (for STRING, aliased to fname)
  - `resdata_ptr: *mut c_void` (for heap types)
  - `refcount: u32`
  - Note: This is the Rust internal representation. The `#[repr(C)]`
    FFI representation comes in Phase 12+.

### Pseudocode traceability
- Uses component-002.md lines 1-34 (SaveResourceIndex)
- Uses component-002.md PutString/PutInteger/PutBoolean/PutColor

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] All Put function stubs exist
- [ ] SaveResourceIndex stub exists
- [ ] ResourceDesc internal type defined
- [ ] Plan markers present

## Semantic Verification Checklist
- [ ] Compilation succeeds with stubs
- [ ] Existing tests still pass

## Success Criteria
- [ ] All stubs compile

## Failure Recovery
- rollback: `git checkout -- rust/src/resource/`

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P09.md`
