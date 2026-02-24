# Rust Resource System — Functional & Technical Specification

> Describes **what** the Rust resource system must do to fully replace the C resource system in UQM.
> This is a functional specification, not an implementation plan.

---

## 1. Purpose and Scope

### What This Spec Covers

This document specifies the functional and technical requirements for a Rust replacement of the UQM C resource loading system. It defines:

- The complete public API surface (every function exposed via `extern "C"`)
- Internal architecture and data flow
- File format parsing requirements (`.rmp`, `.cfg`, `.key`)
- Type-specific resource loading dispatch
- Key-value configuration system behavior
- Path resolution and UIO integration
- Reference counting and memory management semantics
- Thread safety guarantees
- Error handling contracts
- Compatibility constraints with existing C code and save files
- Formal requirements in EARS format

### What This Spec Does NOT Cover

- Implementation details (data structure choices beyond interface contracts, internal algorithms)
- Timeline, milestones, or phasing
- Migration strategy or feature-flag mechanics
- Type-specific binary format parsers (cel, font, sound, etc.) — those remain in C
- UIO filesystem internals — UIO is treated as an opaque dependency

### Relationship to Existing Code

The existing Rust code in `rust/src/resource/` has significant gaps documented in the gap analysis (`resource.md`). The existing code:
- Cannot parse actual `.rmp` files (format mismatch — missing `TYPE:path` split)
- Has wrong key casing (C is case-sensitive; Rust lowercases/uppercases)
- Lacks the key-value config API (`res_Put*`, `SaveResourceIndex`)
- Lacks `res_DetachResource` semantics
- Lacks C-compatible color parsing (`rgb()`, `rgba()`, `rgb15()`)
- Has no integration with UIO or C type-specific loaders

This spec describes the complete target system. The existing Rust code may be refactored, extended, or replaced to meet these requirements.

---

## 2. System Context

### How the Resource System Fits into UQM

The resource system is a **string-keyed, type-dispatched, lazy-loading asset manager** that also serves as the game's **configuration store**. It is one of the first subsystems initialized after `main()` and is called from ~200 sites across the C codebase.

### Call Graph

```
┌───────────────────────────────────────────────────────────────────────┐
│                    C Game Code (uqm/*.c)                              │
│  LoadGraphic, LoadFont, LoadSound, LoadMusic, LoadStringTable,       │
│  LoadCodeRes, res_Get{String,Integer,Boolean,Color}, res_Put*,       │
│  res_HasKey, res_Is*, SaveResourceIndex                               │
└──────────────────────────────┬────────────────────────────────────────┘
                               │ extern "C" FFI calls
                               ▼
┌───────────────────────────────────────────────────────────────────────┐
│              Rust Resource System (this spec)                         │
│  Owns: index (HashMap), resource descriptors, type handler registry, │
│        config get/set, .rmp/.cfg parsing, reference counting,        │
│        serialization (SaveResourceIndex)                              │
└──────────┬──────────────────────────────┬────────────────────────────┘
           │ Calls C type loaders         │ Calls UIO for file I/O
           │ via function pointers        │ via extern "C" imports
           ▼                              ▼
┌────────────────────┐         ┌────────────────────────┐
│ C Type Loaders     │         │ UIO (libs/uio/)        │
│ _GetCelData        │         │ uio_fopen, uio_fclose, │
│ _GetFontData       │         │ uio_fread, uio_fstat,  │
│ _GetSoundBankData  │         │ uio_fseek, uio_ftell   │
│ _GetMusicData      │         │ (virtual filesystem    │
│ _GetConversation..│         │  with zip, mounts)     │
│ etc.               │         └────────────────────────┘
└────────────────────┘
```

### System Boundary

- **Inbound**: C game code calls the Rust resource system through `extern "C"` FFI functions. The function names, parameter types, and return types match the C API in `reslib.h` exactly.
- **Outbound (UIO)**: All file I/O goes through UIO, which remains in C. Rust imports UIO functions via `extern "C"` blocks.
- **Outbound (Type Loaders)**: When a heap resource is first accessed, Rust calls the C-registered `loadFun` function pointer. The C loader reads the file (via UIO) and returns subsystem-specific data (DRAWABLE, FONT, etc.). Rust does not interpret this data — it stores it as an opaque `*mut c_void`.
- **Outbound (Logging)**: Rust uses the existing `log_add` bridge for all log messages.

### Global State

The C resource system uses a single global `curResourceIndex` pointer. The Rust replacement shall use a single global state protected by interior mutability (see Section 10).

---

## 3. Public API Surface

Every function listed below must be exposed via `#[no_mangle] pub extern "C"`. The C signatures are from `reslib.h` and `index.h`. The Rust FFI signatures use C-compatible types from `std::ffi` and `libc`.

### 3.1 Lifecycle

#### `InitResourceSystem`

```c
// C signature (reslib.h)
RESOURCE_INDEX InitResourceSystem(void);
```

```rust
// Rust FFI signature
#[no_mangle]
pub extern "C" fn InitResourceSystem() -> *mut ResourceIndexDesc
```

**Behavioral Contract:**
1. If an index already exists, return the existing index pointer without allocating.
2. Allocate a new `ResourceIndexDesc` containing a `HashMap<String, ResourceDesc>`.
3. Register the 14 built-in types via `InstallResTypeVectors` in order: UNKNOWNRES, STRING, INT32, BOOLEAN, COLOR, GFXRES, FONTRES, STRTAB, BINTAB, CONVERSATION, SNDRES, MUSICRES, 3DOVID, SHIP.
   - The first 5 (value types) have their `loadFun`/`freeFun`/`toString` implemented in Rust.
   - The remaining 9 (heap types) are registered later by C subsystem init code via `InstallResTypeVectors`. Until registered, they use the UNKNOWNRES fallback.
4. Store the index as the current global index.
5. Return a pointer to the index (opaque to C callers).

**Error Handling:** Returns `NULL` if allocation fails (should not happen in practice).

#### `UninitResourceSystem`

```c
// C signature (reslib.h)
void UninitResourceSystem(void);
```

```rust
#[no_mangle]
pub extern "C" fn UninitResourceSystem()
```

**Behavioral Contract:**
1. For each entry in the index: if it has loaded data and a `freeFun`, call `freeFun` to release the data.
2. Drop the HashMap and all Rust-owned allocations (descriptor strings, type handler structs).
3. Set the global index to None/NULL.
4. Safe to call multiple times (subsequent calls are no-ops).

**Error Handling:** None. Must not panic.

### 3.2 Index Loading

#### `LoadResourceIndex`

```c
// C signature (reslib.h)
void LoadResourceIndex(uio_DirHandle *dir, const char *filename, const char *prefix);
```

```rust
#[no_mangle]
pub extern "C" fn LoadResourceIndex(
    dir: *mut uio_DirHandle,
    filename: *const c_char,
    prefix: *const c_char,
)
```

**Behavioral Contract:**
1. If no index exists, call `InitResourceSystem()` to auto-initialize.
2. Open the file via `res_OpenResFile(dir, filename, "rb")`.
3. If the file cannot be opened, silently return (no error).
4. Read the entire file contents into memory.
5. Parse the contents as a property file (see Section 5).
6. For each key=value pair, prepend `prefix` to the key (if prefix is non-NULL), then call the internal `process_resource_desc(key, value)`.
7. Close the file.
8. This function may be called multiple times. Entries accumulate into the same index. Later entries with the same key replace earlier ones.

**Error Handling:** Silently returns on file-not-found. Logs warnings for parse errors.

#### `SaveResourceIndex`

```c
// C signature (reslib.h)
void SaveResourceIndex(
    uio_DirHandle *dir,
    const char *rmpfile,
    const char *root,
    BOOLEAN strip_root
);
```

```rust
#[no_mangle]
pub extern "C" fn SaveResourceIndex(
    dir: *mut uio_DirHandle,
    rmpfile: *const c_char,
    root: *const c_char,
    strip_root: c_int,  // BOOLEAN is typedef'd to int
)
```

**Behavioral Contract:**
1. Open `rmpfile` in `dir` for writing via UIO.
2. If the file cannot be opened, silently return.
3. Iterate all entries in the index.
4. For each entry where:
   - `root` is NULL, OR the key starts with the `root` prefix
   - The entry has a type handler with a non-NULL `toString` function
5. Serialize the entry as: `key = TYPE:serialized_value\n`
   - If `strip_root` is TRUE, remove the `root` prefix from the key.
   - Call `toString(&resdata, buf, 256)` to produce the serialized value.
6. Skip entries with no vtable, no toString, or no value — log a warning for missing vtable/value.
7. Close the file.

**Error Handling:** Silently returns on file open failure. Logs warnings for entries that cannot be serialized.

### 3.3 Type Registration

#### `InstallResTypeVectors`

```c
// C signature (reslib.h)
BOOLEAN InstallResTypeVectors(
    const char *res_type,
    ResourceLoadFun *loadFun,
    ResourceFreeFun *freeFun,
    ResourceStringFun *stringFun
);
```

```rust
#[no_mangle]
pub extern "C" fn InstallResTypeVectors(
    res_type: *const c_char,
    load_fun: Option<ResourceLoadFun>,
    free_fun: Option<ResourceFreeFun>,
    string_fun: Option<ResourceStringFun>,
) -> c_int  // BOOLEAN
```

Where the function pointer types are:
```rust
type ResourceLoadFun = unsafe extern "C" fn(*const c_char, *mut ResourceData);
type ResourceFreeFun = unsafe extern "C" fn(*mut c_void) -> c_int;
type ResourceStringFun = unsafe extern "C" fn(*mut ResourceData, *mut c_char, c_uint);
```

**Behavioral Contract:**
1. Allocate a `ResourceHandlers` struct with the four fields: `resType`, `loadFun`, `freeFun`, `toString`.
2. Store it in the index under key `"sys.<res_type>"`.
3. Return TRUE (1) on success, FALSE (0) on allocation failure.

**Error Handling:** Returns FALSE on failure.

### 3.4 Resource Access

#### `res_GetResource`

```c
// C signature (reslib.h)
void *res_GetResource(RESOURCE res);
// Where RESOURCE is `const char *`
```

```rust
#[no_mangle]
pub extern "C" fn res_GetResource(res: *const c_char) -> *mut c_void
```

**Behavioral Contract:**
1. If `res` is NULL, log warning "Trying to get null resource", return NULL.
2. Look up the key in the index.
3. If not found, log warning "Trying to get undefined resource \<key\>", return NULL.
4. If `resdata.ptr` is NULL (not yet loaded), call `loadResourceDesc(desc)`:
   - Calls `vtable->loadFun(desc->fname, &desc->resdata)`.
   - The C load function populates `resdata.ptr` with the loaded data.
5. If `resdata.ptr` is still NULL after loading (load failed), return NULL without incrementing refcount.
6. Increment `refcount` by 1.
7. Return `resdata.ptr`.

**Error Handling:** Returns NULL on any failure. Logs warnings for null key, undefined key, and load failures.

#### `res_DetachResource`

```c
// C signature (reslib.h)
void *res_DetachResource(RESOURCE res);
```

```rust
#[no_mangle]
pub extern "C" fn res_DetachResource(res: *const c_char) -> *mut c_void
```

**Behavioral Contract:**
1. Look up the key in the index.
2. If not found, log warning and return NULL.
3. If the resource is a non-heap type (`freeFun` is NULL), log warning and return NULL.
4. If the resource is not loaded (`resdata.ptr` is NULL), log warning and return NULL.
5. If `refcount > 1`, log warning "trying to detach a resource referenced N times" and return NULL.
6. Save `resdata.ptr` to a local variable.
7. Set `resdata.ptr = NULL` and `refcount = 0`.
8. Return the saved pointer.

After detach, the caller owns the data. The next `res_GetResource` for this key will trigger a fresh load.

**Error Handling:** Returns NULL on any guard failure. Logs appropriate warnings.

#### `res_FreeResource`

```c
// C signature (reslib.h)
void res_FreeResource(RESOURCE res);
```

```rust
#[no_mangle]
pub extern "C" fn res_FreeResource(res: *const c_char)
```

**Behavioral Contract:**
1. Look up the key in the index. If not found, log warning and return.
2. If `freeFun` is NULL (non-heap type), log warning "trying to free a non-heap resource" and return.
3. If `resdata.ptr` is NULL (not loaded), log warning "trying to free not loaded resource" and return.
4. If `refcount == 0`, log warning "freeing an unreferenced resource."
5. If `refcount > 0`, decrement by 1.
6. If `refcount` reaches 0: call `vtable->freeFun(resdata.ptr)`, then set `resdata.ptr = NULL`.

**Error Handling:** Logs warnings for all error conditions. Never panics.

#### `res_Remove`

```c
// C signature (reslib.h)
BOOLEAN res_Remove(const char *key);
```

```rust
#[no_mangle]
pub extern "C" fn res_Remove(key: *const c_char) -> c_int
```

**Behavioral Contract:**
1. Look up the key in the index.
2. If found:
   a. If `resdata.ptr` is not NULL:
      - If `refcount > 0`, log warning "Replacing while live".
      - If the entry has a vtable with a `freeFun`, call `freeFun(resdata.ptr)`.
   b. Remove the entry from the HashMap, dropping the Rust-owned descriptor.
3. Return TRUE (1) if the key was found and removed, FALSE (0) otherwise.

**Error Handling:** Logs warning when replacing a live resource.

### 3.5 Additional Value Access

#### `res_GetIntResource`

```c
// C signature (reslib.h)
DWORD res_GetIntResource(RESOURCE res);
```

```rust
#[no_mangle]
pub extern "C" fn res_GetIntResource(res: *const c_char) -> u32
```

**Behavioral Contract:** Look up key, return `resdata.num`. No type checking. No refcount increment. Returns 0 if key not found.

#### `res_GetBooleanResource`

```c
// C signature (reslib.h)
BOOLEAN res_GetBooleanResource(RESOURCE res);
```

```rust
#[no_mangle]
pub extern "C" fn res_GetBooleanResource(res: *const c_char) -> c_int
```

**Behavioral Contract:** Returns `res_GetIntResource(key) != 0`.

#### `res_GetResourceType`

```c
// C signature (reslib.h)
const char *res_GetResourceType(RESOURCE res);
```

```rust
#[no_mangle]
pub extern "C" fn res_GetResourceType(res: *const c_char) -> *const c_char
```

**Behavioral Contract:** Returns `desc->vtable->resType` for the given key. Returns NULL if key is null or undefined.

#### `CountResourceTypes`

```c
// C signature (reslib.h)
COUNT CountResourceTypes(void);
```

```rust
#[no_mangle]
pub extern "C" fn CountResourceTypes() -> u16  // COUNT is WORD/u16
```

**Behavioral Contract:** Returns the number of registered type handlers (entries with `"sys."` prefix).

### 3.6 Config Getters

#### `res_HasKey`

```c
BOOLEAN res_HasKey(const char *key);
```

```rust
#[no_mangle]
pub extern "C" fn res_HasKey(key: *const c_char) -> c_int
```

**Behavioral Contract:** Returns TRUE (1) if the key exists in the index, FALSE (0) otherwise.

#### `res_IsString`

```c
BOOLEAN res_IsString(const char *key);
```

```rust
#[no_mangle]
pub extern "C" fn res_IsString(key: *const c_char) -> c_int
```

**Behavioral Contract:** Returns TRUE if key exists AND its type handler's `resType` field equals `"STRING"`.

#### `res_IsInteger`

```c
BOOLEAN res_IsInteger(const char *key);
```

```rust
#[no_mangle]
pub extern "C" fn res_IsInteger(key: *const c_char) -> c_int
```

**Behavioral Contract:** Returns TRUE if key exists AND `resType == "INT32"`.

#### `res_IsBoolean`

```c
BOOLEAN res_IsBoolean(const char *key);
```

```rust
#[no_mangle]
pub extern "C" fn res_IsBoolean(key: *const c_char) -> c_int
```

**Behavioral Contract:** Returns TRUE if key exists AND `resType == "BOOLEAN"`.

#### `res_IsColor`

```c
BOOLEAN res_IsColor(const char *key);
```

```rust
#[no_mangle]
pub extern "C" fn res_IsColor(key: *const c_char) -> c_int
```

**Behavioral Contract:** Returns TRUE if key exists AND `resType == "COLOR"`.

#### `res_GetString`

```c
const char *res_GetString(const char *key);
```

```rust
#[no_mangle]
pub extern "C" fn res_GetString(key: *const c_char) -> *const c_char
```

**Behavioral Contract:**
1. Look up key. If not found or not STRING type, return a pointer to a static empty string `""`.
2. Return `resdata.str` — a pointer to the string value stored in the descriptor.
3. The returned pointer is valid until the entry is modified via `res_PutString` or removed via `res_Remove`.

**Error Handling:** Returns `""` (empty C string) on any failure.

#### `res_GetInteger`

```c
int res_GetInteger(const char *key);
```

```rust
#[no_mangle]
pub extern "C" fn res_GetInteger(key: *const c_char) -> c_int
```

**Behavioral Contract:** Returns `resdata.num` interpreted as `int`. Returns 0 if key not found or not INT32.

#### `res_GetBoolean`

```c
BOOLEAN res_GetBoolean(const char *key);
```

```rust
#[no_mangle]
pub extern "C" fn res_GetBoolean(key: *const c_char) -> c_int
```

**Behavioral Contract:** Returns `resdata.num != 0` as BOOLEAN. Returns FALSE (0) if key not found or not BOOLEAN.

#### `res_GetColor`

```c
Color res_GetColor(const char *key);
```

```rust
#[no_mangle]
pub extern "C" fn res_GetColor(key: *const c_char) -> Color
```

Where `Color` is the C-compatible struct:
```rust
#[repr(C)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}
```

**Behavioral Contract:**
1. Look up key. If not found or not COLOR type, return `Color { r: 0, g: 0, b: 0, a: 0 }`.
2. Unpack `resdata.num` as `(r << 24 | g << 16 | b << 8 | a)`.
3. Return the Color struct.

### 3.7 Config Setters

#### `res_PutString`

```c
void res_PutString(const char *key, const char *value);
```

```rust
#[no_mangle]
pub extern "C" fn res_PutString(key: *const c_char, value: *const c_char)
```

**Behavioral Contract:**
1. If key does not exist or is not STRING type: create entry via `process_resource_desc(key, "STRING:undefined")`.
2. Update the entry's `fname` and `resdata.str` to the new value.
   - In Rust, this means replacing the owned String. There is no in-place-vs-realloc distinction as in C — Rust owns the String.
3. Both `fname` and `resdata.str` point to the same storage (in C they alias; in Rust the descriptor holds a single String and the resdata references it).

#### `res_PutInteger`

```c
void res_PutInteger(const char *key, int value);
```

```rust
#[no_mangle]
pub extern "C" fn res_PutInteger(key: *const c_char, value: c_int)
```

**Behavioral Contract:**
1. If key does not exist or is not INT32 type: create entry via `process_resource_desc(key, "INT32:0")`.
2. Set `resdata.num = value as u32`.

#### `res_PutBoolean`

```c
void res_PutBoolean(const char *key, BOOLEAN value);
```

```rust
#[no_mangle]
pub extern "C" fn res_PutBoolean(key: *const c_char, value: c_int)
```

**Behavioral Contract:**
1. If key does not exist or is not BOOLEAN type: create entry via `process_resource_desc(key, "BOOLEAN:false")`.
2. Set `resdata.num = value as u32`.

#### `res_PutColor`

```c
void res_PutColor(const char *key, Color value);
```

```rust
#[no_mangle]
pub extern "C" fn res_PutColor(key: *const c_char, value: Color)
```

**Behavioral Contract:**
1. If key does not exist or is not COLOR type: create entry via `process_resource_desc(key, "COLOR:rgb(0, 0, 0)")`.
2. Set `resdata.num = (value.r as u32) << 24 | (value.g as u32) << 16 | (value.b as u32) << 8 | (value.a as u32)`.

### 3.8 File I/O Wrappers

These functions wrap UIO operations. They are thin pass-throughs that delegate to C UIO functions.

#### `res_OpenResFile`

```c
uio_Stream *res_OpenResFile(uio_DirHandle *dir, const char *filename, const char *mode);
```

```rust
#[no_mangle]
pub extern "C" fn res_OpenResFile(
    dir: *mut uio_DirHandle,
    filename: *const c_char,
    mode: *const c_char,
) -> *mut uio_Stream
```

**Behavioral Contract:**
1. If the path is a directory (checked via `uio_stat`), return the sentinel value `!0 as *mut uio_Stream` (all bits set).
2. Otherwise, call `uio_fopen(dir, filename, mode)` and return the result.

#### `res_CloseResFile`

```c
BOOLEAN res_CloseResFile(uio_Stream *fp);
```

```rust
#[no_mangle]
pub extern "C" fn res_CloseResFile(fp: *mut uio_Stream) -> c_int
```

**Behavioral Contract:**
1. If `fp` is NULL, return TRUE.
2. If `fp` is the sentinel value (`!0`), return TRUE (no-op).
3. Otherwise, call `uio_fclose(fp)` and return TRUE on success.

#### `LoadResourceFromPath`

```c
void *LoadResourceFromPath(const char *pathname, ResourceLoadFileFun fn);
```

```rust
#[no_mangle]
pub extern "C" fn LoadResourceFromPath(
    pathname: *const c_char,
    load_fn: Option<unsafe extern "C" fn(*mut uio_Stream, u32) -> *mut c_void>,
) -> *mut c_void
```

**Behavioral Contract:**
1. Open `pathname` via `res_OpenResFile(contentDir, pathname, "rb")`.
2. If open fails, log warning and return NULL.
3. Get file length via `LengthResFile(stream)`.
4. If length is 0, log warning, close file, return NULL.
5. Set `_cur_resfile_name = pathname` (global, for loaders that need the current filename).
6. Call `load_fn(stream, length)` to produce the loaded data.
7. Set `_cur_resfile_name = NULL`.
8. Close the file via `res_CloseResFile`.
9. Return the loaded data pointer.

#### Other File I/O Functions

The following are thin wrappers around UIO and shall be exposed with identical signatures:

| Function | C Signature | Behavior |
|----------|-------------|----------|
| `ReadResFile` | `size_t ReadResFile(void *buf, size_t size, size_t count, uio_Stream *fp)` | Calls `uio_fread` |
| `WriteResFile` | `size_t WriteResFile(const void *buf, size_t size, size_t count, uio_Stream *fp)` | Calls `uio_fwrite` |
| `GetResFileChar` | `int GetResFileChar(uio_Stream *fp)` | Calls `uio_getc` |
| `PutResFileChar` | `int PutResFileChar(char ch, uio_Stream *fp)` | Calls `uio_putc` |
| `PutResFileNewline` | `int PutResFileNewline(uio_Stream *fp)` | Writes `\n` (or `\r\n` on Windows) |
| `SeekResFile` | `long SeekResFile(uio_Stream *fp, long offset, int whence)` | Calls `uio_fseek` |
| `TellResFile` | `long TellResFile(uio_Stream *fp)` | Calls `uio_ftell` |
| `LengthResFile` | `size_t LengthResFile(uio_Stream *fp)` | Returns 1 for sentinel, else `uio_fstat` size |
| `DeleteResFile` | `BOOLEAN DeleteResFile(uio_DirHandle *dir, const char *filename)` | Calls `uio_unlink` |
| `GetResourceData` | `void *GetResourceData(uio_Stream *fp, DWORD length)` | Reads DWORD prefix; if `~0`, reads uncompressed data; else logs LZ warning, returns NULL |
| `FreeResourceData` | `BOOLEAN FreeResourceData(void *data)` | Frees via appropriate allocator, returns TRUE |

---

## 4. Internal Architecture

### 4.1 ResourceIndexDesc

The top-level container, equivalent to the C `resource_index_desc`:

- Contains a `HashMap<String, ResourceDesc>` mapping string keys to resource descriptors.
- A single global instance exists, protected by interior mutability.
- Type handler registrations are stored in the same map with keys prefixed by `"sys."` (e.g., `"sys.GFXRES"`), matching C behavior.

### 4.2 ResourceDesc

Each entry in the index, equivalent to the C `resource_desc`:

| Field | Type | Description |
|-------|------|-------------|
| `fname` | `String` | File path (for heap types) or raw value string (for value types). Heap-allocated, owned by Rust. |
| `vtable` | `*const ResourceHandlers` | Pointer to the type handler. NULL for type-registration entries themselves. |
| `resdata` | `ResourceData` | Union-like: loaded data pointer, numeric value, or string pointer. |
| `refcount` | `u32` | Reference count. 0 for unloaded or value types. |

### 4.3 ResourceData

Equivalent to the C `RESOURCE_DATA` union. Rust represents this as a `#[repr(C)]` union:

```rust
#[repr(C)]
pub union ResourceData {
    pub num: u32,        // For INT32, BOOLEAN, COLOR
    pub ptr: *mut c_void, // For heap-loaded resources
    pub str_ptr: *const c_char, // For STRING (aliases fname)
}
```

### 4.4 ResourceHandlers

The type dispatch table, equivalent to C `resource_handlers`:

```rust
#[repr(C)]
pub struct ResourceHandlers {
    pub res_type: *const c_char,   // "GFXRES", "STRING", etc. (static lifetime)
    pub load_fun: Option<ResourceLoadFun>,
    pub free_fun: Option<ResourceFreeFun>,
    pub to_string: Option<ResourceStringFun>,
}
```

### 4.5 Type Handler Registry

Type handlers are stored as entries in the main HashMap with key `"sys.<TYPE>"`. The `ResourceDesc` for a type handler entry has:
- `vtable = NULL` (this is how type entries are distinguished from resource entries)
- `resdata.ptr` = pointer to the `ResourceHandlers` struct

This mirrors the C design exactly, where `InstallResTypeVectors("GFXRES", ...)` creates an entry under key `"sys.GFXRES"`.

### 4.6 Two-Phase Loading

**Phase 1: Index Parse** — When `LoadResourceIndex` is called, each `key = TYPE:path` line is processed:

1. Split the value on the first `:` to extract type name and path.
2. Look up `"sys.<TYPE>"` in the HashMap to find the `ResourceHandlers`.
3. Create a `ResourceDesc` with `fname = path`, `vtable = handlers`, `refcount = 0`.
4. **Value types** (`freeFun` is None/NULL): Immediately call `loadFun(fname, &resdata)` to parse the value (e.g., parse "true" into `resdata.num = 1`). No file I/O.
5. **Heap types** (`freeFun` is Some/non-NULL): Set `resdata.ptr = NULL`. Loading is deferred.
6. Insert into HashMap. If the key already exists, remove the old entry first (via the equivalent of `res_Remove`).

**Phase 2: Lazy Load** — When `res_GetResource(key)` is called and `resdata.ptr` is NULL:

1. Call `vtable->loadFun(fname, &resdata)`.
2. The load function (a C function pointer for heap types) opens the file, parses its format, and stores the result in `resdata.ptr`.
3. If loading succeeds, increment `refcount`.

### 4.7 Value Types vs Heap Types

| Property | Value Types | Heap Types |
|----------|-------------|------------|
| Types | STRING, INT32, BOOLEAN, COLOR, UNKNOWNRES | GFXRES, FONTRES, STRTAB, BINTAB, CONVERSATION, SNDRES, MUSICRES, 3DOVID, SHIP |
| `freeFun` | NULL | Non-NULL |
| When loaded | Immediately during index parse | Lazily on first `res_GetResource()` |
| `resdata` holds | `.num` or `.str_ptr` | `.ptr` (opaque data) |
| Ref counting | Not applicable | Applicable |
| Accessed via | `res_Get{String,Integer,Boolean,Color}` | `res_GetResource` |

---

## 5. .rmp / .cfg Parsing

### 5.1 File Format

The same parser handles `.rmp` (resource maps), `.cfg` (configuration), and `.key` (key binding) files. The format is identical across all three:

```
# Comment line
key = TYPE:value_or_path
```

### 5.2 Parsing Rules

The parser operates on the entire file content loaded into memory. It must implement these exact rules, matching the C `PropFile_from_string` behavior:

1. **Skip leading whitespace** on each line (spaces, tabs, and newlines between entries).
2. **Comments**: If the first non-whitespace character is `#`, skip to end of line.
3. **Key extraction**: Read characters until `=`, `\n`, `#`, or EOF.
   - If no `=` is found before `\n` or EOF: log warning "Key without value" and skip the line.
   - If bare key at EOF: log warning "Bare keyword at EOF" and stop.
4. **Key trimming**: The key is the characters before `=`, with trailing whitespace removed.
5. **Value extraction**: Skip `=`, then skip whitespace (spaces/tabs only, not past `#` or `\n`).
6. **Value termination**: Value extends until `#`, `\n`, or EOF. Trailing whitespace is trimmed.
7. **Inline comments**: A `#` within the value portion terminates the value (everything from `#` onward is a comment).
8. **Prefix**: If a prefix was provided to `LoadResourceIndex`, prepend it to the key. The prefix+key string is limited to 255 characters.
9. **Handler invocation**: Call `process_resource_desc(final_key, value)` for each parsed pair.

### 5.3 Key Case Sensitivity

**Resource keys are case-sensitive.** The parser must preserve the original case of keys exactly as they appear in the file. This matches the C `CharHashTable` behavior.

### 5.4 Value Processing (process_resource_desc)

For each `key = value` pair:

1. Split `value` on the first `:` character.
   - Left side = type name (e.g., `"GFXRES"`)
   - Right side = path or literal value (e.g., `"base/comm/arilou/arilou.ani"`)
2. If no `:` is found: log warning, treat type as `"UNKNOWNRES"`, use entire value as path.
3. Construct the type lookup key: `"sys.<TYPE>"` (e.g., `"sys.GFXRES"`).
4. Look up the type handler in the HashMap.
   - If not found: log warning "Illegal type", fall back to `"sys.UNKNOWNRES"`.
5. Create a `ResourceDesc` as described in Section 4.6.
6. If key already exists in the map: remove old entry via `res_Remove` logic, insert new.

### 5.5 Examples

**`uqm.rmp` (content resources, no prefix):**
```
comm.arilou.graphics = GFXRES:base/comm/arilou/arilou.ani
```
→ Key: `"comm.arilou.graphics"`, Type: `"GFXRES"`, Path: `"base/comm/arilou/arilou.ani"`

**`uqm.cfg` (config, prefix `"config."`):**
```
sfxvol = INT32:20
```
→ Key: `"config.sfxvol"`, Type: `"INT32"`, Path/Value: `"20"`
→ Immediately parsed: `resdata.num = 20`

**`3dovideo.rmp` (embedded colons in value):**
```
slides.spins.00 = 3DOVID:addons/3dovideo/ships/ship00.duk:addons/3dovideo/ships/spin.aif:addons/3dovideo/ships/ship00.aif:89
```
→ Key: `"slides.spins.00"`, Type: `"3DOVID"`, Path: `"addons/3dovideo/ships/ship00.duk:addons/3dovideo/ships/spin.aif:addons/3dovideo/ships/ship00.aif:89"`
(Only the first `:` is the type separator. All subsequent colons are part of the path, parsed by the 3DOVID loader.)

**`menu.key` (key bindings, prefix `"menu."`):**
```
up.1 = STRING:key Up
```
→ Key: `"menu.up.1"`, Type: `"STRING"`, Value: `"key Up"`
→ Immediately stored: `resdata.str = "key Up"`

---

## 6. Type-Specific Resource Loading

### 6.1 Dispatch Architecture

The Rust resource system does NOT implement type-specific binary format parsers. All heap-type loaders remain in C. The dispatch works as follows:

1. C subsystem init code (e.g., `InstallGraphicResTypes()`) calls `InstallResTypeVectors` with C function pointers.
2. Rust stores these function pointers in the `ResourceHandlers` struct.
3. When `res_GetResource` triggers a lazy load, Rust calls the C `loadFun` via the stored function pointer.
4. The C loader opens the file (via UIO), parses the binary format, and writes the result to `resdata.ptr`.
5. Rust stores the opaque pointer. It does not dereference or interpret it.

### 6.2 Built-in Value Type Loaders (Implemented in Rust)

These five types have their load/free/toString functions implemented in Rust:

| Type | loadFun | freeFun | toString | Storage |
|------|---------|---------|----------|---------|
| `UNKNOWNRES` | Copy `fname` to `resdata.str` | NULL | NULL | `resdata.str` aliases `fname` |
| `STRING` | Copy `fname` to `resdata.str` | NULL | Write `resdata.str` to buf | `resdata.str` aliases `fname` |
| `INT32` | Parse `fname` as integer via `atoi()` equivalent, store in `resdata.num` | NULL | Format `resdata.num` as decimal string | `resdata.num` |
| `BOOLEAN` | Case-insensitive compare `fname` to `"true"` → `resdata.num = 1`, else `0` | NULL | Write `"true"` or `"false"` | `resdata.num` |
| `COLOR` | Parse color descriptor (see Section 7.2), store packed RGBA in `resdata.num` | NULL | Format as `rgb()`/`rgba()` (see Section 7.3) | `resdata.num` |

### 6.3 External Heap Type Loaders (Registered from C)

These types have their loaders registered by C subsystem initialization code. Rust stores the function pointers and calls them when needed:

| Type | C Registration Call | Load Pattern |
|------|-------------------|--------------|
| `GFXRES` | `InstallGraphicResTypes()` | Two-tier: `GetCelFileData` → `LoadResourceFromPath(path, _GetCelData)` |
| `FONTRES` | `InstallGraphicResTypes()` | Two-tier: `GetFontFileData` → `LoadResourceFromPath(path, _GetFontData)` |
| `STRTAB` | `InstallStringTableResType()` | Two-tier: `GetStringTableFileData` → `LoadResourceFromPath(path, _GetStringData)` |
| `BINTAB` | `InstallStringTableResType()` | Two-tier: `GetBinaryTableFileData` → `LoadResourceFromPath(path, _GetBinaryTableData)` |
| `CONVERSATION` | `InstallStringTableResType()` | Direct: `_GetConversationData(path, resdata)` — opens files itself |
| `SNDRES` | `InstallAudioResTypes()` | Two-tier: `GetSoundBankFileData` → `LoadResourceFromPath(path, _GetSoundBankData)` |
| `MUSICRES` | `InstallAudioResTypes()` | Two-tier: `GetMusicFileData` → `LoadResourceFromPath(path, _GetMusicData)` |
| `3DOVID` | `InstallVideoResType()` | Direct: `GetLegacyVideoData(path, resdata)` — parses path string, no file I/O |
| `SHIP` | `InstallCodeResType()` | Direct: `GetCodeResData(path, resdata)` — converts number to init function |

### 6.4 Calling C Loaders from Rust

When Rust calls a C load function pointer:

1. The `fname` string must be passed as a valid `*const c_char` (null-terminated).
2. The `resdata` must be passed as a valid `*mut ResourceData`.
3. The call is `unsafe` and must be wrapped in `catch_unwind` is not applicable (C functions do not unwind via Rust panics). The `unsafe` block is sufficient.
4. After the call, Rust reads `resdata.ptr` to check if loading succeeded.

### 6.5 Calling C Free Functions from Rust

When `res_FreeResource` or `res_Remove` needs to free loaded data:

1. Call `vtable->freeFun(resdata.ptr)` via the stored function pointer.
2. The C free function deallocates the subsystem-specific data.
3. Rust sets `resdata.ptr = NULL` after the call.

---

## 7. Key-Value Config System

### 7.1 Overview

The config system uses the same index and descriptor infrastructure as resource loading. Config entries are value types (STRING, INT32, BOOLEAN, COLOR) that are parsed immediately at index-load time. They are accessed via dedicated getter/setter functions and can be persisted via `SaveResourceIndex`.

### 7.2 Color Parsing

The color parser must support three formats, matching the C `DescriptorToColor` function exactly:

#### `rgb(r, g, b)`
- 8-bit components (0-255)
- Alpha defaults to 0xFF
- Components support C integer formats: decimal (`128`), hex (`0x80`), octal (`0200`)
- Whitespace around components and after parentheses is allowed

#### `rgba(r, g, b, a)`
- 8-bit components (0-255) for all four channels
- Same numeric format support as `rgb()`

#### `rgb15(r, g, b)`
- 5-bit components (0-31)
- Converted to 8-bit via the CC5TO8 formula: `((x) << 3) | ((x) >> 2)` (equivalent to `x * 255 / 31` rounded)
- Alpha defaults to 0xFF

#### Clamping and Warnings
- If a component is negative, clamp to 0 and log a warning.
- If a component exceeds the maximum for its bit depth (255 for 8-bit, 31 for 5-bit), clamp to the maximum and log a warning.

#### Unrecognized Format
- If the string cannot be parsed as any of the three formats, log an error and store `0x00000000` (all zeros).

#### Internal Storage
- Packed into a 32-bit DWORD as `(R << 24) | (G << 16) | (B << 8) | A`.

**Note:** The C code has `#rrggbb` hex format disabled (`#if 0`) because `#` starts a comment in the property file parser. The Rust system must NOT support `#rrggbb` for `.rmp`/`.cfg` parsing compatibility. (The existing Rust code supports `#RRGGBB` — this is a bug that must be fixed.)

### 7.3 Color Serialization

When writing colors via `toString` (for `SaveResourceIndex`):

- If alpha == 0xFF: `rgb(0x%02x, 0x%02x, 0x%02x)` (e.g., `rgb(0x1a, 0x00, 0x1a)`)
- If alpha != 0xFF: `rgba(0x%02x, 0x%02x, 0x%02x, 0x%02x)` (e.g., `rgba(0x1a, 0x00, 0x1a, 0x80)`)

Components are always written in lowercase hex with `0x` prefix and zero-padded to 2 digits.

### 7.4 Integer Parsing

The INT32 `loadFun` parses the descriptor string using C `atoi()` semantics:
- Leading whitespace is skipped.
- Optional `+` or `-` sign.
- Decimal digits are read until a non-digit character.
- Returns 0 for non-numeric strings.

### 7.5 Boolean Parsing

The BOOLEAN `loadFun` performs a case-insensitive comparison:
- `"true"` (any case) → `resdata.num = 1` (TRUE)
- Any other value → `resdata.num = 0` (FALSE)

### 7.6 String Storage

The STRING type aliases `resdata.str` to `fname`:
- The `loadFun` (`UseDescriptorAsRes`) sets `resdata.str = fname`.
- In Rust, this means the descriptor holds one String and `resdata` references it.
- `res_GetString` returns a `*const c_char` pointing into this storage.
- `res_PutString` replaces the entire String.

### 7.7 Auto-Creation on Put

All `res_Put*` functions auto-create the entry if it doesn't exist or is the wrong type:
- `res_PutString`: creates `"STRING:undefined"`, then updates.
- `res_PutInteger`: creates `"INT32:0"`, then updates.
- `res_PutBoolean`: creates `"BOOLEAN:false"`, then updates.
- `res_PutColor`: creates `"COLOR:rgb(0, 0, 0)"`, then updates.

---

## 8. Path Resolution and UIO Integration

### 8.1 UIO as the File I/O Layer

All file operations go through UIO (`libs/uio/`), which remains in C. The Rust resource system imports UIO functions via `extern "C"` blocks:

```rust
extern "C" {
    fn uio_fopen(dir: *mut uio_DirHandle, path: *const c_char, mode: *const c_char) -> *mut uio_Stream;
    fn uio_fclose(fp: *mut uio_Stream) -> c_int;
    fn uio_fread(buf: *mut c_void, size: usize, count: usize, fp: *mut uio_Stream) -> usize;
    fn uio_fwrite(buf: *const c_void, size: usize, count: usize, fp: *mut uio_Stream) -> usize;
    fn uio_fseek(fp: *mut uio_Stream, offset: c_long, whence: c_int) -> c_int;
    fn uio_ftell(fp: *mut uio_Stream) -> c_long;
    fn uio_getc(fp: *mut uio_Stream) -> c_int;
    fn uio_putc(c: c_int, fp: *mut uio_Stream) -> c_int;
    fn uio_stat(dir: *mut uio_DirHandle, path: *const c_char, sb: *mut stat) -> c_int;
    fn uio_unlink(dir: *mut uio_DirHandle, path: *const c_char) -> c_int;
}
```

UIO handles are opaque pointers (`*mut c_void` effectively). Rust never dereferences them.

### 8.2 Directory Handles

Two global directory handles are provided by C code:

```rust
extern "C" {
    static contentDir: *mut uio_DirHandle;  // Game content root
    static configDir: *mut uio_DirHandle;   // User config directory
}
```

- `contentDir`: Resolved by C startup code. Points to the game content root which may include layered mounts (base content, addon directories, zip archives).
- `configDir`: Resolved by C startup code. Points to the user config directory (e.g., `~/.uqm`).

### 8.3 Content Path Resolution

Resource paths in `.rmp` files are relative to `contentDir`:
- `"base/comm/arilou/arilou.ani"` → `res_OpenResFile(contentDir, "base/comm/arilou/arilou.ani", "rb")`
- UIO resolves the path across all mounted content (base directory, addon directories, zip packages).

### 8.4 Config Path Resolution

Config files are accessed via `configDir`:
- `LoadResourceIndex(configDir, "uqm.cfg", "config.")` reads from the config directory.
- `SaveResourceIndex(configDir, "uqm.cfg", "config.", TRUE)` writes to the config directory.

### 8.5 Addon Pack Layering

Addon content override works through two mechanisms, both handled transparently:

1. **Key override**: Addon `.rmp` files are loaded after base `.rmp` files. Entries with the same key replace earlier entries in the HashMap (last-writer-wins).
2. **Shadow content**: Addons can mount `shadow-content/` directories over the content root via UIO. This is handled by UIO mount ordering, not by the resource system.

The Rust resource system does not need special addon logic beyond supporting multiple `LoadResourceIndex` calls with last-writer-wins semantics.

### 8.6 Directory Sentinel

When `res_OpenResFile` encounters a directory (instead of a file), it returns the sentinel `(uio_Stream *)~0` (all bits set). This is used by font loading where the path references a directory of character images. The sentinel propagates through:
- `LengthResFile(sentinel)` → returns 1
- `res_CloseResFile(sentinel)` → no-op, returns TRUE
- The font loader detects the sentinel and handles directory-based fonts specially.

---

## 9. Reference Counting and Memory Management

### 9.1 Refcount Semantics

Reference counting applies only to **heap-type** resources (`freeFun` is non-NULL):

| Operation | Effect on refcount |
|-----------|-------------------|
| `res_GetResource` (successful) | `refcount += 1` |
| `res_FreeResource` | `refcount -= 1`; if reaches 0, call `freeFun` and set `ptr = NULL` |
| `res_DetachResource` (successful) | `refcount = 0`, `ptr = NULL` |
| `newResourceDesc` (initial) | `refcount = 0` |

Value-type resources (STRING, INT32, BOOLEAN, COLOR) do not participate in reference counting. Their `refcount` field is always 0.

### 9.2 res_DetachResource Semantics

Detach transfers ownership of the loaded data from the resource system to the caller:

1. The `resdata.ptr` is returned to the caller.
2. The descriptor's `resdata.ptr` is set to NULL.
3. The descriptor's `refcount` is set to 0.
4. The **caller** is now responsible for freeing the data (typically via the subsystem's release function).
5. The next `res_GetResource` for this key will trigger a fresh load from disk.

Detach fails (returns NULL) if:
- Key not found
- Non-heap type
- Not loaded
- `refcount > 1` (cannot detach a multiply-referenced resource)

### 9.3 Load*Instance Pattern

All convenience loaders (`LoadGraphicInstance`, `LoadSoundInstance`, etc.) follow this pattern:

```
res_GetResource(key)     →  refcount becomes 1, ptr returned
res_DetachResource(key)  →  refcount becomes 0, ptr returned to caller, descriptor cleared
```

This means every `Load*Instance` call produces a fresh copy from disk if the resource was previously detached. The resource system does NOT cache detached resources.

### 9.4 Cross-Language Memory Ownership

| Data | Allocated by | Freed by | Mechanism |
|------|-------------|----------|-----------|
| `ResourceDesc` (descriptor struct) | Rust | Rust | Dropped when removed from HashMap |
| `fname` (path/value string) | Rust | Rust | Owned `String`, dropped with descriptor |
| `ResourceHandlers` (type vtable) | Rust | Rust | Dropped when type is unregistered or system uninits |
| `resdata.ptr` (loaded resource data) | C (via `loadFun`) | C (via `freeFun`) | Rust calls `freeFun` when refcount reaches 0 or on `res_Remove` |
| Strings returned by `res_GetString` | Rust | Rust | Pointer into descriptor's `fname`; valid until entry is modified/removed |

**Critical rule**: Rust must never call `free()` or `drop()` on `resdata.ptr` for heap types. Only the registered `freeFun` may be used. Conversely, C must never call `free()` on Rust-owned strings (descriptors, type names).

---

## 10. Thread Safety

### 10.1 Threading Model

The C resource system is single-threaded. There are no locks, mutexes, or atomic operations anywhere in the C resource code. All access occurs from the main thread.

The Rust replacement must be safe for single-threaded use but shall use interior mutability to satisfy Rust's safety requirements.

### 10.2 Interior Mutability Pattern

The global resource index shall be stored in a structure accessible from `extern "C"` functions. The recommended pattern (used elsewhere in the project) is:

- A `static` variable with `Mutex<Option<ResourceState>>` or `RwLock<Option<ResourceState>>`.
- Each FFI function acquires the lock, performs its operation, and releases the lock.
- Lock acquisition must not block indefinitely (single-threaded callers will never contend).
- If a lock is poisoned (due to a panic in a previous call), the FFI function shall return a safe default (NULL/0/FALSE) and log an error.

### 10.3 No Concurrent Access Expected

C callers access the resource system from a single thread. The Rust mutex/rwlock is for Rust's safety model, not for actual concurrency. Performance overhead of locking should be minimal (uncontended mutex acquisition).

### 10.4 Global Mutable State

The following global mutable state must be managed:

- `curResourceIndex`: The current resource index (HashMap + type registry).
- `_cur_resfile_name`: Set during `LoadResourceFromPath`, read by type-specific loaders. Must be accessible from C.

---

## 11. Error Handling

### 11.1 FFI Boundary Contract

All `extern "C"` functions must:

1. **Never panic.** Any Rust panic that would cross the FFI boundary is undefined behavior. All FFI functions must catch potential panics (via `std::panic::catch_unwind` if calling Rust code that could panic) or ensure their code paths cannot panic.
2. **Validate all pointer parameters.** Check for NULL before dereferencing. Check that `*const c_char` points to valid UTF-8 (or handle non-UTF-8 gracefully).
3. **Return safe defaults on error:**
   - Functions returning `*mut c_void` / `*const c_char`: return `NULL`
   - Functions returning `c_int` (BOOLEAN): return `0` (FALSE)
   - Functions returning numeric types: return `0`
   - Functions returning `Color`: return `Color { r: 0, g: 0, b: 0, a: 0 }`

### 11.2 Internal Error Handling

Internally, Rust code uses `Result<T, E>` for operations that can fail. At the FFI boundary, errors are converted to the appropriate C-compatible return value and logged.

### 11.3 Logging

All warnings and errors are logged via the existing `log_add` bridge (available in the project as `rust_bridge_log_msg` or equivalent). Log messages should match the C system's messages where possible for behavioral parity.

Log levels:
- `log_Warning`: Parse errors, undefined keys, type mismatches, refcount anomalies
- `log_Error`: Unrecoverable errors (failed to open critical files)
- `log_Debug`: Initialization, type registration, index loading progress

---

## 12. Compatibility Constraints

### 12.1 Binary Compatibility

- All `extern "C"` functions must have the exact same symbol names as the C functions they replace.
- Parameter types must be ABI-compatible (same size, alignment, and semantics).
- `#[repr(C)]` must be used on all structs and unions that cross the FFI boundary.

### 12.2 Config File Compatibility

- Config files (`.cfg`) written by the C system must be readable by the Rust system and vice versa.
- The `.rmp` parser must produce identical results to the C `PropFile_from_string`.
- `SaveResourceIndex` output must be semantically identical to C output (key order may differ since HashMap iteration is unordered, but each line must have the same format: `key = TYPE:serialized_value\n`).

### 12.3 Save Game Compatibility

- Save games do not directly contain resource system state, but they reference config values persisted via `SaveResourceIndex`.
- A game saved with C and loaded with Rust (or vice versa) must work correctly.

### 12.4 C Caller Compatibility

- All existing C callers (200+ call sites) must work without modification.
- The `Load*Instance` convenience macros in `nameref.h` call `res_GetResource` and `res_DetachResource` — both must work identically.
- The `InstallResTypeVectors` calls from C subsystem init code must work — Rust must correctly store and later invoke C function pointers.

### 12.5 Addon Compatibility

- All existing addon `.rmp` files must load correctly.
- Key override semantics (last-writer-wins) must be preserved.
- Addon loading order must not change (determined by the caller's sequence of `LoadResourceIndex` calls).

### 12.6 Key Case Sensitivity

Resource keys must be **case-sensitive**, matching the C `CharHashTable` behavior. The key `"comm.arilou.graphics"` is distinct from `"COMM.ARILOU.GRAPHICS"`. This is a known gap in the existing Rust code which must be corrected.

---

## 13. Requirements (EARS Format)

### Index and Lifecycle

**REQ-RES-001**: The resource system shall maintain a single global resource index as a string-keyed `HashMap<String, ResourceDesc>`.

**REQ-RES-002**: When `InitResourceSystem()` is called, the system shall allocate a new resource index and store it as the current global index.

**REQ-RES-003**: When `InitResourceSystem()` is called and an index already exists, the system shall return the existing index without allocating a new one.

**REQ-RES-004**: When `InitResourceSystem()` is called, the system shall register exactly 14 resource types in this order: UNKNOWNRES, STRING, INT32, BOOLEAN, COLOR, GFXRES, FONTRES, STRTAB, BINTAB, CONVERSATION, SNDRES, MUSICRES, 3DOVID, SHIP.

**REQ-RES-005**: When `LoadResourceIndex(dir, filename, prefix)` is called, the system shall open the file via `res_OpenResFile`, read its entire contents, and parse it as a property file.

**REQ-RES-006**: When a property file is parsed, the system shall treat lines beginning with `#` (after optional whitespace) as comments and skip them entirely.

**REQ-RES-007**: When a property file is parsed, the system shall treat blank lines (only whitespace) as no-ops and skip them.

**REQ-RES-008**: When a property file line contains a key but no `=` separator, the system shall log a warning "Key without value" and skip to the next line.

**REQ-RES-009**: When a property file has a bare key at EOF without a value, the system shall log a warning "Bare keyword at EOF" and stop parsing.

**REQ-RES-010**: When a property file line has `key = value`, the system shall trim whitespace from both the key (trailing) and value (leading and trailing).

**REQ-RES-011**: When a property file line has an inline `#` character in the value portion, the system shall treat everything from `#` onward as a comment and exclude it from the value.

**REQ-RES-012**: When a prefix is provided to `LoadResourceIndex`, the system shall prepend the prefix to every key before processing. The prefix+key string shall be limited to 255 characters.

**REQ-RES-013**: When a property file cannot be opened, `LoadResourceIndex` shall silently return without error.

**REQ-RES-088**: When `UninitResourceSystem()` is called, the system shall drop the HashMap and all Rust-owned allocations, call `freeFun` for any loaded heap resources, then set the global index to None.

**REQ-RES-089**: The system shall support calling `LoadResourceIndex` multiple times to accumulate entries from multiple files into the same index.

### Type Registration

**REQ-RES-014**: When `InstallResTypeVectors(resType, loadFun, freeFun, stringFun)` is called, the system shall create a `ResourceHandlers` struct and store it in the HashMap under key `"sys.<resType>"`.

**REQ-RES-015**: The system shall store type registrations in the same HashMap as resource entries, distinguished by the `"sys."` prefix on their keys.

**REQ-RES-016**: The `ResourceHandlers` struct shall contain four fields: `resType` (C string pointer), `loadFun`, `freeFun`, and `toString` (C function pointers, any of which may be NULL).

**REQ-RES-017**: When `InstallResTypeVectors` cannot allocate the handlers struct, it shall return FALSE (0).

### Resource Lookup and Creation

**REQ-RES-018**: When `process_resource_desc(key, value)` is called, the system shall parse the value by splitting on the first `:` character to extract the type name and file path.

**REQ-RES-019**: When the value contains no `:` character, the system shall log a warning, treat the type as `"UNKNOWNRES"`, and use the entire value as the path.

**REQ-RES-020**: When the type extracted from the value is not registered, the system shall log a warning "Illegal type" and fall back to the `"UNKNOWNRES"` handler.

**REQ-RES-021**: When the resolved type's `loadFun` is NULL, the system shall log a warning and not create the resource entry.

**REQ-RES-022**: When a `ResourceDesc` is created, it shall have `refcount` initialized to 0.

**REQ-RES-023**: When a resource key already exists in the HashMap, the system shall remove the old entry (including calling `freeFun` if data is loaded) and insert the new one.

### Lazy Loading

**REQ-RES-024**: When a heap-type resource (`freeFun` is non-NULL) is created via `process_resource_desc`, the system shall set `resdata.ptr = NULL` (deferred loading).

**REQ-RES-025**: When a value-type resource (`freeFun` is NULL) is created via `process_resource_desc`, the system shall immediately call `loadFun(fname, &resdata)` to parse the value.

**REQ-RES-026**: When `res_GetResource(key)` is called with `key == NULL`, the system shall log a warning "Trying to get null resource" and return NULL.

**REQ-RES-027**: When `res_GetResource(key)` is called and the key is not found in the index, the system shall log a warning "Trying to get undefined resource" and return NULL.

**REQ-RES-028**: When `res_GetResource(key)` is called and `resdata.ptr` is NULL, the system shall call `loadFun(fname, &resdata)` to trigger the type's loader.

**REQ-RES-029**: When `res_GetResource(key)` is called and the resource is successfully loaded (or already loaded), the system shall increment `refcount` by 1 and return the data pointer.

**REQ-RES-030**: When `res_GetResource(key)` is called and the load fails (ptr remains NULL after loadFun), the system shall return NULL without incrementing refcount.

**REQ-RES-031**: When `LoadResourceFromPath(path, loadFun)` is called, it shall open the file via `res_OpenResFile(contentDir, path, "rb")`, get the file length, set the global `_cur_resfile_name` to path during loading, call the load function, then clear `_cur_resfile_name` to NULL.

**REQ-RES-032**: When `LoadResourceFromPath` cannot open the file, it shall log a warning and return NULL.

**REQ-RES-033**: When `LoadResourceFromPath` opens a zero-length file, it shall log a warning and return NULL.

### Reference Counting

**REQ-RES-034**: The system shall maintain a per-resource `refcount` field that is incremented on each successful `res_GetResource()` call.

**REQ-RES-035**: When `res_FreeResource(key)` is called with `refcount > 0`, the system shall decrement `refcount` by 1.

**REQ-RES-036**: When `res_FreeResource(key)` is called with `refcount == 0`, the system shall log a warning "freeing an unreferenced resource."

**REQ-RES-037**: When `res_FreeResource(key)` is called and `refcount` reaches 0, the system shall call `freeFun(resdata.ptr)` and set `resdata.ptr = NULL`.

**REQ-RES-038**: When `res_FreeResource(key)` is called on a non-heap resource (`freeFun` is NULL), the system shall log a warning "trying to free a non-heap resource."

**REQ-RES-039**: When `res_FreeResource(key)` is called on a resource that is not loaded (`resdata.ptr` is NULL), the system shall log a warning "trying to free not loaded resource."

**REQ-RES-040**: When `res_FreeResource(key)` is called with an unrecognized key, the system shall log a warning.

### Detach

**REQ-RES-041**: When `res_DetachResource(key)` is called successfully, the system shall return the data pointer, set `resdata.ptr = NULL`, and set `refcount = 0`.

**REQ-RES-042**: When `res_DetachResource(key)` is called on an unrecognized key, it shall log a warning and return NULL.

**REQ-RES-043**: When `res_DetachResource(key)` is called on a non-heap resource, it shall log a warning and return NULL.

**REQ-RES-044**: When `res_DetachResource(key)` is called on a resource that is not loaded, it shall log a warning and return NULL.

**REQ-RES-045**: When `res_DetachResource(key)` is called on a resource with `refcount > 1`, it shall log a warning "trying to detach a resource referenced N times" and return NULL.

**REQ-RES-046**: When a resource is detached and `res_GetResource` is called again for the same key, the system shall perform a fresh load (because `resdata.ptr` is NULL).

### Config Get/Set

**REQ-RES-047**: When `res_GetString(key)` is called for a non-existent or non-STRING key, it shall return a pointer to an empty string `""`.

**REQ-RES-048**: When `res_GetInteger(key)` is called for a non-existent or non-INT32 key, it shall return 0.

**REQ-RES-049**: When `res_GetBoolean(key)` is called for a non-existent or non-BOOLEAN key, it shall return FALSE (0).

**REQ-RES-050**: When `res_GetColor(key)` is called for a non-existent or non-COLOR key, it shall return `Color { r: 0, g: 0, b: 0, a: 0 }`.

**REQ-RES-051**: When `res_PutString(key, value)` is called for a non-existent key or a key that is not STRING type, the system shall first create the key as `"STRING:undefined"`, then update it with the new value.

**REQ-RES-052**: When `res_PutString(key, value)` is called, the system shall update the descriptor's `fname` and `resdata.str` to the new value string.

**REQ-RES-053**: *(Removed — C-specific in-place buffer optimization detail not applicable to Rust's owned String model.)*

**REQ-RES-054**: When `res_PutInteger(key, value)` is called for a non-existent key or non-INT32 key, the system shall first create the key as `"INT32:0"`, then set `resdata.num`.

**REQ-RES-055**: When `res_PutBoolean(key, value)` is called for a non-existent key or non-BOOLEAN key, the system shall first create the key as `"BOOLEAN:false"`, then set `resdata.num`.

**REQ-RES-056**: When `res_PutColor(key, value)` is called for a non-existent key or non-COLOR key, the system shall first create the key as `"COLOR:rgb(0, 0, 0)"`, then set `resdata.num` to the packed RGBA value.

**REQ-RES-057**: The `STRING` type shall store its value such that `resdata.str` references the same storage as `fname`. In Rust, the descriptor owns a single `String` and both `fname` and `resdata.str` point to its C-string representation.

**REQ-RES-058**: The `INT32` type shall parse integer values using C `atoi()` semantics and store the result in `resdata.num`.

**REQ-RES-059**: The `BOOLEAN` type shall recognize `"true"` (case-insensitive) as TRUE (1) and all other values as FALSE (0).

### Config Persistence

**REQ-RES-060**: When `SaveResourceIndex(dir, file, root, strip_root)` is called, the system shall iterate all entries in the HashMap.

**REQ-RES-061**: When saving, the system shall only write entries whose key starts with the `root` prefix (if non-NULL) and whose type handler has a non-NULL `toString` function.

**REQ-RES-062**: When saving with `strip_root = TRUE`, the system shall remove the `root` prefix from the key in the output file.

**REQ-RES-063**: When saving, each entry shall be written as `key = TYPE:serialized_value\n` where `serialized_value` is produced by calling `toString(&resdata, buf, 256)`.

**REQ-RES-064**: When saving, entries with no type handler or no `toString` function shall be skipped with a warning logged for missing handler cases.

**REQ-RES-065**: When the output file cannot be opened for writing, `SaveResourceIndex` shall silently return.

### Color Parsing

**REQ-RES-066**: When parsing a color descriptor, the system shall support `rgb(r, g, b)` format with 8-bit integer components and implicit alpha of 0xFF.

**REQ-RES-067**: When parsing a color descriptor, the system shall support `rgba(r, g, b, a)` format with 8-bit integer components.

**REQ-RES-068**: When parsing a color descriptor, the system shall support `rgb15(r, g, b)` format with 5-bit integer components (0-31), converting each to 8-bit via `(x << 3) | (x >> 2)`, and implicit alpha of 0xFF.

**REQ-RES-069**: When parsing color components, the system shall accept decimal, hexadecimal (`0x` prefix), and octal (`0` prefix) integer formats.

**REQ-RES-070**: When a color component value is below 0, the system shall clamp it to 0 and log a warning.

**REQ-RES-071**: When a color component value exceeds the maximum for its bit depth, the system shall clamp it to the maximum and log a warning.

**REQ-RES-072**: If the color descriptor cannot be parsed as any recognized format, the system shall log an error and set the packed value to `0x00000000`.

**REQ-RES-073**: When serializing a color with alpha == 0xFF, the system shall output `rgb(0x%02x, 0x%02x, 0x%02x)` format.

**REQ-RES-074**: When serializing a color with alpha != 0xFF, the system shall output `rgba(0x%02x, 0x%02x, 0x%02x, 0x%02x)` format.

### Path Resolution

**REQ-RES-075**: When `res_OpenResFile(dir, filename, mode)` is called and the path is a directory, it shall return the sentinel value (pointer with all bits set).

**REQ-RES-076**: When `res_CloseResFile` is called with the sentinel value, it shall no-op and return TRUE.

**REQ-RES-077**: When `LengthResFile` is called with the sentinel value, it shall return 1.

**REQ-RES-078**: All heap-type resource loading shall resolve file paths relative to `contentDir` via `res_OpenResFile(contentDir, path, "rb")`.

**REQ-RES-079**: Config files shall be loaded from `configDir` via `LoadResourceIndex(configDir, filename, prefix)`.

**REQ-RES-080**: When loading addons, the system shall support the caller's pattern of opening `contentDir/addons/<addon_name>/`, scanning for `.rmp` files, and calling `LoadResourceIndex` for each.

**REQ-RES-081**: When an addon defines the same resource key as a previously loaded index, the new entry shall replace the old one (enabling content overrides).

### Error Handling

**REQ-RES-082**: When any public API function is called and no index exists, the system shall call `InitResourceSystem()` to auto-initialize.

**REQ-RES-083**: When `res_Remove(key)` is called on a resource with `refcount > 0`, the system shall log a warning "Replacing while live" but proceed with removal.

**REQ-RES-084**: When `res_Remove(key)` is called and the resource has loaded data with a `freeFun`, the system shall call `freeFun` before dropping the descriptor.

**REQ-RES-085**: When `res_Remove(key)` is called, the system shall drop the Rust-owned descriptor (String, struct) when the entry is removed from the HashMap.

### Resource Type: CONVERSATION (Special)

**REQ-RES-090**: The `CONVERSATION` type shall accept a path string containing up to three colon-separated components: text file path, speech clip directory path, and timestamp file path. Parsing is done by the C-registered `loadFun`, not by the Rust resource system.

**REQ-RES-091**: The `CONVERSATION` type's `loadFun` shall be registered as a `ResourceLoadFun` (taking `const char *path, RESOURCE_DATA *resdata`) and shall be called directly by the Rust resource system during lazy loading.

### Resource Type: 3DOVID (Special)

**REQ-RES-092**: The `3DOVID` type shall accept a path string containing up to four colon-separated components. Parsing is done by the C-registered `loadFun`.

**REQ-RES-093**: The `3DOVID` type's `loadFun` parses the path components internally. The Rust resource system passes the entire path string (everything after the first `TYPE:` colon) to the loader.

### Resource Type: SHIP (Special)

**REQ-RES-095**: The `SHIP` type shall accept a path string that is an integer. Parsing is done by the C-registered `loadFun`.

### Convenience Wrappers

**REQ-RES-098**: The `Load*Instance` functions (implemented in C subsystem code) shall continue to call `res_GetResource` followed by `res_DetachResource`. These functions are not part of the Rust resource system but depend on it.

**REQ-RES-099**: The `nameref.h` macros remain in C and call the Rust-backed `res_GetResource` / `res_DetachResource` transparently.

### Additional Value Access

**REQ-RES-101**: `res_GetIntResource(key)` shall look up the key and return `resdata.num` directly (no type checking, no refcount increment). Returns 0 if key not found.

**REQ-RES-102**: `res_GetBooleanResource(key)` shall return `res_GetIntResource(key) != 0`.

**REQ-RES-103**: `res_GetResourceType(key)` shall return `desc->vtable->resType` for the given key, or NULL if the key is null or undefined.

### Binary Resource Data

**REQ-RES-108**: `GetResourceData` shall read a 4-byte `DWORD` length prefix from the stream. If the prefix equals `~0u32` (`0xFFFFFFFF`), it shall treat the remaining data as uncompressed and allocate + read `length - 4` bytes.

**REQ-RES-109**: If the `DWORD` prefix is any value other than `0xFFFFFFFF`, `GetResourceData` shall log a warning about unsupported LZ compression and return NULL.

**REQ-RES-110**: `FreeResourceData` shall free the data and return TRUE.

### File I/O Layer

**REQ-RES-113**: The file I/O layer shall wrap all UIO operations in named `extern "C"` functions matching the C API signatures.

**REQ-RES-114**: `PutResFileNewline` shall write `\r\n` on Windows and `\n` on all other platforms.

**REQ-RES-115**: `res_CloseResFile` shall return TRUE on success (including NULL and sentinel inputs) and FALSE if `uio_fclose` fails.

### Rust-Specific Requirements

**REQ-RES-R001**: All `extern "C"` functions shall catch any potential Rust panic before it crosses the FFI boundary. No Rust panic shall propagate to C callers.

**REQ-RES-R002**: All `extern "C"` functions shall validate pointer parameters for NULL before dereferencing. NULL `*const c_char` parameters shall be treated as empty strings or trigger appropriate error returns.

**REQ-RES-R003**: The global resource index shall be protected by interior mutability (e.g., `Mutex<Option<ResourceState>>` or `RwLock<Option<ResourceState>>`). Access from `extern "C"` functions shall acquire the lock, perform the operation, and release the lock.

**REQ-RES-R004**: If the interior mutability lock is poisoned (due to a prior panic), `extern "C"` functions shall return safe defaults (NULL/0/FALSE) and log an error, rather than propagating the panic.

**REQ-RES-R005**: The Rust resource system shall not cause use-after-free through its safe API. Specifically:
- `resdata.ptr` shall only be set to non-NULL by a successful `loadFun` call and cleared to NULL by `freeFun`, `res_DetachResource`, or `res_Remove`.
- The `fname` String owned by the descriptor shall not be dropped while any external reference to `resdata.str` may exist for STRING types. (This matches C's aliasing of `resdata.str` to `fname`.)

**REQ-RES-R006**: Errors within Rust shall be represented as `Result<T, E>`. At the FFI boundary, `Err` variants shall be converted to the C-compatible error return (NULL/0/FALSE) and logged.

**REQ-RES-R007**: Resource keys stored in the HashMap shall preserve their original case exactly as provided by callers (case-sensitive). The Rust system shall NOT lowercase or uppercase keys.

**REQ-RES-R008**: C function pointers stored in `ResourceHandlers` (`loadFun`, `freeFun`, `toString`) shall be called within `unsafe` blocks. The Rust system shall validate that function pointers are non-NULL (via `Option`) before calling them.

**REQ-RES-R009**: The `ResourceData` union shall be `#[repr(C)]` to ensure layout compatibility with the C `RESOURCE_DATA` union.

**REQ-RES-R010**: The `ResourceHandlers` struct shall be `#[repr(C)]` to ensure layout compatibility with the C `resource_handlers` struct.

**REQ-RES-R011**: The `Color` struct returned by `res_GetColor` shall be `#[repr(C)]` with fields `r, g, b, a` in that order, matching the C `Color` struct layout.

**REQ-RES-R012**: When `res_GetString` returns a pointer into Rust-owned memory, that pointer shall remain valid until the entry is modified (via `res_PutString`) or removed (via `res_Remove` or `UninitResourceSystem`). The system shall maintain a stable C-string representation (e.g., `CString`) for each STRING descriptor.

**REQ-RES-R013**: All logging from the Rust resource system shall use the existing project log bridge (`rust_bridge_log_msg` or equivalent) to route messages to the C `log_add` infrastructure.

**REQ-RES-R014**: The Rust resource system shall be compiled as part of the existing Rust static library (`libuqm_rust.a`) and linked into the C build when enabled via a build flag (e.g., `USE_RUST_RESOURCE`).

**REQ-RES-R015**: When `SaveResourceIndex` iterates the HashMap, the output order of entries may differ from the C implementation (HashMap iteration is unordered vs. C hash table iteration). Each individual line in the output file shall be formatted identically to the C output. The system shall produce a valid `.cfg`/`.key` file regardless of entry order.

---

## Appendix A: C Type Definitions for FFI

For reference, these are the C type mappings required at the FFI boundary:

| C Type | Size | Rust FFI Type |
|--------|------|---------------|
| `RESOURCE` (`const char *`) | pointer | `*const c_char` |
| `RESOURCE_INDEX` (`RESOURCE_INDEX_DESC *`) | pointer | `*mut ResourceIndexDesc` |
| `BOOLEAN` | 4 bytes (int) | `c_int` |
| `DWORD` | 4 bytes (uint32) | `u32` |
| `COUNT` (`WORD`) | 2 bytes (uint16) | `u16` |
| `Color` | 4 bytes (struct) | `#[repr(C)] Color { r: u8, g: u8, b: u8, a: u8 }` |
| `uio_DirHandle *` | pointer (opaque) | `*mut c_void` (or opaque newtype) |
| `uio_Stream *` | pointer (opaque) | `*mut c_void` (or opaque newtype) |
| `ResourceLoadFun` | fn ptr | `unsafe extern "C" fn(*const c_char, *mut ResourceData)` |
| `ResourceFreeFun` | fn ptr | `unsafe extern "C" fn(*mut c_void) -> c_int` |
| `ResourceStringFun` | fn ptr | `unsafe extern "C" fn(*mut ResourceData, *mut c_char, c_uint)` |
| `ResourceLoadFileFun` | fn ptr | `unsafe extern "C" fn(*mut c_void, u32) -> *mut c_void` |
| `NULL_RESOURCE` | `NULL` | `std::ptr::null()` |

## Appendix B: Complete Function Inventory

All `extern "C"` functions the Rust resource system must expose:

| Function | Category | Returns |
|----------|----------|---------|
| `InitResourceSystem` | Lifecycle | `*mut ResourceIndexDesc` |
| `UninitResourceSystem` | Lifecycle | `void` |
| `LoadResourceIndex` | Index | `void` |
| `SaveResourceIndex` | Index | `void` |
| `InstallResTypeVectors` | Type Registration | `BOOLEAN` |
| `res_GetResource` | Resource Access | `*mut c_void` |
| `res_DetachResource` | Resource Access | `*mut c_void` |
| `res_FreeResource` | Resource Access | `void` |
| `res_Remove` | Resource Access | `BOOLEAN` |
| `res_GetIntResource` | Value Access | `DWORD` |
| `res_GetBooleanResource` | Value Access | `BOOLEAN` |
| `res_GetResourceType` | Value Access | `*const c_char` |
| `CountResourceTypes` | Value Access | `COUNT` |
| `res_HasKey` | Config Get | `BOOLEAN` |
| `res_IsString` | Config Get | `BOOLEAN` |
| `res_IsInteger` | Config Get | `BOOLEAN` |
| `res_IsBoolean` | Config Get | `BOOLEAN` |
| `res_IsColor` | Config Get | `BOOLEAN` |
| `res_GetString` | Config Get | `*const c_char` |
| `res_GetInteger` | Config Get | `int` |
| `res_GetBoolean` | Config Get | `BOOLEAN` |
| `res_GetColor` | Config Get | `Color` |
| `res_PutString` | Config Put | `void` |
| `res_PutInteger` | Config Put | `void` |
| `res_PutBoolean` | Config Put | `void` |
| `res_PutColor` | Config Put | `void` |
| `res_OpenResFile` | File I/O | `*mut uio_Stream` |
| `res_CloseResFile` | File I/O | `BOOLEAN` |
| `LoadResourceFromPath` | File I/O | `*mut c_void` |
| `ReadResFile` | File I/O | `size_t` |
| `WriteResFile` | File I/O | `size_t` |
| `GetResFileChar` | File I/O | `int` |
| `PutResFileChar` | File I/O | `int` |
| `PutResFileNewline` | File I/O | `int` |
| `SeekResFile` | File I/O | `long` |
| `TellResFile` | File I/O | `long` |
| `LengthResFile` | File I/O | `size_t` |
| `DeleteResFile` | File I/O | `BOOLEAN` |
| `GetResourceData` | File I/O | `*mut c_void` |
| `FreeResourceData` | File I/O | `BOOLEAN` |

**Total: 38 extern "C" functions.**

## Appendix C: Existing Rust Code Assessment

The existing Rust code in `rust/src/resource/` provides partial coverage. This table summarizes what exists vs. what this spec requires:

| Spec Requirement | Existing Rust Module | Status | Gap |
|-----------------|---------------------|--------|-----|
| HashMap index | `index.rs` (`ResourceIndex`) | Partial | Wrong: lowercases keys. Must be case-sensitive. |
| .rmp parsing | `propfile.rs` (`PropertyFile`) | Partial | Wrong: uses `BufReader::lines()` + `split_once('=')`. Uppercases keys. Missing: `TYPE:path` split, inline `#` comments, prefix support, bare-key-at-EOF handling. |
| Type handler registry | `resource_type.rs` (`ResourceType`) | Partial | Has enum for value types. Missing: C function pointer storage for heap types. Missing: `GFXRES`, `FONTRES`, `STRTAB`, `BINTAB`, `CONVERSATION`, `SNDRES`, `MUSICRES`, `3DOVID`, `SHIP`. |
| Lazy loading / refcount | `resource_system.rs` | Partial | Has `ResourceDescriptor` with `ref_count`. Missing: `res_DetachResource`. Missing: C `loadFun`/`freeFun` dispatch. |
| Config get/set | `resource_system.rs` | Partial | Has typed getters. Missing: all `Put` functions. Missing: `SaveResourceIndex`. |
| Color parsing | `resource_type.rs` (`ColorResource`) | Wrong | Parses `#RRGGBB`/`#RRGGBBAA`. Must parse `rgb()`, `rgba()`, `rgb15()`. |
| LRU cache | `cache.rs` (`ResourceCache`) | Extra | C system has no LRU cache. This is an enhancement, not a replacement component. May be useful but is not part of the C-compatible API. |
| String bank | `stringbank.rs` (`StringBank`) | Divergent | Implements localized string tables with language fallback. C `stringbank.c` is an arena allocator. Different purpose. |
| FFI bridge | `ffi.rs` | Partial | Has 15+ functions but uses different function names (`rust_resource_*`, `rust_cache_*`). Must expose C-compatible names. |
| File I/O wrappers | None | Missing | No UIO wrappers exist in Rust. |
| Resource loader | `loader.rs` (`ResourceLoader`) | Wrong | Uses `std::fs` directly, bypassing UIO. |

## Appendix D: Index Loading Sequence

At startup, the C code calls these functions in this order. The Rust system must produce identical index state after each step:

```
1.  InitResourceSystem()
    → Empty index with 14 type handlers registered

2.  LoadResourceIndex(configDir, "uqm.cfg", "config.")
    → Config entries: config.sfxvol, config.fullscreen, etc.

3.  loadIndices(contentDir)  [loads all .rmp files found]
    → Game resources: comm.*, ship.*, music.*, etc. (963+ entries from uqm.rmp)

4.  LoadResourceIndex(contentDir, "menu.key", "menu.")
    → Menu bindings: menu.up.1, menu.down.1, etc.

5.  LoadResourceIndex(configDir, "override.cfg", "menu.")
    → Menu overrides (if file exists)

6.  LoadResourceIndex(configDir, "flight.cfg", "keys.")
    → Flight key bindings

7.  LoadResourceIndex(contentDir, "uqm.key", "keys.")
    → Default key bindings

8.  [For each addon]: loadAddon(name)
    → LoadResourceIndex for each .rmp in addon dir
    → Addon entries override base entries with same keys
```

## Appendix E: Key Naming Conventions

Resource keys follow hierarchical dot-separated naming:

```
comm.<race>.graphics      — Communication screen animation (GFXRES)
comm.<race>.music         — Communication music (MUSICRES)
comm.<race>.font          — Communication font (FONTRES)
comm.<race>.dialogue      — Conversation data (CONVERSATION)
comm.<race>.colortable    — Color table (BINTAB)
ship.<race>.code          — Ship code resource (SHIP)
ship.<race>.sounds        — Ship sound effects (SNDRES)
ship.<race>.icons         — Ship icon graphics (GFXRES)
music.<name>              — Background music (MUSICRES)
slides.<name>             — Video/slideshow resources (3DOVID)
colortable.<name>         — Global color tables (BINTAB)
config.<name>             — Configuration values (STRING/INT32/BOOLEAN/COLOR)
keys.<n>.<action>         — Key binding definitions (STRING)
menu.<action>             — Menu key bindings (STRING)
```
