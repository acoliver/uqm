# File I/O subsystem initial state

## Scope and current role

In this repository, the file I/O subsystem is the UIO layer: the engine-facing virtual filesystem and stream API exposed through `libs/uio.h` and the underlying `uio_*` symbols. The public umbrella header is still C-owned and re-exports the UIO interface from `uio/io.h` (`/Users/acoliver/projects/uqm/sc2/src/libs/uio.h:21-29`).

### Evidence labeling

This document uses the following labels to distinguish the basis for each claim:

- **Observed** — directly evidenced in cited code (specific file and line range provided).
- **Inferred** — strong behavioral implication from code shape, but not directly tested or executed to verify.
- **Unknown** — plausible but not verified; requires further investigation.

Where no label is given, the statement is **Observed** and the citation immediately follows.

The code-supported responsibilities today are:

- repository lifecycle via `uio_init`, `uio_unInit`, `uio_openRepository`, `uio_closeRepository` (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/io.h:66-76`)
- mount lifecycle and namespace access via `uio_mountDir`, `uio_transplantDir`, `uio_unmountDir`, `uio_unmountAllDirs`, `uio_openDir`, `uio_openDirRelative`, `uio_closeDir`, `uio_getDirList` (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/io.h:78-98`, `/Users/acoliver/projects/uqm/sc2/src/libs/uio/io.h:134-148`)
- descriptor-style file operations via `uio_open`, `uio_close`, `uio_read`, `uio_write`, `uio_lseek`, `uio_fstat`, `uio_stat`, `uio_access`, `uio_rename`, `uio_unlink`, `uio_mkdir`, `uio_rmdir`, `uio_getFileLocation` (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/io.h:100-132`)
- buffered stream operations via `uio_fopen`, `uio_fclose`, `uio_fread`, `uio_fgets`, `uio_fgetc`, `uio_ungetc`, `uio_vfprintf`, `uio_fprintf`, `uio_fputc`, `uio_fputs`, `uio_fseek`, `uio_ftell`, `uio_fwrite`, `uio_fflush`, `uio_feof`, `uio_ferror`, `uio_clearerr`, and `uio_streamHandle` (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.h:32-59`)

The subsystem is partially ported. Rust currently provides most linked `uio_*` entry points, but startup policy, some integration surfaces, and several behaviors remain C-owned, C-shimmed, simplified, or stubbed.

## Current C structure

### Public API and struct contract remain C-defined

The rest of the engine still compiles against the C header contract.

- `uio/io.h` declares repository, mount, directory, descriptor, metadata, and listing APIs (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/io.h:27-158`)
- `uiostream.h` declares the stream API and the `uio_Stream` layout expected by C callers (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.h:28-59`, `/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.h:75-97`)
- mount ordering and placement flags are still defined in the C header as `uio_MOUNT_BOTTOM`, `uio_MOUNT_TOP`, `uio_MOUNT_BELOW`, and `uio_MOUNT_ABOVE` (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/io.h:32-37`)

The `uio_DirList` externally visible shape is also still defined by the C header as `names` plus `numNames`, with hidden internal allocation details (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/io.h:57-63`).

### C startup still owns mount policy and global handles

Mount topology and content-discovery policy are still in `options.c`, not in Rust UIO.

- config storage is mounted at `/` and `configDir` is opened in `prepareConfigDir()` (`/Users/acoliver/projects/uqm/sc2/src/options.c:205-255`)
- `saveDir` and `meleeDir` are derived from `configDir` using `uio_openDirRelative()` (`/Users/acoliver/projects/uqm/sc2/src/options.c:257-327`)
- base content is mounted at `/` and `contentDir` is opened in `mountContentDir()` (`/Users/acoliver/projects/uqm/sc2/src/options.c:329-362`)
- addon policy remains C-driven in `mountAddonDir()` (`/Users/acoliver/projects/uqm/sc2/src/options.c:364-460`)
- package archive discovery remains C-driven in `mountDirZips()`, which enumerates `\.([zZ][iI][pP]|[uU][qQ][mM])$` and mounts each hit as `uio_FSTYPE_ZIP` (`/Users/acoliver/projects/uqm/sc2/src/options.c:462-488`)
- resource-index discovery remains C-driven in `loadIndices()`, which enumerates `\.[rR][mM][pP]$` and calls `LoadResourceIndex()` (`/Users/acoliver/projects/uqm/sc2/src/options.c:490-515`)

This is an important subsystem boundary: Rust supplies primitives, but startup still decides what to mount, where to mount it, and when to enumerate or load content.

### Legacy C implementation files remain present but are excluded

The legacy UIO implementation is still in the repository, but Rust mode excludes the two core C implementation files.

Build selection:

- normal UIO C file set includes `io.c` and `uiostream.c` (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/Makeinfo:6-12`)
- when `USE_RUST_UIO` is set, the UIO sub-build is reduced to `charhashtable.c`, `paths.c`, `uioutils.c`, and `uio_fread_shim.c` (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/Makeinfo:14-18`)

Hard guards in the excluded files:

- `io.c` errors out if compiled with Rust UIO enabled (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/io.c:16-18`)
- `uiostream.c` errors out if compiled with Rust UIO enabled (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.c:16-18`)

So the current partial-port boundary is explicit in code: the full legacy C core is still available in-tree, but build wiring prevents it from being part of the Rust-UIO configuration.

## Current Rust structure

### Rust UIO implementation file

The active Rust implementation is in `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs`.

It defines C-compatible representations for the main opaque objects:

- `uio_DirHandle` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:57-63`)
- `uio_Repository` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:65-68`)
- `uio_MountHandle` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:70-75`)
- `uio_DirList` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:253-259`)
- `uio_FileBlock` stub type (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:901-904`)

The Rust code also mirrors key C constants:

- mount flags and filesystem IDs (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:19-25`)
- stream status and operation constants (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:27-33`)
- match-type constants (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1289-1294`)

### Mount registry design

Rust currently replaces the legacy mount tree with a process-global registry:

- `MountInfo` stores mount id, repository key, mount handle pointer, mount point, mounted root, filesystem type, and `active_in_registry` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:38-46`)
- registry globals are `OnceLock<Mutex<Vec<MountInfo>>>` plus `NEXT_MOUNT_ID` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:48-53`)
- registration happens in `register_mount()` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:316-345`)
- ordering is implemented by sorting on `active_in_registry`, then mount-point length, then descending id (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:347-354`)
- path resolution searches only active entries via `resolve_virtual_mount_path()` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:421-443`)
- `uio_getFileLocation()` also depends on that registry and returns the owning mount handle pointer plus a duplicated path string (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:445-520`)

This is the active Rust replacement for legacy C mount state, but it is flatter and less feature-complete than the original C design implied by `mounttree.h` and mount ordering flags.

## Build and configuration wiring

### Rust UIO is enabled by configuration

`USE_RUST_UIO` is enabled in the active Unix config header (`/Users/acoliver/projects/uqm/sc2/config_unix.h:73-74`).

That same macro is observed by the C UIO headers and source files:

- `uiostream.h` changes the `uio_fread` declaration only when `USE_RUST_UIO` is set (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.h:35-40`)
- `io.c` and `uiostream.c` refuse compilation under Rust mode (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/io.c:16-18`, `/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.c:16-18`)

### UIO sub-build wiring in C

The UIO Makeinfo file exports `USE_RUST_UIO` into the sub-build environment (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/Makeinfo:1-4`) and swaps the source list in Rust mode (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/Makeinfo:14-18`).

That reduced Rust-mode C list still includes `uio_fread_shim.c`, which is evidence that the Rust port does not yet provide a complete direct symbol replacement (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/Makeinfo:16`).

### Rust library build shape

The Rust crate builds a static library named `uqm_rust` (`/Users/acoliver/projects/uqm/rust/Cargo.toml:6-8`).

`build.rs` only compiles `mem_wrapper.c`; it does not compile any UIO C replacement code besides what the C build already selects (`/Users/acoliver/projects/uqm/rust/build.rs:8-16`).

## C↔Rust integration points

### Rust exports consumed by C engine code

The Rust bridge exports many `uio_*` functions as `#[no_mangle] extern "C"`:

- repository lifecycle: `uio_init`, `uio_unInit`, `uio_openRepository`, `uio_closeRepository` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1334-1357`)
- directory and mount APIs: `uio_openDir`, `uio_closeDir`, `uio_mountDir`, `uio_openDirRelative`, `uio_unmountDir`, `uio_unmountAllDirs`, `uio_getMountFileSystemType`, `uio_transplantDir`, `uio_getFileLocation` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:489-595`, `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1395-1570`)
- descriptor-style APIs: `uio_open`, `uio_close`, `uio_read`, `uio_write`, `uio_fstat`, `uio_unlink`, `uio_lseek`, `uio_stat`, `uio_access`, `uio_rename`, `uio_mkdir`, `uio_rmdir` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:81-240`, `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1577-1729`)
- stream APIs except direct `uio_fread`: `uio_fopen`, `uio_fclose`, `uio_fseek`, `uio_ftell`, `uio_fgets`, `uio_fgetc`, `uio_ungetc`, `uio_vfprintf`, `uio_fputc`, `uio_fputs`, `uio_fflush`, `uio_feof`, `uio_ferror`, `uio_fwrite`, `uio_clearerr` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:603-889`, `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1736-1971`)
- listing APIs: `uio_getDirList` and `uio_DirList_free` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1978-2172`)
- FileBlock APIs are present but stubbed (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:906-962`)

### C shim still owns exported `uio_fread`

The partial-port boundary for `uio_fread` is explicit.

Rust exports `rust_uio_fread`, not `uio_fread`:

- Rust symbol export: `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1836-1842`

C provides the linked `uio_fread` symbol and forwards to Rust:

- Rust target declaration in shim: `/Users/acoliver/projects/uqm/sc2/src/libs/uio/uio_fread_shim.c:6`
- C-exported wrapper body: `/Users/acoliver/projects/uqm/sc2/src/libs/uio/uio_fread_shim.c:8-14`

The header also carries a Rust-mode-specific declaration change for `uio_fread` (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.h:35-40`).

This is direct evidence that the Rust port is not yet a full direct replacement for every exported file-I/O symbol.

### C consumers that still sit above Rust UIO

Startup and content orchestration remain C consumers of Rust UIO:

- config/content mount flow uses `uio_mountDir`, `uio_openDir`, `uio_openDirRelative`, `uio_getDirList`, `uio_stat`, and `uio_DirList_free` (`/Users/acoliver/projects/uqm/sc2/src/options.c:238-248`, `/Users/acoliver/projects/uqm/sc2/src/options.c:336-358`, `/Users/acoliver/projects/uqm/sc2/src/options.c:393-458`, `/Users/acoliver/projects/uqm/sc2/src/options.c:469-487`, `/Users/acoliver/projects/uqm/sc2/src/options.c:496-511`)

SDL integration is still entirely C-owned through `sdluio.c`:

- images are opened with `uio_fopen` and wrapped as `SDL_RWops` (`/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdluio.c:43-61`)
- callbacks use `uio_fseek`, `uio_ftell`, `uio_fread`, `uio_fwrite`, `uio_ferror`, and `uio_fclose` (`/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdluio.c:64-149`)
- the SDL graphics build still compiles `sdluio.c` unconditionally in its Makeinfo (`/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/Makeinfo:1-9`)

### Rust consumers still treat UIO as foreign C ABI

Several Rust subsystems also consume UIO through `extern "C"`, which means UIO is not yet an internal Rust-native service boundary.

Examples:

- sound decoders import `uio_open`, `uio_read`, `uio_close`, and `uio_fstat` via FFI (`/Users/acoliver/projects/uqm/rust/src/sound/aiff_ffi.rs:21-31`)
- those decoders then read file content through the exported UIO ABI (`/Users/acoliver/projects/uqm/rust/src/sound/aiff_ffi.rs:51-79`)
- the Rust audio heart imports `uio_fopen`, `uio_fclose`, `uio_fread`, `uio_fseek`, `uio_ftell`, and the C global `contentDir` (`/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:41-52`)
- the audio heart reads bytes by opening from `contentDir`, seeking to end, telling length, and then reading via `uio_fread` (`/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:100-118`)

So even on the Rust side, file I/O is still largely consumed as an FFI ABI defined by the C-side contract.

## What is already ported

### Repository, directory, and basic mount lifecycle

Rust implements repository allocation/free and mount registration/removal:

- `uio_openRepository` allocates a boxed repository (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1344-1349`)
- `uio_closeRepository` calls `uio_unmountAllDirs` and frees the repository (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1351-1357`)
- `uio_mountDir` registers a mount and returns a `uio_MountHandle` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1450-1520`)
- `uio_unmountDir` and `uio_unmountAllDirs` remove registry entries and free handles (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:523-547`)
- `uio_openDir`, `uio_openDirRelative`, and `uio_closeDir` create and release directory handles (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1395-1448`, `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1523-1570`)

### Basic descriptor-style file access on real filesystem paths

Rust implements OS-backed file operations with `std::fs::File` wrapped in `Mutex<File>`:

- open and close (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1577-1643`)
- read and write (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1645-1687`)
- `uio_fstat` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1689-1710`)
- `uio_lseek` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:215-240`)
- `uio_stat`, `uio_access`, `uio_rename`, `uio_mkdir`, `uio_rmdir`, `uio_unlink` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:81-212`, `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1712-1729`)

### Basic stream wrappers exist

Rust implements stream open/close plus several simple stream operations:

- `uio_fopen` and `uio_fclose` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1736-1835`)
- `rust_uio_fread` backing the C shim (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1836-1923`)
- `uio_fseek` and `uio_ftell` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1925-1971`)
- character/line helpers and basic output helpers (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:603-809`)
- `uio_fflush`, `uio_fwrite`, `uio_feof`, `uio_ferror`, `uio_clearerr` are exported, though some are stubbed or incomplete (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:812-889`)

### Directory listing exists for direct filesystem directories

Rust implements `uio_getDirList` by calling `fs::read_dir` on one resolved path, building a contiguous string buffer and pointer array, and returning a C-compatible `uio_DirList` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1978-2108`).

The free path also exists (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:2111-2172`).

### Startup-critical regex cases are partially emulated

Pattern matching has special-case logic for the startup regexes used by C policy code:

- `.rmp` regex special case (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1303-1310`)
- `.zip` / `.uqm` regex special case (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1306-1311`)
- dispatch by match type (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1317-1324`)

That is sufficient to support the current `mountDirZips()` and `loadIndices()` scans in C (`/Users/acoliver/projects/uqm/sc2/src/options.c:469-487`, `/Users/acoliver/projects/uqm/sc2/src/options.c:496-511`).

## What remains C-owned or indirectly wired

### Mount policy and global directory ownership remain in C

The global repository-adjacent handles and startup policy remain C-owned:

- `configDir`, `saveDir`, `meleeDir`, `contentDir`, and the content mount handle are created and managed in `options.c` usage paths (`/Users/acoliver/projects/uqm/sc2/src/options.c:238-248`, `/Users/acoliver/projects/uqm/sc2/src/options.c:283-291`, `/Users/acoliver/projects/uqm/sc2/src/options.c:318-326`, `/Users/acoliver/projects/uqm/sc2/src/options.c:336-358`)
- archive and addon discovery remain caller policy in C (`/Users/acoliver/projects/uqm/sc2/src/options.c:364-488`)

### The exported surface is not fully Rust-owned

`uio_fread` is still linked through C shim code, not directly from Rust (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/uio_fread_shim.c:6-14`).

### SDL_RWops adaptation remains C-owned

The bridge from UIO streams into SDL still lives in `sdluio.c` and is compiled from the graphics build (`/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdluio.c:30-149`, `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/Makeinfo:1-9`).

### Rust internal consumers still cross the ABI boundary

Rust sound code and audio-heart code do not call an internal Rust UIO API. They still import and use the C ABI functions and the C global `contentDir` (`/Users/acoliver/projects/uqm/rust/src/sound/aiff_ffi.rs:21-31`, `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:41-52`).

## Important partial-port boundaries and parity gaps

### Boundary: ZIP mounts are mounted by C policy but excluded from Rust path resolution

This is the clearest functional partial-port boundary.

C requests ZIP mounts here:

- `mountDirZips()` calls `uio_mountDir(repository, mountPoint, uio_FSTYPE_ZIP, dirHandle, dirList->names[i], "/", autoMount, relativeFlags | uio_MOUNT_RDONLY, relativeHandle)` (`/Users/acoliver/projects/uqm/sc2/src/options.c:463-480`)

Rust then explicitly deactivates those sourceDir-based ZIP mounts in the registry:

- when `sourceDir` is non-null, `active_in_registry` is set to `_fsType != UIO_FSTYPE_ZIP` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1476-1490`)
- path resolution only considers entries where `active_in_registry` is true (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:424-427`)

That means the startup code can request archive mounts, but the Rust resolution path intentionally excludes them from normal lookup in that code path.

### Boundary: mount ordering semantics are simplified

The C API exposes explicit top/bottom/above/below semantics (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/io.h:32-37`).

Rust does not preserve those placement semantics directly. Instead it sorts by:

- `active_in_registry`
- mount-point length
- descending id

as implemented in `sort_mount_registry()` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:347-354`).

`uio_mountDir()` accepts `flags` and `relative`, but beyond basic pass-through and logging, the relative placement semantics are not used to place the mount in an order-preserving structure (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1450-1520`).

`uio_transplantDir()` validates the presence of `relative` for ABOVE/BELOW but then simply calls `register_mount()` with `active_in_registry = true` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:561-595`).

### Boundary: regex/listing support is startup-oriented, not general parity

`uio_getDirList()` only reads one real directory via `fs::read_dir` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:2024-2045`).

Pattern support is partial:

- special cases for `.rmp` and `.zip`/`.uqm` regexes (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1303-1311`)
- otherwise regex mode falls back to a lowercase substring-like check (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1312-1324`)

This supports current startup scans but is not evidence of general regex parity.

### Boundary: stream status APIs are stubbed

The stream header contract defines status fields and constants in C (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.h:87-97`).

Rust exports the status APIs, but current behavior is stubbed:

- `uio_feof` always returns `1` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:837-841`)
- `uio_ferror` always returns `0` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:843-847`)
- `uio_clearerr` is a no-op (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:886-889`)

This is especially relevant because the SDL adapter uses `uio_ferror()` to distinguish EOF from error after `uio_fread()` returns 0 (`/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdluio.c:90-100`, `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdluio.c:112-123`).

### Boundary: `uio_vfprintf` is stubbed

Rust explicitly documents and implements `uio_vfprintf` as a stub returning error:

- rationale comment (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:745-746`)
- stub body (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:747-755`)

### Boundary: `uio_fclose` leaks stream buffer allocations

`uio_fclose` acknowledges that if `buf` was allocated, the current code does not know its size and leaks it:

- leak comment and TODO (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1821-1826`)

### Boundary: `uio_DirList_free` depends on side-channel allocation metadata

The C-visible `uio_DirList` does not carry allocation sizes (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/io.h:57-63`).

Rust works around that with a global registry:

- buffer-size registry type and global (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:2179-2188`)
- registration on allocation (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:2076-2077`)
- free path consults the registry to deallocate the buffer (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:2129-2156`)

That is an indirect internal ownership mechanism, not a self-contained object layout.

### Boundary: FileBlock APIs are present but stubbed

Rust exports FileBlock functions, but they are placeholders:

- open/open2 return dummy non-null pointers (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:906-922`)
- close frees the dummy object (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:924-930`)
- access/copy return errors and usage hint is a no-op (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:933-962`)

### Boundary: metadata and access semantics are simplified

`uio_access()` ignores the requested mode and performs only an existence check (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:113-136`).

`uio_stat()` and `uio_fstat()` report limited metadata, mainly size and coarse file/dir bits (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:138-169`, `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1689-1710`).

### Boundary: Rust init/uninit are placeholders

`uio_init()` and `uio_unInit()` currently only log markers and do not establish or tear down explicit subsystem-global handler state (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1334-1342`).

## What remains C-owned or indirect in behavior

Even where Rust owns the function body, many behaviors are still effectively indirect OS-path passthrough rather than full UIO semantics.

- `uio_openDir()` resolves through the registry, but if no mount matches it can return a handle to the original path (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1382-1391`, `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1415-1431`)
- `uio_getDirList()` lists exactly one resolved directory on the native filesystem (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:2007-2045`)
- descriptor and stream opens use `OpenOptions` directly on resolved paths (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1590-1629`, `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1754-1796`)

That means the current Rust layer is strongest for plain stdio-backed directories and weaker for true virtual-filesystem composition. **(Inferred)** — the code proves that only one directory is enumerated and only host paths are opened, but the exact set of user-visible failures this causes has not been exhaustively tested.

## Notable risks and unknowns

### Risk: archive-backed content is likely not fully readable through normal mount resolution

The startup code mounts ZIP/UQM packages (`/Users/acoliver/projects/uqm/sc2/src/options.c:469-480`), but Rust excludes sourceDir-based ZIP mounts from the active resolution registry (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1489-1490`).

Given that `resolve_virtual_mount_path()` filters to active entries only (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:424-427`), archive-backed path resolution is visibly incomplete. **(Inferred)** — the code path is clear, but the runtime impact depends on whether any content is exclusively archive-backed during normal gameplay.

### Risk: SDL read/error handling can misclassify EOF vs error

`sdluio_read()` and `sdluio_write()` rely on `uio_ferror()` to detect real I/O failure after a zero return (`/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdluio.c:90-100`, `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdluio.c:112-123`).

But Rust currently hardcodes `uio_ferror()` to `0` and `uio_feof()` to `1` (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:837-847`). **(Observed)** — the hardcoded returns are directly visible in code. **(Inferred)** — whether this currently causes observable misclassification depends on whether any I/O errors actually occur at runtime.

### Risk: direct export parity is incomplete

Because `uio_fread` still comes from `uio_fread_shim.c` (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/uio_fread_shim.c:6-14`), build/link behavior still depends on a mixed C/Rust export surface.

### Risk: memory ownership remains uneven for stream and listing allocations

- stream buffers can leak on close (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1821-1826`)
- `uio_DirList_free` depends on a global pointer-size registry to avoid incorrect deallocation (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:2141-2156`, `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:2179-2188`)

### Unknown: extent of runtime dependence on `uio_vfprintf` and FileBlock APIs

The Rust bridge exports these APIs, but `uio_vfprintf` is a stub and FileBlock operations are mostly stubbed (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:747-755`, `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:906-962`).

This document does not assert whether production gameplay paths currently depend on them; code here only proves they are not fully ported. **(Unknown)** — requires runtime testing or call-graph analysis to determine.

### Unknown: exact legacy mount-tree semantics needed by callers

The public API exposes relative mount placement (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/io.h:32-37`, `/Users/acoliver/projects/uqm/sc2/src/libs/uio/io.h:78-89`), but the current Rust registry reduces ordering to a sort heuristic (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:347-354`).

The current code proves semantic compression; it does not by itself prove every observable conflict case that may diverge. **(Inferred)** — the sort heuristic is visibly different from explicit placement, but whether any current caller exercises a case where the difference is observable is unknown.

### Unknown: whether `uio_Stream` field layout is accessed directly by C code under Rust mode

The C header defines the `uio_Stream` struct layout (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.h:75-97`), and macros like `uio_INTERNAL` exist that could access fields directly. Whether any C code compiled under `USE_RUST_UIO` actually uses those macros has not been audited. **(Unknown)** — requires an audit of compiled C code to resolve.

## Summary

The file I/O subsystem is partially ported to Rust in the current repository configuration:

- Rust owns most `uio_*` implementations and is the active backend because `USE_RUST_UIO` is enabled (`/Users/acoliver/projects/uqm/sc2/config_unix.h:73-74`)
- the legacy C core `io.c` and `uiostream.c` are explicitly excluded (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/Makeinfo:14-18`, `/Users/acoliver/projects/uqm/sc2/src/libs/uio/io.c:16-18`, `/Users/acoliver/projects/uqm/sc2/src/libs/uio/uiostream.c:16-18`)
- the port boundary is still mixed because `uio_fread` remains C-shimmed (`/Users/acoliver/projects/uqm/sc2/src/libs/uio/uio_fread_shim.c:6-14`)
- startup orchestration, SDL RWops adaptation, and global directory ownership remain C-owned (`/Users/acoliver/projects/uqm/sc2/src/options.c:205-515`, `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/sdl/sdluio.c:43-149`)
- the active Rust implementation is strongest for stdio-backed real filesystem paths, but archive activation, mount ordering fidelity, stream status semantics, FileBlock behavior, and some ownership paths remain incomplete or stubbed (`/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:347-354`, `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1476-1490`, `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:837-889`, `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:906-962`, `/Users/acoliver/projects/uqm/rust/src/io/uio_bridge.rs:1821-1826`)

That is the current evidence-grounded initial state of the file I/O subsystem as checked against code in this repository.