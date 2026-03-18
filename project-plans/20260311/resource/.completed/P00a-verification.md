# Phase P00.5 Verification Verdict

**Plan**: PLAN-20260314-RESOURCE.P00.5  
**Verifier**: LLxprt Code  
**Date**: 2026-03-14  
**Verdict**: **REJECT**

## Summary

I spot-checked the preflight report against the checklist and the current source. Several core claims are accurate, including the main GAP confirmations around missing `uio_stat` wiring, UNKNOWNRES/value-type behavior, and `UninitResourceSystem` dropping state without teardown iteration. However, the preflight contains multiple material inaccuracies in captured artifacts and one important missed blocker/risk in the sentinel/file-open path. Because later phases depend on these artifacts being authoritative, the preflight should not be accepted as written.

## Spot-check results

### Accurate claims confirmed

1. **GAP-1 confirmed: `uio_stat` is not declared in `ffi_bridge.rs`.**  
   Confirmed in `rust/src/resource/ffi_bridge.rs` extern block: `uio_fopen`, `uio_fclose`, `uio_fread`, `uio_fwrite`, `uio_fseek`, `uio_ftell`, `uio_fgetc`, `uio_fputc`, `uio_unlink` are present, but `uio_stat` is absent.

2. **Authoritative C `uio_stat` signature is correct.**  
   `sc2/src/libs/uio/io.h:117` declares:  
   `int uio_stat(uio_DirHandle *dir, const char *path, struct stat *statBuf);`

3. **`ResourceData` layout matches the checklist assumptions.**  
   `rust/src/resource/ffi_types.rs` defines `#[repr(C)] pub union ResourceData { num: u32, ptr: *mut c_void, str_ptr: *const c_char }` with a zeroing `Default` implementation.

4. **`TypeRegistry::install()` stores handlers under `sys.<type>`.**  
   Confirmed in `rust/src/resource/type_registry.rs`.

5. **`TypeRegistry::lookup()` returns `Option<&ResourceHandlers>`.**  
   Confirmed in `rust/src/resource/type_registry.rs`.

6. **`TypeRegistry::count()` returns the number of registered handlers.**  
   Confirmed in `rust/src/resource/type_registry.rs`.

7. **Built-in types include STRING, INT32, BOOLEAN, COLOR, UNKNOWNRES.**  
   Confirmed in `create_initial_state()` in `rust/src/resource/ffi_bridge.rs`.

8. **GAP-3/4 confirmation is accurate: unknown types are forced non-value.**  
   `rust/src/resource/dispatch.rs` sets `(type_handler_key, is_value_type)` to `("UNKNOWNRES".to_string(), false)` when the type is unknown.

9. **`get_resource()` only uses `data.ptr` for lazy-load state.**  
   Confirmed in `rust/src/resource/dispatch.rs`: it checks `unsafe { desc.data.ptr.is_null() }` and does not branch on a separate value-type flag.

10. **GAP-5 confirmation is accurate: uninit just drops the global state.**  
    `rust/src/resource/ffi_bridge.rs` `UninitResourceSystem()` warns if uninitialized, then does `*guard = None;` and nothing else.

11. **`ResourceDispatch::entries` is publicly accessible for teardown iteration.**  
    Confirmed in `rust/src/resource/dispatch.rs` as `pub entries: HashMap<String, FullResourceDesc>`.

12. **Handlers expose `free_fun` via `TypeRegistry::lookup()`.**  
    Confirmed through `ResourceHandlers` in `rust/src/resource/ffi_types.rs` and `lookup()` in `type_registry.rs`.

13. **The documented test command is real and currently passes.**  
    I ran `cargo test --lib --all-features -- resource` in `/Users/acoliver/projects/uqm/rust`; result: `280 passed; 0 failed; 0 ignored; 0 measured; 1296 filtered out`.

14. **The concrete fixture path exists.**  
    `/Users/acoliver/projects/uqm/sc2/content/base/comm/arilou/arilou.ani` exists on disk.

## Rejection reasons

### 1. Sentinel/file-open behavior artifact is incorrect and misses a blocking issue

The preflight says `LoadResourceFromPath` behavior is documented and effectively acceptable, but the actual implementation does **not** match the C-side sentinel semantics that later phases depend on.

- In C, `res_OpenResFile()` returns sentinel `~0` for directories after calling `uio_stat()` and checking `S_ISDIR` in `sc2/src/libs/resource/filecntl.c:32-43`.
- In Rust, `res_OpenResFile()` in `rust/src/resource/ffi_bridge.rs:994-1003` simply calls `uio_fopen(dir, filename, mode)` and never checks `uio_stat()`, so it can never synthesize `STREAM_SENTINEL` for directory-backed resources.
- `LoadResourceFromPath()` in `rust/src/resource/ffi_bridge.rs:1132-1165` does **not** call `res_OpenResFile()` at all; it directly calls `uio_fopen(contentDir, pathname, mode)`.
- Therefore the preflight’s wording around sentinel behavior is materially misleading. The actual issue is stronger: the sentinel branch is currently unreachable through `LoadResourceFromPath`, because both the Rust open wrapper and `LoadResourceFromPath` bypass the C directory-detection logic entirely.

This is not just a documented gap; it is a significant integration issue the preflight should have called out more explicitly.

### 2. `LoadResourceFromPath` signature claim in the checklist section was not verified as written

The checklist says to verify `ResourceDispatch::process_resource_desc()` exists with signature `(key, type_name, path)`. Actual code is `process_resource_desc(&mut self, key: &str, value: &str)` where `value` is the whole `TYPE:path` descriptor. The preflight result quietly rewrote that to “parsing `TYPE:path`” instead of flagging the mismatch between checklist assumption and real API shape.

That is not catastrophic by itself, but it means the preflight did not faithfully verify the checklist item as written.

### 3. Export-count claim is inconsistent with the source/comments

The checklist says “Verify all 41 `#[no_mangle]` exports in `ffi_bridge.rs` compile.” The file header comment says “all 38 `extern "C"` functions,” while the preflight asserts 41 exports compile. The preflight does not provide an actual symbol count or a grep-based proof, only an inference from `cargo test` success.

This is weaker than the report presents. I did not independently recount every export here, but the preflight should not claim an exact verified export count without evidence.

### 4. Build artifact is not authoritative enough

The preflight records the authoritative engine build as:

- `./build.sh uqm config`
- `./build.sh uqm`

from `/Users/acoliver/projects/uqm/sc2`.

Problems:

- It does not provide evidence from the actual build scripts/config showing that `config` is required as part of the authoritative command for this phase.
- The project’s own plan material repeatedly uses `cd sc2 && ./build.sh uqm` as the operative build invocation.
- The preflight did not actually run or validate the engine build command it declared authoritative.

This makes the artifact under-specified relative to the checklist requirement that later phases depend on an exact, authoritative build command.

### 5. Dead-code/build-dependency conclusion is overstated and partly wrong

The preflight says `rust_resource.c` / `rust_resource.h` are “dead code” and even says they “compile to empty object files.” That is incorrect.

Actual findings:

- `sc2/src/libs/resource/Makeinfo` explicitly includes both files.
- `sc2/src/libs/resource/rust_resource.c` contains substantial non-empty code under `#ifdef USE_RUST_RESOURCE`; it does **not** compile to an empty object file when that macro is defined.
- `rust_resource.c` includes `rust_resource.h`, so at minimum there is an active self-contained compile dependency.

What is true:

- I found no evidence from source search that other C translation units include `rust_resource.h`.

What is **not** proven by the preflight:

- that these files are safe to remove,
- that they are unused at link/runtime,
- or that they compile to empty objects.

This overstatement is enough to invalidate the Phase 10 artifact as currently captured.

### 6. Test-path/harness artifact for SaveResourceIndex round-trip is speculative, not authoritative

The preflight’s config round-trip section proposes a future command like `./uqm --config-test` and includes pseudo-Rust code, but it does not identify a real existing engine launch/test harness or a concrete already-supported command. The checklist asked to record the exact verification command, fixture path, or harness location for Phase 11. The artifact provided is aspirational rather than authoritative.

### 7. Missed blocker/risk: sentinel support is not only unverified, it is structurally bypassed

This deserves separate mention from item 1 because it affects planning:

- `STREAM_SENTINEL` exists and many file wrapper functions handle it.
- But the path that should produce sentinel values for directory-backed resources is not wired in the Rust implementation.
- Since `LoadResourceFromPath` bypasses `res_OpenResFile`, later sentinel verification phases are under-specified unless the plan explicitly accounts for changing both the open path and the path-based loader behavior.

The preflight should have raised this as a blocking or at least plan-revision-level issue, not merely documented current behavior.

## Final verdict

**REJECT**

## Required corrections before acceptance

1. Revise the preflight to explicitly state that Rust `res_OpenResFile()` does not implement the C directory-to-sentinel behavior because it lacks `uio_stat()` and directly calls `uio_fopen()`.
2. Revise the preflight to explicitly state that `LoadResourceFromPath()` bypasses `res_OpenResFile()` and therefore currently cannot participate in sentinel handling.
3. Replace the Phase 10 artifact with evidence-based language: these files are still compiled by the build system and are not proven safe to remove yet.
4. Replace speculative Phase 11 build/test artifacts with an actually verified authoritative command/harness, or mark that artifact as not yet established.
5. If the checklist item for `process_resource_desc` is retained, record the mismatch between planned signature assumptions and the actual `(key, value)` API.

Once those are corrected, the main GAP confirmations themselves look sound.