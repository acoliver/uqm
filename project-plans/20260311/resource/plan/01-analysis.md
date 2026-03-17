# Phase 01: Analysis

## Phase ID
`PLAN-20260314-RESOURCE.P01`

## Prerequisites
- Required: Phase 0.5 (Preflight Verification) completed and PASS

## Purpose
Map each identified gap to its root cause, affected code paths, downstream impact, and the minimal change surface required.

## Gap Analysis Detail

### GAP-1: `res_OpenResFile` missing directory sentinel detection

**Root cause:** Rust `res_OpenResFile` at `ffi_bridge.rs:986-995` calls `uio_fopen()` directly without first calling `uio_stat()` to check if the target is a directory.

**C reference behavior** (`filecntl.c:32-41`):
```c
uio_Stream *res_OpenResFile(uio_DirHandle *dir, const char *file, const char *mode) {
    struct stat sb;
    if (uio_stat(dir, file, &sb) == 0 && S_ISDIR(sb.st_mode))
        return (uio_Stream *)~0;  // sentinel
    return uio_fopen(dir, file, mode);
}
```

**Affected callers:**
- `LoadResourceFromPath` in `ffi_bridge.rs` — uses `res_OpenResFile` result; sentinel would need to be detected before passing to loader
- `LengthResFile` in `ffi_bridge.rs` — must return `1` for sentinel handle
- All downstream C loaders that call `res_OpenResFile` (e.g., loose-file speech detection)

**Change surface:**
- Add `uio_stat` to the extern block in `ffi_bridge.rs`
- Add `libc::stat` or equivalent struct definition for the stat buffer
- Add directory detection logic before `uio_fopen` call in `res_OpenResFile`

**Requirements:** REQ-RES-FILE-002, REQ-RES-FILE-003

---

### GAP-2: `res_GetString` missing STRING type check and empty-string fallback

**Root cause:** `res_GetString` at `ffi_bridge.rs:786-817` returns `fname` for any entry regardless of type. Does not check `res_type == "STRING"`. Returns null pointer (not `""`) on missing key.

**C reference behavior** (`resinit.c:456-468`):
```c
const char *res_GetString(const char *res) {
    RESOURCE_INDEX ri = _get_current_index_header();
    ResourceDesc *desc = lookupResourceDesc(ri, res);
    if (desc == NULL) return "";
    if (strcmp(desc->handlers->resType, "STRING") != 0) return "";
    if (desc->fname == NULL) return "";
    return desc->fname;
}
```

**Affected callers:**
- `sc2/src/libs/input/sdl/input.c:86-108` — `res_GetString` for keybinding config
- `sc2/src/uqm.c:358-363` — config string reads
- Any C code doing `const char *s = res_GetString(key); strlen(s);` — crashes on null

**Change surface:**
- `ffi_bridge.rs` `res_GetString`: add type check for "STRING", return `""` on mismatch/missing

**Requirements:** REQ-RES-CONF-003, REQ-RES-ERR-003

---

### GAP-3: `UNKNOWNRES` treated as heap type in `process_resource_desc`

**Root cause:** In `dispatch.rs:89`, when an unknown type is encountered, `is_value_type` is set to `false`. This means UNKNOWNRES entries skip the immediate loadFun call and are treated as deferred-load entries.

Additionally, UNKNOWNRES is registered with `(None, None, None)` — no loadFun at all — in `ffi_bridge.rs:190`. Even if `is_value_type` were `true`, there's no loadFun to call.

**Spec requirement:** UNKNOWNRES is a value type. Its loadFun stores the descriptor pointer as `str_ptr`. Since it has no `freeFun`, it IS a value type by the spec's discriminator.

**Fix needed:**
1. Register UNKNOWNRES with a loadFun that stores descriptor as `data.str_ptr`
2. In `process_resource_desc`, set `is_value_type = true` for UNKNOWNRES fallback (UNKNOWNRES has no freeFun → value type)

**Requirements:** REQ-RES-UNK-001, REQ-RES-UNK-003

---

### GAP-4: `get_resource` doesn't handle value types correctly

**Root cause:** `dispatch.rs:128-168` — `get_resource()` checks `if data.ptr.is_null()` to determine if lazy loading is needed. For value types, `ptr` IS null because the data is in `num` or `str_ptr`. The method returns null for all value-type entries.

**Spec requirement (§7.1):**
- Value-type entries: data is already populated. Return `str_ptr` for string types (STRING, UNKNOWNRES), return `num` cast to pointer for numeric types (INT32, BOOLEAN, COLOR). Increment refcount.
- Heap-type entries: lazy-load if ptr is null, then increment refcount and return ptr.

**Fix needed:**
- `get_resource` must check whether the entry's type has `freeFun` (heap type) vs. no `freeFun` (value type)
- For value types, return the appropriate union field without lazy-load
- For heap types, keep existing lazy-load logic

**Requirements:** REQ-RES-LOAD-011, REQ-RES-LOAD-003

---

### GAP-5: `UninitResourceSystem` doesn't free loaded heap resources

**Root cause:** `ffi_bridge.rs:280-293` — `UninitResourceSystem()` sets `*guard = None`, dropping the `ResourceState`. Rust `Drop` just deallocates the Rust structs. It never iterates entries to call C `freeFun` callbacks on loaded heap resources.

**Spec requirement (§4.3):** For every entry with a loaded heap resource (non-null ptr and freeFun exists), call freeFun to release it. Then destroy the global map.

**Fix needed:**
- Before dropping `ResourceState`, iterate all `dispatch.entries`
- For each entry with non-null `data.ptr` whose handler has `free_fun`, call `free_fun(data.ptr)`
- Then proceed with the drop

**Requirements:** REQ-RES-LIFE-004, REQ-RES-OWN-005, REQ-RES-OWN-010

---

### GAP-6: Entry replacement doesn't free old loaded heap resources

**Root cause:** `dispatch.rs:117` — `self.entries.insert(key, desc)` silently drops the old `FullResourceDesc` if the key existed. If that old entry had a loaded heap resource (non-null ptr with freeFun), the C-allocated resource leaks.

**Spec requirement (§6.4):** When overwriting, if the old entry was a loaded heap resource, call `freeFun` regardless of refcount (warn if refcount > 0).

**Fix needed:**
- Before `insert`, check if old entry exists
- If old entry has non-null `data.ptr` and its handler has `free_fun`, call it
- Warn if old entry has `refcount > 0`

**Requirements:** REQ-RES-OWN-009, REQ-RES-IDX-006

---

### GAP-7: `SaveResourceIndex` emits entries without `toString`

**Root cause:** `ffi_bridge.rs:396-461` — when the handler's `to_string_fun` is `None`, the code falls through to `format!("{}:{}", desc.res_type, desc.fname)` instead of skipping the entry.

**Spec requirement (§6.3):** Skip entries whose current type handler has no `toString` function. UNKNOWNRES and heap types without toString are not emitted.

**Fix needed:**
- If handler has no `to_string_fun`, skip (continue to next entry)
- Do NOT fall through to a format-from-fname path

**Requirements:** REQ-RES-IDX-005, REQ-RES-UNK-002

---

### GAP-8: `CountResourceTypes` return type

**Root cause:** `ffi_bridge.rs:664` returns `u16`. Spec §5.4 says `u32`.

**Fix:** Change return type to `u32`.

**Requirements:** REQ-RES-TYPE-004

---

### GAP-9: `LoadResourceFromPath` missing zero-length guard

**Root cause:** `ffi_bridge.rs:1124-1158` does not check for `length == 0` before calling the loader callback.

**Spec requirement (§9.3 step 4):** If file length is 0, warn, close file, return null.

**Fix:** Add `if length == 0 { warn; close; return null }` check.

**Requirements:** REQ-RES-FILE-005, REQ-RES-FILE-008

---

### GAP-10: `GetResourceData` misleading doc comment

**Root cause:** `ffi_bridge.rs:1160-1163` — doc comment says "seek back 4 bytes" but code correctly does NOT seek back (reads `length - 4`). Code behavior matches spec §9.4; only the comment is wrong.

**Fix:** Correct the doc comment.

**Requirements:** REQ-RES-FILE-006

---

### GAP-11: Non-authoritative dead code modules

**Files:** `ffi.rs`, `resource_system.rs`, `loader.rs`, `cache.rs`, `index.rs`, `config_api.rs`

**Issue:** These modules are not on the authoritative runtime path. They were an earlier attempt at a Rust resource system with different data models (`PropertyFile`, `ResourceValue`, `Arc<T>` caching). No C code calls them. They increase compilation time, create confusion risk, and violate REQ-RES-INT-006 (single authoritative runtime path).

**Fix:** Remove modules and their `mod.rs` declarations. Keep `stringbank.rs` if it's used by `ffi_bridge.rs` (verify).

**Requirements:** REQ-RES-INT-006

---

## Entity/State Transition Summary

### Resource Entry States
```
[Not in map] --LoadResourceIndex--> [Unloaded, value populated] (value type)
[Not in map] --LoadResourceIndex--> [Unloaded, ptr=null] (heap type)
[Unloaded, ptr=null] --res_GetResource--> [Loaded, ptr!=null, refcount=1]
[Loaded, refcount=N] --res_GetResource--> [Loaded, refcount=N+1]
[Loaded, refcount=1] --res_FreeResource--> [Unloaded, ptr=null, refcount=0]
[Loaded, refcount=N>1] --res_FreeResource--> [Loaded, refcount=N-1]
[Loaded, refcount=1] --res_DetachResource--> [Unloaded, ptr=null] (caller owns data)
[Any] --res_Remove--> [Not in map] (freeFun called if loaded)
[Any] --UninitResourceSystem--> [Destroyed] (freeFun called if loaded)
```

### UNKNOWNRES Entry (After Fix)
```
[Index parse, type not found] --> [UNKNOWNRES, str_ptr=descriptor, is_value_type=true]
res_GetResource --> returns str_ptr, refcount++
res_GetString --> returns "" (type mismatch: UNKNOWNRES ≠ STRING)
res_Is* --> all false
SaveResourceIndex --> skipped (no toString)
```

## Integration Touchpoints

| Touchpoint | Direction | Files |
|-----------|-----------|-------|
| C startup calls `InitResourceSystem` | C→Rust | `sc2/src/uqm/setup.c` → `ffi_bridge.rs` |
| C subsystems call `InstallResTypeVectors` | C→Rust | `sc2/src/libs/graphics/resgfx.c` → `ffi_bridge.rs` |
| C loaders call `LoadResourceFromPath` | C→Rust | `sc2/src/libs/graphics/resgfx.c` → `ffi_bridge.rs` |
| Rust dispatch calls C loadFun/freeFun | Rust→C | `dispatch.rs` → C function pointers |
| C consumers call `res_GetString`, `res_GetResource` etc. | C→Rust | `sc2/src/libs/input/sdl/input.c` → `ffi_bridge.rs` |
| C cleanup calls `UninitResourceSystem` | C→Rust | `sc2/src/uqm/cleanup.c` → `ffi_bridge.rs` |

## Old Code to Replace/Remove

| File | What Changes |
|------|-------------|
| `rust/src/resource/dispatch.rs` | Fix `process_resource_desc` UNKNOWNRES handling, `get_resource` value-type path, entry replacement freeFun call |
| `rust/src/resource/ffi_bridge.rs` | Fix `res_GetString`, `UninitResourceSystem`, `SaveResourceIndex`, `res_OpenResFile`, `LoadResourceFromPath`, `CountResourceTypes`, `GetResourceData` doc |
| `rust/src/resource/ffi.rs` | Remove entirely |
| `rust/src/resource/resource_system.rs` | Remove entirely |
| `rust/src/resource/loader.rs` | Remove entirely |
| `rust/src/resource/cache.rs` | Remove entirely |
| `rust/src/resource/index.rs` | Remove entirely |
| `rust/src/resource/config_api.rs` | Remove entirely |
| `rust/src/resource/mod.rs` | Remove dead module declarations |

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Verification Checklist
- [ ] All gaps mapped to requirements
- [ ] All affected files identified
- [ ] Integration touchpoints documented
- [ ] Entity state transitions documented
- [ ] Old code removal list explicit

## Success Criteria
- [ ] Every gap has a root cause, spec reference, and fix description
- [ ] No gap lacks a requirement tracing

## Phase Completion Marker
Create: `project-plans/20260311/resource/.completed/P01.md`
