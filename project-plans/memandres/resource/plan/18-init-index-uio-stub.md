# Phase 18: Init, Index, and UIO Wrappers — Stub

## Phase ID
`PLAN-20260224-RES-SWAP.P18`

## Prerequisites
- Required: Phase 17a (Resource Dispatch Implementation Verification) completed
- Expected: Complete internal Rust resource system (parser, color, config, types, dispatch)

## Requirements Implemented (Expanded)

### REQ-RES-002-003: InitResourceSystem
**Requirement text**: Allocate index, register 14 types. If already initialized, return existing.

### REQ-RES-088: UninitResourceSystem
**Requirement text**: Free all resources, drop index, set to None.

### REQ-RES-005: LoadResourceIndex (via UIO)
**Requirement text**: Open file via UIO, read contents, parse as property file with prefix.

### REQ-RES-075-079: File I/O Wrappers
**Requirement text**: res_OpenResFile, res_CloseResFile, LengthResFile, etc.

### REQ-RES-R001: No Panic Across FFI
### REQ-RES-R002: NULL Pointer Validation
### REQ-RES-R003: Interior Mutability
### REQ-RES-R014: Part of libuqm_rust.a

## Implementation Tasks

This is the phase where all internal Rust functions get their
`#[no_mangle] pub extern "C"` wrappers. The FFI layer bridges
the C ABI to the internal Rust implementations built in P03-P17.

### Files to create/modify

#### `rust/src/resource/ffi_bridge.rs` (new) — Primary FFI exports
All 38 `extern "C"` functions from the spec (Appendix B):

**Lifecycle:**
- `InitResourceSystem() -> *mut c_void` — stub `todo!()`
- `UninitResourceSystem()` — stub `todo!()`

**Index:**
- `LoadResourceIndex(dir, filename, prefix)` — stub `todo!()`
- `SaveResourceIndex(dir, file, root, strip_root)` — stub `todo!()`

**Type Registration:**
- `InstallResTypeVectors(type, load, free, tostring) -> c_int` — stub `todo!()`

**Resource Access:**
- `res_GetResource(key) -> *mut c_void` — stub `todo!()`
- `res_DetachResource(key) -> *mut c_void` — stub `todo!()`
- `res_FreeResource(key)` — stub `todo!()`
- `res_Remove(key) -> c_int` — stub `todo!()`

**Value Access:**
- `res_GetIntResource(key) -> u32` — stub `todo!()`
- `res_GetBooleanResource(key) -> c_int` — stub `todo!()`
- `res_GetResourceType(key) -> *const c_char` — stub `todo!()`
- `CountResourceTypes() -> u16` — stub `todo!()`

**Config Get:**
- `res_HasKey(key) -> c_int` — stub `todo!()`
- `res_IsString(key) -> c_int` — stub `todo!()`
- `res_IsInteger(key) -> c_int` — stub `todo!()`
- `res_IsBoolean(key) -> c_int` — stub `todo!()`
- `res_IsColor(key) -> c_int` — stub `todo!()`
- `res_GetString(key) -> *const c_char` — stub `todo!()`
- `res_GetInteger(key) -> c_int` — stub `todo!()`
- `res_GetBoolean(key) -> c_int` — stub `todo!()`
- `res_GetColor(key) -> Color` — stub `todo!()`

**Config Put:**
- `res_PutString(key, value)` — stub `todo!()`
- `res_PutInteger(key, value)` — stub `todo!()`
- `res_PutBoolean(key, value)` — stub `todo!()`
- `res_PutColor(key, value)` — stub `todo!()`

**File I/O (UIO wrappers):**
- `res_OpenResFile(dir, filename, mode) -> *mut c_void` — stub `todo!()`
- `res_CloseResFile(fp) -> c_int` — stub `todo!()`
- `LoadResourceFromPath(pathname, fn) -> *mut c_void` — stub `todo!()`
- `ReadResFile(buf, size, count, fp) -> usize` — stub `todo!()`
- `WriteResFile(buf, size, count, fp) -> usize` — stub `todo!()`
- `GetResFileChar(fp) -> c_int` — stub `todo!()`
- `PutResFileChar(ch, fp) -> c_int` — stub `todo!()`
- `PutResFileNewline(fp) -> c_int` — stub `todo!()`
- `SeekResFile(fp, offset, whence) -> c_long` — stub `todo!()`
- `TellResFile(fp) -> c_long` — stub `todo!()`
- `LengthResFile(fp) -> usize` — stub `todo!()`
- `DeleteResFile(dir, filename) -> c_int` — stub `todo!()`
- `GetResourceData(fp, length) -> *mut c_void` — stub `todo!()`
- `FreeResourceData(data) -> c_int` — stub `todo!()`

#### UIO extern imports
```rust
extern "C" {
    fn uio_fopen(dir: *mut c_void, path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn uio_fclose(fp: *mut c_void) -> c_int;
    fn uio_fread(buf: *mut c_void, size: usize, count: usize, fp: *mut c_void) -> usize;
    fn uio_fwrite(buf: *const c_void, size: usize, count: usize, fp: *mut c_void) -> usize;
    fn uio_fseek(fp: *mut c_void, offset: c_long, whence: c_int) -> c_int;
    fn uio_ftell(fp: *mut c_void) -> c_long;
    fn uio_getc(fp: *mut c_void) -> c_int;
    fn uio_putc(c: c_int, fp: *mut c_void) -> c_int;
    fn uio_stat(dir: *mut c_void, path: *const c_char, sb: *mut c_void) -> c_int;
    fn uio_unlink(dir: *mut c_void, path: *const c_char) -> c_int;

    static contentDir: *mut c_void;
    static configDir: *mut c_void;
}
```

#### Global state
```rust
static RESOURCE_STATE: Mutex<Option<ResourceState>> = Mutex::new(None);
static CUR_RESFILE_NAME: Mutex<Option<CString>> = Mutex::new(None);
```

- marker: `@plan PLAN-20260224-RES-SWAP.P18`
- marker: `@requirement REQ-RES-002-003, REQ-RES-088, REQ-RES-075-079`

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] All 38 extern "C" stubs exist
- [ ] UIO extern imports declared
- [ ] Global state declarations
- [ ] Module in mod.rs

## Semantic Verification Checklist
- [ ] Compilation succeeds
- [ ] Existing tests pass
- [ ] Symbol names match C reslib.h exactly

## Success Criteria
- [ ] All stubs compile

## Failure Recovery
- rollback: `git checkout -- rust/src/resource/`

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P18.md`
