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
12:   LET decoder = IF decoder_ptr.is_null() THEN None
13:     ELSE Some(unsafe { Box::from_raw(decoder_ptr as *mut dyn SoundDecoder) })
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
33:   IF sample_ptr.is_null() THEN RETURN null END IF
34:   LET sample = unsafe { &*sample_ptr }
35:   MATCH get_sound_sample_data(sample) {
36:     Some(data) => /* extract pointer */ data as *mut c_void,
37:     None => null_mut()
38:   }
39:
40: FUNCTION TFB_SetSoundSampleCallbacks(sample_ptr, callbacks_ptr)
41:   IF sample_ptr.is_null() THEN RETURN END IF
42:   LET sample = unsafe { &mut *sample_ptr }
43:   LET callbacks = convert_c_callbacks(callbacks_ptr)
44:   set_sound_sample_callbacks(sample, callbacks)
45:
46: FUNCTION TFB_GetSoundSampleDecoder(sample_ptr) -> *mut c_void
47:   IF sample_ptr.is_null() THEN RETURN null END IF
48:   LET sample = unsafe { &*sample_ptr }
49:   MATCH get_sound_sample_decoder(sample) {
50:     Some(dec) => dec as *const _ as *mut c_void,
51:     None => null_mut()
52:   }
53:
54: FUNCTION PlayStream(sample_ptr, source, looping, scope, rewind)
55:   IF sample_ptr.is_null() THEN RETURN END IF
56:   LET sample_arc = wrap_as_arc(sample_ptr)
57:   LET _ = play_stream(sample_arc, source as usize, looping != 0, scope != 0, rewind != 0)
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
75:   IF sample_ptr.is_null() THEN RETURN null END IF
76:   LET sample = unsafe { &*sample_ptr }
77:   MATCH find_tagged_buffer(sample, buffer) {
78:     Some(tag) => tag as *const _ as *mut SoundTag,
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
94: FUNCTION GraphForegroundStream(data_ptr, width, height, want_speech) -> i32
95:   IF data_ptr.is_null() THEN RETURN 0 END IF
96:   LET slice = unsafe { std::slice::from_raw_parts_mut(data_ptr, width as usize) }
97:   graph_foreground_stream(slice, width as usize, height as usize, want_speech != 0) as i32
```

## 2. Track Player FFI Functions

```
100: FUNCTION SpliceTrack(track_name_ptr, track_text_ptr, timestamp_ptr, callback_ptr)
101:   LET name = c_str_to_option(track_name_ptr)
102:   LET text = c_str_to_option(track_text_ptr)   // UTF-16 → UTF-8 conversion
103:   LET timestamp = c_str_to_option(timestamp_ptr)
104:   LET callback = IF callback_ptr.is_some() THEN
105:     Some(Box::new(move |val: i32| unsafe { callback_ptr.unwrap()(val as c_int) }))
106:   ELSE None
107:   LET _ = splice_track(name, text, timestamp, callback)
108:
109: FUNCTION SpliceMultiTrack(track_names_ptr, track_text_ptr)
110:   LET names = c_str_array_to_vec(track_names_ptr)   // NULL-terminated array
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
137:   // Return thread-local CString cache to keep pointer valid
138:   MATCH get_track_subtitle() {
139:     Some(text) => cache_and_return_c_str(text),
140:     None => null()
141:   }
142:
143: FUNCTION GetFirstTrackSubtitle() -> *mut c_void
144:   MATCH get_first_track_subtitle() {
145:     Some(sub_ref) => Box::into_raw(Box::new(sub_ref)) as *mut c_void,
146:     None => null_mut()
147:   }
148:
149: FUNCTION GetNextTrackSubtitle(last_ref_ptr) -> *mut c_void
150:   IF last_ref_ptr.is_null() THEN RETURN null_mut() END IF
151:   LET sub_ref = unsafe { &*(last_ref_ptr as *const SubtitleRef) }
152:   MATCH get_next_track_subtitle(sub_ref) {
153:     Some(next) => Box::into_raw(Box::new(next)) as *mut c_void,
154:     None => null_mut()
155:   }
156:
157: FUNCTION GetTrackSubtitleText(sub_ref_ptr) -> *const c_char
158:   IF sub_ref_ptr.is_null() THEN RETURN null() END IF
159:   LET sub_ref = unsafe { &*(sub_ref_ptr as *const SubtitleRef) }
160:   MATCH get_track_subtitle_text(sub_ref) {
161:     Some(text) => cache_and_return_c_str(text),
162:     None => null()
163:   }
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
182:   LET music_ref = MusicRef(music_ref_ptr as *mut SoundSample)
183:   LET _ = plr_play_song(music_ref, continuous != 0, priority)
184:
185: FUNCTION PLRStop(music_ref_ptr)
186:   LET music_ref = MusicRef(music_ref_ptr as *mut SoundSample)
187:   LET _ = plr_stop(music_ref)
188:
189: FUNCTION PLRPlaying(music_ref_ptr) -> c_int
190:   LET music_ref = MusicRef(music_ref_ptr as *mut SoundSample)
191:   IF plr_playing(music_ref) THEN 1 ELSE 0
192:
193: FUNCTION PLRSeek(music_ref_ptr, pos)
194:   LET music_ref = MusicRef(music_ref_ptr as *mut SoundSample)
195:   LET _ = plr_seek(music_ref, pos)
196:
197: FUNCTION PLRPause(music_ref_ptr)
198:   LET music_ref = MusicRef(music_ref_ptr as *mut SoundSample)
199:   LET _ = plr_pause(music_ref)
200:
201: FUNCTION PLRResume(music_ref_ptr)
202:   LET music_ref = MusicRef(music_ref_ptr as *mut SoundSample)
203:   LET _ = plr_resume(music_ref)
204:
205: FUNCTION snd_PlaySpeech(speech_ref_ptr)
206:   IF speech_ref_ptr.is_null() THEN RETURN END IF
207:   LET speech_ref = MusicRef(speech_ref_ptr as *mut SoundSample)
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
221:   LET music_ref = MusicRef(music_ref_ptr as *mut SoundSample)
222:   LET _ = release_music_data(music_ref)   // REQ-MUSIC-RELEASE-04
```

## 4. SFX FFI Functions

```
230: FUNCTION PlayChannel(channel, snd_ptr, pos, positional_object_ptr, priority)
231:   IF snd_ptr.is_null() THEN RETURN END IF
232:   LET bank = unsafe { &*(snd_ptr as *const SoundBank) }
233:   // Note: sound_index resolution from opaque SOUND handle (see spec §5.4 gap)
234:   LET _ = play_channel(channel as usize, bank, 0, pos, positional_object_ptr as usize, priority)
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
270: FUNCTION SetSFXVolume(volume)
271:   set_sfx_volume(volume)
272:
273: FUNCTION SetSpeechVolume(volume)
274:   set_speech_volume(volume)
275:
276: FUNCTION InitSound(argc, argv) -> c_int
277:   MATCH init_sound() {
278:     Ok(()) => 1,
279:     Err(e) => { LOG error; 0 }
280:   }
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
302:     Ok(music_ref) => music_ref.0 as *mut c_void,
303:     Err(e) => { LOG error "{}", e; null_mut() }
304:   }
```

## 7. C Callback Wrapper

```
310: STRUCT CCallbackWrapper {
311:   callbacks: TFB_SoundCallbacks_C,
312: }
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
