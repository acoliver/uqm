# UQM C Resource Loading System — Comprehensive Architectural Analysis

> Ground truth reference for reimplementation. All details extracted from C source code.

---

## 1. System Overview

### Purpose

The UQM resource system is a **string-keyed, type-dispatched, lazy-loading asset manager**. It provides:

1. **Index loading** — Parsing `.rmp` (resource map) and `.cfg`/`.key` property files into a flat key→descriptor hash map.
2. **Type-dispatched loading** — Each resource type (graphics, sound, fonts, etc.) registers load/free/toString handler functions via a vtable.
3. **Lazy loading** — Resource data is not loaded from disk until first access via `res_GetResource()`.
4. **Key-value configuration** — The same system doubles as a config store for STRING, INT32, BOOLEAN, and COLOR values that are read/written at runtime.
5. **Addon overlaying** — Multiple `.rmp` files can be loaded; later entries silently replace earlier ones, enabling addon content to override base resources.

### Lifecycle

```
InitResourceSystem()          — Allocate index, register built-in types
  └─ InstallResTypeVectors()  — For each type (STRING, INT32, BOOLEAN, COLOR,
                                 GFXRES, FONTRES, STRTAB, BINTAB, CONVERSATION,
                                 SNDRES, MUSICRES, 3DOVID, SHIP, UNKNOWNRES)

LoadResourceIndex(dir, file, prefix)  — Parse .rmp/.cfg/.key into index (repeatable)

res_GetResource(key)          — Lazy-load + refcount++
res_DetachResource(key)       — Transfer ownership to caller (ptr=NULL, refcount=0)
res_FreeResource(key)         — refcount--; if 0, free via vtable->freeFun

res_Get{String,Integer,Boolean,Color}(key)  — Direct value access (no refcount)
res_Put{String,Integer,Boolean,Color}(key)  — Modify or create entries

SaveResourceIndex(dir, file, root, strip_root)  — Serialize subset to file

UninitResourceSystem()        — Free hash table and index
```

### Threading Model

The resource system is **single-threaded**. There are no locks, mutexes, or atomic operations anywhere in the resource code. The global `curResourceIndex` pointer is a bare static variable. All access is assumed to occur from the main thread. The `_cur_resfile_name` global in `getres.c` is set during file loading and reset to NULL afterward — it would be unsafe under concurrent access.

---

## 2. Architecture

### Component Relationships

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          Callers (uqm/*.c)                              │
│   LoadGraphic, LoadFont, LoadSound, LoadMusic, LoadStringTable,        │
│   LoadCodeRes, LoadLegacyVideoInstance                                 │
│   res_Get{String,Integer,Boolean,Color}, res_Put*, res_HasKey          │
└────────────────────────────────┬────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                       Public API (reslib.h)                             │
│   InitResourceSystem, UninitResourceSystem, LoadResourceIndex,         │
│   SaveResourceIndex, InstallResTypeVectors,                            │
│   res_GetResource, res_DetachResource, res_FreeResource,               │
│   res_Get*/res_Put*, LoadResourceFromPath                              │
└────────────────────┬───────────────────────────┬───────────────────────┘
                     │                           │
          ┌──────────▼──────────┐     ┌──────────▼──────────┐
          │   resinit.c         │     │   getres.c          │
          │ (init, types,       │     │ (lookup, load,      │
          │  config get/put,    │     │  refcount, detach,  │
          │  save/load index)   │     │  free)              │
          └──────────┬──────────┘     └──────────┬──────────┘
                     │                           │
          ┌──────────▼──────────┐     ┌──────────▼──────────┐
          │   propfile.c        │     │   loadres.c         │
          │ (parse .rmp/.cfg)   │     │ (GetResourceData    │
          │                     │     │  for binary .ct/.xlt│
          └─────────────────────┘     └─────────────────────┘
                     │
          ┌──────────▼──────────┐
          │   filecntl.c        │
          │ (UIO file ops:      │
          │  Open, Read, Write, │
          │  Seek, Length, etc.) │
          └─────────────────────┘
                     │
          ┌──────────▼──────────┐
          │   UIO (libs/uio/)   │
          │ (virtual filesystem │
          │  with mount points, │
          │  zip support)       │
          └─────────────────────┘

  Registered Type Handlers:
  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
  │ resgfx.c     │ │ resinst.c    │ │ sresins.c    │ │ vresins.c    │
  │ GFXRES       │ │ SNDRES       │ │ STRTAB       │ │ 3DOVID       │
  │ FONTRES      │ │ MUSICRES     │ │ BINTAB       │ │              │
  │              │ │              │ │ CONVERSATION  │ │              │
  └──────────────┘ └──────────────┘ └──────────────┘ └──────────────┘
  ┌──────────────┐
  │ dummy.c      │
  │ SHIP         │
  └──────────────┘
```

### Data Flow: .rmp File → Loaded Resource

```
1. LoadResourceIndex(dir, "uqm.rmp", NULL)
   └─ PropFile_from_filename(dir, "uqm.rmp", process_resource_desc, NULL)
      └─ PropFile_from_file(stream, handler, prefix)
         └─ PropFile_from_string(data, handler, prefix)
            └─ For each "key = TYPE:path" line:
               handler(prefix+key, "TYPE:path")

2. process_resource_desc("comm.arilou.graphics", "GFXRES:base/comm/arilou/arilou.ani")
   └─ newResourceDesc(key, resval)
      ├─ Parse type prefix "GFXRES" → lookup "sys.GFXRES" in hash map
      ├─ Get ResourceHandlers vtable from the type descriptor
      ├─ Allocate ResourceDesc { fname="base/comm/arilou/arilou.ani", vtable=&gfx_handlers }
      ├─ If vtable->freeFun == NULL (value type): call loadFun immediately
      │  Else (heap type): resdata.ptr = NULL (lazy)
      └─ Insert into CharHashTable under key "comm.arilou.graphics"

3. res_GetResource("comm.arilou.graphics")       [later, at runtime]
   └─ lookupResourceDesc(idx, key)                → ResourceDesc*
      └─ If desc->resdata.ptr == NULL:
         loadResourceDesc(desc)                    → calls vtable->loadFun
           └─ GetCelFileData(fname, &resdata)
              └─ LoadResourceFromPath(fname, _GetCelData)
                 ├─ res_OpenResFile(contentDir, fname, "rb")
                 ├─ LengthResFile(stream)
                 ├─ _GetCelData(stream, length)   → DRAWABLE
                 └─ res_CloseResFile(stream)
      └─ ++desc->refcount
      └─ return desc->resdata.ptr
```

### Key Data Structures

#### `RESOURCE_INDEX_DESC` (index.h)

```c
struct resource_index_desc {
    CharHashTable_HashTable *map;  // String-keyed hash table
    size_t numRes;                 // (unused field — never written)
};
```

Typedef: `RESOURCE_INDEX = RESOURCE_INDEX_DESC *`

#### `ResourceDesc` (index.h)

```c
struct resource_desc {
    RESOURCE res_id;           // (unused field — never written in current code)
    char *fname;               // File path or raw value string (heap-allocated)
    ResourceHandlers *vtable;  // Pointer to type handler functions
    RESOURCE_DATA resdata;     // Union: loaded data (ptr/num/str)
    unsigned refcount;         // Reference count (rudimentary)
};
```

#### `RESOURCE_DATA` (reslib.h)

```c
typedef union {
    DWORD num;        // For INT32, BOOLEAN, COLOR
    void *ptr;        // For heap-loaded resources (DRAWABLE, FONT, etc.)
    const char *str;  // For STRING type (aliases fname)
} RESOURCE_DATA;
```

#### `ResourceHandlers` (index.h)

```c
struct resource_handlers {
    const char *resType;           // Type name string (e.g., "GFXRES")
    ResourceLoadFun *loadFun;      // void (const char *pathname, RESOURCE_DATA *resdata)
    ResourceFreeFun *freeFun;      // BOOLEAN (void *handle) — NULL for value types
    ResourceStringFun *toString;   // void (RESOURCE_DATA *handle, char *buf, unsigned int size) — for serialization
};
```

#### Function Pointer Typedefs (reslib.h)

```c
typedef void    (ResourceLoadFun)    (const char *pathname, RESOURCE_DATA *resdata);
typedef BOOLEAN (ResourceFreeFun)    (void *handle);
typedef void    (ResourceStringFun)  (RESOURCE_DATA *handle, char *buf, unsigned int size);
typedef void *  (ResourceLoadFileFun)(uio_Stream *fp, DWORD len);
```

#### `CharHashTable_HashTable`

A generic open-addressing hash table from `libs/uio/` parameterized with `char *` keys and `void *` values. Key operations used:

- `CharHashTable_newHashTable(...)` — Create with load factor 0.85/0.9
- `CharHashTable_add(map, key, value)` — Returns 0 if key already exists
- `CharHashTable_find(map, key)` — Returns `void *` value or NULL
- `CharHashTable_remove(map, key)` — Returns nonzero on success
- `CharHashTable_getIterator(map)` — For serialization iteration
- `CharHashTable_iteratorDone/Next/Key/Value` — Iterator protocol
- `CharHashTable_deleteHashTable(map)` — Free table (leaks values — noted as TODO)
- `CharHashTable_freeIterator(it)` — Free iterator

#### Type Registration Storage

Type handlers are stored as regular entries in the *same* hash table with the key prefix `"sys."`. For example, registering type `"GFXRES"` creates an entry with key `"sys.GFXRES"` whose `resdata.ptr` points to a heap-allocated `ResourceHandlers` struct.

#### `RESOURCE` Type

```c
typedef const char *RESOURCE;
#define NULL_RESOURCE NULL
```

Resources are identified purely by their string key.

---

## 3. .rmp / .cfg File Format

### Parser: `propfile.c`

The parser (`PropFile_from_string`) operates on a mutable in-memory copy of the entire file. It writes `\0` terminators in-place.

### Syntax

```
# Lines starting with # are comments
# Inline comments with # are supported (terminate value)

key = TYPE:path
key = TYPE:value

# Whitespace rules:
# - Leading whitespace on lines is skipped
# - Whitespace around = is trimmed
# - Trailing whitespace on values is trimmed
# - Blank lines are skipped
```

### Formal Grammar

```
file     := line*
line     := ws comment | ws keyvalue | ws
comment  := '#' [^\n]* '\n'
keyvalue := key ws '=' ws value (comment | '\n')
key      := [^ \t\n=#]+
value    := [^\n#]* (trailing whitespace trimmed)
```

### Exact Parsing Rules (from `PropFile_from_string`)

1. Skip leading whitespace (including newlines between entries).
2. If `#`, skip to end of line (comment).
3. Read characters until `=`, `\n`, `#`, or EOF.
   - If no `=` found: log warning "Key without value", skip line.
   - If bare key at EOF: log warning "Bare keyword at EOF", stop.
4. Key = characters from start to `=`, with trailing whitespace trimmed.
5. Skip `=`, then skip whitespace (but not past `#` or `\n`).
6. Value = characters until `#`, `\n`, or EOF, with trailing whitespace trimmed.
7. Both key and value are terminated by writing `\0` into the mutable buffer.
8. If prefix is non-NULL, the key is prepended: `snprintf(buf, 255, "%s%s", prefix, key)`.
9. Call `handler(final_key, value)`.

### Real Examples

**From `uqm.rmp` (963 lines, game content):**
```
comm.arilou.graphics = GFXRES:base/comm/arilou/arilou.ani
comm.arilou.music = MUSICRES:base/comm/arilou/arilou.mod
comm.arilou.font = FONTRES:base/fonts/arilou.fon
comm.arilou.dialogue = CONVERSATION:base/comm/arilou/arilou.txt
comm.arilou.colortable = BINTAB:base/comm/arilou/arilou.ct
ship.androsynth.code = SHIP:0
ship.androsynth.sounds = SNDRES:base/ships/androsynth/guardian.snd
colortable.main = BINTAB:base/uqm.ct
```

**From `3dovideo.rmp` (addon, 3DOVID with embedded colons in value):**
```
slides.spins.00 = 3DOVID:addons/3dovideo/ships/ship00.duk:addons/3dovideo/ships/spin.aif:addons/3dovideo/ships/ship00.aif:89
```

**From `3dovoice.rmp` (addon, CONVERSATION with multi-path value):**
```
comm.arilou.dialogue = CONVERSATION:addons/3dovoice/arilou/arilou.txt:addons/3dovoice/arilou/:addons/3dovoice/arilou/arilou.ts
```

**From `uqm.cfg` (config file, loaded with prefix "config."):**
```
alwaysgl = BOOLEAN:false
sfxvol = INT32:20
reswidth = INT32:320
scaler = STRING:no
audiodriver = STRING:mixsdl
fullscreen = BOOLEAN:false
```

**From `menu.key` (input config, loaded with prefix "menu."):**
```
# Arrow key controls...
up.1 = STRING:key Up
down.1 = STRING:key Down
select.1 = STRING:key Return
cancel.1 = STRING:key Space
```

### Value Format: `TYPE:path`

The value string is split on the **first** colon only during `newResourceDesc()`:

```c
path = strchr(resval, ':');
// type = resval[0..colon-1]
// path = resval[colon+1..]
```

Everything after the first colon is the "path" passed to the load function. For types like `3DOVID` and `CONVERSATION`, the path itself contains additional colons that are parsed by the type-specific load function.

### Type Prefix Construction

The type from the file (e.g., `"GFXRES"`) is prefixed with `"sys."` to form the lookup key `"sys.GFXRES"`. The prefix construction:

```c
#define TYPESIZ 32
strncpy(typestr, "sys.", TYPESIZ);
strncat(typestr+1, resval, n);  // Note: +1 is a subtle detail
typestr[n+4] = '\0';
```

If the type is not found in the hash table, the entry falls back to `"sys.UNKNOWNRES"` and uses the raw descriptor (no loading). If there's no colon at all, the entire value is treated as the path and the type defaults to `"sys.UNKNOWNRES"`.

---

## 4. Resource Type System

### Registration Mechanism: `InstallResTypeVectors`

```c
BOOLEAN InstallResTypeVectors(
    const char *resType,        // e.g., "GFXRES"
    ResourceLoadFun *loadFun,   // Called to load/parse the resource
    ResourceFreeFun *freeFun,   // Called to free loaded data (NULL = value type)
    ResourceStringFun *stringFun // Called for serialization (NULL = no save)
);
```

Creates a `ResourceHandlers` struct and a `ResourceDesc` wrapper, storing them in the hash table under key `"sys.<resType>"`. The `ResourceDesc` for a type handler has `vtable = NULL` (which is how it's distinguished from actual resource entries).

### Initialization Order (in `InitResourceSystem`)

```c
InstallResTypeVectors("UNKNOWNRES", UseDescriptorAsRes, NULL, NULL);
InstallResTypeVectors("STRING", UseDescriptorAsRes, NULL, RawDescriptor);
InstallResTypeVectors("INT32", DescriptorToInt, NULL, IntToString);
InstallResTypeVectors("BOOLEAN", DescriptorToBoolean, NULL, BooleanToString);
InstallResTypeVectors("COLOR", DescriptorToColor, NULL, ColorToString);
InstallGraphicResTypes();   // GFXRES, FONTRES
InstallStringTableResType(); // STRTAB, BINTAB, CONVERSATION
InstallAudioResTypes();      // SNDRES, MUSICRES
InstallVideoResType();       // 3DOVID
InstallCodeResType();        // SHIP
```

### Complete Type Registry

| Type | Load Function | Free Function | ToString Function | Source File | Heap? | Count in uqm.rmp |
|------|--------------|---------------|-------------------|-------------|-------|-------------------|
| `UNKNOWNRES` | `UseDescriptorAsRes` | `NULL` | `NULL` | `resinit.c` | No | 0 |
| `STRING` | `UseDescriptorAsRes` | `NULL` | `RawDescriptor` | `resinit.c` | No | 0* |
| `INT32` | `DescriptorToInt` | `NULL` | `IntToString` | `resinit.c` | No | 0* |
| `BOOLEAN` | `DescriptorToBoolean` | `NULL` | `BooleanToString` | `resinit.c` | No | 0* |
| `COLOR` | `DescriptorToColor` | `NULL` | `ColorToString` | `resinit.c` | No | 0* |
| `GFXRES` | `GetCelFileData` → `_GetCelData` | `_ReleaseCelData` | `NULL` | `resgfx.c`, `gfxload.c` | Yes | 582 |
| `FONTRES` | `GetFontFileData` → `_GetFontData` | `_ReleaseFontData` | `NULL` | `resgfx.c`, `gfxload.c` | Yes | 32 |
| `STRTAB` | `GetStringTableFileData` → `_GetStringData` | `FreeResourceData` | `NULL` | `sresins.c`, `getstr.c` | Yes | 51 |
| `BINTAB` | `GetBinaryTableFileData` → `_GetBinaryTableData` | `FreeResourceData` | `NULL` | `sresins.c`, `getstr.c` | Yes | 149 |
| `CONVERSATION` | `_GetConversationData` (direct) | `FreeResourceData` | `NULL` | `sresins.c`, `getstr.c` | Yes | 27 |
| `SNDRES` | `GetSoundBankFileData` → `_GetSoundBankData` | `_ReleaseSoundBankData` | `NULL` | `resinst.c`, `sfx.c` | Yes | 30 |
| `MUSICRES` | `GetMusicFileData` → `_GetMusicData` | `_ReleaseMusicData` | `NULL` | `resinst.c`, `music.c` | Yes | 64 |
| `3DOVID` | `GetLegacyVideoData` (direct parse) | `FreeLegacyVideoData` | `NULL` | `vresins.c` | Yes | 0* |
| `SHIP` | `GetCodeResData` (numeric→init func) | `_ReleaseCodeResData` | `NULL` | `dummy.c` | Yes | 28 |

\* STRING/INT32/BOOLEAN/COLOR appear in `.cfg` files, not `uqm.rmp`. 3DOVID appears in `3dovideo.rmp` addon.

### Value Types vs Heap Types

The key distinction is whether `freeFun` is NULL:

- **Value types** (`STRING`, `INT32`, `BOOLEAN`, `COLOR`, `UNKNOWNRES`): `freeFun == NULL`. The load function is called **immediately** during `newResourceDesc()` at index-load time. The data lives in `resdata.num` or `resdata.str` (which aliases `fname`). No lazy loading. No reference counting.

- **Heap types** (`GFXRES`, `FONTRES`, `STRTAB`, etc.): `freeFun != NULL`. `resdata.ptr` is set to NULL initially. The load function is called lazily on first `res_GetResource()`. Reference counting applies.

### Load Function Patterns

**Two-tier loading** (most heap types): The registered `loadFun` is a thin wrapper that calls `LoadResourceFromPath(pathname, fileLoadFun)`:

```c
static void GetCelFileData(const char *pathname, RESOURCE_DATA *resdata) {
    resdata->ptr = LoadResourceFromPath(pathname, _GetCelData);
}
```

`LoadResourceFromPath` opens the file via `res_OpenResFile(contentDir, path, "rb")`, gets its length, sets `_cur_resfile_name`, and calls the `ResourceLoadFileFun`:

```c
void *LoadResourceFromPath(const char *path, ResourceLoadFileFun *loadFun) {
    uio_Stream *stream = res_OpenResFile(contentDir, path, "rb");
    DWORD dataLen = LengthResFile(stream);
    _cur_resfile_name = path;
    void *resdata = (*loadFun)(stream, dataLen);
    _cur_resfile_name = NULL;
    res_CloseResFile(stream);
    return resdata;
}
```

**Direct loading** (`CONVERSATION`, `3DOVID`, `SHIP`): These types register a `ResourceLoadFun` that directly handles all parsing without `LoadResourceFromPath`. `CONVERSATION` opens files itself via `res_OpenResFile(contentDir, ...)` because it needs to parse the path for sub-components (text, clip dir, timestamps). `3DOVID` parses the path string for video/audio/speech/loop components without opening any files. `SHIP` converts the path to an integer and calls an init function.

---

## 5. Two-Phase Loading

### Phase 1: Index Loading

`LoadResourceIndex(dir, filename, prefix)` calls `PropFile_from_filename`, which reads the entire file into memory, parses key=value pairs, and for each pair calls `process_resource_desc(key, value)`.

`process_resource_desc` calls `newResourceDesc` which:

1. Parses `TYPE:path` from the value.
2. Looks up `sys.TYPE` in the hash table to find the `ResourceHandlers`.
3. Allocates a `ResourceDesc` with `fname` = path portion, `vtable` = handlers, `refcount` = 0.
4. **If** `vtable->freeFun == NULL` (value type): immediately calls `vtable->loadFun(fname, &resdata)` to parse the value.
5. **Else** (heap type): sets `resdata.ptr = NULL` — deferred until actual use.
6. Inserts into hash table. If key already exists, removes the old entry first (`res_Remove`).

### Phase 2: Lazy Loading

`res_GetResource(key)`:

```c
void *res_GetResource(RESOURCE res) {
    ResourceDesc *desc = lookupResourceDesc(idx, res);
    if (desc == NULL) return NULL;           // undefined resource
    if (desc->resdata.ptr == NULL)
        loadResourceDesc(desc);              // lazy load!
    if (desc->resdata.ptr != NULL)
        ++desc->refcount;                    // only increment on success
    return desc->resdata.ptr;                // may still be NULL on load failure
}
```

`loadResourceDesc` simply calls `desc->vtable->loadFun(desc->fname, &desc->resdata)`.

### Reference Counting

- `res_GetResource()` increments `refcount` on each successful access.
- `res_FreeResource()` decrements `refcount`; if it reaches 0, calls `vtable->freeFun` and sets `resdata.ptr = NULL`.
- Comment in code: "refcount is rudimentary as nothing really frees the descriptors"
- Comment in code: `res_FreeResource` "appears to be never called!"

In practice, reference counting is largely vestigial. Most callers use `res_GetResource` followed immediately by `res_DetachResource`.

### `res_DetachResource` Semantics

```c
void *res_DetachResource(RESOURCE res) {
    ResourceDesc *desc = lookupResourceDesc(idx, res);
    // Guards: NULL desc, non-heap resource, not loaded, refcount > 1
    if (desc->refcount > 1) return NULL;   // can't detach multi-referenced
    void *result = desc->resdata.ptr;
    desc->resdata.ptr = NULL;
    desc->refcount = 0;
    return result;
}
```

After detach:
- The descriptor's `resdata.ptr` is NULL, so the next `res_GetResource()` will trigger a fresh load.
- The caller owns the returned pointer and is responsible for freeing it.
- If `refcount > 1`, detach **fails** (returns NULL) with a warning.

### Memory Ownership

All heap-type `Load*Instance` convenience functions follow this pattern:

```c
void *Load*Instance(RESOURCE res) {
    void *hData = res_GetResource(res);    // refcount becomes 1
    if (hData) res_DetachResource(res);    // refcount becomes 0, ptr becomes NULL
    return hData;                          // caller owns the data
}
```

This means every `Load*Instance` call loads a **fresh copy** of the resource if called again, since detach clears the cached pointer.

### `res_Remove` Behavior

```c
BOOLEAN res_Remove(const char *key) {
    ResourceDesc *oldDesc = CharHashTable_find(map, key);
    if (oldDesc != NULL) {
        if (oldDesc->resdata.ptr != NULL) {
            if (oldDesc->refcount > 0) log warning "replacing live resource"
            if (oldDesc->vtable && oldDesc->vtable->freeFun)
                oldDesc->vtable->freeFun(oldDesc->resdata.ptr);
        }
        HFree(oldDesc->fname);
        HFree(oldDesc);
    }
    return CharHashTable_remove(map, key);
}
```

---

## 6. Key-Value Config API

### Type Check Functions

```c
BOOLEAN res_HasKey(const char *key);     // returns TRUE if key exists in index
BOOLEAN res_IsString(const char *key);   // TRUE if key exists AND resType == "STRING"
BOOLEAN res_IsInteger(const char *key);  // TRUE if key exists AND resType == "INT32"
BOOLEAN res_IsBoolean(const char *key);  // TRUE if key exists AND resType == "BOOLEAN"
BOOLEAN res_IsColor(const char *key);    // TRUE if key exists AND resType == "COLOR"
```

### Getter Functions

```c
const char *res_GetString(const char *key);
// Returns desc->resdata.str. Returns "" if key not found, not STRING, or str is NULL.
// WARNING: Lifetime of returned pointer is unclear (TODO comment in source).
// The pointer aliases desc->fname and can be invalidated by res_PutString reallocation.

int res_GetInteger(const char *key);
// Returns desc->resdata.num. Returns 0 if key not found or not INT32.

BOOLEAN res_GetBoolean(const char *key);
// Returns desc->resdata.num ? TRUE : FALSE. Returns FALSE if not found or not BOOLEAN.

Color res_GetColor(const char *key);
// Returns Color struct from packed RGBA in desc->resdata.num.
// Returns buildColorRgba(0,0,0,0) if not found or not COLOR.
```

### Setter Functions

All setters auto-create entries if the key doesn't exist:

```c
void res_PutString(const char *key, const char *value);
// If key doesn't exist or not STRING: creates via process_resource_desc(key, "STRING:undefined")
// If new value is longer than existing fname buffer:
//   - Allocates new buffer, copies value, frees old buffer
//   - desc->fname and desc->resdata.str both point to new buffer
// Else: copies value in-place via strncpy

void res_PutInteger(const char *key, int value);
// If key doesn't exist or not INT32: creates via process_resource_desc(key, "INT32:0")
// Sets desc->resdata.num = value

void res_PutBoolean(const char *key, BOOLEAN value);
// If key doesn't exist or not BOOLEAN: creates via process_resource_desc(key, "BOOLEAN:false")
// Sets desc->resdata.num = value

void res_PutColor(const char *key, Color value);
// If key doesn't exist or not COLOR: creates via process_resource_desc(key, "COLOR:rgb(0, 0, 0)")
// Sets desc->resdata.num = (r<<24 | g<<16 | b<<8 | a)
```

### Color Parsing Formats (in `DescriptorToColor`)

Three formats are supported:

1. **`rgb(r, g, b)`** — 8-bit components, alpha defaults to 0xFF
2. **`rgba(r, g, b, a)`** — 8-bit components including alpha
3. **`rgb15(r, g, b)`** — 5-bit components (0-31), converted to 8-bit via CC5TO8 macro, alpha defaults to 0xFF

Component values support C numeric formats (`%i` scanf: decimal, `0x`hex, `0`octal). Values are clamped to valid range with warnings.

Internal storage: packed 32-bit `DWORD` as `(R<<24 | G<<16 | B<<8 | A)`.

**Note:** `#rrggbb` hex format is present in code but `#if 0` disabled because `#` starts a comment in propfile parsing.

### Color Serialization (`ColorToString`)

```c
// If alpha == 0xFF (opaque):
"rgb(0x%02x, 0x%02x, 0x%02x)"
// Else:
"rgba(0x%02x, 0x%02x, 0x%02x, 0x%02x)"
```

### `SaveResourceIndex`

```c
void SaveResourceIndex(uio_DirHandle *dir, const char *rmpfile,
                       const char *root, BOOLEAN strip_root);
```

Iterates all entries in the hash table. For each entry where:
- Key starts with `root` prefix (if `root` is non-NULL)
- `value->vtable->toString` is non-NULL

Writes:
```
key = TYPE:serialized_value\n
```

If `strip_root` is TRUE, removes the `root` prefix from the key when writing.

Callers:
- `setupmenu.c`: `SaveResourceIndex(configDir, "uqm.cfg", "config.", TRUE)` — saves game options
- `input.c`: `SaveResourceIndex(path, fname, "keys.", TRUE)` — saves key bindings

---

## 7. Path Resolution

### UIO Virtual Filesystem

UQM uses a custom virtual filesystem layer (`libs/uio/`) that supports:
- Mount points (overlaying directories)
- ZIP file reading
- Path resolution across mount points

### Directory Handles

Two primary directory handles (declared in `options.h`):

```c
extern uio_DirHandle *contentDir;  // Game content (base/ and addons/)
extern uio_DirHandle *configDir;   // User configuration files
```

### `res_OpenResFile` (filecntl.c)

```c
uio_Stream *res_OpenResFile(uio_DirHandle *dir, const char *filename, const char *mode) {
    struct stat sb;
    // Special case: if path is a directory, return sentinel (uio_Stream *)~0
    if (uio_stat(dir, filename, &sb) == 0 && S_ISDIR(sb.st_mode))
        return (uio_Stream *)~0;
    return uio_fopen(dir, filename, mode);
}
```

The `(uio_Stream *)~0` sentinel is used by `LengthResFile()` (returns 1 for sentinel) and `res_CloseResFile()` (no-ops for sentinel). This is used by font loading where the path is a directory containing individual character images.

### Content Path Resolution

Resource paths in `.rmp` files are relative to `contentDir`:
- `base/comm/arilou/arilou.ani` → `contentDir/base/comm/arilou/arilou.ani`
- `addons/3domusic/quasispace.ogg` → `contentDir/addons/3domusic/quasispace.ogg`

Most heap-type loaders access content via:
```c
LoadResourceFromPath(pathname, loadFun)
  → res_OpenResFile(contentDir, path, "rb")
```

`CONVERSATION` opens its own files: `res_OpenResFile(contentDir, paths, "rb")`

### Config Path Resolution

Config files are loaded directly:
```c
LoadResourceIndex(configDir, "uqm.cfg", "config.");
```

### Addon Resolution

From `options.c`:

```c
BOOLEAN loadAddon(const char *addon) {
    addonsDir = uio_openDirRelative(contentDir, "addons", 0);
    addonDir = uio_openDirRelative(addonsDir, addon, 0);
    numLoaded = loadIndices(addonDir);  // loads all .rmp files in addon dir
}
```

`loadIndices` scans the directory for `\.rmp$` (regex match) files and calls `LoadResourceIndex` for each.

Addon .rmp files can **override** base resources because `process_resource_desc` calls `res_Remove` on existing entries before inserting new ones:

```c
if (!CharHashTable_add(map, key, newDesc)) {
    res_Remove(key);                    // remove old entry
    CharHashTable_add(map, key, newDesc); // insert new
}
```

### Shadow Content

Addons can also provide a `shadow-content/` directory that gets mounted over the root content directory, allowing them to replace raw asset files without needing .rmp overrides.

---

## 8. Caller Catalog

### Resource Loading API Callers (outside `libs/resource/`)

| File | Function(s) Called | Count |
|------|--------------------|-------|
| `uqm/setupmenu.c` | `res_PutInteger`, `res_PutBoolean`, `res_PutString`, `res_GetBoolean`, `SaveResourceIndex` | 44 |
| `uqm.c` | `LoadResourceIndex`, `res_GetInteger`, `res_GetBoolean`, `res_GetString`, `res_Remove` | 12 |
| `libs/input/sdl/input.c` | `LoadResourceIndex`, `res_GetString`, `res_HasKey`, `res_PutString`, `res_Remove`, `SaveResourceIndex` | 11 |
| `libs/sound/resinst.c` | `res_GetResource`, `res_DetachResource` | 4 |
| `uqm/dummy.c` | `res_GetResource`, `res_DetachResource` | 2 |
| `libs/video/vresins.c` | `res_GetResource`, `res_DetachResource` | 2 |
| `libs/strings/sresins.c` | `res_GetResource`, `res_DetachResource` | 2 |
| `libs/graphics/resgfx.c` | `res_GetResource`, `res_DetachResource` | 2 |
| `uqm/intro.c` | `res_GetResource` (via `LoadLegacyVideoInstance`) | 1 |
| `options.c` | `LoadResourceIndex` | 1 |

### `Load*Instance` Macro Callers (via `nameref.h` macros: `LoadGraphic`, `LoadFont`, `LoadSound`, `LoadMusic`, `LoadStringTable`, `LoadCodeRes`, `LoadColorMapInstance`)

**Total: 144 call sites** across 47 files in `uqm/`.

Top callers by file:

| File | Count | Types |
|------|-------|-------|
| `uqm/planets/lander.c` | 9 | Graphics |
| `uqm/setup.c` | 7 | Graphics, Fonts, Sounds, StringTables |
| `uqm/loadship.c` | 7 | Graphics, Sounds, StringTables, Code, Music |
| `uqm/comm.c` | 6 | Graphics, Fonts, Music, StringTables |
| `uqm/planets/solarsys.c` | 5 | Graphics, Music |
| `uqm/planets/generate/genvux.c` | 5 | Graphics, Sounds, StringTables |
| `uqm/planets/generate/gensol.c` | 5 | Graphics, StringTables |
| `uqm/encount.c` | 5 | Graphics, Sounds, Music |
| `uqm/globdata.c` | 4 | Graphics |
| `uqm/init.c` | 4 | Graphics |
| `uqm/credits.c` | 4 | Graphics, Fonts, StringTables, Music |
| `uqm/cons_res.c` | 4 | Graphics, Music |
| `uqm/outfit.c` | 4 | Graphics, Sounds, Music |
| 34 other files | 1-4 each | Various |

### Callers by Subsystem

| Subsystem | Load* Calls | Config API Calls | Total |
|-----------|-------------|-----------------|-------|
| **Game logic** (`uqm/*.c`) | 64 | 58 | 122 |
| **Planet generation** (`uqm/planets/`) | 54 | 0 | 54 |
| **Super Melee** (`uqm/supermelee/`) | 4 | 0 | 4 |
| **Sound library** (`libs/sound/`) | 0 | 4 | 4 |
| **Graphics library** (`libs/graphics/`) | 0 | 2 | 2 |
| **Strings library** (`libs/strings/`) | 0 | 2 | 2 |
| **Video library** (`libs/video/`) | 0 | 2 | 2 |
| **Input library** (`libs/input/`) | 0 | 11 | 11 |
| **Options** (`options.c`) | 0 | 1 | 1 |

---

## 9. Requirements (EARS Format)

### Index Loading

**REQ-RES-001**: The resource system shall maintain a single global resource index as a string-keyed hash table (`CharHashTable`).

**REQ-RES-002**: When `InitResourceSystem()` is called, the system shall allocate a new resource index with a `CharHashTable` (load factors 0.85/0.9) and store it as the current index.

**REQ-RES-003**: When `InitResourceSystem()` is called and an index already exists, the system shall return the existing index without allocating a new one.

**REQ-RES-004**: When `InitResourceSystem()` is called, the system shall register exactly 14 resource types in this order: UNKNOWNRES, STRING, INT32, BOOLEAN, COLOR, GFXRES, FONTRES, STRTAB, BINTAB, CONVERSATION, SNDRES, MUSICRES, 3DOVID, SHIP.

**REQ-RES-005**: When `LoadResourceIndex(dir, filename, prefix)` is called, the system shall open the file via `res_OpenResFile`, read its entire contents, and parse it as a property file.

**REQ-RES-006**: When a property file is parsed, the system shall treat lines beginning with `#` (after optional whitespace) as comments and skip them entirely.

**REQ-RES-007**: When a property file is parsed, the system shall treat blank lines (only whitespace) as no-ops and skip them.

**REQ-RES-008**: When a property file line contains a key but no `=` separator, the system shall log a warning "Key without value" and skip to the next line.

**REQ-RES-009**: When a property file has a bare key at EOF without a value, the system shall log a warning "Bare keyword at EOF" and stop parsing.

**REQ-RES-010**: When a property file line has `key = value`, the system shall trim whitespace from both the key (trailing) and value (leading and trailing).

**REQ-RES-011**: When a property file line has an inline `#` character in the value portion, the system shall treat everything from `#` onward as a comment and exclude it from the value.

**REQ-RES-012**: When a prefix is provided to `LoadResourceIndex`, the system shall prepend the prefix to every key before processing (e.g., prefix `"config."` + key `"sfxvol"` → `"config.sfxvol"`). The prefix+key buffer shall be limited to 255 characters.

**REQ-RES-013**: When a property file cannot be opened, `PropFile_from_filename` shall silently return without error.

### Type Registration

**REQ-RES-014**: When `InstallResTypeVectors(resType, loadFun, freeFun, stringFun)` is called, the system shall allocate a `ResourceHandlers` struct and store it in the hash table under key `"sys.<resType>"`.

**REQ-RES-015**: The system shall store type registrations in the same hash table as resource entries, distinguished by the `"sys."` prefix on their keys.

**REQ-RES-016**: The `ResourceHandlers` struct shall contain four fields: `resType` (const string pointer), `loadFun`, `freeFun`, and `toString` (function pointers, any of which may be NULL).

**REQ-RES-017**: When `InstallResTypeVectors` cannot allocate memory for the handlers or descriptor, it shall return FALSE.

### Resource Lookup and Creation

**REQ-RES-018**: When `process_resource_desc(key, value)` is called, the system shall parse the value by splitting on the first `:` character to extract the type name and file path.

**REQ-RES-019**: When the value contains no `:` character, the system shall log a warning, treat the type as `"UNKNOWNRES"`, and use the entire value as the path.

**REQ-RES-020**: When the type extracted from the value is not registered, the system shall log a warning "Illegal type" and fall back to the `"UNKNOWNRES"` handler.

**REQ-RES-021**: When the type's `loadFun` is NULL, the system shall log a warning and return NULL (resource not created).

**REQ-RES-022**: When a `ResourceDesc` is created, it shall have `refcount` initialized to 0.

**REQ-RES-023**: When a resource key already exists in the hash table, the system shall remove the old entry (via `res_Remove`) and insert the new one.

### Lazy Loading

**REQ-RES-024**: When a heap-type resource (`freeFun != NULL`) is registered via `newResourceDesc`, the system shall set `resdata.ptr = NULL` (deferred loading).

**REQ-RES-025**: When a value-type resource (`freeFun == NULL`) is registered via `newResourceDesc`, the system shall immediately call `vtable->loadFun(fname, &resdata)` to parse the value.

**REQ-RES-026**: When `res_GetResource(key)` is called with `key == NULL_RESOURCE`, the system shall log a warning "Trying to get null resource" and return NULL.

**REQ-RES-027**: When `res_GetResource(key)` is called and the key is not found in the index, the system shall log a warning "Trying to get undefined resource" and return NULL.

**REQ-RES-028**: When `res_GetResource(key)` is called and `resdata.ptr` is NULL, the system shall call `loadResourceDesc(desc)` to trigger the type's `loadFun`.

**REQ-RES-029**: When `res_GetResource(key)` is called and the resource is successfully loaded (or already loaded), the system shall increment `refcount` by 1 and return the data pointer.

**REQ-RES-030**: When `res_GetResource(key)` is called and the load fails (ptr remains NULL after loadFun), the system shall return NULL without incrementing refcount.

**REQ-RES-031**: When `LoadResourceFromPath(path, loadFun)` is called, it shall open the file via `res_OpenResFile(contentDir, path, "rb")`, get the file length, set `_cur_resfile_name` to path during loading, call the load function, then clear `_cur_resfile_name` to NULL.

**REQ-RES-032**: When `LoadResourceFromPath` cannot open the file, it shall log a warning and return NULL.

**REQ-RES-033**: When `LoadResourceFromPath` opens a zero-length file, it shall log a warning and return NULL.

### Reference Counting

**REQ-RES-034**: The system shall maintain a per-resource `refcount` field that is incremented on each successful `res_GetResource()` call.

**REQ-RES-035**: When `res_FreeResource(key)` is called with `refcount > 0`, the system shall decrement `refcount` by 1.

**REQ-RES-036**: When `res_FreeResource(key)` is called with `refcount == 0`, the system shall log a warning "freeing an unreferenced resource."

**REQ-RES-037**: When `res_FreeResource(key)` is called and `refcount` reaches 0, the system shall call `vtable->freeFun(resdata.ptr)` and set `resdata.ptr = NULL`.

**REQ-RES-038**: When `res_FreeResource(key)` is called on a non-heap resource (`freeFun == NULL`), the system shall log a warning "trying to free a non-heap resource."

**REQ-RES-039**: When `res_FreeResource(key)` is called on a resource that is not loaded (`resdata.ptr == NULL`), the system shall log a warning "trying to free not loaded resource."

**REQ-RES-040**: When `res_FreeResource(key)` is called with an unrecognized key, the system shall log a warning.

### Detach

**REQ-RES-041**: When `res_DetachResource(key)` is called successfully, the system shall return the data pointer, set `resdata.ptr = NULL`, and set `refcount = 0`.

**REQ-RES-042**: When `res_DetachResource(key)` is called on an unrecognized key, it shall log a warning and return NULL.

**REQ-RES-043**: When `res_DetachResource(key)` is called on a non-heap resource, it shall log a warning and return NULL.

**REQ-RES-044**: When `res_DetachResource(key)` is called on a resource that is not loaded, it shall log a warning and return NULL.

**REQ-RES-045**: When `res_DetachResource(key)` is called on a resource with `refcount > 1`, it shall log a warning "trying to detach a resource referenced N times" and return NULL.

**REQ-RES-046**: When a resource is detached and `res_GetResource` is called again for the same key, the system shall perform a fresh load (because `resdata.ptr` is NULL).

### Config Get/Set

**REQ-RES-047**: When `res_GetString(key)` is called for a non-existent or non-STRING key, it shall return an empty string `""`.

**REQ-RES-048**: When `res_GetInteger(key)` is called for a non-existent or non-INT32 key, it shall return 0.

**REQ-RES-049**: When `res_GetBoolean(key)` is called for a non-existent or non-BOOLEAN key, it shall return FALSE.

**REQ-RES-050**: When `res_GetColor(key)` is called for a non-existent or non-COLOR key, it shall return `buildColorRgba(0, 0, 0, 0)`.

**REQ-RES-051**: When `res_PutString(key, value)` is called for a non-existent key, the system shall first create the key with value `"STRING:undefined"`, then update it.

**REQ-RES-052**: When `res_PutString(key, value)` is called with a value longer than the existing `fname` buffer, the system shall allocate a new buffer, copy the value, update both `fname` and `resdata.str`, and free the old buffer.

**REQ-RES-053**: When `res_PutString(key, value)` is called with a value that fits in the existing buffer, the system shall copy the value in-place.

**REQ-RES-054**: When `res_PutInteger(key, value)` is called for a non-existent key, the system shall first create the key with value `"INT32:0"`, then set `resdata.num`.

**REQ-RES-055**: When `res_PutBoolean(key, value)` is called for a non-existent key, the system shall first create the key with value `"BOOLEAN:false"`, then set `resdata.num`.

**REQ-RES-056**: When `res_PutColor(key, value)` is called for a non-existent key, the system shall first create the key with value `"COLOR:rgb(0, 0, 0)"`, then set `resdata.num` to the packed RGBA value.

**REQ-RES-057**: The `STRING` type shall store its value by aliasing `resdata.str` to the same allocation as `fname`. The `UseDescriptorAsRes` load function sets `resdata.str = descriptor` (the fname pointer).

**REQ-RES-058**: The `INT32` type shall parse integer values via `atoi()` and store in `resdata.num`.

**REQ-RES-059**: The `BOOLEAN` type shall recognize `"true"` (case-insensitive) as TRUE and all other values as FALSE.

### Config Persistence

**REQ-RES-060**: When `SaveResourceIndex(dir, file, root, strip_root)` is called, the system shall iterate all entries in the hash table.

**REQ-RES-061**: When saving, the system shall only write entries whose key starts with the `root` prefix (if non-NULL) and whose `vtable->toString` is non-NULL.

**REQ-RES-062**: When saving with `strip_root = TRUE`, the system shall remove the `root` prefix from the key in the output file.

**REQ-RES-063**: When saving, each entry shall be written as `key = TYPE:serialized_value\n` where `serialized_value` is produced by calling `vtable->toString(&resdata, buf, 256)`.

**REQ-RES-064**: When saving, entries with no value, no vtable, or no `toString` function shall be skipped with a warning logged for missing value or vtable cases.

**REQ-RES-065**: When the output file cannot be opened for writing, `SaveResourceIndex` shall silently return.

### Color Parsing

**REQ-RES-066**: When parsing a color descriptor, the system shall support `rgb(r, g, b)` format with 8-bit integer components and implicit alpha of 0xFF.

**REQ-RES-067**: When parsing a color descriptor, the system shall support `rgba(r, g, b, a)` format with 8-bit integer components.

**REQ-RES-068**: When parsing a color descriptor, the system shall support `rgb15(r, g, b)` format with 5-bit integer components (0-31), converting each to 8-bit via the CC5TO8 macro, and implicit alpha of 0xFF.

**REQ-RES-069**: When parsing color components, the system shall accept C integer formats (decimal, 0x hex, 0 octal) via `sscanf %i`.

**REQ-RES-070**: When a color component value is below 0, the system shall clamp it to 0 and log a warning.

**REQ-RES-071**: When a color component value exceeds the maximum for its bit depth, the system shall clamp it to the maximum and log a warning.

**REQ-RES-072**: If the color descriptor cannot be parsed as any recognized format, the system shall log an error and set the value to `0x00000000`.

**REQ-RES-073**: When serializing a color with alpha == 0xFF, the system shall output `rgb(0x%02x, 0x%02x, 0x%02x)` format.

**REQ-RES-074**: When serializing a color with alpha != 0xFF, the system shall output `rgba(0x%02x, 0x%02x, 0x%02x, 0x%02x)` format.

### Path Resolution

**REQ-RES-075**: When `res_OpenResFile(dir, filename, mode)` is called and the path is a directory, it shall return the sentinel value `(uio_Stream *)~0`.

**REQ-RES-076**: When `res_CloseResFile` is called with the sentinel value `(uio_Stream *)~0`, it shall no-op (not call `uio_fclose`).

**REQ-RES-077**: When `LengthResFile` is called with the sentinel value, it shall return 1.

**REQ-RES-078**: All heap-type resource loading shall resolve file paths relative to `contentDir` via `res_OpenResFile(contentDir, path, "rb")`.

**REQ-RES-079**: Config files shall be loaded from `configDir` via `LoadResourceIndex(configDir, filename, prefix)`.

**REQ-RES-080**: When loading addons, the system shall open `contentDir/addons/<addon_name>/`, scan for all `.rmp` files (regex `\.[rR][mM][pP]$`), and call `LoadResourceIndex` for each.

**REQ-RES-081**: When an addon defines the same resource key as a previously loaded index, the new entry shall replace the old one (enabling content overrides).

### Error Handling

**REQ-RES-082**: When `_get_current_index_header()` is called and no index exists, the system shall call `InitResourceSystem()` to auto-initialize.

**REQ-RES-083**: When `res_Remove(key)` is called on a resource with `refcount > 0`, the system shall log a warning "Replacing while live" but proceed with removal.

**REQ-RES-084**: When `res_Remove(key)` is called and the resource has loaded data with a `freeFun`, the system shall call `freeFun` before freeing the descriptor.

**REQ-RES-085**: When `res_Remove(key)` is called, the system shall free both `fname` and the `ResourceDesc` itself via `HFree`.

**REQ-RES-086**: When `GetResourceData` reads a `compLen` prefix that is not `~0`, it shall log a warning "LZ-compressed binary data not supported" and return NULL.

**REQ-RES-087**: When `GetResourceData` reads a `compLen` prefix of `~0`, it shall skip the 4-byte prefix and read `length - sizeof(DWORD)` bytes of uncompressed data.

### Lifecycle

**REQ-RES-088**: When `UninitResourceSystem()` is called, the system shall free the hash table (via `CharHashTable_deleteHashTable`) and the index descriptor, then set the current index to NULL.

**REQ-RES-089**: The system shall support calling `LoadResourceIndex` multiple times to accumulate entries from multiple files into the same index.

### Resource Type: CONVERSATION (Special)

**REQ-RES-090**: The `CONVERSATION` type shall parse its path string for up to three colon-separated components: text file path, speech clip directory path, and timestamp file path.

**REQ-RES-091**: The `CONVERSATION` type's `loadFun` (`_GetConversationData`) shall be registered as a `ResourceLoadFun` (taking `const char *path, RESOURCE_DATA *resdata`) rather than using the `LoadResourceFromPath` two-tier pattern.

### Resource Type: 3DOVID (Special)

**REQ-RES-092**: The `3DOVID` type shall parse its path string for up to four colon-separated components: video path, audio path, speech path, and loop frame number.

**REQ-RES-093**: When the `3DOVID` type's loop frame string contains non-whitespace characters after the number, the system shall log a warning and disable looping (`VID_NO_LOOP`).

**REQ-RES-094**: The `3DOVID` type shall allocate a `LEGACY_VIDEO_DESC` struct containing heap-allocated copies of the video, audio, and speech paths plus the parsed loop frame integer.

### Resource Type: SHIP (Special)

**REQ-RES-095**: The `SHIP` type shall interpret its path string as an integer (via `atoi`) mapping to a `ShipCodeRes` enum value.

**REQ-RES-096**: The `SHIP` type shall use the enum value to select a `RaceDescInitFunc` function pointer and call it to obtain a `RACE_DESC`.

**REQ-RES-097**: If the `SHIP` integer maps to an unknown enum value, the system shall log a warning and the `CODERES_STRUCT` allocation shall be freed (returning NULL).

### Convenience Wrappers

**REQ-RES-098**: Each subsystem shall provide a `Load*Instance` function that calls `res_GetResource` followed by `res_DetachResource`, transferring ownership of a fresh copy to the caller.

**REQ-RES-099**: The `nameref.h` header shall provide macros (`LoadGraphic`, `LoadFont`, `LoadSound`, `LoadMusic`, `LoadStringTable`, `LoadCodeRes`) that call the appropriate `Load*Instance` function with optional cast.

**REQ-RES-100**: `LoadColorMapInstance` shall be aliased to `LoadStringTableInstance` (color maps use the BINTAB format internally).

### Additional Value Access

**REQ-RES-101**: `res_GetIntResource(key)` shall look up the key and return `resdata.num` directly (no type checking, no refcount increment).

**REQ-RES-102**: `res_GetBooleanResource(key)` shall return `res_GetIntResource(key) != 0`.

**REQ-RES-103**: `res_GetResourceType(key)` shall return `desc->vtable->resType` for the given key, or NULL if the key is null or undefined.

### StringBank (Utility)

**REQ-RES-104**: The `stringbank` module shall provide an arena-style string allocator with fixed-size chunks of `STRBANK_CHUNK_SIZE` (1024 - sizeof(void*) - sizeof(int)) bytes.

**REQ-RES-105**: `StringBank_AddString` shall append a string to the first chunk with sufficient space, allocating a new chunk if needed.

**REQ-RES-106**: `StringBank_AddOrFindString` shall search all chunks for an identical string before adding, providing deduplication.

**REQ-RES-107**: `SplitString` shall split a string on a delimiter character, storing up to N substrings in the bank, and return the actual count of substrings produced.

### Binary Resource Data (loadres.c)

**REQ-RES-108**: `GetResourceData` shall read a 4-byte `DWORD` length prefix from the stream. If the prefix equals `~(DWORD)0`, it shall treat the remaining data as uncompressed and allocate + read `length - 4` bytes.

**REQ-RES-109**: If the `DWORD` prefix is any value other than `~(DWORD)0`, `GetResourceData` shall log a warning about unsupported LZ compression and return NULL.

**REQ-RES-110**: `AllocResourceData` shall be a macro alias for `HMalloc`. `FreeResourceData` shall free via `HFree` and return TRUE.

### Directory Entry Loading (direct.c)

**REQ-RES-111**: `LoadDirEntryTable` shall scan a directory for files matching a pattern, skip entries starting with `.` and non-regular files, and return the results as a `STRING_TABLE` (array of name strings).

**REQ-RES-112**: If no matching entries are found, `LoadDirEntryTable` shall return 0 (`NULL`).

### File I/O Layer (filecntl.c)

**REQ-RES-113**: The file I/O layer shall wrap all UIO operations (`uio_fopen`, `uio_fclose`, `uio_fread`, `uio_fwrite`, `uio_fseek`, `uio_ftell`, `uio_getc`, `uio_putc`, `uio_fstat`, `uio_unlink`) in named functions (`res_OpenResFile`, `res_CloseResFile`, `ReadResFile`, `WriteResFile`, `SeekResFile`, `TellResFile`, `GetResFileChar`, `PutResFileChar`, `DeleteResFile`, `LengthResFile`).

**REQ-RES-114**: `PutResFileNewline` shall write `\r\n` on Windows and `\n` on all other platforms.

**REQ-RES-115**: `res_CloseResFile` shall return TRUE on success (including NULL and sentinel inputs) and FALSE if the file pointer is NULL.

---

## Appendix A: File Inventory

| File | Purpose | Lines |
|------|---------|-------|
| `libs/resource/resinit.c` | System init, type registration, config get/set, save/load index, key-value API | ~652 |
| `libs/resource/getres.c` | `res_GetResource`, `res_FreeResource`, `res_DetachResource`, `LoadResourceFromPath`, `lookupResourceDesc` | ~255 |
| `libs/resource/loadres.c` | `GetResourceData` (binary .ct/.xlt file loader with DWORD prefix) | ~52 |
| `libs/resource/propfile.c` | Property file parser (`.rmp`, `.cfg`, `.key`) | ~116 |
| `libs/resource/direct.c` | `LoadDirEntryTable` (directory scanning into STRING_TABLE) | ~97 |
| `libs/resource/filecntl.c` | File I/O wrapper layer over UIO | ~150 |
| `libs/resource/stringbank.c` | Arena-style string allocator with deduplication | ~151 |
| `libs/resource/index.h` | Data structure definitions (ResourceDesc, ResourceHandlers, resource_index_desc) | ~56 |
| `libs/resource/resintrn.h` | Internal header (lookupResourceDesc, load, get/set current index) | ~35 |
| `libs/resource/propfile.h` | Property file API (PROPERTY_HANDLER typedef) | ~32 |
| `libs/resource/stringbank.h` | StringBank API | ~60 |
| `libs/reslib.h` | Public API header (all extern declarations) | ~130 |
| `libs/graphics/resgfx.c` | GFXRES/FONTRES type registration + LoadGraphicInstance | ~54 |
| `libs/sound/resinst.c` | SNDRES/MUSICRES type registration + LoadSoundInstance/LoadMusicInstance | ~67 |
| `libs/strings/sresins.c` | STRTAB/BINTAB/CONVERSATION type registration + LoadStringTableInstance | ~54 |
| `libs/video/vresins.c` | 3DOVID type registration + LoadLegacyVideoInstance | ~197 |
| `uqm/dummy.c` | SHIP type registration + LoadCodeResInstance | ~200 |

## Appendix B: Resource Type Distribution in uqm.rmp

| Type | Count | Percentage |
|------|-------|------------|
| GFXRES | 582 | 60.4% |
| BINTAB | 149 | 15.5% |
| MUSICRES | 64 | 6.6% |
| STRTAB | 51 | 5.3% |
| FONTRES | 32 | 3.3% |
| SNDRES | 30 | 3.1% |
| SHIP | 28 | 2.9% |
| CONVERSATION | 27 | 2.8% |
| **Total** | **963** | **100%** |

## Appendix C: Key Naming Conventions

Resource keys follow a hierarchical dot-separated naming convention:

```
comm.<race>.graphics     — Communication screen animation
comm.<race>.music        — Communication music
comm.<race>.font         — Communication font
comm.<race>.dialogue     — Conversation data
comm.<race>.colortable   — Color table for communication
ship.<race>.code         — Ship code resource (SHIP type)
ship.<race>.sounds       — Ship sound effects
ship.<race>.icons        — Ship icon graphics
music.<name>             — Background music
slides.<name>            — Video/slideshow resources
colortable.<name>        — Global color tables
config.<name>            — Configuration values (from .cfg)
keys.<n>.<action>        — Key binding definitions (from .key)
menu.<action>            — Menu key bindings (from .key)
```
