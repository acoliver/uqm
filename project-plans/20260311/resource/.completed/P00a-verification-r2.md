# Phase P00.5 Verification Verdict (Re-check)

**Plan**: PLAN-20260314-RESOURCE.P00.5  
**Verifier**: LLxprt Code  
**Date**: 2026-03-14  
**Verdict**: **ACCEPT**

## Summary

I re-verified the corrected preflight report against:

1. the corrected report itself,
2. the prior rejection with 5 required corrections, and
3. the original checklist.

I also spot-checked the corrected claims against the current source in:

- `rust/src/resource/ffi_bridge.rs`
- `rust/src/resource/dispatch.rs`

All 5 prior rejection reasons are now addressed explicitly and accurately enough for the preflight gate. The corrected report no longer overclaims on sentinel handling, no longer overstates the dead-code/build artifact, records the authoritative build command with evidence-based qualification, and explicitly captures the `process_resource_desc()` checklist mismatch.

## Re-verification of prior rejection reasons

### 1. Sentinel path explicitly documented as unimplemented

**Status**: ADDRESSED

The corrected preflight now explicitly states that Rust `res_OpenResFile()` does **not** implement the C directory-to-sentinel behavior.

The report cites the current Rust implementation as:

```rust
#[no_mangle]
pub unsafe extern "C" fn res_OpenResFile(
    dir: *mut c_void,
    filename: *const c_char,
    mode: *const c_char,
) -> *mut c_void {
    uio_fopen(dir, filename, mode)
}
```

This matches the actual source in `rust/src/resource/ffi_bridge.rs`: the function directly delegates to `uio_fopen()` and does not call `uio_stat()`, check `S_ISDIR()`, or return `STREAM_SENTINEL` for directories.

The corrected report also describes this as a blocker for later sentinel phases rather than implying the path is already supported.

### 2. LoadResourceFromPath bypass documented

**Status**: ADDRESSED

The corrected preflight now explicitly states that `LoadResourceFromPath()` bypasses `res_OpenResFile()` entirely and directly calls `uio_fopen(contentDir, pathname, mode)`.

That claim matches the current implementation in `rust/src/resource/ffi_bridge.rs`, where `LoadResourceFromPath()` opens the file directly rather than routing through `res_OpenResFile()`.

This addresses the prior rejection reason that the sentinel path was not just missing, but structurally bypassed in the load-from-path call path.

### 3. Dead code artifact uses evidence-based language

**Status**: ADDRESSED

The corrected preflight no longer claims these files compile to empty object files.

Instead, it now says:

- `rust_resource.c` / `rust_resource.h` are still included by the build system,
- the file contents are guarded by `#ifdef USE_RUST_RESOURCE`,
- the files are compiled when enabled,
- no other C files were found to include `rust_resource.h`, and
- the removal conclusion is framed as "compiled but unreferenced dead code" rather than "empty object files."

That is materially stronger and evidence-based. It matches the prior rejection requirement.

### 4. Build command is authoritative

**Status**: ADDRESSED

The corrected preflight now records the authoritative engine build command as:

```bash
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm
```

It also records the basis for dropping the earlier speculative/config step by noting that `config.state` already exists, so the tree is already configured.

This resolves the previous issue where the build artifact was not authoritative enough.

### 5. process_resource_desc signature mismatch recorded

**Status**: ADDRESSED

The corrected preflight now explicitly records:

- the actual signature: `pub fn process_resource_desc(&mut self, key: &str, value: &str)`
- the checklist assumption: a three-parameter `(key, type_name, path)` form
- the explanation that the actual API parses `TYPE:path` from the combined `value`

This matches the actual code in `rust/src/resource/dispatch.rs` and directly addresses the prior rejection reason.

## Additional spot checks

### `process_resource_desc()` actual signature and parsing

Confirmed in `rust/src/resource/dispatch.rs`:

```rust
pub fn process_resource_desc(&mut self, key: &str, value: &str) {
    let (type_name, path) = match value.split_once(':') {
```

So the corrected report is accurate in recording the checklist mismatch.

### Unknown-type handling remains correctly described

Confirmed in `dispatch.rs`: when the type is unknown, the code stores handler key `"UNKNOWNRES"` and sets `is_value_type` to `false`.

That portion of the corrected preflight remains accurate.

### `STREAM_SENTINEL` exists but is not produced by `res_OpenResFile()`

Confirmed in `ffi_bridge.rs`:

- `STREAM_SENTINEL` is defined
- wrapper functions like `res_CloseResFile()` special-case it
- `res_OpenResFile()` itself does not synthesize it

So the corrected preflight’s distinction between sentinel constant existence and sentinel-producing behavior is accurate.

## Verdict

**ACCEPT**

## Rationale

The corrected preflight addresses all 5 required corrections from the prior rejection:

1. Sentinel path explicitly documented as unimplemented.  
2. `LoadResourceFromPath()` bypass explicitly documented.  
3. Dead-code/build artifact rewritten with evidence-based language.  
4. Build command captured authoritatively.  
5. `process_resource_desc()` signature mismatch explicitly recorded.  

The corrected report is now acceptable as a preflight artifact for later phases.