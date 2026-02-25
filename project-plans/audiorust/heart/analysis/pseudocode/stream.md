# Pseudocode — `sound::stream`

Plan ID: `PLAN-20260225-AUDIO-HEART`

---

## 1. init_stream_decoder

```
01: FUNCTION init_stream_decoder() -> AudioResult<()>
02:   IF ENGINE.decoder_thread.is_some() THEN
03:     RETURN Err(AudioError::AlreadyInitialized)
04:   END IF
05:   SET ENGINE.shutdown = false (Ordering::Release)
06:   SPAWN thread "audio stream decoder" running stream_decoder_task
07:   IF spawn fails THEN
08:     RETURN Err(AudioError::NotInitialized)
09:   END IF
10:   STORE JoinHandle in ENGINE.decoder_thread
11:   RETURN Ok(())
```

Validation: REQ-STREAM-INIT-01..03
Side effects: Spawns OS thread, initializes FadeState mutex
Integration: Must be called after mixer_init()

## 2. uninit_stream_decoder

```
20: FUNCTION uninit_stream_decoder() -> AudioResult<()>
21:   IF ENGINE.decoder_thread.is_none() THEN
22:     LOG warn "decoder thread not running"
23:     RETURN Ok(())    // REQ-STREAM-INIT-06
24:   END IF
25:   SET ENGINE.shutdown = true (Ordering::Release)
26:   NOTIFY ENGINE.wake condvar    // wake sleeping thread
27:   TAKE handle = ENGINE.decoder_thread.take()
28:   CALL handle.join()
29:   IF join fails THEN
30:     LOG error "decoder thread panicked"
31:   END IF
32:   RETURN Ok(())
```

Validation: REQ-STREAM-INIT-04..07
Side effects: Joins OS thread, FadeState drops via Rust semantics
Ordering: Must be called before program exit

## 3. create_sound_sample

```
40: FUNCTION create_sound_sample(decoder, num_buffers, callbacks) -> AudioResult<SoundSample>
41:   LET buffers = mixer_gen_buffers(num_buffers)?
42:   IF buffers failed THEN
43:     RETURN Err(AudioError::MixerError(e))
44:   END IF
45:   LET buffer_tags = vec![None; num_buffers as usize]
46:   CONSTRUCT SoundSample {
47:     decoder: decoder,
48:     length: 0.0,
49:     buffers: buffers,
50:     num_buffers: num_buffers,
51:     buffer_tags: buffer_tags,
52:     offset: 0,
53:     data: None,
54:     callbacks: callbacks,
55:   }
56:   RETURN Ok(sample)
```

Validation: REQ-STREAM-SAMPLE-01
Error handling: Propagates mixer_gen_buffers failure

## 4. destroy_sound_sample

```
60: FUNCTION destroy_sound_sample(sample) -> AudioResult<()>
61:   CALL mixer_delete_buffers(&sample.buffers)
62:   CLEAR sample.buffers
63:   CLEAR sample.buffer_tags
64:   SET sample.callbacks = None
65:   // Note: decoder NOT dropped here (owned by caller or chunk)
66:   RETURN Ok(())
```

Validation: REQ-STREAM-SAMPLE-02
Side effects: Frees mixer buffer handles

## 5. play_stream

```
70: FUNCTION play_stream(sample_arc, source_index, looping, scope, rewind) -> AudioResult<()>
71:   VALIDATE source_index < NUM_SOUNDSOURCES
72:   CALL stop_stream(source_index)?    // REQ-STREAM-PLAY-01
73:
74:   LET source = SOURCES.sources[source_index].lock()
75:   LET mut sample = sample_arc.lock()
76:
77:   // REQ-STREAM-PLAY-03: callback abort check
78:   IF let Some(callbacks) = &mut sample.callbacks THEN
79:     IF NOT callbacks.on_start_stream(&mut sample) THEN
80:       RETURN Err(AudioError::EndOfStream)
81:     END IF
82:   END IF
83:
84:   // REQ-STREAM-PLAY-04: clear tags
85:   FOR tag IN sample.buffer_tags.iter_mut() DO
86:     SET *tag = None
87:   END FOR
88:
89:   // REQ-STREAM-PLAY-05/06: handle rewind or compute offset
90:   LET decoder = sample.decoder.as_mut().ok_or(AudioError::InvalidDecoder)?
91:   IF rewind THEN
92:     CALL decoder.seek(0)?
93:     SET offset = sample.offset
94:   ELSE
95:     SET offset = sample.offset + (get_decoder_time(decoder) * ONE_SECOND as f32) as i32
96:   END IF
97:
98:   // REQ-STREAM-PLAY-07: source setup
99:   SET source.sample = Some(Arc::clone(&sample_arc))
100:  SET sample.looping = looping    // stored on sample, not decoder
101:  CALL mixer_source_i(source.handle, SourceProp::Looping, 0)
102:
103:  // REQ-STREAM-PLAY-08: scope buffer
104:  IF scope THEN
105:    LET buf_size = query buffer size from decoder
106:    SET source.sbuf_size = sample.num_buffers * buf_size + PAD_SCOPE_BYTES
107:    SET source.sbuffer = Some(vec![0u8; source.sbuf_size as usize])
108:    SET source.sbuf_tail = 0
109:    SET source.sbuf_head = 0
110:  END IF
111:
112:  // REQ-STREAM-PLAY-09..12: pre-fill buffers
113:  LET format = decoder.format()
114:  LET freq = decoder.frequency()
115:  FOR i IN 0..sample.num_buffers DO
116:    LET mut buf = vec![0u8; buffer_size]
117:    LET result = decoder.decode(&mut buf)
118:    MATCH result:
119:      Ok(0) => BREAK    // REQ-STREAM-PLAY-12
120:      Ok(n) => {
121:        CALL mixer_buffer_data(sample.buffers[i], format, &buf[..n], freq, ...)
122:        CALL mixer_source_queue_buffers(source.handle, &[sample.buffers[i]])
123:        IF let Some(cb) = &mut sample.callbacks THEN
124:          cb.on_queue_buffer(&mut sample, sample.buffers[i])  // REQ-STREAM-PLAY-10
125:        END IF
126:        IF scope THEN add_scope_data(source, &buf[..n]) END IF
127:      }
128:      Err(DecodeError::EndOfFile) => {    // REQ-STREAM-PLAY-11
129:        IF let Some(cb) = &mut sample.callbacks THEN
130:          IF cb.on_end_chunk(&mut sample, sample.buffers[i]) THEN
131:            CONTINUE (decoder replaced)
132:          END IF
133:        END IF
134:        BREAK
135:      }
136:      Err(e) => LOG error and BREAK
137:    END MATCH
138:  END FOR
139:
140:  // REQ-STREAM-PLAY-13: start playback
141:  SET source.sbuf_lasttime = get_time_counter()
142:  SET source.start_time = get_time_counter() as i32 - offset
143:  SET source.pause_time = 0
144:  SET source.stream_should_be_playing = true
145:  CALL mixer_source_play(source.handle)
146:  DROP source lock
147:  NOTIFY ENGINE.wake condvar    // wake decoder thread
148:  RETURN Ok(())
```

Validation: REQ-STREAM-PLAY-01..13
Error handling: Propagates decoder/mixer errors
Integration: Calls mixer API directly, wakes decoder thread
Side effects: Modifies source state, starts mixer playback

## 6. stop_stream

```
160: FUNCTION stop_stream(source_index) -> AudioResult<()>
161:   VALIDATE source_index < NUM_SOUNDSOURCES
162:   LET source = SOURCES.sources[source_index].lock()
163:   CALL stop_source(source_index)?
164:   SET source.stream_should_be_playing = false
165:   SET source.sample = None
166:   SET source.sbuffer = None
167:   SET source.sbuf_size = 0
168:   SET source.sbuf_tail = 0
169:   SET source.sbuf_head = 0
170:   SET source.sbuf_lasttime = 0
171:   SET source.pause_time = 0
172:   RETURN Ok(())
```

Validation: REQ-STREAM-PLAY-14

## 7. pause_stream

```
180: FUNCTION pause_stream(source_index) -> AudioResult<()>
181:   LET source = SOURCES.sources[source_index].lock()
182:   SET source.stream_should_be_playing = false
183:   IF source.pause_time == 0 THEN
184:     SET source.pause_time = get_time_counter()
185:   END IF
186:   CALL mixer_source_pause(source.handle)
187:   RETURN Ok(())
```

Validation: REQ-STREAM-PLAY-15

## 8. resume_stream

```
190: FUNCTION resume_stream(source_index) -> AudioResult<()>
191:   LET source = SOURCES.sources[source_index].lock()
192:   IF source.pause_time != 0 THEN
193:     SET source.start_time += get_time_counter() as i32 - source.pause_time as i32
194:   END IF
195:   SET source.pause_time = 0
196:   SET source.stream_should_be_playing = true
197:   CALL mixer_source_play(source.handle)
198:   RETURN Ok(())
```

Validation: REQ-STREAM-PLAY-16..17

## 9. seek_stream

```
200: FUNCTION seek_stream(source_index, pos_ms) -> AudioResult<()>
201:   LET source = SOURCES.sources[source_index].lock()
202:   IF source.sample.is_none() THEN
203:     RETURN Err(AudioError::InvalidSample)   // REQ-STREAM-PLAY-19
204:   END IF
205:   CALL mixer_source_stop(source.handle)
206:   LET sample = source.sample.as_ref().unwrap().lock()
207:   LET decoder = sample.decoder.as_mut().ok_or(AudioError::InvalidDecoder)?
208:   LET pcm_pos = pos_ms * decoder.frequency() / 1000
209:   CALL decoder.seek(pcm_pos)?
210:   DROP source lock
211:   CALL play_stream(sample_arc, source_index, sample.looping, scope_was_active, false)
212:   RETURN Ok(())
```

Validation: REQ-STREAM-PLAY-18..19

## 10. stream_decoder_task (background thread)

```
220: FUNCTION stream_decoder_task()
221:   WHILE NOT ENGINE.shutdown.load(Ordering::Acquire) DO   // REQ-STREAM-THREAD-01
222:     CALL process_music_fade()   // REQ-STREAM-THREAD-02
223:     LET any_active = false
224:
225:     FOR source_idx IN MUSIC_SOURCE..=SPEECH_SOURCE DO   // REQ-STREAM-THREAD-03
226:       LET source = SOURCES.sources[source_idx].lock()   // REQ-STREAM-THREAD-04
227:
228:       // REQ-STREAM-THREAD-05: skip check
229:       IF source.sample.is_none() THEN CONTINUE END IF
230:       IF NOT source.stream_should_be_playing THEN CONTINUE END IF
231:       LET sample = source.sample.as_ref().unwrap().lock()
232:       IF sample.decoder.is_none() THEN CONTINUE END IF
233:
234:       SET any_active = true
235:       CALL process_source_stream(source, sample)   // see §11
236:       DROP source lock
237:     END FOR
238:
239:     IF NOT any_active THEN
240:       WAIT on ENGINE.wake condvar with timeout 100ms   // REQ-STREAM-THREAD-06
241:     ELSE
242:       CALL std::thread::yield_now()   // REQ-STREAM-THREAD-07
243:     END IF
244:   END WHILE
245:   // REQ-STREAM-THREAD-08: clean exit
```

Validation: REQ-STREAM-THREAD-01..08
Threading: Runs on decoder thread only

## 11. process_source_stream (per-source processing)

```
260: FUNCTION process_source_stream(source, sample)
261:   LET processed = mixer_get_source_i(source.handle, SourceProp::BuffersProcessed)
262:   LET queued = mixer_get_source_i(source.handle, SourceProp::BuffersQueued)
263:
264:   // REQ-STREAM-PROCESS-02..03: end detection / underrun
265:   IF processed == 0 THEN
266:     LET state = mixer_get_source_i(source.handle, SourceProp::SourceState)
267:     IF state != SourceState::Playing THEN
268:       IF queued == 0 AND decoder_at_eof(sample) THEN
269:         SET source.stream_should_be_playing = false
270:         IF let Some(cb) = &mut sample.callbacks THEN
271:           cb.on_end_stream(&mut sample)   // REQ-STREAM-PROCESS-02
272:         END IF
273:         RETURN
274:       ELSE IF queued > 0 THEN
275:         LOG warn "buffer underrun"   // REQ-STREAM-PROCESS-03
276:         CALL mixer_source_play(source.handle)
277:       END IF
278:     END IF
279:     RETURN
280:   END IF
281:
282:   // REQ-STREAM-PROCESS-04..16: process each completed buffer
283:   LET end_chunk_failed = false
284:   FOR _ IN 0..processed DO
285:     LET unqueued = mixer_source_unqueue_buffers(source.handle, 1)
286:     IF unqueued.is_err() THEN    // REQ-STREAM-PROCESS-05
287:       LOG error "unqueue failed"
288:       BREAK
289:     END IF
290:     LET buf_handle = unqueued[0]
291:
292:     // REQ-STREAM-PROCESS-06: tagged buffer callback
293:     IF let Some(cb) = &mut sample.callbacks THEN
294:       IF let Some(tag) = find_tagged_buffer(sample, buf_handle) THEN
295:         cb.on_tagged_buffer(&mut sample, tag)
296:       END IF
297:     END IF
298:
299:     // REQ-STREAM-PROCESS-07: scope remove
300:     IF source.sbuffer.is_some() THEN
301:       CALL remove_scope_data(source, buf_handle)
302:     END IF
303:
304:     // Decode new audio for this buffer
305:     LET decoder = sample.decoder.as_mut()
306:     IF decoder.is_none() OR end_chunk_failed THEN CONTINUE END IF
307:
308:     LET mut buf = vec![0u8; buffer_size]
309:     LET result = decoder.decode(&mut buf)
310:     MATCH result:
311:       Ok(0) => CONTINUE    // REQ-STREAM-PROCESS-13
312:       Ok(n) => {
313:         CALL mixer_buffer_data(buf_handle, ...)   // REQ-STREAM-PROCESS-14
314:         CALL mixer_source_queue_buffers(source.handle, &[buf_handle])
315:         SET source.last_q_buf = buf_handle   // REQ-STREAM-PROCESS-15
316:         IF let Some(cb) = &mut sample.callbacks THEN
317:           cb.on_queue_buffer(&mut sample, buf_handle)
318:         END IF
319:         IF source.sbuffer.is_some() THEN
320:           CALL add_scope_data(source, &buf[..n])   // REQ-STREAM-PROCESS-16
321:         END IF
322:       }
323:       Err(DecodeError::EndOfFile) => {   // REQ-STREAM-PROCESS-08..09
324:         IF let Some(cb) = &mut sample.callbacks THEN
325:           IF cb.on_end_chunk(&mut sample, buf_handle) THEN
326:             // decoder replaced, continue with new decoder
327:           ELSE
328:             SET end_chunk_failed = true
329:           END IF
330:         ELSE
331:           SET end_chunk_failed = true
332:         END IF
333:       }
334:       Err(DecodeError::DecoderError(_)) => {   // REQ-STREAM-PROCESS-12
335:         LOG error "decode error"
336:         SET source.stream_should_be_playing = false
337:         BREAK
338:       }
339:       Err(_) => CONTINUE    // REQ-STREAM-PROCESS-10
340:     END MATCH
341:   END FOR
```

Validation: REQ-STREAM-PROCESS-01..16

## 12. process_music_fade

```
360: FUNCTION process_music_fade()
361:   LET fade = ENGINE.fade.lock()
362:   IF fade.interval == 0 THEN    // REQ-STREAM-FADE-05
363:     RETURN
364:   END IF
365:   LET elapsed = (get_time_counter() - fade.start_time).min(fade.interval)
366:   LET volume = fade.start_volume + fade.delta * elapsed as i32 / fade.interval as i32
367:   CALL set_music_volume(volume)    // REQ-STREAM-FADE-03
368:   IF elapsed >= fade.interval THEN
369:     SET fade.interval = 0   // REQ-STREAM-FADE-04
370:   END IF
```

Validation: REQ-STREAM-FADE-03..05

## 13. set_music_stream_fade

```
380: FUNCTION set_music_stream_fade(how_long, end_volume) -> bool
381:   IF how_long == 0 THEN    // REQ-STREAM-FADE-02
382:     RETURN false
383:   END IF
384:   LET fade = ENGINE.fade.lock()
385:   SET fade.start_time = get_time_counter()
386:   SET fade.interval = how_long
387:   SET fade.start_volume = current_music_volume()
388:   SET fade.delta = end_volume - fade.start_volume
389:   RETURN true    // REQ-STREAM-FADE-01
```

Validation: REQ-STREAM-FADE-01..02

## 14. graph_foreground_stream

```
400: FUNCTION graph_foreground_stream(data, width, height, want_speech) -> usize
401:   // REQ-STREAM-SCOPE-03: source selection
402:   LET source_idx = IF want_speech AND speech_has_decoder() THEN SPEECH_SOURCE ELSE MUSIC_SOURCE
403:   LET source = SOURCES.sources[source_idx].lock()
404:
405:   // REQ-STREAM-SCOPE-04: no stream check
406:   IF source.sample.is_none() OR source.sbuffer.is_none() OR source.sbuf_size == 0 THEN
407:     RETURN 0
408:   END IF
409:
410:   LET sample = source.sample.as_ref().unwrap().lock()
411:   LET decoder = sample.decoder.as_ref()
412:   IF decoder.is_none() THEN RETURN 0 END IF
413:
414:   // REQ-STREAM-SCOPE-05: step size
415:   LET base_step = IF source_idx == SPEECH_SOURCE THEN 1 ELSE 4
416:   LET freq_scale = decoder.frequency() as f32 / 11025.0
417:   LET bytes_per_sample = decoder.format().bytes_per_sample()
418:   LET step = max(1, (base_step as f32 * freq_scale) as usize) * bytes_per_sample
419:
420:   // REQ-STREAM-SCOPE-06: read position
421:   LET delta_time = get_time_counter() - source.sbuf_lasttime
422:   LET delta_bytes = (delta_time as f32 * decoder.frequency() as f32
423:                      * bytes_per_sample as f32 / ONE_SECOND as f32) as u32
424:   LET read_pos = (source.sbuf_head + delta_bytes.min(source.sbuf_size)) % source.sbuf_size
425:
426:   // REQ-STREAM-SCOPE-09..10: AGC state
427:   LET mut agc_pages = [DEF_PAGE_MAX; AGC_PAGE_COUNT]
428:   LET mut agc_idx = 0
429:   LET mut frame_count = 0
430:   LET mut page_max = 0
431:
432:   // REQ-STREAM-SCOPE-07: render loop
433:   LET target_amp = height as i32 / 4
434:   FOR x IN 0..width DO
435:     LET sample_val = read_sound_sample(source.sbuffer, read_pos, format)
436:     IF channels > 1 THEN    // REQ-STREAM-SCOPE-11
437:       sample_val += read_sound_sample(source.sbuffer, read_pos + bytes_per_channel, format)
438:     END IF
439:
440:     // AGC update
441:     SET page_max = max(page_max, sample_val.abs())
442:     INC frame_count
443:     IF frame_count >= AGC_FRAME_COUNT THEN
444:       IF page_max > VAD_MIN_ENERGY THEN    // REQ-STREAM-SCOPE-10
445:         SET agc_pages[agc_idx] = page_max
446:         SET agc_idx = (agc_idx + 1) % AGC_PAGE_COUNT
447:       END IF
448:       SET frame_count = 0
449:       SET page_max = 0
450:     END IF
451:
452:     // Compute average amplitude
453:     LET avg_amp = agc_pages.iter().sum::<i32>() / AGC_PAGE_COUNT
454:     LET scaled = sample_val * target_amp / max(avg_amp, 1)
455:     LET y = (height as i32 / 2 + scaled).clamp(0, height as i32 - 1)
456:     SET data[x] = y
457:
458:     SET read_pos = (read_pos + step as u32) % source.sbuf_size
459:   END FOR
460:   RETURN width
```

Validation: REQ-STREAM-SCOPE-01..11

## 15. read_sound_sample (helper)

```
470: FUNCTION read_sound_sample(buffer, pos, format) -> i16
471:   IF format is 8-bit THEN
472:     LET val = buffer[pos as usize] as i16
473:     RETURN (val - 128) << 8    // REQ-STREAM-SCOPE-08
474:   ELSE
475:     LET lo = buffer[pos as usize] as i16
476:     LET hi = buffer[(pos + 1) as usize] as i16
477:     RETURN lo | (hi << 8)
478:   END IF
```

Validation: REQ-STREAM-SCOPE-08

## 16. Buffer Tagging

```
490: FUNCTION find_tagged_buffer(sample, buffer) -> Option<&SoundTag>
491:   FOR tag_slot IN sample.buffer_tags.iter() DO
492:     IF let Some(tag) = tag_slot THEN
493:       IF tag.buf_handle == buffer THEN
494:         RETURN Some(tag)    // REQ-STREAM-TAG-01
495:       END IF
496:     END IF
497:   END FOR
498:   RETURN None
499:
500: FUNCTION tag_buffer(sample, buffer, data) -> bool
501:   FOR tag_slot IN sample.buffer_tags.iter_mut() DO
502:     IF tag_slot.is_none() OR tag_slot.as_ref().map(|t| t.buf_handle) == Some(buffer) THEN
503:       SET *tag_slot = Some(SoundTag { buf_handle: buffer, data: data })
504:       RETURN true    // REQ-STREAM-TAG-02
505:     END IF
506:   END FOR
507:   RETURN false
508:
509: FUNCTION clear_buffer_tag(tag) {
510:   // The Option<SoundTag> containing this tag is set to None
511:   // REQ-STREAM-TAG-03
512: }
```

Validation: REQ-STREAM-TAG-01..03

## 17. Scope Data Helpers

```
520: FUNCTION add_scope_data(source, decoded_bytes)
521:   LET sbuf = source.sbuffer.as_mut().unwrap()
522:   LET size = source.sbuf_size as usize
523:   FOR byte IN decoded_bytes DO
524:     sbuf[source.sbuf_tail as usize] = *byte
525:     SET source.sbuf_tail = (source.sbuf_tail + 1) as u32 % size as u32
526:   END FOR
527:   // REQ-STREAM-SCOPE-01
528:
529: FUNCTION remove_scope_data(source, buffer_handle)
530:   LET buf_size = mixer_get_buffer_i(buffer_handle, BufferProp::Size).unwrap_or(0) as u32
531:   SET source.sbuf_head = (source.sbuf_head + buf_size) % source.sbuf_size
532:   SET source.sbuf_lasttime = get_time_counter()
533:   // REQ-STREAM-SCOPE-02
```

Validation: REQ-STREAM-SCOPE-01..02
