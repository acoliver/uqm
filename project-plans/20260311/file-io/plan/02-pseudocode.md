# Phase 02: Pseudocode

## Phase ID
`PLAN-20260314-FILE-IO.P02`

## Prerequisites
- Required: Phase 01a completed (analysis verification passed)

## Purpose
Produce algorithmic pseudocode for each gap-closure implementation. All subsequent implementation phases reference these line numbers.

## Pseudocode Components

### PC-01: Stream Status Tracking (REQ-FIO-STREAM-STATUS, REQ-FIO-RESOURCE-MGMT, REQ-FIO-ABI-AUDIT)

```text
01: STRUCT uio_Stream
02:   buf, data_start, data_end, buf_end: *mut c_char
03:   handle: *mut uio_Handle
04:   status: c_int
05:   operation: c_int
06:   open_flags: c_int
07:   -- If P00a audit found direct C field access, preserve exact field order/offsets
08: END STRUCT
09:
10: FUNCTION uio_feof(stream) -> c_int
11:   IF stream IS NULL RETURN 0
12:   RETURN (stream.status == UIO_STREAM_STATUS_EOF) as c_int
13: END
14:
15: FUNCTION uio_ferror(stream) -> c_int
16:   IF stream IS NULL RETURN 0
17:   RETURN (stream.status == UIO_STREAM_STATUS_ERROR) as c_int
18: END
19:
20: FUNCTION uio_clearerr(stream)
21:   IF stream IS NULL RETURN
22:   stream.status = UIO_STREAM_STATUS_OK
23: END
24:
25: FUNCTION set_stream_eof(stream)
26:   IF stream IS NOT NULL
27:     stream.status = UIO_STREAM_STATUS_EOF
28: END
29:
30: FUNCTION set_stream_error(stream)
31:   IF stream IS NOT NULL
32:     stream.status = UIO_STREAM_STATUS_ERROR
33: END
34:
35: FUNCTION uio_fclose(stream) -> c_int
36:   IF stream IS NULL RETURN EOF
37:   flush stream if writable
38:   close underlying handle
39:   IF stream.buf IS NOT NULL AND buf was internally allocated
40:     free(stream.buf)
41:   free(stream struct)
42:   RETURN 0
43: END
```

### PC-02: Stream Output — `uio_vfprintf` (REQ-FIO-STREAM-WRITE, REQ-FIO-BUILD-BOUNDARY)

```text
01: FUNCTION uio_vfprintf(stream, format, args) -> c_int
02:   IF stream IS NULL OR format IS NULL
03:     set_errno(EINVAL)
04:     RETURN -1
05:   -- If helper is required for va_list formatting, helper may be internal-only,
06:   -- but no exported uio_* symbol may depend on a C shim for its ABI presence.
07:   LET formatted = format_args_via_selected_strategy(format, args)
08:   IF formatted IS error RETURN -1
09:   WRITE formatted bytes to stream via uio_fwrite
10:   RETURN formatted.length
11: END
```

### PC-03: Direct `uio_fread` Export (REQ-FIO-BUILD-BOUNDARY)

```text
01: FUNCTION uio_fread(buf, size, nmemb, stream) -> size_t
02:   -- Export exact public symbol from Rust
03:   -- Same body as current Rust implementation after stream-status integration
04: END
05:
06: -- Remove uio_fread_shim.c from active Rust build inputs
07: -- Keep headers aligned to the direct Rust export
```

### PC-04: Path Normalization and Host Confinement (REQ-FIO-PATH-NORM, REQ-FIO-PATH-CONFINEMENT)

```text
01: FUNCTION normalize_virtual_path(base_virtual, input_path) -> PathBuf
02:   IF input_path == ""
03:     RETURN base_virtual
04:   LET combined = IF input_path is absolute THEN input_path ELSE base_virtual.join(input_path)
05:   LET result = empty component stack
06:   FOR EACH logical component in combined
07:     IF component == "" OR component == "." THEN continue
08:     IF component == ".."
09:       IF result not empty THEN pop else stay clamped at root
10:     ELSE push component
11:   RETURN rooted path from result
12: END
13:
14: FUNCTION map_virtual_to_host(mount_root, mount_relative_components) -> Result<PathBuf, errno>
15:   LET host_components = empty component stack
16:   FOR EACH component in mount_relative_components
17:     IF component == ".."
18:       IF host_components empty
19:         -- do not escape above mount physical root
20:         CONTINUE
21:       ELSE pop host_components
22:     ELSE push component
23:   RETURN mount_root.join(host_components)
24: END
```

### PC-05: `errno`, Validation, Safe Failure, and Panic Containment (REQ-FIO-ERRNO, REQ-FIO-PANIC-SAFETY)

```text
01: FUNCTION set_errno(code: c_int)
02:   use platform-appropriate errno setter
03: END
04:
05: FUNCTION fail_errno[T](code, failure_return) -> T
06:   set_errno(code)
07:   RETURN failure_return
08: END
09:
10: FUNCTION ffi_guard(default_failure_return, body) -> ReturnType
11:   TRY
12:     RETURN body()
13:   CATCH Rust panic
14:     set_errno(EIO or audited fallback)
15:     RETURN default_failure_return
16: END
17:
18: -- Every public entry point validates null pointers, invalid flags, invalid whence,
19: -- invalid mode strings, and unsupported combinations before mutating shared state.
20: -- Partial allocations are unwound before failure returns.
21: -- No panic may unwind across any exported extern "C" boundary.
```

### PC-06: Mount Ordering and Conditional Branches (REQ-FIO-MOUNT-ORDER, REQ-FIO-MOUNT-AUTOMOUNT, REQ-FIO-THREAD-SAFETY, REQ-FIO-ERRNO)

```text
01: STRUCT MountInfo
02:   id: usize
03:   repository: usize
04:   handle_ptr: usize
05:   mount_point: String
06:   mounted_root: PathBuf
07:   fs_type: c_int
08:   active_in_registry: bool
09:   position: usize
10:   read_only: bool
11:   auto_mount_rules: optional copied rule set
12: END
13:
14: FUNCTION register_mount(repo, mount_point, root, fs_type, auto_mount, flags, relative) -> Result<MountHandle, errno>
15:   VALIDATE location/relative combination
16:   DERIVE position from TOP/BOTTOM/ABOVE/BELOW semantics
17:   STORE read_only and any required AutoMount rules
18:   INSERT mount deterministically under registry lock
19:   RETURN handle
20: END
21:
22: FUNCTION maybe_apply_automount_rules(listing_context)
23:   IF P00a says AutoMount not required
24:     RETURN without mutation
25:   FOR matching entry in listing
26:     attempt mount at repository bottom
27:     log failure and continue on mount error
28: END
29:
30: FUNCTION registry_iteration_and_mutation_contract()
31:   hold registry lock while snapshotting or mutating topology
32:   ensure readers never observe partially inserted/removed mounts
33: END
```

### PC-07: Access Semantics (REQ-FIO-ACCESS-MODE)

```text
01: FUNCTION uio_access(dir, path, mode) -> c_int
02:   VALIDATE mode bits and arguments
03:   LET visible = resolve_topmost_visible_object(dir, path)
04:   IF missing RETURN fail_errno(ENOENT, -1)
05:   IF mode == F_OK RETURN 0
06:   IF mode has unsupported bits RETURN fail_errno(EINVAL, -1)
07:   IF mode includes R_OK THEN readable => continue
08:   IF mode includes W_OK
09:     IF visible is archive-backed OR mount.read_only RETURN fail_errno(EACCES, -1)
10:   IF mode includes X_OK
11:     IF visible is directory RETURN 0
12:     IF visible is archive-backed RETURN fail_errno(EACCES, -1)
13:     delegate to host access() for stdio-backed file
14:   RETURN 0
15: END
```

### PC-08: Overlay Mutation Resolution (REQ-FIO-MUTATION, REQ-FIO-ERRNO)

```text
01: FUNCTION resolve_parent_for_mutation(dir, target_path) -> Result<(parent_mount, host_parent), errno>
02:   LET parent_virtual = parent_of(normalize_virtual_path(dir.path, target_path))
03:   CHECK visible parent through overlay precedence
04:   IF any traversed parent component is shadowed by visible non-directory
05:     RETURN Err(ENOTDIR)
06:   IF visible parent exists only in read-only layer and no writable mount exposes same parent path
07:     RETURN Err(EACCES)
08:   RETURN topmost writable mount covering parent path
09: END
10:
11: FUNCTION resolve_existing_for_write(dir, target_path) -> Result<(mount, host_path), errno>
12:   LET visible = resolve_topmost_visible_object(dir, target_path)
13:   IF missing RETURN Err(ENOENT)
14:   IF visible.mount.read_only RETURN Err(EACCES)
15:   RETURN visible
16: END
17:
18: FUNCTION uio_rename(old_dir, old_path, new_dir, new_path) -> c_int
19:   RESOLVE source through topmost visible object
20:   RESOLVE destination parent through overlay rules
21:   IF source mount read-only RETURN fail_errno(EACCES, -1)
22:   IF source mount != destination mount RETURN fail_errno(EXDEV, -1)
23:   IF destination visible object exists on different mount RETURN fail_errno(EXDEV, -1)
24:   perform host rename
25: END
```

### PC-09: Regex Compatibility Decision (REQ-FIO-DIRLIST-REGEX)

```text
01: FUNCTION compile_listing_regex(pattern) -> Result<RegexHandle, error>
02:   USE the engine chosen in P00a/P01 analysis
03:   DO NOT claim exact POSIX ERE unless that engine/adapter is proven compatible
04: END
05:
06: FUNCTION matches_pattern(name, pattern, match_type) -> bool
07:   IF pattern empty RETURN true
08:   HANDLE literal/prefix/suffix/substring directly
09:   HANDLE REGEX via audited compatibility layer
10:   invalid regex => return false without crash
11: END
```

### PC-10: Cross-Mount Directory Listing (REQ-FIO-DIRLIST-UNION, REQ-FIO-DIRLIST-EMPTY, REQ-FIO-MOUNT-AUTOMOUNT, REQ-FIO-THREAD-SAFETY)

```text
01: FUNCTION uio_getDirList(dir, path, pattern, matchType) -> *mut DirList
02:   LET virtual_path = normalize_virtual_path(dir.path, path)
03:   ACQUIRE registry snapshot or lock consistent with topology contract
04:   COLLECT visible names by iterating mounts in precedence order
05:   PRESERVE first-seen order from precedence iteration for deterministic output
06:   DEDUP by visible entry name while preserving order
07:   APPLY match semantics
08:   IF AutoMount required, evaluate rules during enumeration and merge new mounts per contract
09:   RETURN non-null empty list when directory resolves but no names match
10: END
11:
12: FUNCTION build_dirlist_result(names) -> *mut uio_DirList
13:   PRESERVE ABI-visible first two fields exactly: names, numNames
14:   OWN allocation so uio_DirList_free can release without hidden side channels
15: END
```

### PC-11: FileBlock Implementation (REQ-FIO-FILEBLOCK, REQ-FIO-ERRNO)

```text
01: STRUCT FileBlockInner
02:   handle: *mut uio_Handle
03:   base_offset: off_t
04:   size: off_t
05:   cache: Vec<u8>
06:   cache_offset: off_t
07:   cache_length: usize
08: END
09:
10: FUNCTION uio_openFileBlock(handle) -> *mut FileBlock
11:   RETURN block covering entire file
12: END
13:
14: FUNCTION uio_openFileBlock2(handle, offset, size) -> *mut FileBlock
15:   VALIDATE offset/size
16:   RETURN block covering specific file region
17: END
18:
19: FUNCTION uio_accessFileBlock(block, offset, length, out_buffer) -> ssize_t
20:   VALIDATE args
21:   READ requested range into internal cache
22:   SET *out_buffer = pointer to internal cache bytes
23:   RETURN bytes available (<= requested length, may be short at EOF)
24: END
25:
26: FUNCTION uio_clearFileBlockBuffers(block)
27:   CLEAR internal cache without invalidating the block
28: END
29:
30: FUNCTION uio_copyFileBlock(block, offset, buffer, length) -> c_int
31:   VALIDATE args
32:   COPY requested bytes into caller buffer
33:   RETURN 0 or -1
34: END
35:
36: FUNCTION uio_closeFileBlock(block) -> c_int
37:   FREE block resources
38:   RETURN 0
39: END
```

### PC-12: StdioAccess and Copy (REQ-FIO-STDIO-ACCESS, REQ-FIO-COPY, REQ-FIO-MOUNT-TEMP, REQ-FIO-POST-UNMOUNT-CLEANUP, REQ-FIO-ERRNO, REQ-FIO-THREAD-SAFETY)

```text
01: STRUCT StdioAccessHandle
02:   path: *mut c_char
03:   kind: DIRECT or TEMP_COPY
04:   temp_dir: optional PathBuf
05:   temp_file: optional PathBuf
06:   source_mount_generation: optional topology token
07: END
08:
09: FUNCTION uio_getStdioAccess(dir, path, flags, tempDir) -> *mut StdioAccessHandle
10:   RESOLVE virtual object via winning visible mount, not public getFileLocation shortcut alone
11:   IF resolved object is directory RETURN fail_errno(EISDIR, NULL)
12:   IF stdio-backed single concrete file RETURN direct-path handle
13:   ELSE materialize temp copy under caller-provided tempDir or audited fallback temp root
14:   RETURN temp-copy handle
15: END
16:
17: FUNCTION uio_releaseStdioAccess(handle)
18:   IF handle.kind == TEMP_COPY
19:     best-effort delete temp file and temp dir, log cleanup failure only
20:   FREE bookkeeping
21: END
22:
23: FUNCTION uio_copyFile(srcDir, srcPath, dstDir, dstPath) -> c_int
24:   OPEN source and destination through virtual namespace rules
25:   COPY in chunks
26:   ON partial failure: close handles, remove partial destination, preserve errno/state
27: END
28:
29: -- If P00a says process-level temp-directory mounting is required,
30: -- add repository-visible temp mount setup/cleanup path in Phase 10/11.
```

### PC-13: Lifecycle, Cleanup Safety, and Threading (REQ-FIO-LIFECYCLE, REQ-FIO-POST-UNMOUNT-CLEANUP, REQ-FIO-THREAD-SAFETY, REQ-FIO-RESOURCE-MGMT)

```text
01: FUNCTION uio_init()
02:   initialize global registries and initialized flag idempotently
03: END
04:
05: FUNCTION uio_unInit()
06:   REQUIRE repositories closed / operations quiesced by caller contract
07:   clear subsystem-owned registries safely
08: END
09:
10: FUNCTION uio_unmountDir(mount)
11:   remove mount from topology under lock
12:   leave already-open handles/streams closeable even if future I/O becomes invalid
13: END
14:
15: FUNCTION cleanup_after_mount_removal(handle_or_stream_or_dir)
16:   ensure close/fclose/closeDir/releaseStdioAccess remain safe after unmount
17: END
18:
19: FUNCTION returned_allocation_lifetime_rules()
20:   dirlists, outPath strings, and stdio paths remain safely consumable until documented free/release point
21: END
22:
23: FUNCTION concurrency_race_audit_targets()
24:   VERIFY mount registry iteration vs mutation
25:   VERIFY repository close vs open handles/dirlists/stdio access handles
26:   VERIFY returned allocations remain valid until documented release point
27: END
```

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P02.md` containing:
- pseudocode component list
- branch notes for AutoMount/temp mount/regex engine decisions
- ABI-sensitive constraints carried into implementation phases
