# Pseudocode — `sound::trackplayer`

Plan ID: `PLAN-20260225-AUDIO-HEART`

---

## 0. Data Structures

```
STRUCT SoundChunk {
  decoder: Option<Box<dyn SoundDecoder>>,   // audio decoder for this chunk
  start_time: f64,           // absolute position in track sequence (milliseconds)
  run_time: i32,             // display hint: positive = exact duration,
                             //   negative = minimum display time (abs value)
  tag_me: bool,              // whether to tag buffer for subtitle sync
  track_num: u32,            // which track this chunk belongs to (0-based)
  text: Option<String>,      // subtitle text for this chunk (if any)
  callback: Option<Box<dyn Fn(i32) + Send>>,  // per-chunk callback (first page only)
                             // FIX ISSUE-ALG-05: Must be Fn, not FnOnce — callbacks can fire
                             // multiple times when the user seeks back through the same chunk.
                             // FnOnce would panic on second invocation.
  next: Option<Box<SoundChunk>>,  // linked list — next chunk
}

// REQ-TRACK-ASSEMBLE-19: Iterative Drop to prevent stack overflow on long chunk lists
impl Drop for SoundChunk {
    fn drop(&mut self) {
        let mut next = self.next.take();
        while let Some(mut chunk) = next {
            next = chunk.next.take();
            // chunk is dropped here (only its own fields, not the list tail)
        }
    }
}

STRUCT TrackPlayerState {
  chunks_head: Option<Box<SoundChunk>>,  // linked list head (owns all chunks)
  chunks_tail: *mut SoundChunk,          // raw pointer to tail (borrowed)
  last_sub: *mut SoundChunk,             // raw pointer to last subtitle chunk
  cur_chunk: Option<NonNull<SoundChunk>>,     // current playback chunk
  cur_sub_chunk: Option<NonNull<SoundChunk>>, // current displayed subtitle chunk
  sound_sample: Option<Arc<Mutex<SoundSample>>>,  // shared sample for streaming
  track_count: u32,
  dec_offset: f64,           // accumulated offset in milliseconds
  no_page_break: bool,       // subtitle continuation flag
  tracks_length: AtomicU32,  // total track length in game ticks
  last_track_name: String,
}
```

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
37:     SET state.sound_sample = Some(Arc::new(parking_lot::Mutex::new(sample)))
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
72:       run_time: run_time,   // i32: display hint (negative = minimum display time)
73:       tag_me: true,
74:       track_num: state.track_count,
75:       text: Some(page.text.clone()),
76:       callback: IF i == 0 THEN callback.take() ELSE None,
77:       next: None,
78:     }
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

The multi-track resource is a single audio file containing multiple speech segments
concatenated end-to-end. Each segment corresponds to one element of `track_names`.
The C code loads each segment as a separate decoder with the same base resource
(large buffer, pre-decoded). This pseudocode shows the complete chunk boundary
calculation so the implementer can reproduce it exactly.

```
100: FUNCTION splice_multi_track(track_names, track_text) -> AudioResult<()>
101:   LET state = TRACK_STATE.lock()
102:
103:   // ── Step 1: Precondition check ──────────────────────────────────
104:   // REQ-TRACK-ASSEMBLE-17: at least one track must already exist
105:   // (splice_multi_track appends to an existing track sequence)
106:   IF state.track_count == 0 THEN
107:     LOG warn "splice_multi_track: no tracks exist, cannot append"
108:     RETURN Err(AudioError::InvalidSample)
109:   END IF
110:
111:   // ── Step 2: Determine how many tracks to process ────────────────
112:   // REQ-TRACK-ASSEMBLE-15: cap at MAX_MULTI_TRACKS (20)
113:   LET num_tracks = min(track_names.len(), MAX_MULTI_TRACKS)
114:   IF num_tracks == 0 THEN
115:     RETURN Ok(())   // nothing to splice
116:   END IF
117:
118:   // ── Step 3: Load each track and compute duration ──────────────────
119:   // Each track_name refers to a separate audio resource file.
120:   // Multi-track uses a large buffer (32KB) for efficient loading.
121:   //
122:   // OPTIMIZATION: Use decoder.length() to compute duration when available,
123:   // avoiding decode_all() which would decode the entire file only to
124:   // throw away the result (the streaming engine re-decodes later).
125:   // Only fall back to decode_all() when length() returns 0.0 (unknown).
126:   LET mut loaded_chunks: Vec<(Box<dyn SoundDecoder>, f64)> = Vec::new()
127:
128:   FOR i IN 0..num_tracks DO
129:     LET name = track_names[i]
130:
131:     // Load decoder for this track segment
132:     // buffer_size=32768 for large pre-decode reads
133:     LET decoder = load_decoder(content_dir, name, 32768, 0, 0)
134:     IF decoder.is_err() THEN
135:       LOG warn "splice_multi_track: failed to load track {}: {}", i, name
136:       CONTINUE   // skip unloadable tracks, don't fail the whole splice
137:     END IF
138:     LET mut decoder = decoder.unwrap()
139:
140:     // ── Step 3a: Compute chunk duration ────────────────────────
141:     // Prefer decoder.length() when available — avoids wasteful
142:     // full decode. Fall back to decode_all() only when length is
143:     // unknown (returns 0.0), which typically means VBR or streaming
144:     // format without a header length field.
145:     LET duration_secs = IF decoder.length() > 0.0 THEN
146:       decoder.length()   // use reported length (no decode needed)
147:     ELSE
148:       // Unknown length — must decode to measure byte count
149:       LET decoded_data = decode_all(&mut decoder)
150:       IF decoded_data.is_err() THEN
151:         LOG warn "splice_multi_track: failed to decode track {}: {}", i, name
152:         CONTINUE
153:       END IF
154:       LET decoded_data = decoded_data.unwrap()
          // FIX ISSUE-ALG-04: After decode_all, the decoder is at EOF. We need
          // to rewind it for the streaming engine. If seek(0) fails (non-seekable
          // decoder), recreate from scratch via load_decoder.
          IF decoder.seek(0).is_err() THEN
            LET fresh = load_decoder(content_dir, name, 32768, 0, 0)
            IF fresh.is_err() THEN
              LOG warn "splice_multi_track: seek(0) failed and reload failed for track {}", i
              CONTINUE
            END IF
            decoder = fresh.unwrap()
          END IF
155:       LET freq = decoder.frequency() as f64
156:       LET bps = decoder.format().bytes_per_sample() as f64
157:       LET channels = decoder.format().channels() as f64
158:       LET bytes_per_second = freq * bps * channels
159:       IF bytes_per_second > 0.0 THEN
160:         decoded_data.len() as f64 / bytes_per_second
161:       ELSE
162:         0.0
163:       END IF
164:       // decoded_data dropped here — streaming engine will re-decode
165:     END IF
166:
167:     loaded_chunks.push((decoder, duration_secs))
168:   END FOR
171:
172:   // ── Step 4: Build SoundChunk linked list ────────────────────────
173:   // Each loaded track becomes a SoundChunk in the linked list.
174:   // chunk.start_time = current dec_offset (accumulated ms from all
175:   //   previous tracks). This is the absolute position in the overall
176:   //   track sequence where this chunk begins.
177:   //
178:   // Chunk boundary diagram for 3 tracks of 2s, 3s, 1.5s:
179:   //
180:   //   dec_offset:  |--- existing ---|--- track A (2s) ---|--- track B (3s) ---|--- track C (1.5s) ---|
181:   //   start_time:                   ^prev_offset         ^prev_offset+2000    ^prev_offset+5000
182:   //   total added:                  6500ms
183:   //
184:   FOR (decoder, duration_secs) IN loaded_chunks.into_iter() DO
185:     // Convert duration to milliseconds for start_time calculation
186:     LET duration_ms = duration_secs * 1000.0
187:
188:     // ── Step 4a: Create the SoundChunk ────────────────────────
189:     // REQ-TRACK-ASSEMBLE-18: chunk fields
190:     LET chunk = SoundChunk {
191:       // The decoder is kept for the streaming engine to decode on
192:       // demand. Audio is NOT pre-decoded here — decoder.length()
193:       // was used to compute duration without full decode (when
194:       // available). Decoder must be rewound to pos 0 (see Step 4c).
195:       decoder: decoder,
196:       // Absolute start time in the overall track sequence (ms)
197:       start_time: state.dec_offset,
198:       // Multi-track chunks are NOT tagged for subtitle display
199:       // (they share the parent track's subtitle)
200:       tag_me: false,
201:       // All multi-track chunks share the current track number
202:       // (track_count - 1 because track_count is 1-based after splice_track)
203:       track_num: state.track_count - 1,
204:       // No subtitle text — multi-track chunks inherit from parent
205:       text: None,
206:       // No per-chunk callback for multi-track
207:       callback: None,
208:       // Linked list pointer (will be set by append)
209:       next: None,
210:     }
211:
212:     // ── Step 4b: Set run_time (display hint) ──────────────────
213:     // Negative run_time = suggested minimum display time.
214:     // -3 * TEXT_SPEED = -240ms, meaning the subtitle should stay
215:     // on screen for at least 240ms even if the audio is shorter.
216:     // The track player uses abs(run_time) when run_time < 0 as
217:     // a minimum, letting the audio length take precedence if longer.
218:     chunk.run_time = -3 * TEXT_SPEED
219:
220:     // ── Step 4c: Rewind decoder for streaming engine ──────────
221:     // The streaming engine expects the decoder at position 0 so it
222:     // can decode from the beginning when this chunk becomes active.
223:     // FIX ISSUE-ALG-04: Check seek result. For decoders that used
224:     // decode_all path, seek(0) was already handled above (with
225:     // decoder recreation fallback). For decoders that used length(),
226:     // seek(0) should succeed (they haven't been consumed).
227:     IF decoder.seek(0).is_err() THEN
228:       LOG warn "splice_multi_track: decoder seek(0) failed for chunk at offset {}", state.dec_offset
229:     END IF
224:
225:     // ── Step 4d: Append to linked list ────────────────────────
226:     // Insert at tail of the SoundChunk linked list.
227:     // If state.chunks_tail is null (shouldn't happen given precondition),
228:     // this becomes the new head.
229:     IF state.chunks_tail.is_null() THEN
230:       // Shouldn't reach here (track_count > 0 means list is non-empty)
231:       LET boxed = Box::new(chunk)
232:       state.chunks_head = Some(boxed)
233:       state.chunks_tail = state.chunks_head.as_ref()
234:         .map(|c| c.as_ref() as *const _ as *mut SoundChunk)
235:         .unwrap_or(std::ptr::null_mut())
236:     ELSE
237:       LET tail = unsafe { &mut *state.chunks_tail }
238:       LET boxed = Box::new(chunk)
239:       LET new_tail_ptr = boxed.as_ref() as *const _ as *mut SoundChunk
240:       tail.next = Some(boxed)
241:       state.chunks_tail = new_tail_ptr
242:     END IF
243:
244:     // ── Step 4e: Advance dec_offset ───────────────────────────
245:     // REQ-TRACK-ASSEMBLE-08: accumulate offset
246:     // This is the critical chunk boundary calculation:
247:     // each chunk's start_time was set to the current dec_offset,
248:     // and now we advance dec_offset by this chunk's duration.
249:     // The NEXT chunk (if any) will start at this new offset.
250:     SET state.dec_offset += duration_ms
251:   END FOR
252:
253:   // ── Step 5: Handle subtitle text continuation ──────────────────
254:   // REQ-TRACK-ASSEMBLE-16: multi-track text is appended to the
255:   // most recent subtitle chunk (the one from the preceding splice_track call).
256:   // This creates the visual effect of a continuous subtitle across
257:   // all the multi-track audio segments.
258:   IF track_text.is_some() THEN
259:     IF state.last_sub.is_null() THEN
260:       LOG warn "splice_multi_track: no subtitle chunk to append to"
261:     ELSE
262:       LET last_sub = unsafe { &mut *state.last_sub }
263:       MATCH &mut last_sub.text {
264:         Some(existing_text) => existing_text.push_str(track_text.unwrap()),
265:         None => last_sub.text = Some(track_text.unwrap().to_string()),
266:       }
267:     END IF
268:   END IF
269:
270:   // REQ-TRACK-ASSEMBLE-16: set no_page_break so the NEXT splice_track
271:   // call will append its first page to the current subtitle instead
272:   // of creating a new one. This ensures visual continuity.
273:   SET state.no_page_break = true
274:   RETURN Ok(())
```

Validation: REQ-TRACK-ASSEMBLE-15..17
Error handling: Individual track load/decode failures are logged and skipped (non-fatal)
Integration: Calls load_decoder, decode_all (stream.md §18, fallback only), modifies TRACK_STATE
Side effects: Appends 0..N chunks to linked list, advances dec_offset

### Chunk Boundary Calculation Summary

The chunk boundary algorithm works as follows:

1. **Input**: A list of `track_names`, each naming an audio resource file.
2. **Per-track processing**: Each track is loaded via `load_decoder(name, buffer=32768)`.
3. **Duration computation**: Prefer `decoder.length()` when available (avoids wasteful full decode). Fall back to `decode_all()` only when `length() == 0.0` (unknown), then compute from `decoded_bytes / (freq * bps * channels)`. Duration in ms = `duration_secs * 1000.0`.
4. **Chunk boundary**: `chunk.start_time = state.dec_offset` (the running total of all previous chunk durations in ms). After creating the chunk, `state.dec_offset += duration_ms`.
5. **Linked list**: Each chunk is appended to the tail of the `SoundChunk` linked list via `tail.next = Some(boxed_chunk)`.
6. **Seek integration**: When `seek_track()` (§10) walks the chunk list, it uses `chunk.start_time` to find which chunk contains the target position, then seeks the decoder to `(target_ms - chunk.start_time) * freq / 1000` PCM frames within that chunk.

Example with 3 tracks (existing dec_offset = 5000ms):
- Track A: 2.0s → chunk.start_time = 5000, dec_offset becomes 7000
- Track B: 3.0s → chunk.start_time = 7000, dec_offset becomes 10000
- Track C: 1.5s → chunk.start_time = 10000, dec_offset becomes 11500

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

**LOCK SAFETY (FIX: ISSUE-CONC-01)**: `play_stream` invokes `on_start_stream` callback
which acquires TRACK_STATE. We must drop TRACK_STATE before calling `play_stream` to
avoid self-deadlock (parking_lot::Mutex is not reentrant). Extract the sample Arc while
holding the lock, then drop, then call `play_stream`.

```
190: FUNCTION play_track() -> AudioResult<()>
191:   // Phase 1: Set up state under TRACK_STATE lock
192:   LET sample_arc = {
193:     LET state = TRACK_STATE.lock()
194:     IF state.sound_sample.is_none() THEN
195:       RETURN Ok(())   // REQ-TRACK-PLAY-02
196:     END IF
197:
198:     // REQ-TRACK-PLAY-01: compute tracks_length
199:     LET end_time = tracks_end_time(&state)
200:     state.tracks_length.store(end_time, Ordering::Release)
201:
202:     SET state.cur_chunk = state.chunks_head.as_ref().map(|c| NonNull::from(c.as_ref()))
203:     SET state.cur_sub_chunk = None
204:
205:     Arc::clone(state.sound_sample.as_ref().unwrap())
206:     // TRACK_STATE lock dropped here (end of block)
207:   }
208:
209:   // Phase 2: Call play_stream WITHOUT holding TRACK_STATE.
210:   // play_stream → on_start_stream can safely acquire TRACK_STATE.
211:   CALL play_stream(
212:     sample_arc,
213:     SPEECH_SOURCE,
214:     false,   // no looping
215:     true,    // scope enabled
216:     true,    // rewind
217:   )?
218:   RETURN Ok(())
```

Validation: REQ-TRACK-PLAY-01..02

## 6. stop_track

**LOCK SAFETY**: `stop_stream` acquires the Source mutex internally, so we must
NOT hold the Source mutex when calling it (parking_lot::Mutex is not reentrant).
We acquire TRACK_STATE first (lock ordering: TRACK_STATE → Source → Sample),
then call `stop_stream` which acquires Source internally, then proceed with
cleanup under TRACK_STATE only.

**MEMORY SAFETY (FIX: ISSUE-CONC-04)**: Buffer tags store raw pointers (as usize)
to SoundChunk nodes. Before dropping the chunk list (`chunks_head = None`), we must
clear all buffer tags in the sample. Otherwise, pending tags would be dangling
pointers — `on_tagged_buffer` in the decoder thread would dereference freed memory.

```
220: FUNCTION stop_track() -> AudioResult<()>
221:   LET state = TRACK_STATE.lock()

223:   // REQ-TRACK-PLAY-03: stop stream — stop_stream acquires Source
224:   // mutex internally, so we must NOT hold it here.
225:   CALL stop_stream(SPEECH_SOURCE)?

227:   SET state.track_count = 0
228:   state.tracks_length.store(0, Ordering::Release)
229:   SET state.cur_chunk = None
230:   SET state.cur_sub_chunk = None

232:   // REQ-TRACK-PLAY-04..05: cleanup
233:   IF let Some(sample_arc) = &state.sound_sample THEN
234:     LET sample = sample_arc.lock()
235:     // FIX ISSUE-CONC-04: Clear ALL buffer tags before dropping chunks.
236:     // Tags store raw pointers to SoundChunk nodes as usize. Dropping
237:     // chunks_head frees all chunks, making those pointers dangling.
238:     // Clearing tags first ensures on_tagged_buffer (decoder thread)
239:     // won't dereference freed memory.
240:     FOR tag_slot IN sample.buffer_tags.iter_mut() DO
241:       SET *tag_slot = None
242:     END FOR
243:     SET sample.decoder = None   // REQ-TRACK-PLAY-05
244:     CALL destroy_sound_sample(&mut sample)?
245:   END IF
246:   SET state.sound_sample = None
247:   SET state.chunks_head = None   // REQ-TRACK-PLAY-04: triggers iterative Drop
248:   SET state.chunks_tail = null
249:   SET state.last_sub = null
250:   SET state.dec_offset = 0.0
251:   RETURN Ok(())
```

Validation: REQ-TRACK-PLAY-03..05

## 7. jump_track

```
250: FUNCTION jump_track() -> AudioResult<()>
251:   // Extract needed state, drop locks, then call seek functions
252:   // (seek_track -> stop_stream acquires Source lock internally)
253:   LET state = TRACK_STATE.lock()
254:   IF state.sound_sample.is_none() THEN
255:     RETURN Ok(())   // REQ-TRACK-PLAY-07
256:   END IF
257:   LET len = state.tracks_length.load(Ordering::Acquire)
258:   DROP state   // drop TRACK_STATE before stop_stream
259:   CALL stop_stream(SPEECH_SOURCE)?   // REQ-TRACK-PLAY-06: jump past end stops stream
260:   // Reacquire to clear state
261:   LET state = TRACK_STATE.lock()
262:   SET state.cur_chunk = None
263:   SET state.cur_sub_chunk = None
264:   RETURN Ok(())
```

Validation: REQ-TRACK-PLAY-06..07

## 8. pause_track / resume_track

**LOCK SAFETY (FIX: ISSUE-CONC-02)**: `pause_stream` and `resume_stream` acquire the
Source lock internally. We must NOT pre-acquire it here — parking_lot::Mutex is not
reentrant, so double-locking would deadlock. For `resume_track`, we check the mixer
state by briefly locking the source, then drop it before calling `resume_stream`.

```
270: FUNCTION pause_track() -> AudioResult<()>
271:   // Do NOT lock Source here — pause_stream locks it internally
272:   CALL pause_stream(SPEECH_SOURCE)   // REQ-TRACK-PLAY-08
273:   RETURN Ok(())
274:
275: FUNCTION resume_track() -> AudioResult<()>
276:   LET state = TRACK_STATE.lock()
277:   IF state.cur_chunk.is_none() THEN RETURN Ok(()) END IF
278:   // Check mixer state under Source lock, then drop before resume_stream
279:   LET should_resume = {
280:     LET source = SOURCES.sources[SPEECH_SOURCE].lock()
281:     LET mixer_state = mixer_get_source_i(source.handle, SourceProp::SourceState)
282:     mixer_state == Ok(SourceState::Paused as i32)
283:     // source lock dropped here
284:   }
285:   IF should_resume THEN
286:     DROP state   // drop TRACK_STATE before resume_stream (lock ordering)
287:     CALL resume_stream(SPEECH_SOURCE)   // REQ-TRACK-PLAY-09
288:   END IF
289:   RETURN Ok(())
```

Validation: REQ-TRACK-PLAY-08..09

## 9. playing_track

```
290: FUNCTION playing_track() -> u32
291:   LET state = TRACK_STATE.lock()
292:   IF state.sound_sample.is_none() THEN RETURN 0 END IF
293:   // Note: cur_chunk is protected by TRACK_STATE, not Source.
294:   // Source lock removed (HIGH-CONC-05 fix) — it served no purpose here.
295:   RETURN state.cur_chunk
296:     .map(|c| unsafe { c.as_ref() }.track_num + 1)
297:     .unwrap_or(0)   // REQ-TRACK-PLAY-10
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
321:         CALL do_track_tag(state, tagged)   // pass state guard (FIX: ISSUE-MISC-03)
322:       END IF
323:       RETURN
324:     END IF
325:     SET cumulative = chunk_end
326:     SET cur = chunk.next.as_ref()
327:   END WHILE
328:
329:   // REQ-TRACK-SEEK-05: past end
330:   // Inline stop logic — Source lock already held by caller, cannot call stop_stream
331:   CALL mixer_source_stop(source.handle)
332:   SET source.stream_should_be_playing = false
333:   SET source.sample = None
334:   SET state.cur_chunk = None
335:   SET state.cur_sub_chunk = None
```

Validation: REQ-TRACK-SEEK-01..05

## 11. Seeking Navigation

```
340: FUNCTION fast_reverse_smooth() -> AudioResult<()>
341:   // LOCK SAFETY: Extract position, drop locks, then call play_stream/seek
342:   // (play_stream and seek_track->stop_stream acquire Source lock internally)
343:   LET (pos, need_restart) = {
344:     LET state = TRACK_STATE.lock()
345:     LET source = SOURCES.sources[SPEECH_SOURCE].lock()
346:     LET p = get_current_track_pos(&state, &source)
347:     LET restart = NOT source.stream_should_be_playing
348:     (p, restart)
349:     // state + source locks dropped here
350:   }
351:   LET new_pos = pos.saturating_sub(ACCEL_SCROLL_SPEED)   // REQ-TRACK-SEEK-07
352:   // Reacquire for seek_track (it needs state + source guards)
353:   LET state = TRACK_STATE.lock()
354:   LET source = SOURCES.sources[SPEECH_SOURCE].lock()
355:   CALL seek_track(&state, &source, new_pos)
356:   DROP source; DROP state   // drop before play_stream
357:   IF need_restart THEN
358:     CALL play_stream(...)   // restart (acquires Source lock internally)
359:   END IF
360:   RETURN Ok(())
361:
362: FUNCTION fast_forward_smooth() -> AudioResult<()>
363:   LET state = TRACK_STATE.lock()
364:   LET source = SOURCES.sources[SPEECH_SOURCE].lock()
365:   LET pos = get_current_track_pos(&state, &source) + ACCEL_SCROLL_SPEED   // REQ-TRACK-SEEK-08
366:   CALL seek_track(&state, &source, pos)
367:   // seek_track's stop_stream path is handled inline (no re-lock needed)
368:   RETURN Ok(())
369:
370: FUNCTION fast_reverse_page() -> AudioResult<()>
371:   LET state = TRACK_STATE.lock()
372:   LET prev = find_prev_page(state.chunks_head, state.cur_sub_chunk)   // REQ-TRACK-SEEK-12
373:   IF let Some(page) = prev THEN
374:     LET source = SOURCES.sources[SPEECH_SOURCE].lock()
375:     CALL seek_track(&state, &source, page.start_offset)   // REQ-TRACK-SEEK-09
376:   END IF
377:   RETURN Ok(())
378:
379: FUNCTION fast_forward_page() -> AudioResult<()>
380:   LET state = TRACK_STATE.lock()
381:   LET next = find_next_page(state.cur_sub_chunk)   // REQ-TRACK-SEEK-11
382:   IF let Some(page) = next THEN
383:     LET source = SOURCES.sources[SPEECH_SOURCE].lock()
384:     CALL seek_track(&state, &source, page.start_offset)   // REQ-TRACK-SEEK-10
385:   ELSE
386:     LET len = state.tracks_length.load(Ordering::Acquire)
387:     DROP state   // drop before stop_stream
388:     CALL stop_stream(SPEECH_SOURCE)?
389:   END IF
390:   RETURN Ok(())
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

**LOCK SAFETY (FIX: ISSUE-CONC-01)**: These callbacks are invoked from `play_stream`
(which does NOT hold TRACK_STATE) and from `execute_deferred_callbacks` (which holds
NO locks). Each callback acquires TRACK_STATE internally — this is safe because no
caller holds TRACK_STATE when invoking these callbacks. `do_track_tag` receives the
state guard as a parameter to avoid re-locking (see §14).

```
410: IMPL StreamCallbacks FOR TrackCallbacks:
411:
412:   FUNCTION on_start_stream(sample) -> bool
413:     LET mut state = TRACK_STATE.lock()
414:     // REQ-TRACK-CALLBACK-01: verify match
415:     IF state.sound_sample.is_none() THEN RETURN false END IF
416:     IF NOT Arc::ptr_eq(sample_arc, state.sound_sample.as_ref().unwrap()) THEN RETURN false END IF
417:     IF state.cur_chunk.is_none() THEN RETURN false END IF
418:
419:     LET chunk = unsafe { state.cur_chunk.unwrap().as_ref() }
420:     // REQ-TRACK-CALLBACK-02: set decoder and offset
421:     SET sample.decoder = Some(borrow of chunk.decoder)
422:     // Unit conversion: start_time is in milliseconds, ONE_SECOND (840) is
423:     // game ticks per second. offset must be in game ticks.
424:     // Formula: offset = start_time_ms * ONE_SECOND / 1000
425:     SET sample.offset = (chunk.start_time * ONE_SECOND as f32 / 1000.0) as i32
426:
427:     // REQ-TRACK-CALLBACK-03: tag if needed
428:     IF chunk.tag_me THEN
429:       CALL do_track_tag(&mut state, chunk)   // pass guard, no re-lock
430:     END IF
431:     RETURN true
432:
433:   FUNCTION on_end_chunk(sample, buffer) -> bool
434:     LET mut state = TRACK_STATE.lock()
435:     // REQ-TRACK-CALLBACK-04: verify match
436:     IF NOT sample_matches(state) THEN RETURN false END IF
437:     IF state.cur_chunk.is_none() THEN RETURN false END IF
438:     LET cur = unsafe { state.cur_chunk.unwrap().as_ref() }
439:     IF cur.next.is_none() THEN RETURN false END IF
440:
441:     // REQ-TRACK-CALLBACK-05: advance
442:     LET next = cur.next.as_ref().unwrap()
443:     SET state.cur_chunk = Some(NonNull::from(next.as_ref()))
444:     SET sample.decoder = Some(borrow of next.decoder)
445:     CALL next.decoder.seek(0)   // rewind
446:
447:     // REQ-TRACK-CALLBACK-06: tag buffer
448:     IF next.tag_me THEN
449:       LET chunk_ptr = next as *const _ as usize
450:       CALL tag_buffer(sample, buffer, chunk_ptr)
451:     END IF
452:     RETURN true
453:
454:   FUNCTION on_end_stream(sample)
455:     LET mut state = TRACK_STATE.lock()
456:     SET state.cur_chunk = None   // REQ-TRACK-CALLBACK-07
457:     SET state.cur_sub_chunk = None
458:
459:   FUNCTION on_tagged_buffer(sample, tag)
460:     LET chunk_ptr = tag.data as *mut SoundChunk   // REQ-TRACK-CALLBACK-08
461:     CALL clear_buffer_tag(tag)
462:     // Acquire TRACK_STATE for do_track_tag (no locks held in deferred context)
463:     LET mut state = TRACK_STATE.lock()
464:     // FIX ISSUE-CONC-04 (defense-in-depth): Validate chunk pointer is still
465:     // in the active chunk list before dereferencing. If stop_track cleared
466:     // chunks_head, the pointer may be dangling. stop_track also clears all
467:     // buffer tags (primary defense), but this check protects against races.
468:     IF state.chunks_head.is_none() THEN RETURN END IF
469:     IF NOT chunk_is_in_list(&state.chunks_head, chunk_ptr) THEN
470:       LOG warn "on_tagged_buffer: stale chunk pointer, skipping"
471:       RETURN
472:     END IF
473:     LET chunk = unsafe { &*chunk_ptr }
474:     CALL do_track_tag(&mut state, chunk)   // pass guard, no re-lock
```

Validation: REQ-TRACK-CALLBACK-01..08

## 14. do_track_tag

**LOCK SAFETY (FIX: ISSUE-MISC-03 / ISSUE-CONC-01)**: `do_track_tag` is called from
`on_start_stream` and `on_end_chunk` (which already hold TRACK_STATE), and from
`on_tagged_buffer` (deferred context, no locks held). To avoid self-deadlock, this
function accepts a `&mut TrackPlayerState` guard instead of re-locking TRACK_STATE.
Callers are responsible for passing the guard they already hold.

```
470: FUNCTION do_track_tag(state: &mut TrackPlayerState, chunk)
471:   // REQ-TRACK-CALLBACK-09
472:   IF let Some(cb) = &chunk.callback THEN
473:     cb(0)
474:   END IF
475:   SET state.cur_sub_chunk = Some(NonNull::from(chunk))
```

Validation: REQ-TRACK-CALLBACK-09

## 15. SubtitleRef Type

`SubtitleRef` is a zero-allocation wrapper around `NonNull<SoundChunk>` — a raw pointer
into the chunk linked list. It performs NO heap allocation. The FFI layer casts it
directly to/from `*mut c_void`. The pointed-to chunk is valid as long as the track
state is not modified (no splice/stop between iteration calls). This matches C behavior
exactly: the C code returns `TFB_TrackChunk*` pointers into the linked list.

```
STRUCT SubtitleRef(NonNull<SoundChunk>)

IMPL SubtitleRef:
  FUNCTION from_chunk_ptr(ptr: NonNull<SoundChunk>) -> SubtitleRef
    SubtitleRef(ptr)

  FUNCTION as_chunk_ref(&self) -> &SoundChunk
    unsafe { self.0.as_ref() }

  FUNCTION as_ptr(&self) -> *mut SoundChunk
    self.0.as_ptr()
```

## 16. Position & Subtitle Queries

```
480: FUNCTION get_track_position(in_units) -> u32
481:   LET state = TRACK_STATE.lock()
482:   IF state.sound_sample.is_none() THEN RETURN 0 END IF
483:   LET len = state.tracks_length.load(Ordering::Acquire)   // REQ-TRACK-POSITION-02
484:   IF len == 0 THEN RETURN 0 END IF
485:   LET source = SOURCES.sources[SPEECH_SOURCE].lock()
486:   LET offset = get_current_track_pos(&state, &source)
487:   RETURN (in_units as u64 * offset as u64 / len as u64) as u32   // REQ-TRACK-POSITION-01 (u64 prevents overflow)
488:
489: FUNCTION get_track_subtitle() -> Option<String>
490:   LET state = TRACK_STATE.lock()
491:   IF state.sound_sample.is_none() THEN RETURN None END IF
492:   LET _source = SOURCES.sources[SPEECH_SOURCE].lock()
493:   RETURN state.cur_sub_chunk
494:     .map(|c| unsafe { c.as_ref() })
495:     .and_then(|c| c.text.clone())   // REQ-TRACK-SUBTITLE-01
496:
497: FUNCTION get_first_track_subtitle() -> Option<NonNull<SoundChunk>>
498:   // REQ-TRACK-SUBTITLE-02: returns raw pointer into chunk list (zero allocation).
499:   // The returned pointer is BORROWED — valid only while track state is unchanged
500:   // (no splice_track/stop_track between calls). Caller must NOT free.
501:   LET state = TRACK_STATE.lock()
502:   RETURN state.chunks_head.as_ref()
503:     .map(|boxed| NonNull::from(boxed.as_ref()))
504:
505: FUNCTION get_next_track_subtitle(chunk_ptr: NonNull<SoundChunk>) -> Option<NonNull<SoundChunk>>
506:   // REQ-TRACK-SUBTITLE-03: walks linked list to find next tagged chunk.
507:   // Returns raw pointer (zero allocation). Same lifetime rules as above.
508:   CALL find_next_page(Some(chunk_ptr))
509:
510: FUNCTION get_track_subtitle_text(chunk_ptr: NonNull<SoundChunk>) -> Option<&str>
511:   // REQ-TRACK-SUBTITLE-04: returns borrowed reference to chunk's text.
512:   LET chunk = unsafe { chunk_ptr.as_ref() }
513:   RETURN chunk.text.as_deref()
```

Validation: REQ-TRACK-POSITION-01..02, REQ-TRACK-SUBTITLE-01..04
