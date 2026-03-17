# Resource subsystem initial state

## Scope and purpose

The resource subsystem is the engine’s typed resource index and dispatch layer. It is responsible for:

- initializing the global resource index and type registry (`InitResourceSystem`, `InstallResTypeVectors`) so subsystems can register loaders for typed resources such as graphics, sound, strings, video, and code (`/Users/acoliver/projects/uqm/sc2/src/libs/reslib.h:67-78`, `/Users/acoliver/projects/uqm/sc2/src/libs/resource/resinit.c:335-362`, `/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:219-277`)
- loading `.rmp`/config-style index files into the live resource map (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/resinit.c:370-373`, `/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:299-367`)
- resolving resource IDs to typed loader callbacks and managing lazy load / refcount / detach / free behavior (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/getres.c:106-205`, `/Users/acoliver/projects/uqm/rust/src/resource/dispatch.rs:129-324`, `/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:503-577`)
- exposing config-style typed key/value accessors (`res_GetString`, `res_PutInteger`, `res_Remove`, etc.) used by startup, input binding, and options code (`/Users/acoliver/projects/uqm/sc2/src/libs/reslib.h:100-118`, `/Users/acoliver/projects/uqm/sc2/src/uqm.c:358-363`, `/Users/acoliver/projects/uqm/sc2/src/libs/input/sdl/input.c:86-108`, `/Users/acoliver/projects/uqm/sc2/src/libs/input/sdl/input.c:176-198`)
- mediating file loading through UIO helpers such as `res_OpenResFile`, `LoadResourceFromPath`, and `GetResourceData` (`/Users/acoliver/projects/uqm/sc2/src/libs/reslib.h:53-66`, `/Users/acoliver/projects/uqm/sc2/src/libs/resource/filecntl.c:32-146`, `/Users/acoliver/projects/uqm/sc2/src/libs/resource/getres.c:38-72`, `/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:980-1224`)

At runtime, startup still treats this as a core engine service: `InitResourceSystem()` is called during setup, resource indices are loaded from config/content directories, and `UninitResourceSystem()` is called during kernel teardown (`/Users/acoliver/projects/uqm/sc2/src/uqm/setup.c:109-115`, `/Users/acoliver/projects/uqm/sc2/src/uqm.c:358-363`, `/Users/acoliver/projects/uqm/sc2/src/uqm/cleanup.c:35-48`, `/Users/acoliver/projects/uqm/sc2/src/options.c:490-515`).

## Current C structure

### Public ABI and data model

The public C API is still declared in `reslib.h`. Important exported concepts are:

- `RESOURCE` is a `const char *` key; `RESOURCE_DATA` is a union of `num`, `ptr`, and `str` (`/Users/acoliver/projects/uqm/sc2/src/libs/reslib.h:31-39`)
- `_cur_resfile_name` is a global load-time filename guard visible to other subsystems (`/Users/acoliver/projects/uqm/sc2/src/libs/reslib.h:43`, `/Users/acoliver/projects/uqm/sc2/src/libs/resource/getres.c:28-31`)
- resource type handlers are function pointers: `ResourceLoadFun`, `ResourceFreeFun`, `ResourceStringFun`, and `ResourceLoadFileFun` (`/Users/acoliver/projects/uqm/sc2/src/libs/reslib.h:45-53`)
- the public ABI includes lifecycle, dispatch, config-get/put, and file-I/O entry points (`/Users/acoliver/projects/uqm/sc2/src/libs/reslib.h:67-118`)

The C-side in-memory structures are in `index.h`:

- `ResourceHandlers` stores `resType`, `loadFun`, `freeFun`, and `toString` (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/index.h:27-33`)
- `ResourceDesc` stores resource id, path string, vtable pointer, loaded data union, and a refcount (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/index.h:35-43`)
- `RESOURCE_INDEX_DESC` owns the resource hash table (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/index.h:45-49`)

### C implementation files and responsibilities

When the Rust replacement is **not** enabled, the C resource subsystem is implemented across five guarded source files:

- `resinit.c` — type registration, index load/save, config get/put/remove, global index ownership (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/resinit.c:34-655`)
- `getres.c` — lazy load, free, detach, resource-type queries, file-backed load helper (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/getres.c:26-261`)
- `filecntl.c` — UIO-backed file wrappers (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/filecntl.c:30-148`)
- `propfile.c` — property-file parser for `.rmp`/cfg-like files (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/propfile.c:26-133`)
- `loadres.c` — raw binary resource loading with legacy prefix handling (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/loadres.c:23-57`)

Two related support files remain entirely C-owned regardless of the guard in the code shown:

- `stringbank.c` — chunked string pool helper (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/stringbank.c:1-151`)
- `direct.c` — directory scan to string-table adapter (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/direct.c:23-95`)

### C type registration and downstream ownership

The classic C initializer registers five built-in value types plus downstream subsystem types:

- built-in value types: `UNKNOWNRES`, `STRING`, `INT32`, `BOOLEAN`, `COLOR` (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/resinit.c:345-350`)
- graphics: `GFXRES`, `FONTRES` (`/Users/acoliver/projects/uqm/sc2/src/libs/graphics/resgfx.c:34-39`)
- string tables: `STRTAB`, `BINTAB`, `CONVERSATION` (`/Users/acoliver/projects/uqm/sc2/src/libs/strings/sresins.c:33-39`)
- audio: `SNDRES`, `MUSICRES` (`/Users/acoliver/projects/uqm/sc2/src/libs/sound/resinst.c:34-39`)
- video: `3DOVID` (`/Users/acoliver/projects/uqm/sc2/src/libs/video/vresins.c:166-170`)
- code resources: `SHIP` (`/Users/acoliver/projects/uqm/sc2/src/uqm/dummy.c:154-159`)

These downstream subsystem files still define the actual resource-specific loaders and call patterns. For example, graphics/audio/string/video loaders still call `LoadResourceFromPath`, and instance constructors still call `res_GetResource` followed by `res_DetachResource` (`/Users/acoliver/projects/uqm/sc2/src/libs/graphics/resgfx.c:21-30`, `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/resgfx.c:42-52`, `/Users/acoliver/projects/uqm/sc2/src/libs/sound/resinst.c:22-31`, `/Users/acoliver/projects/uqm/sc2/src/libs/sound/resinst.c:42-63`, `/Users/acoliver/projects/uqm/sc2/src/libs/strings/sresins.c:21-30`, `/Users/acoliver/projects/uqm/sc2/src/libs/strings/sresins.c:42-53`, `/Users/acoliver/projects/uqm/sc2/src/libs/video/vresins.c:173-184`).

## Current Rust structure

### Rust module surface

The Rust crate exports a dedicated `resource` module from the main library (`/Users/acoliver/projects/uqm/rust/src/lib.rs:7-19`). The resource module currently contains fifteen source files (`/Users/acoliver/projects/uqm/rust/src/resource/mod.rs:4-22`):

- `ffi_bridge.rs` — C ABI replacement for the resource subsystem
- `dispatch.rs` — lazy-load / refcount / detach / remove dispatch logic
- `type_registry.rs` and `ffi_types.rs` — ABI-compatible type registration and handler storage
- `propfile.rs`, `index.rs`, `resource_type.rs` — parser/value helpers
- `config_api.rs` — typed config serialization helpers
- `loader.rs`, `cache.rs`, `ffi.rs`, `resource_system.rs` — a separate Rust-native loader/cache/resource-system surface
- `stringbank.rs`, `tests.rs`

### Rust bridge that currently matters for engine integration

The code actually shaped as a C replacement is `ffi_bridge.rs`.

It defines:

- UIO imports and `contentDir` as external C dependencies (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:28-49`)
- optional Rust-audio registration shims behind `feature = "audio_heart"` (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:51-86`)
- a global `ResourceState` holding `ResourceDispatch`, string/type caches, and a `c_types_registered` guard (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:133-145`)
- a Rust-exported `_cur_resfile_name` global to preserve the C ABI (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:146-153`)
- auto-init and built-in type installation (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:159-205`)
- 41 `#[no_mangle]` exports, including lifecycle, index load/save, resource access, config accessors, file wrappers, and resource-data helpers (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:148-150`, `/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:222-1224`)

The central Rust dispatch state is `ResourceDispatch`, which stores resource entries, the type registry, and a current-resource-file marker (`/Users/acoliver/projects/uqm/rust/src/resource/dispatch.rs:31-48`). It implements:

- descriptor parsing and handler selection (`/Users/acoliver/projects/uqm/rust/src/resource/dispatch.rs:61-126`)
- lazy `get_resource()` with refcount increment (`/Users/acoliver/projects/uqm/rust/src/resource/dispatch.rs:129-182`)
- `free_resource()` with zero-ref cleanup (`/Users/acoliver/projects/uqm/rust/src/resource/dispatch.rs:184-236`)
- `detach_resource()` and `remove_resource()` (`/Users/acoliver/projects/uqm/rust/src/resource/dispatch.rs:238-324`)

The Rust ABI/layout mirror is explicit in `ffi_types.rs`, where `ResourceData` is a `#[repr(C)]` union matching C `RESOURCE_DATA`, and function pointer typedefs mirror the C signatures (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_types.rs:11-80`).

`type_registry.rs` preserves the C convention that resource handler registrations live under `sys.<type>` keys and provides Rust implementations for built-in value loaders/serializers (`/Users/acoliver/projects/uqm/rust/src/resource/type_registry.rs:18-66`, `/Users/acoliver/projects/uqm/rust/src/resource/type_registry.rs:83-274`).

`propfile.rs` contains the parser actually used by `ffi_bridge.rs` through `parse_propfile()` (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:21`, `/Users/acoliver/projects/uqm/rust/src/resource/propfile.rs:15-116`).

### Rust code that exists but is not the active subsystem replacement boundary

A second Rust resource stack also exists:

- `ffi.rs` exports `rust_init_resource_system`, `rust_load_index`, `rust_resource_loader_init`, `rust_resource_load`, cache APIs, and related tests (`/Users/acoliver/projects/uqm/rust/src/resource/ffi.rs:17-34`, `/Users/acoliver/projects/uqm/rust/src/resource/ffi.rs:36-555`)
- `resource_system.rs` defines a separate `ResourceSystem` abstraction using `PropertyFile`, `ResourceType`, direct filesystem `PathBuf`s, and an internal cache of `Arc<ResourceValue>` (`/Users/acoliver/projects/uqm/rust/src/resource/resource_system.rs:11-258`)
- `index.rs` and `config_api.rs` provide alternate Rust-native representations and helpers (`/Users/acoliver/projects/uqm/rust/src/resource/index.rs:53-126`, `/Users/acoliver/projects/uqm/rust/src/resource/config_api.rs:14-211`)

Evidence in the codebase does **not** show C calling `rust_init_resource_system`, `rust_load_index`, or `rust_resource_loader_init`; the hits are declarations in `rust_resource.h` and Rust-side tests/definitions only (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/rust_resource.h:21-30`, `/Users/acoliver/projects/uqm/rust/src/resource/ffi.rs:36-63`, `/Users/acoliver/projects/uqm/rust/src/resource/ffi.rs:226-555`, `/Users/acoliver/projects/uqm/rust/src/resource/tests.rs:900-957`).

That means the active hybrid boundary is the `ffi_bridge.rs` C-ABI replacement, not the `ffi.rs`/`resource_system.rs` loader/cache API.

## Build and configuration wiring

### Compile-time switch

The resource takeover is gated by `USE_RUST_RESOURCE` in the generated config header template (`/Users/acoliver/projects/uqm/sc2/src/config_unix.h.in:95-100`). In the checked-in generated configuration used by search results, `config_unix.h` defines `USE_RUST_RESOURCE` (`/Users/acoliver/projects/uqm/sc2/config_unix.h:99` as found by repository search).

### C compile-time split boundary

The key partial-port boundary is explicit in the C source files:

- `resinit.c` C implementation exists only under `#ifndef USE_RUST_RESOURCE` (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/resinit.c:34`, `/Users/acoliver/projects/uqm/sc2/src/libs/resource/resinit.c:655`)
- `getres.c` only under `#ifndef USE_RUST_RESOURCE` (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/getres.c:26`, `/Users/acoliver/projects/uqm/sc2/src/libs/resource/getres.c:261`)
- `filecntl.c` only under `#ifndef USE_RUST_RESOURCE` (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/filecntl.c:30`, `/Users/acoliver/projects/uqm/sc2/src/libs/resource/filecntl.c:148`)
- `propfile.c` only under `#ifndef USE_RUST_RESOURCE` (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/propfile.c:26`, `/Users/acoliver/projects/uqm/sc2/src/libs/resource/propfile.c:133`)
- `loadres.c` only under `#ifndef USE_RUST_RESOURCE` (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/loadres.c:23`, `/Users/acoliver/projects/uqm/sc2/src/libs/resource/loadres.c:57`)

So when `USE_RUST_RESOURCE` is on, these classic C implementations are compiled out, and the Rust- exported symbols must satisfy the `reslib.h` ABI.

### Rust build wiring

The Rust crate builds as both `staticlib` and `rlib`, which is consistent with being linked into the C engine (`/Users/acoliver/projects/uqm/rust/Cargo.toml:5-8`). The crate has an `audio_heart` feature that changes resource-type registration behavior inside `ffi_bridge.rs` (`/Users/acoliver/projects/uqm/rust/Cargo.toml:31-33`, `/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:45-54`, `/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:250-269`).

## C↔Rust integration points

### Lifecycle handoff

C startup and teardown continue to call the public resource ABI names:

- startup: `InitResourceSystem()` in setup (`/Users/acoliver/projects/uqm/sc2/src/uqm/setup.c:109-111`)
- config load before full setup: `LoadResourceIndex(configDir, "uqm.cfg", "config.")` (`/Users/acoliver/projects/uqm/sc2/src/uqm.c:358-363`)
- addon/content index enumeration: `loadIndices()` calling `LoadResourceIndex()` for every `.rmp` (`/Users/acoliver/projects/uqm/sc2/src/options.c:490-515`)
- teardown: `UninitResourceSystem()` (`/Users/acoliver/projects/uqm/sc2/src/uqm/cleanup.c:35-40`)

With `USE_RUST_RESOURCE`, these calls bind to Rust exports in `ffi_bridge.rs` rather than the old C definitions.

### Type registration remains C-driven at subsystem edges

Even under the Rust replacement, the downstream C subsystems still own their type-specific loaders and register them through the public API:

- Rust `InitResourceSystem()` calls `InstallGraphicResTypes`, `InstallStringTableResType`, `InstallAudioResTypes` or feature-gated Rust audio replacements, `InstallVideoResType`, and `InstallCodeResType` (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:233-272`)
- those C functions in turn call the public `InstallResTypeVectors` ABI (`/Users/acoliver/projects/uqm/sc2/src/libs/graphics/resgfx.c:34-39`, `/Users/acoliver/projects/uqm/sc2/src/libs/strings/sresins.c:33-39`, `/Users/acoliver/projects/uqm/sc2/src/libs/sound/resinst.c:34-39`, `/Users/acoliver/projects/uqm/sc2/src/libs/video/vresins.c:166-170`, `/Users/acoliver/projects/uqm/sc2/src/uqm/dummy.c:154-159`)
- Rust receives those registrations in `ffi_bridge.rs:470-497` and stores them in `TypeRegistry`

This is a major hybrid boundary: Rust owns the registry and dispatch table, but C still supplies most non-value loader/free implementations.

### File loading and global environment remain C-owned dependencies

Rust resource ABI code is still coupled to C UIO/global state:

- UIO function imports and `contentDir` are external C symbols (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:28-49`)
- `LoadResourceIndex` uses `uio_fopen/uio_fread/uio_fclose` directly (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:319-340`)
- `res_OpenResFile`, `ReadResFile`, `WriteResFile`, `SeekResFile`, `TellResFile`, `LengthResFile`, and `DeleteResFile` are thin wrappers over UIO (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:986-1113`)
- `LoadResourceFromPath` opens via `contentDir`, sets `_cur_resfile_name`, calls the downstream load callback, clears the guard, and closes the file (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:1124-1158`)

This preserves the old integration contract rather than replacing it.

### C consumers still call the classic resource ABI

C callers are unchanged and still consume the old API names:

- input config logic uses `LoadResourceIndex`, `res_IsString`, `res_GetString`, `res_Remove`, and `res_PutString` (`/Users/acoliver/projects/uqm/sc2/src/libs/input/sdl/input.c:86-108`, `/Users/acoliver/projects/uqm/sc2/src/libs/input/sdl/input.c:176-198`, `/Users/acoliver/projects/uqm/sc2/src/libs/input/sdl/input.c:528-576`)
- graphics/audio/string/video/code instance loaders call `res_GetResource` and `res_DetachResource` (`/Users/acoliver/projects/uqm/sc2/src/libs/graphics/resgfx.c:42-52`, `/Users/acoliver/projects/uqm/sc2/src/libs/sound/resinst.c:42-63`, `/Users/acoliver/projects/uqm/sc2/src/libs/strings/sresins.c:42-53`, `/Users/acoliver/projects/uqm/sc2/src/libs/video/vresins.c:173-184`, `/Users/acoliver/projects/uqm/sc2/src/uqm/dummy.c:162-171`)

## What is already ported

### Core resource ABI replacement is ported to Rust

The following core C implementation areas have Rust counterparts and are behind the `USE_RUST_RESOURCE` split:

- lifecycle and built-in type install: `InitResourceSystem`, `UninitResourceSystem` (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:219-293`)
- index load/save: `LoadResourceIndex`, `SaveResourceIndex` (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:299-461`)
- type registration: `InstallResTypeVectors`, `CountResourceTypes` (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:467-497`, `/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:662-673`)
- dispatch: `res_GetResource`, `res_DetachResource`, `res_FreeResource`, `res_Remove` (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:503-577`)
- value/config accessors: `res_HasKey`, `res_Is*`, `res_Get*`, `res_Put*` (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:679-978`)
- file and raw-data helpers: `res_OpenResFile`, `res_CloseResFile`, `ReadResFile`, `WriteResFile`, `GetResFileChar`, `PutResFileChar`, `PutResFileNewline`, `SeekResFile`, `TellResFile`, `LengthResFile`, `DeleteResFile`, `LoadResourceFromPath`, `GetResourceData`, `FreeResourceData` (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:980-1224`)

### Built-in value parsing/serialization is ported

Rust has direct replacements for:

- string/int/bool/color built-in loader semantics (`/Users/acoliver/projects/uqm/rust/src/resource/type_registry.rs:83-171`)
- string/int/bool/color serialization (`/Users/acoliver/projects/uqm/rust/src/resource/type_registry.rs:177-274`)
- property-file parsing used for resource indices (`/Users/acoliver/projects/uqm/rust/src/resource/propfile.rs:15-116`)

### Test coverage exists for the Rust module

The resource Rust code has extensive unit tests across modules, including dispatch and FFI surfaces (`/Users/acoliver/projects/uqm/rust/src/resource/dispatch.rs:356-520`, `/Users/acoliver/projects/uqm/rust/src/resource/ffi.rs:483-555`, `/Users/acoliver/projects/uqm/rust/src/resource/tests.rs:900-957`).

## What remains C-owned or hybrid

### Type-specific loaders are still mostly C authority

The Rust resource core does not own most concrete resource decoding/loading behaviors. Those remain in C subsystem code and are plugged into Rust through function-pointer registration:

- graphics loads still enter C `_GetCelData` / `_GetFontData` via C callbacks (`/Users/acoliver/projects/uqm/sc2/src/libs/graphics/resgfx.c:21-30`)
- string/binary/conversation tables still enter C `_GetStringData`, `_GetBinaryTableData`, `_GetConversationData` (`/Users/acoliver/projects/uqm/sc2/src/libs/strings/sresins.c:21-38`)
- video still enters C `GetLegacyVideoData` and `FreeLegacyVideoData` (`/Users/acoliver/projects/uqm/sc2/src/libs/video/vresins.c:166-184`)
- code resources still register through `InstallCodeResType()` in C (`/Users/acoliver/projects/uqm/sc2/src/uqm/dummy.c:154-171`)
- audio remains C-owned by default through `InstallAudioResTypes()` unless the Rust crate is built with `audio_heart` feature (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:245-269`, `/Users/acoliver/projects/uqm/sc2/src/libs/sound/resinst.c:34-39`)

### UIO and global directory state are external C dependencies

The Rust bridge does not own filesystem/mount behavior. It imports C UIO and `contentDir` and depends on those symbols being initialized elsewhere (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:28-49`).

### Support utilities remain C-owned

`stringbank.c` and `direct.c` remain C implementations, and no code evidence shows Rust replacing their engine-facing role (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/stringbank.c:1-151`, `/Users/acoliver/projects/uqm/sc2/src/libs/resource/direct.c:23-95`).

### An older/additional Rust loader/cache path is present but not integrated

`rust_resource.c` and `rust_resource.h` describe a Rust-backed cache/loader sidecar (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/rust_resource.c:1-127`, `/Users/acoliver/projects/uqm/sc2/src/libs/resource/rust_resource.h:1-52`), and `ffi.rs` exports matching functions (`/Users/acoliver/projects/uqm/rust/src/resource/ffi.rs:203-555`). But repository search found declarations and tests, not production call sites, for `rust_init_resource_system`, `rust_load_index`, or `rust_resource_loader_init` (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/rust_resource.h:21-30`, `/Users/acoliver/projects/uqm/rust/src/resource/ffi.rs:36-63`, `/Users/acoliver/projects/uqm/rust/src/resource/ffi.rs:226-555`).

So this path currently looks auxiliary or stale relative to the `ffi_bridge.rs` takeover, not the active authority.

## Partial-port boundaries and guard evidence

### Boundary 1: C core implementation is compiled out, Rust ABI takes over

This is the primary port boundary:

- C core files are guarded out by `#ifndef USE_RUST_RESOURCE` in `resinit.c`, `getres.c`, `filecntl.c`, `propfile.c`, and `loadres.c` (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/resinit.c:34-34`, `/Users/acoliver/projects/uqm/sc2/src/libs/resource/getres.c:26-26`, `/Users/acoliver/projects/uqm/sc2/src/libs/resource/filecntl.c:30-30`, `/Users/acoliver/projects/uqm/sc2/src/libs/resource/propfile.c:26-26`, `/Users/acoliver/projects/uqm/sc2/src/libs/resource/loadres.c:23-23`)
- Rust exports the replacement ABI with the same public names (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:222-1224`)

### Boundary 2: Rust core behavior, C type-specific authority

Rust owns generic resource dispatch, but C still owns most handler implementations:

- Rust `InitResourceSystem()` performs downstream C registration (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:233-272`)
- C subsystem registration functions install handler vectors and continue to point at C loader/free code (`/Users/acoliver/projects/uqm/sc2/src/libs/graphics/resgfx.c:34-39`, `/Users/acoliver/projects/uqm/sc2/src/libs/strings/sresins.c:33-39`, `/Users/acoliver/projects/uqm/sc2/src/libs/sound/resinst.c:34-39`, `/Users/acoliver/projects/uqm/sc2/src/libs/video/vresins.c:166-170`, `/Users/acoliver/projects/uqm/sc2/src/uqm/dummy.c:154-159`)

### Boundary 3: Rust ABI still depends on C UIO and C globals

- UIO imports and `contentDir` remain external C symbols (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:28-49`)
- `_cur_resfile_name` remains an exported global compatibility point shared with C conventions (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:146-153`, `/Users/acoliver/projects/uqm/sc2/src/libs/resource/getres.c:28-31`)

### Boundary 4: audio remains feature-dependent hybrid behavior

- default path: Rust `InitResourceSystem()` calls C `InstallAudioResTypes()` (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:266-269`)
- alternate path when `audio_heart` feature is enabled: Rust installs `SNDRES` and `MUSICRES` handlers backed by Rust sound FFI (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:250-264`)

This makes audio resource ownership conditional rather than universally Rust-owned.

## Exported symbols and important integration points

Important Rust-exported resource ABI symbols include:

- lifecycle: `InitResourceSystem`, `UninitResourceSystem` (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:223-293`)
- index I/O: `LoadResourceIndex`, `SaveResourceIndex` (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:304-461`)
- registration and dispatch: `InstallResTypeVectors`, `res_GetResource`, `res_DetachResource`, `res_FreeResource`, `res_Remove`, `CountResourceTypes` (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:470-577`, `/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:664-673`)
- typed key access: `res_HasKey`, `res_IsString`, `res_IsInteger`, `res_IsBoolean`, `res_IsColor`, `res_GetString`, `res_GetInteger`, `res_GetBoolean`, `res_GetColor`, `res_PutString`, `res_PutInteger`, `res_PutBoolean`, `res_PutColor` (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:680-978`)
- file/resource helpers: `res_OpenResFile`, `res_CloseResFile`, `ReadResFile`, `WriteResFile`, `GetResFileChar`, `PutResFileChar`, `PutResFileNewline`, `SeekResFile`, `TellResFile`, `LengthResFile`, `DeleteResFile`, `LoadResourceFromPath`, `GetResourceData`, `FreeResourceData` (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:986-1224`)
- compatibility global: `_cur_resfile_name` (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:148-150`)

Important C integration points consuming those symbols include:

- engine startup and shutdown (`/Users/acoliver/projects/uqm/sc2/src/uqm/setup.c:109-115`, `/Users/acoliver/projects/uqm/sc2/src/uqm/cleanup.c:35-40`)
- config and content index load (`/Users/acoliver/projects/uqm/sc2/src/uqm.c:358-363`, `/Users/acoliver/projects/uqm/sc2/src/options.c:496-507`)
- input configuration read/write (`/Users/acoliver/projects/uqm/sc2/src/libs/input/sdl/input.c:86-108`, `/Users/acoliver/projects/uqm/sc2/src/libs/input/sdl/input.c:176-198`, `/Users/acoliver/projects/uqm/sc2/src/libs/input/sdl/input.c:528-576`)
- subsystem instance construction via resource handles (`/Users/acoliver/projects/uqm/sc2/src/libs/graphics/resgfx.c:42-52`, `/Users/acoliver/projects/uqm/sc2/src/libs/sound/resinst.c:42-63`, `/Users/acoliver/projects/uqm/sc2/src/libs/strings/sresins.c:42-53`, `/Users/acoliver/projects/uqm/sc2/src/libs/video/vresins.c:173-184`, `/Users/acoliver/projects/uqm/sc2/src/uqm/dummy.c:162-171`)

## Parity gaps and behavior differences visible in code

### `LoadResourceFromPath` sentinel/directory behavior differs from C

C `res_OpenResFile()` detects directories and returns a sentinel `~0`, and `LengthResFile()` treats that sentinel as length `1` (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/filecntl.c:32-41`, `/Users/acoliver/projects/uqm/sc2/src/libs/resource/filecntl.c:136-145`).

Rust defines the same sentinel constant and honors it in many file wrapper APIs (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:152-153`, `/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:999-1099`), but `res_OpenResFile()` itself is currently just a direct `uio_fopen()` call and does not perform the C-side `uio_stat` directory/sentinel check (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:986-995`). That is a concrete behavior gap.

### `GetResourceData` does not exactly match C stream positioning behavior

C `GetResourceData()` reads the 4-byte prefix and, if it is `~0`, reads the remaining `length - 4` bytes (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/loadres.c:29-55`).

Rust implements the same broad rule (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:1160-1224`), but its doc comment says it seeks back 4 bytes and reads `length` raw, while the actual implementation does **not** seek back and instead reads `length - 4` bytes (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:1160-1193`). The code behavior matches the C implementation more than the comment, but the comment itself is misleading.

### Separate Rust `ResourceSystem`/`PropertyFile` path does not match the active `.rmp` contract

The active bridge path parses `TYPE:path` descriptors through `parse_propfile()` and `dispatch.process_resource_desc()` (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:359-366`).

But `resource_system.rs` uses `PropertyFile::load()` and assumes index values look like `FILENAME,TYPE` (`/Users/acoliver/projects/uqm/rust/src/resource/resource_system.rs:49-63`), while `PropertyFile` uppercases keys and stores a different case-normalized model (`/Users/acoliver/projects/uqm/rust/src/resource/propfile.rs:152-260`). That alternate path is not parity-compatible with the active engine contract and is another sign that not all Rust resource code is on the authoritative path.

### `res_GetString` semantics in Rust are simplified

C `res_GetString()` verifies that the descriptor type is `STRING` and returns an empty string on mismatch/missing entry (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/resinit.c:456-468`).

Rust `res_GetString()` currently returns the entry `fname` for any existing key and returns null for a missing entry; it does not enforce `STRING` type at that boundary (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:785-817`). That is a visible semantic mismatch.

### Threading model remains fragile around `_cur_resfile_name`

The C implementation documents `_cur_resfile_name` as a global set during file loads (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/getres.c:28-31`). Rust preserves the same global (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:146-153`) and mutates it in `LoadResourceFromPath()` without synchronization beyond the broader API call flow (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:1148-1155`). That keeps compatibility, but not a stronger ownership/thread-safety model.

## Notable risks and unknowns

### Hybrid ownership risk

The subsystem is only partially ported because Rust core dispatch still relies on C-owned loader/free function pointers for most real asset types. Any mismatch in ABI, memory ownership, or callback assumptions will surface at the Rust↔C handler boundary (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_types.rs:37-63`, `/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:470-497`, `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/resgfx.c:34-39`, `/Users/acoliver/projects/uqm/sc2/src/libs/sound/resinst.c:34-39`).

### Duplicate Rust implementations risk

There are two overlapping Rust resource implementations:

- the active C-ABI replacement (`ffi_bridge.rs` + `dispatch.rs`)
- the older/alternate `ffi.rs` + `resource_system.rs` + `loader.rs`/`cache.rs` path

Code evidence does not show both being integrated into production C call sites. That duplication raises drift risk and makes it unclear which code should be treated as authoritative for future work (`/Users/acoliver/projects/uqm/rust/src/resource/ffi.rs:17-555`, `/Users/acoliver/projects/uqm/rust/src/resource/resource_system.rs:20-258`).

### UIO dependency risk

The Rust resource bridge is not a self-contained resource stack. It cannot function without C UIO, `contentDir`, and downstream subsystem registration symbols (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:28-49`). Any future attempt at full Rust ownership must either preserve those dependencies intentionally or replace them.

### Incomplete parity confidence on low-level file wrappers

The code shows at least one concrete mismatch (`res_OpenResFile` directory sentinel handling) and one semantic difference (`res_GetString`) relative to C (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/filecntl.c:32-41`, `/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:986-995`, `/Users/acoliver/projects/uqm/sc2/src/libs/resource/resinit.c:456-468`, `/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:785-817`). Those imply other wrapper-level edge cases may still need direct code audit.

### Memory-management unknowns across detach/free paths

`dispatch.rs` models heap-vs-value behavior based on whether `free_fun` exists (`/Users/acoliver/projects/uqm/rust/src/resource/dispatch.rs:184-324`). That matches the classic design, but concrete lifetime correctness still depends on each downstream C subsystem honoring the same allocation/free conventions. The code provides the function-pointer plumbing, but not proof of full end-to-end parity for every registered resource type.

## Summary state

The resource subsystem is currently **Rust-owned at the generic core ABI/dispatch layer, but still hybrid in concrete behavior and integration**.

- **Rust-owned now:** public resource ABI replacement, built-in type handling, index parsing/loading, generic dispatch/refcount/remove logic, UIO wrapper exports (`/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:219-1224`)
- **Still C-owned or C-authoritative:** most real resource type loaders/frees, directory and string-bank support helpers, UIO implementation, global content directory state, and much of downstream asset decoding (`/Users/acoliver/projects/uqm/sc2/src/libs/graphics/resgfx.c:21-39`, `/Users/acoliver/projects/uqm/sc2/src/libs/sound/resinst.c:22-39`, `/Users/acoliver/projects/uqm/sc2/src/libs/strings/sresins.c:21-39`, `/Users/acoliver/projects/uqm/sc2/src/libs/video/vresins.c:166-170`, `/Users/acoliver/projects/uqm/sc2/src/libs/resource/direct.c:23-95`, `/Users/acoliver/projects/uqm/sc2/src/libs/resource/stringbank.c:1-151`)
- **Hybrid split boundary:** Rust owns resource-map authority and ABI symbol resolution under `USE_RUST_RESOURCE`, while C still injects most type-specific load/free behavior through callback registration (`/Users/acoliver/projects/uqm/sc2/src/libs/resource/resinit.c:34-34`, `/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:233-272`, `/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:470-577`)

This is therefore a genuine partial port rather than a full subsystem takeover.