# Resource System Domain Model

## Entity Relationships

```
┌─────────────────────────────────────────────┐
│          ResourceIndexDesc (singleton)        │
│  ┌─────────────────────────────────────────┐ │
│  │  HashMap<String, ResourceDesc>          │ │
│  │                                         │ │
│  │  "sys.GFXRES"    → TypeHandler entry    │ │
│  │  "sys.STRING"    → TypeHandler entry    │ │
│  │  "sys.INT32"     → TypeHandler entry    │ │
│  │  "comm.arilou.graphics" → Resource entry│ │
│  │  "config.sfxvol" → Resource entry       │ │
│  └─────────────────────────────────────────┘ │
└─────────────────────────────────────────────┘
         │ contains
         ▼
┌─────────────────────────────────────────┐
│           ResourceDesc                   │
│  fname: CString (owned)                  │
│  vtable: *const ResourceHandlers | NULL  │
│  resdata: ResourceData (union)           │
│  refcount: u32                           │
└─────────┬───────────────────────────────┘
          │ references
          ▼
┌─────────────────────────────────────────┐
│         ResourceHandlers                 │
│  res_type: *const c_char ("GFXRES")     │
│  load_fun: Option<ResourceLoadFun>       │
│  free_fun: Option<ResourceFreeFun>       │
│  to_string: Option<ResourceStringFun>    │
└─────────────────────────────────────────┘
```

## Resource Lifecycle

```
                    ┌───────────────┐
                    │  UNINITIALIZED │
                    └───────┬───────┘
                            │ InitResourceSystem()
                            ▼
                    ┌───────────────┐
                    │  INITIALIZED   │ ← 14 types registered,
                    │  (empty index) │   HashMap created
                    └───────┬───────┘
                            │ LoadResourceIndex() × N
                            ▼
                    ┌───────────────┐
                    │  POPULATED     │ ← Entries in HashMap
                    │                │   Value types: data parsed
                    │                │   Heap types: ptr = NULL
                    └───────┬───────┘
                            │ res_GetResource()
                            ▼
┌───────────┐       ┌───────────────┐
│ LOAD FAIL │ ←──── │  LAZY LOAD    │ ← Call loadFun via vtable
│ (ptr=NULL)│       │  (heap types) │
└───────────┘       └───────┬───────┘
                            │ success: ptr != NULL, refcount++
                            ▼
                    ┌───────────────┐
                    │  LOADED        │ ← refcount ≥ 1
                    │  (cached)      │   ptr = subsystem data
                    └───┬───┬───┬───┘
         res_GetResource│   │   │res_FreeResource (refcount→0)
         (refcount++)   │   │   ▼
                        │   │ ┌───────────────┐
                        │   │ │  FREED         │ ← freeFun(ptr),
                        │   │ │  (ptr = NULL)  │   ptr = NULL
                        │   │ │  (back to lazy)│   next Get reloads
                        │   │ └───────────────┘
                        │   │
                        │   │ res_DetachResource (refcount==1)
                        │   ▼
                    ┌───────────────┐
                    │  DETACHED      │ ← ptr returned to caller
                    │  (ptr = NULL,  │   caller owns data
                    │   refcount = 0)│   next Get reloads
                    └───────────────┘
```

## Two-Phase Loading Model

### Phase 1: Index Loading (startup)

1. `InitResourceSystem()` creates empty HashMap, registers 14 types
2. `LoadResourceIndex(configDir, "uqm.cfg", "config.")` — config values
3. `loadIndices(contentDir)` — all `.rmp` files (963+ entries)
4. `LoadResourceIndex(contentDir, "menu.key", "menu.")` — menu keys
5. `LoadResourceIndex(configDir, "override.cfg", "menu.")` — overrides
6. `LoadResourceIndex(configDir, "flight.cfg", "keys.")` — flight keys
7. `LoadResourceIndex(contentDir, "uqm.key", "keys.")` — default keys
8. For each addon: `loadIndices(addonDir)` — addon overrides

For each `key = TYPE:value` line:
- Split on first `:` → type name + path/value
- Lookup `sys.TYPE` → get ResourceHandlers vtable
- If `freeFun == NULL` (value type): call `loadFun(fname, &resdata)` immediately
- If `freeFun != NULL` (heap type): set `resdata.ptr = NULL` (deferred)
- Insert into HashMap (replace if key exists)

### Phase 2: Lazy Loading (runtime)

- `res_GetResource(key)` → if `ptr == NULL`, call `loadFun`
- C loader opens file via UIO, parses binary format, returns subsystem data
- Rust stores opaque `*mut c_void` — never dereferences it

## Config vs Game Resource Distinction

| Property | Config Resources | Game Resources |
|----------|-----------------|----------------|
| Types | STRING, INT32, BOOLEAN, COLOR | GFXRES, FONTRES, STRTAB, etc. |
| `freeFun` | NULL | Non-NULL |
| Loading | Immediate (at index parse) | Lazy (on first access) |
| Storage | `resdata.num` or `resdata.str` | `resdata.ptr` (opaque) |
| Access API | `res_Get/Put{String,Integer,Boolean,Color}` | `res_GetResource` / `res_DetachResource` |
| Persistence | `SaveResourceIndex` writes to `.cfg` | Never written |
| Sources | `.cfg`, `.key` files | `.rmp` files |
| Prefix | `"config."`, `"keys."`, `"menu."` | None (or addon-specific) |
| Refcount | Always 0 (not applicable) | Managed |

## Path Resolution Through UIO

```
Resource path in .rmp:  "base/comm/arilou/arilou.ani"
                              │
                              ▼
              res_OpenResFile(contentDir, path, "rb")
                              │
                              ▼
              uio_fopen(contentDir, path, "rb")
                              │
                              ▼
         UIO resolves across mount points:
         - Base content directory (stdio mount)
         - Mounted .zip packages
         - Addon shadow-content directories
                              │
                              ▼
         Returns uio_Stream* (opaque to Rust)
```

## Error/Edge Case Map

| Condition | Behavior |
|---|---|
| NULL key to any function | Warning + safe default (NULL/0/"") |
| Undefined key lookup | Warning + safe default |
| File not found on index load | Silent return (no error) |
| Missing `:` in value | Warning, use UNKNOWNRES type |
| Unregistered type name | Warning, fallback to UNKNOWNRES |
| loadFun is NULL | Warning, skip entry |
| Load failure (ptr still NULL) | Return NULL, no refcount increment |
| Free with refcount 0 | Warning "freeing unreferenced" |
| Free non-heap resource | Warning "trying to free non-heap" |
| Free unloaded resource | Warning "trying to free not loaded" |
| Detach with refcount > 1 | Warning, return NULL |
| Detach non-heap resource | Warning, return NULL |
| Replace live resource (res_Remove) | Warning "replacing while live", proceed |
| LZ-compressed data prefix | Warning, return NULL |
| Color component out of range | Clamp + warning |
| Unrecognized color format | Error log, store 0x00000000 |
| Poisoned Rust mutex | Safe default + error log |

## Integration Touchpoints

### Callers to redirect (C → Rust FFI)

| Call site group | Count | Functions used |
|---|---|---|
| Config get/put (`setupmenu.c`) | 44 | `res_PutInteger`, `res_PutBoolean`, `res_PutString`, `res_GetBoolean`, `SaveResourceIndex` |
| Game init (`uqm.c`) | 12 | `LoadResourceIndex`, `res_GetInteger`, `res_GetBoolean`, `res_GetString`, `res_Remove` |
| Input (`input.c`) | 11 | `LoadResourceIndex`, `res_GetString`, `res_HasKey`, `res_PutString`, `res_Remove`, `SaveResourceIndex` |
| Resource loading (5 files) | 10 | `res_GetResource`, `res_DetachResource` |
| Load*Instance macros (47 files) | 144 | `res_GetResource`, `res_DetachResource` (via macros) |

### Old code to replace/remove

When `USE_RUST_RESOURCE` is defined:
- `resinit.c`: Guard all function bodies with `#ifndef USE_RUST_RESOURCE`
- `getres.c`: Guard all function bodies
- `propfile.c`: Guard all function bodies
- `loadres.c`: Guard all function bodies
- `filecntl.c`: Guard all function bodies
- `rust_resource.c`: Repurpose as the Rust dispatch shim
