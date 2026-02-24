# Phase 0.5: Preflight Verification

## Phase ID
`PLAN-20260224-RES-SWAP.P00.5`

## Purpose
Verify assumptions before implementation.

## Toolchain Verification
- [ ] `cargo --version`
- [ ] `rustc --version`
- [ ] `cargo clippy --version`

## Dependency Verification
- [ ] `libc` crate in `Cargo.toml`
- [ ] `std::ffi` available (CString, CStr, c_char, c_int, c_void, c_uint, c_long)
- [ ] `std::collections::HashMap` available
- [ ] `std::sync::Mutex` or `std::sync::RwLock` available
- [ ] `USE_RUST_RESOURCE` defined in `sc2/config_unix.h`

## Type/Interface Verification
- [ ] Existing `ResourceIndex` in `rust/src/resource/index.rs` — verify struct fields
- [ ] Existing `PropertyFile` in `rust/src/resource/propfile.rs` — verify parse method signature
- [ ] Existing `ResourceCache` in `rust/src/resource/cache.rs` — verify it exists (will not be part of new API)
- [ ] Existing `ResourceType` in `rust/src/resource/resource_type.rs` — verify enum variants
- [ ] Existing `ColorResource` in `rust/src/resource/resource_type.rs` — verify parse method
- [ ] Existing FFI globals in `rust/src/resource/ffi.rs` — verify `GLOBAL_RESOURCE_SYSTEM`
- [ ] C `ResourceDesc` in `sc2/src/libs/resource/index.h` — verify struct layout
- [ ] C `ResourceHandlers` in `sc2/src/libs/resource/index.h` — verify struct layout
- [ ] C `RESOURCE_DATA` union in `sc2/src/libs/reslib.h` — verify union layout
- [ ] C `Color` struct — verify `{r, g, b, a}` field order
- [ ] C function pointer typedefs in `reslib.h` — verify `ResourceLoadFun`, `ResourceFreeFun`, `ResourceStringFun`, `ResourceLoadFileFun`
- [ ] C `uio_DirHandle`, `uio_Stream` — verify they are opaque pointers

## Call-Path Feasibility
- [ ] `InitResourceSystem` called from `uqm/setup.c:109` — verify this call site exists
- [ ] `LoadResourceIndex` called from multiple sites — verify `uqm.c`, `options.c`, `input.c`
- [ ] `res_GetResource` called from `getres.c` (currently) and from `Load*Instance` wrappers
- [ ] `res_Get/PutString` called from `setupmenu.c`, `input.c`, `uqm.c`
- [ ] `SaveResourceIndex` called from `setupmenu.c`, `input.c`
- [ ] `InstallResTypeVectors` called from `resinit.c` and from C subsystem init functions
- [ ] UIO functions (`uio_fopen`, `uio_fclose`, `uio_fread`, etc.) are linked and callable
- [ ] `contentDir` and `configDir` globals are declared in `options.h`

## Test Infrastructure Verification
- [ ] Existing test module `rust/src/resource/tests.rs` compiles
- [ ] `cargo test --workspace` includes resource tests
- [ ] Test data files available: `sc2/content/uqm.rmp`, `sc2/content/uqm.key`, `sc2/content/menu.key`
- [ ] `tempfile` crate available for test file I/O

## Existing Code Assessment
- [ ] Document which existing Rust modules will be REFACTORED (propfile.rs, index.rs, resource_type.rs, ffi.rs)
- [ ] Document which existing Rust modules will be PRESERVED (cache.rs, stringbank.rs)
- [ ] Document which existing Rust modules may be DEPRECATED (loader.rs, resource_system.rs — replaced by new FFI-first architecture)

## Blocking Issues
[List any blockers found. If non-empty, stop and revise plan first.]

## Gate Decision
- [ ] PASS: proceed
- [ ] FAIL: revise plan
