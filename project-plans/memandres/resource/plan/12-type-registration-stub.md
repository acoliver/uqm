# Phase 12: Type Registration — Stub

## Phase ID
`PLAN-20260224-RES-SWAP.P12`

## Prerequisites
- Required: Phase 11a (Config API Implementation Verification) completed
- Expected: Working config API, working parser, working color parser

## Requirements Implemented (Expanded)

### REQ-RES-014: InstallResTypeVectors
**Requirement text**: When `InstallResTypeVectors(resType, loadFun, freeFun, stringFun)`
is called, the system shall create a `ResourceHandlers` struct and store it in
the HashMap under key `"sys.<resType>"`.

### REQ-RES-015: Same HashMap for Types and Resources
### REQ-RES-016: ResourceHandlers Fields
### REQ-RES-017: Allocation Failure Returns FALSE
### REQ-RES-R008: C Function Pointer Safety
### REQ-RES-R009: ResourceData Union #[repr(C)]
### REQ-RES-R010: ResourceHandlers #[repr(C)]

## Implementation Tasks

### Files to create/modify
- `rust/src/resource/ffi_types.rs` (new) — FFI-compatible type definitions
  - `#[repr(C)] pub union ResourceData { num: u32, ptr: *mut c_void, str_ptr: *const c_char }`
  - `#[repr(C)] pub struct ResourceHandlers { res_type: *const c_char, load_fun: Option<ResourceLoadFun>, free_fun: Option<ResourceFreeFun>, to_string: Option<ResourceStringFun> }`
  - `pub type ResourceLoadFun = unsafe extern "C" fn(*const c_char, *mut ResourceData);`
  - `pub type ResourceFreeFun = unsafe extern "C" fn(*mut c_void) -> c_int;`
  - `pub type ResourceStringFun = unsafe extern "C" fn(*mut ResourceData, *mut c_char, c_uint);`
  - `pub type ResourceLoadFileFun = unsafe extern "C" fn(*mut c_void, u32) -> *mut c_void;`
  - `#[repr(C)] pub struct Color { pub r: u8, pub g: u8, pub b: u8, pub a: u8 }`
  - Stub all with proper type definitions (no `todo!()` here — just types)
  - marker: `@plan PLAN-20260224-RES-SWAP.P12`

- `rust/src/resource/type_registry.rs` (new) — Type handler registry
  - Stub `install_res_type_vectors(type_name, load, free, to_string) -> bool` — `todo!()`
  - Stub `lookup_type_handler(type_name) -> Option<&ResourceHandlers>` — `todo!()`
  - Stub `count_resource_types() -> u16` — `todo!()`
  - marker: `@plan PLAN-20260224-RES-SWAP.P12`
  - marker: `@requirement REQ-RES-014-017`

### Integration note
Type registration is called from C via FFI. C subsystem init code
(InstallGraphicResTypes, InstallAudioResTypes, etc.) passes C function
pointers to Rust. These function pointers are stored and later called
when `res_GetResource` triggers lazy loading.

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `ffi_types.rs` with all `#[repr(C)]` types
- [ ] `type_registry.rs` with stub functions
- [ ] Module registered in `mod.rs`
- [ ] Plan markers present

## Semantic Verification Checklist
- [ ] All types compile
- [ ] `#[repr(C)]` on union, structs, Color
- [ ] Function pointer types match C signatures from reslib.h

## Success Criteria
- [ ] Compilation succeeds
- [ ] Existing tests pass

## Failure Recovery
- rollback: `git checkout -- rust/src/resource/`

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P12.md`
