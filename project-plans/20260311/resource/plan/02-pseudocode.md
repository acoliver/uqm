# Phase 02: Pseudocode

## Phase ID
`PLAN-20260314-RESOURCE.P02`

## Prerequisites
- Required: Phase 01/01a completed
- Verify previous phase markers/artifacts exist
- Expected files from previous phase: `01-analysis.md`, `01a-analysis-verification.md`

## Requirements Implemented (Expanded)

### REQ-RES-UNK-001: Unknown type fallback
**Requirement text**: When a resource index entry declares a type that is not registered in the type registry, the resource subsystem shall store the entry as the built-in unknown type with the descriptor string preserved, rather than discarding the entry.

Behavior contract:
- GIVEN: An index line declares an unregistered type name
- WHEN: the descriptor is processed into the authoritative resource map
- THEN: the entry is stored as UNKNOWNRES with the descriptor preserved in value-type storage

Why it matters:
- Unknown descriptors must remain observable and debuggable instead of disappearing from the map.

### REQ-RES-LOAD-011: Value-type access through the general resource accessor
**Requirement text**: When the general resource-access function is applied to a value-type entry (including entries of the built-in unknown type), the resource subsystem shall return the entry's current data union representation without invoking heap-style lazy-load semantics, and shall increment the reference count.

Behavior contract:
- GIVEN: A value-type entry is present in the map
- WHEN: `res_GetResource` dispatches through the general accessor
- THEN: it returns the current union-backed representation and increments refcount without lazy loading

Why it matters:
- Existing consumers rely on the generic accessor working uniformly for both heap and scalar/value entries.

### REQ-RES-FILE-003: Directory sentinel compatibility
**Requirement text**: When the established resource-file open helper encounters a directory-like target, the resource subsystem shall return the established sentinel handle value rather than null or a normal stream pointer, so that callers that test for the sentinel can distinguish directories from regular files. The file-length helper shall return `1` for the sentinel handle.

Behavior contract:
- GIVEN: A resource path resolves to a directory
- WHEN: `res_OpenResFile` is called
- THEN: it returns `STREAM_SENTINEL`, and downstream load helpers must treat that sentinel as not loadable content

Why it matters:
- Directory detection only helps if later helper layers reject the sentinel before invoking file loaders.

### REQ-RES-FILE-005: File-backed typed load integration
**Requirement text**: When a type-specific loader is invoked through the file-backed load helper, the resource subsystem shall open the resource relative to the established content/UIO environment, pass a compatible file handle and length to the loader callback, and close the file according to the established ownership contract after the callback returns.

Behavior contract:
- GIVEN: `LoadResourceFromPath` is asked to load a path
- WHEN: the open result is null, sentinel, or zero-length
- THEN: the helper returns failure without invoking the loader callback and closes any owned handle correctly

Why it matters:
- Loader callbacks must see only valid file streams and meaningful lengths.

## Pseudocode for All Gap Fixes

### PC-1: UNKNOWNRES Registration (GAP-3 part 1)

```text
01: FUNCTION unknownres_load_fun(descriptor: *const c_char, data: *mut ResourceData)
02:   data.str_ptr = descriptor   // store raw descriptor pointer as string
03: END FUNCTION
04:
05: // In register_builtin_types:
06: INSTALL type "UNKNOWNRES" with:
07:   loadFun = unknownres_load_fun
08:   freeFun = None         // value type — no free needed
09:   toString = None        // not serializable
```

### PC-2: process_resource_desc UNKNOWNRES Fix (GAP-3 part 2)

```text
10: FUNCTION process_resource_desc(key, type_name, path)
11:   handlers = type_registry.lookup(type_name)
12:   IF handlers IS Some THEN
13:     handler_key = type_name
14:     is_value_type = handlers.free_fun IS None
15:   ELSE
16:     WARN "Unknown resource type '{type_name}' for key '{key}'"
17:     handler_key = "UNKNOWNRES"
18:     is_value_type = TRUE    // UNKNOWNRES has no freeFun → value type
19:   END IF
20:
21:   data = ResourceData::default()
22:   fname_cstring = CString::new(path)
23:
24:   IF is_value_type THEN
25:     actual_handler = type_registry.lookup(handler_key)  // use handler_key, not type_name
26:     IF actual_handler.load_fun IS Some THEN
27:       CALL actual_handler.load_fun(fname_cstring.as_ptr(), &mut data)
28:     END IF
29:   END IF
30:
31:   // Replace existing entry if present — call freeFun on old loaded heap resource
32:   IF entries.contains(key) THEN
33:     old_entry = entries.get(key)
34:     IF old_entry.data.ptr IS NOT null THEN
35:       old_handler = type_registry.lookup(old_entry.type_handler_key)
36:       IF old_handler.free_fun IS Some THEN
37:         IF old_entry.refcount > 0 THEN
38:           WARN "Replacing resource '{key}' with refcount={old_entry.refcount}"
39:         END IF
40:         CALL old_handler.free_fun(old_entry.data.ptr)
41:       END IF
42:     END IF
43:   END IF
44:
45:   entries.insert(key, FullResourceDesc { fname, res_type: handler_key for UNKNOWNRES else type_name, data, refcount: 0, type_handler_key: handler_key, fname_cstring })
46: END FUNCTION
```

### PC-3: get_resource Value-Type Path (GAP-4)

```text
50: FUNCTION get_resource(key) -> Option<*mut c_void>
51:   entry = entries.get_mut(key)?
52:   handler = type_registry.lookup(entry.type_handler_key)
53:
54:   // Determine if this is a value type or heap type
55:   has_free_fun = handler.map(|h| h.free_fun.is_some()).unwrap_or(false)
56:
57:   IF has_free_fun THEN
58:     // --- Heap type: lazy load ---
59:     IF entry.data.ptr IS null THEN
60:       IF handler.load_fun IS Some THEN
61:         CALL handler.load_fun(entry.fname_cstring.as_ptr(), &mut entry.data)
62:       END IF
63:     END IF
64:     IF entry.data.ptr IS null THEN
65:       RETURN None  // load failed
66:     END IF
67:     entry.refcount += 1
68:     RETURN Some(entry.data.ptr)
69:   ELSE
70:     // --- Value type: already populated ---
71:     entry.refcount += 1
72:     // Return appropriate union field based on storage type
73:     // For STRING/UNKNOWNRES (str_ptr), return the string pointer
74:     // For INT32/BOOLEAN/COLOR (num), return num cast to *mut c_void
75:     // Discriminator: if str_ptr is non-null, return str_ptr; else return num as ptr
76:     IF entry.data.str_ptr IS NOT null THEN
77:       RETURN Some(entry.data.str_ptr as *mut c_void)
78:     ELSE
79:       RETURN Some(entry.data.num as *mut c_void)
80:     END IF
81:   END IF
82: END FUNCTION
```

### PC-4: res_GetString Type Check (GAP-2)

```text
90: FUNCTION res_GetString(key: *const c_char) -> *const c_char
91:   IF key IS null THEN
92:     RETURN static_empty_string  // b"\0"
93:   END IF
94:   key_str = CStr::from_ptr(key).to_str()
95:   state = RESOURCE_STATE.lock()
96:   dispatch = state.dispatch
97:
98:   entry = dispatch.entries.get(key_str)
99:   IF entry IS None THEN
100:    RETURN static_empty_string
101:  END IF
102:
103:  IF entry.res_type != "STRING" THEN
104:    RETURN static_empty_string
105:  END IF
106:
107:  IF entry.data.str_ptr IS null THEN
108:    RETURN static_empty_string
109:  END IF
110:
111:  RETURN entry.data.str_ptr
112: END FUNCTION
```

### PC-5: UninitResourceSystem Cleanup (GAP-5)

```text
120: FUNCTION UninitResourceSystem()
121:   guard = RESOURCE_STATE.lock()
122:   IF guard IS None THEN
123:     WARN "UninitResourceSystem called when not initialized"
124:     RETURN
125:   END IF
126:
127:   state = guard.take()  // take ownership
128:   // Iterate all entries and free loaded heap resources
129:   FOR EACH (key, entry) IN state.dispatch.entries
130:     IF entry.data.ptr IS NOT null THEN
131:       handler = state.dispatch.type_registry.lookup(entry.type_handler_key)
132:       IF handler.free_fun IS Some THEN
133:         CALL handler.free_fun(entry.data.ptr)
134:         // Note: we don't need to null the ptr since we're destroying everything
135:       END IF
136:     END IF
137:   END FOR
138:   // state drops, releasing all Rust-owned memory
139: END FUNCTION
```

### PC-6: SaveResourceIndex toString Filtering (GAP-7)

```text
150: FUNCTION SaveResourceIndex(dir, rmpfile, root, strip_root)
151:   // ... open file, iterate entries ...
152:   FOR EACH (key, entry) IN dispatch.entries
153:     IF root IS NOT empty AND NOT key.starts_with(root) THEN
154:       CONTINUE
155:     END IF
156:
157:     handler = type_registry.lookup(entry.type_handler_key)
158:     IF handler IS None OR handler.to_string_fun IS None THEN
159:       CONTINUE  // ← GAP-7 fix: SKIP entries without toString
160:     END IF
161:
162:     // Call toString to serialize
163:     buffer = [0u8; 256]
164:     CALL handler.to_string_fun(&entry.data, buffer.as_mut_ptr(), 256)
165:     output_key = IF strip_root THEN key[root.len()..] ELSE key
166:     WRITE "{output_key} = {entry.res_type}:{buffer_str}\n"
167:   END FOR
168:   // ... close file ...
169: END FUNCTION
```

### PC-7: res_OpenResFile Directory Sentinel (GAP-1)

```text
180: FUNCTION res_OpenResFile(dir, file, mode) -> *mut c_void
181:   // Exact ABI/signature for uio_stat must come from Phase 0.5 artifacts.
182:   // The stat buffer type and mode test below are contingent examples, not
183:   // final code, until preflight confirms the C signature and layout.
184:   stat_buf = MaybeUninit<verified_stat_type>::uninit()
185:   rc = uio_stat(dir, file, stat_buf.as_mut_ptr())
186:   IF rc == 0 THEN
187:     st = stat_buf.assume_init()
188:     IF verified_is_directory(st) THEN
189:       RETURN STREAM_SENTINEL  // (uio_Stream*)~0
190:     END IF
191:   END IF
192:   // Normal file open
193:   RETURN uio_fopen(dir, file, mode)
194: END FUNCTION
```

### PC-8: LoadResourceFromPath Invalid-Open / Zero-Length Guard (GAP-9)

```text
200: FUNCTION LoadResourceFromPath(path, load_fun) -> *mut c_void
201:   file = res_OpenResFile(contentDir, path, "rb")
202:   IF file IS null THEN
203:     WARN "Failed to open resource file: {path}"
204:     RETURN null
205:   END IF
206:   IF file == STREAM_SENTINEL THEN
207:     WARN "Resource path is a directory, not a loadable file: {path}"
208:     RETURN null
209:   END IF
210:   length = LengthResFile(file)
211:   IF length == 0 THEN
212:     WARN "Zero-length resource file: {path}"
213:     uio_fclose(file)
214:     RETURN null
215:   END IF
216:   set _cur_resfile_name = path
217:   result = load_fun(file, length)
218:   clear _cur_resfile_name
219:   uio_fclose(file)
220:   RETURN result
221: END FUNCTION
```

### PC-9: CountResourceTypes Return Type (GAP-8)

```text
230: FUNCTION CountResourceTypes() -> u32   // was u16
231:   state = ensure_initialized()
232:   RETURN state.dispatch.type_registry.count() as u32
233: END FUNCTION
```

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] Every gap (GAP-1 through GAP-10) has pseudocode
- [ ] Pseudocode lines are numbered for traceability
- [ ] Pseudocode includes validation, error handling, ordering constraints
- [ ] Integration boundaries are marked (where C callbacks are invoked)
- [ ] Unverified ABI details are clearly marked as contingent on preflight artifacts

## Semantic Verification Checklist (Mandatory)
- [ ] PC-1/PC-2 correctly handles UNKNOWNRES as value type
- [ ] PC-3 correctly discriminates value vs heap types
- [ ] PC-4 correctly returns "" not null for all non-STRING cases
- [ ] PC-5 iterates all entries and calls freeFun before dropping state
- [ ] PC-6 skips entries without toString (does not format fallback)
- [ ] PC-7 checks `uio_stat` before `uio_fopen` without pretending the ABI is already proven
- [ ] PC-8 rejects both sentinel and zero-length inputs before calling the loader

## Success Criteria
- [ ] Every planned behavior change is represented in pseudocode
- [ ] Contingent ABI details are separated from verified facts
- [ ] Verification commands pass
- [ ] Semantic checks pass

## Failure Recovery
- rollback steps: `git checkout -- project-plans/20260311/resource/plan/02-pseudocode.md`
- blocking issues to resolve before next phase: confirmed `uio_stat` ABI and confirmed `LoadResourceFromPath` sentinel behavior

## Phase Completion Marker
Create: `project-plans/20260311/resource/.completed/P02.md`

Contents:
- phase ID
- timestamp
- files changed
- tests added/updated: none
- verification outputs
- semantic verification summary
