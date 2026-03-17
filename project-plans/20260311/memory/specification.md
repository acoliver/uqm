# Memory Subsystem — Functional and Technical Specification

This document specifies the desired end state of the Rust memory subsystem. It defines what the subsystem does, what it owns, what contracts it must honor, how errors are handled, and how it integrates with the rest of the UQM engine.

This is not an implementation plan. It does not prescribe task ordering, migration strategy, or incremental delivery milestones.

References to current code and implementation details are explanatory examples illustrating the contract, not assertions that the entire end-state contract is already implemented. Sections that describe current launcher behavior or integration preferences are marked as such.

---

## 1. Purpose and Scope

The memory subsystem provides the project-wide C ABI allocation surface. It replaces the legacy C allocator (`w_memlib.c`) when `USE_RUST_MEM` is defined. All C translation units that include `memlib.h` resolve `HMalloc`/`HFree`/`HCalloc`/`HRealloc` to this subsystem. Rust FFI modules that need C-compatible allocations call the subsystem's exported functions directly.

Rust-native allocations (`Box`, `Vec`, `String`, etc.) use the Rust global allocator and are outside the subsystem's scope. The subsystem is not the project-wide heap allocation surface for all code — it is the project-wide C ABI allocation surface.

The subsystem's scope is deliberately narrow: it owns the allocation/deallocation entry points and their behavioral contracts, lifecycle hooks, and diagnostic integration. It does not own memory-management policy at the call-site level — callers are responsible for their own allocation patterns, buffer sizing, and lifetime management.

---

## 2. Subsystem Boundaries

### 2.1 What the Memory Subsystem Owns

- **The six public allocator entry points**: `HMalloc`, `HFree`, `HCalloc`, `HRealloc`, `mem_init`, `mem_uninit` (exposed through the ABI as `rust_hmalloc`, `rust_hfree`, `rust_hcalloc`, `rust_hrealloc`, `rust_mem_init`, `rust_mem_uninit`).
- **Behavioral contracts for those entry points**: zero-size semantics, OOM response, null-pointer handling, zero-fill guarantees, and reallocation data-preservation guarantees.
- **Lifecycle hooks**: subsystem initialization and shutdown, including any future allocator state, diagnostics, or teardown behavior.
- **Diagnostic integration**: OOM logging through the project logging subsystem.

### 2.2 What the Memory Subsystem Does NOT Own

- **Caller allocation policy**: the subsystem does not dictate when, how much, or how often callers allocate. Buffer sizing, pool strategies, and allocation patterns are caller responsibilities.
- **Ownership tracking at call sites**: the subsystem does not enforce or track which component owns a given allocation. Callers manage their own pointer lifetimes.
- **The Rust standard library allocator**: Rust-native allocations via `Box`, `Vec`, `String`, etc. use the Rust global allocator (which defaults to the system allocator). The memory subsystem does not intercept or replace the Rust global allocator.
- **Platform memory services**: virtual memory, memory-mapped I/O, and OS-level memory management are outside scope.
- **The legacy C implementation**: `w_memlib.c` is excluded from compilation when `USE_RUST_MEM` is defined and is not part of the end-state subsystem.

### 2.3 Dual-Allocator Landscape

The end-state engine has two independent allocation paths:

| Path | Allocator | Used By |
|---|---|---|
| C ABI allocator (`HMalloc`/`HFree` family) | This subsystem | All C code via `memlib.h` macros; Rust FFI code that must produce C-compatible allocations |
| Rust standard allocator (`Box`, `Vec`, etc.) | Rust global allocator (system default) | All Rust-native code |

These two paths are independent. Memory allocated through `HMalloc` must be freed through `HFree`. Memory allocated through Rust's standard allocator must be freed through Rust's standard allocator. Cross-freeing between the two paths is undefined behavior.

The memory subsystem is responsible only for the first path. The second path is outside its scope.

### 2.4 Allocator-Family Documentation at FFI Boundaries

Engine FFI APIs that transfer ownership of allocated memory and rely on this allocator family must document which allocator family owns the result. Specifically:

- Functions that return owned memory must state which deallocator the caller should use (e.g., "returned buffer must be released with `HFree`").
- Functions that accept memory to later free must state which deallocator the callee will use.
- Functions that copy input must document that the caller retains ownership of the original.

This is a program-level integration obligation for APIs that use this allocator family, not something the memory subsystem implementation can satisfy or enforce in isolation. The memory subsystem's dual-allocator landscape makes this documentation essential for correctness.

---

## 3. Exported ABI Surface

### 3.1 Symbol Table

The subsystem exports exactly six `extern "C"` symbols with `#[no_mangle]` linkage:

| Symbol | C Declaration | Purpose |
|---|---|---|
| `rust_hmalloc` | `void *rust_hmalloc(size_t size)` | Allocate `size` bytes of uninitialized heap memory |
| `rust_hfree` | `void rust_hfree(void *p)` | Free a previously allocated block |
| `rust_hcalloc` | `void *rust_hcalloc(size_t size)` | Allocate `size` bytes of zero-initialized heap memory |
| `rust_hrealloc` | `void *rust_hrealloc(void *p, size_t size)` | Resize a previously allocated block to `size` bytes |
| `rust_mem_init` | `bool rust_mem_init(void)` | Initialize the memory subsystem |
| `rust_mem_uninit` | `bool rust_mem_uninit(void)` | Shut down the memory subsystem |

### 3.2 C Header Mapping

When `USE_RUST_MEM` is defined, `memlib.h` maps the historical allocator names to the Rust symbols via preprocessor macros:

| Macro | Expands To |
|---|---|
| `HMalloc(s)` | `rust_hmalloc(s)` |
| `HFree(p)` | `rust_hfree(p)` |
| `HCalloc(s)` | `rust_hcalloc(s)` |
| `HRealloc(p, s)` | `rust_hrealloc(p, s)` |
| `mem_init()` | `rust_mem_init()` |
| `mem_uninit()` | `rust_mem_uninit()` |

This mapping is the established and compatibility-critical mechanism by which existing C code reaches the Rust allocator. The macro signatures, parameter types, and return types must not change — they are the primary compatibility contract.

**Note on `HCalloc`**: This project's `HCalloc` takes a single byte-count argument (`HCalloc(size)`), unlike ISO C `calloc(count, size)` which takes separate count and element-size arguments. This is the historical API convention and must be preserved.

### 3.3 Rust Internal Access

Rust code within the crate may call the memory subsystem's exported functions directly. This is the established pattern for Rust FFI modules that need to produce allocations compatible with C callers — for example, the audio-heart FFI layer uses local `HMalloc`/`HFree` wrapper functions that delegate to the memory subsystem.

This internal-access pattern is part of the subsystem's contract: the exported functions must be callable from both C (via linker symbols) and from Rust (via the crate's module system) with identical behavior.

---

## 4. Allocator API Behavior

### 4.1 `rust_hmalloc(size)` — Uninitialized Allocation

**Preconditions**: None. All `size` values including zero are valid.

**Behavior**:

- If `size > 0`: allocate at least `size` bytes of heap memory. The contents of the returned memory are uninitialized (indeterminate). Return a non-null pointer to the allocated block.
- If `size == 0`: return a valid, non-null pointer that can be passed to `rust_hfree`. The allocation must have a minimum backing size of 1 byte so that the pointer is legally dereferenceable for at least one byte, though callers must not depend on being able to use the returned memory for storage when they requested zero bytes.
- If allocation fails for a positive-size request: this is an unrecoverable error. See §6 (OOM Semantics).
- If allocation fails for a zero-size request (the 1-byte fallback): this is also an unrecoverable error. See §6.

**Postconditions**: Returns a non-null `*mut c_void`. The pointer is suitably aligned for any object type supported by `malloc` under the platform C ABI (at least `alignof(max_align_t)`).

**Rationale for zero-size behavior**: The C standard leaves `malloc(0)` implementation-defined — it may return null or a unique pointer. The Rust subsystem normalizes this to always return a non-null pointer. This is a deliberate, approved deviation from the legacy C implementation (which passed `size` through to `malloc` unmodified). The normalization eliminates a class of platform-dependent null-pointer bugs and matches Rust's own zero-sized-type allocation philosophy.

### 4.2 `rust_hfree(ptr)` — Deallocation

**Preconditions**: `ptr` was previously returned by `rust_hmalloc`, `rust_hcalloc`, or `rust_hrealloc` and has not been freed since that allocation. Alternatively, `ptr` is null.

**Behavior**:

- If `ptr` is non-null: free the memory block. The pointer becomes invalid after this call.
- If `ptr` is null: no operation. This is safe and must not crash, log, or produce side effects.

**Postconditions**: None. The freed memory must not be accessed.

**Undefined behavior**: Passing a pointer not obtained from this subsystem's allocator functions, or passing an already-freed pointer, is undefined behavior. The subsystem is not required to detect or gracefully handle double-free.

### 4.3 `rust_hcalloc(size)` — Zero-Initialized Allocation

**Preconditions**: None. All `size` values including zero are valid.

**Behavior**:

- If `size > 0`: allocate at least `size` bytes of heap memory. Every byte of the returned memory must be zero. Return a non-null pointer.
- If `size == 0`: return a valid, non-null pointer (same zero-size semantics as `rust_hmalloc(0)`). The minimum backing byte must be zero.
- OOM handling: same as `rust_hmalloc`. See §6.

**Postconditions**: Returns a non-null `*mut c_void`. All bytes in the range `[ptr, ptr+size)` are zero. The pointer is suitably aligned for any object type supported by `malloc` under the platform C ABI.

**Note on implementation**: The subsystem may use any strategy to produce zero-filled memory (libc `calloc`, `malloc` + `memset`, Rust allocation + zero-fill, etc.). The only observable contract is that the returned memory is zero-filled.

### 4.4 `rust_hrealloc(ptr, size)` — Reallocation

**Preconditions**: `ptr` was previously returned by `rust_hmalloc`, `rust_hcalloc`, or `rust_hrealloc` and has not been freed since that allocation. Alternatively, `ptr` is null.

**Behavior**:

- If `ptr` is non-null and `size > 0`: resize the allocation to at least `size` bytes. Data in the range `[0, min(old_size, size))` is preserved. The contents of any newly allocated bytes (when growing) are uninitialized. The returned pointer may or may not equal `ptr`. If the returned pointer differs from `ptr`, the old pointer is invalidated.
- If `ptr` is non-null and `size == 0`: free the old allocation and return a valid non-null pointer (same zero-size semantics as `rust_hmalloc(0)`). The old pointer is invalidated.
- If `ptr` is null and `size > 0`: equivalent to `rust_hmalloc(size)`.
- If `ptr` is null and `size == 0`: equivalent to `rust_hmalloc(0)`.
- OOM handling for positive-size requests: same as `rust_hmalloc`. The original block is not freed on OOM — the process terminates before the caller could observe the failure. See §6.

**Postconditions**: Returns a non-null `*mut c_void`. If `size > 0`, the allocation contains at least `size` bytes with the preserved prefix intact.

**Realloc-to-zero as free**: The `realloc(ptr, 0)` case explicitly frees the old block and returns a fresh minimal allocation. This differs from some C library implementations that return null for `realloc(ptr, 0)`. Callers can rely on the returned pointer being non-null.

---

## 5. Zero-Size Allocation Semantics

Zero-size allocation is a cross-cutting concern that deserves explicit treatment because it represents a deliberate parity deviation.

### 5.1 Normalization Rule

All three allocation functions (`rust_hmalloc`, `rust_hcalloc`, `rust_hrealloc`) normalize zero-size requests to a minimum 1-byte allocation. The returned pointer is:

- Non-null
- Valid for passing to `rust_hfree`

The current implementation produces distinct pointers for concurrently live zero-size allocations as a side effect of delegating to libc `malloc(1)`. No caller dependence on pointer-identity distinguishability for zero-size allocations has been identified, so this is treated as an implementation property rather than a guaranteed contract. Future implementations that use a different internal strategy for zero-size normalization are permitted provided the non-null and free-safety properties are preserved.

### 5.2 Parity Deviation

The legacy C implementation (`w_memlib.c`) passes `size` through to `malloc`/`realloc` unmodified. On platforms where `malloc(0)` returns null, the legacy code could return null for zero-size requests without triggering the OOM abort (because the OOM check is `p == NULL && size > 0`).

The Rust subsystem intentionally changes this behavior. Zero-size requests always succeed and always return non-null. This is an approved deviation that:

- Eliminates platform-dependent behavior
- Prevents null-pointer dereferences in callers that do not expect null from a "successful" allocation
- Matches the semantics expected by Rust FFI consumers

### 5.3 Caller Obligations

Callers who request zero bytes must still free the returned pointer. The returned pointer must not be dereferenced for read or write beyond the allocated size (which is nominally zero, though the implementation allocates one byte). In practice, callers should treat zero-size allocations as sentinels or placeholders, not as usable storage.

---

## 6. OOM Semantics

### 6.1 Policy: Fatal Termination

Out-of-memory is treated as an unrecoverable error. When any allocation function (`rust_hmalloc`, `rust_hcalloc`, `rust_hrealloc`) fails to obtain memory for a request, the subsystem:

1. Attempts to log a fatal-level message through the project logging subsystem, identifying which function failed (e.g., `"HMalloc() FATAL: out of memory."`). This logging is best-effort — it is not guaranteed to succeed if the runtime is in a degraded state.
2. Terminates the process immediately via a fatal mechanism that does not permit caller recovery or stack unwinding.

There is no recovery path, fallback allocation strategy, or error return code. Callers are not required to check for null returns — they will never observe one.

### 6.2 Termination Mechanism

The process is terminated via a fatal, no-unwind termination path. The current implementation uses `std::process::abort()`, which produces an immediate abnormal termination without unwinding the stack or running destructors. An alternative fatal termination mechanism that provides better diagnostics while remaining no-unwind and non-recoverable would also satisfy this contract.

**Parity note**: The legacy C implementation called `explode()` (a project-specific crash function) after `fflush(stderr)`. The observable behavior (process death on OOM) is preserved; the mechanism differs. This is an approved deviation.

### 6.3 Scope of OOM Detection

OOM detection applies to all positive-size allocation requests and to the internal 1-byte allocations used for zero-size normalization. If even the 1-byte fallback fails, the process terminates.

### 6.4 Implications for Callers

Because OOM is fatal, all allocation functions have a contract that says "returns non-null or does not return." Callers need not write null-check error-handling code after allocation calls. This matches the legacy C codebase's conventions — existing C callers do not check `HMalloc` return values.

### 6.5 Logging Availability and Failure Ordering

OOM diagnostic logging is best-effort. Specifically:

- If the logging subsystem is unavailable or itself in a degraded state, process termination still occurs.
- The subsystem should avoid introducing recursive allocation dependencies in OOM reporting (e.g., the OOM handler must not itself allocate in a way that could trigger a recursive OOM).
- Logging availability is not a prerequisite for compliant OOM handling — the hard requirement is fatal termination, not successful logging.

---

## 7. Lifecycle Hooks

### 7.1 `rust_mem_init()` — Initialization

**Called by**: The engine launcher before calling the C entry point. Also mapped to `mem_init()` in `memlib.h` for any C code that calls it (though no active C call sites currently invoke `mem_init()` directly).

**Behavior in the end state**:

- Perform any one-time subsystem initialization required before allocations begin.
- If the subsystem requires no initialization state (i.e., allocation functions are stateless wrappers), this function is a success-returning no-op with optional informational logging.
- Return `true` on success.
- Return `false` on failure. If initialization fails, the caller (the launcher) must treat this as a fatal startup error and exit the process.

**Idempotency**: `rust_mem_init` must not fail or produce incorrect behavior if called more than once. A second call after a successful first call is a no-op returning `true`.

**Ordering**: In the current launcher architecture, `rust_mem_init` is called before the C entry point and before other Rust subsystem init calls. This is the current integration pattern. The hard contract is that the allocation ABI must be available before any caller attempts to use it; the specific ordering relative to other subsystems is a launcher/harness responsibility, not a universal subsystem invariant.

### 7.2 `rust_mem_uninit()` — Shutdown

**Called by**: The engine launcher after the C entry point returns.

**Behavior in the end state**:

- Perform any subsystem teardown: release allocator-internal state, flush diagnostics, finalize any tracking or reporting.
- If the subsystem has no teardown state, this function is a success-returning no-op with optional informational logging.
- Return `true` on success.
- Return `false` on failure. A failed uninit produces a warning but does not prevent process exit.

**Post-uninit allocations**: Behavior of allocation functions after `rust_mem_uninit` has been called is undefined. The shutdown sequence must ensure no allocations occur after uninit. Responsibility for correct shutdown sequencing lies with the launcher / runtime harness, not with the memory subsystem itself.

### 7.3 Future Lifecycle Responsibilities

The lifecycle hooks are extension points. In the end state, they may optionally:

- Initialize and tear down allocator-internal diagnostics or tracking state
- Log allocation statistics (total allocations, peak usage, etc.)
- Perform leak detection (report allocations that were never freed)
- Set up or tear down any thread-safety infrastructure the allocator requires

None of these capabilities are required for functional correctness. They are permitted extensions that must not break the core allocation contracts if implemented.

---

## 8. Ownership and Lifetime Semantics

This section describes the ownership model as an interface-level semantic contract. The subsystem itself does not track, enforce, or verify ownership at runtime (see §8.2). The "shall" language below defines the API contract callers must follow, not behavior the allocator implementation enforces.

### 8.1 Manual-Lifetime Interface Contract

The memory subsystem provides a raw allocator with manual lifetime management:

- A successful allocation result is considered caller-owned for API-contract purposes. The caller holds the obligation to eventually release or transfer that allocation through the compatible deallocator (`HFree` or `rust_hfree`).
- Which component is responsible for freeing a given block is a call-site convention between caller and callee — not something observed or enforced by the allocator.
- Pointers may be borrowed or shared temporarily between components, but each allocation must ultimately be released exactly once through the compatible deallocator.
- Passing a pointer from one component to another may transfer the release obligation, but this is a convention between those components — the allocator is not aware of it.

This model applies to all callers: C code, Rust FFI code, and any future callers.

### 8.2 No Runtime Ownership Enforcement

The memory subsystem does not track which component owns which allocation. It does not detect:

- Use-after-free
- Double-free
- Memory leaks
- Cross-allocator freeing (e.g., `HFree` on a `Box`-allocated pointer)

These are all undefined behavior from the subsystem's perspective. Call-site correctness is the caller's responsibility.

### 8.3 Cross-Language Allocation Contract

Allocations made through the memory subsystem (`HMalloc`/`HCalloc`/`HRealloc`) may be freely passed between C and Rust code. The only requirement is that they are freed through `HFree` (or equivalently, `rust_hfree`). This is the mechanism that enables mixed-language FFI: a C caller allocates via `HMalloc`, passes the pointer to Rust, and Rust later frees it via `rust_hfree` — or vice versa.

Allocations made through Rust's standard allocator (`Box`, `Vec`, etc.) must not be freed through `HFree`, and allocations made through `HMalloc` must not be freed through Rust's standard allocator. The two allocation pools are independent and incompatible.

### 8.4 Alignment Guarantees

All allocations returned by the memory subsystem must be suitably aligned for any object type supported by `malloc` under the platform C ABI. This means the returned pointer is aligned to at least `alignof(max_align_t)`, which is typically 8 or 16 bytes depending on platform.

---

## 9. Error Handling

### 9.1 Error Categories

The memory subsystem has a deliberately minimal error surface:

| Condition | Response |
|---|---|
| Allocation failure (OOM) | Fatal: log (best-effort) and terminate (§6) |
| `HFree(NULL)` | Safe no-op |
| `HRealloc(NULL, size)` | Equivalent to `HMalloc(size)` |
| Initialization failure | Return `false` from `rust_mem_init` |
| Shutdown failure | Return `false` from `rust_mem_uninit` |
| Double-free | Undefined behavior (not detected) |
| Use-after-free | Undefined behavior (not detected) |
| Cross-allocator free | Undefined behavior (not detected) |

### 9.2 No Partial Failure

Because OOM is fatal, the allocation functions have no partial-failure mode. They either succeed completely or the process terminates. This eliminates the need for error codes, error enums, or `Result` types at the allocator API boundary.

### 9.3 Logging

All detected error conditions are logged through the project logging subsystem before the process terminates or the error is returned, provided the logging subsystem is available. The log messages must identify the specific function that failed. OOM logging is best-effort — if the logging subsystem is unavailable or degraded, the subsystem must still terminate the process (see §6.5). Init/uninit failure logging is likewise best-effort: if the logging subsystem has not been initialized or is otherwise unavailable at the time of the failure, the subsystem should attempt to log but must not depend on logging availability for correct error reporting or return behavior.

---

## 10. Concurrency

### 10.1 Allocation and Free Thread Safety

The four allocation/deallocation entry points (`rust_hmalloc`, `rust_hfree`, `rust_hcalloc`, `rust_hrealloc`) must be safe to call concurrently from any thread. The memory subsystem must not require callers to hold external locks before allocating or freeing memory.

In the current design, this requirement is satisfied by delegating to the platform's libc allocator, which is itself thread-safe. If a future implementation introduces allocator-internal state (tracking tables, diagnostic counters, etc.), that state must be protected against concurrent access.

### 10.2 Lifecycle Hook Thread Safety

`rust_mem_init` and `rust_mem_uninit` are single-threaded lifecycle operations with sequencing preconditions. They are called from the main thread during startup and shutdown respectively. They are not required to be safe for concurrent invocation — the launcher guarantees single-threaded access during these calls.

However, `rust_mem_init` must complete before any threads are spawned that might allocate, and `rust_mem_uninit` must not be called while other threads are still allocating.

### 10.3 Optional Diagnostics

Any optional diagnostics or tracking that require quiescence must either synchronize internally or be restricted to init/uninit phases. Such diagnostics must not introduce thread-safety hazards in the allocation/free hot path.

---

## 11. Integration Points

### 11.1 C Code via `memlib.h`

This is the primary integration point. When `USE_RUST_MEM` is defined, every C file that includes `memlib.h` and calls `HMalloc`, `HFree`, `HCalloc`, or `HRealloc` resolves those calls to the Rust subsystem's exported symbols. The affected surface spans:

- File I/O (`files.c`)
- Resource management (`resinit.c`)
- Threading (`thrcommon.c`)
- Sound, graphics, video, strings, gameplay, and top-level engine code

The breadth of this integration means any ABI or behavioral change in the memory subsystem is a cross-cutting change that affects the entire engine.

### 11.2 Rust FFI Modules

Rust FFI modules that need to produce allocations compatible with C callers import and call the memory subsystem's exported functions directly. Currently known consumer:

- **Audio-heart FFI** (`sound/heart_ffi.rs`): uses `HMalloc`/`HFree` wrapper functions that delegate to the memory subsystem for allocating pointer-sized slots that are later freed by C or Rust code.

Other Rust FFI modules may consume the memory subsystem in the future as additional subsystems are ported. Any such consumer must follow the same pattern: allocate through the C ABI allocator family when producing memory intended for cross-language exchange, and document which allocator family owns the result (see §2.4).

### 11.3 Engine Launcher

The launcher calls `rust_mem_init()` before the C entry point and `rust_mem_uninit()` after it returns. This is the lifecycle integration point. In the current launcher implementation, init failure is treated as a fatal startup error (exits with code 1) and uninit failure is treated as a non-fatal warning. This behavior is observed in the reviewed launcher code but is a launcher-level policy choice, not a subsystem-imposed requirement. The subsystem's contract is limited to returning a boolean status from each hook; the launcher decides how to act on that status.

### 11.4 Logging Subsystem

The memory subsystem depends on the project logging subsystem for OOM messages and lifecycle status messages. In the reviewed launcher code, the logging subsystem appears to be initialized before `rust_mem_init` is called. This is an observed integration pattern but not a universal subsystem invariant — the hard requirement is that OOM logging is best-effort and that logging unavailability does not prevent correct OOM termination (see §6.5). Init/uninit failure logging is similarly best-effort and conditional on logging availability (see §9.3). A launcher that initialized logging after memory, or that omitted a separate logging init, would still satisfy the allocator contract provided OOM termination remains functional.

### 11.5 Build Configuration

- **C side**: `USE_RUST_MEM` is defined in the per-platform config header (e.g., `config_unix.h`). This define drives the macro remapping in `memlib.h` and the `#error` exclusion guard in `w_memlib.c`.
- **Rust side**: The memory module is unconditionally compiled as part of the crate. There is no Cargo feature gate — the module is always present.
- **Linkage**: The Rust crate is built as a `staticlib` (and `rlib`). The six `#[no_mangle] extern "C"` symbols are available to the C linker through the static library archive.

---

## Appendix A. `copy_argv_to_c` Utility (Non-Normative)

This section documents a supporting utility colocated in the memory module. It is not part of the public C ABI or the allocator contract. It is documented here only because it is colocated with the module and exercises allocator-family boundaries, not because it is part of subsystem scope.

### A.1 Purpose

The `copy_argv_to_c` function is a utility that converts a Rust `&[String]` into a C-compatible `argv`-style null-terminated pointer array. It is used by the launcher to prepare arguments for the C entry point.

### A.2 Behavior

- Converts each Rust `String` to a `CString` and obtains a raw `*mut i8` pointer.
- Allocates a pointer array via `rust_hmalloc` large enough for `argv.len() + 1` pointers.
- Copies the string pointers into the array and null-terminates it.
- Returns the array pointer and a `Vec` of the individual string pointers (for lifetime management).

### A.3 Ownership

- The pointer array is allocated via `rust_hmalloc` and must be freed via `rust_hfree`.
- The individual string pointers are `CString::into_raw()` results. These are Rust-owned allocations made through the Rust standard allocator, not through `HMalloc`. They must be reclaimed by reconstructing them via `CString::from_raw()` (or an equivalent Rust-side helper that does so), never via `libc::free` or `HFree`. Even on platforms where the Rust allocator and libc share the same underlying allocator, using `libc::free` on a `CString::into_raw()` pointer violates Rust's allocator-family ownership rules.
- The caller is responsible for both cleanup paths.

### A.4 Scope

This utility is a convenience function colocated in the memory module because it uses `rust_hmalloc`. It is not part of the public C ABI. It is `#[allow(dead_code)]` because it may not be used in all build configurations.

---

## 12. Parity Policy

### 12.1 Parity Baseline

The Rust memory subsystem's behavioral baseline is the legacy C implementation in `w_memlib.c`. The two implementations must produce equivalent behavior for all externally relied-upon semantics in non-deviated cases. Specifically, parity is required for:

- **ABI compatibility**: symbol signatures, parameter types, and return types remain compatible with the public header
- **Success/failure behavior**: allocation requests succeed or fatally terminate under the same conditions
- **Zero-fill guarantees**: `HCalloc` returns zero-initialized memory
- **Reallocation preservation**: `HRealloc` preserves data up to the lesser of old and new sizes
- **Null-free safety**: `HFree(NULL)` is a safe no-op
- **Fatal-on-OOM policy**: OOM terminates the process rather than returning null
- **Lifecycle hook compatibility**: `mem_init`/`mem_uninit` return boolean status with the same success/failure semantics

Implementation-internal behavior (logging formatting and timing, internal call chains, exact termination mechanism, allocator address reuse patterns, interaction with debugging tools) may differ without constituting a parity violation, provided none of the above externally relied-upon semantics change.

### 12.2 Approved Deviations

The following deviations from the legacy C behavior are intentional and approved:

| Deviation | Legacy C Behavior | Rust Behavior | Rationale |
|---|---|---|---|
| Zero-size allocation | Platform-dependent (`malloc(0)` may return null) | Always returns non-null (1-byte minimum) | Eliminates platform-dependent behavior and null-pointer bugs |
| `HRealloc(ptr, 0)` | Platform-dependent (`realloc(ptr, 0)` may free and return null) | Frees old block, returns non-null minimal allocation | Consistent with zero-size normalization |
| OOM termination mechanism | `fflush(stderr)` then `explode()` | Fatal no-unwind termination | Equivalent effect (process death); mechanism differs |
| `HCalloc` implementation | `HMalloc(size)` + `memset` (routes through `HMalloc`'s own OOM check) | Direct `malloc` + `memset` (or equivalent) | Observable behavior is identical (zero-filled memory or abort); internal call chain may differ |
| `HFree(NULL)` | Calls `free(NULL)` (safe per C standard but unconditional call) | Explicit null check; no `free` call on null | Equivalent behavior; avoids unnecessary function call |

### 12.3 Unintentional Deviations

Any behavioral difference not listed in §12.2 is unintentional and must be treated as a bug. If discovered, it should be evaluated for impact and either fixed (to match legacy behavior) or formally approved and added to §12.2.

---

## 13. Constraints

### 13.1 No Panics Across FFI

The six exported functions must never panic. A panic that unwinds across an `extern "C"` boundary is undefined behavior in Rust. All code paths must either succeed or terminate the process via a fatal no-unwind mechanism — never via panic.

### 13.2 No Rust Global Allocator Override

The memory subsystem must not install itself as the Rust global allocator. Doing so would route all Rust-native allocations (`Box`, `Vec`, `String`, etc.) through the C-compatible allocator path, which would complicate debugging, conflict with third-party crate expectations, and provide no practical benefit.

### 13.3 Stable Exported Symbols

The names of the six exported `extern "C"` symbols (`rust_hmalloc`, `rust_hfree`, `rust_hcalloc`, `rust_hrealloc`, `rust_mem_init`, `rust_mem_uninit`) are stable interfaces. Renaming them would break C linkage via the `memlib.h` extern declarations.

Internal module organization (module paths, crate layout, which files contain the implementations) is a current implementation detail, not an architectural contract. Changes to internal organization are permitted provided the exported symbols and their behavior remain unchanged.

### 13.4 Binary Availability

The memory module must be compiled into the static library (for C linkage) and available to the binary crate (for the launcher). The specific mechanism for achieving this (e.g., module declarations in `lib.rs` and `main.rs`) is an implementation detail.

---

## 14. Testing Requirements

### 14.1 Unit Test Coverage

The memory subsystem must have unit tests covering:

- Positive-size allocation and deallocation round-trip
- Zero-initialized allocation (verify all bytes are zero)
- Reallocation with data preservation (verify old data survives realloc)
- Zero-size allocation for all three allocation functions (verify non-null return)
- `HFree(NULL)` safety
- `HRealloc(NULL, size)` equivalence to `HMalloc(size)`
- `HRealloc(ptr, 0)` behavior (old pointer freed, non-null return)
- `copy_argv_to_c` round-trip (string content preservation, null termination)

### 14.2 Mixed-Language Integration Tests

Because the memory subsystem is a cross-language boundary with allocator-family correctness implications, the project test suite should include a small set of dedicated mixed-language integration tests exercising:

- C allocates with `HMalloc`, Rust frees with `rust_hfree`
- Rust allocates with `rust_hmalloc`, C frees with `HFree`
- `HRealloc(NULL, size)` and `HRealloc(ptr, 0)` exercised across the C/Rust seam
- Launcher init/uninit sequencing smoke test

These tests are a project-level integration obligation — they verify boundary-specific risks that single-language unit tests cannot fully cover (C↔Rust ownership transfer, zero-size normalization at the ABI seam, and lifecycle sequencing) but are not something the memory subsystem module can provide in isolation.

### 14.3 OOM Testing

Testing the OOM abort path is inherently difficult (it terminates the process). The requirement is that the OOM code path exists and logs the correct message. Verifying the abort behavior may require process-spawning test harnesses or may be deferred to manual testing.

---

## 15. Open Decisions

| # | Decision | Context |
|---|---|---|
| 1 | Allocation tracking / leak detection | Should `mem_init`/`mem_uninit` support optional allocation tracking for development builds? This would enable leak reports at shutdown but adds overhead. Not required for functional correctness. |
| 2 | Allocation statistics | Should the subsystem track peak allocation count, total bytes allocated, or similar metrics? Useful for profiling but not required for correctness. |
| 3 | Debug-mode double-free detection | Should debug builds detect double-free or use-after-free? This would require maintaining a set of live allocations, which has performance and complexity cost. Not required for correctness. |

These decisions affect only optional diagnostic capabilities. The core allocation contracts specified in §4–§6 are complete and do not depend on these decisions.
