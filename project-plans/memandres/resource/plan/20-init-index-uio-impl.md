# Phase 20: Init, Index, and UIO Wrappers — Implementation

## Phase ID
`PLAN-20260224-RES-SWAP.P20`

## Prerequisites
- Required: Phase 19a (Init/Index/UIO TDD Verification) completed

## Requirements Implemented (Expanded)

### REQ-RES-002: InitResourceSystem — Allocate, Register 14 Types
### REQ-RES-003: Idempotent Init
### REQ-RES-088: UninitResourceSystem — Cleanup
### REQ-RES-005: LoadResourceIndex — File Read + Parse
### REQ-RES-082: Auto-Init on API Call
### REQ-RES-089: Multiple LoadResourceIndex Accumulation
### REQ-RES-075-079: File I/O Wrappers (UIO Delegation)
### REQ-RES-R001: No Panic Across FFI
### REQ-RES-R002: NULL Pointer Validation
### REQ-RES-R003: Interior Mutability (Mutex)
### REQ-RES-R004: Poisoned Lock Recovery

## Implementation Tasks

### Files to modify
- `rust/src/resource/ffi_bridge.rs`
  - Implement all 38 `extern "C"` functions:
  
  **Lifecycle:**
  - `InitResourceSystem`: Lock mutex, check if Some, create ResourceState,
    register 5 built-in value types (Rust-side loaders/toString), return ptr
  - `UninitResourceSystem`: Lock mutex, iterate entries calling freeFun for
    loaded heap types, set to None
  
  **Index loading:**
  - `LoadResourceIndex`: Auto-init, call `uio_fopen(dir, filename, "rt")`,
    read entire file via `uio_fread`, pass to `parse_propfile()`,
    each entry goes through `process_resource_desc()`
  - `SaveResourceIndex`: Lock mutex, open file, iterate entries,
    filter by root, serialize via toString, write
  
  **Type registration:**
  - `InstallResTypeVectors`: Delegate to `type_registry::install()`
  
  **Resource access:**
  - All functions: Lock mutex, validate pointers, delegate to dispatch module
  
  **Config get/set:**
  - All functions: Lock mutex, validate pointers, delegate to config module
  
  **File I/O wrappers:**
  - `res_OpenResFile`: Check if directory via `uio_stat`, return sentinel
    for directories, else `uio_fopen`
  - `res_CloseResFile`: NULL → TRUE, sentinel → TRUE, else `uio_fclose`
  - `LengthResFile`: sentinel → 1, else seek-tell-seek
  - All others: direct delegation to UIO functions
  - `GetResourceData`: Read DWORD prefix, check for ~0, read uncompressed
  - `FreeResourceData`: Free via Rust allocator
  
  **Error handling pattern for all functions:**
  ```rust
  #[no_mangle]
  pub extern "C" fn some_function(key: *const c_char) -> ReturnType {
      // 1. Validate pointers
      if key.is_null() { return DEFAULT; }
      // 2. Convert C string
      let key_str = match unsafe { CStr::from_ptr(key) }.to_str() {
          Ok(s) => s,
          Err(_) => return DEFAULT,
      };
      // 3. Lock state
      let mut guard = match RESOURCE_STATE.lock() {
          Ok(g) => g,
          Err(_) => return DEFAULT, // poisoned
      };
      // 4. Auto-init if needed
      if guard.is_none() { /* init */ }
      // 5. Delegate to internal
      let state = guard.as_mut().unwrap();
      // ... actual logic ...
  }
  ```

  - marker: `@plan PLAN-20260224-RES-SWAP.P20`
  - marker: `@requirement REQ-RES-002-003, REQ-RES-088, REQ-RES-075-079, REQ-RES-R001-R004`

### Global mutable state: `_cur_resfile_name`
Must be accessible from C during `LoadResourceFromPath`:
```rust
#[no_mangle]
pub static mut _cur_resfile_name: *const c_char = std::ptr::null();
```

### Sentinel value
```rust
const STREAM_SENTINEL: *mut c_void = !0usize as *mut c_void;
```

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] All 38 extern "C" functions implemented (no `todo!()`)
- [ ] UIO wrappers delegate correctly
- [ ] Global state properly protected
- [ ] `_cur_resfile_name` exposed as `#[no_mangle]`
- [ ] Sentinel constant defined

## Semantic Verification Checklist
- [ ] All P19 tests pass (GREEN)
- [ ] Init registers 5 value types
- [ ] Init is idempotent
- [ ] Uninit cleans up properly
- [ ] LoadResourceIndex parses files correctly
- [ ] File I/O wrappers handle sentinel correctly
- [ ] No panic can cross FFI boundary

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/resource/ffi_bridge.rs
# Expected: 0 matches
```

## Success Criteria
- [ ] All P19 tests pass
- [ ] Lint/format/test gates pass
- [ ] No placeholder markers

## Failure Recovery
- rollback: `git checkout -- rust/src/resource/`

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P20.md`
