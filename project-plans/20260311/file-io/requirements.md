# File I/O subsystem requirements

## Scope

These requirements define the intended end-state behavior of the file I/O subsystem exposed through the existing UIO-facing contracts. The requirements are language-agnostic except where ABI compatibility or integration behavior is externally observable.

## External compatibility constraints

- The subsystem SHALL preserve the externally visible UIO API contract exposed through the existing public headers and linked symbols, including repository, mount, directory, descriptor-style file, stream, directory-listing, and file-location entry points.
- The subsystem SHALL preserve the externally visible object semantics required by existing C consumers, including the public `uio_DirList` field layout and the public `uio_Stream` operational semantics (EOF/error status reporting, read/write/seek/tell behavior, and stream-handle access).
- Whether `uio_Stream` exact field layout must also be preserved is conditional on audit: if any C code under the Rust-UIO build configuration directly accesses `uio_Stream` fields (e.g., through `uio_INTERNAL` macros), exact layout is mandatory. If all access is through the function API, layout is at the implementation's discretion.
- The subsystem SHALL preserve integration compatibility with existing callers that rely on UIO from C or Rust through the current ABI boundary.
- The subsystem SHALL preserve externally visible filesystem identifiers and mount-placement flag meanings where those identifiers or flags are part of the public contract.

## Repository lifecycle

- When the process starts using the file I/O subsystem, the subsystem SHALL support explicit initialization before repository use.
- When the subsystem is initialized, the subsystem SHALL establish any global state required for subsequent repository, mount, file, and stream operations.
- While the subsystem is initialized, repeated initialization and shutdown sequences SHALL leave the subsystem in a valid state and SHALL NOT leak externally observable resources.
- When a repository is opened, the subsystem SHALL return a repository handle that is valid for subsequent mount, directory, file, stream, and listing operations associated with that repository.
- When a repository is closed, the subsystem SHALL release all mounts owned by that repository before the repository becomes invalid.
- When a repository is closed, the subsystem SHALL invalidate subsequent use of repository-owned mounts and handles according to the public lifetime contract.
- When the subsystem is uninitialized, the subsystem SHALL release remaining subsystem-owned resources and SHALL leave the process ready for a future clean initialization.

## Mount lifecycle and namespace behavior

- When a caller mounts a filesystem into a repository, the subsystem SHALL make that mount visible through the repository namespace at the requested mount point.
- When a caller mounts a directory using a source directory plus source path, the subsystem SHALL resolve the mounted root from that source directory and source path according to the public API contract.
- When a caller mounts a directory using an explicit input path, the subsystem SHALL use that input path as the mount source according to the selected filesystem type.
- When a caller requests a mount using a public filesystem type, the subsystem SHALL implement the externally visible behavior of that filesystem type for lookup, directory enumeration, and file access.
- When a caller mounts an archive-backed or package-backed filesystem supported by the public contract, the subsystem SHALL make that mount reachable through normal path resolution, file opens, and directory listing behavior.
- When multiple mounts overlap in namespace, the subsystem SHALL honor the public mount placement semantics for top, bottom, above, and below placement.
- When a caller supplies a relative mount handle for above or below placement, the subsystem SHALL place the new mount relative to the referenced mount as defined by the public contract.
- When a caller omits a required relative mount handle for above or below placement, the subsystem SHALL fail the mount operation.
- When a caller supplies a relative mount handle where one is not permitted by the public contract, the subsystem SHALL reject the mount operation or ignore the parameter only if that behavior is already part of the public contract.
- When a caller transplants a directory into the repository namespace, the subsystem SHALL expose that directory at the target mount point without changing the source directory contents.
- When a mount is unmounted, the subsystem SHALL remove that mount from subsequent namespace resolution.
- When all mounts in a repository are unmounted, the subsystem SHALL remove all repository-owned namespace contributions from subsequent resolution.
- While mounts remain active, path resolution SHALL be deterministic for the same repository state.

## Path semantics and normalization

- When a caller passes a path to any UIO operation, the subsystem SHALL interpret that path according to the directory handle, repository namespace, and mount table visible to that handle.
- When a caller passes an absolute virtual path, the subsystem SHALL resolve that path from the repository namespace root.
- When a caller passes a relative path, the subsystem SHALL resolve that path relative to the provided directory handle.
- When a caller opens a directory relative to an existing directory handle, the subsystem SHALL preserve the caller-visible semantics of relative navigation.
- When a path contains `.` components, the subsystem SHALL remove them as no-op segments during normalization.
- When a path contains `..` components, the subsystem SHALL resolve them logically by removing the preceding path component.
- When `..` resolution would traverse above the virtual namespace root `/`, the subsystem SHALL clamp the result at `/` rather than failing the operation.
- When a path contains repeated slashes, the subsystem SHALL collapse them to a single separator.
- When a path contains a trailing slash, the subsystem SHALL strip it during normalization.
- When an empty path string is passed, the subsystem SHALL treat it as referring to the directory handle's own location.
- When a path is resolved against a mounted subtree, `..` SHALL NOT escape above the mount's physical root on the host filesystem. The virtual path may traverse above the mount point in the virtual namespace, but the host I/O path SHALL remain confined to the mount's own physical directory tree. This sandboxing applies to logical path normalization only; it does not guarantee containment against host-filesystem symlinks that point outside the mount's directory tree.
- When a path contains nested components, the subsystem SHALL resolve each component consistently across directory opens, file opens, stats, access checks, rename, unlink, mkdir, rmdir, listing, and file-location queries.
- When a path refers to a mounted namespace location, the subsystem SHALL resolve the path through the applicable mount stack rather than bypassing the virtual namespace.
- When no mount matches a virtual path that is required to resolve through the repository namespace, the subsystem SHALL fail the operation rather than silently treating the virtual path as a host-native path.
- When a caller requests the host-visible location of a path, the subsystem SHALL return the effective owning mount and a host-native path only when the resolved object has a single concrete backing object with a direct stable host-native path usable by ordinary stdio/open calls.
- When a resolved object is not directly representable as a single host-native path (including synthetic archive directories, archive entries, and merged directories visible through multiple mounts), the subsystem SHALL fail the file-location query rather than fabricating or selecting a partial native path.
- When path strings are invalid for the public ABI, the subsystem SHALL fail the operation without undefined behavior.

## Directory handles and directory listing

- When a caller opens a directory, the subsystem SHALL return a handle bound to a virtual namespace location within the repository. Subsequent path-relative operations through that handle SHALL resolve against the current repository topology at the time of each operation, not against a snapshot taken at open time.
- When a path-relative operation is performed through a directory handle, that operation SHALL resolve against a topology state that is internally consistent for that individual operation. Successive operations through the same handle are not guaranteed to observe the same topology unless the caller synchronizes externally.
- While a directory handle remains valid, the subsystem SHALL preserve the association between that handle and its repository and virtual path.
- When a directory handle is closed, the subsystem SHALL free all resources owned exclusively by that handle.
- When a caller closes a directory handle, subsequent operations using that handle SHALL NOT be performed; the handle is invalidated. Double-close of the same handle is undefined behavior.
- When a caller requests a directory listing, the subsystem SHALL enumerate the entries visible at the requested path within the virtual namespace.
- When multiple mounts contribute entries to a listed directory, the subsystem SHALL merge visibility according to mount precedence rules.
- When multiple contributing mounts expose the same visible entry name, the subsystem SHALL apply precedence rules deterministically and SHALL NOT return duplicate visible names unless the public contract explicitly requires duplicates.
- When a caller supplies a match pattern and match type, the subsystem SHALL apply the requested matching semantics consistently for directory listing results.
- When the public API advertises regex matching, the subsystem SHALL implement full externally visible regex behavior required by callers and SHALL NOT substitute simplified heuristics.
- When no entries match a directory-list request for a successfully resolved directory, the subsystem SHALL return a non-null empty list with `numNames == 0`. `NULL` return SHALL be reserved for actual errors (unresolvable path, I/O failure, allocation failure).
- When a directory list is returned, the subsystem SHALL allocate and own the returned data in a form that can be freed through the public directory-list free operation.
- When a caller frees a returned directory list, the subsystem SHALL release all memory associated with that list without requiring hidden caller knowledge.

## Descriptor-style file operations

- When a caller opens a file through the descriptor-style API, the subsystem SHALL resolve the file path through the directory handle and virtual namespace before performing the open.
- When a caller opens a file with read-only, write-only, read-write, create, exclusive-create, truncate, append, or other public open flags, the subsystem SHALL honor the externally visible meaning of those flags.
- When a caller requests exclusive create and the target already exists, the subsystem SHALL fail the open.
- When a caller requests truncate on a writable open, the subsystem SHALL truncate the target file before subsequent writes become visible.
- When a caller reads from an open descriptor, the subsystem SHALL return the number of bytes actually read, including short reads at end of file.
- When a caller writes to an open descriptor, the subsystem SHALL return the number of bytes actually written or fail according to the public contract.
- When a caller seeks within an open descriptor, the subsystem SHALL support seek-from-start, seek-relative, and seek-from-end semantics defined by the ABI.
- When a caller closes a descriptor, the subsystem SHALL release the associated descriptor resources exactly once.
- When a caller requests file metadata using `uio_fstat` or `uio_stat`, the subsystem SHALL report metadata sufficient to preserve caller-visible file-versus-directory distinction, size, and permission or mode semantics required by existing integrations.
- When a caller requests an access check, the subsystem SHALL honor the requested access mode semantics rather than performing existence-only checks.
- When a caller renames an object, the subsystem SHALL move or rename the target within the semantics supported by the effective source and destination filesystems.
- When a caller requests a rename across different backing mounts, the subsystem SHALL fail with `EXDEV`.
- When a caller requests an operation across incompatible backing filesystems and the public contract does not support it directly, the subsystem SHALL fail with an appropriate error.
- When a caller creates a directory, the subsystem SHALL create exactly one directory at the requested path unless recursive creation is part of the public contract.
- When a caller removes a directory, the subsystem SHALL succeed only when removal semantics defined by the public contract are satisfied.
- When a caller unlinks a file, the subsystem SHALL remove the file entry from the resolved directory context.

## Mutation resolution in overlay mounts

- When a caller opens an existing file for write, the subsystem SHALL check the topmost mount exposing the file. If that mount is writable, the open SHALL proceed on that mount's backing store. If it is read-only, the operation SHALL fail with `EACCES` or `EROFS`. The subsystem SHALL NOT fall through to a lower writable mount.
- When a caller creates a new file and the name is absent from all visible layers, the subsystem SHALL create the file on the topmost writable mount covering the target directory. If no writable mount covers the path, the operation SHALL fail.
- When a caller creates a file and the name exists in a read-only upper layer, the subsystem SHALL fail the operation. The read-only upper layer shadows the path; the subsystem SHALL NOT bypass it to create in a lower writable layer.
- When a caller creates a file with `O_EXCL` and the name exists in any visible layer, the subsystem SHALL fail with `EEXIST`.
- When a caller creates a directory and the name already exists in any visible layer, the subsystem SHALL fail with `EEXIST`.
- When a caller creates a directory and the name does not exist, the subsystem SHALL create it on the topmost writable mount covering the parent path.
- When a caller unlinks or removes a directory entry that exists in a writable mount, the subsystem SHALL remove it from that mount's backing store.
- When a caller unlinks or removes a directory entry visible only in a read-only layer, the subsystem SHALL fail with `EACCES` or `EROFS`. The subsystem SHALL NOT create whiteout entries.

### Parent-path and visibility precedence for mutations

- When a mutation targets a path whose parent directory is visible only through a read-only mount and no writable mount exposes the same parent directory path, the subsystem SHALL fail the mutation with `EACCES` or `EROFS`. The subsystem SHALL NOT implicitly create parent directories on writable lower layers.
- When a path component in the parent chain is shadowed by a non-directory entry in an upper layer, the subsystem SHALL fail operations attempting to traverse through that component with `ENOTDIR`.
- When a caller renames an object, both source and destination SHALL be resolved through overlay precedence. If the topmost mount exposing the source is read-only, the rename SHALL fail with `EACCES` or `EROFS`. If the source and destination resolve to different backing mounts, the rename SHALL fail with `EXDEV`.

## Access check semantics

- When `F_OK` is requested, the subsystem SHALL return 0 if the path resolves to any existing entry in the visible namespace, regardless of backing type.
- When `R_OK` is requested, the subsystem SHALL return 0 for any existing readable entry (all mounted entries are at least readable).
- When `W_OK` is requested for a stdio-backed entry, the subsystem SHALL return 0 if the owning mount is writable, or -1 with `EACCES` if the mount is read-only.
- When `W_OK` is requested for an archive-backed entry, the subsystem SHALL return -1 with `EACCES`.
- When `X_OK` is requested for a directory (any backing type), the subsystem SHALL return 0 (directories are searchable/traversable).
- When `X_OK` is requested for a stdio-backed file, the subsystem SHALL delegate to the host `access()` result.
- When `X_OK` is requested for an archive-backed file, the subsystem SHALL return -1 with `EACCES`.
- When access is checked, the subsystem SHALL evaluate against the effective resolved visible object only (the topmost mount that exposes the name). The subsystem SHALL NOT scan lower layers.

## Stream operations

- When a caller opens a stream through `uio_fopen`, the subsystem SHALL interpret the mode string according to the public UIO stream contract.
- When a caller opens a stream for reading, writing, update, or append, the subsystem SHALL preserve the caller-visible semantics of the corresponding stdio-like mode.
- When a caller reads through `uio_fread`, the subsystem SHALL return the number of whole items successfully read.
- When a caller writes through `uio_fwrite`, the subsystem SHALL return the number of whole items successfully written.
- When a caller calls `uio_fgets`, the subsystem SHALL return a null-terminated line fragment or line, preserving newline handling consistent with the public contract.
- When a caller calls `uio_fgetc` or `uio_ungetc`, the subsystem SHALL preserve single-character stream semantics required by existing callers.
- When a caller seeks or tells through the stream API, the subsystem SHALL preserve the logical stream position consistently with prior buffered reads and writes.
- When a caller flushes a writable stream, the subsystem SHALL make buffered output visible according to the public contract.
- When a stream reaches end of file, the subsystem SHALL set and report end-of-file status through `uio_feof` according to actual stream state.
- When a stream operation fails, the subsystem SHALL set and report stream error status through `uio_ferror` according to actual stream state.
- When a caller clears stream status, the subsystem SHALL clear end-of-file and error indicators through `uio_clearerr` according to the public contract.
- While a stream remains open, the subsystem SHALL keep stream status and last-operation state consistent with the public `uio_Stream` semantics.
- When a caller requests the underlying handle for a stream, the subsystem SHALL return the descriptor-style handle associated with that stream when such a handle exists.
- When a caller closes a stream, the subsystem SHALL flush or discard buffered state according to mode semantics, release owned resources, and SHALL NOT leak stream-owned buffers.
- When formatted output APIs are part of the public ABI, the subsystem SHALL implement their externally visible behavior rather than returning a permanent stub error.

## File location and stdio access boundaries

- When a caller requests the host location of a stdio-backed file, `uio_getFileLocation` SHALL succeed and return the host-native path within the owning mount's directory tree and the owning mount handle.
- When a caller requests the host location of an archive-backed file, `uio_getFileLocation` SHALL fail (return -1). Archive entries do not have a direct stable host-native path.
- When a caller requests the host location of a file visible through layered mounts, `uio_getFileLocation` SHALL resolve against the winning (topmost) mount. If that mount is stdio-backed, the operation SHALL succeed with that mount's path.
- When a caller requests the host location of a directory visible through a single stdio-backed mount, `uio_getFileLocation` SHALL succeed and return the host-native path from that mount.
- When a caller requests the host location of a directory visible as a union of multiple mounts, `uio_getFileLocation` SHALL fail (return -1). A merged virtual directory does not have a single concrete host-native backing.
- When a caller requests the host location of a synthetic archive directory with no host-native backing, `uio_getFileLocation` SHALL fail (return -1, `errno = ENOENT`).
- When a caller requests the host location of a missing file, `uio_getFileLocation` SHALL fail with `errno = ENOENT`.
- When a caller requests stdio access to a stdio-backed file, `uio_getStdioAccess` SHALL return direct host access without creating a temporary copy.
- When a caller requests stdio access to an archive-backed file, `uio_getStdioAccess` SHALL create a temporary host-visible copy and return the temp path.
- When a caller requests stdio access to a missing file, `uio_getStdioAccess` SHALL fail and return `NULL`.
- When a caller requests stdio access to a directory, `uio_getStdioAccess` SHALL fail and return `NULL` with `errno = EISDIR`.

### Stdio access handle lifetime and release

- When `uio_getStdioAccess` returns a direct-path handle (stdio-backed file), `uio_releaseStdioAccess` SHALL invalidate the handle and free handle-owned bookkeeping. It SHALL NOT delete, modify, or otherwise affect the underlying host file.
- When `uio_getStdioAccess` returns a temp-copy handle (archive-backed file), `uio_releaseStdioAccess` SHALL invalidate the handle and perform best-effort cleanup of the temporary file and its owning temporary directory.
- When cleanup of a temporary file or directory created for stdio access fails, the subsystem SHALL log the failure but SHALL NOT propagate it as an error from the release call.
- While a stdio-access handle remains valid, the path returned by `uio_StdioAccessHandle_getPath` SHALL remain usable. After `uio_releaseStdioAccess`, the path pointer SHALL be treated as invalid.
- When a direct-path stdio-access handle is outstanding, the caller SHALL NOT assume the underlying host file remains part of the virtual namespace after repository or mount changes. The host file may still exist but is no longer guaranteed to be namespace-visible.

## Temp-file and temp-directory behavior

- When a caller requests stdio access to a resolved object and that object already resides on a host-usable stdio-backed filesystem, the subsystem SHALL return direct host access without creating a temporary copy.
- When a caller requests stdio access to a resolved object that is not directly usable through a host stdio path, the subsystem SHALL create a temporary host-visible copy in a writable temp area.
- When the subsystem creates a temporary directory for stdio access mediation, the subsystem SHALL create that directory with owner-restricted permissions unless platform rules require a different externally visible mode.
- When a caller provides a `tempDir` argument to `uio_getStdioAccess`, the subsystem SHALL create temporary files under that caller-provided directory.
- When the subsystem creates a temp directory name or temp file name, the subsystem SHALL avoid collisions and SHALL fail safely if a unique name cannot be established.
- When stdio access is released for a temporary copy, the subsystem SHALL delete the temporary file and its owning temporary directory if they were created for that access handle.
- When cleanup of a temporary file or directory fails, the subsystem SHALL report the failure through the established error or logging channels without corrupting unrelated state.
- While a temporary stdio-access handle remains valid, the subsystem SHALL retain any resources required to keep the temporary path usable by the caller.

### Process-level temp-directory mounting (deferred)

Whether the subsystem must mount a process-level temporary directory into the repository namespace is deferred pending audit (see specification §17, open question 3). The following requirement is conditional:

- **IF** audit determines that current callers depend on repository-visible temp mounts, **THEN** when process-level temporary directory support is initialized, the subsystem SHALL mount or otherwise expose that temporary area through the repository namespace. **OTHERWISE**, this behavior is not required and the subsystem need not mount temp directories into the namespace.

### Temp-root selection (non-normative guidance)

The selection of a fallback temp root when no caller-provided `tempDir` is available is an implementation convenience for `uio_getStdioAccess` internals. It is not part of the externally observable API contract (callers provide `tempDir` explicitly). Implementations should choose a writable location suitable for temporary files on the current platform, but the specific fallback strategy is not a normative requirement.

## Archive mount behavior

- When a caller mounts an archive (ZIP/UQM), the subsystem SHALL index the archive contents at mount time by reading the central directory.
- When archive indexing fails due to corruption or I/O error, the mount operation SHALL fail and return `NULL` with `errno = EIO`.
- When an archive contains file entries without explicit directory entries, the subsystem SHALL synthesize implicit directory entries for all implied path components.
- When an archive contains duplicate entries with the same normalized path, the subsystem SHALL use the last entry in the central directory.
- When archive entry paths contain backslash separators or leading slashes, the subsystem SHALL normalize them to forward-slash-separated relative paths before exposure in the virtual namespace.
- When archive entry lookups are performed, the subsystem SHALL use case-sensitive comparison.
- When archive-backed entries participate in cross-mount directory listings, the subsystem SHALL apply the same union/dedup rules as for any other mount type.
- When a caller opens an archive-backed file for read, the subsystem SHALL return decompressed content transparently.
- When a caller opens an archive-backed file for write, the subsystem SHALL fail with `EACCES`.
- When a caller calls `uio_stat` on an archive-backed file, the subsystem SHALL report the uncompressed size and appropriate file-type bits.
- When decompression or CRC validation fails during a read operation, the subsystem SHALL report the error as `EIO` on the failing read call, not retroactively on the mount.

## FileBlock and block-access behavior

- When the public ABI exposes FileBlock operations, the subsystem SHALL provide functional implementations for opening, closing, and accessing file blocks.
- When a caller accesses a file block, the subsystem SHALL return readable bytes for the requested range or fail according to the public contract.
- When the requested file-block range extends beyond the available file content, the subsystem SHALL return only the available bytes or fail according to the public contract, but SHALL NOT expose uninitialized memory.
- When the public ABI permits usage hints for file blocks, the subsystem SHALL accept those hints without breaking correctness.
- When file-block support is required by archive or package readers, the subsystem SHALL satisfy those integrations without relying on stub behavior.

## Error handling

- When any public operation fails, the subsystem SHALL return the failure indicator defined by the existing ABI for that operation.
- When any public operation fails due to an underlying filesystem or path-resolution error, the subsystem SHALL preserve an error code that allows existing callers to diagnose the failure through standard mechanisms.
- When an operation fails after partially allocating internal resources, the subsystem SHALL release resources not transferred to the caller before returning failure.
- When an operation cannot be completed because a caller passed a null pointer, invalid flag combination, unrecognized mode string, or other detectably invalid argument, the subsystem SHALL fail safely and SHALL NOT crash or exhibit undefined behavior.
- When the subsystem encounters an unsupported combination of public flags or mode values, the subsystem SHALL fail explicitly rather than silently degrading behavior.
- While handling errors, the subsystem SHALL preserve consistent repository, mount, directory, descriptor, stream, and temporary-resource state.

## Thread safety and lifecycle

- While the subsystem is in use, concurrent operations on independent handles SHALL be safe without external synchronization.
- When a caller shares a single handle across threads, the caller SHALL provide external synchronization. The subsystem SHALL NOT guarantee safety for concurrent use of the same handle.
- When a caller mutates repository topology (mount, unmount, transplant) concurrently with path-resolution operations, the caller SHALL provide external synchronization. The subsystem's internal locking SHALL protect data structure integrity but SHALL NOT guarantee consistent topology snapshots mid-mutation.
- When a path-relative operation is performed through a directory handle, that operation SHALL resolve against a topology state that is internally consistent for that individual operation. Successive operations through the same handle SHALL NOT be assumed to observe the same topology unless the caller synchronizes externally.
- When `uio_unInit()` is called, all repositories SHALL have been closed and all operations SHALL have completed. Behavior is undefined otherwise.
- When a repository is closed, all handles obtained from that repository SHALL have been closed first. Using a handle after its repository is closed is undefined behavior.

### Post-unmount handle validity

- When a mount is unmounted, open file handles and streams opened through that mount become invalid for I/O operations (read, write, seek). Callers SHALL close file handles and streams before unmounting when possible.
- When a mount is unmounted, `uio_close` and `uio_fclose` on handles and streams opened through that mount SHALL remain well-defined and safe to call. The subsystem SHALL NOT make cleanup itself undefined.
- When a mount is unmounted, directory handles pointing into the unmounted mount's namespace SHALL remain closeable via `uio_closeDir` but SHALL NOT be relied upon for correct path resolution in new operations.
- When the subsystem returns allocated data to the caller (`uio_DirList`, `outPath` strings, stdio-access paths), that data SHALL remain safe to consume from any thread until the documented free/release point.

### Lifetime violations

- When a caller uses a handle or pointer after its owning close, release, or destruction operation, behavior is undefined. The subsystem is not required to detect or gracefully handle use-after-free of opaque pointers.
- When a caller double-closes the same handle, behavior is undefined.
- When cleanup operations (`uio_close`, `uio_fclose`, `uio_closeDir`, `uio_releaseStdioAccess`) are called on handles invalidated by mount removal, those operations SHALL remain well-defined and safe. This is the defined exception to the general lifetime-violation rule and ensures cleanup is always possible after topology changes.

## Integration obligations

- The subsystem SHALL remain compatible with existing startup code that mounts configuration, content, addon, package, and temp locations through the UIO API.
- When startup policy mounts package archives or addon overlays, the subsystem SHALL make those mounts visible to subsequent directory, file, and stream operations.
- The subsystem SHALL remain compatible with existing SDL RWops adaptation that relies on `uio_fopen`, `uio_fread`, `uio_fwrite`, `uio_fseek`, `uio_ftell`, `uio_ferror`, and `uio_fclose`.
- When an SDL RWops consumer receives a zero-length read or write result from a UIO-backed stream, the subsystem SHALL expose correct EOF-versus-error state so that the adapter can distinguish normal completion from failure.
- The subsystem SHALL remain compatible with existing resource, sound, video, setup, and content-loading callers that use UIO through the current C ABI.
- The subsystem SHALL remain compatible with Rust-side consumers that currently call UIO through foreign-function interfaces rather than an internal native API.
- When mixed-language builds are produced, the subsystem SHALL provide one coherent linked implementation for each externally visible symbol and SHALL NOT require behaviorally significant shims except where the shim is intentionally part of the stable ABI.

## Resource management and non-functional correctness

- While the subsystem is in use, it SHALL manage ownership of repositories, mounts, directory handles, descriptors, streams, directory lists, file-location strings, temporary resources, and file blocks without leaks or double frees.
- When the subsystem returns caller-freed memory through the public ABI, it SHALL document and preserve the required free path through the existing public deallocation entry points.
- The subsystem SHALL avoid hidden side-channel allocation dependencies that are not safely reconstructible at free time through the object or documented public free path.
- The subsystem SHALL preserve deterministic behavior for path resolution and mount precedence independent of incidental allocation order except where allocation order is itself part of the public mount-placement semantics.
- When behavior differs by platform, the subsystem SHALL preserve the same logical contract while using platform-appropriate host filesystem mechanisms.
