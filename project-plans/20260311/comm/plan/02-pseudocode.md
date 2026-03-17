# Phase 02: Pseudocode

## Phase ID
`PLAN-20260314-COMM.P02`

## Prerequisites
- Required: Phase 01a (Analysis Verification) completed

## Pseudocode Components

### Component 1: CommData / LOCDATA FFI

```
01: FUNCTION read_locdata_from_c(locdata_ptr: *LOCDATA) -> CommData
02:   VALIDATE locdata_ptr is not null
03:   data = CommData::default()
04:   data.init_encounter_func = read_field(locdata_ptr, init_encounter_func)
05:   data.post_encounter_func = read_field(locdata_ptr, post_encounter_func)
06:   data.uninit_encounter_func = read_field(locdata_ptr, uninit_encounter_func)
07:   data.alien_frame_res = read_field(locdata_ptr, AlienFrameRes)
08:   data.alien_font_res = read_field(locdata_ptr, AlienFontRes)
09:   data.alien_text_fcolor = read_field(locdata_ptr, AlienTextFColor)
10:   data.alien_text_bcolor = read_field(locdata_ptr, AlienTextBColor)
11:   data.alien_text_baseline = read_field(locdata_ptr, AlienTextBaseline)
12:   data.alien_text_width = read_field(locdata_ptr, AlienTextWidth)
13:   data.alien_text_align = read_field(locdata_ptr, AlienTextAlign)
14:   data.alien_text_valign = read_field(locdata_ptr, AlienTextValign)
15:   data.alien_colormap_res = read_field(locdata_ptr, AlienColorMapRes)
16:   data.alien_song_res = read_field(locdata_ptr, AlienSongRes)
17:   data.alien_alt_song_res = read_field(locdata_ptr, AlienAltSongRes)
18:   data.alien_song_flags = read_field(locdata_ptr, AlienSongFlags)
19:   data.conversation_phrases_res = read_field(locdata_ptr, ConversationPhrasesRes)
20:   data.num_animations = read_field(locdata_ptr, NumAnimations)
21:   FOR i IN 0..data.num_animations
22:     data.alien_ambient_array[i] = read_animation_desc(locdata_ptr, i)
23:   data.alien_transition_desc = read_animation_desc_field(locdata_ptr, AlienTransitionDesc)
24:   data.alien_talk_desc = read_animation_desc_field(locdata_ptr, AlienTalkDesc)
25:   data.alien_number_speech = read_field(locdata_ptr, AlienNumberSpeech)
26:   RETURN data
```

### Component 2: init_race Dispatch

```
30: FUNCTION load_commdata_for_race(comm_id: u32) -> CommResult<CommData>
31:   CALL c_init_race(comm_id) -> locdata_ptr
32:   IF locdata_ptr is null THEN RETURN error
33:   comm_data = read_locdata_from_c(locdata_ptr)
34:   STORE comm_data in COMM_STATE
35:   RETURN success
```

### Component 3: Phrase Enable/Disable

```
40: STRUCT PhraseState
41:   disabled: BitSet  // encounter-local, reset per encounter
42:
43: FUNCTION phrase_enabled(index: i32) -> bool
44:   RETURN NOT disabled.contains(index)
45:
46: FUNCTION disable_phrase(index: i32)
47:   disabled.insert(index)
48:
49: FUNCTION reset_phrase_state()
50:   disabled.clear()
```

### Component 4: NPCPhrase_cb Glue

```
55: FUNCTION npc_phrase_cb(index: i32, callback: Option<PhraseCallback>)
56:   IF index == 0 THEN RETURN  // no-op phrase
57:   IF index == GLOBAL_PLAYER_NAME
58:     text = get_commander_name()
59:     queue_phrase_with_trackplayer(null_clip, text, null_timestamps, callback)
60:     RETURN
61:   IF index == GLOBAL_SHIP_NAME
62:     text = get_ship_name()
63:     queue_phrase_with_trackplayer(null_clip, text, null_timestamps, callback)
64:     RETURN
65:   IF index < 0  // alliance name variants
66:     text = get_alliance_name(index)
67:     queue_phrase_with_trackplayer(null_clip, text, null_timestamps, callback)
68:     RETURN
69:   // Normal phrase — resolve from conversation phrases resource
70:   phrase_data = resolve_phrase(CommData.ConversationPhrases, index - 1)
71:   text = phrase_data.text
72:   clip = phrase_data.audio_clip
73:   timestamps = phrase_data.timestamps
74:   queue_phrase_with_trackplayer(clip, text, timestamps, callback)
```

### Component 5: NPCPhrase_splice

```
80: FUNCTION npc_phrase_splice(index: i32)
81:   phrase_data = resolve_phrase(CommData.ConversationPhrases, index - 1)
82:   IF phrase_data.audio_clip is null
83:     splice_into_current_phrase(null_clip, phrase_data.text, null_timestamps)
84:     // no page break — append to current phrase
85:   ELSE
86:     splice_into_current_phrase([phrase_data.audio_clip], phrase_data.text)
```

### Component 6: NPCNumber

```
90: FUNCTION npc_number(number: i32, fmt: *const c_char)
91:   number_speech = CommData.AlienNumberSpeech
92:   IF number_speech is null
93:     // text-only: format number as string and splice
94:     text = format(fmt, number)
95:     npc_phrase_splice_text(text)
96:     RETURN
97:   // Decompose number into speech components
98:   components = decompose_number(number, number_speech)
99:   FOR EACH component IN components
100:    queue_multi_clip_phrase(component.clips, component.text)
```

### Component 7: construct_response

```
105: FUNCTION construct_response(buf: &mut [u8], response_ref: i32, fragments: &[(i32)])
106:   buf.clear()
107:   FOR EACH fragment_index IN fragments
108:     IF fragment_index == 0 THEN CONTINUE
109:     phrase_data = resolve_phrase(CommData.ConversationPhrases, fragment_index - 1)
110:     buf.append(phrase_data.text)
111:     buf.append(" ")
112:   buf.trim_trailing_whitespace()
```

### Component 8: Segue State

```
115: ENUM Segue { Peace, Hostile, Victory, Defeat }
116:
117: FUNCTION set_segue(segue: Segue)
118:   MATCH segue
119:     Peace => SET BATTLE_SEGUE = 0
120:     Hostile => SET BATTLE_SEGUE = 1
121:     Victory =>
122:       SET BATTLE_SEGUE = 1
123:       SET instantVictory = TRUE
124:     Defeat =>
125:       SET crew = ~0  // sentinel
126:       CALL CHECK_RESTART
127:   STORE current_segue = segue
128:
129: FUNCTION get_segue() -> Segue
130:   IF BATTLE_SEGUE == 0 THEN RETURN Peace
131:   ELSE RETURN Hostile (or check victory flag)
```

### Component 9: Animation Engine (ANIMATION_DESC model)

```
135: STRUCT AnimSequence
136:   desc: AnimationDesc      // from LOCDATA
137:   alarm: u32               // ticks until next frame advance
138:   current_frame: u32       // current frame within sequence
139:   direction: i32           // 1=forward, -1=backward (for yoyo)
140:   frames_remaining: u32    // for one-shot tracking
141:   anim_type: AnimType      // Random, Circular, Yoyo, ColorXForm
142:   active: bool
143:   change_flag: bool
144:
145: STRUCT CommAnimState
146:   sequences: [AnimSequence; MAX_ANIMATIONS + 2]  // 20 ambient + talk + transit
147:   active_mask: u32         // bitmask of active animations
148:   talk: &AnimSequence      // alias to sequences[ambient_count]
149:   transit: &AnimSequence   // alias to sequences[ambient_count + 1]
150:   first_ambient: usize
151:   total_sequences: usize
152:   last_time: TimeCount
153:
154: FUNCTION process_comm_animations(delta_ticks: u32)
155:   FOR EACH seq IN active sequences
156:     IF seq.alarm has expired
157:       ADVANCE frame per anim_type:
158:         RANDOM: random frame != current
159:         CIRCULAR: sequential wrap
160:         YOYO: forward/backward bounce
161:         COLORXFORM: colormap index advance
162:       CHECK BlockMask conflicts — skip if conflicting anim is active
163:       CHECK WAIT_TALKING — settle to neutral frame if talking
164:       SET seq.change_flag = true
165:       RESET alarm = BaseFrameRate + random(0..RandomFrameRate)
166:       IF one-shot and completed THEN disable sequence
167:   APPLY frame changes to portrait rendering
```

### Component 10: Public Entry-Point Routing and Encounter Lifecycle

```
170: FUNCTION race_communication() -> CommResult<()>
171:   IF saved_game_just_loaded()
172:     update_sis_display_for_current_context()
173:   comm_id = resolve_conversation_from_game_state()
174:   RETURN init_communication(comm_id)
175:
176: FUNCTION init_communication(which_comm: u32)
177:   // Resolve encounter type
178:   comm_id = normalize_conversation(which_comm)
179:   // Build NPC fleet if needed
180:   IF combat_possible THEN build_npc_fleet(comm_id)
181:   // Start sphere tracking
182:   start_sphere_tracking(get_race_index(comm_id))
183:   // Init race script data through C-owned dispatch helper
184:   load_commdata_for_race(comm_id)
185:   // Hail or attack decision
186:   IF BATTLE_SEGUE != 0
187:     present hail_or_attack choice
188:     IF attack chosen
189:       CALL post_encounter_func()
190:       CALL uninit_encounter_func()
191:       SET BATTLE_SEGUE = 1
192:       GOTO combat_setup
193:   // Enter dialogue
194:   hail_alien()
195:
196: FUNCTION hail_alien()
197:   // Load resources
198:   load_encounter_resources(comm_data)
199:   // Create graphics contexts
200:   create_subtitle_cache_context()
201:   create_animation_context()
202:   // Reset phrase state
203:   reset_phrase_state()
204:   // Call init_encounter_func
205:   RELEASE comm_state lock
206:   CALL comm_data.init_encounter_func()
207:   REACQUIRE comm_state lock
208:   // Enter main dialogue loop
209:   do_communication()
210:   // Normal exit
211:   RELEASE comm_state lock
212:   CALL comm_data.post_encounter_func()
213:   CALL comm_data.uninit_encounter_func()
214:   REACQUIRE comm_state lock
215:   // Teardown resources
216:   destroy_encounter_resources()
```

### Component 11: Talk Segue

```
220: FUNCTION talk_segue(wait_track: bool)
221:   IF first_call THEN alien_talk_segue_init()
222:   start_track()
223:   set_talking(true)
224:   // Transition to talking animation
225:   IF has_talking_anim AND NOT already_talking
226:     run_intro_anim()
227:     run_talking_anim()
228:   // Playback loop
229:   WHILE playing_track()
230:     IF check_abort THEN BREAK
231:     IF cancel_pressed THEN jump_track(); mark_ended; BREAK
232:     IF left_right_pressed THEN enter_seek_mode()
233:     IF NOT seeking THEN poll_subtitles()
234:     poll_pending_track_completion_and_dispatch_callbacks()
235:     IF seeking THEN pause_animations()
236:     ELSE process_comm_animations(delta)
237:     update_speech_graphics()
238:     sleep_until_next_frame(1/60s)
239:   // Post-playback
240:   clear_subtitles()
241:   set_slider_stop()
242:   transition_to_silent()
243:   fade_music_to_foreground()
244:
245: FUNCTION do_communication()
246:   LOOP
247:     IF NOT talking_finished
248:       talk_segue(wait_track=true)
249:       CONTINUE
250:     IF response_count == 0
251:       // No responses — timeout with replay, then exit
252:       timeout_with_replay()
253:       BREAK
254:     // Player response phase
255:     selected = do_response_input()
256:     IF selected is valid
257:       clear_responses_display()
258:       copy_feedback_text()
259:       stop_track()
260:       clear_subtitles()
261:       fade_music_to_background()
262:       RELEASE comm_state lock
263:       CALL response_callback(response_ref)
264:       REACQUIRE comm_state lock
265:       IF new_phrases_queued THEN set_talking_finished(false)
266:       IF no_responses_and_no_phrases THEN BREAK
```

### Component 12: Response UI

```
270: FUNCTION render_responses()
271:   FOR i IN visible_range(top_response, response_count)
272:     IF i == selected
273:       draw_text_highlighted(responses[i].text, y_pos)
274:     ELSE
275:       draw_text_dimmed(responses[i].text, y_pos)
276:     y_pos += line_height
277:   IF top_response > 0 THEN draw_scroll_up_arrow()
278:   IF top_response + visible_count < response_count THEN draw_scroll_down_arrow()
279:
280: FUNCTION do_response_input() -> Option<usize>
281:   LOOP via DoInput
282:     IF up THEN select_prev(); redraw
283:     IF down THEN select_next(); redraw
284:     IF select THEN RETURN Some(selected)
285:     IF cancel THEN show_conversation_summary()
286:     IF left THEN replay_last_phrase()
```

### Component 13: Conversation Summary

```
290: FUNCTION show_conversation_summary()
291:   subtitles = enumerate_trackplayer_subtitle_history()
292:   pages = paginate_subtitles(subtitles, comm_window_width, summary_font)
293:   current_page = 0
294:   LOOP
295:     render_summary_page(pages[current_page])
296:     wait_for_input()
297:     IF advance AND current_page < pages.len() - 1
298:       current_page += 1
299:     ELSE
300:       BREAK  // return to response selection
```

### Component 14: Lock Discipline for Callback Invocation

```
305: FUNCTION invoke_c_callback_safely(callback: fn_ptr, args...)
306:   // 1. Collect any data needed from comm state while lock is held
307:   needed_data = read_from_comm_state()
308:   // 2. Release the lock
309:   drop(comm_state_guard)
310:   // 3. Call the C callback (may re-enter Rust comm API)
311:   callback(args...)
312:   // 4. Reacquire the lock
313:   comm_state_guard = COMM_STATE.write()
314:   // 5. Apply any deferred mutations from the callback
315:   apply_deferred_mutations(comm_state_guard)
```

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```
