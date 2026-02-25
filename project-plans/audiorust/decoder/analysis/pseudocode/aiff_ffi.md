# AIFF FFI Bridge Pseudocode — `aiff_ffi.rs`

## 1. Module-Level State

```
 1: STATIC RUST_AIFA_FORMATS: Mutex<Option<DecoderFormats>> = Mutex::new(None)
 2: STATIC RUST_AIFA_NAME: &[u8] = b"Rust AIFF\0"
```

## 2. FFI Wrapper Struct

```
 3: STRUCT TFB_RustAiffDecoder [repr(C)]:
 4:   base: TFB_SoundDecoder          // must be first field
 5:   rust_decoder: *mut c_void       // points to Box<AiffDecoder>
```

## 3. UIO File Reading Helper

```
 6: UNSAFE FUNCTION read_uio_file(dir, path) -> Option<Vec<u8>>
 7:   LET handle = uio_open(dir, path, 0, 0)
 8:   IF handle is null:
 9:     RETURN None
10:
11:   LET stat_buf = zeroed libc::stat
12:   IF uio_fstat(handle, &mut stat_buf) != 0:
13:     CALL uio_close(handle)
14:     RETURN None
15:   LET size = stat_buf.st_size as usize
16:
17:   LET data = vec![0u8; size]
18:   LET total = 0
19:   WHILE total < size:
20:     LET n = uio_read(handle, data.ptr + total, size - total)
21:     IF n <= 0:
22:       BREAK
23:     SET total += n as usize
24:
25:   CALL uio_close(handle)
26:
27:   IF total == 0:
28:     RETURN None
29:   TRUNCATE data to total
30:   RETURN Some(data)
```

## 4. GetName

```
31: EXTERN "C" FUNCTION rust_aifa_GetName() -> *const c_char
32:   RETURN RUST_AIFA_NAME.as_ptr() as *const c_char
```

## 5. InitModule

```
33: EXTERN "C" FUNCTION rust_aifa_InitModule(flags, fmts) -> c_int
34:   IF fmts is null:
35:     RETURN 0
36:   UNSAFE:
37:     LET formats = DecoderFormats from (*fmts) fields
38:     LOCK RUST_AIFA_FORMATS:
39:       SET guard = Some(formats)
40:   LET _ = flags                     // ignored (REQ-LF-3)
41:   RETURN 1
```

## 6. TermModule

```
42: EXTERN "C" FUNCTION rust_aifa_TermModule()
43:   LOCK RUST_AIFA_FORMATS:
44:     SET guard = None
```

## 7. GetStructSize

```
45: EXTERN "C" FUNCTION rust_aifa_GetStructSize() -> u32
46:   RETURN size_of::<TFB_RustAiffDecoder>() as u32
```

## 8. GetError

```
47: EXTERN "C" FUNCTION rust_aifa_GetError(decoder) -> c_int
48:   IF decoder is null:
49:     RETURN -1
50:   UNSAFE:
51:     LET rd = decoder as *mut TFB_RustAiffDecoder
52:     IF (*rd).rust_decoder is null:
53:       RETURN -1
54:     LET dec = &mut *((*rd).rust_decoder as *mut AiffDecoder)
55:     RETURN dec.get_error()
```

## 9. Init (Allocation + Format Propagation)

```
56: EXTERN "C" FUNCTION rust_aifa_Init(decoder) -> c_int
57:   IF decoder is null:
58:     RETURN 0
59:   UNSAFE:
60:     LET rd = decoder as *mut TFB_RustAiffDecoder
61:     LET mut dec = Box::new(AiffDecoder::new())
62:
63:     // Propagate formats from global Mutex to instance (REQ-FF-4)
64:     // This matches the wav_ffi.rs Init pattern (lines 138-147).
65:     // Without this, open_from_bytes() would panic on self.formats.unwrap().
66:     LOCK RUST_AIFA_FORMATS:
67:       IF let Some(formats) = guard.as_ref():
68:         CALL dec.init_module(0, formats)
69:     CALL dec.init()
70:
71:     SET (*rd).rust_decoder = Box::into_raw(dec) as *mut c_void
72:     SET (*decoder).need_swap = false
73:   RETURN 1
```

## 10. Term (Deallocation)

```
69: EXTERN "C" FUNCTION rust_aifa_Term(decoder)
70:   IF decoder is null:
71:     RETURN
72:   UNSAFE:
73:     LET rd = decoder as *mut TFB_RustAiffDecoder
74:     IF (*rd).rust_decoder is NOT null:
75:       // Reconstruct Box and drop (REQ-FF-5)
76:       LET dec = Box::from_raw((*rd).rust_decoder as *mut AiffDecoder)
77:       DROP dec
78:       SET (*rd).rust_decoder = null_mut()
```

## 11. Open

```
79: EXTERN "C" FUNCTION rust_aifa_Open(decoder, dir, filename) -> c_int
80:   IF decoder is null OR filename is null:
81:     RETURN 0
82:   UNSAFE:
83:     LET filename_str = CStr::from_ptr(filename).to_str()
84:     IF filename_str is Err:
85:       RETURN 0
86:
87:     LOG "RUST_AIFF_OPEN: {filename_str}"
88:
89:     LET rd = decoder as *mut TFB_RustAiffDecoder
90:     IF (*rd).rust_decoder is null:
91:       RETURN 0
92:
 93:     LET dec = &mut *((*rd).rust_decoder as *mut AiffDecoder)
 94:
 95:     // NOTE: init_module()/init() are called in rust_aifa_Init(), not here.
 96:     // Formats are already propagated to the decoder instance.
 97:
 98:     // Read file via UIO (REQ-FF-6)
100:    LET file_data = read_uio_file(dir, filename)
101:    IF file_data is None:
102:      LOG "RUST_AIFF_OPEN: failed to read {filename_str}"
103:      RETURN 0
104:
105:    // Open from bytes
106:    MATCH dec.open_from_bytes(&file_data, filename_str):
107:      Ok(()) =>
108:        // Update base struct (REQ-FF-7)
109:        SET (*decoder).frequency = dec.frequency()
110:
111:        LOCK RUST_AIFA_FORMATS:
112:          LET formats = MATCH guard.as_ref():
113:            Some(f) => f
114:            None =>
115:              LOG "RUST_AIFF_OPEN: formats not initialized (InitModule not called)"
116:              RETURN 0
117:          LET format_code = MATCH dec.format():
118:            AudioFormat::Mono8   => formats.mono8
119:            AudioFormat::Stereo8 => formats.stereo8
120:            AudioFormat::Mono16  => formats.mono16
121:            AudioFormat::Stereo16 => formats.stereo16
122:          SET (*decoder).format = format_code
123:
124:        SET (*decoder).length = dec.length()
125:        SET (*decoder).is_null = false
126:        SET (*decoder).need_swap = dec.needs_swap()
127:
128:        LOG "RUST_AIFF_OPEN: OK freq={} format={} length={}s"
129:        RETURN 1
130:
131:      Err(e) =>
132:        // (REQ-FF-8)
133:        LOG "RUST_AIFF_OPEN: error: {e}"
134:        RETURN 0
```

## 12. Close

```
135: EXTERN "C" FUNCTION rust_aifa_Close(decoder)
136:   IF decoder is null:
137:     RETURN
138:   UNSAFE:
139:     LET rd = decoder as *mut TFB_RustAiffDecoder
140:     IF (*rd).rust_decoder is NOT null:
141:       LET dec = &mut *((*rd).rust_decoder as *mut AiffDecoder)
142:       CALL dec.close()               // (REQ-FF-15)
```

## 13. Decode

```
143: EXTERN "C" FUNCTION rust_aifa_Decode(decoder, buf, bufsize) -> c_int
144:   IF decoder is null OR buf is null OR bufsize <= 0:
145:     RETURN 0
146:   UNSAFE:
147:     LET rd = decoder as *mut TFB_RustAiffDecoder
148:     IF (*rd).rust_decoder is null:
149:       RETURN 0
150:     LET dec = &mut *((*rd).rust_decoder as *mut AiffDecoder)
151:     LET slice = std::slice::from_raw_parts_mut(buf as *mut u8, bufsize as usize)
152:     // (REQ-FF-9)
153:     MATCH dec.decode(slice):
154:       Ok(n) => RETURN n as c_int
155:       Err(EndOfFile) => RETURN 0
156:       Err(_) => RETURN 0
```

## 14. Seek

```
157: EXTERN "C" FUNCTION rust_aifa_Seek(decoder, pcm_pos) -> u32
158:   IF decoder is null:
159:     RETURN pcm_pos
160:   UNSAFE:
161:     LET rd = decoder as *mut TFB_RustAiffDecoder
162:     IF (*rd).rust_decoder is null:
163:       RETURN pcm_pos
164:     LET dec = &mut *((*rd).rust_decoder as *mut AiffDecoder)
165:     // (REQ-FF-13)
166:     MATCH dec.seek(pcm_pos):
167:       Ok(pos) => RETURN pos
168:       Err(_) => RETURN pcm_pos
```

## 15. GetFrame

```
169: EXTERN "C" FUNCTION rust_aifa_GetFrame(decoder) -> u32
170:   IF decoder is null:
171:     RETURN 0
172:   UNSAFE:
173:     LET rd = decoder as *mut TFB_RustAiffDecoder
174:     IF (*rd).rust_decoder is null:
175:       RETURN 0
176:     LET dec = &*((*rd).rust_decoder as *mut AiffDecoder)
177:     // (REQ-FF-14)
178:     RETURN dec.get_frame()
```

## 16. Vtable Export

```
179: #[no_mangle]
180: STATIC rust_aifa_DecoderVtbl: TFB_SoundDecoderFuncs = TFB_SoundDecoderFuncs {
181:   GetName:        rust_aifa_GetName,
182:   InitModule:     rust_aifa_InitModule,
183:   TermModule:     rust_aifa_TermModule,
184:   GetStructSize:  rust_aifa_GetStructSize,
185:   GetError:       rust_aifa_GetError,
186:   Init:           rust_aifa_Init,
187:   Term:           rust_aifa_Term,
188:   Open:           rust_aifa_Open,
189:   Close:          rust_aifa_Close,
190:   Decode:         rust_aifa_Decode,
191:   Seek:           rust_aifa_Seek,
192:   GetFrame:       rust_aifa_GetFrame,
193: }
```
