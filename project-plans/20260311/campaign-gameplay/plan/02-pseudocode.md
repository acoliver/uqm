# Phase 02: Pseudocode

## Phase ID
`PLAN-20260314-CAMPAIGN.P02`

## Prerequisites
- Required: Phase 01a (Analysis Verification) completed

## Purpose

Algorithmic pseudocode for all major campaign gameplay components. Line numbers are referenced by implementation phases.

---

## Component 001: Campaign Loop and Activity Dispatch

```
001: FUNCTION campaign_run(session: &mut CampaignSession)
002:   start_game(session)
003:   IF session.should_exit THEN RETURN
004:   init_campaign_kernel(session)
005:   init_game_clock(session.start_date)
006:   add_initial_game_events()
007:   LOOP
008:     IF session.has_deferred_transition THEN
009:       adopt_deferred_transition(session)
010:     END IF
011:     IF session.has_start_encounter THEN
012:       IF is_starbase_context(session) THEN
013:         visit_starbase(session)
014:       ELSE
015:         run_encounter(session)
016:       END IF
017:     ELSE IF session.has_start_interplanetary THEN
018:       set_clock_rate(INTERPLANETARY_RATE)
019:       explore_solar_system(session)
020:     ELSE
021:       session.set_activity(IN_HYPERSPACE)
022:       set_clock_rate(HYPERSPACE_RATE)
023:       run_hyperspace(session)
024:     END IF
025:     IF session.victory THEN BREAK
026:     IF session.defeat THEN BREAK
027:     IF session.restart_requested THEN BREAK
028:   END LOOP
029:   uninit_game_clock()
030:   free_campaign_kernel(session)
031: END FUNCTION
```

## Component 002: Start Game / Entry Flow

```
040: FUNCTION start_game(session: &mut CampaignSession) -> StartResult
041:   LOOP
042:     result = try_start_game(session)
043:     MATCH result
044:       NewGame =>
045:         init_new_campaign(session)
046:         IF has_intro_sequence THEN play_intro()
047:         session.set_activity(Interplanetary)
048:         session.set_location(SOL_SYSTEM)
049:         RETURN Ok
050:       LoadGame(slot) =>
051:         load_result = load_game(session, slot)
052:         IF load_result.is_ok() THEN RETURN Ok
053:         ELSE continue  // back to menu
054:       SuperMelee =>
055:         session.should_exit = true
056:         RETURN SuperMelee
057:       Quit =>
058:         session.should_exit = true
059:         RETURN Quit
060:     END MATCH
061:   END LOOP
062: END FUNCTION
063:
064: FUNCTION init_new_campaign(session: &mut CampaignSession)
065:   CLEAR all campaign runtime state
066:   SET session.date = campaign_start_date()
067:   SET session.location = SOL_COORDINATES
068:   SET session.activity = Interplanetary
069:   INIT escort queue with starting ship
070:   INIT game-state bitfield to defaults
071:   CLEAR encounter queue
072:   CLEAR npc ship queue
073: END FUNCTION
```

## Component 003: Deferred Transition

```
080: FUNCTION adopt_deferred_transition(session: &mut CampaignSession)
081:   VALIDATE session.pending_transition is Some
082:   target = session.pending_transition.take()
083:   session.current_activity = target.activity
084:   session.transition_flags = target.flags
085:   // No save-slot mutation, no fake-load side effects
086: END FUNCTION
087:
088: FUNCTION request_deferred_transition(session: &mut CampaignSession, target: Activity, flags: TransitionFlags)
089:   session.pending_transition = Some(DeferredTransition { activity: target, flags })
090: END FUNCTION
```

## Component 004: Hyperspace Transitions

```
100: FUNCTION handle_hyperspace_encounter(session: &mut CampaignSession, collided_group: &NpcGroup)
101:   SAVE hyperspace navigation context to session
102:   REORDER encounter queue so collided_group is first
103:   SET session.start_encounter = true
104: END FUNCTION
105:
106: FUNCTION handle_interplanetary_transition(session: &mut CampaignSession, target_system: SystemId)
107:   CLEAR orbit context
108:   RESET broadcaster state
109:   IF target_system == ARILOU_HOMEWORLD THEN
110:     SET session.start_encounter = true  // route to encounter, not exploration
111:   ELSE
112:     SET session.start_interplanetary = true
113:     SET session.destination_system = target_system
114:   END IF
115: END FUNCTION
116:
117: FUNCTION handle_quasispace_transition(session: &mut CampaignSession, portal: PortalType)
118:   IF portal == ToQuasispace THEN
119:     SAVE hyperspace coords
120:     SET session.in_quasispace = true
121:   ELSE
122:     RESTORE hyperspace coords from portal exit
123:     SET session.in_quasispace = false
124:   END IF
125: END FUNCTION
```

## Component 005: Encounter Handoff

```
130: FUNCTION build_battle(session: &CampaignSession) -> BattleSetup
131:   setup = BattleSetup::new()
132:   FOR EACH ship_fragment IN session.npc_ship_queue
133:     ADD ship_fragment to setup.npc_race_queue
134:   END FOR
135:   SELECT backdrop BASED ON session.current_activity
136:     IN_LAST_BATTLE => sa_matra_backdrop
137:     IN_HYPERSPACE => hyperspace_backdrop
138:     _ => planetary_backdrop
139:   INJECT SIS flagship into setup.player_queue
140:   RETURN setup
141: END FUNCTION
142:
143: FUNCTION encounter_battle(session: &mut CampaignSession) -> BattleResult
144:   SAVE previous_activity = session.current_activity
145:   SET session.battle_segue = true
146:   SET session.current_activity = InEncounter (or InLastBattle)
147:   SEED battle counters to zero
148:   result = invoke_battle(session)
149:   RESTORE session.current_activity = previous_activity
150:   RETURN result
151: END FUNCTION
152:
153: FUNCTION uninit_encounter(session: &mut CampaignSession, result: BattleResult)
154:   IF session.abort OR session.load_requested OR session.defeat OR session.is_last_battle THEN
155:     RETURN  // suppress post-encounter processing
156:   END IF
157:   CLEAR battle_segue flag
158:   DETERMINE victory_state from battle counters and story flags
159:   IDENTIFY encountered_race from npc_built_ship_queue
160:   IF victory THEN
161:     REMOVE defeated NPC ships from encounter queue
162:     AWARD salvage/resources as appropriate
163:     UPDATE campaign progression flags
164:   ELSE IF escape THEN
165:     // minimal cleanup
166:   ELSE  // defeat
167:     REMOVE destroyed escort ships from escort queue
168:   END IF
169:   CLEAN UP encounter state for navigation resume
170: END FUNCTION
```

## Component 006: Starbase Visit Flow

```
180: FUNCTION visit_starbase(session: &mut CampaignSession)
181:   IF is_bomb_transport_context(session) THEN
182:     handle_bomb_transport(session)
183:     RETURN
184:   END IF
185:   SET session.starbase_marker = true
186:   IF NOT is_allied(session) THEN
187:     run_commander_conversation(session)
188:     IF should_trigger_ilwrath_battle(session) THEN
189:       stage_ilwrath_response_battle(session)
190:       run_battle(session)
191:       run_commander_conversation(session)  // return to conversation after battle
192:     END IF
193:     RETURN
194:   END IF
195:   IF is_first_availability(session) OR is_post_bomb_installation(session) THEN
196:     advance_game_clock_days(14)
197:     run_forced_commander_conversation(session)
198:   END IF
199:   do_starbase_menu(session)
200: END FUNCTION
201:
202: FUNCTION do_starbase_menu(session: &mut CampaignSession)
203:   LOOP
204:     IF session.load_requested OR session.abort THEN BREAK
205:     choice = present_starbase_menu()
206:     MATCH choice
207:       Commander => run_commander_conversation(session)
208:       Outfit => run_outfit_screen(session)
209:       Shipyard => run_shipyard_screen(session)
210:       Depart => BREAK
211:     END MATCH
212:   END LOOP
213:   CLEAR starbase_visited flag
214:   request_deferred_transition(session, Interplanetary, START_INTERPLANETARY)
215: END FUNCTION
```

## Component 007: Campaign Event Handlers

```
220: FUNCTION add_initial_game_events(clock: &GameClock)
221:   clock.schedule(HYPERSPACE_ENCOUNTER_EVENT, relative(0, 1, 0))
222:   clock.schedule(ARILOU_ENTRANCE_EVENT, absolute(month=3, day=17, year=START_YEAR))
223:   clock.schedule(KOHR_AH_VICTORIOUS_EVENT, relative(0, 0, VICTORY_YEARS))
224:   clock.schedule(SLYLANDRO_RAMP_UP, immediate())
225: END FUNCTION
226:
227: FUNCTION event_handler(selector: EventSelector, session: &mut CampaignSession)
228:   MATCH selector
229:     ARILOU_ENTRANCE_EVENT =>
230:       SET arilou_portal_open = true
231:       SCHEDULE ARILOU_EXIT_EVENT in 3 days
232:     ARILOU_EXIT_EVENT =>
233:       SET arilou_portal_open = false
234:       SCHEDULE ARILOU_ENTRANCE_EVENT on day 17 of next month
235:     HYPERSPACE_ENCOUNTER_EVENT =>
236:       advance_faction_fleets(session)
237:       IF player_in_hyperspace(session) THEN check_encounter_generation(session)
238:       RESCHEDULE in 1 day
239:     KOHR_AH_VICTORIOUS_EVENT =>
240:       IF utwig_supox_counter_mission_active(session) THEN
241:         SCHEDULE KOHR_AH_GENOCIDE_EVENT in 1 year
242:       ELSE
243:         initiate_genocide(session)
244:       END IF
245:     ADVANCE_PKUNK_MISSION => advance_pkunk_migration(session)
246:     ADVANCE_THRADD_MISSION => advance_thraddash_arc(session)
247:     ZOQFOT_DISTRESS_EVENT => handle_zoqfot_distress(session)
248:     ZOQFOT_DEATH_EVENT => handle_zoqfot_death(session)
249:     SHOFIXTI_RETURN_EVENT => handle_shofixti_return(session)
250:     ADVANCE_UTWIG_SUPOX_MISSION => advance_utwig_supox(session)
251:     KOHR_AH_GENOCIDE_EVENT => handle_genocide(session)
252:     SPATHI_SHIELD_EVENT => handle_spathi_shield(session)
253:     ADVANCE_ILWRATH_MISSION => advance_ilwrath_war(session)
254:     ADVANCE_MYCON_MISSION => advance_mycon_mission(session)
255:     ARILOU_UMGAH_CHECK => handle_arilou_umgah(session)
256:     YEHAT_REBEL_EVENT => handle_yehat_rebellion(session)
257:     SLYLANDRO_RAMP_UP => handle_slylandro_ramp_up(session)
258:     SLYLANDRO_RAMP_DOWN => handle_slylandro_ramp_down(session)
259:   END MATCH
260: END FUNCTION
```

## Component 008: Save Serialization

```
270: FUNCTION save_game(session: &CampaignSession, slot: SaveSlot) -> Result<()>
271:   summary = prepare_summary(session)
272:   state = serialize_game_state(session)
273:   file = open_save_file(slot)?
274:   write_summary(file, summary)?
275:   write_game_state(file, state)?
276:   write_escort_queue(file, session.escort_queue)?
277:   write_npc_queue(file, session.npc_queue)?
278:   FOR EACH system WITH active_battle_groups
279:     save_battle_group_state_file(system)?
280:   END FOR
281:   write_encounter_queue(file, session.encounter_queue)?
282:   close_save_file(file)?
283:   RETURN Ok(())
284: END FUNCTION
285:
286: FUNCTION prepare_summary(session: &CampaignSession) -> SaveSummary
287:   MATCH session.current_activity
288:     InQuasispace => summary_type = "hyperspace", remap_coords_to_hyperspace(session.location)
289:     StarbaseVisit => summary_type = "starbase", location = "starbase:sol"
290:     PlanetOrbit => summary_type = "interplanetary", location = system_coords
291:     LastBattle => summary_type = "last_battle", location = "last_battle:sa_matra"
292:     Encounter => summary_type = "encounter", location = encounter_identity
293:     Hyperspace => summary_type = "hyperspace", location = hyperspace_coords
294:     Interplanetary => summary_type = "interplanetary", location = system_coords
295:   END MATCH
296:   summary.date = session.current_date()
297:   RETURN summary
298: END FUNCTION
299:
300: FUNCTION serialize_game_state(session: &CampaignSession) -> GameStateBlob
301:   blob = GameStateBlob::new()
302:   blob.write_activity(session.current_activity)
303:   blob.write_clock_state(session.clock_state())
304:   blob.write_autopilot(session.autopilot_target)
305:   blob.write_location(session.interplanetary_location)
306:   blob.write_ship_state(session.ship_stamp, session.orientation, session.velocity)
307:   blob.write_orbit_flags(session.orbit_flags)
308:   blob.write_game_state_bitfield(session.game_state_bits())
309:   RETURN blob
310: END FUNCTION
```

## Component 009: Load Deserialization and Validation

```
320: FUNCTION load_game(session: &mut CampaignSession, slot: SaveSlot) -> Result<()>
321:   file = open_save_file(slot)?
322:   summary = read_summary(file)?
323:   state_blob = read_game_state(file)?
324:   VALIDATE state_blob structure
325:   escort_data = read_escort_queue(file)?
326:   npc_data = read_npc_queue(file)?
327:   encounter_data = read_encounter_queue(file)?
328:   close_save_file(file)?
329:
330:   // Restore clock and validate scheduled events
331:   restored_clock = restore_clock_state(state_blob.clock_state)?
332:   VALIDATE_SCHEDULED_EVENTS(restored_clock)?  // §9.4.1 rejection
333:
334:   // Load adjunct artifacts for covered contexts
335:   IF needs_battle_group_adjuncts(state_blob) THEN
336:     FOR EACH required_system IN battle_group_systems(state_blob)
337:       load_battle_group_state_file(required_system)?
338:     END FOR
339:   END IF
340:
341:   // Commit point — all validation passed
342:   session.clear_all_state()
343:   session.restore_from(state_blob)
344:   session.restore_escort_queue(escort_data)
345:   session.restore_npc_queue(npc_data)
346:   session.restore_encounter_queue(encounter_data)
347:
348:   // Derive resume mode and normalize
349:   derive_resume_mode(session)
350:   IF session.is_interplanetary AND NOT session.is_starbase THEN
351:     ENSURE start_interplanetary flag set for solar system re-entry
352:   END IF
353:   IF session.is_starbase THEN
354:     SETUP starbase resume context
355:   END IF
356:
357:   RETURN Ok(())
358: END FUNCTION
359:
360: FUNCTION validate_scheduled_events(events: &[ScheduledEvent]) -> Result<()>
361:   FOR EACH event IN events
362:     IF event.selector NOT IN CAMPAIGN_EVENT_CATALOG THEN
363:       RETURN Err(UnknownEventSelector(event.selector))
364:     END IF
365:     IF NOT is_valid_date_encoding(event.due_date) THEN
366:       RETURN Err(InvalidEventMetadata(event))
367:     END IF
368:     IF NOT is_valid_event_metadata(event) THEN
369:       RETURN Err(InvalidEventMetadata(event))
370:     END IF
371:   END FOR
372:   RETURN Ok(())
373: END FUNCTION
374:
375: FUNCTION safe_failure_on_load(session: &mut CampaignSession, from_start_flow: bool)
376:   // Guarantee: no partial state from rejected save
377:   IF from_start_flow THEN
378:     RETURN to start/load flow
379:   ELSE
380:     // Pre-load session remains active
381:     RESTORE session to pre-load snapshot
382:   END IF
383:   // Guarantee: no save-slot mutation
384:   // Guarantee: no persisted state mutation
385: END FUNCTION
```

## Component 010: Campaign Canonical Export

```
390: FUNCTION export_canonical(save_path: &Path) -> Result<ExportDocument>
391:   file = open_save_file_readonly(save_path)?
392:   summary = read_summary(file)?
393:   state = read_game_state(file)?
394:   events = extract_scheduled_events(state)?
395:   VALIDATE events  // fail with error JSON if invalid
396:
397:   doc = ExportDocument::new()
398:   doc.schema_version = "1.0"
399:   doc.result = "success"
400:   doc.conformance_input_class = classify_input(state)
401:   doc.save_summary = normalize_summary(summary)
402:   doc.resume_context = derive_resume_context(state)
403:   doc.clock_state = extract_clock(state)
404:   doc.scheduled_events = canonicalize_events(events)
405:   doc.campaign_flags = extract_flags(state)
406:   doc.faction_state = extract_factions(state)
407:   doc.encounter_state = extract_encounter(state)
408:
409:   RETURN Ok(doc)
410: END FUNCTION
```

## Pseudocode Summary

| Component | Lines | Referenced By Phase(s) |
|-----------|-------|----------------------|
| 001: Campaign Loop | 001-031 | P09 |
| 002: Start Game | 040-073 | P09 |
| 003: Deferred Transition | 080-090 | P09 |
| 004: Hyperspace Transitions | 100-125 | P10 |
| 005: Encounter Handoff | 130-170 | P11 |
| 006: Starbase Visit | 180-215 | P12 |
| 007: Event Handlers | 220-260 | P04 |
| 008: Save Serialization | 270-310 | P06 |
| 009: Load & Validation | 320-385 | P07 |
| 010: Canonical Export | 390-410 | P14 |
