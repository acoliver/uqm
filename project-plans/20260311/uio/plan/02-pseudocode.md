# Phase 02: Pseudocode

## Phase ID
`PLAN-20260314-UIO.P02`

## Prerequisites
- Required: Phase 01a (analysis verification) completed
- All gaps mapped to requirements

## Component 001: Stream State Machine (GAP-03, GAP-04, GAP-08, GAP-14, GAP-29, GAP-30)

```
01: FUNCTION uio_feof(stream: *mut uio_Stream) -> c_int
02:   IF stream IS NULL THEN RETURN 0
03:   RETURN (stream.status == STATUS_EOF) AS c_int
04: END

05: FUNCTION uio_ferror(stream: *mut uio_Stream) -> c_int
06:   IF stream IS NULL THEN RETURN 0
07:   RETURN (stream.status == STATUS_ERROR) AS c_int
08: END

09: FUNCTION uio_clearerr(stream: *mut uio_Stream)
10:   IF stream IS NULL THEN RETURN
11:   SET stream.status = STATUS_OK
12: END

13: FUNCTION uio_fseek_fix(stream, offset, whence) -> c_int
14:   [existing seek logic]
15:   ON SUCCESS:
16:     SET stream.status = STATUS_OK    // clear EOF and error
17:     SET stream.operation = OPERATION_NONE
18:   ON FAILURE:
19:     SET errno = EIO
20:     RETURN -1
21: END

22: FUNCTION uio_fclose_fix(stream) -> c_int
23:   IF stream IS NULL THEN
24:     SET errno = EINVAL
25:     RETURN -1
26:   END
27:   IF stream.operation == OPERATION_WRITE THEN
28:     CALL uio_fflush(stream)
29:   END
30:   CLOSE underlying handle
31:   IF stream.buf IS NOT NULL THEN
32:     DEALLOCATE stream-owned buffer using the matching allocator strategy
33:   END
34:   DEALLOCATE stream struct
35:   RETURN 0
36: END

37: FUNCTION uio_fflush_fix(stream) -> c_int
38:   IF stream IS NULL THEN
39:     SET errno = EINVAL
40:     RETURN -1 (EOF)       // legacy rejects NULL
41:   END
42:   [existing flush logic]
43: END

44: FUNCTION uio_fwrite_fix(buf, size, count, stream) -> size_t
45:   [existing write logic]
46:   SET stream.operation = OPERATION_WRITE
47:   ON I/O ERROR:
48:     SET stream.status = STATUS_ERROR
49:     SET errno = EIO
50:   RETURN items_written
51: END

52: FUNCTION uio_fputc_fix(c, stream) -> c_int
53:   SET stream.operation = OPERATION_WRITE
54:   [existing write logic]
55:   ON ERROR:
56:     SET stream.status = STATUS_ERROR
57:   RETURN c on success, EOF on error
58: END

59: FUNCTION uio_fputs_fix(s, stream) -> c_int
60:   SET stream.operation = OPERATION_WRITE
61:   [existing write logic]
62:   ON ERROR:
63:     SET stream.status = STATUS_ERROR
64:   RETURN 0 on success, EOF on error
65: END
```

## Component 002: errno and FFI failure containment (GAP-05, GAP-09, GAP-10, GAP-25)

```
66: FUNCTION set_errno(code: c_int)
67:   // Platform: macOS uses __error(), Linux uses __errno_location()
68:   UNSAFE { *libc::__error() = code }
69: END

70: FUNCTION ffi_guard_pointer_result(body) -> *mut T
71:   TRY body
72:   CATCH panic_or_internal_failure:
73:     SET errno = EIO
74:     RETURN null
75: END

76: FUNCTION ffi_guard_int_result(body) -> c_int
77:   TRY body
78:   CATCH panic_or_internal_failure:
79:     SET errno = EIO
80:     RETURN -1
81: END

82: FUNCTION ffi_guard_size_result(body) -> size_t
83:   TRY body
84:   CATCH panic_or_internal_failure:
85:     SET errno = EIO
86:     RETURN 0
87: END

88: PATTERN apply_to_all_error_paths:
89:   BEFORE every error return:
90:     SET errno to ENOENT | EINVAL | EROFS | ENOTSUP | EIO as appropriate
91:   FOR every exported stub/unsupported API:
92:     RETURN the documented failure sentinel immediately
93:     NEVER allocate dummy success objects
94: END
```

## Component 003: Mount ordering, lifecycle, and concurrency baseline (GAP-07, GAP-11, GAP-12)

```
95: STRUCT MountInfo (updated)
96:   id: usize
97:   repository: usize
98:   handle_ptr: usize
99:   mount_point: String
100:  mounted_root: PathBuf
101:  fs_type: c_int
102:  active_in_registry: bool
103:  placement_rank: usize        // explicit placement order / insertion sequence
104: END

105: FUNCTION insert_mount(registry, mount_info, flags, relative) -> usize
106:   LET location = flags & MOUNT_LOCATION_MASK
107:   MATCH location:
108:     MOUNT_TOP:
109:       VALIDATE relative IS NULL
110:       INSERT mount_info into top-placement sequence
111:     MOUNT_BOTTOM:
112:       VALIDATE relative IS NULL
113:       INSERT mount_info into bottom-placement sequence
114:     MOUNT_ABOVE:
115:       VALIDATE relative IS NOT NULL AND relative IS active in same repository
116:       INSERT mount_info immediately above referenced mount within placement order
117:     MOUNT_BELOW:
118:       VALIDATE relative IS NOT NULL AND relative IS active in same repository
119:       INSERT mount_info immediately below referenced mount within placement order
120:   RECORD new mount handle identity distinct from any source mount
121:   RETURN mount_info.id
122: END

123: FUNCTION resolve_virtual_mount_path(registry, path, operation_kind) -> Option<(mount, resolved_target)>
124:   LET normalized = normalize_virtual_path(path)
125:   LET candidates = []
126:   FOR mount IN registry snapshot under lock:
127:     IF NOT mount.active_in_registry THEN CONTINUE
128:     IF normalized matches mount.mount_point prefix THEN
129:       APPEND mount TO candidates
130:     END
131:   END
132:   SORT candidates by:
133:     1) placement precedence established by TOP/BOTTOM/ABOVE/BELOW sequencing
134:     2) longer matching mount-point prefix first
135:     3) recency / insertion order tie-breaker
136:   FOR mount IN candidates:
137:     CHECK whether mount can satisfy normalized path for operation_kind
138:     IF yes THEN RETURN Some(mount, resolved_target)
139:   END
140:   RETURN None
141: END

142: FUNCTION unmount_mount(handle) -> c_int
143:   LOCK registry
144:   FIND active mount by handle
145:   IF not found THEN SET errno = EINVAL; RETURN -1
146:   MARK mount inactive / remove from resolution set
147:   INVALIDATE mount handle as future relative anchor
148:   RELEASE registry lock
149:   CLEAN UP backing registry entries owned by the mount
150:   RETURN 0
151: END

152: FUNCTION close_repository(repo) -> c_int
153:   VALIDATE repo
154:   SERIALIZE repository teardown
155:   UNMOUNT all repository mounts before freeing repository object
156:   INVALIDATE repository pointer on return boundary
157:   RETURN 0
158: END

159: CONCURRENCY RULES
160:   - mount registry reads take a stable snapshot or hold the registry lock long enough to avoid torn state
161:   - mount mutations are serialized
162:   - blocking file/archive I/O occurs after releasing global registry lock where practical
163:   - separate handles may be used concurrently without shared mutable state races
164:   - same handle/stream state changes are synchronized via per-handle mutex or equivalent
165: END
```

## Component 004: Archive support and live-handle safety floors (GAP-01, GAP-02, GAP-11, GAP-12)

```
166: STRUCT ArchiveEntryInfo
167:   name: String
168:   compressed_size: u64
169:   uncompressed_size: u64
170:   compression_method: u16
171:   is_directory: bool
172: END

173: STRUCT MountedArchive
174:   mount_id: usize
175:   archive_path: PathBuf
176:   entries: HashMap<String, ArchiveEntryInfo>
177:   directories: HashSet<String>
178: END

179: STATIC ARCHIVE_REGISTRY: OnceLock<Mutex<Vec<MountedArchive>>> = OnceLock::new()

180: STRUCT ArchiveFileHandle
181:   data: Arc<Vec<u8>>            // keeps already-open content alive after unmount if chosen strategy allows continued reads
182:   position: usize
183:   mount_id: usize
184:   entry_path: String
185:   generation_or_liveness: token // supports clean failure if post-unmount policy chooses to reject future ops
186: END

187: FUNCTION mount_archive(mount_id, archive_path, in_path) -> Result<(), Error>
188:   OPEN archive_path as File
189:   CREATE zip::ZipArchive from File
190:   BUILD normalized entries + synthesized directories index
191:   INSERT MountedArchive into ARCHIVE_REGISTRY under lock
192:   RETURN Ok
193: END

194: FUNCTION lookup_archive_entry(mount_id, path) -> Option<ArchiveEntryInfo>
195:   LOCK archive registry
196:   FIND archive WHERE mount_id matches
197:   RETURN normalized entry clone if present
198: END

199: FUNCTION open_archive_file(mount_id, path) -> Result<ArchiveFileHandle, Error>
200:   LOOK UP archive entry metadata
201:   DECOMPRESS full entry into Vec<u8>
202:   RETURN ArchiveFileHandle with independent live object state
203: END

204: FUNCTION archive_read(handle, buf, count) -> ssize_t
205:   VALIDATE handle liveness policy
206:   COPY from handle.data[position..]
207:   ADVANCE position
208:   RETURN count or 0 at EOF
209: END

210: FUNCTION archive_seek(handle, offset, whence) -> c_int
211:   VALIDATE handle liveness policy
212:   CALCULATE new position for SEEK_SET / SEEK_CUR / SEEK_END
213:   VALIDATE bounds
214:   UPDATE position
215:   RETURN 0 on success, -1 on error
216: END

217: FUNCTION archive_fstat(handle, stat_buf) -> c_int
218:   SET stat_buf.st_size = handle.data.len()
219:   SET stat_buf.st_mode = S_IFREG | 0o444
220:   RETURN 0
221: END

222: STREAM/HANDLE DISPATCH AUDIT FOR ARCHIVE-BACKED OBJECTS
223:   - uio_open / uio_close / uio_read / uio_write / uio_lseek / uio_fstat dispatch correctly
224:   - uio_fopen / uio_fclose / uio_fread / uio_fseek / uio_ftell dispatch correctly
225:   - uio_fgetc / uio_fgets / uio_ungetc update buffering and status correctly
226:   - uio_feof / uio_ferror / uio_clearerr observe archive-backed stream state correctly
227: END
```

## Component 005: Directory enumeration and ABI-safe DirList allocation (GAP-15, GAP-16, GAP-17)

```
228: FUNCTION get_dir_list_merged(registry, dir_path, pattern, match_type) -> DirListNames
229:   LET dedup = ordered map grouped by contributing mount order
230:   LET normalized = normalize_virtual_path(dir_path)
231:   LET candidates = contributing mounts for the directory, ordered by Component 003 rules
232:   FOR mount IN candidates:
233:     ENUMERATE direct children from STDIO dir or archive dir
234:     APPLY first-seen dedup by entry name
235:     APPLY pattern filter
236:   END
237:   IF request is `.rmp` acceptance case THEN
238:     PRESERVE mount precedence with lexical-by-entry-name ordering within each contributing mount
239:   ELSE
240:     PRESERVE deterministic ordering required by current requirement set
241:   END
242:   RETURN names
243: END

244: FUNCTION matches_pattern(name, pattern, match_type) -> bool
245:   IF pattern IS empty THEN RETURN true
246:   MATCH match_type:
247:     MATCH_LITERAL: RETURN name == pattern
248:     MATCH_PREFIX: RETURN name.starts_with(pattern)
249:     MATCH_SUFFIX: RETURN name.ends_with(pattern)
250:     MATCH_SUBSTRING: RETURN name.contains(pattern)
251:     MATCH_REGEX: COMPILE regex and test match
252:     DEFAULT: SET errno = EINVAL; RETURN false
253:   END
254: END

255: FUNCTION build_c_dir_list(names) -> *mut uio_DirList
256:   // Public struct layout must match C header exactly.
257:   // Allocation bookkeeping must live outside ABI-visible fields.
258:   ALLOCATE one block or private wrapper that contains:
259:     - private bookkeeping header (not exposed as uio_DirList)
260:     - uio_DirList public struct with only names + numNames fields
261:     - names array
262:     - NUL-terminated strings
263:   RETURN pointer to public uio_DirList portion
264: END

265: FUNCTION free_c_dir_list(dirlist)
266:   IF dirlist IS NULL THEN RETURN
267:   RECOVER owning allocation from private wrapper/header strategy
268:   FREE names, strings, and public struct storage safely
269: END
```

## Component 006: FileBlock and stdio access cleanup behavior (GAP-19, GAP-20, GAP-23, GAP-24, GAP-31)

```
270: STRUCT uio_FileBlock
271:   handle: retained reference to underlying handle
272:   offset: u64
273:   size: u64
274:   data: Option<Vec<u8>>
275: END

276: FUNCTION uio_openFileBlock2(handle, offset, size) -> *mut uio_FileBlock
277:   VALIDATE handle
278:   ON unsupported or unimplemented path:
279:     SET errno = ENOTSUP
280:     RETURN null
281:   ON partial allocation failure:
282:     CLEAN UP and RETURN null
283:   RETURN live block object
284: END

285: FUNCTION uio_getFileLocation(dir, path, flags, mount_out, out_path) -> c_int
286:   RESOLVE path through ordered mount resolution
287:   IF STDIO-backed THEN
288:     RETURN real path + owning mount
289:   ELSE IF archive-backed THEN
290:     RETURN successful owning-mount information and backing-location token sufficient for stdio temp-copy bridge
291:   ELSE
292:     SET errno = ENOTSUP or ENOENT as appropriate
293:     RETURN -1
294:   END
295: END

296: FUNCTION uio_getStdioAccess(dir, path, flags) -> *mut uio_StdioAccessHandle
297:   CALL uio_getFileLocation or equivalent internal resolution
298:   IF STDIO-backed THEN RETURN direct-path handle
299:   IF archive-backed THEN
300:     CREATE temp directory
301:     COPY entry contents to temp file
302:     RETURN handle owning temp resources
303:   ON any failure after partial setup:
304:     CLEAN UP temp resources
305:     RETURN null
306: END
```

## Component 007: Transplant semantics and post-unmount safety floors (GAP-21, GAP-12)

```
307: FUNCTION uio_transplantDir(mountPoint, sourceDir, flags, relative) -> *mut uio_MountHandle
308:   VALIDATE sourceDir belongs to same repository as relative when relative is provided
309:   DETERMINE backing content referenced by sourceDir
310:   CREATE a NEW mount record with its own mount handle identity
311:   IF backing content is STDIO-backed THEN
312:     REFERENCE same physical root / subpath view
313:   ELSE IF backing content is archive-backed THEN
314:     REFERENCE shared archive backing metadata, but DO NOT reuse original mount handle identity
315:   INSERT transplanted mount according to Component 003 placement rules
316:   RETURN new mount handle
317: END

318: POST-UNMOUNT SAFETY RULES
319:   - live directory handles remain valid allocated objects until close, even if underlying mount is gone
320:   - live file handles / streams either continue operating via retained backing state or fail cleanly after unmount
321:   - neither path may crash, return fake success, or produce undefined behavior
322:   - shutdown-order violations keep the same no-crash/no-UB safety floor
323: END
```
