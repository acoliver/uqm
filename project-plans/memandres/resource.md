# Resource System: C → Rust Swap Specification

## 1. Current State

### 1.1 C Resource Pipeline

The C resource system is a **two-phase** design: an index phase that maps string keys to descriptors, and a loading phase that materializes resources on demand.

#### Phase 1: Index & Type Registration

`InitResourceSystem()` (resinit.c:333) is called once during startup (from `uqm/setup.c:109`). It:

1. Allocates a `RESOURCE_INDEX_DESC` containing a `CharHashTable`.
2. Registers **built-in type handlers** via `InstallResTypeVectors()`:
   - `UNKNOWNRES` — pass-through (descriptor string is the value)
   - `STRING` — pass-through (descriptor string is the value)
   - `INT32` — `atoi()` at registration time (not load time)
   - `BOOLEAN` — `strcasecmp("true")` at registration time
   - `COLOR` — parses `rgb()`, `rgba()`, `rgb15()` at registration time
3. Registers **subsystem type handlers**:
   - `InstallGraphicResTypes()` → `GFXRES` (cel/animation data), `FONTRES` (font data)
   - `InstallStringTableResType()` → `STRTAB` (string tables), `BINTAB` (binary tables), `CONVERSATION` (dialogue)
   - `InstallAudioResTypes()` → `SNDRES` (sound banks), `MUSICRES` (music)
   - `InstallVideoResType()` → `3DOVID` (legacy video)
   - `InstallCodeResType()` → `SHIP` (code/logic resources)

After type registration, `.rmp` index files are loaded via `LoadResourceIndex()` → `PropFile_from_filename()` → `process_resource_desc()`. For each key=value line, `newResourceDesc()`:
- Splits the value at `:` to extract `TYPE:path`
- Looks up the handler via `"sys.TYPE"` key in the hash table
- For non-heap types (STRING, INT32, BOOLEAN, COLOR): **immediately** invokes `loadFun` to parse the descriptor string into `resdata`
- For heap types (GFXRES, etc.): stores `fname` and sets `resdata.ptr = NULL` (deferred loading)

#### Phase 2: Resource Loading

`res_GetResource()` (getres.c:104) is the hot path:
1. Looks up `ResourceDesc` in the hash table by string key
2. If `resdata.ptr == NULL`, calls `loadResourceDesc()` → `vtable->loadFun(fname, &resdata)`
3. For heap resources, `loadFun` calls `LoadResourceFromPath()` which opens the file via **`res_OpenResFile(contentDir, path, "rb")`** — note the use of the global `contentDir` UIO handle
4. Increments `refcount` and returns `resdata.ptr`

The system also provides a **Key-Value API** (resinit.c:466-651) for configuration: `res_GetString()`, `res_GetInteger()`, `res_GetBoolean()`, `res_GetColor()`, and their `Put`/`Is`/`HasKey` counterparts. These operate directly on the same hash table but for non-heap (immediate) resources. This API is heavily used for game settings (`config.*`, `keys.*`, `menu.*`).

#### .rmp File Format

The format is simple `key = TYPE:path` property files:

```
comm.arilou.graphics = GFXRES:base/comm/arilou/arilou.ani
comm.arilou.music = MUSICRES:base/comm/arilou/arilou.mod
music.battle = MUSICRES:addons/3domusic/battle.ogg
```

The `.key` and `.cfg` files use the same format but with `STRING:`, `INT32:`, `BOOLEAN:`, `COLOR:` types:

```
1.name = STRING:Arrows
1.up.1 = STRING:key Up
config.fullscreen = BOOLEAN:true
config.gamma = INT32:100
```

The main `uqm.rmp` contains **963 entries** across 8 resource types:
| Type | Count | Description |
|------|-------|-------------|
| GFXRES | 582 | Graphics (cel/animation) |
| BINTAB | 149 | Binary tables (color tables, etc.) |
| MUSICRES | 64 | Music files |
| STRTAB | 51 | String tables |
| FONTRES | 32 | Font data |
| SNDRES | 30 | Sound banks |
| SHIP | 28 | Ship code resources |
| CONVERSATION | 27 | Dialogue text |

Addon `.rmp` files (3domusic, 3dovideo, 3dovoice, remix) add ~88 more entries, primarily overriding existing keys with addon-specific paths.

#### Index Loading Sequence

At startup:
1. `prepareContentDir()` mounts the content directory via UIO, sets global `contentDir`
2. `prepareConfigDir()` mounts the config directory, sets global `configDir`
3. `InitResourceSystem()` registers type handlers
4. `LoadResourceIndex(configDir, "uqm.cfg", "config.")` — user config (prefixed)
5. `loadIndices(contentDir)` — loads ALL `.rmp` files found via regex `\.[rR][mM][pP]$`
6. `LoadResourceIndex(contentDir, "menu.key", "menu.")` — menu keys (prefixed)
7. `LoadResourceIndex(configDir, "override.cfg", "menu.")` — menu overrides
8. `LoadResourceIndex(configDir, "flight.cfg", "keys.")` — flight keys
9. `LoadResourceIndex(contentDir, "uqm.key", "keys.")` — default keys
10. Addon indices loaded via `loadAddon()` → `loadIndices(addonDir)`

**Critical behavior**: Later loads **override** earlier loads for the same key. This is how addons work — 3domusic.rmp replaces `music.*` entries with its own paths.

### 1.2 Rust Resource System

The Rust resource system lives in `rust/src/resource/` with 8 modules:

| Module | Description | Status |
|--------|-------------|--------|
| `index.rs` | `ResourceIndex` — HashMap-based key→entry mapping | Implemented, case-insensitive, merge support |
| `propfile.rs` | `PropertyFile` — key=value parser | Implemented, BUT uses `split_once('=')` with `BufReader::lines()` |
| `cache.rs` | `ResourceCache` — LRU eviction cache with pinning | Implemented, thread-safe (`RwLock<HashMap>` + `Arc`) |
| `loader.rs` | `ResourceLoader` — raw byte loading from filesystem | Implemented, path traversal protection |
| `resource_system.rs` | `ResourceSystem` — high-level typed resource access | Implemented, BUT format mismatch with C (see gaps) |
| `resource_type.rs` | `ResourceType` enum + `ResourceValue` + `ColorResource` | Implemented, missing key C types |
| `stringbank.rs` | `StringBank` — localized string tables | Implemented, language fallback chain |
| `ffi.rs` | C FFI bindings for all of the above | Implemented, 15+ `#[no_mangle]` functions |

### 1.3 The Existing rust_resource.c Bridge

`rust_resource.c` (compiled when `USE_RUST_RESOURCE` is defined) provides:
- `RustResourceInit()` — initializes cache with 64MB limit
- `RustResourceUninit()` — clears cache
- `RustResourceLoad(name, &size)` — cache-check → `rust_resource_load()` → cache-insert
- `RustResourceFree(data, size)` — frees Rust-allocated memory
- `RustResourceExists(name)` — checks index
- Cache management: `RustResourceCacheClear()`, `RustResourceCacheSize()`, `RustResourceCacheCount()`

**Why it's not called**: `USE_RUST_RESOURCE` is a build option that is not enabled by default. Even if enabled, `rust_resource.c` is a **sidecar cache** — it wraps the Rust loader/cache but is never wired into the actual C resource pipeline. No code in `resinit.c`, `getres.c`, or any consumer calls `RustResourceLoad()`. The bridge exists as scaffolding but has zero integration points.

---

## 2. Gap Analysis

### 2.1 .rmp Format Parsing — Critical Mismatch

**C format**: `key = TYPE:path` (e.g., `comm.arilou.graphics = GFXRES:base/comm/arilou/arilou.ani`)

**Rust `ResourceIndex::parse()`**: Treats the entire right side of `=` as a `file_path`. It does **not** split on `:` to extract the type. This means:
- `ResourceEntry.file_path` would be `"GFXRES:base/comm/arilou/arilou.ani"` (wrong)
- There is no `resource_type` field on `ResourceEntry` that corresponds to the C `TYPE`

**Rust `ResourceSystem::load_index()`**: Expects `FILENAME,TYPE` format (comma-separated) — this is yet another format that matches neither C nor the actual `.rmp` files.

**Rust `PropertyFile`**: Parses correctly but uppercases all keys, which the C code does **not** do (C uses case-sensitive hash table keys for resource names, though it uses case-insensitive matching for the UIO filesystem).

**Gap**: The Rust index parser **cannot read actual .rmp files correctly**. This is a blocking issue.

### 2.2 Type-Specific Resource Loaders — Not in Rust

The C system has 10 registered resource types with specialized loaders:

| C Type | Load Function | Rust Equivalent |
|--------|---------------|-----------------|
| `GFXRES` | `_GetCelData()` → cel/animation parsing | [ERROR] None |
| `FONTRES` | `_GetFontData()` → font file parsing | [ERROR] None |
| `STRTAB` | `_GetStringData()` → string table parsing | [ERROR] None |
| `BINTAB` | `_GetBinaryTableData()` → binary table loading | [ERROR] None |
| `CONVERSATION` | `_GetConversationData()` → dialogue parsing | [ERROR] None |
| `SNDRES` | `_GetSoundBankData()` → sound bank loading | [ERROR] None |
| `MUSICRES` | `_GetMusicData()` → music file loading | [ERROR] None |
| `3DOVID` | `GetLegacyVideoData()` → video loading | [ERROR] None |
| `SHIP` | `_GetCodeResData()` → code resource loading | [ERROR] None |
| `STRING`/`INT32`/`BOOLEAN`/`COLOR` | Immediate parsing (no file I/O) | WARNING: Partial — types exist, parsing differs |

The Rust `ResourceType` enum has: `String`, `Integer`, `Boolean`, `Color`, `Binary`, `Unknown`. It is missing `GFXRES`, `FONTRES`, `STRTAB`, `BINTAB`, `CONVERSATION`, `SNDRES`, `MUSICRES`, `3DOVID`, `SHIP`. More critically, the C type-specific loaders are deeply coupled to the graphics, sound, and video subsystems — they don't just read bytes; they parse binary formats (`.ani`, `.fon`, `.ct`, `.mod`, etc.) into subsystem-specific data structures.

### 2.3 Two-Phase Loading — Partially Modeled

The C system has two distinct kinds of resources:
1. **Immediate** (STRING, INT32, BOOLEAN, COLOR): Value is parsed from the descriptor string at index-load time. No file I/O needed later. `freeFun == NULL`.
2. **Deferred** (GFXRES, etc.): File path is stored. Actual loading happens lazily on first `res_GetResource()` call. `freeFun != NULL`.

The Rust `ResourceSystem` attempts this with `ResourceDescriptor.data: Option<Arc<ResourceValue>>`, but the split isn't clean. `load_resource_file()` reads from the filesystem for ALL types, even `String` and `Integer`, which the C code resolves entirely from the descriptor string (no file I/O at all).

### 2.4 Key-Value Config API — Not in Rust

The C code provides `res_GetString()`, `res_PutString()`, `res_GetInteger()`, `res_PutInteger()`, `res_GetBoolean()`, `res_PutBoolean()`, `res_GetColor()`, `res_PutColor()`, `res_HasKey()`, `res_IsString()`, etc. These are used extensively:
- **85+ call sites** across `uqm.c`, `setupmenu.c`, `input.c`
- Used for reading/writing game configuration (`config.*` prefix)
- Used for keybinding resolution (`keys.*`, `menu.*` prefixes)
- `SaveResourceIndex()` writes the hash table back to `.cfg` files

The Rust FFI exposes `rust_get_string_resource()`, `rust_get_int_resource()`, `rust_get_bool_resource()`, but these go through the `ResourceSystem` which has the format mismatch described above. There is no `Put` API in Rust. There is no `SaveResourceIndex` equivalent.

### 2.5 Resource Descriptor / refcount — Partially Modeled

C `ResourceDesc` has:
- `fname` — the file path (or literal value for immediate types; in STRING, `fname` IS the string value)
- `vtable` — handler (load/free/toString functions)
- `resdata` — union of `{num, ptr, str}`
- `refcount` — rudimentary reference counting

The Rust `ResourceCache` has `CachedResource` with `ref_count: AtomicUsize` and LRU eviction. The Rust `ResourceSystem` has `ResourceDescriptor.ref_count`. These are independent systems. In C, there is exactly one mechanism.

### 2.6 `res_DetachResource()` — Not in Rust

The C code has `res_DetachResource()` which transfers ownership of a loaded resource to the caller and NULLs out the descriptor's pointer, forcing a reload on next `res_GetResource()`. This is used by graphics, sound, strings, and video resource installers. The Rust system has no equivalent.

### 2.7 Color Parsing — Format Mismatch

C parses: `rgb(r, g, b)`, `rgba(r, g, b, a)`, `rgb15(r, g, b)` — functional CSS-like syntax.
Rust parses: `#RRGGBB`, `#RRGGBBAA` — hex notation only.
Actual `.rmp`/`.cfg` files use the C format: `COLOR:rgb(0x1a, 0x00, 0x1a)`.

---

## 3. Path Handling Deep Dive

### 3.1 UIO Virtual Filesystem Layer

UQM uses a custom virtual filesystem (UIO — `libs/uio/`) that provides:
- **Mount points**: Multiple filesystem locations merged into a single virtual directory tree
- **Layered mounts**: Content, addons, packages (`.zip`) are layered with precedence rules (`uio_MOUNT_TOP`, `uio_MOUNT_BELOW`)
- **Case-insensitive matching**: UIO normalizes case on case-sensitive filesystems
- **Transparent decompression**: `.zip` archives mounted as directories

The mount hierarchy:
```
/                     ← contentDir (stdio mount of real content path)
├── packages/*.zip    ← auto-mounted below content
├── addons/
│   ├── 3domusic/     ← loaded if --addon=3domusic
│   ├── 3dovoice/
│   ├── remix/
│   └── *.zip         ← addon zips auto-mounted
```

The global `contentDir` (`uio_DirHandle *`) is an opaque handle to the UIO root. **All** resource file operations go through UIO:
- `res_OpenResFile(contentDir, path, "rb")` → `uio_fopen(dir, path, mode)`
- `LoadDirEntryTable(dirHandle, path, pattern, matchType)` → `uio_getDirList()`
- Font directories: `uio_openDirRelative(contentDir, _cur_resfile_name, 0)`

### 3.2 How contentDir Gets Resolved

`prepareContentDir()` in `options.c:136`:

1. If `--contentdir=PATH` was specified on the command line, use that.
2. Otherwise, try default locations in order:
   - `CONTENTDIR` (compile-time constant from `config.h`, typically `/usr/share/uqm/content`)
   - `""` (current working directory)
   - `"content"` (relative to cwd)
   - `"../../content"` (for MSVC builds)
   - On macOS: `<execDir>/../Resources/content` (app bundle)
3. Validation: looks for a `version` file in each candidate directory.
4. `expandPath()` resolves `~`, environment variables, etc.
5. `uio_mountDir(repository, "/", uio_FSTYPE_STDIO, ..., contentPath, ...)` mounts the resolved path.
6. `contentDir = uio_openDir(repository, "/", 0)` opens the root of the mounted tree.

### 3.3 How configDir Gets Resolved

`prepareConfigDir()` in `options.c:205`:

1. If `--configdir=PATH` was specified, use that.
2. If `UQM_CONFIG_DIR` environment variable is set, use that.
3. Default: `CONFIGDIR` (compile-time, typically `~/.uqm`).
4. `expandPath()` resolves the path.
5. `mkdirhier()` creates the directory if it doesn't exist.
6. `uio_mountDir(repository, "/", ..., configDirName, ...)` — mounted into the **same** repository but as a separate mount.
7. `configDir = uio_openDir(repository, "/", 0)` — opens the root.

**Critical detail**: `configDir` and `contentDir` share the **same UIO repository** but are opened as separate `uio_DirHandle`s. When `LoadResourceIndex(configDir, "uqm.cfg", "config.")` is called, the file is read from the config directory. When `LoadResourceIndex(contentDir, "uqm.rmp", NULL)` is called, the file is read from the content directory. The `uio_DirHandle` parameter tells UIO **which mount context** to resolve the path in.

### 3.4 Paths Inside .rmp Files

Resource paths in `.rmp` files are **relative to the content root**:
```
comm.arilou.graphics = GFXRES:base/comm/arilou/arilou.ani
```
The path `base/comm/arilou/arilou.ani` is passed to `res_OpenResFile(contentDir, path, "rb")`. UIO resolves it relative to the content mount.

For addons:
```
music.battle = MUSICRES:addons/3domusic/battle.ogg
```
The path includes the `addons/3domusic/` prefix. Since addons are mounted under `/addons/` in the UIO tree, and `contentDir` sees the entire merged tree, these paths resolve correctly.

### 3.5 Rust Path Handling — Where It Diverges

The Rust `ResourceLoader` uses:
```rust
let full_path = self.base_path.join(file_path);
```
This is **standard filesystem** path joining — no UIO layer. This means:
- [ERROR] No support for `.zip` package mounts
- [ERROR] No case-insensitive resolution on Linux
- [ERROR] No layered addon override behavior
- [ERROR] No virtual filesystem merging
- WARNING: `base_path` must be resolved to the actual filesystem path, not a UIO handle

The `resolve_path()` method does canonicalization and path traversal protection, which is good, but it operates on the raw filesystem, bypassing UIO entirely.

### 3.6 Addon Path Resolution

Addons use **key override** semantics, not path override:
1. Base `uqm.rmp` defines `music.battle = MUSICRES:base/lander/battle.mod`
2. Addon `3domusic.rmp` defines `music.battle = MUSICRES:addons/3domusic/battle.ogg`
3. Since addon indices load **after** the base index, the addon entry replaces the base entry in the hash table
4. Subsequent `res_GetResource("music.battle")` loads from the addon path

This is purely a hash table override — no path magic involved. The Rust `ResourceIndex::merge()` supports this pattern correctly.

### 3.7 Case Sensitivity

- **C/UIO**: Case-insensitive on all platforms (UIO normalizes)
- **C hash table**: Uses `CharHashTable` which is **case-sensitive** for resource keys
- **Rust ResourceIndex**: Case-insensitive by default (lowercases keys)
- **Rust PropertyFile**: Case-insensitive (uppercases keys)

**Mismatch**: The C resource keys are case-sensitive (`comm.arilou.graphics` ≠ `COMM.ARILOU.GRAPHICS`), but the Rust `ResourceIndex` lowercases them and the Rust `PropertyFile` uppercases them. This could cause lookup failures when C code queries the Rust index by the original-case key.

---

## 4. Approach Options

### 4.1 Option A: Incremental — Rust Cache Alongside C Loader

**Description**: C remains the authority for index loading and resource resolution. Rust provides an LRU cache layer. After C loads a resource via `res_GetResource()`, the raw bytes are inserted into the Rust cache. Before C loads, the cache is checked.

**Feasibility**: HIGH. This is essentially what `rust_resource.c` was designed for. The changes are:
1. Wire `RustResourceInit()` into `InitResourceSystem()`
2. Add cache check at the top of `res_GetResource()`
3. Add cache insert after successful `loadResourceDesc()`

**Pros**:
- Minimal risk — C resource pipeline unchanged
- Cache is purely additive
- Easy to feature-flag with `USE_RUST_RESOURCE`
- Does not require Rust to understand .rmp format or type-specific loaders

**Cons**:
- Rust is just a dumb byte cache — no understanding of resource semantics
- Cache coherence: if C modifies a resource in-place (via `res_PutString` etc.), Rust cache is stale
- Two memory allocators managing the same data
- Does not advance the goal of replacing C resource code

**What's missing**: The cache-get in `rust_resource.c` returns a **copy** (`libc::malloc` + `copy_nonoverlapping`). C expects to get back its own `resdata.ptr`, not a copy. The bridge needs rethinking for heap resources — you can't just hand C a Rust-allocated copy and expect `freeFun` to work on it.

### 4.2 Option B: Parallel Read Path — Rust Reads .rmp Files

**Description**: Rust handles index parsing and the key-value config API (`res_GetString`, `res_GetInteger`, etc.). C retains all type-specific loaders (`_GetCelData`, `_GetSoundBankData`, etc.).

**Feasibility**: MEDIUM. The key-value config API is self-contained (85+ call sites but all go through a small number of functions) and doesn't involve UIO. However:

1. Rust must correctly parse `TYPE:path` format from `.rmp` files (currently broken)
2. Rust must handle the prefix mechanism (`LoadResourceIndex(dir, "uqm.cfg", "config.")`)
3. Rust must support `Put` operations and `SaveResourceIndex()`
4. Rust must support `res_Remove()` for overriding entries
5. Rust must integrate with the UIO filesystem for actually reading `.rmp`/`.cfg`/`.key` files, OR the C side must read the files and pass the content to Rust as strings

**Pros**:
- Replaces the most-called C resource functions (85+ config API call sites)
- Key-value API is well-defined and testable
- No dependency on type-specific loaders
- Enables Rust-side config validation

**Cons**:
- Two hash tables (C and Rust) must stay synchronized
- `res_GetResource()` for heap types still goes through C — can't eliminate C resource system
- UIO integration remains a problem

### 4.3 Option C: Full Replacement

**Description**: Replace the entire C resource pipeline with Rust, including index parsing, type dispatch, and file loading. Type-specific loaders remain in C but are called from Rust.

**Feasibility**: LOW in the near term. This requires:
1. Rust UIO integration or replacement
2. Rust type-specific loader dispatch (calling C `_GetCelData` etc. from Rust FFI — reversed direction)
3. All 21+ `res_GetResource` call sites, 85+ config API call sites, and all `LoadResourceIndex` call sites updated
4. `SaveResourceIndex()` reimplemented
5. All refcount semantics preserved
6. `res_DetachResource()` semantics preserved

**Pros**:
- Clean Rust ownership of the resource pipeline
- Enables Rust-native resource types in the future
- Eliminates C resource code entirely

**Cons**:
- Extremely high risk on a critical startup path
- Reverse-FFI (Rust calling C loaders) is complex
- UIO dependency is deep — ~30 call sites use `contentDir`/`configDir`

### 4.4 Recommended Phased Approach

**Phase 1: Fix Rust Index Parser + Wire Cache (Option A)**
- Fix `ResourceIndex::parse()` to handle `TYPE:path` format
- Fix `PropertyFile` key casing to match C behavior (case-preserving)
- Wire `RustResourceInit/Uninit` into C lifecycle
- Enable `USE_RUST_RESOURCE` in build
- Cache raw bytes only (no semantic understanding)
- Scope: ~200 LoC changes

**Phase 2: Rust Key-Value Config API (Option B subset)**
- Implement `res_GetString`, `res_GetInteger`, `res_GetBoolean`, `res_GetColor` in Rust
- Implement `res_PutString`, `res_PutInteger`, `res_PutBoolean`, `res_PutColor` in Rust
- Implement `res_HasKey`, `res_IsString`, `res_IsInteger`, `res_IsBoolean`, `res_IsColor` in Rust
- Implement `SaveResourceIndex()` in Rust
- Wire into C via `#ifdef USE_RUST_RESOURCE` replacement of the functions in `resinit.c:466-651`
- C reads files and passes content strings; Rust parses and stores
- Scope: ~600-800 LoC

**Phase 3: Rust Index Ownership (Option B full)**
- Rust owns the hash table; C calls into Rust for all `lookupResourceDesc` operations
- `LoadResourceIndex()` passes file content to Rust for parsing
- Type-specific loaders still in C, called via C function pointers
- Scope: ~500-700 LoC

**Phase 4: Full Replacement (Option C, deferred)**
- Only after UIO replacement/integration is complete
- Only after type-specific loaders are individually migrated
- Likely happens organically as subsystems move to Rust

---

## 5. Test Coverage Plan

### 5.1 Existing Rust Tests

**`resource/tests.rs`** contains integration tests organized by module:

| Test Suite | Count | Coverage |
|------------|-------|----------|
| `propfile_tests` | 15 | Parsing, case-insensitivity, comments, merge, save/load roundtrip |
| `stringbank_tests` | 10 | Load, fallback chain, formatted strings, multi-language |
| `resource_system_tests` | — | (in resource_system.rs: 8 tests) String/int/bool/color get, caching, release, enable/disable |
| `resource_index_tests` | — | (in index.rs: 8 tests) Parse, lookup, case sensitivity, merge |
| `cache_tests` | — | (in cache.rs: 11 tests) Insert/get, eviction, pinning, thread safety |
| `loader_tests` | — | (in loader.rs: 9 tests) Load, path traversal, string load, subdirectories |
| `resource_type_tests` | — | (in resource_type.rs: 13 tests) Type parsing, color hex, value conversions |

**Total**: ~74 tests across all resource modules.

### 5.2 Critical Gap: No Tests Against Real .rmp Files

None of the existing tests load an actual UQM `.rmp` file. All tests use synthetic data. This means the **format mismatch** (Section 2.1) has never been caught by tests.

### 5.3 Required Tests Before Phase 1

#### A. .rmp Format Compatibility Tests

```
test_parse_real_uqm_rmp
    Load sc2/content/uqm.rmp
    Verify 963 entries parsed
    Verify TYPE is correctly extracted (GFXRES, MUSICRES, etc.)
    Verify path is correctly extracted (base/comm/arilou/arilou.ani)
    Verify key is preserved with original casing

test_parse_real_addon_rmp
    Load sc2/content/addons/3domusic/3domusic.rmp
    Verify 13 entries parsed
    Verify addon paths correct (addons/3domusic/battle.ogg)

test_parse_key_config_file
    Load sc2/content/uqm.key
    Verify STRING type entries parsed
    Verify prefix mechanism ("keys." prefix)

test_parse_menu_key_file
    Load sc2/content/menu.key
    Verify STRING type entries parsed
    Verify "menu." prefix application
```

#### B. Index Override/Merge Tests

```
test_addon_overrides_base
    Load uqm.rmp, then 3domusic.rmp
    Verify music.battle points to addon path, not base path

test_config_override
    Create base index, then apply config prefix override
    Verify prefixed key takes precedence

test_multiple_rmp_loading_order
    Load multiple .rmp files
    Verify last-writer-wins semantics
```

#### C. Key Case Sensitivity Tests

```
test_c_compatible_case_sensitivity
    Insert "comm.arilou.graphics" 
    Lookup by exact case → found
    Lookup by different case → behavior must match C (case-sensitive)

test_property_file_preserves_key_case
    Parse file with mixed-case keys
    Verify stored keys preserve original case
```

#### D. Color Parsing Compatibility Tests

```
test_parse_c_rgb_format
    Parse "rgb(255, 128, 64)" → (255, 128, 64, 255)
    Parse "rgba(255, 128, 64, 128)" → (255, 128, 64, 128)
    Parse "rgb15(31, 16, 8)" → CC5TO8 conversion
    Parse "rgb(0x1a, 0x00, 0x1a)" → hex component values
```

#### E. Path Resolution Tests

```
test_content_relative_paths
    Given base_path=/path/to/content
    Verify "base/comm/arilou/arilou.ani" → /path/to/content/base/comm/arilou/arilou.ani

test_addon_relative_paths
    Verify "addons/3domusic/battle.ogg" → /path/to/content/addons/3domusic/battle.ogg

test_path_traversal_rejected
    Verify "../../../etc/passwd" is rejected
    Verify "base/../../etc/passwd" is rejected

test_case_insensitive_path_resolution (platform-specific)
    On macOS (case-insensitive FS): verify "Base/Comm/Arilou.ani" resolves
    On Linux (case-sensitive FS): document expected behavior
```

#### F. Cache Integration Tests

```
test_cache_insert_and_retrieve
    Insert bytes, retrieve, verify identical

test_cache_lru_eviction
    Fill cache to limit, verify oldest unpinned entry evicted

test_cache_does_not_evict_pinned
    Pin an entry, fill cache, verify pinned entry survives

test_cache_key_matches_resource_key
    Verify cache keys use same casing/format as resource index keys
```

### 5.4 Required Tests Before Phase 2

#### G. Key-Value Config API Compatibility

```
test_res_get_string_matches_c
    Load uqm.key via Rust parser
    Verify res_GetString("keys.1.name") returns "Arrows"

test_res_get_integer_matches_c
    Load uqm.cfg with INT32 entries
    Verify res_GetInteger("config.gamma") returns correct int

test_res_get_boolean_matches_c
    Verify "true" → TRUE, "false" → FALSE
    Verify case-insensitive ("True", "TRUE", "true" all work)

test_res_put_and_get_roundtrip
    Put a string, get it back, verify identical
    Put an integer, get it back, verify identical

test_res_has_key
    Verify existing key → true
    Verify missing key → false

test_res_is_type_checking
    Verify res_IsString returns true for STRING type, false for INT32

test_save_resource_index_roundtrip
    Load index, modify values via Put, save, reload, verify
```

#### H. Prefix Mechanism Tests

```
test_prefix_prepended_to_keys
    Load with prefix "config."
    Verify "fullscreen" becomes "config.fullscreen"

test_null_prefix_passes_through
    Load with NULL prefix
    Verify keys unchanged

test_prefix_with_override
    Load base without prefix, load override with prefix
    Verify override takes precedence for prefixed key
```

---

## 6. Risk Assessment

### 6.1 Critical Startup Path

Resource loading is one of the first things that happens after `main()`. `InitResourceSystem()` is called from `uqm/setup.c:109`. If it fails or returns incorrect data, the game will crash immediately or exhibit subtle corruption. **Any Rust integration must be 100% transparent** — the C code should see identical behavior whether Rust is involved or not.

### 6.2 Type-Specific Loader Coupling

The type-specific loaders (`_GetCelData`, `_GetSoundBankData`, etc.) are deeply intertwined with their respective subsystems. They don't return raw bytes — they return parsed, subsystem-specific structures (`DRAWABLE`, `FONT`, `SOUND_REF`, `MUSIC_REF`, `STRING_TABLE`). Any Rust replacement must either:
- Call these C loaders via FFI (complex, requires passing `uio_Stream` across the boundary)
- Reimplement the binary format parsers in Rust (massive scope, each format is different)

**Recommendation**: Do not attempt to replace type-specific loaders as part of the resource system migration. They will be replaced individually as their subsystems move to Rust.

### 6.3 UIO Dependency

The UIO virtual filesystem is pervasive — **79+ call sites** reference `contentDir` or `configDir`. Rust's `std::fs` cannot replicate UIO's behavior (layered mounts, case-insensitive matching, transparent zip decompression). Options:
- **Short term**: Have C read files through UIO and pass content to Rust as byte slices
- **Medium term**: Create Rust FFI wrappers for UIO functions
- **Long term**: Replace UIO entirely (separate project)

### 6.4 Case Sensitivity

- macOS: HFS+/APFS are case-insensitive by default → works accidentally
- Linux: ext4 is case-sensitive → UIO normalizes, but raw `std::fs` will fail on mismatched case
- Resource keys: C hash table is case-sensitive; Rust `ResourceIndex` lowercases keys; Rust `PropertyFile` uppercases keys

**This is a bug waiting to happen**. Before any integration, the Rust code must match C's case behavior exactly:
- Resource keys: **case-sensitive** (preserve original case)
- File paths: delegate to UIO (or implement case-insensitive fallback for `std::fs`)

### 6.5 Memory Ownership

The C resource system uses `HMalloc`/`HFree` for all allocations. Rust uses its own allocator. Data crossing the FFI boundary must have clear ownership:
- If Rust allocates data returned to C, C must call a Rust-side free function
- If C allocates data passed to Rust, Rust must not attempt to free it
- The existing `rust_resource.c` bridge correctly uses `libc::malloc` for data returned to C, which is compatible with C's `free()`, but NOT with `HFree()` (which may be a wrapper)

### 6.6 Thread Safety

The C resource system uses a **single global** `curResourceIndex` with no locking. It is effectively single-threaded. The Rust system uses `Mutex<Option<ResourceSystem>>` and `OnceLock<ResourceCache>` — properly thread-safe. This mismatch is not a problem in practice (game is effectively single-threaded for resource operations) but the Rust locking adds overhead. The `Mutex` in the FFI hot path (`rust_get_string_resource` etc.) could become a bottleneck if called frequently.

### 6.7 Addon Resolution Ordering

Addons are loaded in a specific order determined by directory listing + command-line `--addon` order. If Rust index loading changes the order or timing of `.rmp` processing, addon overrides may apply incorrectly. The last-writer-wins semantics must be preserved exactly.

### 6.8 SaveResourceIndex Fidelity

`SaveResourceIndex()` writes the hash table back to files using the `toString` function pointer for each type. The Rust system must produce **byte-identical** output (or at least semantically identical — key order may differ since hash table iteration is unordered). Key ordering differences could cause unnecessary diff noise in config files.

### 6.9 The STRING Lifetime Problem

The C code has a documented TODO: "Work out exact STRING semantics, specifically, the lifetime of the returned value." `res_GetString()` returns a `const char *` pointing directly into the `ResourceDesc.fname` field. If the resource is replaced (via `res_PutString` or `res_Remove`), the pointer becomes dangling. The Rust system, by returning owned `String`s via FFI (`CString::into_raw()`), actually fixes this bug — but callers must be updated to call `rust_free_string()`. This is a subtle but important migration concern.
