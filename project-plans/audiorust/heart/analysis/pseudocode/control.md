# Pseudocode — `sound::control`

Plan ID: `PLAN-20260225-AUDIO-HEART`

---

## 1. SoundSourceArray Initialization

```
01: FUNCTION SoundSourceArray::new() -> SoundSourceArray
02:   // REQ-VOLUME-INIT-01: create NUM_SOUNDSOURCES mixer sources
03:   LET handles = mixer_gen_sources(NUM_SOUNDSOURCES as u32)
04:   IF handles.is_err() THEN
05:     PANIC "failed to create mixer sources"   // Fatal: cannot operate without sources
06:   END IF
07:   LET handles = handles.unwrap()
08:
09:   LET sources = array of NUM_SOUNDSOURCES parking_lot::Mutex<SoundSource>
10:   FOR (i, handle) IN handles.iter().enumerate() DO
11:     SET sources[i] = parking_lot::Mutex::new(SoundSource {
12:       sample: None,
13:       handle: *handle,
14:       stream_should_be_playing: false,
15:       start_time: 0,
16:       pause_time: 0,
17:       positional_object: 0,
18:       last_q_buf: 0,
19:       sbuffer: None,
20:       sbuf_size: 0,
21:       sbuf_tail: 0,
22:       sbuf_head: 0,
23:       sbuf_lasttime: 0,
24:     })
25:   END FOR
26:
27:   RETURN SoundSourceArray { sources }
```

Validation: REQ-VOLUME-INIT-01
Side effects: Allocates mixer sources (requires mixer to be initialized)

## 2. VolumeState Initialization

```
30: FUNCTION VolumeState::new() -> VolumeState
31:   RETURN VolumeState {
32:     music_volume: NORMAL_VOLUME,          // REQ-VOLUME-INIT-02
33:     music_volume_scale: 1.0,              // REQ-VOLUME-INIT-03
34:     sfx_volume_scale: 1.0,
35:     speech_volume_scale: 1.0,
36:   }
```

Validation: REQ-VOLUME-INIT-02..03

## 3. init_sound / uninit_sound

```
40: FUNCTION init_sound() -> AudioResult<()>
41:   // REQ-VOLUME-INIT-04: callable, returns Ok
42:   // Future: could accept config struct
43:   RETURN Ok(())
44:
45: FUNCTION uninit_sound()
46:   // REQ-VOLUME-INIT-05: no-op
47:   // Resource cleanup handled by Rust Drop on program exit
```

Validation: REQ-VOLUME-INIT-04..05

## 4. stop_source

```
50: FUNCTION stop_source(source_index) -> AudioResult<()>
51:   // Validate index
52:   IF source_index >= NUM_SOUNDSOURCES THEN
53:     RETURN Err(AudioError::InvalidSource(source_index))
54:   END IF
55:
56:   LET source = SOURCES.sources[source_index].lock()
57:
58:   // REQ-VOLUME-SOURCE-01: stop mixer then clean
59:   CALL mixer_source_stop(source.handle)
60:   DROP source
61:   CALL clean_source(source_index)?
62:   RETURN Ok(())
```

Validation: REQ-VOLUME-SOURCE-01

## 5. clean_source

```
70: FUNCTION clean_source(source_index) -> AudioResult<()>
71:   IF source_index >= NUM_SOUNDSOURCES THEN
72:     RETURN Err(AudioError::InvalidSource(source_index))
73:   END IF
74:
75:   LET source = SOURCES.sources[source_index].lock()
76:
77:   // REQ-VOLUME-SOURCE-02: reset positional, unqueue, rewind
78:   SET source.positional_object = 0
79:
80:   LET processed = mixer_get_source_i(source.handle, SourceProp::BuffersProcessed)
81:     .unwrap_or(0) as u32
82:
83:   IF processed > 0 THEN
84:     // REQ-VOLUME-SOURCE-03: Rust Vec handles allocation
85:     LET _unqueued = mixer_source_unqueue_buffers(source.handle, processed)
86:     // Ignore errors during cleanup
87:   END IF
88:
89:   CALL mixer_source_rewind(source.handle)
90:   RETURN Ok(())
```

Validation: REQ-VOLUME-SOURCE-02..03

## 6. stop_sound

```
100: FUNCTION stop_sound()
101:   // REQ-VOLUME-SOURCE-04: stop all SFX sources
102:   FOR i IN FIRST_SFX_SOURCE..=LAST_SFX_SOURCE DO
103:     LET _ = stop_source(i)   // ignore errors during bulk stop
104:   END FOR
```

Validation: REQ-VOLUME-SOURCE-04

## 7. set_sfx_volume

```
110: FUNCTION set_sfx_volume(volume: i32)
111:   // REQ-VOLUME-CONTROL-01: apply gain to all SFX sources
112:   // volume is 0..255 integer (consistent with set_music_volume).
113:   // Compute gain: volume / MAX_VOLUME (float 0.0..1.0)
114:   LET vol_state = VOLUME.lock()
115:   SET vol_state.sfx_volume_scale = volume as f32 / MAX_VOLUME as f32
116:   FOR i IN FIRST_SFX_SOURCE..=LAST_SFX_SOURCE DO
117:     LET source = SOURCES.sources[i].lock()
118:     CALL mixer_source_f(source.handle, SourceProp::Gain, vol_state.sfx_volume_scale)
119:   END FOR
```

Validation: REQ-VOLUME-CONTROL-01

## 8. set_speech_volume

```
120: FUNCTION set_speech_volume(volume: i32)
121:   // REQ-VOLUME-CONTROL-02: apply gain to SPEECH_SOURCE
122:   // volume is 0..255 integer (consistent with set_music_volume/set_sfx_volume).
123:   LET vol_state = VOLUME.lock()
124:   SET vol_state.speech_volume_scale = volume as f32 / MAX_VOLUME as f32
125:   LET source = SOURCES.sources[SPEECH_SOURCE].lock()
126:   CALL mixer_source_f(source.handle, SourceProp::Gain, vol_state.speech_volume_scale)
```

Validation: REQ-VOLUME-CONTROL-02

## 9. sound_playing

**LOCK SAFETY**: `playing_stream` also acquires the Source mutex internally, so
calling it while holding the source lock would self-deadlock (parking_lot::Mutex
is not reentrant). For sources with decoders (streaming sources), we check the
`stream_should_be_playing` flag directly using the already-held source guard
instead of calling `playing_stream`. For non-streaming sources (SFX), we query
the mixer state directly.

```
130: FUNCTION sound_playing() -> bool
131:   // REQ-VOLUME-QUERY-01: check all sources
132:   FOR i IN 0..NUM_SOUNDSOURCES DO
133:     LET source = SOURCES.sources[i].lock()
134:     IF source.sample.is_some() THEN
135:       IF let Some(sample_arc) = &source.sample THEN
136:         LET sample = sample_arc.lock()
136:       IF sample.decoder.is_some() THEN
137:         // Streaming source: check the flag directly instead of calling
138:         // playing_stream(i), which would re-lock the source (deadlock).
139:         IF source.stream_should_be_playing THEN
140:           RETURN true
141:         END IF
142:       ELSE
143:         // Non-streaming source (pre-decoded SFX): query mixer directly
144:         LET state = mixer_get_source_i(source.handle, SourceProp::SourceState)
145:         IF state == Ok(SourceState::Playing as i32) THEN
146:           RETURN true
147:         END IF
148:       END IF
149:     END IF
150:   END FOR
151:   RETURN false
```

Validation: REQ-VOLUME-QUERY-01

## 10. wait_for_sound_end

```
150: FUNCTION wait_for_sound_end(channel: Option<usize>)
151:   // REQ-VOLUME-QUERY-02: poll loop with sleep
152:   // This function blocks the calling thread until the specified source
153:   // stops playing. It uses a polling approach (not condvar) matching
154:   // the C implementation in sound.c:WaitForSoundEnd.
155:   //
156:   // The C code uses TaskSwitch() which yields for ~10ms. We use an
157:   // explicit 10ms sleep to match this behavior. This gives the decoder
158:   // thread time to process buffers and detect end-of-stream.
159:   LOOP
160:     // REQ-VOLUME-QUERY-03: quit break — exit immediately if game
161:     // is shutting down, even if audio is still technically playing.
162:     // This prevents the polling loop from blocking program exit.
163:     IF quit_posted() THEN BREAK END IF
164:
165:     // Check whether the target source(s) are still playing.
166:     // channel=None checks ALL sources (any sound playing at all).
167:     // channel=Some(ch) checks a specific SFX channel.
168:     LET still_playing = MATCH channel {
169:       None => sound_playing(),       // calls sound_playing() which checks all sources
170:       Some(ch) => channel_playing(ch), // checks specific SFX channel via mixer state
171:     }
172:
173:     IF NOT still_playing THEN BREAK END IF
174:
175:     // Sleep 10ms between polls, matching C TaskSwitch() granularity.
176:     // This is a deliberate busy-wait with sleep, not a condvar-based
177:     // approach, because:
178:     // 1. The C code uses TaskSwitch() (cooperative yield ~10ms)
179:     // 2. Multiple unrelated sources may finish at different times
180:     // 3. The streaming thread has no direct way to signal "done"
181:     //    to a waiting caller without adding complexity
182:     // 10ms gives responsive detection while keeping CPU usage low.
183:     SLEEP Duration::from_millis(10)
184:   END LOOP
```

Validation: REQ-VOLUME-QUERY-02..03
Side effects: Blocks calling thread with 10ms polling interval (matches C TaskSwitch granularity)
Threading: Runs on the calling thread (typically the main/game thread); does NOT hold any locks during sleep
