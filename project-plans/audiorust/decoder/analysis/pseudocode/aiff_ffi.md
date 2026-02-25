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

## 9. Init (Allocation)

```
56: EXTERN "C" FUNCTION rust_aifa_Init(decoder) -> c_int
57:   IF decoder is null:
58:     RETURN 0
59:   UNSAFE:
60:     LET rd = decoder as *mut TFB_RustAiffDecoder
61:     LET dec = Box::new(AiffDecoder::new())
62:     // Store raw pointer (REQ-FF-4)
63:     SET (*rd).rust_decoder = Box::into_raw(dec) as *mut c_void
64:     SET (*decoder).need_swap = false
65:   RETURN 1
```

## 10. Term (Deallocation)

```
66: EXTERN "C" FUNCTION rust_aifa_Term(decoder)
67:   IF decoder is null:
68:     RETURN
69:   UNSAFE:
70:     LET rd = decoder as *mut TFB_RustAiffDecoder
71:     IF (*rd).rust_decoder is NOT null:
72:       // Reconstruct Box and drop (REQ-FF-5)
73:       LET dec = Box::from_raw((*rd).rust_decoder as *mut AiffDecoder)
74:       DROP dec
75:       SET (*rd).rust_decoder = null_mut()
```

## 11. Open

```
76: EXTERN "C" FUNCTION rust_aifa_Open(decoder, dir, filename) -> c_int
77:   IF decoder is null OR filename is null:
78:     RETURN 0
79:   UNSAFE:
80:     LET filename_str = CStr::from_ptr(filename).to_str()
81:     IF filename_str is Err:
82:       RETURN 0
83:
84:     LOG "RUST_AIFF_OPEN: {filename_str}"
85:
86:     LET rd = decoder as *mut TFB_RustAiffDecoder
87:     IF (*rd).rust_decoder is null:
88:       RETURN 0
89:
90:     LET dec = &mut *((*rd).rust_decoder as *mut AiffDecoder)
91:
92:     // NOTE: Do NOT call init_module()/init() here — those are separate
93:     // vtable calls made by the C framework (matching dukaud_ffi.rs pattern).
94:
95:     // Read file via UIO (REQ-FF-6)
96:     LET file_data = read_uio_file(dir, filename)
97:     IF file_data is None:
98:       LOG "RUST_AIFF_OPEN: failed to read {filename_str}"
99:       RETURN 0
100:
101:    // Open from bytes
102:    MATCH dec.open_from_bytes(&file_data, filename_str):
103:      Ok(()) =>
104:        // Update base struct (REQ-FF-7)
105:        SET (*decoder).frequency = dec.frequency()
106:
107:        LOCK RUST_AIFA_FORMATS:
108:          LET format_code = MATCH dec.format():
109:            AudioFormat::Mono8   => formats.mono8
110:            AudioFormat::Stereo8 => formats.stereo8
111:            AudioFormat::Mono16  => formats.mono16
112:            AudioFormat::Stereo16 => formats.stereo16
113:          SET (*decoder).format = format_code
114:
115:        SET (*decoder).length = dec.length()
116:        SET (*decoder).is_null = false
117:        SET (*decoder).need_swap = dec.needs_swap()
118:
119:        LOG "RUST_AIFF_OPEN: OK freq={} format={} length={}s"
120:        RETURN 1
121:
122:      Err(e) =>
123:        // (REQ-FF-8)
124:        LOG "RUST_AIFF_OPEN: error: {e}"
125:        RETURN 0
```

## 12. Close

```
129: EXTERN "C" FUNCTION rust_aifa_Close(decoder)
130:   IF decoder is null:
131:     RETURN
132:   UNSAFE:
133:     LET rd = decoder as *mut TFB_RustAiffDecoder
134:     IF (*rd).rust_decoder is NOT null:
135:       LET dec = &mut *((*rd).rust_decoder as *mut AiffDecoder)
136:       CALL dec.close()               // (REQ-FF-15)
```

## 13. Decode

```
137: EXTERN "C" FUNCTION rust_aifa_Decode(decoder, buf, bufsize) -> c_int
138:   IF decoder is null OR buf is null OR bufsize <= 0:
139:     RETURN 0
140:   UNSAFE:
141:     LET rd = decoder as *mut TFB_RustAiffDecoder
142:     IF (*rd).rust_decoder is null:
143:       RETURN 0
144:     LET dec = &mut *((*rd).rust_decoder as *mut AiffDecoder)
145:     LET slice = std::slice::from_raw_parts_mut(buf as *mut u8, bufsize as usize)
146:     // (REQ-FF-9)
147:     MATCH dec.decode(slice):
148:       Ok(n) => RETURN n as c_int
149:       Err(EndOfFile) => RETURN 0
150:       Err(_) => RETURN 0
```

## 14. Seek

```
151: EXTERN "C" FUNCTION rust_aifa_Seek(decoder, pcm_pos) -> u32
152:   IF decoder is null:
153:     RETURN pcm_pos
154:   UNSAFE:
155:     LET rd = decoder as *mut TFB_RustAiffDecoder
156:     IF (*rd).rust_decoder is null:
157:       RETURN pcm_pos
158:     LET dec = &mut *((*rd).rust_decoder as *mut AiffDecoder)
159:     // (REQ-FF-13)
160:     MATCH dec.seek(pcm_pos):
161:       Ok(pos) => RETURN pos
162:       Err(_) => RETURN pcm_pos
```

## 15. GetFrame

```
163: EXTERN "C" FUNCTION rust_aifa_GetFrame(decoder) -> u32
164:   IF decoder is null:
165:     RETURN 0
166:   UNSAFE:
167:     LET rd = decoder as *mut TFB_RustAiffDecoder
168:     IF (*rd).rust_decoder is null:
169:       RETURN 0
170:     LET dec = &*((*rd).rust_decoder as *mut AiffDecoder)
171:     // (REQ-FF-14)
172:     RETURN dec.get_frame()
```

## 16. Vtable Export

```
173: #[no_mangle]
174: STATIC rust_aifa_DecoderVtbl: TFB_SoundDecoderFuncs = TFB_SoundDecoderFuncs {
175:   GetName:        rust_aifa_GetName,
176:   InitModule:     rust_aifa_InitModule,
177:   TermModule:     rust_aifa_TermModule,
178:   GetStructSize:  rust_aifa_GetStructSize,
179:   GetError:       rust_aifa_GetError,
180:   Init:           rust_aifa_Init,
181:   Term:           rust_aifa_Term,
182:   Open:           rust_aifa_Open,
183:   Close:          rust_aifa_Close,
184:   Decode:         rust_aifa_Decode,
185:   Seek:           rust_aifa_Seek,
186:   GetFrame:       rust_aifa_GetFrame,
187: }
```
