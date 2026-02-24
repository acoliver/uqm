# Functional/Technical Specification: Swap C Memory Allocator to Rust

## 1. Current State

### 1.1 `w_memlib.c` — The C Implementation

**Location:** `sc2/src/libs/memory/w_memlib.c`

The C memory subsystem is a thin wrapper around the standard C library allocator.
It provides four allocation functions and two lifecycle stubs:

| Function | Behavior |
|---|---|
| `HMalloc(size)` | Calls `malloc(size)`. If `malloc` returns `NULL` **and** `size > 0`, logs a fatal error via `log_add(log_Fatal, ...)`, flushes `stderr`, and calls `explode()`. Returns `NULL` for zero-size allocations (standard `malloc` behavior — implementation-defined). |
| `HFree(p)` | Calls `free(p)` unconditionally. `free(NULL)` is defined as a no-op by the C standard, so this is safe. |
| `HCalloc(size)` | Calls `HMalloc(size)` then `memset(p, 0, size)`. Note: does **not** use `calloc()` — it manually zeroes. If `HMalloc` returned `NULL` for `size == 0`, the `memset` on `NULL` is undefined behavior (a latent bug). |
| `HRealloc(p, size)` | Calls `realloc(p, size)`. Same OOM handling as `HMalloc`: fatal on `NULL` return when `size > 0`. |
| `mem_init()` | Stub — returns `true`. |
| `mem_uninit()` | Stub — returns `true`. |

**Error handling:** On OOM, `explode()` is called, which in `DEBUG` builds calls `abort()` (for debugger attachment), and in release builds calls `exit(EXIT_FAILURE)`. This means OOM is always fatal — no caller checks return values from `HMalloc`/`HRealloc` (with one exception: `getchar.c:69` checks the return of `HMalloc` for `NULL`, a dead check since `HMalloc` already aborts on failure).

**Build integration:** `libs/memory/Makeinfo` contains a single line:
```
uqm_CFILES="w_memlib.c"
```
The file is compiled unconditionally — there is no `USE_RUST_*` guard.

### 1.2 `memory.rs` — The Rust Implementation

**Location:** `rust/src/memory.rs`

The Rust memory subsystem mirrors the C API with `#[no_mangle] pub unsafe extern "C"` functions:

| Function | Behavior |
|---|---|
| `rust_hmalloc(size)` | For `size == 0`: allocates 1 byte via `libc::malloc(1)` and returns a non-null pointer. For `size > 0`: calls `libc::malloc(size)`. On `NULL` return: logs via `log_add(LogLevel::User, ...)` and calls `std::process::abort()`. |
| `rust_hfree(ptr)` | Checks `!ptr.is_null()` before calling `libc::free(ptr)`. The explicit null check is redundant (C `free(NULL)` is a no-op) but defensive. |
| `rust_hcalloc(size)` | For `size == 0`: allocates 1 byte, zeroes it. For `size > 0`: `libc::malloc(size)`, fatal on OOM, then `libc::memset(ptr, 0, size)`. |
| `rust_hrealloc(ptr, size)` | For `size == 0`: frees `ptr` (if non-null), then allocates 1 byte. For `size > 0`: `libc::realloc(ptr, size)`, fatal on OOM. |
| `rust_mem_init()` | Logs "Rust memory management initialized." and returns `true`. |
| `rust_mem_uninit()` | Logs "Rust memory management deinitialized." and returns `true`. |

**Additional utility:** `copy_argv_to_c()` — converts a Rust `&[String]` to a C `argc/argv` array. Uses `rust_hmalloc` internally.

### 1.3 The Header Layer — `memlib.h`

**Location:** `sc2/src/libs/memlib.h`

Currently, `memlib.h` declares the functions as plain `extern` prototypes — no macros:

```c
extern bool mem_init (void);
extern bool mem_uninit (void);
extern void *HMalloc (size_t size);
extern void HFree (void *p);
extern void *HCalloc (size_t size);
extern void *HRealloc (void *p, size_t size);
```

Every C source file that calls `HMalloc` etc. includes `libs/memlib.h` and the linker resolves the symbol to `w_memlib.c`. There are **no macros** involved — `HMalloc(s)` is a direct function call, not a macro expansion.

Two additional macro-based call sites exist in headers:
- `uqm/displist.h` defines `AllocQueueTab(pq)` which calls `HMalloc(...)`, `FreeQueueTab(pq)` which calls `HFree(...)`, `AllocLink(pq)` which calls `HMalloc(...)`, and `FreeLink(pq, h)` which calls `HFree(...)`.
- `libs/graphics/font.h` has inline functions that call `HMalloc`, `HCalloc`, and `HFree`.
- `libs/graphics/context.h` defines `AllocContext()` as `HCalloc(sizeof(CONTEXT_DESC))`.
- `libs/decomp/lzh.h` defines `AllocCodeDesc()` as `HCalloc(sizeof(LZHCODE_DESC))`.
- `libs/decomp/lzencode.c` has a macro that calls `HCalloc(...)`.

All of these go through `HMalloc`/`HCalloc`/`HFree` — redirecting those six functions redirects everything.

---

## 2. Approach

### 2.1 Options Evaluated

#### Option A: Header Macro Redirect (RECOMMENDED)

**Mechanism:** Modify `memlib.h` so that when `USE_RUST_MEM` is defined, the function declarations are replaced by macros that redirect to the Rust `extern "C"` functions:

```c
#ifdef USE_RUST_MEM

extern void *rust_hmalloc (size_t size);
extern void  rust_hfree (void *p);
extern void *rust_hcalloc (size_t size);
extern void *rust_hrealloc (void *p, size_t size);
extern bool  rust_mem_init (void);
extern bool  rust_mem_uninit (void);

#define HMalloc(s)     rust_hmalloc(s)
#define HFree(p)       rust_hfree(p)
#define HCalloc(s)     rust_hcalloc(s)
#define HRealloc(p, s) rust_hrealloc(p, s)
#define mem_init()     rust_mem_init()
#define mem_uninit()   rust_mem_uninit()

#else

extern bool mem_init (void);
extern bool mem_uninit (void);
extern void *HMalloc (size_t size);
extern void HFree (void *p);
extern void *HCalloc (size_t size);
extern void *HRealloc (void *p, size_t size);

#endif
```

**Pros:**
- Zero changes to any of the 322+ call sites across 60+ files.
- Exact pattern already used by `rust_audiocore.h` (`#define audio_Sourcei(...) rust_audio_source_i(...)`) and `rust_vcontrol.h` — proven in this codebase.
- Compile-time switch: preprocessor does the redirect, no runtime overhead.
- Both paths remain compilable — remove the `#define` and the C implementation links instead.
- `w_memlib.c` gets a `#error` guard (like `files.c`, `io.c`, `clock.c`, `vcontrol.c`) to catch accidental double-linking.

**Cons:**
- Macros interact with any code that uses `HMalloc` as a token for non-call purposes (function pointer casts, `&HMalloc`). Searched: no call site in the codebase takes `&HMalloc` or uses it as a function pointer. Safe.
- The macro will expand `HMalloc` even in comments or string literals if they happen to look like macro invocations. Irrelevant in practice.

#### Option B: `#error` Guard + Linker Symbol Aliasing

**Mechanism:** Add `#error` to `w_memlib.c`, exclude it from the build via `Makeinfo`, and have the Rust library export symbols named `HMalloc`, `HFree`, etc. directly (via `#[export_name = "HMalloc"]`).

**Pros:**
- Zero call site changes. No macros needed.
- Simplest conceptually — Rust exports the exact symbol the C code expects.

**Cons:**
- Requires the Rust functions to be exported with C-standard names (`HMalloc`) rather than the `rust_` prefixed names they currently have. This means `memory.rs` must be modified to either rename the functions or add alias exports.
- Naming collision risk: if both `w_memlib.o` and the Rust library accidentally end up in the link, you get duplicate symbol errors with no clear diagnostic. The `#error` guard mitigates this, but it only fires if `w_memlib.c` is compiled — if someone just links a stale `.o`, it won't catch it.
- The Rust `rust_hmalloc` function is already called by other Rust code (`copy_argv_to_c`, `main.rs`). Renaming it to `HMalloc` would work (Rust code can call `extern "C"` functions by any name), but it makes Rust-side code less clear.
- Doesn't follow the established pattern in this codebase (other modules use header macros, not symbol aliasing).

#### Option C: Make `w_memlib.c` Call Through to Rust

**Mechanism:** Keep `w_memlib.c` in the build but replace its function bodies with calls to `rust_hmalloc` etc.:

```c
void *HMalloc(size_t size) {
    return rust_hmalloc(size);
}
```

**Pros:**
- Zero call site changes. Header stays the same.
- No macros, no symbol tricks.

**Cons:**
- Adds an extra function call layer (C → Rust → libc). Trivially optimizable by LTO, but conceptually wasteful.
- Still compiles `w_memlib.c` — the whole point of the migration is to stop compiling it.
- Doesn't test the full redirect path (the C linker still resolves `HMalloc` to C code).
- Doesn't match the established pattern. Other modules completely exclude the C file.
- When we eventually want to replace the libc allocator with a Rust-native allocator (e.g., `Box`, arena allocator), we'd need to do the header redirect anyway.

### 2.2 Recommendation: Option A

**Option A** is the clear winner because:

1. **Precedent:** This is exactly how `USE_RUST_AUDIO`, `USE_RUST_INPUT`, `USE_RUST_MIXER`, and `USE_RUST_RESOURCE` work. The pattern is proven.
2. **Zero call site edits** for 322+ references across 60+ files.
3. **Clean separation:** C path and Rust path are mutually exclusive at compile time.
4. **Forward-compatible:** When Rust eventually uses a native allocator instead of `libc::malloc`, only `memory.rs` changes — no C code touched.

### 2.3 Implementation Sketch (Option A)

Files to modify:

| File | Change |
|---|---|
| `sc2/src/libs/memlib.h` | Add `#ifdef USE_RUST_MEM` block with `extern` declarations for `rust_*` functions and `#define` macros |
| `sc2/src/libs/memory/w_memlib.c` | Add `#ifdef USE_RUST_MEM` / `#error` guard at top |
| `sc2/src/libs/memory/Makeinfo` | Conditional: if `USE_RUST_MEM`, set `uqm_CFILES=""` |
| `sc2/src/config_unix.h.in` | Add `@SYMBOL_USE_RUST_MEM_DEF@` |
| `sc2/config_unix.h` | Add `#define USE_RUST_MEM` |
| `sc2/build.vars.in` | Add `USE_RUST_MEM` / `uqm_USE_RUST_MEM` / `SYMBOL_USE_RUST_MEM_DEF` entries |
| `rust/src/memory.rs` | No changes needed — already exports `rust_hmalloc` etc. with `#[no_mangle]` |

Files NOT modified: **Every single call site.** That's the point.

---

## 3. Call Site Catalog

### 3.1 Summary

| Function | Call Sites (excluding declarations/definitions) |
|---|---|
| `HMalloc` | 112 calls across 39 files |
| `HFree` | 160 calls across 44 files |
| `HCalloc` | 17 calls across 14 files |
| `HRealloc` | 10 calls across 7 files |
| `mem_init` | 1 call (uqm.c) |
| `mem_uninit` | 1 call (uqm.c) |
| **Total** | **~301 call sites across ~55 unique files** |

### 3.2 Detailed Catalog by Subsystem

#### Top-Level (`sc2/src/`)

| File | HMalloc | HFree | HCalloc | HRealloc | Total |
|---|---|---|---|---|---|
| `options.c` | 2 | 2 | — | — | 4 |
| `uqm.c` | 1 | 1 | — | 1 | 3 |
|  | | | | | + `mem_init` (1), `mem_uninit` (1) |

#### `libs/sound/`

| File | HMalloc | HFree | HCalloc | HRealloc | Total |
|---|---|---|---|---|---|
| `sound.c` | 1 | 1 | — | — | 2 |
| `sfx.c` | 1 | — | — | — | 1 |
| `stream.c` | — | 2 | 4 | — | 6 |
| `trackplayer.c` | 1 | 3 | 1 | 3 | 8 |
| `audiocore_rust.c` | — | 1 | — | — | 1 |
| `openal/audiodrv_openal.c` | — | 1 | — | — | 1 |
| `mixer/mixer.c` | 2 | 3 | — | — | 5 |
| `mixer/sdl/audiodrv_sdl.c` | — | 1 | — | — | 1 |
| `mixer/nosound/audiodrv_nosound.c` | 1 | 2 | — | — | 3 |
| `decoders/decoder.c` | 2 | 4 | 1 | 2 | 9 |
| `decoders/dukaud.c` | 2 | 2 | — | — | 4 |
| `decoders/modaud.c` | 1 | 1 | — | — | 2 |

#### `libs/graphics/`

| File | HMalloc | HFree | HCalloc | HRealloc | Total |
|---|---|---|---|---|---|
| `font.h` (inline) | 1 | 2 | 1 | — | 4 |
| `drawable.c` | 1 | 1 | 1 | — | 3 |
| `cmap.c` | 1 | 2 | — | — | 3 |
| `gfxload.c` | 4 | 10 | — | — | 14 |
| `tfb_draw.c` | 2 | 1 | — | — | 3 |
| `dcqueue.c` | — | 2 | — | — | 2 |
| `context.h` (macro) | — | — | 1 | — | 1 |
| `sdl/rotozoom.c` | 4 | 4 | — | — | 8 |
| `sdl/sdluio.c` | 1 | 1 | — | — | 2 |
| `sdl/palette.c` | — | 1 | 1 | — | 2 |
| `sdl/canvas.c` | — | — | 1 | — | 1 |

#### `libs/resource/`

| File | HMalloc | HFree | HCalloc | HRealloc | Total |
|---|---|---|---|---|---|
| `direct.c` | 1 | — | — | — | 1 |
| `resinit.c` | 7 | 3 | — | — | 10 |
| `getres.c` | — | 1 | — | — | 1 |

#### `libs/threads/`

| File | HMalloc | HFree | HCalloc | HRealloc | Total |
|---|---|---|---|---|---|
| `thrcommon.c` | 5 | 2 | — | — | 7 |
| `rust_thrcommon.c` | 1 | 1 | — | — | 2 |
| `sdl/sdlthreads.c` | 5 | 5 | — | — | 10 |
| `pthread/posixthreads.c` | 5 | 5 | — | — | 10 |

#### `libs/file/`

| File | HMalloc | HFree | HCalloc | HRealloc | Total |
|---|---|---|---|---|---|
| `dirs.c` | 4 | 5 | — | — | 9 |
| `files.c` | 1 | 2 | — | — | 3 |
| `temp.c` | 1 | — | — | 1 | 2 |

#### `libs/strings/`

| File | HMalloc | HFree | HCalloc | HRealloc | Total |
|---|---|---|---|---|---|
| `strings.c` | 2 | 3 | — | — | 5 |
| `getstr.c` | 5 | 8 | — | 1 | 14 |

#### `libs/input/`

| File | HMalloc | HFree | HCalloc | HRealloc | Total |
|---|---|---|---|---|---|
| `sdl/vcontrol.c` | 5 | 4 | — | — | 9 |
| `sdl/input.c` | 1 | 2 | 1 | — | 4 |

#### `libs/video/`

| File | HMalloc | HFree | HCalloc | HRealloc | Total |
|---|---|---|---|---|---|
| `dukvid.c` | 5 | 7 | — | — | 12 |
| `videodec.c` | 1 | 3 | 1 | — | 5 |
| `vresins.c` | 4 | 4 | — | — | 8 |
| `legacyplayer.c` | — | 1 | 1 | — | 2 |
| `video.c` | — | 1 | 1 | — | 2 |

#### `libs/math/`

| File | HMalloc | HFree | HCalloc | HRealloc | Total |
|---|---|---|---|---|---|
| `random2.c` | 2 | 1 | — | — | 3 |

#### `libs/cdp/`

| File | HMalloc | HFree | HCalloc | HRealloc | Total |
|---|---|---|---|---|---|
| `cdpapi.c` | 2 | 6 | — | 1 | 9 |

#### `libs/decomp/`

| File | HMalloc | HFree | HCalloc | HRealloc | Total |
|---|---|---|---|---|---|
| `lzh.h` (macro) | — | — | 1 | — | 1 |
| `lzencode.c` (macro) | — | — | 1 | — | 1 |

#### `uqm/` (Game Logic)

| File | HMalloc | HFree | HCalloc | HRealloc | Total |
|---|---|---|---|---|---|
| `displist.h` (macros) | 2 | 2 | — | — | 4 |
| `dummy.c` | 1 | 3 | — | — | 4 |
| `setupmenu.c` | 2 | 2 | — | — | 4 |
| `gameopt.c` | 1 | 1 | — | — | 2 |
| `getchar.c` | 2 | 4 | — | — | 6 |
| `battlecontrols.c` | 3 | 1 | — | — | 4 |
| `state.c` | 1 | 1 | — | 1 | 3 |
| `flash.c` | 2 | 3 | — | — | 5 |
| `planets/plangen.c` | 4 | 2 | 1 | — | 7 |
| `planets/pstarmap.c` | 1 | 1 | — | — | 2 |
| `planets/planets.c` | — | 3 | — | — | 3 |
| `supermelee/loadmele.c` | 2 | 3 | — | — | 5 |
| `supermelee/meleesetup.c` | 2 | 2 | — | — | 4 |
| `ships/pkunk/pkunk.c` | 1 | 1 | — | — | 2 |
| `ships/androsyn/androsyn.c` | 1 | 1 | — | — | 2 |
| `ships/sis_ship/sis_ship.c` | 1 | 1 | — | — | 2 |
| `ships/mmrnmhrm/mmrnmhrm.c` | 1 | 1 | — | — | 2 |
| `ships/umgah/umgah.c` | 1 | 1 | — | — | 2 |

#### `getopt/`

| File | HMalloc | HFree | HCalloc | HRealloc | Total |
|---|---|---|---|---|---|
| `getopt.c` | 2 | — | — | — | 2 |

---

## 4. Risk Assessment

### 4.1 Behavioral Equivalence

Both implementations are thin wrappers around `libc::malloc`/`free`/`realloc`. The allocation path is literally the same system allocator. Memory allocated by one implementation can be freed by the other (they both call `free()`). This is the lowest-risk kind of swap.

### 4.2 Zero-Size Allocation Handling

**This is the most significant behavioral difference.**

| Operation | C (`w_memlib.c`) | Rust (`memory.rs`) |
|---|---|---|
| `HMalloc(0)` | Calls `malloc(0)` — returns `NULL` or a unique pointer (implementation-defined per C standard). On most platforms (glibc, macOS), returns a non-null pointer. The `NULL && size > 0` check passes, so no abort. | Explicitly calls `malloc(1)` — always returns non-null. |
| `HCalloc(0)` | Calls `HMalloc(0)` then `memset(p, 0, 0)`. If `malloc(0)` returned `NULL`, the `memset(NULL, 0, 0)` is **undefined behavior**. | Calls `malloc(1)`, zeroes the 1 byte. Always non-null. |
| `HRealloc(p, 0)` | Calls `realloc(p, 0)` — behavior is implementation-defined. On glibc, frees `p` and returns `NULL`. On macOS, may return a non-null pointer. The `NULL && size > 0` check passes (size is 0), so no abort. | Frees `p`, allocates 1 byte. Always non-null. |

**Risk level: LOW.** The Rust implementation is strictly safer — it guarantees non-null returns for zero-size allocations, avoiding the latent UB in the C `HCalloc(0)` path. Callers that check for `NULL` returns will simply never trigger. No caller depends on getting `NULL` from a zero-size allocation.

### 4.3 Error Handling Differences

| Aspect | C (`w_memlib.c`) | Rust (`memory.rs`) |
|---|---|---|
| OOM action | `log_add(log_Fatal, ...)` → `fflush(stderr)` → `explode()` | `log_add(LogLevel::User, ...)` → `std::process::abort()` |
| `explode()` behavior | In `DEBUG`: `abort()`. In release: `exit(EXIT_FAILURE)`. | Always `abort()` regardless of build mode. |
| Log level | `log_Fatal` | `LogLevel::User` |

**Risk:**
- In release builds, the C version calls `exit()` which runs `atexit` handlers and flushes stdio. The Rust version calls `abort()` which does not. This is a behavioral difference on OOM — but OOM is a fatal, unrecoverable state anyway. In practice, this difference is immaterial.
- The log level difference (`log_Fatal` vs `LogLevel::User`) should be harmonized. The Rust implementation should use a fatal-equivalent level for correctness. **This is a minor bug to fix in `memory.rs` before the swap.**

### 4.4 Thread Safety

Both implementations are thread-safe to the same degree: `malloc`/`free`/`realloc` are thread-safe in all modern C runtime libraries. Neither the C nor Rust wrapper adds any synchronization of its own (nor needs to).

**Note:** `sc2/src/libs/threads/sdl/sdlthreads.c` and `pthread/posixthreads.c` contain a comment: `"TODO. The w_memlib uses Mutexes right now, so we can't use HMalloc or HFree in CreateMutex"`. This comment is stale — `w_memlib.c` does not use mutexes. The Rust implementation also does not use mutexes. No action needed.

### 4.5 Cross-Allocation Freeing

During migration, it's possible for memory allocated by the C `HMalloc` (from `w_memlib.c`) to be freed by Rust `rust_hfree`, or vice versa. Since both call `libc::malloc` and `libc::free` on the same heap, this is **fully safe**. No risk.

### 4.6 `HFree(NULL)` Safety

Multiple call sites guard `HFree` with null checks (e.g., `if (pTES->CacheStr) HFree(pTES->CacheStr)` in `getchar.c`). Both the C and Rust implementations handle `NULL` safely:
- C: `free(NULL)` is a no-op per C standard.
- Rust: explicit `!ptr.is_null()` guard before `libc::free`.

No risk.

### 4.7 Macro Interaction Risk

Since Option A uses `#define HMalloc(s) rust_hmalloc(s)`, any code that uses `HMalloc` as a token outside of a function-call context would break. Verified: **no file in the codebase** uses `&HMalloc`, passes `HMalloc` as a function pointer, or references `HMalloc` in any non-call context. All 122 uses are direct `HMalloc(...)` calls. Safe.

---

## 5. Test Plan

### 5.1 Existing Rust Tests

`rust/src/memory.rs` contains 5 tests in a `#[cfg(test)]` module:

| Test | Coverage |
|---|---|
| `test_hmalloc_hfree` | Allocates 100 bytes, writes/reads back, frees |
| `test_hcalloc` | Allocates 100 bytes, verifies zero-filled |
| `test_hrealloc` | Allocates 10, writes, reallocs to 100, verifies preserved data |
| `test_zero_size_allocations` | `rust_hmalloc(0)`, `rust_hcalloc(0)`, `rust_hrealloc(ptr, 0)` — verifies all return non-null |
| `test_copy_argv_to_c` | Tests the argv conversion utility |

These tests validate the Rust functions in isolation. They are necessary but not sufficient.

### 5.2 Verification After the Swap

**Build verification:**
1. Clean build with `USE_RUST_MEM` defined — must compile and link successfully.
2. Clean build with `USE_RUST_MEM` undefined — must compile and link with the C implementation (regression check).
3. Verify `w_memlib.o` is NOT in the build output when `USE_RUST_MEM` is defined.
4. Verify the `#error` fires if `w_memlib.c` is somehow compiled with the flag set.

**Runtime verification:**
1. Launch the game. If it starts at all, the allocator is working — the game allocates thousands of objects during init.
2. Play through the main menu, start a new game, enter orbit, enter melee — exercise all major subsystems.
3. Exit cleanly — verifies no double-free or use-after-free in shutdown paths.

**Stress paths to exercise:**
- Planet generation (`plangen.c`) — heavy allocation with `HMalloc`/`HCalloc` + `HFree`.
- Sound loading/decoding — allocates buffers, reallocates (`HRealloc`), frees.
- Resource loading — `resinit.c` does many small allocations.
- Super Melee team loading — `loadmele.c` allocates index arrays.
- Video playback — `dukvid.c` does large buffer allocations.
- String table loading — `getstr.c` does multiple allocations per string table.

**Automated verification:**
- Run `cargo test` — existing memory tests pass.
- Run the full C build test suite (if any).
- Valgrind/AddressSanitizer run of the game binary to detect any leak or corruption introduced by the swap.

---

## 6. Rollback Plan

The swap is fully reversible with zero code changes at call sites:

1. **Undefine `USE_RUST_MEM`** in `config_unix.h` (or comment out the `#define`).
2. **Rebuild.** The preprocessor will select the `#else` branch in `memlib.h`, restoring the `extern` declarations. The `Makeinfo` conditional will re-include `w_memlib.c` in the build. The linker resolves `HMalloc` → `w_memlib.o` as before.

No source files other than configuration/header files need to change. The rollback is a one-line config change plus rebuild.

If the build system is in an intermediate state (e.g., `Makeinfo` was already updated but `memlib.h` was not), the `#error` in `w_memlib.c` will produce a clear compile error directing the developer to either complete or revert the migration.
