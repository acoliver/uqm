# Pseudocode — `sound::trackplayer`

Plan ID: `PLAN-20260225-AUDIO-HEART`

---

## 1. splice_track

```
01: FUNCTION splice_track(track_name, track_text, timestamp, callback) -> AudioResult<()>
02:   LET state = TRACK_STATE.lock()
03:
04:   // REQ-TRACK-ASSEMBLE-06: no text early return
05:   IF track_text.is_none() THEN
06:     RETURN Ok(())
07:   END IF
08:
09:   // REQ-TRACK-ASSEMBLE-04..05: subtitle-only append (no track_name)
10:   IF track_name.is_none() THEN
11:     IF state.track_count == 0 THEN
12:       LOG warn "splice_track: no tracks to append to"   // REQ-TRACK-ASSEMBLE-05
13:       RETURN Ok(())
14:     END IF
15:     // Append text to last subtitle chunk
16:     LET last_sub = unsafe { &mut *state.last_sub }
17:     LET pages = split_sub_pages(track_text.unwrap())   // REQ-TRACK-ASSEMBLE-01
18:     last_sub.text.as_mut().map(|t| t.push_str(&pages[0].text))
19:     // Append remaining pages as new chunks linked after last_sub
20:     FOR page IN pages[1..] DO
21:       APPEND new SoundChunk with page text to linked list
22:     END FOR
23:     RETURN Ok(())
24:   END IF
25:
26:   // REQ-TRACK-ASSEMBLE-07: new track with decoder
27:   LET name = track_name.unwrap()
28:   LET decoder = load_decoder(content_dir, name, 4096, 0, 0)?
29:   IF decoder fails THEN
30:     RETURN Err(AudioError::ResourceNotFound(name))
31:   END IF
32:
33:   // First track: create sound_sample
34:   IF state.track_count == 0 THEN
35:     LET callbacks = Box::new(TrackCallbacks) as Box<dyn StreamCallbacks + Send>
36:     LET sample = create_sound_sample(None, 8, Some(callbacks))?
37:     SET state.sound_sample = Some(Arc::new(Mutex::new(sample)))
38:   END IF
39:
40:   // REQ-TRACK-ASSEMBLE-01: split pages
41:   LET pages = split_sub_pages(track_text.unwrap())
42:
43:   // REQ-TRACK-ASSEMBLE-09: explicit timestamps
44:   LET timestamps = IF timestamp.is_some() THEN
45:     get_time_stamps(timestamp.unwrap())
46:   ELSE
47:     vec![]
48:   END IF
49:
50:   // Build chunks from pages
51:   LET dec_length = decoder.length() * 1000.0   // ms
52:   FOR (i, page) IN pages.iter().enumerate() DO
53:     LET run_time = IF i < timestamps.len() THEN timestamps[i] as i32 ELSE page.timestamp
54:
55:     // REQ-TRACK-ASSEMBLE-10: negate last page timestamp
56:     IF i == pages.len() - 1 THEN
57:       SET run_time = -run_time.abs()
58:     END IF
59:
60:     // REQ-TRACK-ASSEMBLE-11: no_page_break handling
61:     IF state.no_page_break AND state.track_count > 0 AND i == 0 THEN
62:       LET last_sub = unsafe { &mut *state.last_sub }
63:       last_sub.text.as_mut().map(|t| t.push_str(&page.text))
64:       SET state.no_page_break = false   // REQ-TRACK-ASSEMBLE-13
65:       CONTINUE
66:     END IF
67:
68:     // REQ-TRACK-ASSEMBLE-12: create chunk
69:     LET chunk = SoundChunk {
70:       decoder: decoder (for first page) / NullDecoder (for subsequent),
71:       start_time: state.dec_offset,
72:       tag_me: true,
73:       track_num: state.track_count,
74:       text: Some(page.text.clone()),
75:       callback: IF i == 0 THEN callback.take() ELSE None,
76:       next: None,
77:     }
78:
79:     // Link into list
80:     APPEND chunk to linked list (tail insertion)
81:     SET state.last_sub = ptr to chunk (if has text)
82:     SET state.no_page_break = false   // REQ-TRACK-ASSEMBLE-13
83:   END FOR
84:
85:   // REQ-TRACK-ASSEMBLE-08: accumulate dec_offset
86:   SET state.dec_offset += dec_length
87:   SET state.track_count += 1
88:   SET state.last_track_name = name.to_string()
89:   RETURN Ok(())
```

Validation: REQ-TRACK-ASSEMBLE-01..14
Error handling: Decoder load failure → ResourceNotFound
Integration: Calls load_decoder, create_sound_sample
Side effects: Modifies TRACK_STATE, allocates chunks

## 2. splice_multi_track

```
100: FUNCTION splice_multi_track(track_names, track_text) -> AudioResult<()>
101:   LET state = TRACK_STATE.lock()
102:
103:   // REQ-TRACK-ASSEMBLE-17: precondition
104:   IF state.track_count == 0 THEN
105:     LOG warn "splice_multi_track: no tracks exist"
106:     RETURN Err(AudioError::InvalidSample)
107:   END IF
108:
109:   // REQ-TRACK-ASSEMBLE-15: load up to MAX_MULTI_TRACKS decoders
110:   FOR (i, name) IN track_names.iter().take(MAX_MULTI_TRACKS).enumerate() DO
111:     LET decoder = load_decoder(content_dir, name, 32768, 0, 0)?
112:     LET decoded_data = decode_all(&mut decoder)?   // pre-decode all
113:
114:     LET chunk = SoundChunk {
115:       decoder: decoder,
116:       start_time: state.dec_offset,
117:       tag_me: false,
118:       track_num: state.track_count - 1,   // shares current track_num
119:       text: None,
120:       callback: None,
121:       next: None,
122:     }
123:     chunk.run_time = -3 * TEXT_SPEED   // negative = suggested minimum
124:
125:     APPEND chunk to linked list
126:     SET state.dec_offset += decoder.length() * 1000.0
127:   END FOR
128:
129:   // REQ-TRACK-ASSEMBLE-16: append text and set no_page_break
130:   IF track_text.is_some() THEN
131:     LET last_sub = unsafe { &mut *state.last_sub }
132:     last_sub.text.as_mut().map(|t| t.push_str(track_text.unwrap()))
133:   END IF
134:   SET state.no_page_break = true
135:   RETURN Ok(())
```

Validation: REQ-TRACK-ASSEMBLE-15..17

## 3. split_sub_pages

```
140: FUNCTION split_sub_pages(text) -> Vec<SubPage>
141:   LET parts = text.split("\r\n")   // REQ-TRACK-ASSEMBLE-01
142:   LET result = Vec::new()
143:
144:   FOR (i, part) IN parts.enumerate() DO
145:     LET mut page_text = part.to_string()
146:
147:     // REQ-TRACK-ASSEMBLE-03: continuation marks
148:     IF i > 0 THEN
149:       SET page_text = format!("..{}", page_text)
150:     END IF
151:     IF i < parts.len() - 1 THEN
152:       LET last_char = part.chars().last()
153:       IF last_char is not whitespace or punctuation THEN
154:         page_text.push_str("...")
155:       END IF
156:     END IF
157:
158:     // REQ-TRACK-ASSEMBLE-02: timing
159:     LET char_count = page_text.chars().count() as i32
160:     LET timestamp = max(1000, char_count * TEXT_SPEED)
161:
162:     result.push(SubPage { text: page_text, timestamp: timestamp })
163:   END FOR
164:   RETURN result
```

Validation: REQ-TRACK-ASSEMBLE-01..03

## 4. get_time_stamps

```
170: FUNCTION get_time_stamps(timestamp_str) -> Vec<u32>
171:   LET result = Vec::new()
172:   FOR token IN timestamp_str.split([',', '\r', '\n']) DO   // REQ-TRACK-ASSEMBLE-14
173:     LET trimmed = token.trim()
174:     IF let Ok(val) = trimmed.parse::<u32>() THEN
175:       IF val > 0 THEN
176:         result.push(val)
177:       END IF
178:     END IF
179:   END FOR
180:   RETURN result
```

Validation: REQ-TRACK-ASSEMBLE-14

## 5. play_track

```
190: FUNCTION play_track() -> AudioResult<()>
191:   LET state = TRACK_STATE.lock()
192:   IF state.sound_sample.is_none() THEN
193:     RETURN Ok(())   // REQ-TRACK-PLAY-02
194:   END IF
195:
196:   // REQ-TRACK-PLAY-01: compute tracks_length
197:   LET end_time = tracks_end_time(&state)
198:   state.tracks_length.store(end_time, Ordering::Release)
199:
200:   SET state.cur_chunk = state.chunks_head.as_ref().map(|c| NonNull::from(c.as_ref()))
201:   SET state.cur_sub_chunk = None
202:
203:   CALL play_stream(
204:     state.sound_sample.as_ref().unwrap().clone(),
205:     SPEECH_SOURCE,
206:     false,   // no looping
207:     true,    // scope enabled
208:     true,    // rewind
209:   )?
210:   RETURN Ok(())
```

Validation: REQ-TRACK-PLAY-01..02

## 6. stop_track

```
220: FUNCTION stop_track() -> AudioResult<()>
221:   LET state = TRACK_STATE.lock()
222:   LET source = SOURCES.sources[SPEECH_SOURCE].lock()
223:
224:   CALL stop_stream(SPEECH_SOURCE)?   // REQ-TRACK-PLAY-03
225:   SET state.track_count = 0
226:   state.tracks_length.store(0, Ordering::Release)
227:   SET state.cur_chunk = None
228:   SET state.cur_sub_chunk = None
229:   DROP source lock
230:
231:   // REQ-TRACK-PLAY-04..05: cleanup
232:   IF let Some(sample_arc) = &state.sound_sample THEN
233:     LET sample = sample_arc.lock()
234:     SET sample.decoder = None   // REQ-TRACK-PLAY-05
235:     CALL destroy_sound_sample(&mut sample)?
236:   END IF
237:   SET state.sound_sample = None
238:   SET state.chunks_head = None   // REQ-TRACK-PLAY-04: triggers recursive Drop
239:   SET state.chunks_tail = null
240:   SET state.last_sub = null
241:   SET state.dec_offset = 0.0
242:   RETURN Ok(())
```

Validation: REQ-TRACK-PLAY-03..05

## 7. jump_track

```
250: FUNCTION jump_track() -> AudioResult<()>
251:   LET state = TRACK_STATE.lock()
252:   IF state.sound_sample.is_none() THEN
253:     RETURN Ok(())   // REQ-TRACK-PLAY-07
254:   END IF
255:   LET source = SOURCES.sources[SPEECH_SOURCE].lock()
256:   LET len = state.tracks_length.load(Ordering::Acquire)
257:   CALL seek_track(&state, &source, len + 1)   // REQ-TRACK-PLAY-06
258:   RETURN Ok(())
```

Validation: REQ-TRACK-PLAY-06..07

## 8. pause_track / resume_track

```
270: FUNCTION pause_track() -> AudioResult<()>
271:   LET _source = SOURCES.sources[SPEECH_SOURCE].lock()
272:   CALL pause_stream(SPEECH_SOURCE)   // REQ-TRACK-PLAY-08
273:
274: FUNCTION resume_track() -> AudioResult<()>
275:   LET state = TRACK_STATE.lock()
276:   LET source = SOURCES.sources[SPEECH_SOURCE].lock()
277:   IF state.cur_chunk.is_none() THEN RETURN Ok(()) END IF
278:   LET mixer_state = mixer_get_source_i(source.handle, SourceProp::SourceState)
279:   IF mixer_state == Ok(SourceState::Paused as i32) THEN
280:     CALL resume_stream(SPEECH_SOURCE)   // REQ-TRACK-PLAY-09
281:   END IF
282:   RETURN Ok(())
```

Validation: REQ-TRACK-PLAY-08..09

## 9. playing_track

```
290: FUNCTION playing_track() -> u32
291:   LET state = TRACK_STATE.lock()
292:   IF state.sound_sample.is_none() THEN RETURN 0 END IF
293:   LET _source = SOURCES.sources[SPEECH_SOURCE].lock()
294:   RETURN state.cur_chunk
295:     .map(|c| unsafe { c.as_ref() }.track_num + 1)
296:     .unwrap_or(0)   // REQ-TRACK-PLAY-10
```

Validation: REQ-TRACK-PLAY-10

## 10. seek_track (internal)

```
300: FUNCTION seek_track(state, source, offset)
301:   LET len = state.tracks_length.load(Ordering::Acquire)
302:   LET clamped = offset.clamp(0, len + 1)   // REQ-TRACK-SEEK-01
303:
304:   SET source.start_time = get_time_counter() as i32 - clamped as i32   // REQ-TRACK-SEEK-02
305:
306:   // REQ-TRACK-SEEK-03: walk chunk list
307:   LET cumulative = 0
308:   LET last_tagged = None
309:   LET cur = state.chunks_head.as_ref()
310:   WHILE let Some(chunk) = cur DO
311:     IF chunk.tag_me THEN SET last_tagged = Some(chunk) END IF
312:     LET chunk_end = cumulative + chunk_duration(chunk)
313:     IF chunk_end > clamped THEN
314:       // Found target chunk
315:       LET within_chunk_ms = clamped - cumulative
316:       LET decoder = &mut chunk.decoder
317:       CALL decoder.seek(within_chunk_ms * decoder.frequency() / 1000)   // REQ-TRACK-SEEK-04
318:       SET sample.decoder = Some(borrow of chunk.decoder)
319:       SET state.cur_chunk = Some(NonNull::from(chunk))
320:       IF let Some(tagged) = last_tagged THEN
321:         CALL do_track_tag(tagged)
322:       END IF
323:       RETURN
324:     END IF
325:     SET cumulative = chunk_end
326:     SET cur = chunk.next.as_ref()
327:   END WHILE
328:
329:   // REQ-TRACK-SEEK-05: past end
330:   CALL stop_stream(SPEECH_SOURCE)
331:   SET state.cur_chunk = None
332:   SET state.cur_sub_chunk = None
```

Validation: REQ-TRACK-SEEK-01..05

## 11. Seeking Navigation

```
340: FUNCTION fast_reverse_smooth() -> AudioResult<()>
341:   LET state = TRACK_STATE.lock()
342:   LET source = SOURCES.sources[SPEECH_SOURCE].lock()   // REQ-TRACK-SEEK-13
343:   LET pos = get_current_track_pos(&state, &source)
344:   LET new_pos = pos.saturating_sub(ACCEL_SCROLL_SPEED)   // REQ-TRACK-SEEK-07
345:   CALL seek_track(&state, &source, new_pos)
346:   IF NOT source.stream_should_be_playing THEN
347:     CALL play_stream(...)   // restart
348:   END IF
349:   RETURN Ok(())
350:
351: FUNCTION fast_forward_smooth() -> AudioResult<()>
352:   LET source = SOURCES.sources[SPEECH_SOURCE].lock()
353:   LET pos = get_current_track_pos(...) + ACCEL_SCROLL_SPEED   // REQ-TRACK-SEEK-08
354:   CALL seek_track(..., pos)
355:   RETURN Ok(())
356:
357: FUNCTION fast_reverse_page() -> AudioResult<()>
358:   LET state = TRACK_STATE.lock()
359:   LET source = SOURCES.sources[SPEECH_SOURCE].lock()
360:   LET prev = find_prev_page(state.chunks_head, state.cur_sub_chunk)   // REQ-TRACK-SEEK-12
361:   IF let Some(page) = prev THEN
362:     // restart from page   // REQ-TRACK-SEEK-09
363:     CALL seek_track(...)
364:   END IF
365:   RETURN Ok(())
366:
367: FUNCTION fast_forward_page() -> AudioResult<()>
368:   LET state = TRACK_STATE.lock()
369:   LET source = SOURCES.sources[SPEECH_SOURCE].lock()
370:   LET next = find_next_page(state.cur_sub_chunk)   // REQ-TRACK-SEEK-11
371:   IF let Some(page) = next THEN
372:     CALL seek_track(...)   // REQ-TRACK-SEEK-10
373:   ELSE
374:     CALL seek_track(..., tracks_length + 1)
375:   END IF
376:   RETURN Ok(())
```

Validation: REQ-TRACK-SEEK-07..13

## 12. find_next_page / find_prev_page

```
380: FUNCTION find_next_page(cur) -> Option<NonNull<SoundChunk>>
381:   IF cur.is_none() THEN RETURN None END IF
382:   LET node = unsafe { cur.unwrap().as_ref() }
383:   LET mut ptr = node.next.as_ref()
384:   WHILE let Some(chunk) = ptr DO
385:     IF chunk.tag_me THEN
386:       RETURN Some(NonNull::from(chunk.as_ref()))   // REQ-TRACK-SEEK-11
387:     END IF
388:     SET ptr = chunk.next.as_ref()
389:   END WHILE
390:   RETURN None
391:
392: FUNCTION find_prev_page(head, cur) -> Option<NonNull<SoundChunk>>
393:   IF head.is_none() OR cur.is_none() THEN RETURN head.as_ref().map(NonNull::from) END IF
394:   LET cur_ptr = cur.unwrap().as_ptr()
395:   LET mut last_tagged = head.as_ref().map(NonNull::from)
396:   LET mut node = head.as_ref()
397:   WHILE let Some(chunk) = node DO
398:     IF ptr::eq(chunk.as_ref(), cur_ptr) THEN BREAK END IF
399:     IF chunk.tag_me THEN
400:       SET last_tagged = Some(NonNull::from(chunk.as_ref()))   // REQ-TRACK-SEEK-12
401:     END IF
402:     SET node = chunk.next.as_ref()
403:   END WHILE
404:   RETURN last_tagged
```

Validation: REQ-TRACK-SEEK-11..12

## 13. TrackCallbacks Implementation

```
410: IMPL StreamCallbacks FOR TrackCallbacks:
411:
412:   FUNCTION on_start_stream(sample) -> bool
413:     LET state = TRACK_STATE.lock()
414:     // REQ-TRACK-CALLBACK-01: verify match
415:     IF state.sound_sample.is_none() THEN RETURN false END IF
416:     IF NOT Arc::ptr_eq(sample_arc, state.sound_sample.as_ref().unwrap()) THEN RETURN false END IF
417:     IF state.cur_chunk.is_none() THEN RETURN false END IF
418:
419:     LET chunk = unsafe { state.cur_chunk.unwrap().as_ref() }
420:     // REQ-TRACK-CALLBACK-02: set decoder and offset
421:     SET sample.decoder = Some(borrow of chunk.decoder)
422:     SET sample.offset = (chunk.start_time * ONE_SECOND as f32) as i32
423:
424:     // REQ-TRACK-CALLBACK-03: tag if needed
425:     IF chunk.tag_me THEN
426:       CALL do_track_tag(chunk)
427:     END IF
428:     RETURN true
429:
430:   FUNCTION on_end_chunk(sample, buffer) -> bool
431:     LET state = TRACK_STATE.lock()
432:     // REQ-TRACK-CALLBACK-04: verify match
433:     IF NOT sample_matches(state) THEN RETURN false END IF
434:     IF state.cur_chunk.is_none() THEN RETURN false END IF
435:     LET cur = unsafe { state.cur_chunk.unwrap().as_ref() }
436:     IF cur.next.is_none() THEN RETURN false END IF
437:
438:     // REQ-TRACK-CALLBACK-05: advance
439:     LET next = cur.next.as_ref().unwrap()
440:     SET state.cur_chunk = Some(NonNull::from(next.as_ref()))
441:     SET sample.decoder = Some(borrow of next.decoder)
442:     CALL next.decoder.seek(0)   // rewind
443:
444:     // REQ-TRACK-CALLBACK-06: tag buffer
445:     IF next.tag_me THEN
446:       LET chunk_ptr = next as *const _ as usize
447:       CALL tag_buffer(sample, buffer, chunk_ptr)
448:     END IF
449:     RETURN true
450:
451:   FUNCTION on_end_stream(sample)
452:     LET state = TRACK_STATE.lock()
453:     SET state.cur_chunk = None   // REQ-TRACK-CALLBACK-07
454:     SET state.cur_sub_chunk = None
455:
456:   FUNCTION on_tagged_buffer(sample, tag)
457:     LET chunk_ptr = tag.data as *mut SoundChunk   // REQ-TRACK-CALLBACK-08
458:     CALL clear_buffer_tag(tag)
459:     LET chunk = unsafe { &*chunk_ptr }
460:     CALL do_track_tag(chunk)
```

Validation: REQ-TRACK-CALLBACK-01..08

## 14. do_track_tag

```
470: FUNCTION do_track_tag(chunk)
471:   // REQ-TRACK-CALLBACK-09
472:   IF let Some(cb) = &chunk.callback THEN
473:     cb(0)
474:   END IF
475:   LET state = TRACK_STATE.lock()  // may already be held
476:   SET state.cur_sub_chunk = Some(NonNull::from(chunk))
```

Validation: REQ-TRACK-CALLBACK-09

## 15. Position & Subtitle Queries

```
480: FUNCTION get_track_position(in_units) -> u32
481:   LET state = TRACK_STATE.lock()
482:   IF state.sound_sample.is_none() THEN RETURN 0 END IF
483:   LET len = state.tracks_length.load(Ordering::Acquire)   // REQ-TRACK-POSITION-02
484:   IF len == 0 THEN RETURN 0 END IF
485:   LET source = SOURCES.sources[SPEECH_SOURCE].lock()
486:   LET offset = get_current_track_pos(&state, &source)
487:   RETURN in_units * offset / len   // REQ-TRACK-POSITION-01
488:
489: FUNCTION get_track_subtitle() -> Option<String>
490:   LET state = TRACK_STATE.lock()
491:   IF state.sound_sample.is_none() THEN RETURN None END IF
492:   LET _source = SOURCES.sources[SPEECH_SOURCE].lock()
493:   RETURN state.cur_sub_chunk
494:     .map(|c| unsafe { c.as_ref() })
495:     .and_then(|c| c.text.clone())   // REQ-TRACK-SUBTITLE-01
496:
497: FUNCTION get_first_track_subtitle() -> Option<SubtitleRef>
498:   LET state = TRACK_STATE.lock()
499:   RETURN state.chunks_head.as_ref().map(SubtitleRef::from)   // REQ-TRACK-SUBTITLE-02
500:
501: FUNCTION get_next_track_subtitle(last_ref) -> Option<SubtitleRef>
502:   CALL find_next_page(last_ref.as_nonnull())   // REQ-TRACK-SUBTITLE-03
503:     .map(SubtitleRef::from)
504:
505: FUNCTION get_track_subtitle_text(sub_ref) -> Option<&str>
506:   LET chunk = sub_ref.as_chunk_ref()
507:   RETURN chunk.text.as_deref()   // REQ-TRACK-SUBTITLE-04
```

Validation: REQ-TRACK-POSITION-01..02, REQ-TRACK-SUBTITLE-01..04
