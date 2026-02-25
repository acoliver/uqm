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

## 3. IEEE 754 80-bit Float Parsing

```
32: FUNCTION read_be_f80(cursor)
33:   READ se = read_be_u16(cursor)?            // sign + exponent
34:   READ mantissa_hi = read_be_u32(cursor)?    // high 32 bits
35:   READ _mantissa_lo = read_be_u32(cursor)?   // low 32 bits (discarded)
36:   LET sign = (se >> 15) & 1
37:   LET biased_exponent = (se & 0x7FFF)
38:
39:   // Edge case: Denormalized numbers (biased exponent == 0)
40:   // These represent very small values (< 2^-16382); for sample rates this is
41:   // effectively zero. Real AIFF files never use denormalized sample rates.
42:   IF biased_exponent == 0:
43:     RETURN Ok(0)
44:
45:   // Edge case: Infinity and NaN (biased exponent == 0x7FFF)
46:   // Sample rates cannot be infinity or NaN. Return InvalidData.
47:   IF biased_exponent == 0x7FFF:
48:     RETURN Err(InvalidData("invalid sample rate: infinity or NaN in f80"))
49:
50:   // Normal case
51:   LET exponent = biased_exponent as i32
52:   LET mantissa = (mantissa_hi >> 1) as i32   // shift to fit in signed 31 bits
53:   SET exponent = exponent - 16383             // unbias (2^14 - 1)
54:   LET shift = exponent - 31 + 1
55:   IF shift > 0:
56:     SET mantissa = 0x7FFF_FFFF               // overflow clamp
57:   ELSE IF shift < 0:
58:     SET mantissa = mantissa >> (-shift)       // arithmetic right shift
59:   IF sign == 1:
60:     SET mantissa = -mantissa
61:   RETURN Ok(mantissa)
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
116:     MATCH chunk_hdr.id:
117:       COMMON_ID =>
118:         // (REQ-FP-8, REQ-FP-9, REQ-FP-10, REQ-FP-11)
119:         SET self.common = read_common_chunk(&mut cursor, chunk_hdr.size)?
120:
121:       SOUND_DATA_ID =>
122:         // (REQ-FP-12, REQ-FP-13)
123:         LET ssnd_hdr = read_sound_data_header(&mut cursor)?
124:         SET data_ofs = cursor.position() + ssnd_hdr.offset as u64
125:         SET ssnd_found = true
126:         LET skip_bytes = chunk_hdr.size - AIFF_SSND_SIZE as u32
127:         SEEK cursor forward by skip_bytes
128:
129:       _ =>
130:         // (REQ-FP-7)
131:         SEEK cursor forward by chunk_hdr.size
132:
133:     // Alignment padding (REQ-FP-5)
134:     IF chunk_hdr.size & 1 != 0:
135:       SEEK cursor forward by 1
136:
137:     SET remaining -= consume
138:
139:   // Validation phase (REQ-SV-5)
140:   IF self.common.sample_frames == 0:
141:     SET self.last_error = -2
142:     CALL self.close()
143:     RETURN Err(InvalidData("no sound data"))
144:
145:   // (REQ-SV-1)
146:   SET self.bits_per_sample = ((self.common.sample_size + 7) & !7) as u32
147:
148:   // (REQ-SV-2)
149:   IF self.bits_per_sample == 0 OR self.bits_per_sample > 16:
150:     SET self.last_error = -2
151:     CALL self.close()
152:     RETURN Err(UnsupportedFormat("bits_per_sample"))
153:
154:   // (REQ-SV-3)
155:   IF self.common.channels != 1 AND self.common.channels != 2:
156:     SET self.last_error = -2
157:     CALL self.close()
158:     RETURN Err(UnsupportedFormat("channels"))
159:
160:   // (REQ-SV-4)
161:   IF self.common.sample_rate < 300 OR self.common.sample_rate > 128000:
162:     SET self.last_error = -2
163:     CALL self.close()
164:     RETURN Err(UnsupportedFormat("sample_rate"))
165:
166:   // (REQ-SV-6)
167:   IF NOT ssnd_found:
168:     SET self.last_error = -2
169:     CALL self.close()
170:     RETURN Err(InvalidData("no SSND chunk"))
171:
172:   // Compression handling (REQ-CH-1 through REQ-CH-4)
173:   IF NOT is_aifc:
174:     IF self.common.ext_type_id != 0:
175:       CALL self.close()
176:       RETURN Err(UnsupportedFormat("AIFF with extension"))
177:     SET self.comp_type = CompressionType::None
178:   ELSE:
179:     IF self.common.ext_type_id == SDX2_COMPRESSION:
180:       SET self.comp_type = CompressionType::Sdx2
181:     ELSE:
182:       CALL self.close()
183:       RETURN Err(UnsupportedFormat("unknown AIFC compression"))
184:
185:   // SDX2-specific validation (REQ-CH-5, REQ-CH-6)
186:   IF self.comp_type == CompressionType::Sdx2:
187:     IF self.bits_per_sample != 16:
188:       CALL self.close()
189:       RETURN Err(UnsupportedFormat("SDX2 requires 16-bit"))
190:     IF self.common.channels as usize > MAX_CHANNELS:
191:       CALL self.close()
192:       RETURN Err(UnsupportedFormat("SDX2 too many channels"))
193:
194:   // Calculate block sizes (REQ-SV-7, REQ-SV-8, REQ-SV-9)
195:   SET self.block_align = (self.bits_per_sample / 8) * self.common.channels as u32
196:   IF self.comp_type == CompressionType::None:
197:     SET self.file_block = self.block_align
198:   ELSE:
199:     SET self.file_block = self.block_align / 2    // 2:1 SDX2 compression
200:
201:   // Extract audio data (REQ-SV-10)
202:   LET data_start = data_ofs as usize
203:   LET data_size = self.common.sample_frames as usize * self.file_block as usize
204:   IF data_start + data_size > data.len():
205:     CALL self.close()
206:     RETURN Err(InvalidData("audio data extends past file"))
207:   SET self.data = data[data_start..data_start + data_size].to_vec()
208:
209:   // Set metadata (REQ-SV-11, REQ-SV-12, REQ-SV-13)
210:   SET self.format = AudioFormat from (channels, bits_per_sample)
211:   SET self.frequency = self.common.sample_rate as u32
212:   SET self.max_pcm = self.common.sample_frames
213:   SET self.cur_pcm = 0
214:   SET self.data_pos = 0
215:   SET self.length = self.max_pcm as f32 / self.frequency as f32
216:   SET self.last_error = 0
217:
218:   // SDX2 endianness override (REQ-CH-7)
219:   IF self.comp_type == CompressionType::Sdx2:
220:     SET self.need_swap = cfg!(target_endian = "big") != self.formats.unwrap().want_big_endian
221:
222:   // Predictor initialization (REQ-DS-7)
223:   SET self.prev_val = [0; MAX_CHANNELS]
224:
225:   RETURN Ok(())
```

## 8. PCM Decode

```
226: FUNCTION decode_pcm(buf)
227:   // (REQ-DP-6)
228:   IF self.cur_pcm >= self.max_pcm:
229:     RETURN Err(EndOfFile)
230:
231:   // (REQ-DP-1)
232:   LET dec_pcm = min(buf.len() as u32 / self.block_align, self.max_pcm - self.cur_pcm)
233:   LET read_bytes = dec_pcm as usize * self.file_block as usize
234:   LET write_bytes = dec_pcm as usize * self.block_align as usize
235:
236:   // (REQ-DP-2)
237:   COPY self.data[self.data_pos .. self.data_pos + read_bytes] → buf[..write_bytes]
238:
239:   // 8-bit conversion (REQ-DP-5)
240:   IF self.bits_per_sample == 8:
241:     FOR each byte in buf[..write_bytes]:
242:       SET byte = byte.wrapping_add(128)
243:
244:   // (REQ-DP-3)
245:   SET self.cur_pcm += dec_pcm
246:   SET self.data_pos += read_bytes
247:
248:   // (REQ-DP-4)
249:   RETURN Ok(write_bytes)
```

## 9. SDX2 Decode

```
250: FUNCTION decode_sdx2(buf)
251:   // (REQ-DS-8)
252:   IF self.cur_pcm >= self.max_pcm:
253:     RETURN Err(EndOfFile)
254:
255:   // (REQ-DS-1)
256:   LET dec_pcm = min(buf.len() as u32 / self.block_align, self.max_pcm - self.cur_pcm)
257:   LET compressed_bytes = dec_pcm as usize * self.file_block as usize
258:   LET channels = self.common.channels as usize
259:
260:   // (REQ-DS-2)
261:   LET compressed = &self.data[self.data_pos .. self.data_pos + compressed_bytes]
262:
263:   LET out_pos = 0
264:
265:   // (REQ-DS-4, REQ-DS-5)
266:   FOR frame_idx IN 0..dec_pcm:
267:     FOR ch IN 0..channels:
268:       LET byte_idx = frame_idx as usize * channels + ch
269:       LET sample_byte = compressed[byte_idx] as i8
270:       LET sample = sample_byte as i32
271:       LET abs_val = sample.abs()
272:       LET v = (sample * abs_val) << 1
273:
274:       IF (sample_byte as u8) & 1 != 0:
275:         SET v += self.prev_val[ch]          // delta mode
276:
277:       SET v = v.clamp(-32768, 32767)        // saturate to i16
278:       SET self.prev_val[ch] = v
279:
280:       // Write i16 to output buffer
281:       LET sample_i16 = v as i16
282:       LET bytes = IF self.need_swap:
283:         sample_i16.to_be_bytes()  // or swap
284:       ELSE:
285:         sample_i16.to_ne_bytes()
286:       WRITE bytes to buf[out_pos .. out_pos + 2]
287:       SET out_pos += 2
288:
289:   // (REQ-DS-3)
290:   SET self.cur_pcm += dec_pcm
291:   SET self.data_pos += compressed_bytes
292:
293:   // (REQ-DS-6)
294:   LET write_bytes = dec_pcm as usize * self.block_align as usize
295:   RETURN Ok(write_bytes)
```

## 10. Decode Dispatch

```
296: FUNCTION decode(buf)
297:   MATCH self.comp_type:
298:     CompressionType::None => RETURN self.decode_pcm(buf)
299:     CompressionType::Sdx2 => RETURN self.decode_sdx2(buf)
```

## 11. Seek

```
300: FUNCTION seek(pcm_pos)
301:   // (REQ-SK-1)
302:   LET pcm_pos = pcm_pos.min(self.max_pcm)
303:
304:   // (REQ-SK-2)
305:   SET self.cur_pcm = pcm_pos
306:   SET self.data_pos = pcm_pos as usize * self.file_block as usize
307:
308:   // (REQ-SK-3)
309:   SET self.prev_val = [0i32; MAX_CHANNELS]
310:
311:   // (REQ-SK-4)
312:   RETURN Ok(pcm_pos)
```

## 12. Close

```
313: FUNCTION close()
314:   SET self.data = empty Vec (clear and deallocate)
315:   SET self.data_pos = 0
316:   SET self.cur_pcm = 0
317:   SET self.max_pcm = 0
318:   SET self.prev_val = [0; MAX_CHANNELS]
```

## 13. Trait Method Implementations

```
319: FUNCTION name() -> &'static str
320:   RETURN "AIFF"

321: FUNCTION init_module(flags, formats)
322:   LET _ = flags                    // (REQ-LF-3)
323:   SET self.formats = Some(*formats) // (REQ-LF-2)
324:   RETURN true

325: FUNCTION term_module()
326:   SET self.formats = None           // (REQ-LF-4)

327: FUNCTION get_error() -> i32
328:   LET err = self.last_error         // (REQ-EH-1)
329:   SET self.last_error = 0
330:   RETURN err

331: FUNCTION init() -> bool
332:   // (REQ-LF-5)
333:   SET self.need_swap = !self.formats.unwrap().want_big_endian
334:   SET self.initialized = true
335:   RETURN true

336: FUNCTION term()
337:   CALL self.close()                 // (REQ-EH-5)

338: FUNCTION open(path)
339:   LET data = std::fs::read(path)?
340:   LET name = path.to_string_lossy()
341:   RETURN self.open_from_bytes(&data, &name)

342: FUNCTION get_frame() -> u32
343:   RETURN 0                          // (REQ-LF-6)

344: FUNCTION frequency() -> u32
345:   RETURN self.frequency

346: FUNCTION format() -> AudioFormat
347:   RETURN self.format

348: FUNCTION length() -> f32
349:   RETURN self.length

350: FUNCTION is_null() -> bool
351:   RETURN false                      // (REQ-LF-9)

352: FUNCTION needs_swap() -> bool
353:   RETURN self.need_swap             // (REQ-LF-10)
```
