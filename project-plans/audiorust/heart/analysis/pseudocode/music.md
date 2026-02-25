# Pseudocode — `sound::music`

Plan ID: `PLAN-20260225-AUDIO-HEART`

---

## 1. plr_play_song

```
01: FUNCTION plr_play_song(music_ref, continuous, priority) -> AudioResult<()>
02:   // REQ-MUSIC-PLAY-02: validate ref
03:   IF music_ref.is_null() THEN
04:     RETURN Err(AudioError::InvalidSample)
05:   END IF
06:
07:   LET sample_ptr = music_ref.0
08:   LET sample = unsafe { &mut *sample_ptr }
09:   LET sample_arc = Arc::new(parking_lot::Mutex::new(/* wrap sample */))
10:
11:   // REQ-MUSIC-PLAY-01: play on MUSIC_SOURCE
12:   CALL play_stream(
13:     sample_arc,
14:     MUSIC_SOURCE,
15:     continuous,    // looping
16:     true,          // scope always true
17:     true,          // rewind always true
18:   )?
19:
20:   // Store as current music ref
21:   LET state = MUSIC_STATE.lock()
22:   SET state.cur_music_ref = Some(music_ref)
23:
24:   // REQ-MUSIC-PLAY-03: priority ignored
25:   RETURN Ok(())
```

Validation: REQ-MUSIC-PLAY-01..03
Integration: Calls play_stream, modifies MUSIC_STATE
Side effects: Starts music playback on MUSIC_SOURCE

## 2. plr_stop

```
30: FUNCTION plr_stop(music_ref) -> AudioResult<()>
31:   LET state = MUSIC_STATE.lock()
32:
33:   // REQ-MUSIC-PLAY-04: stop if matches or wildcard
34:   LET is_wildcard = music_ref.0 == !0usize as *mut _
35:   LET matches = is_wildcard OR state.cur_music_ref
36:     .map(|cur| ptr::eq(cur.0, music_ref.0))
37:     .unwrap_or(false)
38:
39:   IF matches THEN
40:     CALL stop_stream(MUSIC_SOURCE)?
41:     SET state.cur_music_ref = None
42:   END IF
43:   RETURN Ok(())
```

Validation: REQ-MUSIC-PLAY-04

## 3. plr_playing

```
50: FUNCTION plr_playing(music_ref) -> bool
51:   LET state = MUSIC_STATE.lock()
52:
53:   // REQ-MUSIC-PLAY-05: check ref match and stream state
54:   LET is_wildcard = music_ref.0 == !0usize as *mut _
55:   IF state.cur_music_ref.is_none() THEN RETURN false END IF
56:
57:   LET matches = is_wildcard OR ptr::eq(
58:     state.cur_music_ref.unwrap().0,
59:     music_ref.0
60:   )
61:   IF NOT matches THEN RETURN false END IF
62:
63:   RETURN playing_stream(MUSIC_SOURCE)
```

Validation: REQ-MUSIC-PLAY-05

## 4. plr_seek / plr_pause / plr_resume

```
70: FUNCTION plr_seek(music_ref, pos) -> AudioResult<()>
71:   IF ref_matches_or_wildcard(music_ref) THEN
72:     CALL seek_stream(MUSIC_SOURCE, pos)?   // REQ-MUSIC-PLAY-06
73:   END IF
74:   RETURN Ok(())
75:
76: FUNCTION plr_pause(music_ref) -> AudioResult<()>
77:   IF ref_matches_or_wildcard(music_ref) THEN
78:     CALL pause_stream(MUSIC_SOURCE)?   // REQ-MUSIC-PLAY-07
79:   END IF
80:   RETURN Ok(())
81:
82: FUNCTION plr_resume(music_ref) -> AudioResult<()>
83:   IF ref_matches_or_wildcard(music_ref) THEN
84:     CALL resume_stream(MUSIC_SOURCE)?   // REQ-MUSIC-PLAY-08
85:   END IF
86:   RETURN Ok(())
```

Validation: REQ-MUSIC-PLAY-06..08

## 5. snd_play_speech / snd_stop_speech

```
90: FUNCTION snd_play_speech(speech_ref) -> AudioResult<()>
91:   LET state = MUSIC_STATE.lock()
92:   LET sample_ptr = speech_ref.0
93:   LET sample = unsafe { &mut *sample_ptr }
94:   LET sample_arc = Arc::new(parking_lot::Mutex::new(/* wrap sample */))
95:
96:   // REQ-MUSIC-SPEECH-01: no looping, no scope, rewind
97:   CALL play_stream(
98:     sample_arc,
99:     SPEECH_SOURCE,
100:    false,   // no looping
101:    false,   // no scope
102:    true,    // rewind
103:  )?
104:  SET state.cur_speech_ref = Some(speech_ref)
105:  RETURN Ok(())
106:
107: FUNCTION snd_stop_speech() -> AudioResult<()>
108:   LET state = MUSIC_STATE.lock()
109:   IF state.cur_speech_ref.is_none() THEN
110:     RETURN Ok(())   // REQ-MUSIC-SPEECH-02
111:   END IF
112:   CALL stop_stream(SPEECH_SOURCE)?
113:   SET state.cur_speech_ref = None
114:   RETURN Ok(())
```

Validation: REQ-MUSIC-SPEECH-01..02

## 6. get_music_data

```
120: FUNCTION get_music_data(filename) -> AudioResult<MusicRef>
121:   // REQ-MUSIC-LOAD-01: empty check
122:   IF filename.is_empty() THEN
123:     RETURN Err(AudioError::NullPointer)
124:   END IF
125:
126:   // REQ-MUSIC-LOAD-02: load decoder
127:   LET decoder = load_decoder(content_dir, filename, 4096, 0, 0)
128:   IF decoder.is_err() THEN
129:     RETURN Err(AudioError::ResourceNotFound(filename.to_string()))   // REQ-MUSIC-LOAD-04
130:   END IF
131:   LET decoder = decoder.unwrap()
132:
133:   // Create sample with 64 buffers, no callbacks
134:   LET sample = create_sound_sample(Some(decoder), 64, None)
135:   IF sample.is_err() THEN
136:     // REQ-MUSIC-LOAD-05: drop decoder (automatic), return error
137:     RETURN Err(sample.unwrap_err())
138:   END IF
139:
140:   // REQ-MUSIC-LOAD-03: leak Box to raw pointer
141:   LET boxed = Box::new(sample.unwrap())
142:   LET ptr = Box::into_raw(boxed)
143:   RETURN Ok(MusicRef(ptr))
```

Validation: REQ-MUSIC-LOAD-01..05

## 7. release_music_data

```
150: FUNCTION release_music_data(music_ref) -> AudioResult<()>
151:   // REQ-MUSIC-RELEASE-01
152:   IF music_ref.is_null() THEN
153:     RETURN Err(AudioError::NullPointer)
154:   END IF
155:
156:   LET sample = unsafe { &mut *music_ref.0 }
157:
158:   // REQ-MUSIC-RELEASE-02: stop if currently active
159:   IF sample.decoder.is_some() THEN
160:     LET source = SOURCES.sources[MUSIC_SOURCE].lock()
161:     IF source.sample.as_ref().map(|s| arc_points_to(s, music_ref.0)).unwrap_or(false) THEN
162:       CALL stop_stream(MUSIC_SOURCE)?
163:     END IF
164:   END IF
165:
166:   // REQ-MUSIC-RELEASE-03: cleanup
167:   SET sample.decoder = None   // drop decoder
168:   CALL destroy_sound_sample(sample)?
169:   LET _ = unsafe { Box::from_raw(music_ref.0) }   // reclaim and drop
170:   RETURN Ok(())
```

Validation: REQ-MUSIC-RELEASE-01..03

## 8. check_music_res_name

```
180: FUNCTION check_music_res_name(filename) -> Option<String>
181:   // REQ-MUSIC-LOAD-06: warn if missing, still return Some
182:   IF NOT file_exists_in_content_dir(filename) THEN
183:     LOG warn "music resource not found: {}", filename
184:   END IF
185:   RETURN Some(filename.to_string())
```

Validation: REQ-MUSIC-LOAD-06

## 9. set_music_volume

```
190: FUNCTION set_music_volume(volume)
191:   LET state = MUSIC_STATE.lock()
192:   SET state.music_volume = volume
193:   LET gain = (volume as f32 / 255.0) * state.music_volume_scale
194:   LET source = SOURCES.sources[MUSIC_SOURCE].lock()
195:   CALL mixer_source_f(source.handle, SourceProp::Gain, gain)
196:   // REQ-MUSIC-VOLUME-01
```

Validation: REQ-MUSIC-VOLUME-01

## 10. fade_music

```
200: FUNCTION fade_music(end_vol, time_interval) -> u32
201:   // REQ-VOLUME-CONTROL-03: clamp on quit
202:   LET interval = IF quit_posted() OR time_interval < 0 THEN 0 ELSE time_interval
203:
204:   // REQ-VOLUME-CONTROL-04: attempt fade
205:   LET accepted = set_music_stream_fade(interval as u32, end_vol)
206:   IF NOT accepted THEN
207:     CALL set_music_volume(end_vol)
208:     RETURN get_time_counter()
209:   END IF
210:
211:   // REQ-VOLUME-CONTROL-05: return completion time
212:   RETURN get_time_counter() + interval as u32 + 1
```

Validation: REQ-VOLUME-CONTROL-03..05
