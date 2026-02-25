# Pseudocode — `sound::heart_ffi`

Plan ID: `PLAN-20260225-AUDIO-HEART`

---

## Design Principle

Every FFI function is a thin shim: convert C types → Rust types, call the Rust API, convert results → C types. No logic beyond pointer conversion and error translation. All functions are `#[no_mangle] pub extern "C" fn`.

## Error Convention (REQ-CROSS-GENERAL-08)

- Functions returning `bool` → `c_int`: 1 for success, 0 for failure
- Functions returning counts → `i32`: 0 for failure
- Functions returning pointers → null for failure
- Internal `Result` errors logged before conversion

## Thread-Local CString Caches (FIX: ISSUE-FFI-01)

FFI functions returning `*const c_char` use thread-local `RefCell<CString>` caches to
keep the returned pointer valid until the next call. **Separate** caches are used for
each function family to prevent one call from invalidating another's pointer:

```
thread_local! {
    // Used by GetTrackSubtitle()
    static SUBTITLE_CACHE: RefCell<CString> = RefCell::new(CString::default());
    // Used by GetTrackSubtitleText()
    static SUBTITLE_TEXT_CACHE: RefCell<CString> = RefCell::new(CString::default());
}

FUNCTION cache_and_return_c_str_subtitle(text) -> *const c_char
  SUBTITLE_CACHE.with(|cache| { cache.replace(CString::new(text).unwrap_or_default()); cache.borrow().as_ptr() })

FUNCTION cache_and_return_c_str_text(text) -> *const c_char
  SUBTITLE_TEXT_CACHE.with(|cache| { cache.replace(CString::new(text).unwrap_or_default()); cache.borrow().as_ptr() })
```

---

## 1. Stream FFI Functions

```
01: FUNCTION InitStreamDecoder() -> c_int
02:   MATCH init_stream_decoder() {
03:     Ok(()) => 0,       // success = 0 (matching C convention)
04:     Err(e) => { LOG error "{}", e; -1 }
05:   }
06:
07: FUNCTION UninitStreamDecoder()
08:   LET _ = uninit_stream_decoder()   // ignore errors on shutdown
09:
10: FUNCTION TFB_CreateSoundSample(decoder_ptr, num_buffers, callbacks_ptr) -> *mut SoundSample
11:   // Convert decoder pointer
12:   // SAFETY (FIX: ISSUE-FFI-02): decoder_ptr MUST have been created by
13:   // Box::into_raw() on a Box<dyn SoundDecoder> (produced by Rust's
14:   // load_decoder or a decoder constructor). The fat pointer layout (data +
15:   // vtable) must match exactly. Passing a pointer not created by
16:   // Box::into_raw (e.g., a C-allocated pointer) is undefined behavior.
17:   LET decoder = IF decoder_ptr.is_null() THEN None
18:     ELSE Some(unsafe { Box::from_raw(decoder_ptr as *mut dyn SoundDecoder) })
14:   // Convert callbacks
15:   LET callbacks = convert_c_callbacks(callbacks_ptr)
16:   MATCH create_sound_sample(decoder, num_buffers, callbacks) {
17:     Ok(sample) => Box::into_raw(Box::new(sample)),
18:     Err(e) => { LOG error; null_mut() }
19:   }
20:
21: FUNCTION TFB_DestroySoundSample(sample_ptr)
22:   IF sample_ptr.is_null() THEN RETURN END IF
23:   LET sample = unsafe { &mut *sample_ptr }
24:   LET _ = destroy_sound_sample(sample)
25:
26: FUNCTION TFB_SetSoundSampleData(sample_ptr, data_ptr)
27:   IF sample_ptr.is_null() THEN RETURN END IF
28:   LET sample = unsafe { &mut *sample_ptr }
29:   LET data = Box::new(data_ptr as usize)   // store raw pointer as opaque
30:   set_sound_sample_data(sample, data)
31:
32: FUNCTION TFB_GetSoundSampleData(sample_ptr) -> *mut c_void
33:   // LIFETIME: returned pointer is BORROWED — valid only while the
34:   // SoundSample exists. Caller must NOT free it.
35:   IF sample_ptr.is_null() THEN RETURN null END IF
36:   LET sample = unsafe { &*sample_ptr }
37:   MATCH get_sound_sample_data(sample) {
38:     Some(data) => /* extract pointer */ data as *mut c_void,
39:     None => null_mut()
40:   }
39:
40: FUNCTION TFB_SetSoundSampleCallbacks(sample_ptr, callbacks_ptr)
41:   IF sample_ptr.is_null() THEN RETURN END IF
42:   LET sample = unsafe { &mut *sample_ptr }
43:   LET callbacks = convert_c_callbacks(callbacks_ptr)
44:   set_sound_sample_callbacks(sample, callbacks)
45:
46: FUNCTION TFB_GetSoundSampleDecoder(sample_ptr) -> *mut c_void
47:   // LIFETIME: returned pointer is BORROWED — valid only while the
48:   // SoundSample exists and its decoder is not replaced. Caller must NOT free it.
49:   IF sample_ptr.is_null() THEN RETURN null END IF
50:   LET sample = unsafe { &*sample_ptr }
51:   MATCH get_sound_sample_decoder(sample) {
52:     Some(dec) => dec as *const _ as *mut c_void,
53:     None => null_mut()
54:   }
53:
54: FUNCTION PlayStream(sample_ptr, source, looping, scope, rewind)
55:   IF sample_ptr.is_null() THEN RETURN END IF
56:   // FIX ISSUE-FFI-03: Define Arc lifecycle at FFI boundary.
57:   // sample_ptr was created by Arc::into_raw() (e.g., in get_music_data or
58:   // the track player's sound_sample). We increment the strong count and
59:   // create a new Arc handle WITHOUT taking ownership (the original owner
60:   // still holds its reference). This is the standard Arc FFI pattern:
61:   //   Arc::into_raw → pass to C → Arc::increment_strong_count + Arc::from_raw
62:   //
63:   // SAFETY: sample_ptr must have been created by Arc::into_raw on an
64:   // Arc<parking_lot::Mutex<SoundSample>>. The pointer is a fat pointer for
65:   // Mutex<SoundSample> (not a trait object), so it's a thin pointer — safe.
66:   LET sample_arc = unsafe {
67:     Arc::increment_strong_count(sample_ptr as *const parking_lot::Mutex<SoundSample>)
68:     Arc::from_raw(sample_ptr as *const parking_lot::Mutex<SoundSample>)
69:   }
70:   LET _ = play_stream(sample_arc, source as usize, looping != 0, scope != 0, rewind != 0)
58:
59: FUNCTION StopStream(source)
60:   LET _ = stop_stream(source as usize)
61:
62: FUNCTION PauseStream(source)
63:   LET _ = pause_stream(source as usize)
64:
65: FUNCTION ResumeStream(source)
66:   LET _ = resume_stream(source as usize)
67:
68: FUNCTION SeekStream(source, pos)
69:   LET _ = seek_stream(source as usize, pos)
70:
71: FUNCTION PlayingStream(source) -> c_int
72:   IF playing_stream(source as usize) THEN 1 ELSE 0
73:
74: FUNCTION TFB_FindTaggedBuffer(sample_ptr, buffer) -> *mut SoundTag
75:   // LIFETIME: returned pointer is BORROWED — valid only while the
76:   // SoundSample exists and the buffer tag is not cleared. Caller must NOT free it.
77:   IF sample_ptr.is_null() THEN RETURN null END IF
78:   LET sample = unsafe { &*sample_ptr }
79:   MATCH find_tagged_buffer(sample, buffer) {
80:     Some(tag) => tag as *const _ as *mut SoundTag,
79:     None => null_mut()
80:   }
81:
82: FUNCTION TFB_TagBuffer(sample_ptr, buffer, data) -> c_int
83:   IF sample_ptr.is_null() THEN RETURN 0 END IF
84:   LET sample = unsafe { &mut *sample_ptr }
85:   IF tag_buffer(sample, buffer, data as usize) THEN 1 ELSE 0
86:
87: FUNCTION TFB_ClearBufferTag(tag_ptr)
88:   IF tag_ptr.is_null() THEN RETURN END IF
89:   // Set containing Option to None (requires knowing the containing slot)
90:
91: FUNCTION SetMusicStreamFade(how_long, end_volume) -> c_int
92:   IF set_music_stream_fade(how_long as u32, end_volume) THEN 1 ELSE 0
93:
94: FUNCTION GraphForegroundStream(data_ptr: *mut i32, width: u32, height: u32, want_speech: c_int) -> u32
95:   // FFI boundary: width/height are u32 (matching C uint32_t), data is *mut i32 (C int32_t).
96:   // Internally graph_foreground_stream uses usize; convert at boundary.
97:   IF data_ptr.is_null() OR width == 0 OR height == 0 THEN RETURN 0 END IF
98:   LET slice = unsafe { std::slice::from_raw_parts_mut(data_ptr, width as usize) }
99:   graph_foreground_stream(slice, width as usize, height as usize, want_speech != 0) as u32
```

## 2. Track Player FFI Functions

```
100: FUNCTION SpliceTrack(track_name_ptr, track_text_ptr: *const u16, timestamp_ptr, callback_ptr)
101:   LET name = c_str_to_option(track_name_ptr)
102:   // track_text is UNICODE* (UCS-2/UTF-16LE) in C. Convert to UTF-8 String.
103:   // Read u16 values until null terminator, then String::from_utf16_lossy().
104:   LET text = utf16_ptr_to_option(track_text_ptr)   // *const u16 → Option<String>
103:   LET timestamp = c_str_to_option(timestamp_ptr)
104:   // FIX ISSUE-ALG-05: Callback is Fn (not FnOnce) — can fire multiple times on seek
105:   LET callback: Option<Box<dyn Fn(i32) + Send>> = IF callback_ptr.is_some() THEN
106:     Some(Box::new(move |val: i32| unsafe { callback_ptr.unwrap()(val as c_int) }))
107:   ELSE None
107:   LET _ = splice_track(name, text, timestamp, callback)
108:
109: FUNCTION SpliceMultiTrack(track_names_ptr, track_text_ptr)
110:   IF track_names_ptr.is_null() THEN RETURN END IF
111:   LET names = c_str_array_to_vec(track_names_ptr)   // NULL-terminated array
111:   LET text = c_str_to_option(track_text_ptr)
112:   LET name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect()
113:   LET _ = splice_multi_track(&name_refs, text)
114:
115: FUNCTION PlayTrack()
116:   LET _ = play_track()
117:
118: FUNCTION StopTrack()
119:   LET _ = stop_track()
120:
121: FUNCTION JumpTrack()
122:   LET _ = jump_track()
123:
124: FUNCTION PauseTrack()
125:   LET _ = pause_track()
126:
127: FUNCTION ResumeTrack()
128:   LET _ = resume_track()
129:
130: FUNCTION PlayingTrack() -> c_int
131:   playing_track() as c_int
132:
133: FUNCTION GetTrackPosition(in_units) -> i32
134:   get_track_position(in_units as u32) as i32
135:
136: FUNCTION GetTrackSubtitle() -> *const c_char
137:   // FIX ISSUE-FFI-01: Use a SEPARATE thread-local cache from GetTrackSubtitleText.
138:   // C code may hold a pointer from GetTrackSubtitle while calling GetTrackSubtitleText.
139:   // Using the same cache would invalidate the first pointer.
140:   MATCH get_track_subtitle() {
141:     Some(text) => cache_and_return_c_str_subtitle(text),  // dedicated cache
142:     None => null()
143:   }
142:
143: FUNCTION GetFirstTrackSubtitle() -> *mut c_void
144:   // LIFETIME: Returns a raw pointer into the chunk linked list — NO allocation.
145:   // Matches C behavior: the C code returns TFB_TrackChunk* directly.
146:   // Pointer is BORROWED — valid only while track state is unchanged
147:   // (no splice_track/stop_track/play_track between calls).
148:   // Caller must NOT free the returned pointer.
149:   MATCH get_first_track_subtitle() {
150:     Some(chunk_ptr) => chunk_ptr.as_ptr() as *mut c_void,
151:     None => null_mut()
152:   }
153:
154: FUNCTION GetNextTrackSubtitle(last_ref_ptr) -> *mut c_void
155:   // LIFETIME: Same as GetFirstTrackSubtitle — borrowed pointer, no allocation.
156:   IF last_ref_ptr.is_null() THEN RETURN null_mut() END IF
157:   LET chunk_ptr = unsafe { NonNull::new_unchecked(last_ref_ptr as *mut SoundChunk) }
158:   MATCH get_next_track_subtitle(chunk_ptr) {
159:     Some(next_ptr) => next_ptr.as_ptr() as *mut c_void,
160:     None => null_mut()
161:   }
162:
163: FUNCTION GetTrackSubtitleText(sub_ref_ptr) -> *const c_char
164:   // LIFETIME: returned CString is thread-local cached — valid until the
165:   // next call to GetTrackSubtitleText on the same thread.
166:   // FIX ISSUE-FFI-01: Uses cache_and_return_c_str_text (SEPARATE cache from
167:   // GetTrackSubtitle's cache_and_return_c_str_subtitle). This ensures calling
168:   // GetTrackSubtitleText does NOT invalidate a pointer from GetTrackSubtitle.
165:   IF sub_ref_ptr.is_null() THEN RETURN null() END IF
166:   LET chunk_ptr = unsafe { NonNull::new_unchecked(sub_ref_ptr as *mut SoundChunk) }
167:   MATCH get_track_subtitle_text(chunk_ptr) {
168:     Some(text) => cache_and_return_c_str_text(text),  // dedicated cache (FIX ISSUE-FFI-01)
169:     None => null()
170:   }
164:
165: FUNCTION FastReverse_Smooth()
166:   LET _ = fast_reverse_smooth()
167:
168: FUNCTION FastForward_Smooth()
169:   LET _ = fast_forward_smooth()
170:
171: FUNCTION FastReverse_Page()
172:   LET _ = fast_reverse_page()
173:
174: FUNCTION FastForward_Page()
175:   LET _ = fast_forward_page()
```

## 3. Music FFI Functions

```
180: FUNCTION PLRPlaySong(music_ref_ptr, continuous, priority)
181:   IF music_ref_ptr.is_null() THEN RETURN END IF
182:   // FIX ISSUE-FFI-05: Reconstruct MusicRef from Arc raw pointer.
183:   // Clone the Arc (increment refcount) — don't take ownership.
184:   LET music_ref = unsafe { reconstruct_music_ref_borrowed(music_ref_ptr) }
185:   LET _ = plr_play_song(music_ref, continuous != 0, priority)
186:
187: FUNCTION PLRStop(music_ref_ptr)
188:   IF music_ref_ptr.is_null() THEN RETURN END IF
189:   LET music_ref = unsafe { reconstruct_music_ref_borrowed(music_ref_ptr) }
190:   LET _ = plr_stop(music_ref)
191:
192: FUNCTION PLRPlaying(music_ref_ptr) -> c_int
193:   IF music_ref_ptr.is_null() THEN RETURN 0 END IF
194:   LET music_ref = unsafe { reconstruct_music_ref_borrowed(music_ref_ptr) }
195:   IF plr_playing(music_ref) THEN 1 ELSE 0
196:
197: FUNCTION PLRSeek(music_ref_ptr, pos)
198:   IF music_ref_ptr.is_null() THEN RETURN END IF
199:   LET music_ref = unsafe { reconstruct_music_ref_borrowed(music_ref_ptr) }
200:   LET _ = plr_seek(music_ref, pos)
201:
202: FUNCTION PLRPause(music_ref_ptr)
203:   IF music_ref_ptr.is_null() THEN RETURN END IF
204:   LET music_ref = unsafe { reconstruct_music_ref_borrowed(music_ref_ptr) }
205:   LET _ = plr_pause(music_ref)
206:
207: FUNCTION PLRResume(music_ref_ptr)
208:   IF music_ref_ptr.is_null() THEN RETURN END IF
209:   LET music_ref = unsafe { reconstruct_music_ref_borrowed(music_ref_ptr) }
210:   LET _ = plr_resume(music_ref)
204:
205: FUNCTION snd_PlaySpeech(speech_ref_ptr)
206:   IF speech_ref_ptr.is_null() THEN RETURN END IF
207:   LET speech_ref = unsafe { reconstruct_music_ref_borrowed(speech_ref_ptr) }
208:   LET _ = snd_play_speech(speech_ref)
209:
210: FUNCTION snd_StopSpeech()
211:   LET _ = snd_stop_speech()
212:
213: FUNCTION SetMusicVolume(volume)
214:   set_music_volume(volume as i32)
215:
216: FUNCTION FadeMusic(end_vol, time_interval) -> u32
217:   fade_music(end_vol as i32, time_interval as i32)
218:
219: FUNCTION DestroyMusic(music_ref_ptr)
220:   IF music_ref_ptr.is_null() THEN RETURN END IF
221:   // FIX ISSUE-FFI-05: Take ownership (decrement refcount) — this is the
222:   // release path. Arc::from_raw without increment_strong_count.
223:   LET music_ref = unsafe { reconstruct_music_ref_owned(music_ref_ptr) }
224:   LET _ = release_music_data(music_ref)   // REQ-MUSIC-RELEASE-04
```

## 4. SFX FFI Functions

```
230: FUNCTION PlayChannel(channel, snd_ptr, sound_index, pos, positional_object_ptr, priority)
231:   IF snd_ptr.is_null() THEN RETURN END IF
232:   LET bank = unsafe { &*(snd_ptr as *const SoundBank) }
233:   // sound_index selects which sample within the bank to play.
234:   // The C caller resolves SOUND handle → (bank_ptr, index) before calling.
235:   LET _ = play_channel(channel as usize, bank, sound_index as usize, pos, positional_object_ptr as usize, priority)
235:
236: FUNCTION StopChannel(channel, priority)
237:   LET _ = stop_channel(channel as usize, priority)
238:
239: FUNCTION ChannelPlaying(channel) -> c_int
240:   IF channel_playing(channel as usize) THEN 1 ELSE 0
241:
242: FUNCTION SetChannelVolume(channel, volume, priority)
243:   set_channel_volume(channel as usize, volume, priority)
244:
245: FUNCTION UpdateSoundPosition(channel, pos)
246:   update_sound_position(channel as usize, pos)
247:
248: FUNCTION GetPositionalObject(channel) -> *mut c_void
249:   get_positional_object(channel as usize) as *mut c_void
250:
251: FUNCTION SetPositionalObject(channel, obj_ptr)
252:   set_positional_object(channel as usize, obj_ptr as usize)
253:
254: FUNCTION DestroySound(snd_ptr)
255:   IF snd_ptr.is_null() THEN RETURN END IF
256:   LET bank = unsafe { Box::from_raw(snd_ptr as *mut SoundBank) }
257:   LET _ = release_sound_bank_data(*bank)   // REQ-SFX-RELEASE-04
```

## 5. Sound Control FFI Functions

```
260: FUNCTION StopSound()
261:   stop_sound()
262:
263: FUNCTION SoundPlaying() -> c_int
264:   IF sound_playing() THEN 1 ELSE 0
265:
266: FUNCTION WaitForSoundEnd(channel)
267:   LET ch = IF channel < 0 THEN None ELSE Some(channel as usize)
268:   wait_for_sound_end(ch)
269:
270: FUNCTION SetSFXVolume(volume: c_int)
271:   set_sfx_volume(volume as i32)
272:
273: FUNCTION SetSpeechVolume(volume: c_int)
274:   set_speech_volume(volume as i32)
275:
276: FUNCTION InitSound(argc: c_int, argv: *const *const c_char) -> c_int
277:   // argc and argv are vestigial — the C API accepted them but never used them.
278:   // We accept them for signature compatibility but ignore them.
279:   LET _ = (argc, argv);   // suppress unused warnings
280:   MATCH init_sound() {
281:     Ok(()) => 1,
282:     Err(e) => { LOG error; 0 }
283:   }
281:
282: FUNCTION UninitSound()
283:   uninit_sound()
```

## 6. File Loading FFI Functions

```
290: FUNCTION LoadSoundFile(filename_ptr) -> *mut c_void
291:   IF filename_ptr.is_null() THEN RETURN null_mut() END IF
292:   LET filename = unsafe { CStr::from_ptr(filename_ptr) }.to_str().unwrap_or("")
293:   MATCH load_sound_file(filename) {
294:     Ok(bank) => Box::into_raw(Box::new(bank)) as *mut c_void,
295:     Err(e) => { LOG error "{}", e; null_mut() }
296:   }
297:
298: FUNCTION LoadMusicFile(filename_ptr) -> *mut c_void
299:   IF filename_ptr.is_null() THEN RETURN null_mut() END IF
300:   LET filename = unsafe { CStr::from_ptr(filename_ptr) }.to_str().unwrap_or("")
301:   MATCH load_music_file(filename) {
302:     Ok(music_ref) => Arc::into_raw(music_ref.0) as *mut c_void,  // into_raw prevents refcount decrement
303:     Err(e) => { LOG error "{}", e; null_mut() }
304:   }
```

## 7. C Callback Wrapper

```
310: STRUCT CCallbackWrapper {
311:   callbacks: TFB_SoundCallbacks_C,
312: }
313: // SAFETY: CCallbackWrapper implements Send because C function pointers are
314: // just addresses and are inherently Send. HOWEVER, the C functions themselves
315: // may not be thread-safe — they could access thread-local storage or non-atomic
316: // globals. These callbacks will be invoked from the DECODER THREAD, not the
317: // main thread. In practice, the track player uses pure-Rust TrackCallbacks,
318: // and C code rarely creates samples with callbacks directly. If C callbacks
319: // ever assume main-thread execution, this would be a threading bug.
320: // This hazard should be documented in the FFI header comments.
313:
314: IMPL StreamCallbacks FOR CCallbackWrapper:
315:   FUNCTION on_start_stream(sample) -> bool
316:     IF let Some(f) = self.callbacks.on_start_stream THEN
317:       unsafe { f(sample as *mut SoundSample) != 0 }
318:     ELSE true
319:
320:   FUNCTION on_end_chunk(sample, buffer) -> bool
321:     IF let Some(f) = self.callbacks.on_end_chunk THEN
322:       unsafe { f(sample as *mut SoundSample, buffer) != 0 }
323:     ELSE false
324:
325:   FUNCTION on_end_stream(sample)
326:     IF let Some(f) = self.callbacks.on_end_stream THEN
327:       unsafe { f(sample as *mut SoundSample) }
328:
329:   FUNCTION on_tagged_buffer(sample, tag)
330:     IF let Some(f) = self.callbacks.on_tagged_buffer THEN
331:       unsafe { f(sample as *mut SoundSample, tag as *mut SoundTag) }
332:
333:   FUNCTION on_queue_buffer(sample, buffer)
334:     IF let Some(f) = self.callbacks.on_queue_buffer THEN
335:       unsafe { f(sample as *mut SoundSample, buffer) }
336:
337: FUNCTION convert_c_callbacks(ptr) -> Option<Box<dyn StreamCallbacks + Send>>
338:   IF ptr.is_null() THEN RETURN None END IF
339:   LET callbacks = unsafe { *ptr }
340:   Some(Box::new(CCallbackWrapper { callbacks }))
```

Validation: REQ-CROSS-FFI-01..04, REQ-CROSS-GENERAL-03, REQ-CROSS-GENERAL-08

## 8. MusicRef FFI Helpers (FIX: ISSUE-FFI-03 + ISSUE-FFI-05)

These helpers manage the Arc lifecycle at the FFI boundary. C code stores
opaque `*mut c_void` handles that are actually `Arc::into_raw` pointers.

```
// Reconstruct a MusicRef from a raw pointer WITHOUT taking ownership.
// Increments Arc strong count so the original reference remains valid.
// Use for: PLRPlaySong, PLRStop, PLRPlaying, PLRSeek, PLRPause, PLRResume, snd_PlaySpeech
FUNCTION reconstruct_music_ref_borrowed(ptr: *mut c_void) -> MusicRef
  // SAFETY: ptr was created by Arc::into_raw on Arc<Mutex<SoundSample>>.
  // We increment the strong count first, then create a new Arc handle.
  // When this MusicRef is dropped, it decrements the count back.
  LET typed_ptr = ptr as *const parking_lot::Mutex<SoundSample>
  unsafe { Arc::increment_strong_count(typed_ptr) }
  LET arc = unsafe { Arc::from_raw(typed_ptr) }
  MusicRef(arc)

// Reconstruct a MusicRef from a raw pointer, TAKING ownership.
// Does NOT increment the strong count — the caller's reference is consumed.
// Use for: DestroyMusic (release_music_data)
FUNCTION reconstruct_music_ref_owned(ptr: *mut c_void) -> MusicRef
  // SAFETY: ptr was created by Arc::into_raw. This Arc::from_raw call
  // reclaims the reference that was "leaked" to C. When MusicRef is dropped,
  // the strong count decrements. If this was the last reference, the
  // SoundSample is dropped.
  LET typed_ptr = ptr as *const parking_lot::Mutex<SoundSample>
  LET arc = unsafe { Arc::from_raw(typed_ptr) }
  MusicRef(arc)
```

Validation: REQ-MUSIC-RELEASE-01..04, REQ-CROSS-FFI-02
