# Resource Subsystem — Functional and Technical Specification

## 1. Purpose and scope

The resource subsystem is the engine's typed resource index and dispatch layer. It provides a single global registry that maps string keys to typed resource descriptors, manages lazy loading and reference counting for heap-allocated resources, stores scalar configuration values, and provides resource-level file-access wrappers (`OpenResFile`, `CloseResFile`, etc.) that route through the UIO virtual filesystem.

**Ownership split with file-io:** The resource subsystem owns wrapper-level policy for resource-content file access: when resource loaders open files, how sentinel/directory cases are interpreted for resource loading, and which UIO directory handles are used for resource lookup. The underlying stream, path, stat, and file-I/O semantics (everything observable through `uio_*` API calls) are owned by the file-I/O subsystem (`file-io/specification.md`). If a file-access behavior differs because of underlying stream semantics, that is a file-I/O conformance issue. If the behavior differs because of resource-wrapper policy (e.g., sentinel return for directories, `_cur_resfile_name` tracking), that is a resource conformance issue.

This specification defines the required externally observable behavior of the resource subsystem. It covers the complete public contract: lifecycle, typed resource registration, resource lookup/load/free/detach semantics, configuration property behavior, file-loading integration, ownership and lifetime rules, error handling, and integration points with UIO and downstream resource consumers.

This is not an implementation plan. Internal module boundaries, data structure choices, and concurrency primitives are out of scope unless they are ABI-visible or required for correctness at the C interop boundary.

**Document layering:** This specification and its companion `requirements.md` are the normative target contract. The companion `initialstate.md` is a descriptive analysis of the current codebase state, not a normative document. Where the two diverge, this specification is authoritative for required behavior.

---

## 2. Glossary

| Term | Definition |
|------|-----------|
| **Resource key** | A NUL-terminated C string (byte string) that uniquely identifies an entry in the global resource map. Keys are treated as opaque byte sequences terminated by NUL; the subsystem does not validate or require UTF-8 encoding. |
| **Descriptor string** | The portion of an index-file value after the `TYPE:` separator; for heap types this is typically a file path, for value types it is a literal. |
| **Global resource map** | The single authoritative in-memory map from resource keys to resource entries. All public API operations query and mutate this map. |
| **Type handler** | A registration record (type name, `loadFun`, optional `freeFun`, optional `toString`) that defines how entries of a given type are loaded, freed, and serialized. The public registration parameter `stringFun` corresponds to the `toString` callback referred to throughout this document. |
| **Type registry** | The collection of all registered type handlers, keyed by type name. Stored under `sys.<type>` keys in the global resource map. |
| **Value type** | A type whose handler has no `freeFun`. Data is stored directly in the data union and populated at index-load time. |
| **Heap type** | A type whose handler has a `freeFun`. Data is a pointer to a heap-allocated object, loaded lazily on first access. |
| **Materialized** | A heap-type entry whose data pointer is non-null (i.e., the resource has been loaded). |
| **Persistent property entry** | A value-type entry (string, integer, boolean, color) that can be serialized via `toString` and round-tripped through save/load. These entries function as the engine's typed configuration store. This is a documentation-level category, not a separate runtime storage class; these entries live in the same global resource map as all other entries. |
| **Dispatch-only resource descriptor** | A heap-type entry that refers to loadable asset content. These entries may lack a `toString` and are not necessarily meaningful to serialize. This is a documentation-level category, not a separate runtime storage class. |
| **Sentinel handle** | The special pointer value `(uio_Stream *)~0` (all bits set) returned by `res_OpenResFile` for directory paths. |

---

## 3. Data model

### 3.1 Resource keys

A resource key (`RESOURCE`) is a NUL-terminated C string. Keys are byte strings terminated by NUL; the subsystem does not validate or enforce UTF-8 encoding. Keys are case-sensitive and use dot-separated namespaces by convention (e.g., `comm.Arilou.Graphics`, `config.sfxvol`). The maximum effective key length is 255 bytes including any prefix applied during index loading.

System-internal keys used for type handler registration use the prefix `sys.` (e.g., `sys.GFXRES`). These keys occupy the same namespace as resource entries and must not collide with user-facing resource keys.

### 3.2 Resource data union

The resource data payload is a C-compatible union of three fields:

| Field     | Type             | Usage                                       |
|-----------|------------------|---------------------------------------------|
| `num`     | `u32` / `DWORD`  | Scalar value types: INT32, BOOLEAN, COLOR   |
| `ptr`     | `*mut c_void`    | Heap-allocated resource handle (opaque)      |
| `str_ptr` | `*const c_char`  | NUL-terminated string pointer (STRING type)  |

The union is `#[repr(C)]` and exactly matches the C `RESOURCE_DATA` layout. Only one field is meaningful at a time; the active field is determined by the resource's registered type.

### 3.3 Resource entries

Each entry in the global resource map is described by:

| Field              | Description                                                        |
|--------------------|--------------------------------------------------------------------|
| Key                | The resource's string key in the global resource map                |
| Descriptor string  | The string parsed from the index file (the part after `TYPE:`)      |
| Type name          | The registered type string (e.g., `STRING`, `GFXRES`, `SNDRES`)    |
| Handler reference  | Link to the registered type handler for this resource's type        |
| Data               | The resource data union                                             |
| Reference count    | Number of outstanding `res_GetResource` references                  |

### 3.4 Two classes of entries in the global resource map

The global resource map stores two functional classes of entries under a single key namespace. Both classes use the same runtime entry structure and live in the same map — the distinction is a documentation-level categorization based on type capabilities, not a separate runtime storage mechanism.

**Persistent property entries** (value types with `toString`): String, integer, boolean, and color entries whose data is stored directly in the data union and populated at index-load time. These entries are serializable and round-trip through `SaveResourceIndex` / `LoadResourceIndex`. They constitute the engine's typed configuration store. All public `res_Get*` / `res_Put*` configuration accessors operate on these entries.

**Dispatch-only resource descriptors** (heap types): Entries whose descriptor string identifies loadable asset content (graphics, sounds, fonts, string tables, etc.). These entries are loaded lazily on first `res_GetResource` call. They may lack a `toString` function, and if so they are not emitted during `SaveResourceIndex`. The `res_GetResource` / `res_FreeResource` / `res_DetachResource` APIs are the normal access path for these entries.

The two classes share the same key namespace, the same replacement semantics during index loading, and the same `res_Remove` behavior. The distinction matters for:

- **Save behavior**: `SaveResourceIndex` iterates the global map, applies root/prefix filtering, and serializes only entries whose current type handler has a `toString` function. Entries without `toString` — including `UNKNOWNRES` and heap types that do not register a serializer — are silently skipped. This is not a class-based filter; it is a direct consequence of the serializer-presence check. In practice, the entries that pass this check are configuration/property entries, so save operationally functions as configuration persistence. But the normative rule is `toString` presence plus root match, not entry classification.
- **Mutability surface**: Configuration consumers use the typed `Put` APIs to mutate persistent property entries. Heap-type entries are not normally mutated through the `Put` APIs; their content is controlled by the registered loader.
- **`res_Remove` on heap entries loaded from content indices**: Valid but atypical. The entry is freed and removed. If the same key is needed later, the index must be reloaded.

### 3.5 Type handlers

A type handler registration consists of:

| Field       | Type                  | Required | Description                                          |
|-------------|-----------------------|----------|------------------------------------------------------|
| `resType`   | String (≤31 bytes)    | Yes      | Type name (e.g., `GFXRES`)                          |
| `loadFun`   | `ResourceLoadFun`     | Yes      | Loads/parses a resource from its descriptor string   |
| `freeFun`   | `ResourceFreeFun`     | No       | Frees a heap-allocated resource; `None` for value types |
| `toString`  | `ResourceStringFun`   | No       | Serializes resource data back to descriptor form     |

The presence or absence of `freeFun` is the canonical discriminator between **value types** and **heap types**:

- **Value types** (`freeFun` is `None`): The data is a scalar stored directly in the data union. `loadFun` is called immediately at index-load time to populate the union. Examples: `STRING`, `INT32`, `BOOLEAN`, `COLOR`, `UNKNOWNRES`.
- **Heap types** (`freeFun` is `Some`): The data is a pointer to a heap-allocated object. Loading is deferred until first access (lazy loading). Examples: `GFXRES`, `FONTRES`, `SNDRES`, `MUSICRES`, `STRTAB`, `BINTAB`, `CONVERSATION`, `3DOVID`, `SHIP`.

### 3.6 Function pointer signatures

The C-compatible function pointer types are:

```
ResourceLoadFun:     fn(*const c_char, *mut ResourceData)
ResourceFreeFun:     fn(*mut c_void) -> c_int
ResourceStringFun:   fn(*mut ResourceData, *mut c_char, c_uint)
ResourceLoadFileFun: fn(*mut c_void /* uio_Stream* */, u32 /* length */) -> *mut c_void
```

These signatures are ABI-stable and must not change without coordinated updates to all registered C callbacks.

---

## 4. Lifecycle

### 4.1 Initialization — `InitResourceSystem`

**Signature:** `fn InitResourceSystem() -> *mut c_void`

Behavior:

1. If the resource system is already initialized, return the existing index handle. Initialization is idempotent.
2. Create a new, empty global resource map.
3. Register the five built-in value types (see §5.1).
4. Invoke downstream subsystem type registration functions in order:
   - `InstallGraphicResTypes()` → registers `GFXRES`, `FONTRES`
   - `InstallStringTableResType()` → registers `STRTAB`, `BINTAB`, `CONVERSATION`
   - `InstallAudioResTypes()` → registers `SNDRES`, `MUSICRES`. (In the current state this uses the C callback path. In the end state, audio-heart may supply Rust-implemented handlers via a feature-gated alternative — see Appendix A. The resource subsystem owns the registration mechanism and dispatch slot; audio-heart owns the handler implementation. Either C or Rust registration is conforming provided the registered handlers satisfy the `loadFun`/`freeFun` ABI and the audio-heart type-specific loading contract in `audio-heart/specification.md` §14.)
   - `InstallVideoResType()` → registers `3DOVID`
   - `InstallCodeResType()` → registers `SHIP`
5. Return a non-null opaque handle representing the initialized index.

Explicit initialization is the normal startup sequence. However, all public APIs must tolerate pre-init calls by auto-initializing (see §4.2).

### 4.2 Auto-initialization on first use

If any public resource API function is called before explicit initialization, the system shall auto-initialize (matching the C `_get_current_index_header()` defensive pattern). This is a compatibility requirement: existing C code may rely on this behavior.

### 4.3 Shutdown — `UninitResourceSystem`

**Signature:** `fn UninitResourceSystem()`

Behavior:

1. For every entry in the global resource map:
   - If the entry has a loaded heap resource (non-null `ptr` and `freeFun` exists), call `freeFun` to release it.
   - Free all internal storage for the entry (descriptor strings, handler references).
2. Destroy the global resource map and type registry.
3. Reset the global state so that a subsequent `InitResourceSystem` call would reinitialize from scratch.

After shutdown, all previously returned resource pointers, string pointers from `res_GetString`, and type name pointers from `res_GetResourceType` become invalid.

### 4.4 Lifecycle ordering

The expected call sequence during engine operation is:

1. `InitResourceSystem()` — during engine setup
2. `LoadResourceIndex(configDir, "uqm.cfg", "config.")` — load user configuration before full setup
3. `LoadResourceIndex(...)` — load content `.rmp` indices (potentially multiple calls, one per content package)
4. Normal operation: `res_GetResource`, `res_DetachResource`, `res_FreeResource`, config accessors, file I/O
5. `SaveResourceIndex(...)` — persist configuration changes
6. `UninitResourceSystem()` — during engine teardown

---

## 5. Typed resource registration

### 5.1 Built-in value types

The resource system registers five built-in value types at initialization. These are always available and are owned by the resource subsystem itself:

| Type name     | `loadFun` behavior                              | `freeFun` | `toString` behavior                          |
|---------------|------------------------------------------------|-----------|----------------------------------------------|
| `UNKNOWNRES`  | Stores descriptor pointer as `str_ptr`          | None      | None                                         |
| `STRING`      | Stores descriptor pointer as `str_ptr`          | None      | Copies `str_ptr` content to output buffer    |
| `INT32`       | Parses decimal integer → `num` (0 on failure)   | None      | Formats `num` as signed decimal string       |
| `BOOLEAN`     | Case-insensitive `"true"` → `num=1`, else `num=0` | None   | `num != 0` → `"true"`, else `"false"`        |
| `COLOR`       | Parses `rgb()`, `rgba()`, `rgb15()` → packed RGBA in `num` | None | Formats as `rgb(0xRR, 0xGG, 0xBB)` or `rgba(0xRR, 0xGG, 0xBB, 0xAA)` |

**COLOR packing format:** `(R << 24) | (G << 16) | (B << 8) | A`. Opaque colors have `A = 0xFF`. The `rgb15()` format accepts 5-bit component values and scales them to 8-bit using the formula `CC5TO8(v) = (v << 3) | (v >> 2)`.

### 5.2 `UNKNOWNRES` lifecycle

`UNKNOWNRES` is the fallback type for entries whose declared type is not found in the type registry at index-load time. Its full lifecycle:

**At load time:**
- When the type name from the index file does not match any registered handler, the entry is stored as `UNKNOWNRES`.
- The original type name from the index file is **not** preserved. The entry's type is recorded as `UNKNOWNRES` and the descriptor string (the part after `:`) is stored as-is in `str_ptr`.
- A warning is logged identifying the unrecognized type name and key.

**Accessor behavior:**
- `res_GetResourceType` returns `"UNKNOWNRES"`.
- Type predicates (`res_IsString`, `res_IsInteger`, etc.) return `FALSE`.
- `res_GetString` returns the empty string (type mismatch: entry is `UNKNOWNRES`, not `STRING`).
- `res_GetResource` returns the `str_ptr` value (the stored descriptor string) and increments the reference count. See §7.1 for how `res_GetResource` handles value types including `UNKNOWNRES`.
- `res_FreeResource` and `res_DetachResource` treat `UNKNOWNRES` as a value type (no `freeFun`) and log a warning.

**Save behavior:**
- `UNKNOWNRES` entries are **skipped** during `SaveResourceIndex` because `UNKNOWNRES` has no `toString` function. This is the same serializer-presence rule that governs all save emission (§6.3). It applies regardless of whether the entry's key matches the save root/prefix filter. In practice, `UNKNOWNRES` entries typically come from content indices and would not match a configuration-oriented save root, but that is not the reason for omission — the sole reason is the absence of `toString`.

**Late registration:**
- If a type handler is registered after index loading and entries of that type were previously stored as `UNKNOWNRES`, those entries are **not** retroactively converted. The entries remain `UNKNOWNRES` until the index is reloaded.

### 5.3 External type registration — `InstallResTypeVectors`

**Signature:** `fn InstallResTypeVectors(res_type: *const c_char, loadFun: ResourceLoadFun, freeFun: Option<ResourceFreeFun>, stringFun: Option<ResourceStringFun>) -> c_int`

Behavior:

1. Validate that `res_type` is non-null and that `loadFun` is provided.
2. Store the handler record under the key `sys.<res_type>` in the type registry.
3. If a handler already exists for this type name, overwrite it. The new handler applies to all future dispatch operations for that type. Behavior for entries already materialized under the old handler is implementation-defined: the old `freeFun` is no longer available through the registry, so those entries may be freed using the new handler's `freeFun` when eventually released. For this reason, type registration replacement is only safe before any entries of that type are materialized. Implementations may refuse replacement after materialization or may snapshot handler identity per entry; the only required invariant is that the replacement must result in one coherent callback set for future operations. In practice, all type registrations occur during `InitResourceSystem` before any index loading, so replacement of an in-use handler is not an expected operational scenario.
4. Return `TRUE` (1) on success, `FALSE` (0) on failure.

Downstream C subsystems call this function during their own `Install*ResType*` routines, which are invoked by `InitResourceSystem`. The resource system does not interpret or validate the semantics of externally registered load/free functions — it only stores and dispatches them.

### 5.4 Type count — `CountResourceTypes`

**Signature:** `fn CountResourceTypes() -> u32`

Returns the number of registered type handlers in the type registry.

---

## 6. Index loading and saving

### 6.1 Index file format

Resource indices are stored in `.rmp` files (and `uqm.cfg`) using a simple property-file format:

```
# Comment lines start with #
key = TYPE:path_or_value   # inline comments also supported
```

Rules:
- Lines starting with `#` (after optional whitespace) are comments and are skipped.
- Blank lines are skipped.
- Keys and values are separated by `=`.
- Leading and trailing whitespace around keys and values is trimmed.
- Inline `#` comments after the value are stripped (trailing whitespace before `#` is trimmed).
- Key case is preserved exactly as written.
- The value must contain a `:` separator; the part before `:` is the type name, the part after is the descriptor string. Lines whose value does not contain `:` are malformed and are skipped with a warning (see §6.5 for the full error model).

### 6.2 Loading — `LoadResourceIndex`

**Signature:** `fn LoadResourceIndex(dir: *mut c_void, rmpfile: *const c_char, prefix: *const c_char)`

Behavior:

1. Open the file `rmpfile` within the UIO directory handle `dir` for reading.
2. If the file cannot be opened, return silently (no entries are loaded, no error is reported to the caller). This is a `void` function.
3. Read the entire file content.
4. Parse using the property-file format (§6.1).
5. For each key-value pair:
   a. If `prefix` is non-null and non-empty, prepend it to the key. Truncate the combined key to 255 bytes if necessary.
   b. Split the value at the first `:` to extract `type_name` and `descriptor_string`. If no `:` is present, this is a line-local parse error: log a warning and skip this entry. Parsing continues with the next line.
   c. Look up `type_name` in the type registry.
   d. If the type is not found, log a warning and store the entry as `UNKNOWNRES` (see §5.2).
   e. If the type is a value type (no `freeFun`), call `loadFun` immediately to populate the data union.
   f. If the type is a heap type, store the descriptor string and leave the data pointer null (deferred loading).
   g. Insert the entry into the global resource map. If the key already exists, remove the old entry first (calling `freeFun` if the old entry was a loaded heap resource). See §6.4 for replacement/invalidation rules.
6. Close the file.

**Partial-load behavior:** Entries are inserted into the live global resource map as they are parsed. Loading is non-transactional: the historical contract does not provide all-or-nothing index loading. Entries successfully parsed before any unrecoverable failure are already committed to the map. See §6.5 for the error model.

This function may be called multiple times to load multiple index files. Entries accumulate in the global resource map; later loads can override earlier entries for the same key.

### 6.3 Saving — `SaveResourceIndex`

**Signature:** `fn SaveResourceIndex(dir: *mut c_void, rmpfile: *const c_char, root: *const c_char, strip_root: c_int)`

Behavior:

1. Open `rmpfile` within `dir` for writing (create or truncate).
2. Iterate all entries in the global resource map.
3. For each entry:
   a. If `root` is non-null, skip entries whose key does not start with `root`.
   b. Skip entries whose current type handler has no `toString` function. This means `UNKNOWNRES` entries and heap types without `toString` are not emitted.
   c. Call the entry's `toString` function to serialize the data union to a string buffer.
   d. Write the line: `key = TYPE:serialized_value\n`.
   e. If `strip_root` is true and `root` is non-null, strip the `root` prefix from the key in the output.
4. Close the file.

Platform-specific newline handling: on Windows, output `\r\n`; on other platforms, output `\n`.

**Serialization scope:** The normative save contract is: iterate the global map, apply root/prefix filtering, and emit each entry whose current type handler has a `toString` function. Entries without `toString` are unconditionally skipped — this is the sole emission gate beyond root matching. In practice, the entries that pass are configuration/property entries, so save functions as configuration persistence. But "configuration persistence" is a usage characterization, not an additional filtering rule. An implementation must not introduce class-based or category-based filtering beyond `toString` presence and root match.

**Save failure semantics:** Save uses a direct open-truncate-then-write strategy. There is no atomic temp-file replacement. If the file is successfully opened but a subsequent write fails mid-stream, the output file may contain a partial set of entries. This is a known limitation of the historical design. Callers should not assume transactional save behavior.

**Additional serialization behavior:**

- **Root matching:** The `root` parameter is matched by plain byte-prefix comparison against each key. It is not namespace-aware.
- **Output order:** The iteration order over the global resource map is implementation-defined. Deterministic (sorted or insertion-ordered) output is not required by the historical contract. Callers must not depend on output ordering.
- **Comment/formatting preservation:** Save output is a fresh serialization. Comments, blank lines, and formatting from previously loaded files are not preserved. This is explicitly out of scope.
- **Canonicalization:** Successive save operations with the same map contents may produce output that differs in key ordering. The output is not canonicalized.

### 6.4 Entry replacement and outstanding-reference invalidation

When a key that already exists in the global resource map is overwritten (by `LoadResourceIndex` inserting a duplicate key, or by a `res_Put*` call on a key of a different type), the old entry is removed first. If the old entry was a loaded heap resource, its `freeFun` is called regardless of its current reference count (a warning is logged if the reference count is > 0).

**Consequence for outstanding references:** If a caller holds a pointer obtained from `res_GetResource` for a heap-type entry, and that entry is subsequently replaced by a new `LoadResourceIndex` call or removed via `res_Remove`, the held pointer becomes **invalid**. The subsystem does not track or invalidate outstanding caller-held pointers. This is a known hazard of the historical design.

**Scope of the hazard:**
- For **persistent property entries** (config values), replacement during reloading is expected and safe because callers do not hold long-lived pointers to config data (except `res_GetString` pointers, which become invalid on replacement — see §8.3).
- For **heap-type resource descriptors**, replacement is unusual in normal engine operation. Content indices are typically loaded once at startup. If a second `LoadResourceIndex` redefines a key whose resource is already loaded and referenced, the old resource is freed and callers holding that pointer observe undefined behavior.
- This behavior is **tolerated by the historical contract** but is dangerous. Normal engine usage avoids it by not reloading content indices after resources are materialized.

### 6.5 Index parse error model

Index parsing distinguishes two classes of errors:

**Line-local parse errors** (recoverable): These are problems with individual entries that do not prevent continued parsing. After logging a warning, the parser skips the malformed entry and continues with the next line. Line-local errors include:
- A value line whose value portion does not contain a `:` separator.
- An unrecognized type name (the entry is stored as `UNKNOWNRES` — this is a warning, not a skip).

**Unrecoverable failures:** These stop all further processing for the current file. They include:
- Failure to open the file (return silently, no entries loaded).
- Any failure that prevents the parser from obtaining or continuing to process file content (e.g., I/O read errors, encoding failures, or memory exhaustion during parsing).

Loading is non-transactional: entries successfully parsed and committed to the global resource map before an unrecoverable failure remain committed. The exact boundary of partial commitment when a low-level failure occurs is implementation-dependent — the contract guarantees only that already-committed entries are not rolled back, not that parsing reaches a specific point before stopping.

---

## 7. Resource access and dispatch

### 7.1 Get resource — `res_GetResource`

**Signature:** `fn res_GetResource(res: *const c_char) -> *mut c_void`

`res_GetResource` applies to both heap-type and value-type entries. The behavior varies by type class:

**Common behavior (all types):**

1. If `res` is null, log a warning and return null.
2. Look up the key in the global resource map.
3. If not found, log a warning and return null.

**Heap-type entries** (entries with `freeFun`):

4. If the entry's data pointer is null (not yet loaded):
   a. Call the type handler's `loadFun` with the entry's descriptor string and a mutable reference to the data union.
   b. During the load callback, the `_cur_resfile_name` global may be set by `LoadResourceFromPath` if the loader uses that helper (see §10.1 for the precise contract).
5. If the data pointer is still null after the load attempt (load failed), return null.
6. Increment the entry's reference count.
7. Return the data pointer.

This implements **lazy loading**: the actual resource content is not loaded until first access. Subsequent calls for the same resource return the cached pointer and increment the reference count.

**Value-type entries** (entries without `freeFun`, including `UNKNOWNRES`):

4. The entry's data is already populated (value types are loaded at index time). No lazy-load dispatch occurs.
5. Increment the entry's reference count.
6. Return the entry's current data union representation: for types using `str_ptr` (STRING, UNKNOWNRES), the string pointer; for types using `num` (INT32, BOOLEAN, COLOR), the `num` value cast to pointer type.

For `UNKNOWNRES` specifically, this returns the stored descriptor string pointer. This is a consequence of `UNKNOWNRES` being a value type — it follows the same code path as other value types, not a special case. The returned pointer is subsystem-owned and subject to the same lifetime rules as other subsystem-owned string pointers (§11.1).

### 7.2 Get integer resource — `res_GetIntResource`

**Signature:** `fn res_GetIntResource(res: *const c_char) -> u32`

Returns `data.num` for the given resource key. Returns 0 if the key is null, not found, or not an integer type. This does not trigger lazy loading (value types are loaded at index time).

### 7.3 Get boolean resource — `res_GetBooleanResource`

**Signature:** `fn res_GetBooleanResource(res: *const c_char) -> c_int`

Returns `res_GetIntResource(res) != 0` as a boolean.

### 7.4 Get resource type — `res_GetResourceType`

**Signature:** `fn res_GetResourceType(res: *const c_char) -> *const c_char`

Returns a pointer to a NUL-terminated string containing the type name for the given resource (e.g., `"GFXRES"`, `"STRING"`). Returns null if the key is null or not found. The returned pointer remains valid until the resource system is shut down or the entry is removed.

### 7.5 Free resource — `res_FreeResource`

**Signature:** `fn res_FreeResource(res: *const c_char)`

Behavior:

1. Look up the key in the global resource map.
2. If not found, log a warning and return.
3. If the type has no `freeFun` (value type), log a warning and return.
4. If the data pointer is null (not loaded), log a warning and return.
5. If the reference count is 0, log a warning (freeing an unreferenced resource).
6. If the reference count is > 0, decrement it.
7. If the reference count reaches 0 after decrement:
   a. Call `freeFun` with the data pointer.
   b. Set the data pointer to null.

The entry remains in the global resource map after freeing. A subsequent `res_GetResource` call for the same key will trigger a fresh load.

### 7.6 Detach resource — `res_DetachResource`

**Signature:** `fn res_DetachResource(res: *const c_char) -> *mut c_void`

Behavior:

1. Look up the key in the global resource map.
2. If not found, log a warning and return null.
3. If the type has no `freeFun` (value type), log a warning and return null.
4. If the data pointer is null (not loaded), log a warning and return null.
5. If the reference count is > 1, log a warning and return null. Detach requires sole ownership.
6. Save the data pointer.
7. Set the entry's data pointer to null and reference count to 0.
8. Return the saved pointer.

After detach, the **caller owns the resource** and is responsible for freeing it using the appropriate subsystem-specific free function. The resource entry remains in the global resource map; a subsequent `res_GetResource` call will trigger a fresh load.

This is the primary mechanism for resource consumers to take ownership:
```c
void *handle = res_GetResource("comm.Arilou.Graphics");  // refcount → 1
FRAME frame = res_DetachResource("comm.Arilou.Graphics"); // refcount → 0, ptr → NULL
// caller now owns `frame` and must call DestroyDrawable() when done
```

### 7.7 Remove resource — `res_Remove`

**Signature:** `fn res_Remove(key: *const c_char) -> c_int`

Behavior:

1. Look up the key in the global resource map.
2. If not found, return `FALSE` (0).
3. If the entry has a loaded resource (non-null ptr):
   a. If the reference count is > 0, log a warning (removing a live resource).
   b. If the type has a `freeFun`, call it to release the resource.
4. Free the entry's internal storage (descriptor string, etc.).
5. Remove the entry from the global resource map.
6. Return `TRUE` (1).

---

## 8. Configuration property accessors

The resource subsystem doubles as a typed key-value store for engine configuration. Configuration values are loaded from index files (typically `uqm.cfg` with a `config.` prefix) and accessed through type-checked accessor functions.

### 8.1 Key existence — `res_HasKey`

**Signature:** `fn res_HasKey(key: *const c_char) -> c_int`

Returns `TRUE` if the key exists in the global resource map, `FALSE` otherwise.

### 8.2 Type checking — `res_Is*`

| Function         | Returns `TRUE` if the entry's type name matches: |
|-----------------|---------------------------------------------------|
| `res_IsString`  | `"STRING"`                                        |
| `res_IsInteger` | `"INT32"`                                         |
| `res_IsBoolean` | `"BOOLEAN"`                                       |
| `res_IsColor`   | `"COLOR"`                                         |

All return `FALSE` if the key does not exist.

### 8.3 Typed getters

| Function         | Return type       | Behavior on missing/type-mismatch           |
|-----------------|-------------------|----------------------------------------------|
| `res_GetString`  | `*const c_char`   | Returns empty string `""` (not null)          |
| `res_GetInteger` | `c_int`           | Returns `0`                                  |
| `res_GetBoolean` | `c_int`           | Returns `FALSE` (0)                          |
| `res_GetColor`   | `Color` (by value)| Returns `rgba(0, 0, 0, 0)`                  |

**`res_GetString` specific requirements:**

1. The entry must exist, have a non-null string value, **and** have type `"STRING"`. If any condition fails, return a pointer to an empty string `""`. This is a critical parity requirement — returning null instead of `""` will crash callers.
2. The returned pointer must remain valid until the entry is removed, overwritten, or the resource system is shut down. The implementation must ensure pointer stability for returned strings.
3. The returned pointer points to the entry's descriptor string (the value portion after `TYPE:`).

**`res_GetColor` specific requirements:**

The returned value is a `Color` struct (4 bytes: r, g, b, a) unpacked from the stored `num` field: `r = num >> 24`, `g = (num >> 16) & 0xFF`, `b = (num >> 8) & 0xFF`, `a = num & 0xFF`.

### 8.4 Typed setters

| Function         | Value type        | Behavior                                       |
|-----------------|-------------------|------------------------------------------------|
| `res_PutString`  | `*const c_char`   | See below                                      |
| `res_PutInteger` | `c_int`           | Sets `data.num = value`                        |
| `res_PutBoolean` | `c_int`           | Sets `data.num = value`                        |
| `res_PutColor`   | `Color`           | Packs to `(r<<24)\|(g<<16)\|(b<<8)\|a` in `data.num` |

**Upsert semantics:** If the key does not exist or the existing entry is not of the correct type, the setter must create a new entry of the appropriate type with a default value, then overwrite the value. This is the "create-on-write" pattern:

- `res_PutString` on missing key → create `STRING:undefined`, then overwrite with the new value.
- `res_PutInteger` on missing key → create `INT32:0`, then overwrite.
- `res_PutBoolean` on missing key → create `BOOLEAN:false`, then overwrite.
- `res_PutColor` on missing key → create `COLOR:rgb(0, 0, 0)`, then overwrite.

**`res_PutString` specific requirements:**

For STRING entries, the descriptor string and the `str_ptr` field must both be updated to reflect the new value. After the put, `data.str_ptr` must point to the current descriptor string so that subsequent `res_GetString` calls return the new value. The implementation must ensure that the update is correct and that pointer stability holds after the call returns; the choice of allocation strategy (in-place reuse, reallocation, or unconditional new allocation) is implementation-defined.

---

## 9. File I/O integration

The resource subsystem provides file I/O wrapper functions that operate through the UIO virtual filesystem. These are used by downstream resource loaders and by the subsystem itself for index loading/saving.

### 9.1 File operations

| Function           | Signature                                                           | Behavior                                |
|-------------------|---------------------------------------------------------------------|-----------------------------------------|
| `res_OpenResFile`  | `fn(dir: *mut c_void, filename: *const c_char, mode: *const c_char) -> *mut c_void` | See §9.2 |
| `res_CloseResFile` | `fn(fp: *mut c_void) -> c_int`                                      | Closes the stream; returns TRUE/FALSE    |
| `ReadResFile`      | `fn(buf: *mut c_void, size: usize, count: usize, fp: *mut c_void) -> usize` | Reads `size * count` bytes via UIO      |
| `WriteResFile`     | `fn(buf: *const c_void, size: usize, count: usize, fp: *mut c_void) -> usize` | Writes `size * count` bytes via UIO     |
| `GetResFileChar`   | `fn(fp: *mut c_void) -> c_int`                                      | Reads one byte; returns -1 on EOF/error |
| `PutResFileChar`   | `fn(ch: c_char, fp: *mut c_void) -> c_int`                         | Writes one byte                          |
| `PutResFileNewline`| `fn(fp: *mut c_void) -> c_int`                                      | Writes platform newline                  |
| `SeekResFile`      | `fn(fp: *mut c_void, offset: c_long, whence: c_int) -> c_long`     | Seeks within the stream                  |
| `TellResFile`      | `fn(fp: *mut c_void) -> c_long`                                     | Returns current position                 |
| `LengthResFile`    | `fn(fp: *mut c_void) -> usize`                                      | Returns file size; see §9.2             |
| `DeleteResFile`    | `fn(dir: *mut c_void, filename: *const c_char) -> c_int`           | Deletes a file; returns TRUE/FALSE       |

### 9.2 Directory sentinel handling

`res_OpenResFile` must detect directories and handle them specially:

1. Before attempting to open the file, stat the path.
2. If the path is a directory, return the **sentinel handle** `(uio_Stream *)~0` (all bits set) without opening a file.
3. If the path is a regular file, open it normally via `uio_fopen` and return the stream pointer.
4. If the path does not exist or the stat fails, attempt to open normally (which will return null on failure).

The sentinel handle propagates through other file operations:

- `res_CloseResFile`: If `fp` is the sentinel, return TRUE without closing anything.
- `LengthResFile`: If `fp` is the sentinel, return `1`.
- `ReadResFile`, `WriteResFile`, `SeekResFile`, `TellResFile`, `GetResFileChar`, `PutResFileChar`: If `fp` is the sentinel, these are effectively no-ops / return error indicators. The behavior with sentinels is not well-exercised but must not crash.

This sentinel mechanism exists to support resource loaders that need to distinguish "path is a directory of loose files" from "path is a single data file."

### 9.3 Content-relative file loading — `LoadResourceFromPath`

**Signature:** `fn LoadResourceFromPath(pathname: *const c_char, loadFun: ResourceLoadFileFun) -> *mut c_void`

Behavior:

1. Open `pathname` relative to the global `contentDir` via `res_OpenResFile(contentDir, pathname, "rb")`.
2. If the open fails (returns null), log a warning and return null.
3. Query the file length via `LengthResFile`.
4. If the length is 0, log a warning, close the file, and return null.
5. Set the global `_cur_resfile_name` to `pathname`.
6. Call the provided `loadFun(stream, length)` to perform the actual load.
7. Clear `_cur_resfile_name` to null.
8. Close the file.
9. Return the result of `loadFun` (which may be null if the load failed).

This function is the standard entry point for downstream C resource loaders (graphics, sound, strings, video). The `loadFun` callback is typically the subsystem-specific file parser.

### 9.4 Raw binary resource loading — `GetResourceData`

**Signature:** `fn GetResourceData(fp: *mut c_void, length: u32) -> *mut c_void`

Behavior:

1. Read a 4-byte `u32` prefix from the stream.
2. If the read fails, return null.
3. If the prefix is **not** `~0` (i.e., not `0xFFFFFFFF`), log a warning about unsupported LZ-compressed data and return null. (Legacy compressed resources are not supported.)
4. Subtract 4 from `length` to get the remaining payload size.
5. Allocate a buffer of `length - 4` bytes.
6. Read `length - 4` bytes from the stream into the buffer.
7. If the read is short, free the buffer and return null.
8. Return the buffer pointer.

The caller is responsible for freeing the returned buffer via `FreeResourceData`.

### 9.5 Resource data deallocation — `FreeResourceData`

**Signature:** `fn FreeResourceData(data: *mut c_void) -> c_int`

Frees a buffer previously returned by `GetResourceData` or `AllocResourceData`. Returns `TRUE` (1) always. `AllocResourceData` is an alias for the engine's standard allocator (`HMalloc`).

---

## 10. Global state and the `_cur_resfile_name` contract

### 10.1 `_cur_resfile_name`

`_cur_resfile_name` is a global `*const c_char` exported with `#[no_mangle]` for direct C access. Its contract is scoped to `LoadResourceFromPath`:

- **Null** at all times except during an active `LoadResourceFromPath` call.
- **Set** to the pathname argument at the start of the `LoadResourceFromPath` load callback invocation.
- **Cleared** to null immediately after the `LoadResourceFromPath` load callback returns, on all exit paths including failure.

`LoadResourceFromPath` is the common helper that establishes `_cur_resfile_name`. The generic `res_GetResource` dispatch does not itself set or clear `_cur_resfile_name` — that responsibility belongs to `LoadResourceFromPath` when a type-specific loader uses it. A non-file-backed heap type loaded through `res_GetResource` whose `loadFun` does not call `LoadResourceFromPath` will not observe `_cur_resfile_name` being set during its load.

Downstream loaders (particularly graphics loaders) read this global to determine the filename of the resource currently being loaded, for logging and error reporting.

### 10.2 Thread safety model

The resource subsystem is designed for **single-threaded access from the main engine thread**. The `_cur_resfile_name` global is mutated without synchronization. Concurrent access from multiple threads is not a supported use case and may produce incorrect results (particularly around `_cur_resfile_name`, lazy loading, and reference counting).

Implementations may include internal defensive hardening (e.g., mutex protection of global state), but this does not constitute a supported concurrency contract. Callers must not rely on concurrent access being safe.

---

## 11. Ownership and lifetime rules

### 11.1 Ownership domains

The resource subsystem exposes three ownership domains for resource data:

**Subsystem-owned values:** Value-type entries (`STRING`, `INT32`, `BOOLEAN`, `COLOR`, `UNKNOWNRES`) store data directly in the data union. The subsystem owns this storage. Callers receive copies or pointers into subsystem-owned memory.

For `STRING` and `UNKNOWNRES`, the `str_ptr` field points into subsystem-managed memory. The pointer remains valid until:
- The entry is removed via `res_Remove`.
- The entry is overwritten via `res_PutString` or by loading a new index that redefines the key.
- The resource system is shut down via `UninitResourceSystem`.

**Subsystem-owned heap resources:** After `res_GetResource` returns for a heap type, the subsystem owns the resource. The caller has a reference (refcount > 0) but does not own the memory. When the reference count reaches 0 via `res_FreeResource`, the subsystem calls `freeFun` and nulls the pointer.

**Caller-owned (detached) resources:** After `res_DetachResource`, the caller owns the resource pointer and is responsible for freeing it through the appropriate subsystem-specific destructor (e.g., `DestroyDrawable`, `DestroySound`). The subsystem no longer tracks or manages the resource.

### 11.2 Reference counting rules

- `res_GetResource` increments the refcount and returns the pointer.
- `res_FreeResource` decrements the refcount; when it reaches 0, `freeFun` is called and the pointer is nulled.
- `res_DetachResource` requires `refcount ≤ 1`; it transfers ownership by nulling the internal pointer and zeroing the refcount.
- `res_Remove` unconditionally frees the resource (if loaded) and removes the entry, regardless of refcount (with a warning if refcount > 0).

### 11.3 Memory allocation domains

Raw data buffers returned by `GetResourceData` (and allocated by `AllocResourceData`) use the engine's standard allocator (`HMalloc`/`HFree`). These buffers must be freed via `FreeResourceData`, which calls `HFree`.

For typed heap resources (e.g., graphics frames, sound objects), the destruction path is determined by the registered `freeFun` for that type. The `freeFun` is responsible for using the correct deallocation method matching the allocation domain of that resource type. The subsystem does not require universal allocator interchangeability across all resource types — each type's `freeFun` must match its `loadFun`'s allocation behavior.

---

## 12. Error handling

### 12.1 Error reporting strategy

The resource subsystem uses **log-based error reporting**. No function in the public API returns an error code for operational issues; instead, warnings are logged and safe defaults are returned:

| Condition                           | Behavior                           |
|------------------------------------|------------------------------------|
| Null resource key                   | Log warning, return null/zero/false |
| Key not found                       | Log warning, return null/zero/default |
| Type mismatch on getter             | Return type-specific default value  |
| Load failure                        | Return null pointer                 |
| Free of unloaded resource           | Log warning, no-op                 |
| Free of unreferenced resource       | Log warning, proceed with free      |
| Detach with refcount > 1            | Log warning, return null           |
| Detach of non-heap type             | Log warning, return null           |
| Unknown type in index               | Log warning, store as UNKNOWNRES   |
| Invalid descriptor (no `:`)         | Log warning, skip entry (line-local; parsing continues) |
| File open failure (`LoadResourceFromPath`) | Log warning, return null    |
| File open failure (`LoadResourceIndex`) | Return silently (void function) |
| LZ-compressed data encountered      | Log warning, return null           |

### 12.2 Failure behavior by API

| API function | Return on failure | Logs warning | Partial state changes |
|---|---|---|---|
| `InitResourceSystem` | Non-null (idempotent) | No | N/A |
| `LoadResourceIndex` | void (no return) | On per-entry errors | Yes — loading is non-transactional; entries parsed before a failure are committed (see §6.5) |
| `SaveResourceIndex` | void (no return) | On file-open failure | No output on file-open failure; partial file may remain on mid-write I/O failure (see §6.3) |
| `res_GetResource` | null | Yes | No |
| `res_FreeResource` | void | Yes (on various conditions) | No |
| `res_DetachResource` | null | Yes | No |
| `res_Remove` | FALSE (0) | No (not found is silent) | No |
| `res_GetString` | `""` (empty string) | No | No |
| `res_GetInteger` | 0 | No | No |
| `LoadResourceFromPath` | null | Yes | No |
| `GetResourceData` | null | Yes (on LZ) | No |

### 12.3 Crash-safety requirements

No resource operation may cause a crash, abort, or undefined behavior when given:
- Null pointers for any parameter.
- Keys that do not exist in the global resource map.
- Type mismatches (e.g., calling `res_GetInteger` on a `STRING` entry).
- Double-free or free-without-get sequences.
- Operations on an uninitialized system (must auto-initialize per §4.2).

The one exception is that callers must not pass invalid (dangling) pointers disguised as valid ones — the subsystem cannot detect this.

### 12.4 Diagnostic integration

The resource subsystem reports diagnostic information through the engine's established logging path (`log_add` / `fprintf(stderr, ...)` in the C codebase). The diagnostic contract:

- **Logging is best-effort:** Warning messages aid debugging but their presence or absence is not part of the ABI contract. Callers must not parse or depend on warning text content.
- **Context in warnings:** When available, warnings for resource operations should include the key and/or type name. Warnings during file-backed loading should include the file path when available.
- **Warning text stability:** The exact text of warning messages is not stable and may change across implementations. Only the category of diagnostic (warning vs. none) is part of the behavioral contract as documented in §12.1.

---

## 13. Integration points

### 13.1 Runtime authority boundaries

The resource subsystem operates across a hybrid boundary where different runtime components own different aspects of the contract. The normative authority split is:

**The resource subsystem is authoritative for:**
- The global resource map: creation, population, lookup, mutation, and destruction of entries.
- The type registry: storage and lookup of type handler registrations.
- Dispatch routing: selecting the correct handler for a given entry's type and invoking the appropriate callback.
- Lifetime accounting: reference counting, free-on-zero, detach, and remove semantics.
- Public ABI behavior: all externally visible function contracts defined in this specification.
- Cross-type semantic rules derived from registration metadata: the subsystem — not individual handlers — is authoritative for unknown-type fallback (§5.2), value-type versus heap-type treatment based on `freeFun` presence (§3.5), save eligibility via `toString` presence (§6.3), and entry lifetime policy including replacement, removal, and shutdown cleanup (§6.4, §7.5–7.7, §4.3). These rules apply uniformly across all registered types and must not be delegated to or overridden by handler implementations.

**Type-specific parsing, loading, and freeing semantics are the responsibility of the registered handler implementation for each type.** The resource subsystem stores and dispatches handler callbacks but does not interpret or validate the domain-specific behavior of those callbacks (e.g., how a graphics loader decodes pixel data, or how an audio loader interprets a sound format).

**Ownership split with audio-heart for `SNDRES`/`MUSICRES`:** The resource subsystem owns typed resource dispatch, lazy-load lifecycle, reference counting, handler registration, and wrapper-level file-access policy for `SNDRES` and `MUSICRES` resource types. The type-specific handler implementations — file parsing, decoder creation, format detection, error handling, opaque handle semantics, and destroy behavior — are owned by the audio-heart subsystem (`audio-heart/specification.md` §14). If a `SNDRES`/`MUSICRES` loading defect is caused by dispatch, lifecycle, or refcount behavior, that is a resource conformance issue. If it is caused by type-specific decode, format detection, or opaque handle semantics, that is an audio-heart conformance issue.

**Pre-init loading behavior for audio resource types:** The audio-heart subsystem requires `init_stream_decoder()` to have completed before its `SNDRES`/`MUSICRES` handler implementations can successfully load resources (`audio-heart/specification.md` §14.6). The following seam contract governs `res_GetResource` for audio types before audio-heart stream-decoder initialization:

1. **Resource auto-init still applies:** The resource system auto-initializes and registers audio handlers as normal (§4.2). Handler registration is a resource-subsystem operation and does not require audio-heart stream-decoder init.
2. **Handler dispatch is legal but returns null:** When `res_GetResource` dispatches to a `SNDRES`/`MUSICRES` handler before `init_stream_decoder()` has run, the handler's `loadFun` returns null. This is an expected startup-ordering condition defined by `audio-heart/specification.md` §14.6, not a resource-subsystem conformance failure.
3. **Null return means no materialization:** A null `loadFun` return means the resource entry is not materialized. The entry's reference count is not incremented, no opaque handle is stored, and the entry remains in its pre-load state. `res_GetResource` returns null to the caller.
4. **Retry after audio-heart init succeeds normally:** A subsequent `res_GetResource` call for the same resource after `init_stream_decoder()` has completed shall dispatch to the handler again and shall succeed if the underlying resource data is valid. The resource subsystem does not cache or persist the earlier null failure.
5. **Fault attribution:** A null result for an audio resource before `init_stream_decoder()` is attributed to startup ordering (caller responsibility), not to a defect in either the resource subsystem or audio-heart.

**Correctness at the public boundary depends on preserving the following across the hybrid boundary:**
- Callback ABI: function pointer signatures, argument representation, and return-value interpretation must match between the dispatcher and the handler implementation.
- Ownership interpretation: the handler's `loadFun` must return data whose ownership is compatible with the subsystem's `freeFun` dispatch, and the `freeFun` must correctly release what `loadFun` allocated.
- UIO/file-handle conventions: handlers that use `LoadResourceFromPath` or direct file I/O helpers must follow the UIO stream lifecycle expected by the resource subsystem.

### 13.2 UIO dependency

The resource subsystem depends on the UIO virtual filesystem for all file I/O. Required UIO symbols:

| Symbol          | Purpose                                        |
|-----------------|------------------------------------------------|
| `uio_fopen`     | Open a file within a UIO directory handle       |
| `uio_fclose`    | Close a UIO stream                             |
| `uio_fread`     | Read from a UIO stream                         |
| `uio_fwrite`    | Write to a UIO stream                          |
| `uio_fseek`     | Seek within a UIO stream                       |
| `uio_ftell`     | Get current position in a UIO stream           |
| `uio_fgetc`     | Read one byte from a UIO stream                |
| `uio_fputc`     | Write one byte to a UIO stream                 |
| `uio_unlink`    | Delete a file                                  |
| `uio_stat`      | Stat a path (for directory detection)          |
| `uio_fstat`     | Stat an open file (for length queries)         |
| `contentDir`    | Global UIO directory handle for game content   |

The resource subsystem does not own or manage UIO initialization. UIO and `contentDir` must be initialized before any resource file operations are performed.

### 13.3 Downstream resource consumers

Downstream subsystems interact with the resource system at two boundaries:

**Registration boundary (subsystem → resource system):**

Each downstream subsystem registers its resource types during initialization by calling `InstallResTypeVectors`. The resource system calls the subsystem's `Install*ResType*` function during `InitResourceSystem`. The subsystem provides:
- A type name string (e.g., `"GFXRES"`)
- A `loadFun` that, given a descriptor string and a data union pointer, loads the resource (typically by calling `LoadResourceFromPath` with a subsystem-specific `ResourceLoadFileFun`)
- A `freeFun` that releases the loaded resource
- Optionally, a `toString` function for serialization

Registered types and their handler implementations:

| Subsystem  | Types registered                        | Load entry point                                           |
|------------|----------------------------------------|-----------------------------------------------------------|
| Graphics   | `GFXRES`, `FONTRES`                    | `_GetCelData`, `_GetFontData` (call `LoadResourceFromPath`) |
| Strings    | `STRTAB`, `BINTAB`, `CONVERSATION`     | `_GetStringData`, `_GetBinaryTableData`, `_GetConversationData` |
| Audio      | `SNDRES`, `MUSICRES`                   | C: `_GetSoundBankData`, `_GetMusicData` (default path)     |
| Video      | `3DOVID`                               | `GetLegacyVideoData`                                      |
| Code       | `SHIP`                                 | Code resource loader in `dummy.c`                          |

**Consumption boundary (resource system → subsystem consumer):**

C code that needs a resource follows the get-detach pattern:

```c
// Load-and-detach: caller takes ownership
void *handle = res_GetResource("comm.Arilou.Graphics");  // lazy load + refcount++
FRAME frame = (FRAME) res_DetachResource("comm.Arilou.Graphics");  // transfer ownership
// ... use frame ...
DestroyDrawable(frame);  // subsystem-specific cleanup
```

Or the reference pattern:

```c
// Reference: resource system retains ownership
void *handle = res_GetResource("some.resource");  // refcount++
// ... use handle ...
res_FreeResource("some.resource");  // refcount--; freed at 0
```

### 13.4 Configuration consumers

Configuration values are accessed through the typed property accessors (§8). Key consumers:

| Consumer             | Keys accessed (examples)                                     | Operations                      |
|---------------------|--------------------------------------------------------------|---------------------------------|
| Engine startup      | `config.sfxvol`, `config.musicvol`, `config.smooth`, etc.    | `res_GetInteger`, `res_GetBoolean`, `res_GetString` |
| Input system        | `config.keys.*`, `config.joy.*`                              | `res_GetString`, `res_PutString`, `res_IsString`, `res_Remove` |
| Options screen      | Various `config.*` keys                                      | `res_GetInteger`, `res_PutInteger`, `res_GetBoolean`, `res_PutBoolean` |
| Config save         | `config.*` prefix                                            | `SaveResourceIndex` with `root="config."` |

---

## 14. Parity requirements

The implementation must be behavior-compatible with the C implementation for all ABI-visible operations. The following are specific areas where the specification mandates C-parity:

### 14.1 `res_OpenResFile` directory detection

Must perform `uio_stat` before `uio_fopen` and return the sentinel `~0` for directories, matching the C implementation in `filecntl.c:32-41`.

### 14.2 `res_GetString` type enforcement

Must verify that the entry's type is `"STRING"` before returning. On type mismatch or missing entry, must return a pointer to an empty string `""` (not null), matching C `resinit.c:476-486`.

### 14.3 `GetResourceData` prefix handling

Must read a 4-byte prefix, reject non-`~0` prefixes, and read `length - 4` remaining bytes. Must not seek backwards. Must match C `loadres.c:26-55`.

### 14.4 `res_PutString` post-put correctness

After `res_PutString` returns, `data.str_ptr` must point to a valid NUL-terminated string containing the new value, and the descriptor string must be updated consistently. Subsequent `res_GetString` calls must return the new value. The pointer returned by `res_GetString` must remain valid until the next overwrite, removal, or shutdown (see §8.3). The implementation may use any allocation strategy (in-place reuse when the new value fits, unconditional reallocation, or other approaches) provided these post-conditions are met.

### 14.5 Color parsing

Must support `rgb()`, `rgba()`, and `rgb15()` color descriptor formats. Must clamp component values to valid ranges. Must use the `CC5TO8` scaling formula for `rgb15()` components. Must match C `resinit.c:177-286`.

### 14.6 Property file parsing

Must preserve key case exactly (no uppercasing). Must handle inline `#` comments. Must trim whitespace around keys and values. Must support optional key prefix. Must cap combined key+prefix length at 255 bytes. Must match C `propfile.c` behavior.

---

## 15. Exported symbol inventory

The following symbols must be exported with `#[no_mangle]` and C ABI linkage to satisfy the `reslib.h` contract:

**Lifecycle:**
- `InitResourceSystem`
- `UninitResourceSystem`

**Index I/O:**
- `LoadResourceIndex`
- `SaveResourceIndex`

**Type registration:**
- `InstallResTypeVectors`
- `CountResourceTypes`

**Resource dispatch:**
- `res_GetResource`
- `res_DetachResource`
- `res_FreeResource`
- `res_GetIntResource`
- `res_GetBooleanResource`
- `res_GetResourceType`

**Configuration accessors:**
- `res_HasKey`
- `res_IsString`, `res_IsInteger`, `res_IsBoolean`, `res_IsColor`
- `res_GetString`, `res_GetInteger`, `res_GetBoolean`, `res_GetColor`
- `res_PutString`, `res_PutInteger`, `res_PutBoolean`, `res_PutColor`
- `res_Remove`

**File I/O:**
- `res_OpenResFile`
- `res_CloseResFile`
- `ReadResFile`
- `WriteResFile`
- `GetResFileChar`
- `PutResFileChar`
- `PutResFileNewline`
- `SeekResFile`
- `TellResFile`
- `LengthResFile`
- `DeleteResFile`

**Resource data helpers:**
- `LoadResourceFromPath`
- `GetResourceData`
- `FreeResourceData`

**Global state:**
- `_cur_resfile_name` (mutable global `*const c_char`)

---

## 16. Edge-case examples

### 16.1 Unknown type fallback

```
# In content.rmp:
alien.NewRace.Dialog = HOLOVID:comm/newrace/dialog.hv
```

If `HOLOVID` is not registered when this index is loaded:
- A warning is logged: unrecognized type `HOLOVID` for key `alien.NewRace.Dialog`.
- The entry is stored as `UNKNOWNRES` with descriptor string `comm/newrace/dialog.hv` in `str_ptr`.
- `res_GetResourceType("alien.NewRace.Dialog")` → `"UNKNOWNRES"`.
- If saved later with a matching root prefix, this entry is skipped (no `toString`).

### 16.2 `res_GetString` on a non-string key

```
# In uqm.cfg:
sfxvol = INT32:128
```

After loading with prefix `config.`:
- `res_GetString("config.sfxvol")` → returns `""` (empty string, not null). The entry exists but its type is `INT32`, not `STRING`.
- `res_IsString("config.sfxvol")` → `FALSE`.
- `res_GetInteger("config.sfxvol")` → `128`.

### 16.3 Key replacement during second index load

```
# First load (content-base.rmp):
comm.Arilou.Graphics = GFXRES:comm/arilou/arilou.ani

# Second load (content-addon.rmp), same key:
comm.Arilou.Graphics = GFXRES:comm/arilou_hd/arilou.ani
```

During the second `LoadResourceIndex`:
- The existing entry for `comm.Arilou.Graphics` is found.
- If the old entry was already loaded (materialized), its `freeFun` is called and the old resource is destroyed — even if callers still hold pointers to it.
- The new entry replaces it with the addon's descriptor string.
- Next `res_GetResource("comm.Arilou.Graphics")` will lazy-load from the addon path.

---

## Appendix A. Non-normative notes

This appendix contains migration notes, cleanup candidates, and conditional behavior that is not part of the required external contract but is relevant for implementation planning.

### A.1 Feature-gated audio integration (`audio_heart`)

When the `audio_heart` Cargo feature is active:
- `InitResourceSystem` installs Rust-implemented `SNDRES` and `MUSICRES` handlers instead of calling `InstallAudioResTypes()`.
- The Rust audio handlers invoke `LoadSoundFile`/`LoadMusicFile` and `DestroySound`/`DestroyMusic` from the Rust sound subsystem.

When `audio_heart` is not active, the C `InstallAudioResTypes()` path is used and audio resource loading follows the standard C callback path.

This feature gate is a transitional mechanism. The normative contract for `SNDRES`/`MUSICRES` is defined by the externally visible behavior (correct loading, freeing, and dispatch), regardless of which language implements the handlers.

### A.2 Duplicate Rust resource stack

The codebase contains a second, non-authoritative Rust resource implementation. This code is not part of the active engine integration path and must not be used as an implementation target:
- `ffi.rs` — exports `rust_init_resource_system`, `rust_load_index`, `rust_resource_loader_init`, `rust_resource_load`, and cache APIs.
- `resource_system.rs` — defines a separate `ResourceSystem` with `PropertyFile`, `ResourceType`, `PathBuf`-based paths, and an `Arc<ResourceValue>` cache.
- `loader.rs`, `cache.rs` — LRU cache and loader abstractions.
- `index.rs`, `config_api.rs` — alternate Rust-native representations.

No C production code calls into these modules. The authoritative path is `ffi_bridge.rs` + `dispatch.rs` + `type_registry.rs` + `propfile.rs` + `ffi_types.rs`.

Similarly, `rust_resource.c` and `rust_resource.h` on the C side describe a sidecar integration path that is not called from production code.

### A.3 Support utilities

`stringbank.c` and `direct.c` remain C-owned utility implementations. They are consumed by other C code (string table handling, directory listing), not by the core resource dispatch layer. Their eventual replacement is outside the scope of this specification but does not affect the resource subsystem's contract.

### A.4 UIO transition

If and when UIO is fully ported to Rust, the resource subsystem should transition from importing UIO as `extern "C"` symbols to calling Rust-native equivalents. The behavioral contracts (sentinel handling, stat semantics, stream operations) must remain identical during this transition.
