# Phase 0.5: Preflight Verification

## Phase ID
`PLAN-20260314-RESOURCE.P00.5`

## Purpose
Verify assumptions about the current codebase before implementing any gap-closure work.

## Toolchain Verification
- [ ] `cargo --version`
- [ ] `rustc --version`
- [ ] `cargo clippy --version`
- [ ] `cargo llvm-cov --version` (if a coverage gate is later introduced for this plan)

## Dependency Verification
- [ ] `Cargo.toml` includes `log` crate (used for `log::warn!` throughout resource code)
- [ ] `Cargo.toml` includes `libc` (or equivalent) for `c_void`, `c_char`, `c_int`
- [ ] `audio_heart` feature flag exists in `Cargo.toml` features section
- [ ] `USE_RUST_RESOURCE` is defined in `sc2/config_unix.h`

## Type/Interface Verification

### UIO externs in ffi_bridge.rs
- [ ] `uio_fopen` is declared in the extern block (`rust/src/resource/ffi_bridge.rs`)
- [ ] `uio_fclose` is declared
- [ ] `uio_fread` is declared
- [ ] `uio_fwrite` is declared
- [ ] `uio_fseek` is declared
- [ ] `uio_ftell` is declared
- [ ] Verify whether `uio_stat` is declared — expected: NOT declared (GAP-1 dependency)
- [ ] Verify `contentDir` extern symbol is declared

### ResourceData union layout
- [ ] `ResourceData` in `ffi_types.rs` has `num: u32`, `ptr: *mut c_void`, `str_ptr: *const c_char`
- [ ] `ResourceData` is `#[repr(C)]`
- [ ] `ResourceData` implements `Default` (all-zero)

### TypeRegistry
- [ ] `type_registry.rs` — `TypeRegistry::install()` stores handler under `sys.<type>` key
- [ ] `type_registry.rs` — `TypeRegistry::lookup()` returns `Option<&ResourceHandlers>`
- [ ] `type_registry.rs` — `TypeRegistry::count()` returns count of handlers
- [ ] Built-in types registered: STRING, INT32, BOOLEAN, COLOR, UNKNOWNRES

### ResourceDispatch
- [ ] `dispatch.rs` — `ResourceDispatch::process_resource_desc()` exists with signature `(key, type_name, path)`
- [ ] `dispatch.rs` — `ResourceDispatch::get_resource()` exists with signature `(key) -> Option<*mut c_void>`
- [ ] `dispatch.rs` — `ResourceDispatch::free_resource()` exists
- [ ] `dispatch.rs` — `ResourceDispatch::detach_resource()` exists
- [ ] `dispatch.rs` — `ResourceDispatch::remove_resource()` exists

### Exported symbols
- [ ] Verify all 41 `#[no_mangle]` exports in `ffi_bridge.rs` compile
- [ ] `_cur_resfile_name` is exported as a mutable global
- [ ] `STREAM_SENTINEL` constant is defined

## Call-Path Feasibility

### GAP-1 / GAP-9: file-open and load-from-path behavior
- [ ] Check if `uio_stat` is declared anywhere in the C codebase: `sc2/src/libs/uio/uio.h`
- [ ] Capture the exact `uio_stat` signature from the authoritative C headers/source and record it for later phases
- [ ] Verify whether the stat buffer type is `struct stat` or a UIO-specific wrapper
- [ ] Determine the Rust extern declaration needed for the ABI exactly as implemented in C
- [ ] Verify the current `LoadResourceFromPath` behavior for both `null` and `STREAM_SENTINEL` results from `res_OpenResFile`
- [ ] Verify the current `LoadResourceFromPath` behavior for `length == 0`

### GAP-3 / GAP-4: UNKNOWNRES value-type path
- [ ] Verify that `process_resource_desc` line 89 sets `is_value_type = false` for unknown types
- [ ] Verify that `get_resource` only checks `data.ptr` and does not handle value types separately

### GAP-5: UninitResourceSystem freeFun call path
- [ ] Verify `UninitResourceSystem` currently does `*guard = None` and nothing more
- [ ] Verify that `ResourceDispatch::entries` is accessible for iteration at teardown time
- [ ] Verify that `TypeRegistry::lookup` returns handler with `free_fun` field

### GAP-7: SaveResourceIndex toString filtering
- [ ] Verify current SaveResourceIndex code path and confirm the fallback-format behavior

### GAP-11: dead-code removal dependency analysis
- [ ] Check whether `sc2/src/libs/resource/rust_resource.h` is included by any compiled C translation unit
- [ ] Check whether `sc2/src/libs/resource/rust_resource.c` is compiled by the active build system when `USE_RUST_RESOURCE` is enabled
- [ ] Record the exact build-system evidence that proves those C-side files are unused, conditionally compiled, or still required

## Test Infrastructure Verification
- [ ] `rust/src/resource/tests.rs` exists and contains test functions
- [ ] `cargo test --lib -- resource` runs existing resource tests successfully
- [ ] `rust/src/resource/dispatch.rs` has `#[cfg(test)] mod tests` block
- [ ] `rust/src/resource/type_registry.rs` has `#[cfg(test)] mod tests` block
- [ ] Tests can construct `ResourceDispatch` and `TypeRegistry` instances without global state
- [ ] Determine whether `SaveResourceIndex` can be exercised through a real or shimmed UIO/temp-file path in tests

## Preflight Artifacts Required by Later Phases
- [ ] Record the confirmed `uio_stat` signature/location for Phase 08
- [ ] Record the authoritative engine build command for Phase 11, including the exact repository-relative working directory and invocation to use for a full `USE_RUST_RESOURCE` build
- [ ] Record any authoritative engine launch/test command needed after the build, if different from existing project docs/scripts
- [ ] Record one concrete repository path, fixture, or directory-backed resource case that can exercise the sentinel branch for Phases 08, 08a, and 11
- [ ] Record the exact verification command, fixture path, or harness location that will be used to prove config save/load round-trip in Phase 11
- [ ] Record the confirmed build-system status of `rust_resource.h` / `rust_resource.c` for Phase 10
- [ ] Record whether `SaveResourceIndex` can be tested end-to-end at the FFI/UIO layer for Phase 07

## Blocking Issues
If any of the following are true, stop and revise the plan:
- `uio_stat` does not exist in the C codebase (would require UIO-level work before GAP-1)
- The authoritative engine build command cannot be identified and recorded (would leave Phase 11 under-specified)
- No concrete directory-backed resource path/fixture can be identified for sentinel verification (would leave GAP-1/GAP-9 integration proof under-specified)
- `ResourceData` layout doesn't match assumptions (would require broader FFI audit)
- Existing tests are broken (would require stabilization before new work)
- `rust_resource.h` / `rust_resource.c` are active build dependencies and no safe removal/guard strategy is identified

## Gate Decision
- [ ] PASS: proceed to Phase 1
- [ ] FAIL: revise plan before proceeding
