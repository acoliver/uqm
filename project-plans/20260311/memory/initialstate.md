# Memory subsystem initial state

## Scope and purpose

The memory subsystem is still an ABI-compatibility layer, not a Rust-native allocator. Its current responsibility is to provide the historical UQM allocation surface (`HMalloc`, `HFree`, `HCalloc`, `HRealloc`, `mem_init`, `mem_uninit`) to the largely C codebase while routing those calls to Rust-exported functions when `USE_RUST_MEM` is enabled. Evidence:

- the C public allocator surface is defined in `/Users/acoliver/projects/uqm/sc2/src/libs/memlib.h:30-54`
- the Rust replacement exports exactly those backing symbols in `/Users/acoliver/projects/uqm/rust/src/memory.rs:8-99`
- `USE_RUST_MEM` is enabled in `/Users/acoliver/projects/uqm/sc2/config_unix.h:119-120`

This is a thin port because the Rust side still delegates allocation to libc rather than introducing Rust-owned allocation policy or pervasive Rust ownership.

## Responsibilities in the current tree

At present the subsystem does four concrete things:

1. exposes the C allocator API through macros when Rust memory is selected (`/Users/acoliver/projects/uqm/sc2/src/libs/memlib.h:30-44`)
2. performs heap allocation, zeroed allocation, reallocation, and free via libc on the Rust side (`/Users/acoliver/projects/uqm/rust/src/memory.rs:9-77`)
3. provides init/uninit stubs that only log and return success (`/Users/acoliver/projects/uqm/rust/src/memory.rs:85-99`)
4. preserves fatal-on-OOM behavior by logging and aborting on allocation failure for positive-size allocations (`/Users/acoliver/projects/uqm/rust/src/memory.rs:15-21`, `/Users/acoliver/projects/uqm/rust/src/memory.rs:49-55`, `/Users/acoliver/projects/uqm/rust/src/memory.rs:72-77`)

## Current C structure

### Public C boundary

`memlib.h` remains the integration hub for the allocator API. Under `USE_RUST_MEM`, it declares the Rust symbols and remaps the historical names via macros:

- extern declarations for `rust_hmalloc`, `rust_hfree`, `rust_hcalloc`, `rust_hrealloc`, `rust_mem_init`, `rust_mem_uninit`: `/Users/acoliver/projects/uqm/sc2/src/libs/memlib.h:32-37`
- macro remapping of `HMalloc`, `HFree`, `HCalloc`, `HRealloc`, `mem_init`, `mem_uninit`: `/Users/acoliver/projects/uqm/sc2/src/libs/memlib.h:39-44`
- fallback C declarations for the non-Rust build remain in the `#else` arm: `/Users/acoliver/projects/uqm/sc2/src/libs/memlib.h:45-54`

This header is widely included across the C tree, which is the main reason the Rust port can stay thin while still affecting many subsystems. Representative call sites include:

- file copy buffer allocation in `/Users/acoliver/projects/uqm/sc2/src/libs/file/files.c:99-130`
- resource string replacement in `/Users/acoliver/projects/uqm/sc2/src/libs/resource/resinit.c:500-510`
- thread spawn request allocation in `/Users/acoliver/projects/uqm/sc2/src/libs/threads/thrcommon.c:161-169`, `/Users/acoliver/projects/uqm/sc2/src/libs/threads/thrcommon.c:173-181`, `/Users/acoliver/projects/uqm/sc2/src/libs/threads/thrcommon.c:211-229`

The header itself is included from many modules across sound, graphics, resource, file, video, strings, threading, gameplay, and top-level startup code; search evidence shows broad dependence on `/Users/acoliver/projects/uqm/sc2/src/libs/memlib.h` from those C files.

### Legacy C implementation is explicitly excluded

The original C allocator implementation still exists in the tree as `/Users/acoliver/projects/uqm/sc2/src/libs/memory/w_memlib.c`, but it is intentionally blocked from participating when Rust memory is enabled:

- hard compile-time guard: `/Users/acoliver/projects/uqm/sc2/src/libs/memory/w_memlib.c:1-2`
- legacy C implementations of `mem_init`, `mem_uninit`, `HMalloc`, `HFree`, `HCalloc`, `HRealloc` remain below that guard: `/Users/acoliver/projects/uqm/sc2/src/libs/memory/w_memlib.c:32-87`

Those legacy implementations also delegate to libc and preserve fatal-on-OOM semantics, so the current Rust port is behaviorally close to the old C layer rather than a redesign.

## Current Rust structure

### Rust module layout

The subsystem is a single Rust module:

- module exported from the library crate: `/Users/acoliver/projects/uqm/rust/src/lib.rs:7-18`
- module compiled into the Rust launcher binary as well: `/Users/acoliver/projects/uqm/rust/src/main.rs:1-5`
- implementation file: `/Users/acoliver/projects/uqm/rust/src/memory.rs:1-99`

There is no deeper Rust memory subsystem structure, allocator object graph, registry, tagging, diagnostics layer, or ownership abstraction.

### Exported Rust symbols

The Rust module exports six FFI symbols with `#[no_mangle] extern "C"`:

- `rust_hmalloc`: `/Users/acoliver/projects/uqm/rust/src/memory.rs:8-23`
- `rust_hfree`: `/Users/acoliver/projects/uqm/rust/src/memory.rs:29-34`
- `rust_hcalloc`: `/Users/acoliver/projects/uqm/rust/src/memory.rs:40-56`
- `rust_hrealloc`: `/Users/acoliver/projects/uqm/rust/src/memory.rs:62-78`
- `rust_mem_init`: `/Users/acoliver/projects/uqm/rust/src/memory.rs:84-89`
- `rust_mem_uninit`: `/Users/acoliver/projects/uqm/rust/src/memory.rs:95-99`

### Actual implementation behavior

The implementation is intentionally minimal:

- `rust_hmalloc` calls `libc::malloc(size)`, but special-cases `size == 0` by allocating 1 byte and returning a non-null pointer: `/Users/acoliver/projects/uqm/rust/src/memory.rs:9-22`
- `rust_hfree` frees non-null pointers only: `/Users/acoliver/projects/uqm/rust/src/memory.rs:30-33`
- `rust_hcalloc` is not a true `calloc` call; it uses `malloc` plus `memset`, including a 1-byte zeroed allocation for `size == 0`: `/Users/acoliver/projects/uqm/rust/src/memory.rs:41-55`
- `rust_hrealloc` uses `libc::realloc`, but for `size == 0` it frees the old pointer and returns `malloc(1)`: `/Users/acoliver/projects/uqm/rust/src/memory.rs:63-77`
- init/uninit only log and return `true`: `/Users/acoliver/projects/uqm/rust/src/memory.rs:85-99`

### Internal Rust use beyond C-facing export

The memory module is also used internally by Rust audio-heart FFI glue to mimic C allocation patterns:

- local `HMalloc` wrapper calls `crate::memory::rust_hmalloc`: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:54-56`
- local `HFree` wrapper calls `crate::memory::rust_hfree`: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:58-60`
- sample pointer-slot allocation uses that wrapper: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:1198-1199`
- music-ref slot allocation and free also use that wrapper pair: `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:1276-1276`, `/Users/acoliver/projects/uqm/rust/src/sound/heart_ffi.rs:1296-1296`

So the Rust memory layer is not only servicing C macros; some Rust FFI code explicitly relies on it to maintain compatible allocation semantics at mixed-language boundaries.

## Build and configuration wiring

### Configuration switch

The subsystem is selected by compile-time config, not runtime negotiation:

- `USE_RUST_MEM` is defined in `/Users/acoliver/projects/uqm/sc2/config_unix.h:119-120`

That define drives the macro remapping in `memlib.h` and the exclusion guard in `w_memlib.c`.

### Rust crate linkage

The Rust crate is built as a static library and an rlib:

- crate type declaration: `/Users/acoliver/projects/uqm/rust/Cargo.toml:6-8`

This is the mechanism that makes the `rust_*` allocator symbols available to the C side during final linkage.

### Rust build script

The Rust build script does not compile any memory implementation C code. It only compiles `mem_wrapper.c` into the Rust-side static archive:

- compile step: `/Users/acoliver/projects/uqm/rust/build.rs:8-13`
- rerun rule: `/Users/acoliver/projects/uqm/rust/build.rs:15-15`

That matters here because the memory subsystem is not bridged through an adapter C shim analogous to other subsystems. The important boundary is direct symbol linkage plus C preprocessor remapping, not build-script compilation of legacy allocator code.

## C↔Rust integration points

### Primary port boundary: header macro remap

The decisive C→Rust handoff is in `memlib.h`:

- declarations: `/Users/acoliver/projects/uqm/sc2/src/libs/memlib.h:32-37`
- macro mapping: `/Users/acoliver/projects/uqm/sc2/src/libs/memlib.h:39-44`

Every C translation unit that includes `memlib.h` and calls `HMalloc`/`HFree`/`HCalloc`/`HRealloc` is, under this build, compiled to call the Rust-exported symbols instead of the old C implementation.

### Exclusion guard for legacy C implementation

The clearest evidence for the thin-port boundary is the explicit compile failure in the old C allocator file:

- `/Users/acoliver/projects/uqm/sc2/src/libs/memory/w_memlib.c:1-2`

This is strongly evidenced as intentional and enforced, though full proof would require verifying the final-link build recipe (see "Build-system proof" below).

### Runtime lifecycle integration

The Rust launcher invokes the memory init/uninit exports directly around the C entry point:

- memory init before calling into C: `/Users/acoliver/projects/uqm/rust/src/main.rs:35-39`
- C entry point invocation: `/Users/acoliver/projects/uqm/rust/src/main.rs:76-77`
- memory uninit on shutdown: `/Users/acoliver/projects/uqm/rust/src/main.rs:79-82`

At the C-header level, `mem_init()` and `mem_uninit()` are also mapped to the Rust exports, but current code search only found those names in the macro definitions themselves, not active C call sites. Evidence: `/Users/acoliver/projects/uqm/sc2/src/libs/memlib.h:43-44`.

### Direct Rust reuse of the allocator ABI

As noted above, `rust/src/sound/heart_ffi.rs` directly calls `crate::memory::rust_hmalloc` and `crate::memory::rust_hfree` instead of inventing a separate allocation path. That is an internal Rust integration point showing the memory module is treated as the project-wide compatibility allocator for mixed-language FFI objects.

## What is already ported

The following memory responsibilities are ported into Rust today:

- exported allocator symbols for malloc/free/calloc/realloc compatibility: `/Users/acoliver/projects/uqm/rust/src/memory.rs:8-78`
- exported init/uninit hooks: `/Users/acoliver/projects/uqm/rust/src/memory.rs:84-99`
- C API remap onto those symbols: `/Users/acoliver/projects/uqm/sc2/src/libs/memlib.h:30-44`
- compile-time exclusion of the legacy C allocator implementation: `/Users/acoliver/projects/uqm/sc2/src/libs/memory/w_memlib.c:1-2`
- unit coverage for ordinary allocation, zeroed allocation, realloc growth, zero-size cases, and the helper argv allocator: `/Users/acoliver/projects/uqm/rust/src/memory.rs:142-281`

## What remains delegated or indirect

The thin-port character is visible in how much is still delegated:

- actual storage allocation is still done by `libc::malloc`, `libc::realloc`, and `libc::free`, not a Rust allocator abstraction: `/Users/acoliver/projects/uqm/rust/src/memory.rs:12-15`, `/Users/acoliver/projects/uqm/rust/src/memory.rs:32-32`, `/Users/acoliver/projects/uqm/rust/src/memory.rs:44-49`, `/Users/acoliver/projects/uqm/rust/src/memory.rs:67-72`
- zero-fill is manually implemented with `libc::memset` rather than `calloc`: `/Users/acoliver/projects/uqm/rust/src/memory.rs:44-45`, `/Users/acoliver/projects/uqm/rust/src/memory.rs:54-54`
- most callers remain C code using legacy allocation idioms through macros, not Rust ownership: representative examples in `/Users/acoliver/projects/uqm/sc2/src/libs/file/files.c:99-130`, `/Users/acoliver/projects/uqm/sc2/src/libs/resource/resinit.c:503-509`, `/Users/acoliver/projects/uqm/sc2/src/libs/threads/thrcommon.c:163-168`
- init/uninit do not own or release subsystem state yet; they only log success: `/Users/acoliver/projects/uqm/rust/src/memory.rs:85-99`

## Parity observations and gaps

### Close parity with the old C layer

The current Rust implementation intentionally mirrors the shape of the old C layer:

- both expose the same six public entry points: compare `/Users/acoliver/projects/uqm/sc2/src/libs/memlib.h:32-53` with `/Users/acoliver/projects/uqm/rust/src/memory.rs:8-99`
- both use libc allocation primitives and fatal-on-OOM handling: compare `/Users/acoliver/projects/uqm/sc2/src/libs/memory/w_memlib.c:45-86` with `/Users/acoliver/projects/uqm/rust/src/memory.rs:15-21`, `/Users/acoliver/projects/uqm/rust/src/memory.rs:49-55`, `/Users/acoliver/projects/uqm/rust/src/memory.rs:72-77`
- both have stub init/uninit behavior returning success: `/Users/acoliver/projects/uqm/sc2/src/libs/memory/w_memlib.c:32-41`, `/Users/acoliver/projects/uqm/rust/src/memory.rs:85-99`

### Known behavioral differences from legacy C

There are also concrete differences visible in code:

- zero-size allocation behavior differs. The Rust layer guarantees a non-null minimal allocation for `rust_hmalloc(0)`, `rust_hcalloc(0)`, and `rust_hrealloc(ptr, 0)`: `/Users/acoliver/projects/uqm/rust/src/memory.rs:10-13`, `/Users/acoliver/projects/uqm/rust/src/memory.rs:42-46`, `/Users/acoliver/projects/uqm/rust/src/memory.rs:64-69`. The legacy C layer simply passes size through to `malloc`/`realloc`: `/Users/acoliver/projects/uqm/sc2/src/libs/memory/w_memlib.c:47-55`, `/Users/acoliver/projects/uqm/sc2/src/libs/memory/w_memlib.c:78-86`.
- `rust_hcalloc` uses `malloc + memset` rather than `HMalloc + memset` or libc `calloc`: `/Users/acoliver/projects/uqm/rust/src/memory.rs:41-55`. That keeps behavior simple, but it is not structurally identical to the old C implementation at `/Users/acoliver/projects/uqm/sc2/src/libs/memory/w_memlib.c:64-73`.
- Rust OOM exits with `std::process::abort()`: `/Users/acoliver/projects/uqm/rust/src/memory.rs:20-20`, `/Users/acoliver/projects/uqm/rust/src/memory.rs:52-52`, `/Users/acoliver/projects/uqm/rust/src/memory.rs:75-75`. The old C layer called `explode()` after flushing stderr: `/Users/acoliver/projects/uqm/sc2/src/libs/memory/w_memlib.c:50-53`, `/Users/acoliver/projects/uqm/sc2/src/libs/memory/w_memlib.c:81-83`.

These differences may be benign, but they are real parity edges.

## Notable risks and unknowns

### Zero-size semantics may not be C-identical

Because Rust deliberately substitutes 1-byte allocations for zero-size requests, any code that historically relied on platform-specific `malloc(0)` or `realloc(ptr, 0)` behavior is now seeing normalized semantics instead. Evidence: `/Users/acoliver/projects/uqm/rust/src/memory.rs:10-13`, `/Users/acoliver/projects/uqm/rust/src/memory.rs:42-46`, `/Users/acoliver/projects/uqm/rust/src/memory.rs:64-69`.

### The port does not improve ownership safety by itself

Most subsystem allocations are still performed by C callers and managed manually through raw pointers and macro-based lifetime conventions. The memory port changes the callee for allocation, but not the ownership model. Representative evidence:

- raw buffer allocate/free in `/Users/acoliver/projects/uqm/sc2/src/libs/file/files.c:99-130`
- pointer replacement and free in `/Users/acoliver/projects/uqm/sc2/src/libs/resource/resinit.c:503-509`
- raw spawn-request allocations in `/Users/acoliver/projects/uqm/sc2/src/libs/threads/thrcommon.c:163-168`, `/Users/acoliver/projects/uqm/sc2/src/libs/threads/thrcommon.c:175-180`, `/Users/acoliver/projects/uqm/sc2/src/libs/threads/thrcommon.c:213-228`

### Init/uninit are placeholders

`rust_mem_init` and `rust_mem_uninit` currently provide no allocator state management, teardown, leak reporting, or synchronization setup. Evidence: `/Users/acoliver/projects/uqm/rust/src/memory.rs:85-99`. If later subsystems assume allocator-global services live there, that functionality does not yet exist.

### Broad reach means regressions would be cross-cutting

Because `memlib.h` is included across many engine areas, any ABI or behavior mistake in the Rust allocator would affect a large surface area. Search evidence shows includes in file, resource, sound, graphics, threads, strings, video, gameplay, and top-level engine code rooted under `/Users/acoliver/projects/uqm/sc2/src`.

### Build-system proof is indirect beyond the macro/header boundary

The code clearly shows the configuration define, the header remap, the legacy-source exclusion guard, and the Rust exported symbols. What is not shown in the examined files is the full top-level native build recipe that links the Rust static library into the C executable. The staticlib declaration in `/Users/acoliver/projects/uqm/rust/Cargo.toml:6-8` and the live mixed-language usage elsewhere strongly imply that linkage path exists, but that exact final-link wiring was not established from the files reviewed for this document.

## Initial-state conclusion

The memory subsystem is currently ported only at the allocator ABI seam. The old C allocator implementation is intentionally excluded when `USE_RUST_MEM` is enabled (`/Users/acoliver/projects/uqm/sc2/src/libs/memory/w_memlib.c:1-2`), and the public allocator macros in `memlib.h` are redirected to six Rust-exported functions (`/Users/acoliver/projects/uqm/sc2/src/libs/memlib.h:32-44`, `/Users/acoliver/projects/uqm/rust/src/memory.rs:8-99`).

However, the subsystem remains thinly ported because the Rust implementation is still just a libc wrapper with logging and abort behavior, while the wider engine continues to allocate and free memory through legacy C patterns. The main port boundary is strongly evidenced to be real and enforced, but narrow: Rust owns the allocator entry points; libc still owns the actual heap behavior; and the rest of the engine still mostly owns memory-management policy.
