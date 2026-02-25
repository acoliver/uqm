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
63:   SET sample.num_buffers = 0
64:   CLEAR sample.buffer_tags
65:   SET sample.callbacks = None
66:   // Note: decoder NOT dropped here (owned by caller or chunk)
67:   RETURN Ok(())
```

Validation: REQ-STREAM-SAMPLE-02
Side effects: Frees mixer buffer handles

## 5. play_stream

**LOCK NOTE**: The buffer fill loop (lines 113-138) calls on_queue_buffer/on_end_chunk while
holding Source+Sample locks. TrackCallbacks' on_end_chunk acquires TRACK_STATE, making the
order Source→Sample→TRACK_STATE (inverting the hierarchy). This matches the C implementation
where PlayStream calls callbacks under the source lock. It is safe because play_stream and
stop_track are single-threaded (main thread only in UQM). If multi-threaded callers are ever
added, the buffer fill callbacks would need the deferred callback pattern too.

```
70: FUNCTION play_stream(sample_arc, source_index, looping, scope, rewind) -> AudioResult<()>
71:   VALIDATE source_index < NUM_SOUNDSOURCES ELSE RETURN Err(AudioError::InvalidSource(source_index))
72:   CALL stop_stream(source_index)?    // REQ-STREAM-PLAY-01
73:
74:   // REQ-STREAM-PLAY-03: callback abort check (BEFORE acquiring locks to avoid ordering violation)
75:   // Callbacks may acquire TRACK_STATE which is higher in lock hierarchy than Source/Sample.
76:   LET mut sample_pre = sample_arc.lock()
77:   IF let Some(callbacks) = &mut sample_pre.callbacks THEN
78:     IF NOT callbacks.on_start_stream(&mut sample_pre) THEN
79:       RETURN Err(AudioError::EndOfStream)
80:     END IF
81:   END IF
82:   DROP sample_pre   // release before acquiring Source lock
83:
84:   LET source = SOURCES.sources[source_index].lock()
85:   LET mut sample = sample_arc.lock()
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
161:   VALIDATE source_index < NUM_SOUNDSOURCES ELSE RETURN Err(AudioError::InvalidSource(source_index))
162:   LET source = SOURCES.sources[source_index].lock()
163:   // Inline stop_source logic to avoid self-deadlock (source lock already held)
164:   CALL mixer_source_stop(source.handle)
165:   SET source.stream_should_be_playing = false
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

**LOCK SAFETY (FIX: ISSUE-ALG-02)**: `play_stream` acquires the Source lock internally.
We must NOT hold the Source lock when calling it (parking_lot::Mutex is not reentrant).
Extract necessary state while holding the lock, drop both locks, then call `play_stream`.

```
200: FUNCTION seek_stream(source_index, pos_ms) -> AudioResult<()>
201:   // Phase 1: Extract state under locks
202:   LET source = SOURCES.sources[source_index].lock()
203:   IF source.sample.is_none() THEN
204:     RETURN Err(AudioError::InvalidSample)   // REQ-STREAM-PLAY-19
205:   END IF
206:   LET sample_arc = Arc::clone(source.sample.as_ref().unwrap())
207:   LET mixer_handle = source.handle
208:   LET scope_was_active = source.sbuffer.is_some()
209:   CALL mixer_source_stop(mixer_handle)
210:
211:   LET sample = sample_arc.lock()
212:   LET decoder = sample.decoder.as_mut().ok_or(AudioError::InvalidDecoder)?
213:   LET pcm_pos = pos_ms * decoder.frequency() / 1000
214:   CALL decoder.seek(pcm_pos)?
215:   LET looping = sample.looping
216:
217:   // Phase 2: Drop both locks before calling play_stream
218:   DROP sample lock
219:   DROP source lock
220:
221:   // Phase 3: Restart — play_stream acquires Source+Sample locks internally
222:   CALL play_stream(sample_arc, source_index, looping, scope_was_active, false)
223:   RETURN Ok(())
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

**LOCK SAFETY**: This function is called with Source mutex and Sample mutex held.
Callbacks (`on_end_stream`, `on_end_chunk`, `on_tagged_buffer`, `on_queue_buffer`)
may need to acquire `TRACK_STATE` (e.g., `TrackCallbacks`). To respect the lock
ordering (`TRACK_STATE → Source → Sample → FadeState`), callbacks MUST NOT be
invoked while Source or Sample locks are held.

**Solution**: Collect deferred callback actions into a `Vec<DeferredCallback>` enum
during processing, then release both locks, then execute deferred callbacks outside
the locks.

```
259: ENUM DeferredCallback {
260:   EndStream,
261:   EndChunk { buf_handle: u32, decoder_replaced: bool },
262:   TaggedBuffer { tag: SoundTag },
263:   QueueBuffer { buf_handle: u32 },
264: }

266: FUNCTION process_source_stream(source, sample)
267:   // --- Phase 1: Process under locks, collect deferred callbacks ---
268:   LET mut deferred: Vec<DeferredCallback> = Vec::new()
269:   LET processed = mixer_get_source_i(source.handle, SourceProp::BuffersProcessed)
270:   LET queued = mixer_get_source_i(source.handle, SourceProp::BuffersQueued)

272:   // REQ-STREAM-PROCESS-02..03: end detection / underrun
273:   // Note: when processed == 0 and state == Playing, this is a no-op
274:   // (mixer is still consuming queued buffers; decoder will check again next iteration)
275:   IF processed == 0 THEN
276:     LET state = mixer_get_source_i(source.handle, SourceProp::SourceState)
277:     IF state != SourceState::Playing THEN
278:       IF queued == 0 AND decoder_at_eof(sample) THEN
279:         SET source.stream_should_be_playing = false
280:         deferred.push(DeferredCallback::EndStream)   // REQ-STREAM-PROCESS-02
281:         // --- Drop locks, then execute deferred callbacks ---
282:         DROP sample lock
283:         DROP source lock
284:         CALL execute_deferred_callbacks(deferred, sample_arc_clone, source_idx)
285:         RETURN
286:       ELSE IF queued > 0 THEN
287:         LOG warn "buffer underrun"   // REQ-STREAM-PROCESS-03
288:         CALL mixer_source_play(source.handle)
289:       END IF
290:     END IF
291:     RETURN
292:   END IF

294:   // REQ-STREAM-PROCESS-04..16: process each completed buffer
295:   LET end_chunk_failed = false
296:   FOR _ IN 0..processed DO
297:     LET unqueued = mixer_source_unqueue_buffers(source.handle, 1)
298:     IF unqueued.is_err() THEN    // REQ-STREAM-PROCESS-05
299:       LOG error "unqueue failed"
300:       BREAK
301:     END IF
302:     LET buf_handle = unqueued[0]

304:     // REQ-STREAM-PROCESS-06: tagged buffer callback (deferred)
305:     IF let Some(tag) = find_tagged_buffer(sample, buf_handle) THEN
306:       deferred.push(DeferredCallback::TaggedBuffer { tag: tag.clone() })
307:     END IF

309:     // REQ-STREAM-PROCESS-07: scope remove
310:     IF source.sbuffer.is_some() THEN
311:       CALL remove_scope_data(source, buf_handle)
312:     END IF

314:     // Decode new audio for this buffer
315:     LET decoder = sample.decoder.as_mut()
316:     IF decoder.is_none() OR end_chunk_failed THEN CONTINUE END IF

318:     LET mut buf = vec![0u8; buffer_size]
319:     LET result = decoder.decode(&mut buf)
320:     MATCH result:
321:       Ok(0) => CONTINUE    // REQ-STREAM-PROCESS-13
322:       Ok(n) => {
323:         CALL mixer_buffer_data(buf_handle, ...)   // REQ-STREAM-PROCESS-14
324:         CALL mixer_source_queue_buffers(source.handle, &[buf_handle])
325:         SET source.last_q_buf = buf_handle   // REQ-STREAM-PROCESS-15
326:         deferred.push(DeferredCallback::QueueBuffer { buf_handle })
327:         IF source.sbuffer.is_some() THEN
328:           CALL add_scope_data(source, &buf[..n])   // REQ-STREAM-PROCESS-16
329:         END IF
330:       }
331:       Err(DecodeError::EndOfFile) => {   // REQ-STREAM-PROCESS-08..09
332:         deferred.push(DeferredCallback::EndChunk { buf_handle, decoder_replaced: false })
333:         SET end_chunk_failed = true   // tentatively; callback may replace decoder
334:       }
335:       Err(DecodeError::DecoderError(_)) => {   // REQ-STREAM-PROCESS-12
336:         LOG error "decode error"
337:         SET source.stream_should_be_playing = false
338:         BREAK
339:       }
340:       Err(_) => CONTINUE    // REQ-STREAM-PROCESS-10
341:     END MATCH
342:   END FOR

344:   // --- Phase 2: Drop locks, then execute deferred callbacks ---
345:   LET sample_arc_clone = source.sample.as_ref().map(Arc::clone)
346:   DROP sample lock
347:   DROP source lock
348:   CALL execute_deferred_callbacks(deferred, sample_arc_clone, source_idx)

350: FUNCTION execute_deferred_callbacks(deferred, sample_arc_opt, source_index)
351:   // Called with NO locks held — safe to acquire TRACK_STATE
352:   //
353:   // FIX ISSUE-ALG-01 (TOCTOU): Between dropping locks and arriving here,
354:   // another thread may have called stop_stream(), setting source.sample = None.
355:   // The sample_arc clone keeps the SoundSample alive (Arc refcount > 0), but
356:   // the source no longer references it. Before executing callbacks, verify
357:   // the source still points to this sample. If not, skip callbacks — the
358:   // stream was stopped and callbacks are stale.
359:   IF sample_arc_opt.is_none() THEN RETURN END IF
360:   LET sample_arc = sample_arc_opt.unwrap()
361:
362:   // Validity check: re-lock source briefly to verify sample pointer match
363:   {
364:     LET source = SOURCES.sources[source_index].lock()
365:     IF source.sample.is_none()
366:        OR NOT Arc::ptr_eq(source.sample.as_ref().unwrap(), &sample_arc) THEN
367:       LOG debug "deferred callbacks skipped: source sample changed (stop_stream race)"
368:       RETURN   // stream was stopped — skip all deferred callbacks
369:     END IF
370:     // source lock dropped here
371:   }
372:
373:   FOR action IN deferred DO
355:     MATCH action:
356:       DeferredCallback::EndStream => {
357:         LET mut sample = sample_arc.lock()
358:         IF let Some(cb) = &mut sample.callbacks THEN
359:           cb.on_end_stream(&mut sample)
360:         END IF
361:       }
362:       DeferredCallback::EndChunk { buf_handle, .. } => {
363:         LET mut sample = sample_arc.lock()
364:         IF let Some(cb) = &mut sample.callbacks THEN
365:           IF cb.on_end_chunk(&mut sample, buf_handle) THEN
366:             // decoder replaced by callback — subsequent decode calls
367:             // will pick up the new decoder on next iteration
368:           END IF
369:         END IF
370:       }
371:       DeferredCallback::TaggedBuffer { tag } => {
372:         LET mut sample = sample_arc.lock()
373:         IF let Some(cb) = &mut sample.callbacks THEN
374:           cb.on_tagged_buffer(&mut sample, &tag)
375:         END IF
376:       }
377:       DeferredCallback::QueueBuffer { buf_handle } => {
378:         LET mut sample = sample_arc.lock()
379:         IF let Some(cb) = &mut sample.callbacks THEN
380:           cb.on_queue_buffer(&mut sample, buf_handle)
381:         END IF
382:       }
383:     END MATCH
384:   END FOR
```

Validation: REQ-STREAM-PROCESS-01..16
Lock safety: Callbacks are invoked in `execute_deferred_callbacks` with NO locks
held, allowing them to safely acquire TRACK_STATE (respecting lock ordering:
TRACK_STATE → Source → Sample → FadeState).

## 12. process_music_fade

```
360: FUNCTION process_music_fade()
361:   // LOCK SAFETY: Read fade state, drop lock, then call set_music_volume
362:   // (set_music_volume acquires MUSIC_STATE + Source, which are higher in hierarchy than FadeState)
363:   LET (volume, done) = {
364:     LET fade = ENGINE.fade.lock()
365:     IF fade.interval == 0 THEN    // REQ-STREAM-FADE-05
366:       RETURN
367:     END IF
368:     LET elapsed = (get_time_counter() - fade.start_time).min(fade.interval)
369:     // NOTE: Integer division produces non-linear stepping — the volume
370:     // won't change until elapsed >= interval/delta (first ~6ms of a
371:     // typical fade). This matches the C implementation's behavior and
372:     // is intentional for compatibility. No overflow risk for realistic
373:     // parameters (delta * elapsed stays well within i32 range).
374:     LET vol = fade.start_volume + fade.delta * elapsed as i32 / fade.interval as i32
375:     LET is_done = elapsed >= fade.interval
376:     (vol, is_done)
377:     // fade lock dropped here
378:   }
379:   CALL set_music_volume(volume)    // REQ-STREAM-FADE-03
380:   IF done THEN
381:     LET fade = ENGINE.fade.lock()
382:     SET fade.interval = 0   // REQ-STREAM-FADE-04
383:   END IF
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
470: FUNCTION read_sound_sample(buffer, pos, size, format) -> i16
471:   // size = ring buffer length; wrap reads at boundary to avoid panic
472:   IF format is 8-bit THEN
473:     LET val = buffer[pos as usize] as i16
474:     RETURN (val - 128) << 8    // REQ-STREAM-SCOPE-08
475:   ELSE
476:     LET lo = buffer[pos as usize] as i16
477:     LET hi = buffer[((pos + 1) % size) as usize] as i16   // wrap at ring boundary
478:     RETURN lo | (hi << 8)
479:   END IF
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
509: FUNCTION clear_buffer_tag(sample, tag_ptr)
510:   // Find the buffer_tags slot whose SoundTag matches tag_ptr by pointer
511:   // comparison, then set that slot to None.  REQ-STREAM-TAG-03
512:   FOR tag_slot IN sample.buffer_tags.iter_mut() DO
513:     IF let Some(tag) = tag_slot THEN
514:       IF std::ptr::eq(tag, tag_ptr) THEN
515:         SET *tag_slot = None
516:         RETURN
517:       END IF
518:     END IF
519:   END FOR
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

## 18. decode_all (SoundDecoder default method)

```
540: FUNCTION decode_all(decoder) -> AudioResult<Vec<u8>>
541:   // Buffer growth strategy: pre-allocate if length is known,
542:   // otherwise use doubling strategy starting at 64KB.
543:   // This avoids O(n^2) reallocation for large files.
544:
545:   LET known_length = decoder.length()   // seconds (0.0 if unknown)
546:   LET freq = decoder.frequency()
547:   LET bps = decoder.format().bytes_per_sample()
548:   LET channels = decoder.format().channels()
549:
550:   // Step 1: Estimate total size and pre-allocate
551:   LET initial_capacity = IF known_length > 0.0 THEN
552:     // Known length: pre-allocate exact expected size + 10% headroom
553:     LET estimated_bytes = (known_length * freq as f64 * bps as f64 * channels as f64) as usize
554:     estimated_bytes + estimated_bytes / 10
555:   ELSE
556:     // Unknown length: start at 64KB
557:     65536
558:   END IF
559:
560:   LET mut result = Vec::with_capacity(initial_capacity)
561:   LET mut decode_buf = vec![0u8; 4096]   // fixed-size decode scratch buffer
562:
563:   // Step 2: Decode loop with doubling growth
564:   LOOP
565:     LET n = decoder.decode(&mut decode_buf)
566:     MATCH n:
567:       Ok(0) => BREAK                        // EOF: zero bytes decoded
568:       Ok(bytes_read) => {
569:         // Append decoded bytes to result
570:         // Vec::extend_from_slice handles growth internally;
571:         // because we pre-allocated, this rarely reallocates.
572:         result.extend_from_slice(&decode_buf[..bytes_read])
573:       }
574:       Err(DecodeError::EndOfFile) => BREAK   // explicit EOF signal
575:       Err(DecodeError::DecoderError(e)) => {
576:         LOG error "decode_all: decoder error: {}", e
577:         RETURN Err(AudioError::DecoderError(e.to_string()))
578:       }
579:       Err(DecodeError::NotInitialized) => RETURN Err(AudioError::NotInitialized)   // permanent
580:       Err(DecodeError::InvalidData(_)) => RETURN Err(AudioError::InvalidData)     // permanent
581:       Err(DecodeError::UnsupportedFormat) => RETURN Err(AudioError::Unsupported)  // permanent
582:       Err(DecodeError::NotFound) => RETURN Err(AudioError::NotFound)              // permanent
583:       Err(e) => {                              // IoError, SeekFailed: retry up to 3 times
584:         SET retry_count += 1
585:         IF retry_count > 3 THEN RETURN Err(AudioError::DecoderError(e.to_string())) END IF
586:         CONTINUE
587:       }
580:     END MATCH
581:   END LOOP
582:
583:   // Step 3: Shrink to fit (release unused capacity)
584:   result.shrink_to_fit()
585:   RETURN Ok(result)
```

Validation: REQ-SFX-LOAD-03 (called from get_sound_bank_data)
Buffer strategy:
- **Known length**: Pre-allocate `length * freq * bytes_per_sample * channels + 10%` — single allocation for most files.
- **Unknown length**: Start at 64KB. `Vec::extend_from_slice` uses the standard Rust doubling strategy internally (amortized O(1) per byte), so total reallocation cost is O(n log n) worst case, not O(n^2).
- **Scratch buffer**: Fixed 4KB decode buffer avoids per-iteration allocation.
- **Shrink**: `shrink_to_fit()` releases over-allocation after decoding completes.
