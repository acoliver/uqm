# AIFF Decoder Pseudocode — `aiff.rs`

## 1. Constructor

```
 1: FUNCTION AiffDecoder::new()
 2:   SET frequency = 0
 3:   SET format = AudioFormat::Stereo16
 4:   SET length = 0.0
 5:   SET need_swap = false
 6:   SET last_error = 0
 7:   SET formats = None
 8:   SET initialized = false
 9:   SET common = CommonChunk::default()
10:   SET comp_type = CompressionType::None
11:   SET bits_per_sample = 0
12:   SET block_align = 0
13:   SET file_block = 0
14:   SET data = empty Vec
15:   SET data_pos = 0
16:   SET max_pcm = 0
17:   SET cur_pcm = 0
18:   SET prev_val = [0; MAX_CHANNELS]
19:   RETURN self
```

## 2. Byte Reading Helpers

```
20: FUNCTION read_be_u16(cursor)
21:   READ 2 bytes from cursor into buf
22:   IF read fails RETURN Err(InvalidData("read u16"))
23:   RETURN Ok(u16::from_be_bytes(buf))

24: FUNCTION read_be_u32(cursor)
25:   READ 4 bytes from cursor into buf
26:   IF read fails RETURN Err(InvalidData("read u32"))
27:   RETURN Ok(u32::from_be_bytes(buf))

28: FUNCTION read_be_i16(cursor)
29:   READ 2 bytes from cursor into buf
30:   IF read fails RETURN Err(InvalidData("read i16"))
31:   RETURN Ok(i16::from_be_bytes(buf))
```

## 3. IEEE 754 80-bit Extended Precision Float Parsing

The 80-bit extended precision format (used in AIFF COMM chunks for sample rate) is:
- Bit 79: sign (1 bit)
- Bits 78–64: biased exponent (15 bits, bias = 16383)
- Bits 63–0: significand (64 bits, **including explicit integer bit** at bit 63)

Unlike IEEE 754 32/64-bit formats, the integer bit is **not** implicit — it is stored
explicitly. For normalized numbers, bit 63 must be 1.

The mathematical value of a normal extended-precision number is:
`value = (-1)^sign × significand × 2^(biased_exponent − 16383 − 63)`

The `−63` accounts for the 63 fractional bits (the significand is an integer
representation of a 1.63 fixed-point value scaled by 2^63).

For sample rate decoding, we convert to `u32` via truncation (matching C integer cast).

```
32: FUNCTION read_be_f80(cursor) -> Result<i32>
33:   // Read all 10 bytes: 2 (sign+exp) + 4 (sig high) + 4 (sig low)
34:   READ se = read_be_u16(cursor)?                     // sign (1 bit) + exponent (15 bits)
35:   READ sig_hi = read_be_u32(cursor)?                  // significand bits 63..32
36:   READ sig_lo = read_be_u32(cursor)?                  // significand bits 31..0
37:
38:   LET sign = (se >> 15) & 1
39:   LET biased_exp = se & 0x7FFF
40:   LET significand: u64 = ((sig_hi as u64) << 32) | (sig_lo as u64)
41:
42:   // --- Class 1: Zero (exp=0, significand=0) ---
43:   IF biased_exp == 0 AND significand == 0:
44:     RETURN Ok(0)
45:
46:   // --- Class 2: Denormalized (exp=0, significand!=0) ---
47:   // Denormals represent values < 2^-16382; far below any valid sample rate.
48:   // Design choice: return 0 (not InvalidData), because the value is
49:   // mathematically near-zero and will be rejected by the sample rate
50:   // validation (min 300 Hz) that follows. This avoids a confusing "invalid
51:   // f80" error message for what is really just "sample rate too low".
52:   IF biased_exp == 0 AND significand != 0:
53:     RETURN Ok(0)
54:
55:   // --- Class 3: Infinity / NaN (exp=0x7FFF) ---
56:   // Sample rates cannot be infinity or NaN.
57:   IF biased_exp == 0x7FFF:
58:     RETURN Err(InvalidData("invalid sample rate: infinity or NaN in f80"))
59:
60:   // --- Class 4: Normal ---
61:   // value = (-1)^sign × significand × 2^(biased_exp − 16383 − 63)
62:   //
63:   // We need to convert to an integer. Let shift = biased_exp − 16383 − 63.
64:   // If shift >= 0: value = significand << shift  (could overflow u32)
65:   // If shift < 0:  value = significand >> (-shift)  (truncation = floor)
66:   //
67:   // Example: 44100 Hz
68:   //   biased_exp = 16398, significand = 0xAC44_0000_0000_0000
69:   //   shift = 16398 − 16383 − 63 = −48
70:   //   significand >> 48 = 0xAC44_0000_0000_0000 >> 48 = 0xAC44 = 44100 [OK]
71:
72:   LET shift: i32 = (biased_exp as i32) - 16383 - 63
73:
 74:   LET abs_val: u64
 75:   IF shift >= 0:
 76:     // Left shift: use checked_shl to detect overflow, then clamp to i32::MAX
 77:     LET shifted = significand.checked_shl(shift as u32).unwrap_or(u64::MAX)
 78:     IF shifted > 0x7FFF_FFFF:
 79:       SET abs_val = 0x7FFF_FFFF                 // clamp to i32::MAX (matching C behavior)
 80:     ELSE:
 81:       SET abs_val = shifted
 82:   ELSE:
84:     LET right_shift = (-shift) as u32
85:     IF right_shift >= 64:
86:       SET abs_val = 0                            // shifted away entirely
87:     ELSE:
88:       SET abs_val = significand >> right_shift    // truncation toward zero
89:
90:   LET result = abs_val as i32
91:   IF sign == 1:
92:     SET result = -result
93:   RETURN Ok(result)
```

## 4. Chunk Header Parsing

```
48: FUNCTION read_chunk_header(cursor)
49:   LET id = read_be_u32(cursor)?
50:   LET size = read_be_u32(cursor)?
51:   RETURN Ok(ChunkHeader { id, size })
```

## 5. Common Chunk (COMM) Parsing

```
52: FUNCTION read_common_chunk(cursor, chunk_size)
53:   IF chunk_size < AIFF_COMM_SIZE (18):
54:     SET self.last_error = -2
55:     RETURN Err(InvalidData("COMM chunk too small"))
56:   LET start_pos = cursor.position()
57:   LET mut common = CommonChunk::default()
58:   SET common.channels = read_be_u16(cursor)?
59:   SET common.sample_frames = read_be_u32(cursor)?
60:   SET common.sample_size = read_be_u16(cursor)?
61:   SET common.sample_rate = read_be_f80(cursor)?
62:   IF chunk_size >= AIFF_EXT_COMM_SIZE (22):
63:     SET common.ext_type_id = read_be_u32(cursor)?
64:   LET consumed = cursor.position() - start_pos
65:   LET remaining = chunk_size as u64 - consumed
66:   IF remaining > 0:
67:     SEEK cursor forward by remaining bytes
68:   RETURN Ok(common)
```

## 6. Sound Data Header Parsing

```
69: FUNCTION read_sound_data_header(cursor)
70:   LET offset = read_be_u32(cursor)?
71:   LET block_size = read_be_u32(cursor)?
72:   RETURN Ok(SoundDataHeader { offset, block_size })
```

## 7. open_from_bytes (Main Entry Point)

```
 73: FUNCTION open_from_bytes(data, name)
 74:   // Reset state (REQ-LF-7)
 75:   CALL self.close()
 76:   SET self.common = CommonChunk::default()
 77:
 78:   // Validate minimum size
 79:   IF data.len() < 12:
 80:     SET self.last_error = -2
 81:     RETURN Err(InvalidData("file too small for AIFF header"))
 82:
 83:   LET cursor = Cursor::new(data)
 84:
 85:   // Parse file header (REQ-FP-1)
 86:   LET chunk_id = read_be_u32(&mut cursor)?
 87:   LET chunk_size = read_be_u32(&mut cursor)?
 88:   LET form_type = read_be_u32(&mut cursor)?
 89:
 90:   // Validate FORM (REQ-FP-2)
 91:   IF chunk_id != FORM_ID:
 92:     SET self.last_error = -2
 93:     CALL self.close()
 94:     RETURN Err(InvalidData("not a FORM file"))
 95:
 96:   // Validate form type (REQ-FP-3)
 97:   LET is_aifc = match form_type:
 98:     FORM_TYPE_AIFF => false
 99:     FORM_TYPE_AIFC => true
100:     _ =>
101:       SET self.last_error = -2
102:       CALL self.close()
103:       RETURN Err(InvalidData("unsupported form type"))
104:
105:   // Chunk iteration (REQ-FP-4, REQ-FP-6)
106:   LET remaining = chunk_size as i64 - 4
107:   LET data_ofs: u64 = 0
108:   LET ssnd_found = false
109:
110:   WHILE remaining > 0:
111:     LET chunk_hdr = read_chunk_header(&mut cursor)?
112:     LET consume = 8 + chunk_hdr.size as i64
113:     IF chunk_hdr.size & 1 != 0:     // (REQ-FP-5)
114:       consume += 1                   // alignment padding
115:
116:     // Overflow guard: reject chunks that claim to extend past the file
117:     IF consume > remaining:
118:       SET self.last_error = -2
119:       CALL self.close()
120:       RETURN Err(InvalidData("chunk size exceeds remaining file data"))
121:
122:     MATCH chunk_hdr.id:
123:       COMMON_ID =>
124:         // (REQ-FP-8, REQ-FP-9, REQ-FP-10, REQ-FP-11)
125:         SET self.common = read_common_chunk(&mut cursor, chunk_hdr.size)?
126:
127:       SOUND_DATA_ID =>
128:         // (REQ-FP-12, REQ-FP-13)
129:         LET ssnd_hdr = read_sound_data_header(&mut cursor)?
130:         SET data_ofs = cursor.position() + ssnd_hdr.offset as u64
131:         SET ssnd_found = true
132:         LET skip_bytes = chunk_hdr.size - AIFF_SSND_SIZE as u32
133:         SEEK cursor forward by skip_bytes
134:
135:       _ =>
136:         // (REQ-FP-7)
137:         SEEK cursor forward by chunk_hdr.size
138:
139:     // Alignment padding (REQ-FP-5)
140:     IF chunk_hdr.size & 1 != 0:
141:       SEEK cursor forward by 1
142:
143:     SET remaining -= consume
144:
145:   // Validation phase (REQ-SV-5)
146:   IF self.common.sample_frames == 0:
147:     SET self.last_error = -2
148:     CALL self.close()
149:     RETURN Err(InvalidData("no sound data"))
150:
151:   // (REQ-SV-1)
152:   SET self.bits_per_sample = ((self.common.sample_size + 7) & !7) as u32
153:
154:   // (REQ-SV-2)
155:   IF self.bits_per_sample == 0 OR self.bits_per_sample > 16:
156:     SET self.last_error = -2
157:     CALL self.close()
158:     RETURN Err(UnsupportedFormat("bits_per_sample"))
159:
160:   // (REQ-SV-3)
161:   IF self.common.channels != 1 AND self.common.channels != 2:
162:     SET self.last_error = -2
163:     CALL self.close()
164:     RETURN Err(UnsupportedFormat("channels"))
165:
166:   // (REQ-SV-4)
167:   IF self.common.sample_rate < 300 OR self.common.sample_rate > 128000:
168:     SET self.last_error = -2
169:     CALL self.close()
170:     RETURN Err(UnsupportedFormat("sample_rate"))
171:
172:   // (REQ-SV-6)
173:   IF NOT ssnd_found:
174:     SET self.last_error = -2
175:     CALL self.close()
176:     RETURN Err(InvalidData("no SSND chunk"))
177:
178:   // Compression handling (REQ-CH-1 through REQ-CH-4)
179:   IF NOT is_aifc:
180:     IF self.common.ext_type_id != 0:
181:       CALL self.close()
182:       RETURN Err(UnsupportedFormat("AIFF with extension"))
183:     SET self.comp_type = CompressionType::None
184:   ELSE:
185:     IF self.common.ext_type_id == SDX2_COMPRESSION:
186:       SET self.comp_type = CompressionType::Sdx2
187:     ELSE:
188:       CALL self.close()
189:       RETURN Err(UnsupportedFormat("unknown AIFC compression"))
190:
191:   // SDX2-specific validation (REQ-CH-5, REQ-CH-6)
192:   IF self.comp_type == CompressionType::Sdx2:
193:     IF self.bits_per_sample != 16:
194:       CALL self.close()
195:       RETURN Err(UnsupportedFormat("SDX2 requires 16-bit"))
196:     IF self.common.channels as usize > MAX_CHANNELS:
197:       CALL self.close()
198:       RETURN Err(UnsupportedFormat("SDX2 too many channels"))
199:
200:   // Calculate block sizes (REQ-SV-7, REQ-SV-8, REQ-SV-9)
201:   SET self.block_align = (self.bits_per_sample / 8) * self.common.channels as u32
202:   IF self.comp_type == CompressionType::None:
203:     SET self.file_block = self.block_align
204:   ELSE:
205:     SET self.file_block = self.block_align / 2    // 2:1 SDX2 compression
206:
207:   // Extract audio data (REQ-SV-10)
208:   LET data_start = data_ofs as usize
209:   LET data_size = self.common.sample_frames as usize * self.file_block as usize
210:   IF data_start + data_size > data.len():
211:     CALL self.close()
212:     RETURN Err(InvalidData("audio data extends past file"))
213:   SET self.data = data[data_start..data_start + data_size].to_vec()
214:
215:   // Set metadata (REQ-SV-11, REQ-SV-12, REQ-SV-13)
216:   SET self.format = AudioFormat from (channels, bits_per_sample)
217:   SET self.frequency = self.common.sample_rate as u32
218:   SET self.max_pcm = self.common.sample_frames
219:   SET self.cur_pcm = 0
220:   SET self.data_pos = 0
221:   SET self.length = self.max_pcm as f32 / self.frequency as f32
222:   SET self.last_error = 0
223:
224:   // Set need_swap based on compression type (REQ-LF-5, REQ-CH-7)
225:   // The FFI Init() calls init_module() and init() to propagate formats,
226:   // so self.formats is guaranteed to be Some(...) here.
227:   LET fmts = self.formats.as_ref().ok_or(AiffError::NotInitialized)?
228:   IF self.comp_type == CompressionType::Sdx2:
229:     // SDX2 produces samples in the output byte order indicated by formats.big_endian
230:     SET self.need_swap = fmts.big_endian != fmts.want_big_endian    // (REQ-CH-7)
231:   ELSE:
232:     // PCM: AIFF stores big-endian; swap if mixer wants little-endian
233:     SET self.need_swap = !fmts.want_big_endian                      // (REQ-LF-5)
234:
235:   // Predictor initialization (REQ-DS-7)
236:   SET self.prev_val = [0; MAX_CHANNELS]
237:
238:   RETURN Ok(())
```

## 8. PCM Decode

### PCM 16-bit Endianness Contract

AIFF files store 16-bit PCM samples in **big-endian** byte order (network order).
The Rust PCM decoder does **NOT** perform inline byte swapping.

**Why no inline swap:**
The C framework's `SoundDecoder_Decode()` in `decoder.c` (lines 556-562) already
performs byte swapping when `decoder->need_swap == true` for 16-bit formats. The C
AIFF decoder (`aifa_DecodePCM`) also does NOT perform inline swapping — it just
copies raw big-endian bytes. If the Rust decoder swapped inline AND the C mixer
also swapped (via the base struct's `need_swap` field), the data would be swapped
twice, producing garbled audio.

**How `need_swap` is determined:**
- In `init()`: `self.need_swap = !self.formats.as_ref().ok_or(...)?.want_big_endian`
- This value is propagated to `(*decoder).need_swap` by the FFI Open function
- The C mixer reads this field and swaps if needed — the Rust decoder just copies raw bytes

**What the decoder does:**
- Copy raw big-endian PCM bytes from `self.data` to `buf`
- 8-bit samples: apply `wrapping_add(128)` signed→unsigned conversion
- 16-bit samples: no transformation — bytes stay in file (big-endian) order
- The C framework handles any needed byte swap via `SoundDecoder_SwapWords()`

```
239: FUNCTION decode_pcm(buf)
240:   // (REQ-DP-6)
241:   IF self.cur_pcm >= self.max_pcm:
242:     RETURN Err(EndOfFile)
243:
244:   // (REQ-DP-1)
245:   LET dec_pcm = min(buf.len() as u32 / self.block_align, self.max_pcm - self.cur_pcm)
246:   LET read_bytes = dec_pcm as usize * self.file_block as usize
247:   LET write_bytes = dec_pcm as usize * self.block_align as usize
248:
249:   // (REQ-DP-2) Copy raw big-endian PCM data from file buffer to output
250:   // Do NOT perform inline byte swap — the C framework handles it via need_swap
251:   COPY self.data[self.data_pos .. self.data_pos + read_bytes] -> buf[..write_bytes]
252:
253:   // 8-bit conversion (REQ-DP-5)
254:   IF self.bits_per_sample == 8:
255:     FOR each byte in buf[..write_bytes]:
256:       SET byte = byte.wrapping_add(128)
257:
258:   // NOTE: No 16-bit endian swap here. The C framework's SoundDecoder_Decode()
259:   // reads (*decoder).need_swap and calls SoundDecoder_SwapWords() for 16-bit
260:   // formats. Matching the C AIFF decoder (aifa_DecodePCM) which also does not swap.
261:
262:   // (REQ-DP-3)
263:   SET self.cur_pcm += dec_pcm
264:   SET self.data_pos += read_bytes
265:
266:   // (REQ-DP-4)
267:   RETURN Ok(write_bytes)
```

## 9. SDX2 Decode

```
270: FUNCTION decode_sdx2(buf)
271:   // (REQ-DS-8)
272:   IF self.cur_pcm >= self.max_pcm:
273:     RETURN Err(EndOfFile)
274:
275:   // (REQ-DS-1)
276:   LET dec_pcm = min(buf.len() as u32 / self.block_align, self.max_pcm - self.cur_pcm)
277:   LET compressed_bytes = dec_pcm as usize * self.file_block as usize
278:   LET channels = self.common.channels as usize
279:
280:   // (REQ-DS-2)
281:   LET compressed = &self.data[self.data_pos .. self.data_pos + compressed_bytes]
282:
283:   LET out_pos = 0
284:
285:   // (REQ-DS-4, REQ-DS-5)
286:   FOR frame_idx IN 0..dec_pcm:
287:     FOR ch IN 0..channels:
288:       LET byte_idx = frame_idx as usize * channels + ch
289:       LET sample_byte = compressed[byte_idx] as i8
290:       LET sample = sample_byte as i32
291:       LET abs_val = sample.abs()
292:       LET v = (sample * abs_val) << 1
293:
294:       IF (sample_byte as u8) & 1 != 0:
295:         SET v += self.prev_val[ch]          // delta mode
296:
297:       SET v = v.clamp(-32768, 32767)        // saturate to i16
298:       SET self.prev_val[ch] = v
299:
300:       // Write i16 to output buffer with correct endianness
301:       LET sample_i16 = v as i16
302:       LET bytes = IF self.need_swap:
303:         sample_i16.swap_bytes().to_ne_bytes()   // swap from native to opposite endianness
304:       ELSE:
305:         sample_i16.to_ne_bytes()                // native byte order (no swap)
306:       WRITE bytes to buf[out_pos .. out_pos + 2]
307:       SET out_pos += 2
308:
309:   // (REQ-DS-3)
310:   SET self.cur_pcm += dec_pcm
311:   SET self.data_pos += compressed_bytes
312:
313:   // (REQ-DS-6)
314:   LET write_bytes = dec_pcm as usize * self.block_align as usize
315:   RETURN Ok(write_bytes)
```

## 10. Decode Dispatch

```
316: FUNCTION decode(buf)
317:   MATCH self.comp_type:
318:     CompressionType::None => RETURN self.decode_pcm(buf)
319:     CompressionType::Sdx2 => RETURN self.decode_sdx2(buf)
```

## 11. Seek

```
320: FUNCTION seek(pcm_pos)
321:   // (REQ-SK-1)
322:   LET pcm_pos = pcm_pos.min(self.max_pcm)
323:
324:   // (REQ-SK-2)
325:   SET self.cur_pcm = pcm_pos
326:   SET self.data_pos = pcm_pos as usize * self.file_block as usize
327:
328:   // (REQ-SK-3)
329:   SET self.prev_val = [0i32; MAX_CHANNELS]
330:
331:   // (REQ-SK-4)
332:   RETURN Ok(pcm_pos)
```

## 12. Close

```
333: FUNCTION close()
334:   SET self.data = empty Vec (clear and deallocate)
335:   SET self.data_pos = 0
336:   SET self.cur_pcm = 0
337:   SET self.max_pcm = 0
338:   SET self.prev_val = [0; MAX_CHANNELS]
```

## 13. Trait Method Implementations

```
339: FUNCTION name() -> &'static str
340:   RETURN "AIFF"

341: FUNCTION init_module(flags, formats)
342:   LET _ = flags                    // (REQ-LF-3)
343:   SET self.formats = Some(*formats) // (REQ-LF-2)
344:   RETURN true

345: FUNCTION term_module()
346:   SET self.formats = None           // (REQ-LF-4)

347: FUNCTION get_error() -> i32
348:   LET err = self.last_error         // (REQ-EH-1)
349:   SET self.last_error = 0
350:   RETURN err

351: FUNCTION init() -> bool
352:   // (REQ-LF-5) — Note: open_from_bytes() also sets need_swap unconditionally,
353:   // so init() is not strictly required for correctness. This method exists
354:   // for SoundDecoder trait compliance and direct (non-FFI) usage.
355:   LET fmts = MATCH self.formats.as_ref():
356:     Some(f) => f
357:     None => RETURN false             // not initialized — InitModule not called
358:   SET self.need_swap = !fmts.want_big_endian
359:   SET self.initialized = true
360:   RETURN true

361: FUNCTION term()
362:   CALL self.close()                 // (REQ-EH-5)

363: FUNCTION open(path)
364:   LET data = std::fs::read(path)?
365:   LET name = path.to_string_lossy()
366:   RETURN self.open_from_bytes(&data, &name)

367: FUNCTION get_frame() -> u32
368:   RETURN 0                          // (REQ-LF-6)

369: FUNCTION frequency() -> u32
370:   RETURN self.frequency

371: FUNCTION format() -> AudioFormat
372:   RETURN self.format

373: FUNCTION length() -> f32
374:   RETURN self.length

375: FUNCTION is_null() -> bool
376:   RETURN false                      // (REQ-LF-9)

377: FUNCTION needs_swap() -> bool
378:   RETURN self.need_swap             // (REQ-LF-10)
```
