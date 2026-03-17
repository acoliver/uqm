# Resource Subsystem Requirements

These requirements define the expected externally observable behavior and compatibility obligations of the resource subsystem. They are written in EARS format and are language-agnostic except where ABI, callback, file-format, or integration compatibility requires externally visible behavior to remain specific.

The resource subsystem is the engine service responsible for resource-type registration, typed resource index management, lazy resource loading, typed configuration/property access, resource file helper functions, and the ownership/lifetime rules that govern loaded resource values exposed to the rest of UQM.

**Document layering:** This requirements document and its companion specification are the normative target contract. The companion `initialstate.md` is a descriptive analysis of the current codebase state, not a normative document.

## Scope and compatibility priorities

1. **Primary compatibility contract:** The established public resource ABI and its externally visible behavior, including lifecycle calls, typed lookup/update calls, load/free/detach operations, and file-loading helper functions.
2. **Secondary integration contract:** Interactions with downstream type-specific loaders, the UIO subsystem, engine startup/shutdown, and consumers that treat the subsystem as the authoritative typed resource map.
3. **Compatibility boundary:** The subsystem may change its internal implementation and language, but it shall preserve externally visible semantics required by existing consumers unless a requirement explicitly marks an area as implementation-defined.

## Lifecycle requirements

### REQ-RES-LIFE-001 Initialization readiness
- **When** the resource subsystem is initialized, **the resource subsystem shall** become ready to accept resource-type registrations, load resource indices, answer typed property queries, and service resource load/free/detach requests through the established API surface.

### REQ-RES-LIFE-002 Built-in type availability
- **When** initialization completes successfully, **the resource subsystem shall** provide the built-in scalar/config value types required by existing configuration and index consumers, including unknown, string, integer, boolean, and color-compatible value handling.

### REQ-RES-LIFE-003 Startup integration
- **When** engine startup invokes the resource subsystem before loading configuration or content indices, **the resource subsystem shall** be able to accept those index loads without requiring consumers to perform additional implementation-specific bootstrap steps.

### REQ-RES-LIFE-004 Clean shutdown
- **When** the resource subsystem is uninitialized, **the resource subsystem shall** release subsystem-owned global state and resource-index state in a way that leaves the process able to shut down cleanly.

### REQ-RES-LIFE-005 Reinitialization after teardown
- **When** the resource subsystem is initialized after a prior successful shutdown, **the resource subsystem shall** re-enter a clean initialized state without reusing invalidated resource-entry or type-registry state from the earlier lifetime.

### REQ-RES-LIFE-006 Failure cleanup during initialization
- **If** initialization fails after partially creating subsystem-owned state, **then the resource subsystem shall** release any partially created subsystem-owned state before reporting failure.

### REQ-RES-LIFE-007 Auto-initialization on first use
- **If** a public resource API function is called before explicit initialization, **then the resource subsystem shall** auto-initialize so that the call succeeds as if initialization had already been performed. This preserves the established defensive behavior expected by existing consumers.

## Type registration requirements

### REQ-RES-TYPE-001 Registration interface compatibility
- **When** a consumer registers a resource type through the established type-registration API, **the resource subsystem shall** record the externally supplied type identifier and associated load, free, and string-conversion callbacks using an ABI-compatible calling contract.

### REQ-RES-TYPE-002 Registration before use
- **When** a resource entry refers to a resource type, **the resource subsystem shall** require a handler registration for that type before a typed load of that entry can succeed.

### REQ-RES-TYPE-003 Built-in and downstream coexistence
- **When** built-in value types and downstream subsystem types are registered in the same runtime, **the resource subsystem shall** support all registrations within a single authoritative type registry used by subsequent dispatch operations.

### REQ-RES-TYPE-004 Type count visibility
- **When** a consumer queries the number of registered resource types through the established API, **the resource subsystem shall** report a count that reflects the registrations currently active in the authoritative type registry.

### REQ-RES-TYPE-005 Stable dispatch source
- **Ubiquitous:** The resource subsystem shall dispatch typed resource operations using the authoritative registration associated with the resource entry's declared type rather than by inferring loader behavior from file extensions, implementation language, or call-site-specific side tables.

### REQ-RES-TYPE-006 Registration replacement consistency
- **When** a registration for an already-known resource type is replaced or updated through the public registration surface, **the resource subsystem shall** use one coherent callback set for future dispatch of that type and shall not mix old and new callback vectors within a single resource-load operation.

### REQ-RES-TYPE-007 Callback ABI preservation
- **Where** externally visible compatibility requires function-pointer callbacks and resource-data unions to match the established ABI layout and calling convention, **the resource subsystem shall** preserve that ABI exactly.

### REQ-RES-TYPE-008 Registration replacement — already-materialized entries
- **When** a type registration is replaced after entries of that type already exist in the global resource map, **the resource subsystem shall** apply the new handler to all future dispatch operations for that type. The behavior for entries already materialized under the old handler is implementation-defined: the old free callback is no longer available through the registry, and those entries may be freed using the new handler's free callback when released. Implementations may alternatively snapshot handler identity per entry or refuse replacement after materialization. The only required invariant is that replacement must result in one coherent callback set for future operations. In practice, all type registrations occur during initialization before any entries are materialized, so replacement of an in-use handler is not an expected operational scenario.

## Resource index and descriptor requirements

### REQ-RES-IDX-001 Index load acceptance
- **When** a caller loads a resource index file through the established API, **the resource subsystem shall** parse the file format accepted by existing consumers and merge or install the resulting entries into the live resource map according to the established load contract.

### REQ-RES-IDX-002 Prefix application
- **When** index loading is invoked with a caller-supplied key prefix, **the resource subsystem shall** apply that prefix consistently to the loaded entry keys that become visible in the resource map.

### REQ-RES-IDX-003 Prefix length limit
- **When** index loading applies a caller-supplied prefix to a key, **the resource subsystem shall** truncate the combined key to 255 bytes if the prefix plus key would exceed that limit.

### REQ-RES-IDX-004 Descriptor interpretation
- **When** a resource index entry declares a typed descriptor, **the resource subsystem shall** interpret that descriptor according to the established resource descriptor contract used by current resource indices.

### REQ-RES-IDX-005 Save compatibility
- **When** a caller saves a resource index through the established API, **the resource subsystem shall** iterate the global map, apply root/prefix filtering, and emit only entries whose current type handler has a serialization function (`toString`), producing a representation compatible with subsequent loading through the same subsystem contract. The sole emission gate beyond root matching is `toString` presence; the subsystem shall not apply class-based or category-based filtering beyond this check. In practice, the entries that pass are configuration/property entries, so save functions as configuration persistence — but that is a usage characterization, not an additional filtering rule.

### REQ-RES-IDX-006 Entry replacement
- **When** a loaded or written entry uses a key that already exists in the authoritative resource map, **the resource subsystem shall** update the authoritative map according to a single deterministic replacement rule rather than leaving multiple active entries for the same key.

### REQ-RES-IDX-007 Index load error model
- **When** a resource index file is loaded, **the resource subsystem shall** insert entries into the live resource map as they are parsed. Line-local parse errors (such as a missing type-descriptor separator) shall cause the individual malformed entry to be skipped with continued parsing of subsequent lines. Loading is non-transactional: entries successfully parsed and committed before an unrecoverable failure remain committed. The exact boundary of partial commitment when a low-level failure (such as an I/O read error) occurs is implementation-dependent; the contract guarantees only that already-committed entries are not rolled back, not that parsing reaches a specific point before stopping.

### REQ-RES-IDX-008 Missing index file behavior
- **If** a requested resource index file cannot be opened, **then the resource subsystem shall** return without modifying the resource map and without reporting an error to the caller (the load function is void and returns silently on file-open failure).

## Unknown type requirements

### REQ-RES-UNK-001 Unknown type fallback
- **When** a resource index entry declares a type that is not registered in the type registry, **the resource subsystem shall** store the entry as the built-in unknown type with the descriptor string preserved, rather than discarding the entry.

### REQ-RES-UNK-002 Unknown type save behavior
- **When** a save operation encounters an entry stored as the built-in unknown type, **the resource subsystem shall** skip that entry because the unknown type has no serialization function (`toString`). This is the same serializer-presence rule that governs all save emission; it applies regardless of whether the entry's key matches the save root/prefix filter.

### REQ-RES-UNK-003 Unknown type accessor behavior
- **When** typed getters or type predicates are applied to an entry stored as the built-in unknown type, **the resource subsystem shall** treat the entry as a type mismatch and return the established default/false value for that accessor. **When** the general resource-access function is applied to an unknown-type entry, **the resource subsystem shall** return the stored descriptor string pointer and increment the reference count, following the same value-type access path as other value types.

### REQ-RES-UNK-004 Unknown type late registration
- **If** a type handler is registered after index loading and entries of that type were previously stored as the built-in unknown type, **then the resource subsystem shall** not retroactively convert those entries. The entries remain as the unknown type until the index is reloaded.

## Typed property and configuration requirements

### REQ-RES-CONF-001 Key presence queries
- **When** a caller queries whether a key exists, **the resource subsystem shall** report whether that key is currently present in the authoritative resource map.

### REQ-RES-CONF-002 Type predicate behavior
- **When** a caller invokes a typed predicate such as string, integer, boolean, or color classification, **the resource subsystem shall** determine the result from the entry's declared or stored resource type according to the established typed-property contract.

### REQ-RES-CONF-003 String get semantics
- **When** a caller requests a string value through the established string getter, **the resource subsystem shall** return the configured string value only for entries whose type is the string value type, whose value is non-null, and that satisfy the string-value contract. For missing keys, keys of non-string type, or keys with null values, the subsystem shall return a pointer to an empty string (not a null pointer).

### REQ-RES-CONF-004 Integer get semantics
- **When** a caller requests an integer value through the established integer getter, **the resource subsystem shall** return the value using the integer conversion and defaulting behavior defined by the existing integration contract.

### REQ-RES-CONF-005 Boolean get semantics
- **When** a caller requests a boolean value through the established boolean getter, **the resource subsystem shall** return the value using the boolean parsing and defaulting behavior defined by the existing integration contract.

### REQ-RES-CONF-006 Color get semantics
- **When** a caller requests a color value through the established color getter, **the resource subsystem shall** return the value using the color parsing and representation behavior defined by the existing integration contract.

### REQ-RES-CONF-007 Typed put behavior
- **When** a caller stores a string, integer, boolean, or color value through the established typed put APIs, **the resource subsystem shall** create or update the key with a type-consistent representation that can later be observed through the corresponding typed query APIs.

### REQ-RES-CONF-008 Remove behavior for config keys
- **When** a caller removes a key through the established API, **the resource subsystem shall** remove that key from subsequent presence checks, typed queries, and save output unless it is reintroduced later by another load or put operation.

### REQ-RES-CONF-009 Config persistence compatibility
- **When** configuration-oriented consumers load, mutate, and save resource-backed property data through the public APIs, **the resource subsystem shall** preserve the externally visible behavior required for existing startup, options, and input-binding flows to continue working. The save operation emits only entries whose current type handler has a serialization function (`toString`), applying root/prefix filtering. The global resource map mixes entries with and without serializers; only those with `toString` are persisted through save/load cycles. Configuration persistence is the common use case, not a separate filtering mechanism.

## Typed resource load, detach, and free requirements

### REQ-RES-LOAD-001 Lazy load dispatch
- **When** a caller requests a typed resource that is present in the authoritative resource map and not yet materialized, **the resource subsystem shall** resolve the entry's declared type, invoke the registered load path for that type, and return the resulting typed resource data through the established resource-data representation.

### REQ-RES-LOAD-002 Loaded resource reuse
- **When** a caller requests a typed resource that is already materialized and still retained by the subsystem, **the resource subsystem shall** return the already materialized resource according to the established handle/data reuse semantics rather than reloading it unconditionally.

### REQ-RES-LOAD-003 Reference acquisition
- **When** `get`-style resource acquisition succeeds for a resource type that participates in retain/release lifetime management, **the resource subsystem shall** record the caller-visible acquisition in the resource's lifetime accounting.

### REQ-RES-LOAD-004 Unknown or missing resource lookup
- **If** a caller requests a resource key that is missing, unregistered, malformed, or otherwise cannot be resolved into a valid typed load operation, **then the resource subsystem shall** report failure without returning a forged or partially initialized typed value.

### REQ-RES-LOAD-005 Detach behavior
- **When** a caller detaches a previously acquired resource through the established detach API, **the resource subsystem shall** relinquish subsystem ownership of that caller-visible acquisition while preserving the detached resource's validity according to the existing detach contract.

### REQ-RES-LOAD-006 Free behavior for retained resources
- **When** a caller frees a previously acquired resource through the established free API, **the resource subsystem shall** release the corresponding caller-visible acquisition and shall invoke resource destruction only when the last subsystem-tracked ownership claim requiring destruction has been released.

### REQ-RES-LOAD-007 Value-type free safety
- **When** a caller frees or detaches a built-in scalar/config value resource that does not require external destructor logic, **the resource subsystem shall** handle that operation safely without attempting an incompatible destructor call.

### REQ-RES-LOAD-008 Remove behavior for materialized resources
- **When** a caller removes an entry whose resource is currently materialized, **the resource subsystem shall** update the authoritative resource map and lifetime state according to the established remove contract so that later operations do not observe a stale authoritative entry.

### REQ-RES-LOAD-009 Canonical typed loader path
- **Ubiquitous:** The resource subsystem shall provide one canonical loader path per registered resource type so that all entry points observe consistent validation, ownership, error handling, and typed-result semantics for that type.

### REQ-RES-LOAD-010 Loader callback result integrity
- **When** a type-specific loader callback reports failure or returns an invalid result according to the established callback contract, **the resource subsystem shall** treat the load as failed and shall not publish the resource as a successfully materialized entry.

### REQ-RES-LOAD-011 Value-type access through the general resource accessor
- **When** the general resource-access function is applied to a value-type entry (including entries of the built-in unknown type), **the resource subsystem shall** return the entry's current data union representation without invoking heap-style lazy-load semantics, and shall increment the reference count. For entries using string-pointer storage, this returns the stored string pointer; for entries using numeric storage, this returns the numeric value cast to pointer type.

## File-loading and raw-data integration requirements

### REQ-RES-FILE-001 UIO-backed file access
- **When** the resource subsystem opens, reads, writes, seeks, tells, measures, or deletes resource files through the established helper APIs, **the resource subsystem shall** do so through the project's UIO integration contract rather than assuming unrestricted direct host-filesystem access patterns.

### REQ-RES-FILE-002 Open helper compatibility
- **When** a caller opens a resource file through the established helper API, **the resource subsystem shall** preserve the externally visible success, failure, and special-case behaviors required by existing resource consumers.

### REQ-RES-FILE-003 Directory sentinel compatibility
- **When** the established resource-file open helper encounters a directory-like target, **the resource subsystem shall** return the established sentinel handle value rather than null or a normal stream pointer, so that callers that test for the sentinel can distinguish directories from regular files. The file-length helper shall return `1` for the sentinel handle.

### REQ-RES-FILE-004 Current-resource-name guard
- **When** a file-backed resource load is active through the established load-from-path helper, **the resource subsystem shall** maintain the externally visible current-resource-name guard for the full duration of the downstream load callback and shall clear or restore it on every exit path, including failure paths. This guard is scoped to the load-from-path helper; it is not set by the general resource-access function itself. A non-file-backed heap type whose loader does not use the load-from-path helper will not observe the guard being set during its load.

### REQ-RES-FILE-005 File-backed typed load integration
- **When** a type-specific loader is invoked through the file-backed load helper, **the resource subsystem shall** open the resource relative to the established content/UIO environment, pass a compatible file handle and length to the loader callback, and close the file according to the established ownership contract after the callback returns.

### REQ-RES-FILE-006 Raw resource data compatibility
- **When** a caller requests raw resource bytes through the established raw-data helper, **the resource subsystem shall** read and validate the legacy 4-byte prefix, reject non-uncompressed prefixes, and return `length - 4` payload bytes, preserving the externally visible interpretation required by existing consumers.

### REQ-RES-FILE-007 Raw-data ownership
- **When** the raw-data helper returns an allocated buffer, **the resource subsystem shall** provide a corresponding free operation that releases that buffer according to the established ownership contract.

### REQ-RES-FILE-008 No leaked file handles on failure
- **If** a file-backed load or raw-data operation fails after opening a file or allocating intermediate state, **then the resource subsystem shall** release any subsystem-owned file handles and intermediate allocations before reporting failure.

## Ownership and lifetime requirements

### REQ-RES-OWN-001 Authoritative map ownership
- **Ubiquitous:** The resource subsystem shall remain the authoritative owner of the live resource index and resource-entry metadata until those entries are detached, removed, or discarded according to the established API contract.

### REQ-RES-OWN-002 Loader-result ownership clarity
- **When** a type-specific loader returns a resource value to the subsystem, **the resource subsystem shall** treat the returned value's ownership according to the registered type's established load/free contract and shall apply the matching destruction behavior on release.

### REQ-RES-OWN-003 Detached resource lifetime
- **When** a resource is detached through the established API, **the resource subsystem shall** transfer or relinquish ownership exactly as required by the detach contract so that downstream consumers can retain the detached object without requiring further hidden subsystem references for its continued validity.

### REQ-RES-OWN-004 No premature destroy during active ownership
- **If** a resource still has an active ownership claim under the established lifetime model, **then the resource subsystem shall** not call the registered free path or otherwise invalidate the resource prematurely.

### REQ-RES-OWN-005 Destructor use only for compatible types
- **When** the subsystem destroys a materialized resource, **the resource subsystem shall** invoke type-specific destructor logic only for resource types whose registrations define such destructor behavior.

### REQ-RES-OWN-006 Caller-visible invalidation after final release
- **When** the last ownership claim governed by the established resource lifetime contract is released and the resource is destroyed, **the resource subsystem shall** treat the prior materialized object as no longer valid for subsequent subsystem-mediated use.

### REQ-RES-OWN-007 No stale published materialization
- **When** resource destruction completes, **the resource subsystem shall** remove or invalidate the subsystem's published materialized-state record so that a later successful lookup can materialize a fresh resource if the authoritative entry still exists.

### REQ-RES-OWN-008 Global-state lifetime safety
- **When** the resource subsystem is shut down in the supported single-threaded execution model, **the resource subsystem shall** complete teardown so that no in-flight public operation observes already-released subsystem-global state. Concurrent shutdown from a separate thread while resource operations are in progress is outside the supported contract.

### REQ-RES-OWN-009 Entry replacement invalidation
- **When** a loaded heap-type resource entry is replaced by a subsequent index load or removed via the public API while outstanding references exist, **the resource subsystem shall** log a warning and proceed with freeing the old resource. The subsystem does not track or invalidate caller-held pointers; callers holding pointers to the replaced resource observe undefined behavior. This behavior is inherited from the historical contract.

### REQ-RES-OWN-010 Destruction path matches allocation domain
- **When** the subsystem destroys a materialized heap resource, **the resource subsystem shall** invoke the type-specific `freeFun` registered for that resource's type rather than assuming a single universal deallocation function applies to all resource types.

## Error handling and diagnostics requirements

### REQ-RES-ERR-001 Distinct failure reporting
- **When** a public resource operation fails because of missing keys, missing type registrations, malformed descriptors, file-open failures, parse failures, callback failures, or ownership violations detectable at the API boundary, **the resource subsystem shall** report failure through the mechanism defined by that API surface rather than masquerading the failure as a successful load or update.

### REQ-RES-ERR-002 No partial success publication
- **If** a resource load, typed put, or save operation fails after partially mutating transient implementation state, **then the resource subsystem shall** avoid publishing a caller-visible success state that would make consumers believe the operation completed fully. Note: index loading is an exception — entries are committed as parsed (see REQ-RES-IDX-007). Save operations may leave a partially written output file on mid-write I/O failure; this is a known limitation, not transactional behavior.

### REQ-RES-ERR-003 Missing-or-type-mismatch getter behavior
- **When** a typed getter is applied to a missing key or a key of an incompatible type, **the resource subsystem shall** preserve the established externally visible result behavior for that getter so that existing consumers continue to handle defaults and fallback logic correctly.

### REQ-RES-ERR-004 Callback-failure containment
- **If** a downstream type-specific callback fails, **then the resource subsystem shall** contain that failure to the current operation and shall not leave callback-owned temporary state published as a valid materialized resource.

### REQ-RES-ERR-005 Diagnostic integration
- **When** the subsystem encounters an externally significant resource failure during initialization, index load, typed load, save, or file-backed helper execution, **the resource subsystem shall** make that failure available to the project's established diagnostic or logging path to the extent required by the surrounding integration contract. The presence or absence of specific diagnostic messages is best-effort and not part of the ABI contract, except where a specific API's error table explicitly requires a warning.

### REQ-RES-ERR-006 Diagnostic context
- **When** the resource subsystem logs a warning for a resource operation failure, **the resource subsystem shall** include available context (key name, type name, or file path) in the diagnostic output to aid debugging.

## Runtime authority and integration requirements

### REQ-RES-INT-001 Engine startup compatibility
- **When** engine startup loads configuration indices, addon indices, or other established resource maps through the resource subsystem, **the resource subsystem shall** preserve the behavior required for those startup flows to continue functioning through the existing public calls.

### REQ-RES-INT-002 Downstream loader registration compatibility
- **When** downstream graphics, strings, audio, video, code, or other engine subsystems register resource handlers through the established API, **the resource subsystem shall** accept and use those registrations without requiring those consumers to change their externally visible registration pattern.

### REQ-RES-INT-003 Downstream get-then-detach pattern compatibility
- **When** downstream consumers use the established pattern of acquiring a typed resource and then detaching it for longer-lived ownership, **the resource subsystem shall** preserve that pattern's externally visible semantics.

### REQ-RES-INT-004 UIO dependency contract
- **Ubiquitous:** The resource subsystem shall treat UIO-backed directory and file services as a stable integration boundary and shall not require downstream consumers to bypass UIO in order to obtain correct resource behavior.

### REQ-RES-INT-005 Content-directory integration
- **When** file-backed resource loading depends on an externally supplied content directory or equivalent UIO resolution context, **the resource subsystem shall** use that established context for path resolution.

### REQ-RES-INT-006 Single authoritative runtime path
- **Ubiquitous:** The resource subsystem shall expose one authoritative runtime behavior for the public resource ABI so that consumers do not observe divergent semantics based solely on whether the implementation is backed by one internal module stack or another. This is a future-proofing constraint: if alternate internal runtime paths ever become active, they must converge on a single externally observable behavior.

### REQ-RES-INT-007 Cross-language callback safety
- **Where** resource loading or freeing crosses a language or FFI boundary, **the resource subsystem shall** preserve callback argument representation, return-value interpretation, and ownership expectations required by the established ABI.

### REQ-RES-INT-008 Consumer-facing ABI stability
- **Ubiquitous:** The resource subsystem shall preserve the established public names, callable entry points, global compatibility symbols, and externally visible data layouts required by existing consumers, except where a separately approved compatibility change explicitly updates that contract.

### REQ-RES-INT-009 Runtime authority split
- **Ubiquitous:** The resource subsystem shall be authoritative for the global resource map, type registry, dispatch routing, lifetime accounting, and public ABI behavior. The subsystem shall also be authoritative for cross-type semantic rules derived from registration metadata: unknown-type fallback, value-type versus heap-type treatment based on the presence of a free callback, save eligibility via serialization-function presence, and entry lifetime policy (replacement, removal, shutdown cleanup). These rules apply uniformly across all registered types and shall not be delegated to or overridden by handler implementations. Type-specific parsing, loading, and freeing semantics shall remain the responsibility of the registered handler implementation for each type. Correctness at the public boundary depends on preserving callback ABI, ownership interpretation, and UIO/file-handle conventions across the hybrid boundary between the resource subsystem and the handler implementations.
