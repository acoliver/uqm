# Memory subsystem requirements

## Scope

These requirements define the externally observable contract for the memory subsystem that provides the historical engine allocation surface (`HMalloc`, `HFree`, `HCalloc`, `HRealloc`, `mem_init`, `mem_uninit`) when the Rust-backed memory path is enabled. The requirements are language-agnostic except where ABI compatibility and integration behavior make symbol-level compatibility externally visible.

Requirements are categorized as either **subsystem obligations** (behavior the subsystem implementation must provide) or **usage constraints** (obligations that fall on callers or the broader project). Each requirement is labeled accordingly.

## Allocation API requirements

### REQ-MEM-ALLOC-001 Historical allocation surface availability
When the build selects the memory subsystem replacement path, the subsystem shall provide the historical allocation API surface for heap allocation, zeroed allocation, reallocation, free, initialization, and shutdown so that existing engine code can continue to call `HMalloc`, `HFree`, `HCalloc`, `HRealloc`, `mem_init`, and `mem_uninit` without source-level behavioral changes.

_Category: subsystem obligation._

### REQ-MEM-ALLOC-002 ABI-compatible callable entry points
When allocation services are exposed across the C↔replacement-language boundary, the subsystem shall provide callable entry points whose symbol signatures and parameter/return types remain compatible with the public memory header (`memlib.h`) so that existing translation units compile, link, and call without adapter-specific call-sequence knowledge or call-site changes.

_Category: subsystem obligation._

### REQ-MEM-ALLOC-003 Ordinary allocation success contract
When `HMalloc(size)` is called with a positive size and sufficient memory is available, the subsystem shall return a non-null writable heap pointer that remains valid until it is later released through the compatible free/reallocation contract.

_Category: subsystem obligation._

### REQ-MEM-ALLOC-004 Zeroed allocation success contract
When `HCalloc(size)` is called and the request succeeds, the subsystem shall return a writable heap pointer whose first `size` bytes are zero-initialized.

_Category: subsystem obligation._

### REQ-MEM-ALLOC-005 Reallocation preservation contract
When `HRealloc(ptr, new_size)` is called with a pointer currently owned by the memory subsystem and the call succeeds, the subsystem shall return a pointer representing the resized allocation and shall preserve the previously stored contents up to the lesser of the old and new sizes.

_Category: subsystem obligation._

### REQ-MEM-ALLOC-006 Free compatibility contract
When `HFree(ptr)` is called with a pointer currently owned by the memory subsystem, the subsystem shall release that allocation without requiring the caller to know the underlying allocator implementation.

_Category: subsystem obligation._

### REQ-MEM-ALLOC-007 Cross-boundary allocator compatibility
When memory is allocated through one supported entry point of the subsystem and later released or resized through another supported entry point of the same subsystem boundary, the subsystem shall treat the pointer as compatible and shall not require callers to pair allocations with implementation-specific private deallocators.

_Category: subsystem obligation._

### REQ-MEM-ALLOC-008 Allocation alignment
When any allocation function returns a successful result, the returned pointer shall be suitably aligned for any object type supported by `malloc` under the platform's C ABI (at least `alignof(max_align_t)`), consistent with the alignment contract of the historical allocation surface.

_Category: subsystem obligation._

### REQ-MEM-ALLOC-009 Single-argument zeroed allocation convention
Where the historical API provides zeroed allocation via `HCalloc(size)` as a single byte-count argument, the subsystem shall preserve this single-argument convention rather than requiring count-and-element-size arguments.

_Category: subsystem obligation._

### REQ-MEM-ALLOC-010 Null-pointer reallocation as allocation
When `HRealloc(NULL, size)` is called with a positive size, the subsystem shall behave equivalently to `HMalloc(size)`. When `HRealloc(NULL, 0)` is called, the subsystem shall behave equivalently to `HMalloc(0)` under the subsystem's zero-size contract. In both cases the null pointer shall be treated as a request for a fresh allocation, not as an error.

_Category: subsystem obligation._

## Zero-size behavior requirements

### REQ-MEM-ZERO-001 Zero-size allocation returns non-null
When `HMalloc(0)` or `HCalloc(0)` is called, the subsystem shall return a non-null pointer that is safe to pass later to `HFree` or `HRealloc` under the same subsystem contract.

_Category: subsystem obligation._

### REQ-MEM-ZERO-002 Zero-size calloc initialization safety
When `HCalloc(0)` is called, the subsystem shall not perform any invalid memory operation as part of satisfying the request, including invalid zero-fill behavior on a null pointer.

_Category: subsystem obligation._

### REQ-MEM-ZERO-003 Zero-size realloc ownership transition
When `HRealloc(ptr, 0)` is called with a pointer currently owned by the subsystem, the subsystem shall release the old allocation and shall return a non-null pointer that is safe for the caller to treat according to the subsystem's zero-size contract.

_Category: subsystem obligation._

### REQ-MEM-ZERO-004 Stable zero-size policy
Where the underlying platform allocator leaves zero-size allocation behavior implementation-defined, the subsystem shall impose and document a single subsystem-level policy so that callers observe consistent semantics independent of libc-specific edge cases.

_Category: subsystem obligation._

## Out-of-memory behavior requirements

### REQ-MEM-OOM-001 Fatal OOM policy for positive-size allocation requests
When `HMalloc`, `HCalloc`, or `HRealloc` receives a positive-size request that cannot be satisfied, the subsystem shall treat the condition as fatal and shall not require ordinary callers to recover from a null return.

_Category: subsystem obligation._

### REQ-MEM-OOM-002 OOM diagnostic emission
When a fatal out-of-memory condition is detected for a positive-size allocation request, the subsystem shall make a best-effort attempt to emit a diagnostic identifying the failing operation before process termination. This logging is not guaranteed to succeed if the runtime is in a degraded state; the hard requirement is fatal termination, not successful logging.

_Category: subsystem obligation._

### REQ-MEM-OOM-003 No false OOM fatality for zero-size requests
When a zero-size allocation or reallocation request follows the subsystem's defined zero-size contract, the subsystem shall not treat implementation-defined libc zero-size behavior by itself as a positive-size out-of-memory failure.

_Category: subsystem obligation._

### REQ-MEM-OOM-004 No successful-null contract for positive-size requests
When a caller issues a positive-size allocation or reallocation request through this subsystem, the subsystem shall not require callers to treat a null return as a normal success-path result.

_Category: subsystem obligation._

### REQ-MEM-OOM-005 No recursive allocation in OOM handling
When reporting a fatal out-of-memory condition, the subsystem shall avoid introducing allocation dependencies that could trigger a recursive OOM before the diagnostic is emitted or the process is terminated.

_Category: subsystem obligation._

## Lifecycle hook requirements

### REQ-MEM-LIFE-001 Initialization hook availability
When engine startup invokes `mem_init`, the subsystem shall provide a callable initialization hook that returns a boolean success status compatible with the historical API.

_Category: subsystem obligation._

### REQ-MEM-LIFE-002 Shutdown hook availability
When engine shutdown invokes `mem_uninit`, the subsystem shall provide a callable shutdown hook that returns a boolean success status compatible with the historical API.

_Category: subsystem obligation._

### REQ-MEM-LIFE-003 Startup readiness guarantee
When `mem_init` reports success, the subsystem shall be ready to service subsequent allocation, free, and reallocation requests through the public memory API.

_Category: subsystem obligation._

### REQ-MEM-LIFE-004 Shutdown cleanup shall not invalidate prior operations
When `mem_uninit` is called after successful startup, the subsystem shall complete shutdown without retroactively invalidating the correctness of allocations and frees that were performed according to the subsystem's ownership rules before shutdown was invoked. Any subsequent allocation, free, or reallocation calls after `mem_uninit` completes remain outside the subsystem's contract (see REQ-MEM-LIFE-007).

_Category: subsystem obligation._

### REQ-MEM-LIFE-005 Placeholder lifecycle compatibility
Where the intended end-state implementation does not yet require allocator-global state, the subsystem may implement `mem_init` and `mem_uninit` as no-op or logging-compatible lifecycle hooks, provided the historical success/failure contract remains intact and callers are not required to compensate for missing hidden side effects.

_Category: subsystem obligation._

### REQ-MEM-LIFE-006 Initialization idempotency
When `mem_init` is called more than once, the subsystem shall not fail or produce incorrect behavior on subsequent calls. A second call after a successful first call shall be a no-op returning success.

_Category: subsystem obligation._

### REQ-MEM-LIFE-007 Post-uninit allocation behavior is undefined
When `mem_uninit` has been called and completed, the behavior of subsequent allocation, free, or reallocation calls through the subsystem is undefined. The launcher or runtime harness is responsible for ensuring no such calls occur after uninit.

_Category: usage constraint (launcher/harness responsibility)._

## Ownership and lifetime requirements

### REQ-MEM-OWN-001 Caller-owned allocation result
When the subsystem returns a successful allocation result, the result is considered caller-owned for API-contract purposes: the caller holds the obligation to eventually release that allocation through the compatible deallocator (`HFree` or the equivalent subsystem entry point), or to transfer that obligation to another component through a documented convention. The subsystem itself does not track or enforce this ownership.

_Category: interface semantic (defines the API contract; not enforced by the subsystem at runtime)._

### REQ-MEM-OWN-002 Single-release contract
When a caller holds a live allocation returned by the subsystem, the allocation shall be released exactly once through the compatible deallocator. The subsystem shall not require any separate hidden bookkeeping call for ordinary ownership completion. Double-free and use-after-free are undefined behavior that the subsystem is not required to detect.

_Category: usage constraint (caller responsibility); the subsystem's obligation is limited to correct behavior when the contract is followed._

### REQ-MEM-OWN-003 Null-free safety
When `HFree(NULL)` is called, the subsystem shall treat the call as a safe no-op.

_Category: subsystem obligation._

### REQ-MEM-OWN-004 Realloc ownership replacement
When `HRealloc(ptr, new_size)` succeeds, the subsystem shall transfer continued ownership to the returned allocation result, and callers shall no longer be required or permitted to continue using any superseded storage except through the returned pointer.

_Category: subsystem obligation (return semantics); usage constraint (caller must stop using old pointer)._

### REQ-MEM-OWN-005 Realloc failure ownership retention
When a positive-size `HRealloc(ptr, new_size)` request fails internally before the subsystem's fatal OOM policy terminates the process, the subsystem shall not create an externally visible state in which ownership of the original allocation becomes ambiguous.

_Category: subsystem obligation._

### REQ-MEM-OWN-006 No hidden ownership model change at ABI boundary
When C code or mixed-language FFI code uses the historical memory API, the subsystem shall preserve raw-pointer ownership expectations rather than requiring those callers to adopt language-native ownership constructs merely to remain correct.

_Category: subsystem obligation._

### REQ-MEM-OWN-007 Mixed-language raw-pointer compatibility
When Rust-side FFI glue or other mixed-language integration code uses the historical memory allocation surface to create storage intended for C-compatible consumers, the subsystem shall preserve the same release and lifetime expectations that apply to C callers of the same surface.

_Category: subsystem obligation._

## Integration obligations

### REQ-MEM-INT-001 Public header compatibility
When the replacement memory subsystem is enabled, the integration layer shall preserve compatibility with the established public memory header so that existing engine modules can continue including that header and using the historical macro/function surface.

_Category: subsystem obligation._

### REQ-MEM-INT-002 Legacy implementation exclusion
When the replacement memory subsystem is enabled in a build, the integration shall exclude simultaneous compilation or selection of an incompatible duplicate legacy allocator implementation for the same public surface.

_Category: subsystem obligation._

### REQ-MEM-INT-003 Engine-wide allocator uniformity
When the replacement memory subsystem is selected for a build, all engine modules using the historical allocation header shall resolve to the same active allocator surface for that build configuration.

_Category: subsystem obligation._

### REQ-MEM-INT-004 Runtime entry sequencing
When the launcher or runtime harness is responsible for subsystem startup and shutdown sequencing, it shall ensure the memory allocation ABI is available before dependent engine execution begins, and shall ensure no allocation calls occur after the shutdown hook completes.

_Category: usage constraint (launcher/harness responsibility). The subsystem's obligation is to be ready after successful init and to define post-uninit behavior as undefined._

### REQ-MEM-INT-005 Underlying allocator abstraction freedom
Where allocator implementation details are not externally visible through the public ABI, the subsystem may use libc allocation primitives or a future replacement allocator, provided the externally visible allocation, zero-size, OOM, lifecycle, and ownership contracts remain satisfied.

_Category: subsystem obligation._

### REQ-MEM-INT-006 Thread-safe allocation and free
When the wider engine issues allocation or free calls from multiple threads, the memory subsystem's allocation and deallocation entry points shall remain safe for concurrent use without requiring callers to hold external locks.

_Category: subsystem obligation._

### REQ-MEM-INT-007 No caller-visible dependency on implementation language
When the subsystem is replaced behind the established memory ABI, existing C callers shall not be required to change call signatures, data layouts, or release conventions, and existing linked modules shall continue to compile and link without call-site changes, solely because the implementation language differs from the legacy implementation.

_Category: subsystem obligation._

### REQ-MEM-INT-008 Allocator-family documentation at FFI boundaries
When a cross-language API transfers ownership of allocated memory and relies on this allocator family, the API shall document which allocator family (C ABI allocator or language-native allocator) owns the result and which deallocator the receiver should use.

_Category: program-level integration obligation for APIs that use this allocator family. This is not something the memory subsystem implementation can satisfy or enforce in isolation._

### REQ-MEM-INT-009 Mixed-language integration test coverage
When the memory subsystem serves as a cross-language allocation boundary, the project test suite should include dedicated mixed-language integration tests that exercise allocation in one language and deallocation in the other, zero-size normalization at the ABI seam, and lifecycle sequencing, so that boundary-specific risks are directly verified rather than only indirectly covered by single-language unit tests.

_Category: program-level integration obligation. These tests verify boundary-specific risks but are not something the memory subsystem module can provide in isolation._
