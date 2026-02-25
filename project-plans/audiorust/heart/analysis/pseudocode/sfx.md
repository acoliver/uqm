# Pseudocode — `sound::sfx`

Plan ID: `PLAN-20260225-AUDIO-HEART`

---

## 1. play_channel

```
01: FUNCTION play_channel(channel, sound_bank, sound_index, pos, positional_object, priority) -> AudioResult<()>
02:   // Validate channel
03:   IF channel > LAST_SFX_SOURCE THEN
04:     RETURN Err(AudioError::InvalidChannel(channel))
05:   END IF
06:
07:   // REQ-SFX-PLAY-01: stop before play
08:   CALL stop_source(channel)?
09:
10:   // REQ-SFX-PLAY-02: check finished channels
11:   CALL check_finished_channels()
12:
13:   // REQ-SFX-PLAY-03: validate sample exists
14:   LET sample = sound_bank.samples.get(sound_index)
15:     .and_then(|s| s.as_ref())
16:     .ok_or(AudioError::InvalidSample)?
17:
18:   // REQ-SFX-PLAY-04: set source state
19:   LET source = SOURCES.sources[channel].lock()
20:   SET source.sample = Some(Arc::new(parking_lot::Mutex::new(sample.clone())))
21:   SET source.positional_object = positional_object
22:
23:   // REQ-SFX-PLAY-05: positional audio
24:   LET sfx_state = SFX_STATE.lock()
25:   IF sfx_state.opt_stereo_sfx THEN
26:     CALL update_sound_position(channel, pos)
27:   ELSE
28:     CALL update_sound_position(channel, SoundPosition::NON_POSITIONAL)
29:   END IF
30:
31:   // REQ-SFX-PLAY-06: bind buffer and play
32:   CALL mixer_source_i(source.handle, SourceProp::Buffer, sample.buffers[0] as i32)
33:   CALL mixer_source_play(source.handle)
34:
35:   RETURN Ok(())
```

Validation: REQ-SFX-PLAY-01..06
Error handling: InvalidChannel for out-of-range, InvalidSample for missing
Integration: Calls mixer API directly, modifies source state
Side effects: Stops previous sound, starts new playback

## 2. stop_channel

```
40: FUNCTION stop_channel(channel, priority) -> AudioResult<()>
41:   // REQ-SFX-PLAY-07: stop source, ignore priority
42:   CALL stop_source(channel)?
43:   RETURN Ok(())
```

Validation: REQ-SFX-PLAY-07

## 3. channel_playing

```
50: FUNCTION channel_playing(channel) -> bool
51:   LET source = SOURCES.sources[channel].lock()
52:   LET state = mixer_get_source_i(source.handle, SourceProp::SourceState)
53:   RETURN state == Ok(SourceState::Playing as i32)   // REQ-SFX-PLAY-09
```

Validation: REQ-SFX-PLAY-09

## 4. set_channel_volume

```
60: FUNCTION set_channel_volume(channel, volume, priority)
61:   LET vol_state = VOLUME.lock()
62:   LET gain = (volume as f32 / MAX_VOLUME as f32) * vol_state.sfx_volume_scale
63:   LET source = SOURCES.sources[channel].lock()
64:   CALL mixer_source_f(source.handle, SourceProp::Gain, gain)
65:   // REQ-SFX-VOLUME-01, priority ignored
```

Validation: REQ-SFX-VOLUME-01

## 5. check_finished_channels

```
70: FUNCTION check_finished_channels()
71:   FOR i IN FIRST_SFX_SOURCE..=LAST_SFX_SOURCE DO   // REQ-SFX-PLAY-08
72:     LET source = SOURCES.sources[i].lock()
73:     LET state = mixer_get_source_i(source.handle, SourceProp::SourceState)
74:     IF state == Ok(SourceState::Stopped as i32) THEN
75:       DROP source
76:       CALL clean_source(i)
77:     END IF
78:   END FOR
```

Validation: REQ-SFX-PLAY-08

## 6. update_sound_position

```
80: FUNCTION update_sound_position(channel, pos)
81:   LET source = SOURCES.sources[channel].lock()
82:
83:   IF pos.positional THEN
84:     // REQ-SFX-POSITION-01: compute 3D position
85:     LET x = pos.x as f32 / ATTENUATION
86:     LET y = 0.0f32
87:     LET z = pos.y as f32 / ATTENUATION
88:
89:     // REQ-SFX-POSITION-02: min distance check
90:     LET dist = (x*x + z*z).sqrt()
91:     IF dist < MIN_DISTANCE THEN
92:       IF dist > 0.0 THEN
93:         LET scale = MIN_DISTANCE / dist
94:         SET x *= scale
95:         SET z *= scale
96:       ELSE
97:         SET z = -MIN_DISTANCE   // default direction
98:       END IF
99:     END IF
100:
101:    // Use three separate mixer_source_f calls (mixer_source_fv does not exist)
102:    CALL mixer_source_f(source.handle, SourceProp::PositionX, x)
103:    CALL mixer_source_f(source.handle, SourceProp::PositionY, y)
104:    CALL mixer_source_f(source.handle, SourceProp::PositionZ, z)
105:  ELSE
106:    // REQ-SFX-POSITION-03: non-positional
107:    CALL mixer_source_f(source.handle, SourceProp::PositionX, 0.0)
108:    CALL mixer_source_f(source.handle, SourceProp::PositionY, 0.0)
109:    CALL mixer_source_f(source.handle, SourceProp::PositionZ, -1.0)
110:  END IF
```

Validation: REQ-SFX-POSITION-01..03
Integration: Uses three separate mixer_source_f calls for X, Y, Z (no mixer_source_fv needed)

**Note**: Positional audio storage (PositionX/Y/Z) is added to the mixer in phase P02b.
Positions are stored but not used for panning (matching the C mixer's no-op behavior).

## 7. get_positional_object / set_positional_object

```
110: FUNCTION get_positional_object(channel) -> usize
111:   LET source = SOURCES.sources[channel].lock()
112:   RETURN source.positional_object   // REQ-SFX-POSITION-04
113:
114: FUNCTION set_positional_object(channel, obj)
115:   LET source = SOURCES.sources[channel].lock()
116:   SET source.positional_object = obj   // REQ-SFX-POSITION-05
```

Validation: REQ-SFX-POSITION-04..05

## 8. get_sound_bank_data

```
120: FUNCTION get_sound_bank_data(filename, data) -> AudioResult<SoundBank>
121:   // REQ-SFX-LOAD-01: extract directory
122:   LET dir = Path::new(filename).parent().unwrap_or(Path::new(""))
123:
124:   // REQ-SFX-LOAD-02: parse lines
125:   LET lines = String::from_utf8_lossy(data)
126:   LET mut samples: Vec<Option<SoundSample>> = Vec::new()
127:
128:   FOR (i, line) IN lines.lines().take(MAX_FX).enumerate() DO
129:     LET sfx_name = line.trim()
130:     IF sfx_name.is_empty() THEN
131:       samples.push(None)
132:       CONTINUE
133:     END IF
134:
135:     LET full_path = dir.join(sfx_name)
136:
137:     // REQ-SFX-LOAD-03: load, decode all, upload
138:     LET decoder_result = load_decoder(content_dir, full_path, 4096, 0, 0)
139:     IF decoder_result.is_err() THEN
140:       LOG warn "failed to load SFX: {}", sfx_name
141:       samples.push(None)
142:       CONTINUE
143:     END IF
144:     LET mut decoder = decoder_result.unwrap()
145:
146:     // Create sample with 1 buffer, no callbacks
147:     LET sample_result = create_sound_sample(None, 1, None)
148:     IF sample_result.is_err() THEN
149:       samples.push(None)
150:       CONTINUE
151:     END IF
152:     LET mut sample = sample_result.unwrap()
153:
154:     // Pre-decode all audio
155:     LET decoded = decode_all(&mut decoder)?
156:     LET format = decoder.format()
157:     LET freq = decoder.frequency()
158:
159:     // Upload to single mixer buffer
160:     CALL mixer_buffer_data(sample.buffers[0], format, &decoded, freq, ...)
161:     // Decoder dropped automatically
162:
163:     samples.push(Some(sample))
164:   END FOR
165:
166:   // REQ-SFX-LOAD-04: check at least one loaded
167:   IF samples.iter().all(|s| s.is_none()) THEN
168:     RETURN Err(AudioError::ResourceNotFound(filename.to_string()))
169:   END IF
170:
171:   // REQ-SFX-LOAD-05: return bank
172:   RETURN Ok(SoundBank { samples })
```

Validation: REQ-SFX-LOAD-01..07

## 9. release_sound_bank_data

**LOCK SAFETY (FIX: ISSUE-CONC-03)**: `stop_source` acquires the Source lock internally.
We must NOT hold the Source lock when calling it — parking_lot::Mutex is not reentrant.
Collect indices needing a stop, drop the source lock, then stop them.

```
180: FUNCTION release_sound_bank_data(bank) -> AudioResult<()>
181:   // REQ-SFX-RELEASE-01: bank moved by value, empty is no-op

183:   FOR sample_opt IN bank.samples.iter() DO
184:     IF let Some(sample) = sample_opt THEN
185:       // REQ-SFX-RELEASE-02: check all sources — collect indices to stop
186:       // FIX ISSUE-CONC-03: Do NOT call stop_source while holding source lock.
187:       LET mut to_stop: Vec<usize> = Vec::new()
188:       FOR i IN 0..NUM_SOUNDSOURCES DO
189:         LET source = SOURCES.sources[i].lock()
190:         IF source.sample.as_ref().map(|s| arc_matches(s, sample)).unwrap_or(false) THEN
191:           to_stop.push(i)
192:         END IF
193:         // source lock dropped here (end of loop iteration)
194:       END FOR
195:
196:       // Now stop and clear the matched sources (no locks held)
      // FIX MED-EDGE-03: Re-check match in Phase 2 to prevent TOCTOU
197:       FOR i IN to_stop DO
198:         CALL stop_source(i)?   // acquires Source lock internally
199:         LET source = SOURCES.sources[i].lock()
200:         // Re-verify the sample still matches (another thread may have swapped it)
201:         IF source.sample.as_ref().map(|s| arc_matches(s, sample)).unwrap_or(false) THEN
202:           SET source.sample = None
203:         END IF
204:       END FOR
202:
203:       // REQ-SFX-RELEASE-03: destroy sample
204:       CALL destroy_sound_sample(sample)?
205:     END IF
206:   END FOR

208:   // Bank dropped (Rust drop semantics)
209:   RETURN Ok(())
```

Validation: REQ-SFX-RELEASE-01..03
