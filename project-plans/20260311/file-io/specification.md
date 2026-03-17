# File I/O Subsystem — Functional and Technical Specification

## 1. Purpose and scope

This document specifies the desired end state of the UQM file I/O subsystem (UIO) as a Rust-owned virtual filesystem and stream layer. It defines what the subsystem must do, what contracts it must honor, and where its boundaries lie — without prescribing internal implementation strategies beyond what is externally observable.

**Relationship to `uio/` documentation set:** This document and `uio/specification.md` describe the same subsystem — the UIO virtual filesystem. This document (`file-io/`) is the sole normative owner of all behavior observable through any `uio_*` exported API call, including mount ordering, overlay resolution, directory enumeration, stream semantics, and archive-backed path behavior. The `uio/` document set is secondary/supporting documentation that may describe rationale, implementation constraints, port-completeness tracking, and unresolved audit areas, but may not independently define pass/fail behavior for any API-observable semantic. Where both documents discuss the same behavior and diverge, this document controls. `uio/` must defer to `file-io/` for all overlapping API-observable specifications.

The subsystem's mission is unchanged from the original C UIO design: present the rest of the engine with a single unified namespace that transparently overlays real directories, archive contents, and addon content, so that callers never need to know where a resource physically lives.

---

## 2. Responsibilities

The file I/O subsystem is responsible for:

1. **Virtual namespace management** — maintaining a layered mount tree that maps virtual paths to physical directories and archive contents.
2. **Descriptor-style file access** — open, close, read, write, seek, and stat on files resolved through the virtual namespace.
3. **Buffered stream access** — `fopen`/`fclose`/`fread`/`fwrite`/`fseek`/`ftell` and character-level I/O over the same virtual namespace, with correct EOF and error status tracking.
4. **Directory listing with pattern matching** — enumerating entries across all active mounts at a given virtual path, with support for literal, prefix, suffix, substring, and POSIX-extended-regex match types.
5. **File metadata and access checks** — `stat`, `fstat`, `access` against the virtual namespace.
6. **Filesystem mutation** — `mkdir`, `rmdir`, `rename`, `unlink` against writable mounts.
7. **Temporary stdio access** — providing callers with a guaranteed-stdio-backed path to a virtual file (copying to a temp location when the source is not on a stdio filesystem), and cleaning up on release.
8. **FileBlock random-access** — presenting an offset/length block-access interface over file handles, used internally by the ZIP filesystem handler to read archive metadata and decompress entries.
9. **Subsystem lifecycle** — initialization, shutdown, and cleanup of all internal state.

The subsystem is **not** responsible for:

- Deciding _what_ to mount or _where_ — that is startup/policy code (currently `options.c`).
- SDL integration — the `SDL_RWops` adapter (`sdluio.c`) sits above UIO and consumes its public API.
- Resource index loading — `LoadResourceIndex` sits above UIO and uses its directory-listing and file-read APIs.
- Audio/graphics decoding — Rust sound decoders and C graphics code are consumers, not part of UIO.

---

## 2A. Terminology

The following terms are used consistently throughout this document:

- **Host-native path**: A path string that can be passed directly to POSIX `open()`, `fopen()`, or `stat()` on the host operating system and will access the same content. Synonymous with "host-usable path" and "physical path" as used elsewhere in this document set.
- **Virtual path**: A `/`-separated path in the repository namespace, resolved through the mount registry. Virtual paths have no direct meaning to the host OS.
- **Backing object**: The concrete host-filesystem file or directory, or archive entry, that a virtual path resolves to after mount lookup.

---

## 3. Boundaries

### 3.1 Upward boundary — engine callers

The subsystem exposes a C-ABI-compatible function surface. Every exported symbol must be linkable from C object files without modification to existing C headers. The canonical API contract is defined by:

- `libs/uio/io.h` — repository, mount, directory, descriptor, metadata, listing, and FileBlock APIs
- `libs/uio/uiostream.h` — buffered stream APIs
- `libs/uio/utils.h` — `uio_copyFile`, `uio_getStdioAccess`, `uio_releaseStdioAccess`, `uio_StdioAccessHandle_getPath`
- `libs/uio/fileblock.h` — `uio_openFileBlock`, `uio_openFileBlock2`, `uio_accessFileBlock`, `uio_copyFileBlock`, `uio_closeFileBlock`, `uio_setFileBlockUsageHint`
- `libs/uio/fstypes.h` (public portion) — `uio_FileSystemID`, `uio_FSTYPE_STDIO`, `uio_FSTYPE_ZIP`
- `libs/uio/match.h` (public portion) — `match_MatchType` enum values
- `libs/uio/mount.h` (public portion) — `uio_MOUNT_RDONLY`, `uio_MountLocation` enum

All callers — C engine code, the SDL RWops adapter, and Rust subsystems — interact through this surface. The C-ABI surface is the authoritative contract.

### 3.2 Internal Rust-native API (non-normative)

Once the subsystem reaches end state, Rust-side callers should also have access to an internal Rust-native API (safe, typed) that avoids the FFI round-trip. This is a desirable migration goal but is **non-normative**: it must not alter public behavior, must not delay ABI parity items, and must not be treated as a blocking requirement for implementation or review. It is documented here only to acknowledge the intent; all behavioral requirements are defined in terms of the C-ABI surface.

### 3.3 Downward boundary — host OS and archives

The subsystem accesses the host filesystem through standard OS APIs (POSIX file I/O or equivalent). It reads archive contents (ZIP/UQM packages) through its own internal archive handler. It does not depend on any external archive library being linked by callers.

### 3.4 Lateral boundary — startup policy

Mount topology decisions — what directories to mount, what archives to scan, where addon content lives — remain the responsibility of the caller (currently `options.c` and `mountAddonDir` / `mountDirZips` / `loadIndices`). The subsystem provides the primitives; the caller provides the policy.

### 3.5 Build boundary (architectural constraint — not behavioral)

The following are project architecture goals that constrain the build structure but are not externally observable behavioral requirements. They are separated here to distinguish them from the runtime contract defined in the rest of this specification.

When the Rust UIO subsystem is complete:

- The `USE_RUST_UIO` build flag remains the selection mechanism.
- All `uio_*` symbols are exported directly from the Rust static library (`uqm_rust`). No C shim files (such as `uio_fread_shim.c`) are required for any exported symbol.
- The C-side UIO sub-build under `USE_RUST_UIO` is reduced to only those C files that provide non-UIO helpers still needed by other C code (e.g., `paths.c`, `uioutils.c`, `charhashtable.c`) — or eliminated entirely if those helpers are also ported.
- Legacy C implementation files (`io.c`, `uiostream.c`, `fileblock.c`, etc.) remain in-tree but are excluded from the Rust-UIO build, as they are today.

---

## 4. Public behavior — Repository and mount lifecycle

### 4.1 Repository

A **repository** is the top-level container for a set of mounts and the root of a virtual namespace.

| Operation | Behavior |
|-----------|----------|
| `uio_init()` | Initializes subsystem-global state. Must be called once before any other UIO operation. Idempotent if called more than once. |
| `uio_unInit()` | Tears down subsystem-global state. All repositories should be closed first; behavior is undefined if open repositories remain. |
| `uio_openRepository(flags)` | Allocates and returns a new repository. Returns `NULL` on allocation failure. |
| `uio_closeRepository(repository)` | Unmounts all directories in the repository, frees all associated state, and invalidates the pointer. |

A process may have multiple repositories, though UQM currently uses one.

### 4.2 Mounting

Mounts bind a physical location (a host directory or an archive within a host directory) into the virtual namespace at a specified mount point.

| Operation | Behavior |
|-----------|----------|
| `uio_mountDir(destRep, mountPoint, fsType, sourceDir, sourcePath, inPath, autoMount, flags, relative)` | Creates a new mount. Returns a `uio_MountHandle` on success, `NULL` on failure (with `errno` set). |
| `uio_transplantDir(mountPoint, sourceDir, flags, relative)` | Re-mounts an existing directory handle at a new virtual location within the same repository. |
| `uio_unmountDir(mountHandle)` | Removes a specific mount. Returns 0 on success, -1 on error. |
| `uio_unmountAllDirs(repository)` | Removes all mounts from a repository. Returns 0 on success. |
| `uio_getMountFileSystemType(mountHandle)` | Returns the `uio_FileSystemID` for a mount. |

**Mount placement semantics.** The `flags` parameter carries a `uio_MountLocation` value (`uio_MOUNT_BOTTOM`, `uio_MOUNT_TOP`, `uio_MOUNT_BELOW`, `uio_MOUNT_ABOVE`). Combined with the `relative` parameter (a handle to an existing mount), these control ordering:

- `BOTTOM` / `TOP`: place the new mount at the bottom or top of the repository's mount list. `relative` must be `NULL`.
- `BELOW` / `ABOVE`: place the new mount below or above the mount identified by `relative`. `relative` must not be `NULL`.

**Mount ordering affects resolution.** When multiple mounts cover the same virtual path, the mount closest to the top of the list wins for read operations. For write operations, the first writable mount wins (see §5.5 for detailed mutation resolution rules).

**Filesystem types.** The subsystem must support at minimum:
- `uio_FSTYPE_STDIO` (value 1): host-filesystem-backed directory.
- `uio_FSTYPE_ZIP` (value 2): ZIP/UQM archive. When `sourceDir` and `sourcePath` identify an archive file, the subsystem must open and index that archive and make its contents available for reading under the mount point.

**Read-only flag.** `uio_MOUNT_RDONLY` (bit 1) marks a mount as read-only. Write, rename, unlink, mkdir, and rmdir operations must fail with `errno = EACCES` against read-only mounts.

**AutoMount.** The `autoMount` parameter is a NULL-terminated array of `uio_AutoMount` rules. When a file matching a rule's pattern is encountered during directory enumeration, the subsystem should automatically mount it as the specified filesystem type. AutoMount is part of the legacy API surface. Whether it is required for current engine parity is an open question (see §17). If implemented:

- Enumeration that triggers auto-mount mutates repository state immediately.
- New auto-mounts are inserted at the bottom of the mount list for the containing repository.
- If an auto-mount attempt fails (e.g., corrupt archive), the failing entry is skipped and enumeration continues; the failure is logged but does not abort the listing.
- Repeated listings of the same directory may produce different results if new mounts were added by a prior listing.

### 4.3 Directory handles

| Operation | Behavior |
|-----------|----------|
| `uio_openDir(repository, path, flags)` | Opens a directory handle to the given virtual path within the repository. Returns `NULL` if the path cannot be resolved. |
| `uio_openDirRelative(base, path, flags)` | Opens a directory handle relative to an existing handle. Returns `NULL` on failure. |
| `uio_closeDir(dirHandle)` | Releases a directory handle. Returns 0. |

**Directory handle resolution model.** A `uio_DirHandle` represents a virtual namespace location — specifically, a `(repository, virtual path)` binding. It does not pin to a particular backing mount or backing directory chosen at open time. When the handle is subsequently used for child lookups, file opens, listings, or other path-relative operations, those operations resolve the child path against the **current** repository topology (mount ordering and active mounts) at the time of each operation. This means:

- If a higher-priority mount is added after `uio_openDir`, subsequent operations through the handle will see the new mount's content if it covers the handle's virtual path.
- If the mount that originally made the directory visible is removed, the handle remains closeable (see §11.4) but child lookups may fail or resolve differently depending on what other mounts still cover the path.
- For directories visible as a union of multiple mounts, the handle refers to the merged virtual directory, not to any single backing directory.

Each individual operation through a valid directory handle resolves against a topology state that is internally consistent for that operation. However, successive operations through the same handle are not guaranteed to observe the same topology unless the caller synchronizes externally. A multi-step sequence (e.g., list then open entries) may observe mount additions or removals that occur between steps.

This dynamic-resolution model is the intended semantic. It matches the behavior implied by the overlay listing rules (§7.1), the mutation resolution rules (§5.5), and the legacy C design where directory handles carry a virtual path that is resolved through the mount tree at each use.

**Directory handle ownership model.** Directory handles use simple ownership semantics: `uio_openDir` and `uio_openDirRelative` return a handle with exactly one owner (the caller). `uio_closeDir` releases that ownership and frees the handle's resources. Callers must not copy a raw `uio_DirHandle *` and pass it to `uio_closeDir` more than once — double-close is undefined behavior.

Internal subsystem components (such as mount internals or `uio_transplantDir`) may retain their own references to directory-related state, but this is not visible to callers. The caller's handle remains valid from open until close regardless of internal aliasing.

Whether the implementation uses reference counting internally is an implementation choice, not an externally observable contract. The public contract is: one open produces one handle; one close releases it.

---

## 5. Public behavior — Path and file operations

### 5.1 Virtual path resolution

All file and directory operations accept virtual paths relative to a `uio_DirHandle`. The subsystem resolves these through the mount registry:

1. Combine the directory handle's virtual path with the relative path argument.
2. Normalize the combined path (see §5.2).
3. Search the mount registry to find all mounts that cover the resulting virtual path.
4. For read operations: try mounts in order from top to bottom until one succeeds.
5. For write operations: apply the mutation resolution rules (see §5.5).

Paths use `/` as the separator. Leading `/` indicates an absolute virtual path; otherwise the path is relative to the directory handle.

### 5.2 Path normalization

Before mount lookup, the subsystem must normalize paths as follows:

1. **`.` components** are removed (no-op path segment).
2. **`..` components** are resolved logically by removing the preceding path component.
3. **Repeated slashes** (`//`) are collapsed to a single `/`.
4. **Trailing slashes** are stripped (trailing `/` is not meaningful for distinguishing files from directories; `uio_stat` or `uio_access` is used for that).
5. **Empty path strings** refer to the directory handle's own location (equivalent to `.`).

**Root clamping.** Virtual `..` resolution clamps at the repository root `/`. A path such as `/../../../foo` resolves to `/foo`. Over-traversal past root never fails; it silently stops at root.

**Mount-root sandboxing (logical path normalization only).** When a path is resolved against a mounted subtree, `..` must not escape above the mounted root into the host filesystem. The virtual path may traverse above the mount point in the virtual namespace (reaching content from sibling or parent mounts), but the underlying host path used for I/O is always confined to the mount's own physical root.

**Important limitation:** this sandboxing guarantee applies only to logical path normalization performed by the subsystem. It does not provide containment against host-filesystem symlinks. If a symlink within a mount's directory tree points outside that tree, following the symlink may expose host content not otherwise reachable through the virtual namespace. The subsystem follows host symlinks transparently (inherited from standard OS file I/O) and does not add symlink-aware sandboxing. Callers must not rely on mount-root sandboxing to prevent access to content reachable via host symlinks.

---

## 5.3 Descriptor-style file I/O

| Operation | Behavior |
|-----------|----------|
| `uio_open(dir, file, flags, mode)` | Opens a file. `flags` follow POSIX conventions (`O_RDONLY`, `O_WRONLY`, `O_RDWR`, `O_CREAT`, `O_EXCL`, `O_TRUNC`). `mode` is used with `O_CREAT`. Returns `NULL` on failure (with `errno` set). |
| `uio_close(handle)` | Closes a file handle. Returns 0 on success, -1 on error. |
| `uio_read(handle, buf, count)` | Reads up to `count` bytes. Returns bytes read (≥ 0), or -1 on error. Returns 0 at EOF. |
| `uio_write(handle, buf, count)` | Writes `count` bytes. Returns bytes written (≥ 0), or -1 on error. |
| `uio_lseek(handle, offset, whence)` | Repositions the file offset. `whence` is `SEEK_SET`, `SEEK_CUR`, or `SEEK_END`. Returns 0 on success, -1 on error. (Note: the C header returns `int`, not `off_t`; the subsystem must conform to that.) |

### 5.4 Metadata and access checks

| Operation | Behavior |
|-----------|----------|
| `uio_fstat(handle, statBuf)` | Fills `struct stat` fields for an open file. At minimum: `st_size`, `st_mode` (file-type bits `S_IFREG`/`S_IFDIR` and permission bits). Returns 0 on success, -1 on error. |
| `uio_stat(dir, path, statBuf)` | Same as `uio_fstat` but for a path relative to a directory handle. |
| `uio_access(dir, path, mode)` | Tests access. `mode` follows POSIX (`F_OK`, `R_OK`, `W_OK`, `X_OK`). Returns 0 if access is granted, -1 otherwise. |

For archive-backed files, `uio_stat` must report the uncompressed size and mark the entry as a regular file.

**`uio_access` semantics by mode and backing type:**

| Mode | stdio-backed file | stdio-backed dir | archive-backed file | archive-backed dir |
|------|-------------------|-------------------|---------------------|---------------------|
| `F_OK` | 0 if exists | 0 if exists | 0 if exists | 0 if exists |
| `R_OK` | 0 (readable) | 0 (readable) | 0 (readable) | 0 (readable) |
| `W_OK` | 0 if mount is writable | 0 if mount is writable | -1 (`EACCES`) | -1 (`EACCES`) |
| `X_OK` | host `access()` result | 0 (searchable/traversable) | -1 (`EACCES`) | 0 (searchable/traversable) |

Access checks operate against the effective resolved visible object only (the topmost mount that exposes the name). The subsystem does not scan lower writable layers when checking access on a name visible from a higher read-only layer.

### 5.5 Mutation resolution rules

All mutation operations (open-for-write, create, truncate, mkdir, rmdir, unlink, rename) must resolve their target through the overlay stack. The following rules define how mutations interact with layered mounts:

| Scenario | Behavior | errno on failure |
|----------|----------|-----------------|
| **Open existing file for write** | The topmost mount exposing the file is checked. If it is writable, the open proceeds on that mount's backing store. If it is read-only, the operation fails. The subsystem does not fall through to a lower writable mount. | `EACCES` or `EROFS` |
| **Create new file (`O_CREAT`) when name is absent everywhere** | The topmost writable mount covering the target directory receives the new file. If no writable mount covers the path, the operation fails. | `EACCES` or `EROFS` |
| **Create (`O_CREAT`) when name exists in read-only upper layer** | The operation fails. The read-only upper layer shadows the path; the subsystem does not bypass it to create in a lower writable layer. | `EACCES` or `EROFS` |
| **Create (`O_CREAT \| O_EXCL`) when name exists in any visible layer** | Fails with `EEXIST` regardless of which layer holds the name. | `EEXIST` |
| **Truncate (`O_TRUNC`) on writable mount** | Proceeds on the mount where the file is opened. If the file is visible only through a read-only mount, truncate fails. | `EACCES` or `EROFS` |
| **`uio_mkdir` where name already exists in any visible layer** | Fails with `EEXIST`. | `EEXIST` |
| **`uio_mkdir` where name does not exist** | Creates the directory on the topmost writable mount covering the parent path. | `EACCES` if no writable mount |
| **`uio_unlink` / `uio_rmdir` of entry in writable mount** | Proceeds. Removes the entry from the writable mount's backing store. If a lower layer also has an entry with the same name, that entry may become visible after removal. | — |
| **`uio_unlink` / `uio_rmdir` of entry visible only in read-only layer** | Fails. The subsystem does not create whiteout entries. | `EACCES` or `EROFS` |
| **`uio_rename` within same backing mount** | Proceeds as a host-level rename. | — |
| **`uio_rename` across different backing mounts** | Fails. Cross-mount rename is not supported. | `EXDEV` |

**Parent-path resolution for mutations.** When a mutation targets a path whose parent directory is visible through the overlay, the parent must be resolved to determine where the mutation occurs:

- The parent directory is resolved through normal overlay precedence (topmost mount exposing the parent wins).
- If the visible parent directory exists only on a read-only mount and no writable mount exposes the same parent directory path, file creation and `mkdir` in that parent fail with `EACCES` or `EROFS`. The subsystem does not implicitly create parent directories on writable lower layers.
- If a path component in the parent chain is shadowed by a non-directory entry in an upper layer (e.g., a file named `foo` shadows a directory `foo/` from a lower layer), operations attempting to traverse through that component fail with `ENOTDIR`. The upper-layer non-directory entry takes precedence.

**Rename source and destination resolution.** Both source and destination of `uio_rename` are resolved through overlay precedence:

- The source object is identified by the topmost mount exposing it. If that mount is read-only, the rename fails with `EACCES` or `EROFS`.
- The destination path is resolved to determine the backing mount. If the source and destination resolve to different backing mounts, the rename fails with `EXDEV`.
- If the destination name is visible from a different (higher or lower) mount than the source's mount, the rename fails with `EXDEV` because the operation cannot atomically affect two mounts.

### 5.6 File location query

| Operation | Behavior |
|-----------|----------|
| `uio_getFileLocation(dir, inPath, flags, mountHandle, outPath)` | Resolves a virtual path to the owning mount and the host-native path within that mount. On success, `*mountHandle` receives the mount handle and `*outPath` receives a freshly allocated (`malloc`'d) C string with the resolved path. Caller is responsible for freeing `*outPath`. Returns 0 on success, -1 on failure. |

**Resolution model.** `uio_getFileLocation` resolves the target as a single concrete backing object on a single mount, using normal overlay precedence (topmost mount exposing the name wins). It does not return union or merged results. The operation succeeds only when the resolved object has a single concrete host-native backing — that is, the winning mount is stdio-backed and the object maps to exactly one host-filesystem entry. It fails for any object that lacks a singular concrete host-native backing, including archive entries, synthetic archive directories, and directories that are visible as a union of multiple mounts.

**`uio_getFileLocation` vs `uio_getStdioAccess` boundaries:**

| Scenario | `uio_getFileLocation` | `uio_getStdioAccess` |
|----------|----------------------|---------------------|
| **stdio-backed file** | Succeeds. `*outPath` is the host-native path within the owning mount's directory tree. `*mountHandle` identifies the mount. | Succeeds. Returns direct host path (no copy). |
| **ZIP/archive-backed file** | Fails (returns -1). Archive entries do not have a direct stable host-native path usable by ordinary `open()`/`fopen()` calls. | Succeeds. Uses internal mount-type inspection to determine the backing type, then copies the entry to a temp file and returns the temp path. `uio_getStdioAccess` does not depend on public `uio_getFileLocation` succeeding for archive-backed content; it performs its own internal resolution to identify the owning mount and backing type. |
| **File in layered mounts** | Succeeds for the winning (topmost) mount that exposes the file, if that mount is stdio-backed. `*outPath` reflects the winning layer's real path. | Succeeds using the winning layer's resolution. |
| **Directory on a single stdio-backed mount** | Succeeds. Returns the host-native path of that mount's backing directory. | Not applicable (stdio access is for files; see §8). |
| **Directory visible as a union of multiple mounts** | Fails (returns -1). A merged virtual directory is not a single concrete backing object; returning one layer's path would misrepresent the visible directory. | Not applicable. |
| **Synthetic archive directory (no host-native backing)** | Fails (returns -1, `errno = ENOENT`). Synthesized archive directories have no host-native path. | Not applicable. |
| **Missing file** | Fails (returns -1, `errno = ENOENT`). | Fails (returns `NULL`). |

**errno for `uio_getFileLocation` failures:**

| Condition | errno |
|-----------|-------|
| File not found | `ENOENT` |
| Resolved object is archive-backed (no host-native path) | `ENOENT` |
| Resolved object is a synthetic directory with no host-native backing | `ENOENT` |
| Resolved object is a merged directory visible through multiple mounts | `ENOENT` |
| Invalid arguments | `EINVAL` |

---

## 6. Public behavior — Buffered stream I/O

### 6.1 Stream lifecycle

| Operation | Behavior |
|-----------|----------|
| `uio_fopen(dir, path, mode)` | Opens a buffered stream. `mode` follows C `fopen` conventions (`"r"`, `"rb"`, `"w"`, `"wb"`, `"a"`, `"r+"`, etc.). Returns `NULL` on failure. |
| `uio_fclose(stream)` | Flushes and closes the stream. Frees all associated memory including the internal buffer. Returns 0 on success, `EOF` on error. |

### 6.2 Stream read operations

| Operation | Behavior |
|-----------|----------|
| `uio_fread(buf, size, nmemb, stream)` | Reads up to `nmemb` items of `size` bytes each. Returns the number of items fully read. Sets the stream's EOF flag if end-of-file is reached, or the error flag on I/O error. |
| `uio_fgets(buf, size, stream)` | Reads at most `size - 1` characters, stopping at newline or EOF. Null-terminates the result. Returns `buf` on success, `NULL` on EOF with no data read or error. |
| `uio_fgetc(stream)` / `uio_getc(stream)` | Reads one byte. Returns the byte as `unsigned char` cast to `int`, or `EOF` (-1) on end-of-file or error. |
| `uio_ungetc(c, stream)` | Pushes back one character. At least one character of pushback must be supported. Returns `c` on success, `EOF` on failure. |

### 6.3 Stream write operations

| Operation | Behavior |
|-----------|----------|
| `uio_fwrite(buf, size, nmemb, stream)` | Writes `nmemb` items of `size` bytes each. Returns number of items written. Sets the error flag on failure. |
| `uio_fprintf(stream, format, ...)` | Formatted output. Returns number of characters written, or a negative value on error. |
| `uio_vfprintf(stream, format, args)` | `va_list` variant of `uio_fprintf`. Must produce correct formatted output — the end state must not stub this. (Used by netplay debug logging.) |
| `uio_fputc(c, stream)` / `uio_putc(c, stream)` | Writes one byte. Returns the byte written, or `EOF` on error. |
| `uio_fputs(s, stream)` | Writes a null-terminated string (without the terminator). Returns a non-negative value on success, `EOF` on error. |

### 6.4 Stream positioning

| Operation | Behavior |
|-----------|----------|
| `uio_fseek(stream, offset, whence)` | Repositions the stream. Clears the EOF flag. Returns 0 on success, -1 on error. |
| `uio_ftell(stream)` | Returns the current position, or -1 on error. |

### 6.5 Stream status

| Operation | Behavior |
|-----------|----------|
| `uio_feof(stream)` | Returns non-zero if the EOF flag is set, 0 otherwise. The EOF flag is set when a read operation encounters end-of-file. It is cleared by `uio_fseek`, `uio_clearerr`, or `uio_rewind` (if implemented). |
| `uio_ferror(stream)` | Returns non-zero if the error flag is set, 0 otherwise. The error flag is set when an I/O error occurs during a read or write operation. |
| `uio_clearerr(stream)` | Clears both the EOF and error flags. |
| `uio_fflush(stream)` | Flushes the write buffer to the underlying file. For read-mode streams, behavior is implementation-defined. Passing `NULL` flushes all open streams (or is a no-op). Returns 0 on success, `EOF` on error. |

**Critical correctness requirement:** `uio_feof` and `uio_ferror` must return accurate status. The SDL RWops adapter (`sdluio.c`) relies on `uio_ferror` returning 0 after a short `uio_fread` to distinguish EOF from error. Hardcoded return values are not acceptable.

### 6.6 Stream handle access

| Operation | Behavior |
|-----------|----------|
| `uio_streamHandle(stream)` | Returns the underlying `uio_Handle` for the stream. The handle remains owned by the stream; the caller must not close it independently. |

---

## 7. Public behavior — Directory listing

| Operation | Behavior |
|-----------|----------|
| `uio_getDirList(dirHandle, path, pattern, matchType)` | Enumerates directory entries at `dirHandle/path` matching `pattern` according to `matchType`. Returns a heap-allocated `uio_DirList`, or `NULL` on error. For a successfully resolved directory with no matches, returns a non-null `uio_DirList` with `numNames == 0` (empty match is not an error). `NULL` is reserved for actual errors (unresolvable path, I/O failure, allocation failure). |
| `uio_DirList_free(dirList)` | Frees all memory associated with a `uio_DirList`. Safe to call with `NULL`. |

### 7.1 Cross-mount listing

When multiple mounts cover the same virtual directory, `uio_getDirList` must return the **union** of entries from all active mounts at that path, respecting mount ordering for deduplication (a name that appears in a higher-priority mount shadows the same name in a lower-priority mount for purposes of the listing — but since directory listings are flat name lists, in practice this means the same name appears only once).

### 7.2 Match types

The `matchType` parameter corresponds to the `match_MatchType` enum:

| Value | Name | Semantics |
|-------|------|-----------|
| 0 | `match_MATCH_LITERAL` | Exact string equality. |
| 1 | `match_MATCH_PREFIX` | Entry name starts with `pattern`. |
| 2 | `match_MATCH_SUFFIX` | Entry name ends with `pattern`. |
| 3 | `match_MATCH_SUBSTRING` | Entry name contains `pattern`. |
| 4 | `match_MATCH_REGEX` | POSIX extended regular expression. |

An empty pattern matches all entries.

Regex matching must support the patterns actually used by the engine:
- `\.[rR][mM][pP]$` — resource index files
- `\.([zZ][iI][pP]|[uU][qQ][mM])$` — archive packages

The subsystem must support general POSIX extended regex, not just hardcoded special cases.

### 7.3 `uio_DirList` layout

The externally visible struct is:

```c
struct uio_DirList {
    const char **names;  // array of pointers to null-terminated strings
    int numNames;        // number of entries
    // internal fields follow (not visible to callers)
};
```

The `names` array and the strings it points to must remain valid until `uio_DirList_free` is called. The subsystem owns all memory; the caller must not modify or free individual entries.

---

## 8. Public behavior — Temporary stdio access

| Operation | Behavior |
|-----------|----------|
| `uio_getStdioAccess(dir, path, flags, tempDir)` | Returns a `uio_StdioAccessHandle` that provides a guaranteed stdio-filesystem path to the specified virtual file. If the file already resides on a stdio filesystem, the returned path points directly to it and no copy is made. If the file is archive-backed (or otherwise not stdio-accessible), the subsystem copies it to a temporary directory and returns a path to the copy. |
| `uio_StdioAccessHandle_getPath(handle)` | Returns a pointer to the stdio path string. Valid for the lifetime of the handle. |
| `uio_releaseStdioAccess(handle)` | Releases the handle. If a temporary copy was made, it is deleted and the temporary directory is removed. |

**Input constraints.** `uio_getStdioAccess` is defined for files only. If the resolved virtual path refers to a directory, the operation fails and returns `NULL` with `errno = EISDIR`.

**Handle types and release semantics.** A stdio-access handle is one of two kinds, determined at creation time:

- **Direct-path handle** (stdio-backed file): The handle records the host-native path of the existing file. No copy is made. `uio_releaseStdioAccess` invalidates the handle and frees handle-owned bookkeeping storage, but does **not** delete, modify, or otherwise affect the underlying host file. The caller must not assume the underlying host file remains part of the virtual namespace after repository or mount changes — the path may still be valid on the host filesystem but its relationship to the virtual namespace is not maintained.
- **Temp-copy handle** (archive-backed or otherwise non-stdio file): The handle records the path of a temporary copy. `uio_releaseStdioAccess` invalidates the handle, deletes the temporary file, and removes the temporary directory (best-effort cleanup). If cleanup fails, a warning is logged but the release still succeeds.

In both cases, the path string returned by `uio_StdioAccessHandle_getPath` is valid only until `uio_releaseStdioAccess` is called. After release, the handle and the path pointer are both invalid.

**Temp-file behavior:**
- Temporary directories are created under the `tempDir` provided by the caller.
- Directory names are generated to avoid collisions (e.g., hex-encoded counter).
- Creation retries up to a reasonable limit if the name collides.
- On release, the temporary file is unlinked, then the temporary directory is removed.
- If cleanup fails, a warning is logged but the release still succeeds (resource leak rather than error propagation).

**Process-level temp-directory mounting (deferred).** Whether the subsystem must mount a process-level temporary directory into the repository namespace (as distinct from the per-handle temp behavior above) is an unresolved question. The legacy C implementation included such behavior, but whether current callers depend on repository-visible temp mounts has not been confirmed. This is deferred pending audit (see §17). Requirements and implementation must not assume this behavior is needed until the audit resolves it.

---

## 9. Public behavior — FileBlock random access

FileBlock provides an optimized random-access interface over a file handle. It is the mechanism the ZIP filesystem handler uses to read archive central directories, local headers, and compressed entry data.

| Operation | Behavior |
|-----------|----------|
| `uio_openFileBlock(handle)` | Creates a FileBlock covering the entire file. Returns `NULL` on failure. |
| `uio_openFileBlock2(handle, offset, size)` | Creates a FileBlock covering a specific region of the file. Returns `NULL` on failure. |
| `uio_accessFileBlock(block, offset, length, buffer)` | Makes `length` bytes at `offset` available. On success, `*buffer` points to the data and the return value is the number of bytes available (may be less than `length` at EOF). The buffer remains valid until the next `uio_accessFileBlock` call on the same block or until `uio_closeFileBlock`. Returns -1 on error. |
| `uio_copyFileBlock(block, offset, buffer, length)` | Copies `length` bytes at `offset` into the caller-provided `buffer`. Returns 0 on success, -1 on error. |
| `uio_closeFileBlock(block)` | Closes the FileBlock and frees associated resources. Returns 0 on success. |
| `uio_setFileBlockUsageHint(block, usage, flags)` | Provides a hint about access pattern (e.g., sequential forward). The subsystem may use this to optimize readahead or caching. No-op is acceptable. |

The `uio_accessFileBlock` return-a-pointer-to-internal-buffer pattern is the critical contract: the ZIP handler reads headers by getting a pointer into the FileBlock's buffer and parsing in place. The buffer must remain stable between calls.

---

## 10. Public behavior — ZIP/UQM archive integration

### 10.1 Archive entry path form

Archive entry paths stored in ZIP central directory records are normalized to the virtual namespace form before exposure:
- Path separators within the archive (`\` or `/`) are normalized to `/`.
- Leading `/` is stripped (archive entries are relative to the mount point).
- Trailing `/` on directory entries is stripped during normalization.

### 10.2 Case sensitivity

Archive entry lookups are **case-sensitive**. This matches the legacy C implementation behavior. The virtual namespace does not perform case-folding on any path component, whether backed by stdio or archive mounts.

### 10.3 Implicit directories

ZIP archives may contain file entries without corresponding explicit directory entries. The subsystem must synthesize implicit directory entries for any path component implied by a file entry. For example, if an archive contains `comm/commander/text.txt` but no explicit `comm/` or `comm/commander/` directory entries, the subsystem must treat those directories as existing for purposes of `uio_stat`, `uio_access`, `uio_openDir`, and `uio_getDirList`.

### 10.4 Duplicate entries within an archive

If a ZIP archive contains multiple entries with the same normalized path, the **last entry in the central directory** wins. This matches common ZIP tool behavior.

### 10.5 Archive indexing

Archive contents are indexed at mount time (eager indexing). The central directory is read and parsed when `uio_mountDir` is called with `uio_FSTYPE_ZIP`. If the archive is corrupt or unreadable, the mount operation fails and returns `NULL` with `errno = EIO`.

Errors encountered while reading individual entries during subsequent file opens (e.g., decompression failure, CRC mismatch) are reported as `EIO` on the failing `uio_read` or `uio_fread` call, not at mount time.

### 10.6 Archive entries in virtual namespace operations

| Operation | Archive-backed behavior |
|-----------|------------------------|
| `uio_stat` | Reports `S_IFREG` for file entries, `S_IFDIR` for directory entries (including synthetic). `st_size` is the uncompressed size. |
| `uio_access` | `F_OK` and `R_OK` succeed. `W_OK` and `X_OK` fail with `EACCES`. |
| `uio_getDirList` | Archive entries participate in cross-mount union/dedup. Entry names from archives are compared and deduplicated against stdio-backed entries using the same precedence rules as any other mount. |
| `uio_getFileLocation` | Fails (returns -1). Archive entries have no direct host-native path. |
| `uio_getStdioAccess` | Succeeds by copying the decompressed content to a temp file. |
| `uio_open` (read) | Succeeds. Returns a handle that reads decompressed content. |
| `uio_open` (write) | Fails with `EACCES`. Archive mounts are read-only. |

---

## 11. Ownership and lifecycle

### 11.1 Object ownership rules

| Object | Allocated by | Freed by | Notes |
|--------|-------------|----------|-------|
| `uio_Repository` | `uio_openRepository` | `uio_closeRepository` | Closing frees all child mounts. |
| `uio_MountHandle` | `uio_mountDir` / `uio_transplantDir` | `uio_unmountDir` / `uio_closeRepository` | Handle is invalidated after unmount. |
| `uio_DirHandle` | `uio_openDir` / `uio_openDirRelative` | `uio_closeDir` | Single-owner. |
| `uio_Handle` (file) | `uio_open` | `uio_close` | Caller must close. |
| `uio_Stream` | `uio_fopen` | `uio_fclose` | Owns its buffer and underlying handle. `uio_fclose` frees everything. |
| `uio_DirList` | `uio_getDirList` | `uio_DirList_free` | Subsystem owns all internal memory. |
| `uio_FileBlock` | `uio_openFileBlock[2]` | `uio_closeFileBlock` | Owns internal read buffer. |
| `uio_StdioAccessHandle` | `uio_getStdioAccess` | `uio_releaseStdioAccess` | May own a temp-file copy. |
| `outPath` from `uio_getFileLocation` | `uio_getFileLocation` | Caller (via `free`) | Caller-owned after return. |

### 11.2 No memory leaks

Every allocation made by the subsystem must have a corresponding deallocation path reachable through the public API. Specifically:

- `uio_fclose` must free the stream struct, the internal buffer (if allocated), and close the underlying file handle.
- `uio_DirList_free` must free the name pointer array, the string buffer, and the list struct itself, without requiring side-channel registries.
- No global pointer-to-size registries should be required at end state for normal deallocation paths.

**Architectural guidance (non-normative).** A self-contained allocation strategy (e.g., storing size metadata in fields not visible to callers, or using a single contiguous allocation whose layout the subsystem controls) is the recommended approach for `uio_DirList`. This is design guidance, not an externally observable behavioral requirement; any strategy that avoids leaks and does not require hidden caller knowledge is acceptable.

### 11.3 Thread safety and concurrency

The subsystem's global state (mount registry, filesystem handler registry) must be safe to access from multiple threads. Individual file handles and streams are not required to be safe for concurrent use from multiple threads on the same handle — that matches POSIX `FILE*` semantics. The caller is responsible for external synchronization if sharing a handle across threads.

**Detailed concurrency guarantees:**

| Operation category | Guarantee |
|--------------------|-----------|
| **Concurrent reads/opens on independent handles** | Safe. Multiple threads may independently open, read, seek, and close different files/streams without external synchronization. |
| **Concurrent use of same handle from multiple threads** | Not safe. Callers must synchronize externally, same as POSIX `FILE*`. |
| **Concurrent mount/unmount vs. path resolution** | The subsystem's internal locking must protect data structure integrity (no crashes, no corruption). However, the subsystem does not guarantee a consistent topology snapshot if a mount or unmount is racing with a path resolution. Callers that need topology stability during a sequence of operations must synchronize externally. |
| **Operations through a directory handle during topology changes** | Each individual operation resolves against a topology state that is internally consistent for that operation. Successive operations through the same directory handle are not guaranteed to observe the same topology. A multi-step sequence is not a snapshot. |
| **`uio_unInit()` racing with active operations** | Undefined behavior. Callers must ensure all repositories are closed and all operations have completed before calling `uio_unInit()`. |
| **Repository shutdown while handles are open** | Handles obtained from a repository become invalid after `uio_closeRepository`. Using them after close is undefined behavior. Callers must close all handles before closing the repository. |
| **Returned `uio_DirList`, `outPath`, stdio-access paths** | Safe to consume from any thread until their documented free/release point. The subsystem does not mutate returned data after returning it. |

In summary: operations are safe for independent handles; repository mutation and shutdown require external synchronization by the caller.

### 11.4 Post-unmount handle validity

When a mount is unmounted:

- **Open file handles** (`uio_Handle`) and **open streams** (`uio_Stream`) that were opened through the unmounted mount become invalid for I/O. Subsequent read, write, and seek operations on them produce undefined behavior. However, **`uio_close` and `uio_fclose` remain well-defined**: callers may (and should) close invalidated handles and streams without undefined behavior. This ensures cleanup is always safe even when a mount is removed unexpectedly. Callers should close file handles and streams before unmounting when possible.
- **Directory handles** (`uio_DirHandle`) that point into an unmounted mount's namespace remain closeable via `uio_closeDir`. They may no longer resolve paths correctly for new operations (child lookups, listings, file opens through the handle may fail or produce different results). The directory handle does not keep the mount alive.
- **`uio_getFileLocation` / `outPath`** strings already returned to callers remain valid (they are caller-owned copies).
- **`uio_StdioAccessHandle`** handles for temp copies remain valid (the temp file is independent of the mount). Handles that returned a direct path into the unmounted mount's directory tree may reference a path that is still valid on the host filesystem but is no longer part of the virtual namespace.

---

## 12. Error handling

### 12.1 Error reporting convention

All functions follow C/POSIX conventions:
- Pointer-returning functions return `NULL` on failure.
- Integer-returning functions return -1 (or `EOF` for stream functions) on failure.
- `errno` is set to an appropriate POSIX error code on failure.

### 12.2 Required errno values

The subsystem must set `errno` correctly for at least:

| Code | Condition |
|------|-----------|
| `ENOENT` | File or directory not found in any active mount. |
| `EEXIST` | File or directory already exists (for `O_EXCL`, `mkdir`). |
| `EACCES` / `EROFS` | Write operation against a read-only mount or archive. |
| `ENOTDIR` | A path component is not a directory. |
| `EISDIR` | Attempted file operation on a directory. |
| `ENOTEMPTY` | `rmdir` on a non-empty directory. |
| `EINVAL` | Invalid argument (bad flags, invalid seek whence, etc.). |
| `ENOMEM` | Allocation failure. |
| `EIO` | Underlying I/O error or archive corruption. |
| `EXDEV` | Rename across different backing mounts. |

### 12.3 Overlay-specific errno guidance

| Scenario | Recommended errno |
|----------|-------------------|
| Write denied because topmost visible layer is read-only | `EACCES` or `EROFS` |
| Rename across different backing mounts | `EXDEV` |
| `uio_getFileLocation` on archive-backed file | `ENOENT` |
| `uio_getFileLocation` on synthetic archive directory | `ENOENT` |
| `uio_getFileLocation` on merged multi-mount directory | `ENOENT` |
| Opening a directory as a file | `EISDIR` |
| `uio_getStdioAccess` on a directory | `EISDIR` |
| Attempting to remove a name that exists only in a read-only layer | `EACCES` or `EROFS` |
| Parent path component shadowed by non-directory in upper layer | `ENOTDIR` |

### 12.4 Safe-failure scope and lifetime violations

The subsystem must fail safely — without crashing or exhibiting undefined behavior — for **detectably invalid arguments**: `NULL` pointers where a valid pointer is required, invalid flag combinations, unrecognized mode strings, and similar programmatically validatable inputs.

**Lifetime violations** are a separate category. Using a handle after its owning close operation (`uio_close`, `uio_fclose`, `uio_closeDir`, `uio_closeRepository`, `uio_releaseStdioAccess`), after repository destruction, or after `uio_unInit()` constitutes use of a previously freed or invalidated opaque pointer. Such use remains **undefined behavior** — the subsystem is not required to detect or gracefully handle it. This matches standard C ABI semantics for opaque-pointer APIs.

The defined exception is **cleanup operations themselves**: `uio_close`, `uio_fclose`, `uio_closeDir`, and `uio_releaseStdioAccess` on handles invalidated by mount removal remain well-defined and safe (see §11.4). This preserves the ability to clean up resources even after topology changes. Double-close of the same handle, however, remains undefined behavior.

### 12.5 Panic safety

No Rust panic may propagate across the FFI boundary. All `extern "C"` entry points must catch panics and convert them to appropriate error returns.

---

## 13. Integration points

### 13.1 C engine startup (options.c)

The startup code is the primary policy driver. It calls:

- `uio_mountDir` with `uio_FSTYPE_STDIO` to mount config and content directories.
- `uio_mountDir` with `uio_FSTYPE_ZIP` to mount discovered archive packages.
- `uio_openDir` / `uio_openDirRelative` to obtain `configDir`, `saveDir`, `meleeDir`, `contentDir`.
- `uio_getDirList` with regex patterns to discover `.zip`/`.uqm` and `.rmp` files.
- `uio_stat` to distinguish files from directories in addon enumeration.
- `uio_transplantDir` to re-mount directories at alternative locations.
- `uio_DirList_free` to release listing results.

The subsystem must support all of these call patterns as-is, without requiring changes to `options.c` beyond what is needed for the Rust UIO build flag.

### 13.2 C globals: contentDir, configDir, saveDir, meleeDir

These are `uio_DirHandle*` globals owned by `options.c`. They are passed into various engine subsystems and into Rust FFI consumers (e.g., `heart_ffi.rs` imports `contentDir`). The subsystem must produce `uio_DirHandle` pointers that remain valid and functional for the full engine lifetime after startup completes.

### 13.3 SDL RWops adapter (sdluio.c)

`sdluio.c` wraps a `uio_Stream*` as an `SDL_RWops` by storing the stream pointer in `context->hidden.unknown.data1` and forwarding `seek`, `read`, `write`, and `close` calls. Critically:

- `sdluio_read` calls `uio_fread`, then checks `uio_ferror` on a zero return to distinguish EOF from error.
- `sdluio_write` does the same with `uio_fwrite` and `uio_ferror`.
- `sdluio_close` calls `uio_fclose`.

The subsystem must ensure `uio_ferror` returns 0 after a normal short read (EOF) and non-zero after a genuine I/O error.

### 13.4 Rust sound subsystem

Rust sound decoders (`aiff_ffi.rs`, `mod_ffi.rs`, `wav_ffi.rs`, `dukaud_ffi.rs`, `ffi.rs` for OGG) currently consume UIO via `extern "C"` FFI declarations. In end state, these may be migrated to use the internal Rust API (§3.2) if and when it is available. The internal Rust API would provide equivalent functionality:

- Open a file by `(DirHandle, path)` → owned file handle
- Read into a buffer
- Query file size
- Close

The `heart_ffi.rs` module uses `uio_fopen`/`uio_fread`/`uio_fseek`/`uio_ftell`/`uio_fclose` through FFI and accesses the C global `contentDir`. In end state, it may use the internal Rust API and obtain `contentDir` through a Rust-side accessor.

### 13.5 Netplay debug logging

The netplay subsystem (`packetq.c`, `netsend.c`, `netrcv.c`, `crc.h`) uses `uio_fprintf` to write debug output to a `uio_Stream`. This means `uio_fprintf` and `uio_vfprintf` must be fully functional, not stubbed.

### 13.6 ZIP filesystem handler

The ZIP handler is a core subsystem component (not an external consumer). In the original C code, it lives in `libs/uio/zip/zip.c` and depends heavily on `uio_FileBlock` for reading archive contents. The Rust end state must provide equivalent ZIP reading capability:

- Parse ZIP end-of-central-directory record, central directory entries, and local file headers.
- Decompress entries using deflate (and store for uncompressed entries).
- Expose archive entries through the virtual namespace as read-only files.
- Support nested access: `uio_fopen` on a path that resolves to a ZIP entry must return a working stream.

See §10 for detailed observable behavior requirements.

---

## 14. ABI-visible struct layouts

The following struct layouts are ABI-visible (C code accesses their fields directly) and must be preserved exactly:

### 14.1 `uio_DirList` (external view)

```c
struct uio_DirList {
    const char **names;
    int numNames;
    // opaque tail permitted
};
```

C code reads `names` and `numNames` directly (e.g., `dirList->names[i]`, `dirList->numNames`). The Rust-side struct must place these two fields at the same offsets. Additional internal fields may follow.

### 14.2 `uio_Stream` ABI visibility

**Audit prerequisite (blocking).** Whether `uio_Stream` field layout must be preserved in Rust mode depends on whether any C code directly inspects `uio_Stream` fields (e.g., through `uio_INTERNAL` macros). This is a factual dependency resolved by code audit, not a design choice.

- If any C code still directly accesses `uio_Stream` fields, the following layout must be preserved exactly:

```c
struct uio_Stream {
    char *buf;
    char *dataStart;
    char *dataEnd;
    char *bufEnd;
    uio_Handle *handle;
    int status;
    uio_StreamOperation operation;
    int openFlags;
};
```

- If all access is through the function API, the struct may be treated as opaque and its layout is at the implementation's discretion.

This must be resolved by auditing C code before implementation begins. See §17.

### 14.3 `uio_AutoMount`

```c
struct uio_AutoMount {
    const char *pattern;
    match_MatchType matchType;
    uio_FileSystemID fileSystemID;
    int mountFlags;
};
```

Passed by the caller as a NULL-terminated array of pointers. The subsystem must be able to read this layout.

### 14.4 Opaque pointers

`uio_Handle`, `uio_DirHandle`, `uio_Repository`, `uio_MountHandle`, `uio_FileBlock`, and `uio_StdioAccessHandle` are opaque from the C caller's perspective. Their internal layout is at the subsystem's discretion, as long as the pointer-based API contract is honored.

---

## 15. Constants and enumerations

The following values are ABI-visible and must match the C header definitions:

| Category | Name | Value |
|----------|------|-------|
| Mount flags | `uio_MOUNT_RDONLY` | `1 << 1` (0x02) |
| Mount location | `uio_MOUNT_BOTTOM` | `0 << 2` (0x00) |
| Mount location | `uio_MOUNT_TOP` | `1 << 2` (0x04) |
| Mount location | `uio_MOUNT_BELOW` | `2 << 2` (0x08) |
| Mount location | `uio_MOUNT_ABOVE` | `3 << 2` (0x0C) |
| Mount location mask | `uio_MOUNT_LOCATION_MASK` | `3 << 2` (0x0C) |
| Filesystem type | `uio_FSTYPE_STDIO` | 1 |
| Filesystem type | `uio_FSTYPE_ZIP` | 2 |
| Match type | `match_MATCH_LITERAL` | 0 |
| Match type | `match_MATCH_PREFIX` | 1 |
| Match type | `match_MATCH_SUFFIX` | 2 |
| Match type | `match_MATCH_SUBSTRING` | 3 |
| Match type | `match_MATCH_REGEX` | 4 |
| Stream status | `uio_Stream_STATUS_OK` | 0 |
| Stream status | `uio_Stream_STATUS_EOF` | 1 |
| Stream status | `uio_Stream_STATUS_ERROR` | 2 |
| Stream operation | `uio_StreamOperation_none` | 0 |
| Stream operation | `uio_StreamOperation_read` | 1 |
| Stream operation | `uio_StreamOperation_write` | 2 |

---

## 16. Non-requirements and deferrals

The following are explicitly outside the scope of this specification:

- **Glob matching** (`match_MATCH_GLOB`): The engine does not use glob matching. The `HAVE_GLOB` define is commented out in `match.h`. The subsystem may omit it or provide a stub that returns an error.
- **Write-through to archives**: Writing to ZIP-backed files is not required. All archive mounts are read-only.
- **Network filesystems**: No network filesystem types are needed.
- **mmap-backed FileBlock**: Whether `uio_accessFileBlock` uses `mmap` or a read buffer is an internal decision. Both are acceptable as long as the pointer-stability contract is met.
- **Migration of `options.c` to Rust**: Mount policy remains C-owned. The subsystem provides primitives; it does not absorb policy.
- **Migration of `sdluio.c` to Rust**: The SDL adapter remains C-owned. The subsystem ensures its public API is sufficient for the adapter.
- **Custom filesystem types beyond stdio and ZIP**: The `uio_registerFileSystem` / `uio_unRegisterFileSystem` internal API existed in legacy C code. Whether the Rust subsystem exposes a pluggable filesystem handler registry is an internal design decision. It must at minimum support stdio and ZIP.

---

## 17. Open questions

The following items are unresolved and must be answered before implementation can be considered complete. They are collected here to prevent accidental implementation drift under unresolved assumptions.

1. **`uio_Stream` layout ABI visibility (audit prerequisite).** Is any C code under `USE_RUST_UIO` directly accessing `uio_Stream` fields (e.g., through `uio_INTERNAL` macros)? If yes, exact field layout is mandatory. If no, the struct is opaque and layout is at the implementation's discretion. **Resolution required:** audit C code before implementation begins.

2. **AutoMount parity requirement.** Is auto-mount behavior (transparent archive discovery during directory enumeration) required for current engine parity, or is it legacy API surface that current callers do not exercise? **Resolution required:** audit `options.c` and addon loading to determine if any autoMount arrays are non-NULL in practice.

3. **Temp-directory mounting.** Is process-level temporary directory mounting into the repository namespace a hard compatibility requirement used by current codepaths, or an implementation convenience inherited from legacy design? If required, which current consumers depend on it? **Resolution required:** audit callers of temp-related mount operations.

---

## 18. Implementation notes (non-normative)

The following are implementation observations and migration notes. They are not requirements and do not constrain implementation choices.

- The internal Rust-native API for Rust-side callers (§3.2) is a quality-of-life improvement that avoids FFI overhead. It should be designed after the C-ABI surface is fully correct and tested.
- Where the specification uses phrases like "search the mount registry" or "parse ZIP central directory," it describes externally observable effects, not required internal data structures. Implementations may use any internal representation that produces the specified observable behavior.
- The legacy C implementation used a mount tree structure (`mounttree.h`). The Rust implementation may use a flat list, a tree, or any other structure, as long as mount ordering and resolution semantics are preserved.
