# Rust UIO Subsystem — Functional and Technical Specification

## 1. Overview

UIO is the virtual filesystem layer for UQM. It presents a unified namespace to the rest of the engine, merging real filesystem directories, ZIP/UQM archive contents, and addon trees into a single path hierarchy. All file and stream I/O in the engine flows through UIO's public API.

This document specifies the desired end-state behavior of the Rust UIO subsystem. It covers functional behavior, API contracts, integration boundaries, and ownership responsibilities.

**Relationship to `file-io/` documentation set:** This document and `file-io/specification.md` describe the same subsystem — the UIO virtual filesystem. `file-io/specification.md` is the sole normative owner of all behavior observable through any `uio_*` exported API call. This document (`uio/`) is secondary/supporting documentation that describes rationale, implementation constraints, port-completeness tracking, and unresolved audit areas. Where both documents discuss the same behavior and diverge, `file-io/specification.md` controls. This document must defer to `file-io/` for all overlapping API-observable specifications and may not independently define pass/fail behavior for any exported API semantic.

### 1.1 Terminology

This document uses three distinct levels of requirement:

- **Legacy parity** — behavior that must match the legacy C implementation because existing consumers depend on it. These are hard compatibility requirements backed by observed code evidence.
- **Engine-required compatibility** — behavior that current UQM engine code exercises and that must work correctly, even if the exact legacy semantics are not fully proven. These are strong requirements backed by integration evidence.
- **Desired improvement** — behavior that improves correctness, safety, or maintainability beyond what the legacy system provided. These are goals, not compatibility blockers.

Where behavior is specified without qualification, it is a legacy-parity or engine-required-compatibility item. Desired improvements are labeled explicitly.

#### 1.1.1 Glossary

The following related terms appear throughout this document set and have distinct meanings:

- **Parity** — matching the externally observable behavior of the legacy C implementation for a given API, as exercised by known consumers.
- **Compatibility-complete** — providing full API-surface coverage for all exported symbols, including items not proven to be required for current engine startup/runtime viability. A compatibility-complete item may still be required for supported in-tree consumers, tooling, edge cases, or future use; it is not synonymous with “optional” or “speculative.” Compatibility-complete items remain mandatory for final subsystem completion even when they are not current engine-viability gates. Clean stub failure is acceptable only as an interim implementation state unless a specific requirement explicitly defines failure-only behavior as the intended long-term contract.
- **Port-completeness** — the state where all exported `uio_*` symbols are directly provided by Rust code with no remaining C forwarding shims. This is an internal integration-cleanliness goal and does not affect externally observable behavior.
- **Interim acceptance rule** — a temporary normative rule used to keep implementation and testing deterministic while legacy behavior is still unverified. Interim acceptance rules are binding for current acceptance, but they do not by themselves establish the final parity contract; verified legacy behavior supersedes them.


### 1.2 Evidence classification

**Non-normative evidence labels:** The evidence classifications below ("proven contract," "proposed design") describe the provenance of behavioral claims within this document's analysis of legacy behavior. They do **not** independently define pass/fail criteria for any exported `uio_*` API behavior. All normative pass/fail obligations for exported API behavior are owned by `file-io/specification.md`. Where `file-io` makes a behavior mandatory, the "proposed design" label in this document is historical provenance context only and does not make that behavior revisable or provisional. See §1.2.1 for the controlling non-normative prioritization note.

Where a behavioral claim is grounded in different levels of evidence, this document marks the basis:

- **Proven contract** — directly observed in code, headers, or consumer call sites. This label describes evidence strength, not normative authority (which is owned by `file-io`).
- **Proposed design** — a reasonable design choice based on API signatures, header comments, or expected use cases, but whose legacy behavior is not yet confirmed. Where `file-io/specification.md` has made the same behavior normatively mandatory, the "proposed design" label here is superseded and should be read as historical rationale only.

Sections that mix both bases use inline labels to distinguish them.
### 1.2.1 Current acceptance boundary summary

**Non-normative prioritization note:** The priority tiers and acceptance gates described in this document (engine-critical, compatibility-complete, supported in-tree surface, etc.) are informative implementation-planning aids only. They do not independently define pass/fail criteria for any exported `uio_*` API behavior. All normative pass/fail obligations for exported API behavior are owned by `file-io/specification.md`. An acceptance tier stated in this document cannot narrow, defer, or override a `file-io` obligation unless `file-io` says so explicitly.

For current parity acceptance, this document uses three informative buckets:

1. **Verified engine-critical parity requirements now** — behavior directly required by proven startup/resource consumers.
2. **Interim acceptance rules now** — deterministic temporary rules used for unresolved ordering/overlap/backing-type cases until legacy verification confirms or replaces them.
3. **Deferred compatibility-complete scope** — supported exported surface and broader parity goals that are not current engine-viability blockers.

Applied to the highest-risk UIO areas:
- merged directory behavior is engine-critical only for the proven startup/resource-consumer scenarios documented in the acceptance-boundary sections;
- generalized merged-directory parity across arbitrary overlapping mount combinations is compatibility-complete, not a current parity gate;
- startup-visible `.rmp` visibility/deduplication is engine-critical to the extent demonstrated by current evidence, while broader combined STDIO+archive `.rmp` discovery remains deferred until legacy verification proves it matters;
- provisional mount-resolution and ordering rules are required for current implementation/testing where the docs mark them as interim acceptance rules, but they are not yet final legacy parity claims.



---

## 2. Subsystem Responsibilities

The Rust UIO subsystem is responsible for the capabilities listed below. Each item is tagged with its priority tier (see §14 for definitions) so that the reader can distinguish engine-critical responsibilities from compatibility-complete or quality/cleanup items at a glance.

1. **Repository management** [engine-critical] — creating and destroying the top-level `uio_Repository` object that anchors the mount namespace.
2. **Mount management** [engine-critical] — mounting filesystem directories and ZIP/UQM archives into the virtual namespace at specified mount points, with ordering control (top, bottom, above, below relative to an existing mount).
3. **Directory handles** [engine-critical] — opening, closing, and lifetime-managing `uio_DirHandle` objects that represent positions in the virtual namespace.
4. **File descriptor I/O** [engine-critical] — `uio_open` / `uio_close` / `uio_read` / `uio_write` / `uio_lseek` / `uio_fstat` providing POSIX-style unbuffered access to files located through the mount namespace.
5. **Buffered stream I/O** [engine-critical for core operations] — `uio_fopen` / `uio_fclose` / `uio_fread` / `uio_fwrite` / `uio_fseek` / `uio_ftell` / `uio_fgets` / `uio_fgetc` / `uio_ungetc` / `uio_fputc` / `uio_fputs` / `uio_fflush` / `uio_feof` / `uio_ferror` providing stdio-style buffered access. `uio_clearerr` is [compatibility-complete]. `uio_vfprintf` is [compatibility-complete] (see §5.3).
6. **Directory enumeration and listing lifecycle** [engine-critical] — `uio_getDirList` / `uio_DirList_free` providing the listing behavior needed by current startup/resource consumers, including the acceptance-bounded merged-list cases documented in §5.4.
7. **Full match-mode parity for directory enumeration** [compatibility-complete] — literal, prefix, suffix, substring, and general POSIX-regex match semantics across the exported listing API. The currently proven startup-used subset is engine-critical; full general regex parity remains compatibility-complete. See §5.4 and §9.1.
8. **Filesystem metadata** [engine-critical for stat/access/unlink; compatibility-complete for mkdir/rmdir/rename] — `uio_stat` / `uio_access` / `uio_mkdir` / `uio_rmdir` / `uio_rename` / `uio_unlink`.
9. **File location resolution** [compatibility-complete, supported in-tree surface] — `uio_getFileLocation` mapping virtual paths to physical paths and their originating mount. This is not proven startup-critical, but it is part of the documented exported API surface and is referenced by in-tree helper paths that participate in native-path bridging.
10. **Mount transplanting** [compatibility-complete] — `uio_transplantDir` remounting an existing directory handle at a new virtual location. The only observed consumer call site is addon shadow-content mounting in `options.c:582`, which transplants shadow directories above `contentMountHandle`. No engine-critical startup dependency on transplant has been established — `prepareMeleeDir()` uses `uio_openDirRelative`, not `uio_transplantDir`. If transplant fails, addon shadow-content mounting degrades but the engine remains viable for base content loading and core gameplay.
11. **FileBlock API** [compatibility-complete] — `uio_openFileBlock` / `uio_closeFileBlock` / `uio_accessFileBlock` / `uio_copyFileBlock` / `uio_setFileBlockUsageHint` for memory-mapped or block-oriented file access. In-tree usage exists in the legacy ZIP implementation (`sc2/src/libs/uio/zip/zip.c`), but engine-critical runtime dependence through the exported API is not established. If the Rust implementation provides archive support independently of the legacy FileBlock layering, these APIs remain compatibility-complete as exported symbols. See §5.9.
12. **Stdio access bridge** [compatibility-complete, supported in-tree surface] — `uio_getStdioAccess` / `uio_releaseStdioAccess` / `uio_StdioAccessHandle_getPath` providing a real filesystem path for subsystems that need to bypass UIO (e.g., passing paths to external libraries). In-tree usage exists in legacy helper and debug code (`sc2/src/libs/uio/utils.c`, `sc2/src/libs/uio/debug.c`). Engine-critical runtime dependence through the exported API is not established, but this is still documented supported surface for in-tree consumers and must not be treated as optional. See §5.10.
13. **Diagnostics** [compatibility-complete, non-blocking] — `uio_printMounts`, `uio_DirHandle_print`, and bridge logging. These must exist as callable symbols but are not tied to proven runtime-critical consumer expectations. See §11.

## 3. Subsystem Boundaries

### 3.1 What UIO owns

**Important boundary:** this document uses “ownership” in two distinct senses that must not be conflated. UIO owns the exported `uio_*` API behavior, mount primitives, namespace resolution, and UIO object lifetimes. C startup code continues to own mount-selection policy and the lifecycle of global directory/mount handles. In this document, **port-completeness** refers only to exported-symbol ownership, not to migrating startup orchestration into Rust UIO.

- All `uio_*` exported symbols that make up the UIO API boundary. In the final state, the Rust crate should directly export every `uio_*` function with C ABI linkage, eliminating any remaining C forwarding shims. This is a **port-completeness** goal (see §1.1.1), not a transfer of startup-policy ownership.
- The global mount registry and all namespace/path-resolution logic that interprets the active mount set.
- Allocation and deallocation of all UIO objects (`uio_Repository`, `uio_DirHandle`, `uio_MountHandle`, `uio_Handle`, `uio_Stream`, `uio_DirList`, `uio_FileBlock`, `uio_StdioAccessHandle`).
- ZIP/UQM archive reading (decompression, directory traversal within archives).

### 3.2 What UIO does not own

- **Mount-selection policy and startup orchestration.** The decisions about *which* directories and archives to mount, *where* to mount them, and *when* startup should create or dispose of global handles remain in C startup code (`options.c`). UIO provides the mount primitives and namespace behavior; the caller decides the startup policy. This boundary is intentional and stable.
- **Global directory handle variables.** `contentDir`, `configDir`, `saveDir`, `meleeDir`, and `contentMountHandle` are C-owned globals. UIO creates the handles; the caller stores and manages their lifetimes.
- **SDL_RWops adaptation.** The `sdluio.c` adapter wraps UIO streams into SDL_RWops callbacks. This is a consumer of UIO, not part of UIO.
- **Resource index loading.** `LoadResourceIndex()` consumes UIO directory listings to discover `.rmp` files but is not a UIO responsibility.

### 3.3 Integration points

| Consumer | API surface used | Notes |
|---|---|---|
| C startup (`options.c`) | `uio_openRepository`, `uio_mountDir`, `uio_openDir`, `uio_openDirRelative`, `uio_closeDir`, `uio_getDirList`, `uio_DirList_free`, `uio_stat`, `uio_unmountDir`, `uio_transplantDir` | Mount orchestration |
| SDL graphics (`sdluio.c`) | `uio_fopen`, `uio_fclose`, `uio_fread`, `uio_fwrite`, `uio_fseek`, `uio_ftell`, `uio_ferror` | SDL_RWops bridge |
| Rust sound decoders | `uio_open`, `uio_close`, `uio_read`, `uio_fstat` | File descriptor style |
| Rust audio heart | `uio_fopen`, `uio_fclose`, `uio_fread`, `uio_fseek`, `uio_ftell` | Stream style via FFI |
| Rust resource bridge | `uio_fopen`, `uio_fclose`, `uio_fread`, `uio_fwrite`, `uio_fseek`, `uio_ftell`, `uio_fgetc`, `uio_fputc`, `uio_unlink` | Stream + metadata |

---

## 4. Data Structures

This section defines the externally observable layout and semantics of UIO data structures. Internal organization choices (container types, synchronization primitives, index structures) are non-normative and left to the implementation unless they affect ABI-visible layout or caller-observable behavior.

### 4.1 `uio_Repository`

An opaque handle representing a mount namespace. UQM uses exactly one repository. The public API permits creating multiple repositories, but no current consumer exercises independent multi-repository behavior. Multi-repository support is retained for API completeness, not as a proven compatibility requirement.

**Externally observable semantics:**

- Maintains an ordered set of mounts where position determines override priority.
- **Proven contract:** Mount ordering must correctly implement the placement semantics where `uio_MOUNT_TOP` places a mount at the front and `uio_MOUNT_BOTTOM` at the back of the priority list, and `uio_MOUNT_ABOVE` / `uio_MOUNT_BELOW` place relative to an existing mount. The basic precedence model (TOP before BOTTOM, relative placement above/below an anchor) is established by header constants and consumer call patterns.
- **Proposed design (provisional):** Edge cases where mount-point specificity and insertion order interact are not yet fully characterized — see §15, question 8. The current normative guarantee is limited to the non-ambiguous/common cases exercised by startup and content-loading flows; edge-case resolution outside those observed cases remains acceptance-test-defined pending verification and must not be treated as settled legacy contract.
- Remains associated with its created mount namespace for its lifetime and serves as the ownership root for mounts opened within that namespace.

The reviewed evidence does not currently establish that external consumers depend on named public fields within `uio_Repository`, so this specification does not freeze a field-level ABI contract for the struct beyond compatibility with the public C type and its observable behavior.

**Implementation notes (non-normative):** The internal mount list and any mount-tree index structure are implementation choices. A flat list sorted by criteria, a tree, or any other structure is acceptable as long as the externally visible precedence semantics are correct.

### 4.2 `uio_DirHandle`

A handle to a directory in the virtual namespace. The legacy C implementation uses reference counting. UQM does not share directory handles across independent owners, so the practical contract is close-once ownership. The implementation must support at least close-once deallocation; reference counting is acceptable but not externally required by current consumers.

**Externally observable semantics:**

- the handle identifies a directory position within a repository namespace
- operations using the handle resolve relative paths against that directory position
- the handle remains associated with its owning repository for its lifetime

The reviewed evidence does not currently establish that external consumers depend on named public fields within `uio_DirHandle`, so this specification does not freeze a field-level ABI contract for the struct beyond compatibility with the public C type and its observable behavior.

**Lifetime contract:** `uio_openDir` and `uio_openDirRelative` create handles. `uio_closeDir` releases the handle. When the final ownership reference is released, the handle is deallocated.

### 4.3 `uio_MountHandle`

Returned by `uio_mountDir` and `uio_transplantDir`. Used as a token for unmounting and as a relative-position anchor for subsequent mounts.

**Externally observable semantics:**

- the handle identifies a specific mounted contribution within a repository namespace
- the handle is accepted by `uio_unmountDir` to remove that mounted contribution
- the handle may be used as the `relative` anchor for subsequent mount-placement operations where the API allows it
- the handle remains associated with its owning repository for its lifetime

The reviewed evidence does not currently establish that external consumers depend on named public fields within `uio_MountHandle`, so this specification does not freeze a field-level ABI contract for the struct beyond compatibility with the public C type and its observable behavior.

### 4.4 `uio_Handle`

An open file descriptor.

```
Semantics:
  - Created by uio_open()
  - Supports read/write/seek/fstat depending on open flags
  - Destroyed by uio_close()
  - For files inside ZIP archives: the handle wraps a decompressed view (in-memory or streamed)
```

### 4.5 `uio_Stream`

A buffered I/O stream wrapping a `uio_Handle`.

```
Fields (must match C struct layout exactly):
  - buf: *mut c_char        — start of the buffer
  - dataStart: *mut c_char  — start of valid data in buffer
  - dataEnd: *mut c_char    — end of valid data in buffer
  - bufEnd: *mut c_char     — end of buffer allocation
  - handle: *mut uio_Handle — underlying file handle
  - status: c_int           — STATUS_OK=0, STATUS_EOF=1, STATUS_ERROR=2
  - operation: c_int        — NONE=0, READ=1, WRITE=2
  - openFlags: c_int        — flags from the open call

Invariants:
  - buf <= dataStart <= dataEnd <= bufEnd
  - if operation == WRITE then buf == dataStart
```

**Buffer ownership:** The stream owns its buffer. `uio_fclose` must deallocate the buffer. The buffer may be allocated lazily on first read/write or eagerly on open; either is acceptable as long as it is freed on close without leaking.

### 4.6 `uio_DirList`

```
Fields (C-compatible public layout):
  - names: *mut *mut c_char  — array of C string pointers
  - numNames: c_int          — number of entries
```

All name strings are owned by the listing. `uio_DirList_free` must deallocate `names`, all backing string storage, and the `uio_DirList` struct itself.

---

## 5. Functional Behavior

### 5.1 Mount Semantics

#### 5.1.1 Filesystem types

Two filesystem types are supported:

- **`uio_FSTYPE_STDIO` (1)** — mounts a real filesystem directory. File operations delegate to the OS.
- **`uio_FSTYPE_ZIP` (2)** — mounts the contents of a ZIP or UQM archive file. The archive is read and its entries are exposed as virtual files under the mount point. Files within the archive are read-only.

#### 5.1.2 Mount ordering

Mounts are ordered within the repository. When multiple mounts cover the same virtual path, the mount with higher priority (closer to the top of the list) wins for read operations. The ordering flags are:

| Flag | Value | Behavior |
|---|---|---|
| `uio_MOUNT_BOTTOM` | `0 << 2` | Place at the bottom of the mount list |
| `uio_MOUNT_TOP` | `1 << 2` | Place at the top of the mount list |
| `uio_MOUNT_BELOW` | `2 << 2` | Place below the `relative` mount |
| `uio_MOUNT_ABOVE` | `3 << 2` | Place above the `relative` mount |

When `uio_MOUNT_BELOW` or `uio_MOUNT_ABOVE` is specified, the `relative` parameter must be non-null and must point to an existing mount in the same repository. When `uio_MOUNT_TOP` or `uio_MOUNT_BOTTOM` is specified, `relative` must be null.

**Proven contract:** A mount placed with `TOP` takes precedence over all existing mounts; a mount placed with `BOTTOM` has lower precedence than all existing mounts; `ABOVE` and `BELOW` place the new mount immediately above or below the referenced mount in the precedence order. Any internal representation that produces these observable precedence outcomes is acceptable. This is established by header constants and consumer call patterns in `options.c`.

**Proposed design (provisional — edge cases):** When mount-point specificity (path depth) and insertion order interact — for example, a shorter mount point inserted with `TOP` versus a longer, more-specific mount point inserted earlier — the exact legacy winner is not yet confirmed. Until legacy verification resolves this, the interim implementation rule shall be: (1) respect explicit placement precedence (`TOP`/`BOTTOM`/`ABOVE`/`BELOW`) as the primary ordering relation, (2) among otherwise competing active mounts that both match a path, prefer the longer matching mount-point prefix, and (3) if ambiguity still remains, break ties by recency/insertion order. This is a deterministic provisional rule for implementation/testing, not a claim that legacy C necessarily used the same rule. Implementers must treat these specificity-vs-order edge cases as provisional and acceptance-test-defined pending legacy verification (see §15, question 8).

#### 5.1.3 Read-only flag

`uio_MOUNT_RDONLY` (`1 << 1`) marks a mount as read-only. Write operations (`uio_open` with write flags, `uio_unlink`, `uio_rename` into the mount, `uio_mkdir`, `uio_rmdir`) must fail with an appropriate error when targeting a read-only mount.

#### 5.1.4 Mount path resolution

Given a virtual path like `/content/packages/foo.zip`, the general resolution approach is:

1. Normalize the path (collapse `.`, `..`, duplicate separators).
2. Walk the mount list in priority order.
3. For each mount, check if the mount point is a prefix of the virtual path.
4. If matched, strip the mount point prefix and append the remainder to the mount's physical root.
5. For STDIO mounts, check if the resulting physical path exists on the real filesystem.
6. For ZIP mounts, check if the resulting path exists as an entry in the archive's directory.
7. Return the first match found.

For write operations, only writable mounts are considered.

**Proven contract:** The basic flow (priority-order walk, prefix match, first-hit return) is established by the legacy C mount tree and consumer expectations.

**Proposed design (provisional):** The common-case algorithm above (priority-order walk, prefix match, first-hit return) is the current target for normal overlapping mounts. For unresolved overlap cases, the interim implementation rule from §5.1.2 applies: explicit placement precedence first, then longest matching mount-point prefix, then recency/insertion order. Edge-case interactions between mount-point specificity and insertion-based priority are not yet fully validated against the legacy C implementation — see §15, question 8. Implementers must use the interim rule consistently for deterministic behavior, while still treating it as provisional rather than proven legacy parity.

#### 5.1.5 ZIP/UQM archive mounting

When `uio_mountDir` is called with `fsType = uio_FSTYPE_ZIP`:

- `sourceDir` + `sourcePath` identify the archive file (e.g., `sourceDir` is a handle to `/packages`, `sourcePath` is `"content.uqm"`).
- `inPath` specifies the root within the archive to expose (typically `"/"`).
- The archive is opened and its directory is parsed to build an entry index.
- All entries in the archive become accessible as read-only virtual files under `mountPoint`.

The archive must remain accessible for the lifetime of the mount. Decompression occurs on read.

#### 5.1.6 .rmp discovery and load-order integration

**Context:** After mounting content and archives, C startup code (`loadIndices()` in `options.c:490–507`) enumerates `.rmp` resource index files via `uio_getDirList` with a regex match and calls `LoadResourceIndex()` for each result.

**Proven contract:** `loadIndices()` enumerates `.rmp` files from the merged namespace after archive mounting. The merged directory listing produced by `uio_getDirList` determines which `.rmp` files are discovered and in what order they are loaded.

**Open verification question:** Whether production content actually relies on `.rmp` files existing inside ZIP/UQM archives (as opposed to only in STDIO-backed directories) is not confirmed by static inspection. `initialstate.md` explicitly leaves this unknown. For parity purposes, combined STDIO+archive `.rmp` discovery therefore remains an open compatibility question rather than a proven production requirement. Supporting that combined case is still a reasonable design-forward target for a more capable merged namespace, but it must not be treated as parity-mandatory unless legacy verification establishes that production behavior depends on it.

**Proposed design (provisional — acceptance criteria pending verification):** The following behaviors represent the provisional target until verified against the legacy C implementation:

- When `.rmp` files with the same name appear in multiple mounts contributing to the same virtual directory, first-seen deduplication (by mount precedence) determines which `.rmp` is discovered. Duplicate names from lower-precedence mounts are not included in the listing.
- The order in which `.rmp` files appear in the merged listing determines the order `LoadResourceIndex()` processes them. If load order affects resource-override semantics (e.g., later indices overriding earlier ones), then listing order is behaviorally significant.
- Whether listing order must match a specific legacy order (mount-precedence then filesystem-order, lexical order, or another scheme) is unknown. Until legacy ordering is confirmed, the current interim acceptance rule is: use mount-precedence order with first-seen deduplication and lexical-by-entry-name ordering within each contributing mount. This rule exists to keep implementation and testing deterministic; it is not yet a claim that lexical intra-mount ordering is the final legacy parity contract.

**Acceptance criterion:** For current parity acceptance, mount a real content tree matching the currently exercised startup/resource-loading cases and verify that `uio_getDirList` returns the startup-visible `.rmp` files required by the proven evidence baseline, with correct deduplication under the active merged listing contract. Until legacy merged-list ordering is confirmed, current acceptance testing must require deterministic `.rmp` enumeration under the provisional rule of mount-precedence ordering with first-seen deduplication and lexical-by-entry-name ordering within each contributing mount. That deterministic ordering is currently required for reproducible implementation and testing, not yet as a proven runtime-significant parity contract. Compare the resulting discovery order against the legacy C implementation to confirm or revise the final ordering contract. A supplemental design-forward test may also mount `.rmp` files in both STDIO directories and ZIP archives to evaluate the broader merged-namespace target, but that broader combined-source case is not parity-mandatory until legacy verification establishes that production behavior depends on it.

### 5.2 File Descriptor API

#### `uio_open(dir, path, flags, mode) -> *mut uio_Handle`

Opens a file relative to `dir`. Resolves the path through the mount registry. Supports flags `O_RDONLY`, `O_WRONLY`, `O_RDWR`, `O_CREAT`, `O_EXCL`, `O_TRUNC`. Returns null on failure. Sets `errno` on failure (see §6.2).

For files inside ZIP mounts: opens a read-only decompressed view. Write flags must fail.

#### `uio_close(handle) -> c_int`

Closes the file handle and releases its resources. Returns 0 on success, -1 on error.

#### `uio_read(handle, buf, count) -> ssize_t`

Reads up to `count` bytes. Returns the number of bytes read (may be less than `count` at EOF). Returns -1 on error. Returns 0 at EOF.

#### `uio_write(handle, buf, count) -> ssize_t`

Writes `count` bytes. Returns the number of bytes written. Returns -1 on error. Fails on read-only mounts.

#### `uio_lseek(handle, offset, whence) -> c_int`

Seeks within the file. `whence` is `SEEK_SET` (0), `SEEK_CUR` (1), or `SEEK_END` (2). Returns 0 on success, -1 on error.

#### `uio_fstat(handle, stat_buf) -> c_int`

Fills `stat_buf` with file metadata. Must populate at minimum `st_size` and `st_mode` (file type and permissions bits). Returns 0 on success, -1 on error.

### 5.3 Buffered Stream API

The stream API provides buffered I/O semantics compatible with the contract defined in the UIO C headers (`uiostream.h`). Where UIO stream behavior aligns with C stdio conventions, that alignment is intentional and must be preserved because consumers (especially `sdluio.c`) depend on the stdio-like contract. Where UIO's exact behavior in edge cases is not proven to match ISO C stdio in all respects, the implementation should follow the UIO header contract and known consumer expectations rather than attempting to replicate every stdio edge case.

#### `uio_fopen(dir, path, mode) -> *mut uio_Stream`

Opens a buffered stream. `mode` follows C `fopen` conventions: `"r"`, `"rb"`, `"w"`, `"wb"`, `"a"`, `"r+"`, etc. Returns null on failure.

**Proven contract:** The stream must present a valid `uio_Stream` object whose buffer-related fields satisfy the invariants in §4.5 while the stream is live. The default buffer size in the C implementation is 8192 bytes, but matching that exact size is not required. Alternative internal strategies are acceptable only if they still populate the public `uio_Stream` fields in an ABI-compatible way and preserve the caller-visible buffering invariants; a truly bufferless representation that leaves those fields invalid is not acceptable.

#### `uio_fclose(stream) -> c_int`

Flushes any pending writes, closes the underlying handle, deallocates the buffer and stream. Returns 0 on success, `EOF` (-1) on error.

**Ownership/lifetime contract:** `uio_fclose` must close the underlying `uio_Handle` and reclaim the `uio_Stream` object plus any resources whose lifetime is owned by that stream, including any buffer memory reflected through the public stream fields. No memory may leak.

#### `uio_fread(buf, size, nmemb, stream) -> size_t`

Reads up to `nmemb` items of `size` bytes each. Returns the number of complete items read. Returns 0 on error or EOF. Sets `stream->status` to `STATUS_EOF` if fewer items than requested were available, or `STATUS_ERROR` on I/O failure. Sets `stream->operation` to `READ`.

#### `uio_fwrite(buf, size, nmemb, stream) -> size_t`

Writes `nmemb` items of `size` bytes each. Returns the number of complete items written. Sets error status on failure.

#### `uio_fseek(stream, offset, whence) -> c_int`

Seeks the stream. Flushes the buffer if the stream was in write mode. Discards the buffer if the stream was in read mode. Clears the EOF flag. Returns 0 on success, -1 on error.

#### `uio_ftell(stream) -> c_long`

Returns the current stream position. If the stream has buffered data, the returned position must account for the buffer (i.e., the position the caller perceives, not the underlying file descriptor position). Returns -1 on error.

#### `uio_fgets(buf, size, stream) -> *mut c_char`

Reads up to `size - 1` characters, stopping after a newline or at EOF. Null-terminates the buffer. Returns `buf` on success, null on EOF with no data read or on error.

#### `uio_fgetc(stream) -> c_int`

Reads and returns the next byte as an unsigned char cast to int. Returns `EOF` (-1) on end-of-file or error. Must set stream status accordingly.

#### `uio_ungetc(c, stream) -> c_int`

Pushes back one character onto the stream. Only one character of pushback is guaranteed. Returns the pushed-back character on success, `EOF` on failure.

**External contract:** The caller-visible requirement is that after `uio_ungetc(c, stream)`, the next `uio_fgetc` returns `c`, and stream position/buffer invariants (§4.5) are preserved. Any internal mechanism that achieves these observable semantics is acceptable.

#### `uio_fputc(c, stream) -> c_int`

Writes a single character. Returns the character written on success, `EOF` on error.

#### `uio_fputs(s, stream) -> c_int`

Writes a null-terminated string (without the null terminator). Returns non-negative on success, `EOF` on error.

#### `uio_fflush(stream) -> c_int`

Flushes pending writes to the underlying file. The public contract requires a valid non-null stream pointer; legacy UIO explicitly distinguishes this function from stdio `fflush(NULL)` and does not define null as a "flush all streams" operation. Returns 0 on success, `EOF` on error.

#### `uio_feof(stream) -> c_int`

Returns non-zero if the EOF indicator is set on the stream, 0 otherwise. The EOF indicator is set when a read operation encounters end-of-file. It is cleared by `uio_clearerr`, `uio_fseek`, or `uio_rewind`.

**Must not be hardcoded.** The return value must reflect `stream->status == STATUS_EOF`.

#### `uio_ferror(stream) -> c_int`

Returns non-zero if the error indicator is set on the stream, 0 otherwise. The error indicator is set when an I/O operation fails. It is cleared by `uio_clearerr`.

**Must not be hardcoded.** The return value must reflect `stream->status == STATUS_ERROR`.

#### `uio_clearerr(stream)`

Clears both the EOF and error indicators on the stream. Sets `stream->status = STATUS_OK`.

#### `uio_streamHandle(stream) -> *mut uio_Handle`

Returns the underlying file handle for a stream. Does not transfer ownership.

#### `uio_vfprintf(stream, format, args) -> c_int`

Writes formatted output to the stream. This requires C variadic argument handling.

**Usage evidence:** `initialstate.md` explicitly marks uncertainty around whether any production path materially depends on `uio_vfprintf`. This function is classified as **compatibility-complete** (not engine-critical) and should not block core parity work.

### 5.4 Directory Enumeration

#### `uio_getDirList(dir, path, pattern, matchType) -> *mut uio_DirList`

Enumerates entries in the directory at `path` relative to `dir`. Filters entries by `pattern` using the match mode specified by `matchType`:

| matchType | Value | Semantics |
|---|---|---|
| `match_MATCH_LITERAL` | 0 | Exact string match |
| `match_MATCH_PREFIX` | 1 | Entry name starts with `pattern` |
| `match_MATCH_SUFFIX` | 2 | Entry name ends with `pattern` |
| `match_MATCH_SUBSTRING` | 3 | Entry name contains `pattern` |
| `match_MATCH_REGEX` | 4 | POSIX extended regular expression |

**Regex support target:** full general-purpose regex matching compatible with the legacy POSIX ERE contract is part of compatibility-complete API parity, not merely the narrow currently proven startup patterns. The current implementation hard-codes only `.rmp` and `.zip`/`.uqm` patterns, which is insufficient for complete API coverage even though current engine-critical evidence proves only a subset of regex usage.

When the directory spans multiple mounts in startup/resource-loading cases whose need is already grounded by the evidence baseline and acceptance criteria, the enumeration must merge entries from the contributing mounts needed for those exercised cases, deduplicating by name. For any duplicate name, the higher-precedence visible entry wins wholesale, including entry kind. This requirement is about proven archive-backed visibility needs for current consumers; it does not by itself prove that every combined STDIO+archive merged-listing scenario is currently part of parity. Archive-internal per-directory enumeration within a mounted archive is required for archive parity once archive support is functional, but broader generalized cross-mount merged-directory behavior across arbitrary mount combinations remains a compatibility-complete provisional target rather than a fully parity-established contract.

**Result ordering (provisional — acceptance criteria pending verification):** The ordering of entries in a merged directory listing is not yet fully characterized from the legacy C implementation. For the general merged-listing contract, only mount-precedence ordering with first-seen deduplication is fixed today; no legacy-final intra-mount ordering rule is yet established for arbitrary listings. **Special current acceptance rule for `.rmp` discovery only:** see §5.1.6 and REQ-UIO-LIST-015/017, which temporarily require lexical-by-entry-name ordering within each contributing mount for deterministic `.rmp` acceptance/testing until legacy verification confirms or replaces that narrower rule. Outside that `.rmp` acceptance case, intra-mount ordering remains an open compatibility question — see §15, question 1.

**Current merged-list acceptance matrix:**

| Scenario | Backing types involved | Current status | Ordering / dedup rule | Basis |
|---|---|---|---|---|
| Package/archive discovery in startup paths | STDIO directories containing archive files | Required now | Visible entries by mount precedence with first-seen dedup; no additional legacy-final intra-mount rule established here | Proven startup consumer |
| Startup-visible `.rmp` discovery in currently exercised namespace | Startup-visible contributors after archive mounting | Required now | Dedup by name; provisional deterministic rule uses mount precedence + first-seen dedup + lexical-by-entry-name within each contributing mount | Proven startup consumer + interim acceptance rule |
| Archive-internal ordinary per-directory enumeration within a mounted archive namespace | Archive namespace only | Required now once archive support is functional | Ordinary directory enumeration with applicable precedence/dedup rules | Archive parity requirement |
| Ordinary listings where a proven current consumer depends on archive-backed assets being visible through merged namespace | Archive + other active contributors | Required now | Dedup by name; higher-precedence visible entry wins wholesale, including entry kind | Proven current consumer |
| Overlapping STDIO-only mounts with no proven current consumer dependency | STDIO + STDIO | Deferred / compatibility-complete | Deterministic behavior required, but legacy-final merged-list parity not yet elevated to engine-critical | Compatibility-complete scope |
| Broader combined-source `.rmp` discovery from both STDIO and archive contributors beyond proven startup-visible case | STDIO + archive | Deferred / compatibility-complete | Provisional deterministic `.rmp` rule in §5.1.6 / REQ-UIO-LIST-015 until legacy verification confirms or replaces it | Design-forward target + interim acceptance rule |

An empty pattern matches all entries.

Returns null if the directory cannot be opened. Returns a valid `uio_DirList` with `numNames = 0` if the directory is empty or no entries match.

#### `uio_DirList_free(dirList)`

Frees all memory associated with the directory listing: the name pointer array, the backing string storage, and the `uio_DirList` struct. Must handle null input gracefully (no-op).

### 5.5 Filesystem Metadata Operations

#### `uio_stat(dir, path, stat_buf) -> c_int`

Stats a file by name relative to `dir`. Resolves through the mount registry. Populates at minimum `st_size` and `st_mode`. Returns 0 on success, -1 on error.

#### `uio_access(dir, path, mode) -> c_int`

Tests file accessibility. `mode` follows POSIX `access()` semantics (`F_OK` for existence, `R_OK`, `W_OK`, `X_OK`). Returns 0 if accessible, -1 otherwise.

**Archive-backed entries:** For files within ZIP/UQM archives, `R_OK` shall succeed (archive content is readable), `W_OK` and `X_OK` shall fail (archive content is read-only and not executable), and `F_OK` shall succeed for entries present in the archive index. For directories synthesized from archive paths, `F_OK` and `R_OK` shall succeed.

#### `uio_mkdir(dir, path, mode) -> c_int`

Creates a directory. Fails on read-only mounts. Returns 0 on success, -1 on error.

#### `uio_rmdir(dir, path) -> c_int`

Removes an empty directory. Fails on read-only mounts. Returns 0 on success, -1 on error.

#### `uio_rename(oldDir, oldPath, newDir, newPath) -> c_int`

Renames or moves a file/directory. Both source and destination must resolve through writable mounts. Returns 0 on success, -1 on error.

#### `uio_unlink(dir, path) -> c_int`

Deletes a file. Fails on read-only mounts. Returns 0 on success, -1 on error.

### 5.6 File Location and Native-Path Semantics

#### `uio_getFileLocation(dir, inPath, flags, mountHandle, outPath) -> c_int`

Resolves `inPath` (relative to `dir`) to its physical location. Outputs the mount handle that owns the file and the resolved physical path. Returns 0 on success, -1 on error. The caller must free the `outPath` string.

**Behavior by backing type:**

- **STDIO-backed content (proven contract):** Returns the real filesystem path and the owning STDIO mount handle.
- **Archive-backed content (legacy implementation note):** The legacy helper stack in `utils.c` calls `uio_getFileLocation()` before deciding whether direct stdio access is possible or whether a temporary-copy fallback is required. However, the normative end-state contract for `uio_getFileLocation` is defined by `file-io/specification.md`, which specifies that `uio_getFileLocation` fails for archive-backed content (archive entries have no direct host-native path). In the end state, `uio_getStdioAccess` performs its own internal mount-type inspection to determine the backing type and initiate temp-copy when needed, without depending on public `uio_getFileLocation` succeeding for archive entries. This document defers to `file-io/specification.md` on the public behavioral contract of `uio_getFileLocation`.
- **Unresolvable content:** Returns -1 and sets `errno`.

### 5.7 Mount Management

#### `uio_mountDir(destRep, mountPoint, fsType, sourceDir, sourcePath, inPath, autoMount, flags, relative) -> *mut uio_MountHandle`

Mounts a filesystem or archive into the repository. See §5.1 for detailed semantics. Returns null on failure.

**`autoMount` parameter:** In the legacy C implementation, this supported automatic sub-mounting of detected archives. In UQM's current usage, `autoMount` is always `{ NULL }`. The implementation may accept and ignore this parameter, but must not crash on non-null input.

#### `uio_transplantDir(mountPoint, sourceDir, flags, relative) -> *mut uio_MountHandle`

Re-mounts the backing subtree currently referenced by `sourceDir` at a new `mountPoint` within the same repository. In the only observed in-tree use (`options.c:575-589`), `sourceDir` is an opened addon `shadow-content` directory and transplant is called with `uio_MOUNT_RDONLY | uio_MOUNT_ABOVE` relative to `contentMountHandle`. The current acceptance contract is therefore that transplant must preserve that resolved backing content view closely enough for addon shadow-content overlay semantics to work as observed. Broader guarantees about original-mount identity, non-STDIO backing types, or additional flag combinations are not yet fully verified and remain compatibility-complete rather than engine-critical. Returns null on failure.

#### `uio_unmountDir(mountHandle) -> c_int`

Removes a single mount from the repository. Deallocates the mount handle. Returns 0 on success, -1 on error.

#### `uio_unmountAllDirs(repository) -> c_int`

Removes all mounts from the repository. Called during `uio_closeRepository`. Returns 0 on success.

#### `uio_getMountFileSystemType(mountHandle) -> c_int`

Returns the filesystem type (`uio_FSTYPE_STDIO` or `uio_FSTYPE_ZIP`) of the mount.

### 5.8 Lifecycle

#### `uio_init()`

Initializes the UIO subsystem. Registers default filesystem handlers (STDIO, ZIP).

**Idempotency (proposed design):** Idempotent initialization is a desired improvement for robustness. The legacy C contract does not clearly establish that consumers depend on idempotent init, but making it safe to call more than once is a reasonable defensive choice.

#### `uio_unInit()`

Tears down the UIO subsystem. Unregisters filesystem handlers. Releases global state.

#### `uio_openRepository(flags) -> *mut uio_Repository`

Creates a new repository. Returns null on failure.

#### `uio_closeRepository(repository)`

Unmounts all directories in the repository, then frees the repository.

#### 5.8.1 Handle and mount lifecycle interactions

This section defines the contract for object validity across lifecycle transitions.

**Externally observable contract:**

- After `uio_closeRepository` returns, the repository pointer is invalid and must not be used.
- Mounts owned exclusively by the closed repository are no longer usable for path resolution, directory enumeration, or unmount operations after repository close completes.
- `uio_unmountDir` removes one mount from path resolution. After unmount returns, the mount handle is invalid and must not be used as a relative-placement anchor.
- Any internal cleanup sequence that achieves those observable effects without leaking repository-owned resources is acceptable unless stronger legacy evidence is established.

**Proposed safe-failure robustness target (provisional — not verified legacy behavior):**

The following behaviors describe a proposed robustness target for post-unmount handle semantics. They are defensive design goals, not verified legacy contract. Whether the legacy C implementation enforced these behaviors or relied on callers not exercising them is unknown (see `initialstate.md`). The hard requirement is: none of these scenarios may crash the process or produce undefined behavior. Exact post-unmount functional behavior (specific error codes, handle state transitions) remains provisional and acceptance-test-defined pending legacy verification.

- **Directory handles after unmount:** If a caller holds a `uio_DirHandle` and the underlying mount is unmounted, subsequent operations using that directory handle (e.g., `uio_open`, `uio_getDirList`) must not crash or produce undefined behavior. The directory handle itself remains a valid allocated object until `uio_closeDir` is called, but path resolution through it may fail because the mount is gone. Whether post-unmount operations return specific error codes or exhibit other defined functional behavior beyond the no-crash/no-UB guarantee is acceptance-test-defined pending legacy verification. This is a proposed safe-failure robustness target, not a proven compatibility claim.
- **File handles and streams after unmount:** If a file handle or stream is open when its backing mount is unmounted, the minimum required safety floor is that subsequent operations through that already-open object must not crash the process or produce undefined behavior. Beyond that safety floor, the exact post-unmount functional behavior for already-open file handles and streams is not yet established: implementations may keep the object usable until close, or may cause subsequent operations to fail cleanly with documented error signaling, but they must not return misleading success or rely on undefined behavior. This remains an open compatibility question that must be finalized through legacy verification. Callers should still close all file handles and streams before unmounting the backing mount. UQM's observed shutdown ordering closes files before unmounting.
- **Directory handles do not pin mounts:** A live `uio_DirHandle` does not prevent unmounting of the mount it was opened through. The directory handle retains its path and repository association but does not extend the mount's lifetime.
- **Shutdown ordering:** UQM's observed shutdown sequence closes files/streams first, then closes directory handles, then unmounts, then closes the repository. The UIO subsystem does not enforce this ordering — correct ordering is the caller's responsibility. If callers violate the expected ordering, UIO must not crash or exhibit undefined behavior. Exact post-unmount functional behavior beyond that safety floor remains provisional and acceptance-test-defined pending legacy verification.


### 5.9 FileBlock API

The FileBlock API provides a block-access abstraction over file handles, potentially backed by memory mapping or read-ahead caching.

**Evidence basis:** The legacy ZIP implementation in `sc2/src/libs/uio/zip/zip.c` uses FileBlock APIs (`uio_openFileBlock2`, `uio_copyFileBlock`, `uio_accessFileBlock`, `uio_setFileBlockUsageHint`) extensively as an internal dependency. However, whether any engine-critical runtime path exercises FileBlock through the exported public API is not established by `initialstate.md`. The Rust bridge currently stubs all FileBlock entry points.

**Classification:** FileBlock is **compatibility-complete** as an exported API surface. If the Rust implementation provides ZIP/UQM archive support through a different internal architecture that does not rely on FileBlock, the exported FileBlock APIs remain compatibility-complete symbols that must exist for API coverage but are not on the critical path for archive parity. These APIs are required for supported-surface completeness, but they are not current engine-viability or parity-acceptance gates unless future evidence or implementation choices promote them. If a future implementation choice preserves the legacy ZIP layering where archive reading depends on FileBlock internally, FileBlock would need to be elevated accordingly.

**All behavioral descriptions below are proposed design** based on API signatures and header comments. They represent reasonable implementation targets, not proven legacy contract. Until consumer usage or legacy behavior is confirmed through code inspection or runtime testing, implementations should follow this design but treat it as revisable.

#### `uio_openFileBlock(handle) -> *mut uio_FileBlock`

Creates a FileBlock for the given file handle. Returns null on failure.

#### `uio_openFileBlock2(handle, offset, size) -> *mut uio_FileBlock`

Creates a FileBlock constrained to the specified file subrange. Returns null on failure.

#### `uio_closeFileBlock(block) -> c_int`

Closes and frees a FileBlock. Returns 0 on success.

#### `uio_accessFileBlock(block, offset, length, flags) -> isize`

Requests access to a range of the file. Returns a non-negative value on success, -1 on error.

#### `uio_copyFileBlock(block, offset, buffer, length) -> c_int`

Copies data from the file into `buffer`. Returns 0 on success, -1 on error.

#### `uio_setFileBlockUsageHint(block, usage, flags)`

Provides a hint about future access patterns. No-op if the implementation does not support hints.

### 5.10 Stdio Access Bridge

**Evidence basis:** The StdioAccess API is used when external libraries need a real filesystem path that they can open independently (bypassing UIO). Legacy in-tree usage exists: `sc2/src/libs/uio/utils.c:166–210` implements a helper around `uio_getFileLocation` and `uio_getMountFileSystemType` that feeds into StdioAccess patterns, and `sc2/src/libs/uio/debug.c:472` calls `uio_getStdioAccess`. However, whether any engine-critical runtime path depends on StdioAccess is not established. This API is classified as **compatibility-complete** for prioritization purposes, but it is still part of the supported in-tree exported surface and should not be treated as optional or speculative.

**Proven contract:** The API must accept a virtual path, and for STDIO-backed content, return a handle exposing the real filesystem path. The handle's path must remain valid for the handle's lifetime. `uio_releaseStdioAccess` must clean up resources.

**Note on legacy provenance:** The archive-backed behavior described below was initially classified as "proposed design" in early UIO analysis because full legacy verification was pending. That provenance classification is historical context only. The normative end-state contract for `uio_getStdioAccess` is defined by `file-io/specification.md`, which makes archive-backed temp-copy behavior mandatory. This document defers to that normative contract.

#### `uio_getStdioAccess(dir, path, flags) -> *mut uio_StdioAccessHandle`

For STDIO-backed files, returns a handle that provides the real filesystem path. Returns null on failure.

**Archive-backed content (mandatory per `file-io/specification.md`):** `uio_getStdioAccess()` shall not simply fail for non-STDIO-backed content. It uses internal mount-type inspection to determine the backing type, and if the content is not STDIO-backed, creates a temporary directory, copies the file there, and returns a usable stdio path to the copied file. This is mandatory end-state behavior: direct-path success for STDIO-backed content, temp-copy-backed stdio access for non-STDIO content when internal resolution succeeds, and clean failure only when temporary-directory creation, file copy, or temporary-path resolution fails. See `file-io/specification.md` for the authoritative behavioral contract.

#### `uio_releaseStdioAccess(handle)`

Releases the access handle. If temporary resources were created to satisfy the request, cleans up those resources.

#### `uio_StdioAccessHandle_getPath(handle) -> *const c_char`

Returns the real filesystem path. The returned pointer is valid for the lifetime of the handle.

---

## 6. Error and Status Semantics

### 6.1 Return conventions

- Functions returning `c_int`: 0 = success, -1 = error (matching POSIX convention).
- Functions returning pointers: null = error.
- `uio_fread` / `uio_fwrite`: return item count; 0 on error/EOF.
- `uio_fgetc`: returns character value 0–255 on success, -1 (EOF) on end-of-file or error.

### 6.2 `errno` setting

Functions that fail shall set `errno` to an appropriate POSIX error code before returning. This is required because C-side consumers check `strerror(errno)` after failures (visible in `sdluio.c` and `options.c`).

Functions that succeed may leave `errno` in an unspecified state. Callers should not inspect `errno` after a successful return.

#### 6.2.1 Required error categories

The following error categories must produce meaningful `errno` values. Specific POSIX codes are representative; equivalent codes are acceptable as long as `strerror()` produces a meaningful message for the failure class.

| Failure class | Expected `errno` | Applies to |
|---|---|---|
| File or directory not found | `ENOENT` | `uio_open`, `uio_fopen`, `uio_stat`, `uio_access`, `uio_unlink`, `uio_rename`, `uio_getDirList` (target dir) |
| Write to read-only mount or archive content | `EROFS` or `EACCES` | `uio_open` (write flags), `uio_write`, `uio_unlink`, `uio_mkdir`, `uio_rmdir`, `uio_rename` |
| Invalid argument (null pointer, invalid flags) | `EINVAL` | All public APIs receiving pointer or flag arguments |
| Unsupported operation on backing type | `ENOTSUP` or `ENOSYS` | Write operations on archive-backed content, operations not meaningful for the filesystem type |
| I/O error during read/write/seek | `EIO` | `uio_read`, `uio_write`, `uio_fread`, `uio_fwrite`, `uio_lseek`, `uio_fseek` |
| Directory not empty (rmdir) | `ENOTEMPTY` | `uio_rmdir` |
| File/directory already exists (exclusive create) | `EEXIST` | `uio_open` with `O_CREAT | O_EXCL`, `uio_mkdir` |
| Archive parse/open failure during mount | `EIO` or `EINVAL` | `uio_mountDir` with `uio_FSTYPE_ZIP` |

**Defensive invalid-pointer cases:** When a public API receives an obviously invalid pointer (null handle, null stream), the function must fail safely and return the error sentinel. Setting `errno` to `EINVAL` is appropriate. For invalid but non-null pointer arguments that cannot be reliably detected without dereference (e.g., dangling pointers), the function should fail safely to the extent possible but `errno` setting is best-effort.

### 6.3 Stream status

`uio_Stream.status` must be maintained as a state machine:

```
STATUS_OK (0) — normal operating state
STATUS_EOF (1) — set when a read encounters end-of-file
STATUS_ERROR (2) — set when an I/O error occurs

Transitions:
  - uio_fread returning short: STATUS_OK -> STATUS_EOF
  - uio_fread/fwrite I/O error: any -> STATUS_ERROR
  - uio_fseek success: any -> STATUS_OK (clears EOF and error)
  - uio_clearerr: any -> STATUS_OK
```

**Proven contract:** The status field values and their meanings are defined in `uiostream.h`. The transition from STATUS_OK to STATUS_EOF on short read and STATUS_ERROR on I/O failure is the documented contract. The `uio_fseek` clearing behavior follows C stdio convention and is consistent with the header contract.

**Proposed design (provisional):** The exact set of intermediate states during compound operations (e.g., whether a failed flush during `uio_fseek` leaves status in ERROR or attempts recovery) is not fully characterized from legacy behavior and should be implemented conservatively — fail to ERROR and let the caller recover via `uio_clearerr`. This is an implementation recommendation, not a proven legacy obligation.

### 6.4 Error propagation through SDL_RWops

The SDL adapter in `sdluio.c` distinguishes zero-byte reads from errors:

```c
numRead = uio_fread(ptr, size, maxnum, stream);
if (numRead == 0 && uio_ferror(stream)) {
    SDL_SetError("Error reading from uio_Stream: %s", strerror(errno));
    return 0;
}
```

This means `uio_ferror` must return accurate error state. If `uio_fread` returns 0 due to EOF (not an error), `uio_ferror` must return 0 so the caller can distinguish the two cases.

### 6.5 Stubbed exported API failure behavior

Exported APIs that are not yet implemented must fail cleanly and explicitly. They must not return placeholder success objects (non-null handles, dummy pointers, or zero return codes) that defer failure to a later unrelated call site. Specifically:

- The function must return the public failure sentinel for its return type (null for pointer-returning functions, -1 for int-returning functions).
- The function must set `errno` to a meaningful value (e.g., `ENOTSUP`) when the API contract expects error reporting.
- The function must not return non-null dummy handles or fake success indicators that cause callers to proceed as if the operation succeeded.

This applies to all currently stubbed exported APIs, including but not limited to FileBlock, StdioAccess, and any other compatibility-complete APIs that are not yet functionally implemented. The intent is to ensure that integration tests and callers discover unimplemented functionality immediately at the call site rather than experiencing mysterious failures downstream.

---

## 7. Ownership and Memory Management

### 7.1 Allocation ownership rules

| Object | Allocator | Deallocator | Owner |
|---|---|---|---|
| `uio_Repository` | `uio_openRepository` | `uio_closeRepository` | Caller (C startup) |
| `uio_MountHandle` | `uio_mountDir` / `uio_transplantDir` | `uio_unmountDir` or `uio_unmountAllDirs` | Repository |
| `uio_DirHandle` | `uio_openDir` / `uio_openDirRelative` | `uio_closeDir` | Caller |
| `uio_Handle` | `uio_open` | `uio_close` | Caller |
| `uio_Stream` | `uio_fopen` | `uio_fclose` | Caller |
| `uio_Stream.buf` | `uio_fopen` (or lazy on first I/O) | `uio_fclose` | Stream |
| `uio_Stream.handle` | `uio_fopen` (internally) | `uio_fclose` (internally) | Stream |
| `uio_DirList` | `uio_getDirList` | `uio_DirList_free` | Caller |
| `uio_DirList.names` | `uio_getDirList` | `uio_DirList_free` | DirList |
| `uio_FileBlock` | `uio_openFileBlock` | `uio_closeFileBlock` | Caller |
| `uio_StdioAccessHandle` | `uio_getStdioAccess` | `uio_releaseStdioAccess` | Caller |

### 7.2 No leaks on close

`uio_fclose` must free the stream buffer, close the underlying file handle, and free the stream struct. Zero memory leaks. The current implementation acknowledges leaking the stream buffer — this must be fixed.

### 7.3 Self-describing allocation (non-normative preference)

**Observable requirement:** `uio_DirList_free` must correctly deallocate all memory owned by the listing without leaking.

**Preferred approach (non-normative):** The allocation strategy should be self-describing: either by wrapping the C-compatible struct in an internal structure that carries metadata, or by using language-native allocation facilities that track size. This avoids the fragility of global side-channel registries that could become inconsistent. The current global `HashMap` side-channel is acceptable as a transitional mechanism but is not the desired end-state.

Any internal approach that achieves correct, leak-free deallocation without introducing observable defects is acceptable. The preference for self-describing allocation is an implementation quality guideline, not an externally observable contract.

### 7.4 FFI safety

All exported functions must:

- Validate pointer arguments for null before dereferencing.
- Never panic across the FFI boundary. All panics must be caught at the FFI boundary before crossing into foreign code.
- Use appropriate heap-ownership-transfer patterns for allocated objects.
- Not create aliased mutable references to the same data.

The current defensive pointer guards (low-address checks, panic-catch around handle dereference) are appropriate safety measures and should be retained.

---

## 8. ZIP/UQM Archive Behavior

Archive support is the largest functional gap in the current Rust port. This section breaks archive parity into explicit sub-capabilities, each of which must be independently functional for archive support to be considered complete.

### 8.0 Minimum archive parity acceptance criterion

Archive support is the highest-risk area in the UIO port. All lower-level archive requirements in this section and in §14 Tier 1 roll up to a single end-to-end acceptance criterion:

> **After C startup code mounts package archives via `uio_mountDir` with `uio_FSTYPE_ZIP`, real engine consumers — including `sdluio.c`, Rust sound decoders, and the Rust resource bridge — must be able to discover, open, read, seek, and query archive-backed assets through the standard exported UIO APIs (`uio_getDirList`, `uio_fopen`, `uio_fread`, `uio_fseek`, `uio_ftell`, `uio_feof`, `uio_ferror`, `uio_open`, `uio_read`, `uio_fstat`) without requiring any special-case behavior or caller-side awareness that the content is archive-backed.**

This criterion is testable: mount a real `.uqm` package archive, enumerate its contents via `uio_getDirList`, open a known asset via `uio_fopen`, read its full contents via `uio_fread`, verify seek/tell consistency, and confirm that `uio_feof`/`uio_ferror` report correct status. All operations must behave identically to the same operations over STDIO-backed content from the consumer's perspective.

### 8.1 Archive sub-capabilities

Archive parity requires the following capabilities, listed in dependency order:

1. **Mount registration** — `uio_mountDir` with `uio_FSTYPE_ZIP` must register the archive mount as fully active in the mount registry. Currently, ZIP mounts are registered but set as inactive for path resolution. This must be fixed.

2. **Archive entry index construction** — When a ZIP mount is registered, the implementation must parse the archive's directory and construct an index of all entries (files and directories) within the archive. This index must persist for the lifetime of the mount.

3. **Path resolution into archive entries** — The mount-registry path-resolution logic (§5.1.4) must search ZIP mounts alongside STDIO mounts according to mount priority. A virtual path that resolves to an archive entry must be found and returned.

4. **Directory enumeration across archive and non-archive mounts** — `uio_getDirList` must merge entries from ZIP mounts and STDIO mounts that contribute to the same virtual directory for the startup/resource-consumer cases that actually rely on archive-backed content becoming visible through ordinary listings. Archive-contributed entries must appear in those required listings alongside filesystem entries, deduplicated by name. Full generalized merged-directory parity across arbitrary mount combinations remains compatibility-complete rather than part of the minimum engine-critical archive acceptance boundary. See also §5.1.6 for `.rmp` discovery implications.

5. **`stat`/metadata behavior for archive entries** — `uio_stat` and `uio_fstat` must return meaningful metadata for archive-backed files, including at minimum `st_size` (uncompressed size) and `st_mode` (regular file, read-only).

6. **`uio_open`/`uio_read`/`uio_lseek` behavior for archive content** — Opening a file descriptor for archive-backed content must produce a handle that supports read and seek operations over the decompressed content. Write operations must fail.

7. **`uio_fopen`/stream behavior for archive content** — Opening a buffered stream for archive-backed content must produce a stream with full read/seek/tell/EOF/error semantics, identical to streams over STDIO-backed content from the caller's perspective.

8. **`uio_access` for archive entries** — See §5.5 for archive-specific `uio_access` semantics.

9. **`uio_getFileLocation` for archive entries** — See §5.6 for archive-specific file-location semantics.

10. **`uio_getStdioAccess` for archive entries** — See §5.10 for archive-backed native-path semantics.

### 8.2 Archive detection and mounting

Archives are `.zip` or `.uqm` files (case-insensitive extension matching). The C startup code discovers them via `uio_getDirList` with the regex `\\.([zZ][iI][pP]|[uU][qQ][mM])$` and then calls `uio_mountDir` with `uio_FSTYPE_ZIP`.

### 8.3 Archive format support

ZIP archives with the following are expected:

- Stored (uncompressed) entries
- Deflate-compressed entries
- Entries with directory paths
- Entry-name resolution sufficient to locate existing package content as exercised by real engine asset paths. Whether legacy ZIP lookup was generally case-insensitive for entry-name resolution is not established by the current evidence baseline; this remains an open compatibility question rather than a settled universal contract. Until verified, parity acceptance requires only enough entry-name matching fidelity to locate the real package content exercised by known engine asset paths.

### 8.4 Archive mount resolution

When a ZIP mount is registered, its contents become part of the virtual namespace. Path resolution must search ZIP mounts alongside STDIO mounts according to mount priority. Currently, ZIP mounts are explicitly set as inactive when mounted with a source directory, which prevents them from participating in path resolution. At parity, ZIP mounts must be fully active.

### 8.5 Reading from archives

When `uio_open` or `uio_fopen` resolves to a file inside a ZIP archive:

1. Locate the entry in the archive's entry index.
2. If stored: create a handle that reads directly from the archive file at the entry's offset.
3. If deflated: decompress the entry (either fully into memory, or lazily with a streaming decompressor). The implementation may choose any decompression strategy that produces correct decompressed content accessible through the handle.
4. The handle must support `read`, `seek`, `fstat` (reporting the uncompressed size).
5. Write operations must fail.

### 8.6 Integration requirement

Archive mount parity is not complete when path resolution alone works. Resource consumers must be able to read package content through the same exported APIs (`uio_fopen`, `uio_fread`, `uio_fseek`, `uio_ftell`, `uio_feof`, `uio_ferror`, etc.) after startup completes. The archive mount must function end-to-end through the API surface used by `sdluio.c`, Rust sound decoders, and the Rust resource bridge.

---

## 9. Pattern Matching

### 9.1 Match types

The match engine must support all types defined in `match.h`:

- **LITERAL (0):** Exact byte-for-byte match between pattern and name.
- **PREFIX (1):** Name starts with pattern.
- **SUFFIX (2):** Name ends with pattern.
- **SUBSTRING (3):** Name contains pattern.
- **REGEX (4):** POSIX extended regular expression. The pattern is compiled once and applied to each candidate name. A match anywhere in the name counts (equivalent to `regexec` without `REG_NOSUB`).

### 9.2 Case sensitivity

LITERAL, PREFIX, SUFFIX, and SUBSTRING matches are case-sensitive (matching C `strcmp`/`strncmp` behavior). REGEX matching follows POSIX ERE rules (case-sensitive unless the pattern uses character classes like `[rR]`).

### 9.3 Empty pattern

An empty pattern matches all entries for all match types.

---

## 10. Concurrency

### 10.1 Thread safety requirements

UQM is primarily single-threaded for game logic but may have concurrent audio decoding threads.

**Proven contract (minimum required safety):** The mount registry must be safe for concurrent read access from audio threads that resolve paths while the main thread is not mutating mounts. Individual file handles used by audio threads must be safe for concurrent operations on separate handles.

**Proposed design (stronger desired guarantee):** Full same-handle concurrent safety (multiple threads operating on the same file handle or stream simultaneously) is a desirable robustness property, but the degree to which current consumers exercise same-handle concurrency is not fully established. `initialstate.md` notes possible concurrent audio decoding threads, but does not demonstrate same-handle sharing between threads. Until same-handle concurrent usage is demonstrated by actual consumers or tests, full same-handle safety is a desired improvement rather than a proven legacy obligation.

### 10.2 Lock granularity

The mount registry lock should be held for the minimum duration necessary — acquire, perform the lookup or mutation, release. File operations should not hold the mount registry lock.

Individual file handle synchronization is per-handle. Operations on different handles do not contend.

---

## 11. Logging and Diagnostics

These diagnostic interfaces are **compatibility-complete, non-blocking**. They must exist as callable symbols for consumers that reference them, but they are not tied to proven runtime-critical consumer evidence. Their output format and behavior are not tightly constrained by legacy contract.

If specific consumer invocation evidence is established in the future, these items may be reclassified.

### 11.1 `uio_printMounts(outStream, repository)`

Prints a human-readable summary of all mounts in the repository to the given `FILE*` stream. Useful for debugging mount topology.

### 11.2 `uio_DirHandle_print(dirHandle, outStream)`

Prints the directory handle's path and metadata.

### 11.3 Bridge logging

The current `rust_bridge_log_msg` / `log_marker` pattern is appropriate for development diagnostics. Production builds should minimize logging to error conditions only.

### 11.4 `rust_bridge_log_msg_c(message) -> c_int`

Exported C-ABI function allowing C code to write to the Rust bridge log. Returns 0 on success, -1 on failure.

---

## 12. Internal Rust API (Future)

### 12.1 Native Rust consumers

Currently, Rust subsystems (sound decoders, resource bridge, audio heart) consume UIO through FFI `extern "C"` declarations, treating it as an external C library. As a **desired improvement**, UIO should additionally expose a safe Rust API for internal consumers, providing idiomatic Rust types and error handling over the same internal implementation that the C ABI functions use.

This is a structural improvement goal, not a functional parity requirement. The C ABI surface must be complete and correct first.

---

## 13. Build Integration

### 13.1 Feature flag

The `USE_RUST_UIO` preprocessor macro, defined in `config_unix.h`, selects the Rust implementation. When defined:

- `io.c` and `uiostream.c` are excluded from the C build.
- The C UIO library is reduced to: `charhashtable.c`, `paths.c`, `uioutils.c`.
- At port completion, the `uio_fread_shim.c` file should be eliminated (currently still present as a transitional artifact).
- The Rust static library provides all `uio_*` symbols.

### 13.2 Symbol completeness

At port completion, the Rust crate should export every symbol that C code links against without requiring C shim files. The `uio_fread` function should be exported directly from Rust as `uio_fread`, not as `rust_uio_fread`. This is a **port-completeness** goal (see §1.1.1).

### 13.3 Remaining C helper files

`charhashtable.c`, `paths.c`, and `uioutils.c` provide utility functions used by other C subsystems (not by UIO itself). These remain part of the C build and are outside the scope of the UIO port.

---

## 14. Parity Priority Tiers

Requirements and capabilities are grouped into priority tiers based on engine impact. Implementation effort should focus on higher tiers first.

### Tier 1 — Engine-critical

These items block correct engine operation. Without them, the game cannot load content or run correctly.

| Capability | Current State | Known Consumer |
|---|---|---|
| ZIP mount activation and path resolution | Registered but inactive | `options.c` archive mounting, all content loading |
| Archive content reading (open/read/seek) | Not functional for ZIP mounts | `sdluio.c`, Rust sound decoders, Rust resource bridge |
| Stream EOF/error state (`uio_feof`, `uio_ferror`) | Hardcoded stubs | `sdluio.c` error distinction |
| Mount precedence correctness (TOP/BOTTOM/ABOVE/BELOW basic cases) | Partial (path-length sort) | `options.c` mount orchestration |
| `errno` setting on failure (see §6.2.1) | Not set | `sdluio.c`, `options.c` error reporting |
| Stream buffer deallocation on close | Leaks | All stream consumers |
| Error propagation through SDL_RWops | Broken (hardcoded ferror) | `sdluio.c` |

### Tier 2 — Compatibility-complete

These items are needed for full API coverage and correct edge-case behavior, but are not immediate engine blockers.

| Capability | Current State | Notes |
|---|---|---|
| Cross-mount directory enumeration (generalized merge) | Single directory only | Engine-critical *narrow* need (archive + STDIO merge for package/index discovery) is covered by Tier 1 archive support. The broader generalized merged-directory contract across arbitrary mount combinations is compatibility-complete. |
| `uio_clearerr` | No-op stub | Needed for correct stream-status recovery, but no proven engine-critical caller dependency on retry-after-clear. |
| `uio_transplantDir` | Partial | Used by `options.c` for addon shadow-content mounting. Required for full addon support, but not a startup-or-runtime viability blocker for base content loading (see §2 item 10). |
| General regex directory matching | Hard-coded patterns only | Currently only `.rmp` and `.zip`/`.uqm` patterns work |
| `uio_access` full mode checking | Existence only | Needed for correct write-guard semantics |
| `uio_DirList_free` self-describing allocation | Side-channel registry | Correctness risk if registry gets out of sync |
| Metadata accuracy for archive entries | Not applicable (archives inactive) | Depends on Tier 1 archive support |
| `uio_getFileLocation` for archive content | Undefined | Supported exported surface; not proven startup-critical, but used by in-tree native-path-bridging helpers. Current target is success sufficient for helper-stack dispatch and owning-mount identification; exact returned path/string shape remains open — see §5.6, §15 |
| `uio_getStdioAccess` for archive content | Stub | Supported exported surface for in-tree helper/debug consumers; not proven startup-critical. Current target is temp-copy-backed stdio access when direct native path access is unavailable; reuse/caching details remain open — see §5.10, §15 |
| FileBlock API | Stubs | In-tree usage in legacy ZIP code; engine-critical runtime dependence on exported API not established (see §5.9) |
| StdioAccess API | Stubs | Supported in-tree helper/debug surface; engine-critical runtime dependence not established, but not optional (see §5.10) |
| Diagnostics (`uio_printMounts`, `uio_DirHandle_print`) | Minimal | Must exist as callable symbols; no proven runtime-critical dependency |

### Tier 3 — Quality/cleanup

These items improve code quality, developer experience, or forward maintainability.

| Capability | Current State | Notes |
|---|---|---|
| `uio_fread` direct export (port-completeness) | Requires C shim | Observable behavior is identical with or without shim |
| `uio_vfprintf` | Stub returning -1 | Diagnostics/debug only; no proven production dependency |
| Native Rust API wrappers | Not present | Structural improvement for Rust consumers |
| GPDir/GPFile/PRoot internal API | Stubs | Needed for ZIP mount internals (overlaps Tier 1) |

---

## 15. Open Compatibility Questions

The following questions are not yet resolved by the current evidence in `initialstate.md` or the legacy C headers. They should be answered by testing against the C implementation or by code inspection before the relevant features are implemented.

1. **Directory listing order for merged mounts.** What ordering does the legacy C implementation produce when directory entries come from multiple mounts? Is it mount-precedence order, filesystem order, lexical order, or some other arrangement? See §5.4. Behaviorally significant for `.rmp` load order — see §5.1.6.

2. **`uio_getFileLocation` for archive-backed content.** The remaining open question is the exact path/string shape returned for a file inside a ZIP archive once location discovery succeeds. Current evidence already establishes that helper-stack consumers expect success sufficient to identify the owning mount and dispatch follow-on stdio-access behavior; what remains unresolved is whether the returned location is an archive path, a member pseudo-path, or another path-like representation. See §5.6.

3. **`uio_vfprintf` usage.** Is `uio_vfprintf` called in any meaningful runtime path, or only in diagnostics/debug code? If only debug, it can remain a low-priority item.

4. **Multiple independent repositories.** Does any current or historical consumer exercise independent multi-repository behavior, or is the single-repository pattern the only exercised contract?

5. **`uio_ungetc` on non-seekable streams.** Does UQM ever exercise `ungetc` on a stream backed by non-seekable content? If not, the pushback-slot requirement is a correctness concern but not an urgent compatibility blocker.

6. **`uio_fflush(NULL)` legacy behavior.** Resolved by code inspection: legacy UIO does **not** accept NULL in `uio_fflush`; `uiostream.c` explicitly distinguishes this from stdio and asserts non-null.

7. **`uio_getStdioAccess` for archive-backed content.** Current evidence establishes temp-copy-backed stdio access rather than clean-failure-by-default. The remaining open questions are implementation details such as whether repeated access for the same file reuses/caches a previous materialization, what exact lifecycle/cleanup guarantees apply to temporary copies, and whether any additional backing-type-specific edge cases exist beyond the helper flow observed in `utils.c`. See §5.10.

8. **Mount-point specificity vs. insertion order edge cases.** When a shorter mount point inserted with `TOP` competes with a longer, more-specific mount point, which wins in the legacy C implementation? Is resolution purely precedence-order, or does path specificity factor in? This question is referenced by §4.1, §5.1.2, and §5.1.4 — those sections are provisionally specified pending the answer.

9. **`.rmp` files inside archives.** Do production content packages actually contain `.rmp` files inside ZIP/UQM archives, or do `.rmp` files only exist in STDIO-backed directories? This determines whether the combined STDIO+archive `.rmp` discovery path is exercised in practice. See §5.1.6.

---

## 16. Traceability Map

This section provides a compact cross-reference from observed gaps/defects in `initialstate.md` to specification targets and normative requirements.

### Tier 1 — Engine-critical traceability

| Observed gap (initialstate.md) | Specification target | Requirement(s) | Acceptance evidence |
|---|---|---|---|
| ZIP mounts registered but inactive in Rust registry | §5.1.5, §8.1 item 1, §8.4 | REQ-UIO-ARCHIVE-001, REQ-UIO-ARCHIVE-002 | Mount a `.uqm` archive; verify path resolution finds archive entries |
| Archive content unreadable through UIO APIs | §8.1 items 6–7, §8.5, §8.6 | REQ-UIO-ARCHIVE-003, REQ-UIO-ARCHIVE-008, REQ-UIO-ARCHIVE-ACCEPT | Open a known asset inside a mounted archive via `uio_fopen`; read full contents; verify seek/tell consistency |
| `uio_feof` hardcoded to 1 | §5.3 `uio_feof`, §6.3 | REQ-UIO-STREAM-007, REQ-UIO-ERR-005 | Read to EOF; verify `uio_feof` returns non-zero. Read partial; verify `uio_feof` returns 0. |
| `uio_ferror` hardcoded to 0 | §5.3 `uio_ferror`, §6.3, §6.4 | REQ-UIO-STREAM-008, REQ-UIO-ERR-006 | Trigger I/O error; verify `uio_ferror` returns non-zero. On success, verify it returns 0. |
| `errno` not set on failure | §6.2, §6.2.1 | REQ-UIO-ERR-002 | Call `uio_open` with nonexistent path; verify `errno == ENOENT`. Call write on read-only mount; verify `errno == EROFS` or `EACCES`. |
| Stream buffer leaks on `uio_fclose` | §7.2 | REQ-UIO-MEM-004 | Open/close stream under memory sanitizer; verify zero leaks |
| SDL_RWops error propagation broken | §6.4 | REQ-UIO-INT-003, REQ-UIO-STREAM-008 | Trigger read error through `sdluio.c`; verify `SDL_SetError` is called |
| Mount precedence uses path-length sort | §5.1.2 | REQ-UIO-MOUNT-002, REQ-UIO-MOUNT-003 | Mount A with TOP, mount B with BOTTOM; verify A wins for overlapping paths |
| Startup-visible `.rmp` discovery in the currently proven acceptance boundary | §5.1.6, §5.4 | REQ-UIO-LIST-016, REQ-UIO-LIST-017 | Mount the startup-visible content configuration exercised by current evidence; enumerate `.rmp`; verify the required startup-visible files appear with the documented provisional deterministic ordering and dedup behavior. This Tier 1 check does not attempt to prove the deferred broader combined STDIO+archive `.rmp` case governed by REQ-UIO-LIST-015. |

### Tier 2 — Compatibility-complete traceability (selected items)

| Observed gap (initialstate.md) | Specification target | Requirement(s) | Acceptance evidence |
|---|---|---|---|
| `uio_clearerr` is no-op | §5.3 `uio_clearerr`, §6.3 | REQ-UIO-STREAM-009 | Set error; call `uio_clearerr`; verify status cleared |
| Regex matching hard-coded | §9.1 | REQ-UIO-LIST-009 | Apply novel regex pattern via `uio_getDirList`; verify correct matches |
| `uio_DirList_free` uses side-channel | §7.3 | REQ-UIO-MEM-005 | Free listing under sanitizer; verify correct deallocation |

---
### Tier 3 — Quality/cleanup traceability (selected items)

| Observed gap (initialstate.md) | Specification target | Requirement(s) | Acceptance evidence |
|---|---|---|---|
| `uio_fread` requires C shim | §13.2 | REQ-UIO-STREAM-018 | Verify `uio_fread` symbol exported directly from Rust crate |

---



## 17. Summary of Behavioral Parity Requirements

This summary preserves the same tier model used elsewhere in the document. The sections below are not all equal current acceptance blockers: Tier 1 items are current engine-critical gates, Tier 2 items are compatibility-complete supported-surface obligations for final subsystem completion, and the cleanup section collects non-behavioral port-completeness work.

### 17.1 Current engine-critical parity gates

| Capability | Current State | Required end state |
|---|---|---|
| Repository open/close | [OK] Implemented | [OK] |
| STDIO mount/unmount | [OK] Implemented | [OK] |
| ZIP/UQM archive mounting | Registered but inactive | [OK] Fully active with archive reading |
| Mount ordering (TOP/BOTTOM/ABOVE/BELOW) | Partial (sorted by path length) | [OK, provisional where §15 still leaves edge cases open] Correct priority ordering under the documented contract |
| Directory handles | [OK] Implemented | [OK] |
| File descriptor I/O | [OK] Implemented | [OK] |
| Buffered stream I/O | [OK] Mostly implemented | [OK] |
| `uio_feof` | Hardcoded to 1 | [OK] Reflects stream status |
| `uio_ferror` | Hardcoded to 0 | [OK] Reflects stream status |
| Stream buffer management | Leaks on close | [OK] Properly freed |
| Directory listing for the current acceptance-bounded engine-required merged cases | Single directory only | [OK] Merges across mounts for the documented narrow current acceptance cases, including startup-visible `.rmp` discovery; broader generalized merged-directory behavior and the deferred combined STDIO+archive `.rmp` case remain compatibility-complete |
| `errno` setting on failure | Not set | [OK] Set appropriately |

### 17.2 Compatibility-complete supported-surface obligations

| Capability | Current State | Required end state |
|---|---|---|
| `uio_clearerr` | No-op | [OK] Clears status |
| `uio_ungetc` | Uses seek(-1) | [OK] Meets the one-character pushback contract |
| `uio_vfprintf` | Stub returning -1 | [OK] Provides an integration-compatible functional path for supported callers |
| Directory listing (general regex support) | Hard-coded patterns | [OK] General regex support |
| `uio_DirList_free` | Uses side-channel registry | [OK] Self-describing allocation |
| `uio_access` mode checking | Existence only | [OK] Full mode check |
| FileBlock API | Stubs | [OK] Functional where the supported surface requires it |
| StdioAccess API | Stubs | [OK] Supported surface with documented archive temp-copy behavior |
| GPDir/GPFile/PRoot internal API | Stubs | [OK] Functional where required by the chosen archive internals |

### 17.3 Port-completeness / cleanup items (not behavioral parity gates)

Remaining mixed-language export cleanup and symbol-ownership cleanup that does not change externally observable behavior belongs in this section rather than in Tier 1 or Tier 2 parity obligations.

| Capability | Current State | Desired cleanup state |
|---|---|---|
| `uio_fread` direct export | Requires C shim | Direct Rust export if/when cleanup effort removes the shim |
| Native Rust API wrappers | Not present | Ergonomic Rust-facing wrappers for Rust-internal consumers without changing the public C ABI |
| Logging/diagnostic polish | Minimal | Cleaner low-noise diagnostics once parity work is complete |
