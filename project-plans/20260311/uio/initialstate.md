# UIO subsystem initial state

## Scope and purpose

UIO is the repository's virtual file-system layer. In the C game startup path it is responsible for:

- opening the UIO repository and directory handles used as global roots such as `contentDir`, `configDir`, `saveDir`, and `meleeDir` (`/Users/acoliver/projects/uqm/sc2/src/options.c:64-68`)
- mounting the base content tree, config tree, addon trees, and package archives into a merged namespace (`/Users/acoliver/projects/uqm/sc2/src/options.c:238-248`, `/Users/acoliver/projects/uqm/sc2/src/options.c:336-358`, `/Users/acoliver/projects/uqm/sc2/src/options.c:375-404`, `/Users/acoliver/projects/uqm/sc2/src/options.c:463-480`)
- enumerating mounted content for `.zip` / `.uqm` packages and `.rmp` resource indices (`/Users/acoliver/projects/uqm/sc2/src/options.c:469-480`, `/Users/acoliver/projects/uqm/sc2/src/options.c:496-507`)
- providing the stream and handle APIs consumed by graphics, sound, and resource loading code via `uio_*` functions (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.h:32-59`)

In this repository, UIO is partially ported: core exported `uio_*` entry points are implemented in Rust, but important behavior remains simplified, stubbed, or still structurally dependent on C-side integration.

## Evidence classification convention

Throughout this document, claims are marked with one of three confidence levels:

- **[Observed in code]** — directly visible in inspected source files. File and line references are provided. These are facts.
- **[Strong inference]** — not directly observed as a failing path or runtime behavior, but a high-confidence conclusion drawn from code structure, API contracts, and consumer patterns. The reasoning chain is stated.
- **[Unknown / requires runtime verification]** — behavior that cannot be determined from static code inspection alone. Stated as an open question.

## Current C structure

### Public C API surface

**[Observed in code]** The public umbrella header is still C-owned and simply re-exports the internal UIO API:

- `/Users/acoliver/projects/uqm/sc2/src/libs/uio.h:21-29`

**[Observed in code]** The stream API contract remains defined by C headers. `uio_Stream`, `uio_DirHandle`, and `uio_Handle` are still the types expected by the rest of the C codebase, and the stream API still includes the full stdio-style surface:

- `/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.h:28-59`
- `/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.h:75-97`

**[Observed in code]** Notably, when `USE_RUST_UIO` is enabled, only `uio_fread` gets a special declaration annotation in the C header:

- `/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.h:35-40`

That is evidence that the C side still treats the Rust port as a special interop case rather than a transparent replacement.

### C startup / mount orchestration still owns subsystem policy

**[Observed in code]** The overall mount policy is still in C startup code, not Rust:

- `prepareConfigDir()` mounts config storage and opens `configDir` (`/Users/acoliver/projects/uqm/sc2/src/options.c:205-255`)
- `mountContentDir()` mounts the base content tree, opens `contentDir`, then scans `/packages` and mounts archive files (`/Users/acoliver/projects/uqm/sc2/src/options.c:329-362`)
- `mountAddonDir()` mounts addon directories, enumerates addon subdirectories, and mounts ZIP/UQM content beneath them (`/Users/acoliver/projects/uqm/sc2/src/options.c:364-459`)
- addon shadow-content mounting is also still orchestrated in C: after opening each addon's `shadow-content` directory, startup mounts ZIP/UQM shadow packages through `mountDirZips()` and then mounts non-zipped shadow content above `/` with `uio_transplantDir("/", shadowDir, uio_MOUNT_RDONLY | uio_MOUNT_ABOVE, contentMountHandle)` (`/Users/acoliver/projects/uqm/sc2/src/options.c:575-589`)

- `mountDirZips()` is the key archive integration point: it enumerates `\.([zZ][iI][pP]|[uU][qQ][mM])$` and mounts each entry as `uio_FSTYPE_ZIP` (`/Users/acoliver/projects/uqm/sc2/src/options.c:463-480`)
- `loadIndices()` enumerates `\.[rR][mM][pP]$` and calls `LoadResourceIndex()` for each hit (`/Users/acoliver/projects/uqm/sc2/src/options.c:490-507`)

This means subsystem ownership is split: Rust provides many primitive operations, but content-discovery and mount topology are still orchestrated by C.

### C source exclusion boundaries under Rust mode

**[Observed in code]** Two major legacy implementation files are explicitly guarded out when Rust UIO is enabled:

- `/Users/acoliver/projects/uqm/sc2/src/libs/uio/io.c:16-18`
- `/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.c:16-18`

**[Observed in code]** Build selection for the UIO library also changes under `USE_RUST_UIO`. In normal mode, the UIO library includes the full C implementation set including `io.c` and `uiostream.c`; in Rust mode it reduces to:

- `charhashtable.c`
- `paths.c`
- `uioutils.c`
- `uio_fread_shim.c`

Evidence:

- full list: `/Users/acoliver/projects/uqm/sc2/src/libs/uio/Makeinfo:6-12`
- Rust-mode replacement list: `/Users/acoliver/projects/uqm/sc2/src/libs/uio/Makeinfo:14-18`

So the old C core is not fully present in a Rust build, but some C-side helpers remain linked.

## Current Rust structure

### Module location and crate wiring

**[Observed in code]** The Rust UIO implementation lives in:

- `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs`

It is exported through the Rust I/O module:

- `/Users/acoliver/projects/uqm/rust/src/io/mod.rs:6-10`

And the Rust static library is linked as part of the main Rust crate:

- `/Users/acoliver/projects/uqm/rust/Cargo.toml:6-8`
- `/Users/acoliver/projects/uqm/rust/src/lib.rs:7-23`

### Rust-owned UIO data structures

**[Observed in code]** The Rust bridge defines C-compatible representations for the main opaque UIO objects:

- `uio_DirHandle` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:57-63`)
- `uio_Repository` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:65-68`)
- `uio_MountHandle` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:70-75`)
- `uio_Stream` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:269-279`)
- `uio_DirList` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:253-259`)

**[Observed in code]** The file-level comment describes the implementation as a minimal stdio-backed replacement:

- `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1-2`

That description is consistent with the actual implementation choices below.

### Mount registry implementation

**[Observed in code]** The Rust port maintains a process-global mount registry and sorts mounts by active state, mount-path length, and recency:

- mount metadata: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:38-46`
- registry globals: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:48-53`
- registration: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:316-345`
- ordering: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:347-354`
- removal helpers: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:356-380`
- virtual-path resolution against the registry: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:421-465`

This is the main Rust-side replacement for the legacy C mount tree.

## Build and configuration wiring

Rust UIO is compiled in by configuration macro and Makeinfo switching.

### Global build flag

**[Observed in code]** The generated platform config currently defines the feature on Unix:

- `/Users/acoliver/projects/uqm/sc2/config_unix.h:73-74`

### UIO library source selection

**[Observed in code]** The UIO sub-build exports `USE_RUST_UIO` to Makeinfo and then excludes the main C UIO core files when enabled:

- export: `/Users/acoliver/projects/uqm/sc2/src/libs/uio/Makeinfo:1-4`
- normal C file set: `/Users/acoliver/projects/uqm/sc2/src/libs/uio/Makeinfo:6-12`
- Rust-mode reduced set: `/Users/acoliver/projects/uqm/sc2/src/libs/uio/Makeinfo:14-18`

### Graphics build still compiles C SDL/UIO adapter

**[Observed in code]** The SDL graphics sub-build still compiles `sdluio.c` unconditionally in the current Makeinfo file:

- `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/Makeinfo:1-9`

That file is therefore part of the current mixed Rust/C integration boundary.

## C <-> Rust integration points

### Rust exports consumed by C

**[Observed in code]** The Rust bridge exports the major UIO entry points as unmangled C ABI functions, including:

- directory/repository/mount functions: `uio_openRepository`, `uio_closeRepository`, `uio_openDir`, `uio_closeDir`, `uio_mountDir`, `uio_openDirRelative`, `uio_unmountDir` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:524-524`, `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1345-1524`)
- file-handle APIs: `uio_open`, `uio_close`, `uio_read`, `uio_write`, `uio_fstat`, `uio_unlink` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1578-1713`)
- stream APIs: `uio_fopen`, `uio_fclose`, `uio_fseek`, `uio_ftell`, `rust_uio_fread`, `uio_fgets`, `uio_fgetc`, `uio_ungetc`, `uio_fputc`, `uio_fputs`, `uio_fflush`, `uio_feof`, `uio_ferror`, `uio_clearerr`, `uio_streamHandle` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:603-887`, `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1217-1218`, `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1737-1956`)
- directory listing: `uio_getDirList`, `uio_DirList_free` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1979-2172`)

**[Observed in code]** Important split detail: the Rust file exports `rust_uio_fread`, not `uio_fread`:

- `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1837-1842`

### C shim forwarding back into Rust

**[Observed in code]** The actual `uio_fread` symbol is still provided by a C shim which logs and forwards to `rust_uio_fread`:

- declaration of Rust target: `/Users/acoliver/projects/uqm/sc2/src/libs/uio/uio_fread_shim.c:6`
- C-exported `uio_fread` wrapper: `/Users/acoliver/projects/uqm/sc2/src/libs/uio/uio_fread_shim.c:8-14`

This is a hard evidence point that the port is not yet a pure Rust symbol-for-symbol takeover.

### C code paths depending on Rust UIO exports

**[Observed in code]** The C startup code depends on the Rust replacements for repository, mount, directory-open, directory-list, stat, and stream functions:

- config/content mounting: `/Users/acoliver/projects/uqm/sc2/src/options.c:238-248`, `/Users/acoliver/projects/uqm/sc2/src/options.c:336-358`
- addon/archive discovery: `/Users/acoliver/projects/uqm/sc2/src/options.c:393-404`, `/Users/acoliver/projects/uqm/sc2/src/options.c:469-480`
- resource-index enumeration: `/Users/acoliver/projects/uqm/sc2/src/options.c:496-507`

**[Observed in code]** The SDL image loader still depends on stream-style UIO from C:

- file open: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdluio.c:43-61`
- seek/read/write/error/close callbacks into `uio_*`: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdluio.c:64-149`

### Rust subsystems consuming UIO via FFI

**[Observed in code]** Other Rust subsystems use UIO as an external service rather than replacing it internally.

Sound decoders use handle-style `uio_open` / `uio_read` / `uio_close` / `uio_fstat`:

- AIFF: `/Users/acoliver/projects/uqm/rust/src/sound/aiff_ffi.rs:21-31`, `/Users/acoliver/projects/uqm/rust/src/sound/aiff_ffi.rs:51-79`
- WAV: `/Users/acoliver/projects/uqm/rust/src/sound/wav_ffi.rs:20-30`
- MOD: `/Users/acoliver/projects/uqm/rust/src/sound/mod_ffi.rs:20-30`
- DukAud: `/Users/acoliver/projects/uqm/rust/src/sound/dukaud_ffi.rs:17-27`, `/Users/acoliver/projects/uqm/rust/src/sound/dukaud_ffi.rs:47-75`

**[Observed in code]** The Rust audio-heart bridge uses stream-style `uio_fopen` / `uio_fread` / `uio_fseek` / `uio_ftell` via FFI and reads from the C global `contentDir`:

- `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:41-52`
- `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:100-119`

**[Observed in code]** The Rust resource bridge also uses UIO via FFI and imports the C global `contentDir`:

- UIO imports: `/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:28-49`
- helper/test stubs showing UIO remains an external boundary: `/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:88-127`
**[Observed in code]** Legacy native-path bridging is stronger than an `ENOTSUP` fallback for non-STDIO content. `uio_getStdioAccess()` first calls `uio_getFileLocation()`, returns a direct path when the owning mount is `uio_FSTYPE_STDIO`, and otherwise creates a temporary directory, copies the file there with `uio_copyFile()`, then returns a stdio path to the temporary copy. This means the legacy helper stack expects `uio_getFileLocation()` to succeed far enough to identify the owning mount and provide a path-like location even when the original backing filesystem is not directly stdio-usable (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/utils.c:166-323`, `/Users/acoliver/projects/uqm/sc2/src/libs/uio/io.c:1137-1229`).

**[Observed in code]** Legacy `uio_fflush()` does **not** accept null. `uiostream.c` explicitly notes that stdio `fflush(NULL)` flushes all streams but UIO does not, and the implementation asserts `stream != NULL` before proceeding (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.c:450-455`).



This shows that even Rust subsystems are mostly written against the C ABI surface, not against an internal Rust UIO API.

## What is already ported to Rust

### Core repository / mount / directory operations

**[Observed in code]** Rust owns the exported implementations for:

- repository open/close (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1345-1359`)
- mount registration / unmounting (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1451-1520`, `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:524-544`)
- opening directory handles and relative directory handles (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1395-1432`, `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1524-1570`)
- directory listing and freeing `uio_DirList` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1979-2172`)

### Core file and stream operations

**[Observed in code]** Rust owns the main implementations for:

- descriptor-style file open/read/write/close/stat/unlink (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1578-1713`)
- stream open/close/read/seek/tell (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1737-1956`)
- line and character operations (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:603-743`)
- simple output helpers (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:758-883`)

### Pattern-based directory enumeration used by startup

**[Observed in code]** The current Rust implementation has explicit support for the two startup patterns used by `options.c`:

- `.rmp` matching: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1304-1310`
- `.zip` / `.uqm` matching: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1306-1311`
- use in `uio_getDirList()`: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:2037-2042`

This is enough to support the current C startup scans in `loadIndices()` and `mountDirZips()`.

### Unit-test coverage exists for selected pieces

**[Observed in code]** `uio_bridge.rs` has unit tests covering mount resolution, pattern matching, and side registries:

- tests module start: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:2218-2480`
- mount registry tests: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:2256-2318`
- pattern matching tests: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:2361-2433`

This is evidence of some Rust-side verification, though it is narrow and mostly local.

## What remains C-owned

### Mount policy and global-directory lifecycle

**[Observed in code]** The game still creates and manages `contentDir`, `configDir`, `saveDir`, `meleeDir`, and `contentMountHandle` in C globals:

- `/Users/acoliver/projects/uqm/sc2/src/options.c:64-68`

**[Observed in code]** The logic for where to mount content, how to discover addons, and when to load `.rmp` indices remains C-owned:

- `/Users/acoliver/projects/uqm/sc2/src/options.c:135-202`
- `/Users/acoliver/projects/uqm/sc2/src/options.c:205-255`
- `/Users/acoliver/projects/uqm/sc2/src/options.c:329-362`
- `/Users/acoliver/projects/uqm/sc2/src/options.c:364-459`
- `/Users/acoliver/projects/uqm/sc2/src/options.c:490-507`

### Part of the UIO link surface is still implemented in C

**[Observed in code]** `uio_fread` itself is still a C function in Rust mode, via the shim:

- `/Users/acoliver/projects/uqm/sc2/src/libs/uio/uio_fread_shim.c:8-14`

That means not every exported `uio_*` symbol is directly Rust-owned.

### Graphics-side SDL adapter remains C-owned

**[Observed in code]** The `SDL_RWops` adapter around UIO is still a C file compiled into the build:

- build inclusion: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/Makeinfo:1-9`
- implementation: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdluio.c:30-149`

### Public C headers still define the contract

**[Observed in code]** The rest of the codebase still compiles against the C UIO headers and struct layout contract, especially `uiostream.h`:

- `/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.h:32-59`
- `/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.h:75-97`

## Partial-port boundaries, stubs, and fallback behavior

### Boundary: ZIP mounts are recognized but not active in Rust registry

**[Observed in code]** When Rust handles `uio_mountDir()`, it records mounts with `active_in_registry = _fsType != UIO_FSTYPE_ZIP` when mounting relative to a source dir:

- `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1476-1490`

**[Observed in code]** This is a key partial-port boundary. The C startup code still requests ZIP mounts using `uio_FSTYPE_ZIP`:

- `/Users/acoliver/projects/uqm/sc2/src/options.c:477-480`

But the Rust mount registry explicitly does not activate those ZIP mounts for path resolution in that code path. That is direct evidence that archive mounting semantics are not fully ported.

### Boundary: pattern matching is a hard-coded approximation, not a full regex engine

**[Observed in code]** `uio_getDirList()` accepts a pattern and match type, but the Rust implementation only emulates a few known regex forms and otherwise falls back to substring-like behavior:

- match-type constants: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1289-1294`
- special-cased `.rmp` and `.zip`/`.uqm` logic: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1303-1314`
- dispatch by match type: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1317-1323`

This is sufficient for the current startup scans but is not general parity with a regex-capable directory matcher.

### Stub: `uio_vfprintf`

**[Observed in code]** `uio_vfprintf` is explicitly stubbed because of stable-Rust variadic limitations:

- rationale comment: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:745-746`
- stubbed function body returning error: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:748-755`

### Stub: `uio_feof`

**[Observed in code]** `uio_feof` is not derived from stream state; it always returns true:

- `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:837-841`

That is an explicit parity gap for any caller that relies on EOF state.

### Stub: `uio_ferror`

**[Observed in code]** `uio_ferror` is also stubbed and always reports no error:

- `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:843-847`

**[Observed in code]** This matters because the C SDL adapter checks `uio_ferror()` after zero-byte reads/writes:

- read path: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdluio.c:92-100`
- write path: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdluio.c:114-123`

**[Strong inference]** Because `uio_ferror()` is hardcoded to return 0 and `uio_feof()` is hardcoded to return 1, error propagation through the SDL_RWops integration point is degraded: `sdluio.c` cannot distinguish a real I/O error from a normal EOF, and the always-true EOF return may cause callers to misinterpret stream state. The inference is that this will produce incorrect error-handling behavior at runtime, though the exact failure mode depends on which SDL consumers exercise the error path.

### Stub: `uio_clearerr`

**[Observed in code]** `uio_clearerr` is a no-op stub:

- `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:886-889`

### Fallback / compromise: `uio_fclose` buffer handling leaks

**[Observed in code]** `uio_fclose()` acknowledges missing buffer deallocation metadata and intentionally leaks the stream buffer if allocated:

- `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1821-1826`

### Fallback / side registry: `uio_DirList_free`

**[Observed in code]** `uio_DirList_free()` relies on an auxiliary global buffer-size registry because the C struct does not carry enough metadata for direct Rust deallocation:

- explanation and workaround: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:2117-2156`
- registry globals/helpers: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:2179-2211`

This works as a compatibility layer but is another sign of incomplete ergonomic ownership.

### Defensive guard behavior around stream handles

**[Observed in code]** `rust_uio_fread()` contains extra pointer sanity checks and catches unwind while dereferencing the stream handle:

- null/size validation: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1847-1869`
- low-address guard: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1871-1880`
- `catch_unwind` around handle dereference: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1882-1891`

These guards are evidence that the interop boundary is considered fragile enough to need runtime hardening.

## Parity gaps versus the legacy C subsystem

## ZIP / packaged content parity gap

**[Observed in code]** The largest gap visible in code is archive-backed mounting.

- C still asks to mount ZIP/UQM packages as `uio_FSTYPE_ZIP` (`/Users/acoliver/projects/uqm/sc2/src/options.c:477-480`)
- Rust explicitly deactivates ZIP mounts in the registry for one important `uio_mountDir()` path (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1489-1490`)

**[Strong inference]** Because the Rust registry marks ZIP mounts as inactive, package content discovery may succeed at the enumeration level (the listing may still work for the directory containing the archive files), but path resolution into archive-backed content will fail — callers attempting to open files inside mounted archives will not find them through the normal resolution path. This means the archive-mounting flow completes without error but does not produce functional archive access.

## Stream-state parity gap

**[Observed in code]** The current Rust bridge does not provide full stream-state behavior expected by the C API:

- `uio_feof()` always returns 1 (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:837-841`)
- `uio_ferror()` always returns 0 (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:843-847`)
- `uio_clearerr()` does nothing (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:886-889`)

That is not full parity with the contract implied by `uiostream.h` (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.h:55-59`, `/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.h:87-96`).

## Variadic output parity gap

**[Observed in code]** `uio_vfprintf()` is unimplemented and returns error:

- `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:745-755`

**[Unknown / requires runtime verification]** Whether any caller exercises `uio_vfprintf` in a runtime path that affects correctness is not established by the inspected code. It may be debug/diagnostic only.

## Directory-match semantics parity gap

**[Observed in code]** The Rust implementation's pseudo-regex matcher only special-cases a few known patterns and otherwise does simplified string matching:

- `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1296-1323`

This is narrower than a general regex/match facility.

## Ownership/API parity gap inside Rust

**[Observed in code]** Even Rust subsystems use the C ABI surface rather than a native Rust UIO abstraction:

- sound decoder FFI imports: `/Users/acoliver/projects/uqm/rust/src/sound/aiff_ffi.rs:21-31`, `/Users/acoliver/projects/uqm/rust/src/sound/wav_ffi.rs:20-30`, `/Users/acoliver/projects/uqm/rust/src/sound/mod_ffi.rs:20-30`, `/Users/acoliver/projects/uqm/rust/src/sound/dukaud_ffi.rs:17-27`
- resource bridge UIO imports: `/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs:28-38`
- audio-heart UIO imports: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:42-48`

That is not wrong, but it shows the subsystem has been ported as an ABI replacement, not yet as a Rust-internal service.

## Notable risks and unknowns

### Package/archive content may not resolve like legacy UIO

**[Observed in code]** C still requests ZIP mounts but Rust does not fully activate them in its registry:

- request to mount ZIP archives: `/Users/acoliver/projects/uqm/sc2/src/options.c:469-480`
- ZIP inactive in registry: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1489-1490`

**[Strong inference]** Because ZIP mounts are registered but inactive, any content that lives exclusively inside `.zip` / `.uqm` packages will be unreachable through UIO path resolution. This is the highest-risk functional gap because a significant portion of game content is distributed as package archives. The inference follows directly from the `active_in_registry = false` assignment for ZIP mounts — the resolution loop skips inactive mounts.

### Error reporting through SDL_RWops

**[Observed in code]** `sdluio.c` expects `uio_ferror()` to distinguish zero-byte read/write from actual error:

- `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdluio.c:92-100`
- `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdluio.c:114-123`

**[Observed in code]** Rust currently hardcodes `uio_ferror()` to zero:

- `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:843-847`

**[Strong inference]** Because `uio_ferror()` always returns 0, `sdluio.c` will never enter the `SDL_SetError` error-reporting branch after a failed read or write. This means I/O errors on UIO streams will be silently swallowed by the SDL adapter rather than propagated to SDL consumers. The inference follows from the hardcoded return value combined with the explicit conditional in `sdluio.c`. The exact user-visible impact depends on whether SDL consumers check for and react to these errors, which is not established by the inspected code.

### Some memory-management paths are compatibility workarounds, not clean ownership

**[Observed in code]** These are directly visible workarounds:

- stream buffers may leak on `uio_fclose()` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1821-1826`)
- `uio_DirList_free()` depends on a global size side table (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:2117-2156`, `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:2179-2211`)

These are survivable, but they are evidence that some object-lifetime details are still provisional.

### Interop fragility is explicitly acknowledged in code

**[Observed in code]** The need for a C `uio_fread` shim plus Rust-side pointer guards suggests the stream ABI boundary has been troublesome:

- C shim: `/Users/acoliver/projects/uqm/sc2/src/libs/uio/uio_fread_shim.c:6-14`
- Rust defensive checks: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1865-1891`

### Unknowns not resolved by current code evidence

**[Unknown / requires runtime verification]** The code shows current implementation shape, but some behaviors are still not provable from the inspected sources alone:

- whether all call sites needing archive-backed reads still work correctly when content lives inside `.zip` / `.uqm` packages, given the inactive ZIP mount handling boundary
- whether any production code path depends materially on `uio_vfprintf`, `uio_clearerr`, or accurate `uio_feof` / `uio_ferror` state beyond the `sdluio.c` paths already identified
- whether the current mixed model causes lifetime or aliasing bugs for long-lived `uio_Stream` objects shared across C and Rust
- whether `.rmp` resource index files exist inside ZIP/UQM archives, in STDIO-backed directories only, or both — `loadIndices()` (`options.c:490-507`) enumerates `.rmp` files after archive mounting completes, but whether the legacy C implementation's merged directory listing actually returns `.rmp` entries from inside archives is not confirmed by static inspection
- whether the ordering of `.rmp` files in a merged directory listing affects resource-override semantics in `LoadResourceIndex()` — if later-loaded indices override earlier ones, then listing order is behaviorally significant and the merged-listing ordering contract must match the legacy C implementation
- whether duplicate `.rmp` file names across multiple contributing mounts (STDIO and ZIP) are deduplicated by the legacy C directory listing or returned as separate entries — this determines whether the first-seen deduplication assumption is correct
- what happens to directory handles, file handles, and streams when their backing mount is unmounted while they are still live — whether the legacy C implementation enforced safe failure or relied on callers not doing this

## Summary

**[Observed in code]** The UIO subsystem is no longer primarily implemented by the legacy C `io.c` / `uiostream.c` core in Rust-enabled builds. Rust now owns the main exported repository, directory, file, stream, and directory-list operations, with build-time guards preventing the old C core from compiling (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/Makeinfo:14-18`, `/Users/acoliver/projects/uqm/sc2/src/libs/uio/io.c:16-18`, `/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.c:16-18`).

However, this is still a partial port rather than a finished takeover. The strongest evidence is:

- **[Observed in code]** `uio_fread` still crosses a C shim boundary (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/uio_fread_shim.c:8-14`)
- **[Observed in code]** mount policy remains in C startup code (`/Users/acoliver/projects/uqm/sc2/src/options.c:205-255`, `/Users/acoliver/projects/uqm/sc2/src/options.c:329-507`)
- **[Observed in code]** ZIP/package mounts are not fully active in Rust path resolution (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1476-1490`)
- **[Observed in code]** several stream APIs are explicit stubs or simplified approximations (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:745-755`, `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:837-889`)

So the subsystem's current state is: Rust owns most primitive `uio_*` machinery, but the repository still depends on C-owned mount orchestration, C-owned adapters, a C shim for at least one stream function, and several non-parity behaviors in the Rust bridge.
