# Component 001: State File I/O — Pseudocode

## A. state.c Function Redirects

When `USE_RUST_STATE` is defined, each function computes the file index
from the pointer and delegates to Rust.

### OpenStateFile redirect
```
 1: FUNCTION OpenStateFile(stateFile: int, mode: *char) → GAME_STATE_FILE*
 2:   IF stateFile < 0 OR stateFile >= NUM_STATE_FILES
 3:     RETURN NULL
 4:   result = rust_open_state_file(stateFile, mode)
 5:   IF result == 0
 6:     RETURN NULL
 7:   RETURN &state_files[stateFile]   // pointer to static array element
```

### CloseStateFile redirect
```
10: FUNCTION CloseStateFile(fp: GAME_STATE_FILE*)
11:   file_index = (int)(fp - state_files)
12:   IF file_index < 0 OR file_index >= NUM_STATE_FILES
13:     RETURN
14:   rust_close_state_file(file_index)
```

### ReadStateFile redirect
```
17: FUNCTION ReadStateFile(lpBuf: *void, size: COUNT, count: COUNT, fp: GAME_STATE_FILE*) → int
18:   file_index = (int)(fp - state_files)
19:   IF file_index < 0 OR file_index >= NUM_STATE_FILES
20:     RETURN 0
21:   RETURN (int)rust_read_state_file(file_index, lpBuf, size, count)
```

### WriteStateFile redirect
```
24: FUNCTION WriteStateFile(lpBuf: *void, size: COUNT, count: COUNT, fp: GAME_STATE_FILE*) → int
25:   file_index = (int)(fp - state_files)
26:   IF file_index < 0 OR file_index >= NUM_STATE_FILES
27:     RETURN 0
28:   RETURN (int)rust_write_state_file(file_index, lpBuf, size, count)
```

### SeekStateFile redirect
```
31: FUNCTION SeekStateFile(fp: GAME_STATE_FILE*, offset: long, whence: int) → int
32:   file_index = (int)(fp - state_files)
33:   IF file_index < 0 OR file_index >= NUM_STATE_FILES
34:     RETURN 0
35:   RETURN rust_seek_state_file(file_index, (int64_t)offset, whence)
```

### LengthStateFile redirect
```
38: FUNCTION LengthStateFile(fp: GAME_STATE_FILE*) → DWORD
39:   file_index = (int)(fp - state_files)
40:   IF file_index < 0 OR file_index >= NUM_STATE_FILES
41:     RETURN 0
42:   RETURN (DWORD)rust_length_state_file(file_index)
```

### DeleteStateFile redirect
```
45: FUNCTION DeleteStateFile(stateFile: int)
46:   IF stateFile < 0 OR stateFile >= NUM_STATE_FILES
47:     RETURN
48:   rust_delete_state_file(stateFile)
```

## B. Seek-Past-End Fix

Fix `StateFile::seek` to remove upper-bound clamping.

### seek (fixed)
```
51: FUNCTION seek(self, offset: i64, whence: SeekWhence) → Result
52:   new_pos = MATCH whence:
53:     Set:     offset
54:     Current: self.ptr as i64 + offset
55:     End:     self.used as i64 + offset
56:   IF new_pos < 0
57:     self.ptr = 0
58:     RETURN Err(SeekClamped)   // or Ok with return value 0
59:   self.ptr = new_pos as usize
60:   RETURN Ok(())               // return value 1
```

**Critical change**: Lines 56-60 replace the old logic that clamped
`new_pos` to `self.data.len()`. Now `self.ptr` can be any non-negative
value, even far beyond the buffer.

### read (updated for separate used/physical tracking)
```
63: FUNCTION read(self, buf: &mut [u8]) → Result<usize>
64:   physical_size = self.data.len()
65:   IF self.ptr >= physical_size
66:     RETURN Ok(0)          // EOF
67:   available = physical_size - self.ptr
68:   bytes_to_read = MIN(buf.len(), available)
69:   IF bytes_to_read > 0
70:     COPY self.data[self.ptr .. self.ptr + bytes_to_read] → buf
71:     self.ptr += bytes_to_read
72:   RETURN Ok(bytes_to_read)
```

### write (updated for separate used tracking)
```
75: FUNCTION write(self, buf: &[u8]) → Result<()>
76:   required_end = self.ptr + buf.len()
77:   IF required_end > self.data.len()
78:     new_size = MAX(required_end, self.data.len() * 3 / 2)
79:     self.data.resize(new_size, 0)     // zero-fills gap
80:     IF new_size > self.size_hint
81:       self.size_hint = new_size
82:   COPY buf → self.data[self.ptr .. self.ptr + buf.len()]
83:   self.ptr += buf.len()
84:   IF self.ptr > self.used
85:     self.used = self.ptr
86:   RETURN Ok(())
```

### length (returns used, not data.len())
```
89: FUNCTION length(self) → usize
90:   RETURN self.used
```

### open (pre-allocates physical buffer)
```
93: FUNCTION open(self, mode: FileMode) → Result
94:   self.open_count += 1
95:   IF self.open_count > 1
96:     LOG warning
97:   IF self.data.is_empty()
98:     self.data = vec![0u8; self.size_hint]   // physical allocation
99:     self.used = 0                            // logical size = 0
100:  MATCH mode:
101:    Write:
102:      self.used = 0
103:      // data stays allocated at physical size, just reset logical
104:    Read | ReadWrite:
105:      // preserve used
106:  self.ptr = 0
107:  RETURN Ok(())
```

### delete (frees buffer, resets everything)
```
110: FUNCTION delete(self)
111:   IF self.open_count != 0
112:     LOG warning
113:   self.data.clear()
114:   self.data.shrink_to_fit()
115:   self.used = 0
116:   self.ptr = 0
```

## C. Deadlock Fix for rust_copy_game_state

### copy_game_state (fixed)
```
119: FUNCTION rust_copy_game_state(dest_bit: int, src_start_bit: int, src_end_bit: int)
120:   guard = GLOBAL_GAME_STATE.lock()         // Single lock acquisition
121:   IF guard is None
122:     RETURN
123:   state = guard.unwrap()
124:   // Snapshot source bits before mutating
125:   snapshot_bytes = state.as_bytes().clone()
126:   src_snapshot = GameState::from_bytes(&snapshot_bytes)
127:   state.copy_state(dest_bit, &src_snapshot, src_start_bit, src_end_bit)
```

**Alternative** (more efficient — copy_state_self method):
```
130: FUNCTION GameState::copy_state_self(self, dest_bit, src_start, src_end)
131:   begin = src_start
132:   target = dest_bit
133:   WHILE begin <= src_end
134:     delta = MIN(7, src_end - begin)
135:     b = self.get_state(begin, begin + delta)    // reads before writes
136:     self.set_state(target, target + delta, b)    // no overlap in practice
137:     begin += delta + 1
138:     target += delta + 1
```

The snapshot approach is safer (handles overlapping ranges). The self-copy
approach is more efficient but only safe when ranges don't overlap.
Since the C caller (`load_legacy.c`) uses non-overlapping ranges in the
transpose table, either approach works. We use the snapshot approach for
correctness.

## D. SeekStateFile Return Value Semantics

The C function returns:
- `1` on success (cursor set to non-negative position)
- `0` when cursor was clamped to 0 (negative seek)

The Rust `seek` currently returns `Ok(())` always. We need to distinguish:

```
141: FUNCTION seek(self, offset: i64, whence: SeekWhence) → (Result, i32)
142:   // ... compute new_pos as above
143:   IF new_pos < 0
144:     self.ptr = 0
145:     RETURN (Ok(()), 0)    // clamped — C returns 0
146:   self.ptr = new_pos as usize
147:   RETURN (Ok(()), 1)      // success — C returns 1
```

The FFI layer maps: `Ok with 1` → `1`, `Ok with 0` → `0`.
