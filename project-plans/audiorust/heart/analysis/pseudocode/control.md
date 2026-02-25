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
110: FUNCTION set_sfx_volume(volume: f32)
111:   // REQ-VOLUME-CONTROL-01: apply gain to all SFX sources
112:   FOR i IN FIRST_SFX_SOURCE..=LAST_SFX_SOURCE DO
113:     LET source = SOURCES.sources[i].lock()
114:     CALL mixer_source_f(source.handle, SourceProp::Gain, volume)
115:   END FOR
```

Validation: REQ-VOLUME-CONTROL-01

## 8. set_speech_volume

```
120: FUNCTION set_speech_volume(volume: f32)
121:   // REQ-VOLUME-CONTROL-02: apply gain to SPEECH_SOURCE
122:   LET source = SOURCES.sources[SPEECH_SOURCE].lock()
123:   CALL mixer_source_f(source.handle, SourceProp::Gain, volume)
```

Validation: REQ-VOLUME-CONTROL-02

## 9. sound_playing

```
130: FUNCTION sound_playing() -> bool
131:   // REQ-VOLUME-QUERY-01: check all sources
132:   FOR i IN 0..NUM_SOUNDSOURCES DO
133:     LET source = SOURCES.sources[i].lock()
134:     IF source.sample.is_some() THEN
135:       LET sample = source.sample.as_ref().unwrap().lock()
136:       IF sample.decoder.is_some() THEN
137:         IF playing_stream(i) THEN
138:           RETURN true
139:         END IF
140:       ELSE
141:         LET state = mixer_get_source_i(source.handle, SourceProp::SourceState)
142:         IF state == Ok(SourceState::Playing as i32) THEN
143:           RETURN true
144:         END IF
145:       END IF
146:     END IF
147:   END FOR
148:   RETURN false
```

Validation: REQ-VOLUME-QUERY-01

## 10. wait_for_sound_end

```
150: FUNCTION wait_for_sound_end(channel: Option<usize>)
151:   // REQ-VOLUME-QUERY-02: poll loop
152:   LOOP
153:     // REQ-VOLUME-QUERY-03: quit break
154:     IF quit_posted() THEN BREAK END IF
155:
156:     LET still_playing = MATCH channel {
157:       None => sound_playing(),
158:       Some(ch) => channel_playing(ch),
159:     }
160:
161:     IF NOT still_playing THEN BREAK END IF
162:
163:     SLEEP Duration::from_millis(50)
164:   END LOOP
```

Validation: REQ-VOLUME-QUERY-02..03
Side effects: Blocks calling thread
