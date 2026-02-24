# Domain Model: Memory Allocator Swap

## Entity Analysis

### HMalloc(size) — Heap Allocation

| Aspect | C (`w_memlib.c`) | Rust (`memory.rs`) |
|---|---|---|
| Implementation | `malloc(size)` | `libc::malloc(size)` (or `libc::malloc(1)` for size==0) |
| Zero-size | Returns `malloc(0)` result (implementation-defined, usually non-null on glibc/macOS) | Always returns non-null (allocates 1 byte) |
| OOM (size > 0) | `log_add(log_Fatal, ...)` → `fflush(stderr)` → `explode()` | `log_add(LogLevel::User, ...)` → `std::process::abort()` |
| Return type | `void *` | `*mut c_void` |

### HFree(ptr) — Heap Deallocation

| Aspect | C (`w_memlib.c`) | Rust (`memory.rs`) |
|---|---|---|
| Implementation | `free(p)` unconditionally | `libc::free(ptr)` with explicit null guard |
| NULL input | Safe — `free(NULL)` is C standard no-op | Safe — explicit `!ptr.is_null()` check before `free` |
| Double-free | Undefined behavior (same in both) | Undefined behavior (same in both) |

### HCalloc(size) — Zero-Filled Allocation

| Aspect | C (`w_memlib.c`) | Rust (`memory.rs`) |
|---|---|---|
| Implementation | `HMalloc(size)` then `memset(p, 0, size)` | `libc::malloc(size)` then `libc::memset(ptr, 0, size)` |
| Zero-size | Latent UB: if `malloc(0)` returns NULL, `memset(NULL, 0, 0)` is UB | Safe: allocates 1 byte, zeroes it |
| OOM | Handled by `HMalloc` internally | Same fatal path as `rust_hmalloc` |
| Note | Does NOT use `calloc()` — manually zeroes | Also does NOT use `calloc()` — manually zeroes |

### HRealloc(ptr, size) — Reallocation

| Aspect | C (`w_memlib.c`) | Rust (`memory.rs`) |
|---|---|---|
| Implementation | `realloc(p, size)` | `libc::realloc(ptr, size)` |
| Zero-size | `realloc(p, 0)` — implementation-defined (may free and return NULL) | Frees ptr, allocates 1 byte — always non-null |
| OOM (size > 0) | Same fatal path as HMalloc | Same fatal path |
| NULL ptr input | `realloc(NULL, size)` == `malloc(size)` per C standard | Same — `libc::realloc` handles this |

### mem_init() / mem_uninit() — Lifecycle

| Aspect | C (`w_memlib.c`) | Rust (`memory.rs`) |
|---|---|---|
| `mem_init()` | Stub — returns `true` | Logs "Rust memory management initialized." — returns `true` |
| `mem_uninit()` | Stub — returns `true` | Logs "Rust memory management deinitialized." — returns `true` |

## Zero-Size Allocation Handling — Detailed Analysis

The Rust implementation intentionally diverges from C for zero-size allocations:

1. **`HMalloc(0)`**: C's `malloc(0)` may return NULL (per C standard, implementation-defined). Rust allocates 1 byte, guaranteeing non-null. No caller depends on NULL from zero-size allocation.

2. **`HCalloc(0)`**: C has latent UB — calls `memset(NULL, 0, 0)` if `malloc(0)` returns NULL. Rust avoids this entirely. This is a bug fix.

3. **`HRealloc(p, 0)`**: C's `realloc(p, 0)` may act as free-and-return-NULL on some platforms. Rust explicitly frees and returns a 1-byte allocation. The `NULL && size > 0` OOM check in C passes for size==0, so C never aborts here.

**Conclusion**: Rust's zero-size handling is a strict superset of C's — always non-null, never UB. Safe to swap.

## OOM Path Differences

| Aspect | C | Rust |
|---|---|---|
| Log level | `log_Fatal` (== `log_User` == 1) | `LogLevel::User` (== 1) |
| Log numeric value | **Same (1)** | **Same (1)** |
| Post-log action | `fflush(stderr)` then `explode()` | `std::process::abort()` |
| `explode()` in DEBUG | `abort()` | N/A — always `abort()` |
| `explode()` in release | `exit(EXIT_FAILURE)` | N/A — always `abort()` |
| Difference | Release: `exit()` runs atexit handlers | `abort()` does not run atexit handlers |

**Risk**: Minimal. OOM is a fatal, unrecoverable state. The difference between `exit()` and `abort()` in release mode is whether atexit handlers run. The game has no critical atexit handlers that would matter during an OOM crash.

## Cross-Allocation Freeing Safety

During transition or mixed usage: memory allocated by C `malloc` can be freed by Rust `libc::free` and vice versa. Both use the same system heap allocator. **Fully safe.**

## HFree(NULL) Safety

Multiple call sites guard with `if (p) HFree(p)`. Both implementations handle NULL safely:
- C: `free(NULL)` is a standard no-op.
- Rust: explicit `!ptr.is_null()` guard (redundant but defensive).

The guards at call sites are harmless and do not need to be changed.

## Macro Interaction Safety

Confirmed by codebase search: no call site uses `HMalloc` as a function pointer, passes `&HMalloc`, or references `HMalloc` outside of a direct call context. All uses are `HMalloc(...)` invocations. Macro redirect is safe for all 322+ call sites.
