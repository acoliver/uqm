# Phase 02: Pseudocode

## Phase ID
`PLAN-20260314-SHIPS.P02`

## Prerequisites
- Required: Phase 01a (Analysis Verification) completed and PASS

## Purpose
Algorithmic pseudocode for all major ship subsystem components. Implementation phases reference line ranges from these pseudocode blocks.

## Component 1: Ship Registry / Dispatch

```text
01: FUNCTION create_ship_behavior(species_id: SpeciesId) -> Result<Box<dyn ShipBehavior>>
02:   MATCH species_id
03:     Arilou => ArilouShip::new()
04:     Chmmr => ChmmrShip::new()
05:     Earthling => HumanShip::new()
06:     ... (all 28 species mapped)
07:     _ => RETURN Err(UnknownSpecies)
08:   RETURN Ok(boxed behavior)
09: END
10:
11: FUNCTION create_race_desc(species_id: SpeciesId) -> Result<RaceDesc>
12:   LET behavior = create_ship_behavior(species_id)
13:   LET template = behavior.descriptor_template()
14:   LET desc = RaceDesc {
15:     ship_info: template.ship_info.clone(),
16:     fleet: template.fleet.clone(),
17:     characteristics: template.characteristics.clone(),
18:     ship_data: ShipData::default(),  // loaded later by loader
19:     intel: template.intel.clone(),
20:     behavior: behavior,
21:     private_data: None,
22:   }
23:   RETURN Ok(desc)
24: END
```

## Component 2: Two-Tier Ship Loader

```text
30: ENUM LoadTier { MetadataOnly, BattleReady }
31:
32: FUNCTION load_ship(species_id: SpeciesId, tier: LoadTier) -> Result<RaceDesc>
33:   LET desc = create_race_desc(species_id)
34:   IF desc IS Err THEN RETURN desc
35:
36:   // Metadata loading (both tiers)
37:   desc.ship_info.icons = load_resource(desc.ship_info.icons_res)?
38:   desc.ship_info.melee_icon = load_resource(desc.ship_info.melee_icon_res)?
39:   desc.ship_info.race_strings = load_string_table(desc.ship_info.race_strings_res)?
40:
41:   IF tier == MetadataOnly THEN
42:     RETURN Ok(desc)
43:   END
44:
45:   // Battle-ready loading
46:   FOR each resolution_level IN [Big, Med, Sml]
47:     desc.ship_data.ship[level] = load_graphic(desc.ship_info.ship_res[level])?
48:     desc.ship_data.weapon[level] = load_graphic_if_present(desc.ship_info.weapon_res[level])
49:     desc.ship_data.special[level] = load_graphic_if_present(desc.ship_info.special_res[level])
50:   END
51:   desc.ship_data.captain.background = load_graphic(desc.ship_info.captain_res)?
52:   desc.ship_data.victory_ditty = load_music(desc.ship_info.victory_res)?
53:   desc.ship_data.ship_sounds = load_sound(desc.ship_info.sounds_res)?
54:
55:   RETURN Ok(desc)
56: END
57:
58: FUNCTION free_ship(desc: &mut RaceDesc, free_battle: bool, free_metadata: bool)
59:   // Invoke race teardown hook
60:   desc.behavior.uninit()
61:
62:   IF free_battle THEN
63:     free_graphic(desc.ship_data.ship)
64:     free_graphic(desc.ship_data.weapon)
65:     free_graphic(desc.ship_data.special)
66:     free_graphic(desc.ship_data.captain.background)
67:     free_music(desc.ship_data.victory_ditty)
68:     free_sound(desc.ship_data.ship_sounds)
69:   END
70:
71:   IF free_metadata THEN
72:     free_resource(desc.ship_info.icons)
73:     free_resource(desc.ship_info.melee_icon)
74:     free_string_table(desc.ship_info.race_strings)
75:   END
76: END
```

## Component 3: Master Ship Catalog

```text
80: STRUCT MasterShipInfo {
81:   species_id: SpeciesId,
82:   ship_info: ShipInfo,
83:   fleet: FleetStuff,
84: }
85:
86: FUNCTION load_master_ship_list() -> Result<()>
87:   LET catalog = Vec::new()
88:   FOR species_id IN ARILOU_ID..=LAST_MELEE_ID
89:     LET desc = load_ship(species_id, MetadataOnly)?
90:     LET entry = MasterShipInfo {
91:       species_id,
92:       ship_info: desc.ship_info.clone(),
93:       fleet: desc.fleet.clone(),
94:     }
95:     // Free the full descriptor (we only keep metadata copies)
96:     free_ship(desc, false, false)  // icons/strings transferred to entry
97:     catalog.push(entry)
98:   END
99:   // Sort by race name
100:  catalog.sort_by(|a, b| a.race_name().cmp(&b.race_name()))
101:  STORE catalog in global MASTER_CATALOG
102:  RETURN Ok(())
103: END
104:
105: FUNCTION free_master_ship_list()
106:   FOR entry IN MASTER_CATALOG
107:     free_resource(entry.ship_info.icons)
108:     free_resource(entry.ship_info.melee_icon)
109:     free_string_table(entry.ship_info.race_strings)
110:   END
111:   CLEAR MASTER_CATALOG
112: END
113:
114: FUNCTION find_master_ship(species_id: SpeciesId) -> Option<&MasterShipInfo>
115:   MASTER_CATALOG.iter().find(|e| e.species_id == species_id)
116: END
117:
118: FUNCTION find_master_ship_by_index(index: usize) -> Option<&MasterShipInfo>
119:   MASTER_CATALOG.get(index)
120: END
121:
122: FUNCTION get_ship_cost(index: usize) -> Option<u16>
123:   find_master_ship_by_index(index).map(|e| e.ship_info.ship_cost)
124: END
```

## Component 4: Queue & Build Primitives

```text
130: FUNCTION build_ship(queue: &mut Queue<Starship>, species_id: SpeciesId) -> Result<Handle>
131:   LET entry = Starship::new(species_id)
132:   entry.species_id = species_id
133:   // Zero-init crew, energy, flags
134:   LET handle = queue.insert(entry)
135:   RETURN Ok(handle)
136: END
137:
138: FUNCTION get_starship_from_index(queue: &Queue<Starship>, index: usize) -> Option<&Starship>
139:   queue.get_by_index(index)
140: END
141:
142: FUNCTION clone_ship_fragment(src_fleet: &FleetInfo, src_queue: &Queue, dst: &mut ShipFragment)
143:   dst.species_id = src_fleet.species_id
144:   dst.crew_level = src_fleet.crew_level  // or from ship_info
145:   dst.max_crew = src_fleet.max_crew
146:   dst.energy_level = src_fleet.max_energy
147:   dst.max_energy = src_fleet.max_energy
148:   dst.icons = clone_handle(src.icons)
149:   dst.melee_icon = clone_handle(src.melee_icon)
150:   dst.race_strings = clone_handle(src.race_strings)
151: END
152:
153: FUNCTION add_escort_ships(count: u8, species_id: SpeciesId) -> usize
154:   LET added = 0
155:   FOR i IN 0..count
156:     IF built_ship_q.len() < MAX_BUILT_SHIPS
157:       build_ship(&mut built_ship_q, species_id)
158:       added += 1
159:     END
160:   END
161:   RETURN added
162: END
```

## Component 5: Shared Runtime Pipeline

```text
170: FUNCTION ship_preprocess(element: &mut Element, ship: &mut Starship)
171:   LET desc = ship.race_desc.as_mut()
172:   LET status = ship.cur_status_flags
173:
174:   // First-frame setup
175:   IF element.state_flags.APPEARING THEN
176:     element.state_flags.REMOVE(APPEARING)
177:     element.life_span = NORMAL_LIFE
178:     ship.ShipFacing = initial_facing()
179:     init_status_display(ship)
180:   END
181:
182:   // Race-specific preprocess
183:   desc.behavior.preprocess(ship_state, battle_ctx)?
184:
185:   // Energy regeneration
186:   IF desc.characteristics.energy_wait > 0 THEN
187:     IF ship.energy_counter >= desc.characteristics.energy_wait THEN
188:       ship.energy_counter = 0
189:       IF ship.energy_level < desc.ship_info.max_energy THEN
190:         ship.energy_level += desc.characteristics.energy_regeneration
191:         IF ship.energy_level > desc.ship_info.max_energy THEN
192:           ship.energy_level = desc.ship_info.max_energy
193:         END
194:       END
195:     ELSE
196:       ship.energy_counter += 1
197:     END
198:   END
199:
200:   // Turn handling
201:   IF status.LEFT THEN
202:     IF ship.turn_counter == 0 THEN
203:       ship.ShipFacing = NORMALIZE_FACING(ship.ShipFacing - 1)
204:       ship.turn_counter = desc.characteristics.turn_wait
205:     END
206:   ELSE IF status.RIGHT THEN
207:     IF ship.turn_counter == 0 THEN
208:       ship.ShipFacing = NORMALIZE_FACING(ship.ShipFacing + 1)
209:       ship.turn_counter = desc.characteristics.turn_wait
210:     END
211:   END
212:   IF ship.turn_counter > 0 THEN ship.turn_counter -= 1 END
213:
214:   // Thrust handling
215:   IF status.THRUST THEN
216:     inertial_thrust(element, ship, desc)
217:   END
218: END
219:
220: FUNCTION inertial_thrust(element: &mut Element, ship: &Starship, desc: &RaceDesc)
221:   IF ship.thrust_counter == 0 THEN
222:     apply_acceleration(element, desc.characteristics.thrust_increment, ship.ShipFacing)
223:     cap_velocity(element, desc.characteristics.max_thrust)
224:     ship.thrust_counter = desc.characteristics.thrust_wait
225:   END
226:   IF ship.thrust_counter > 0 THEN ship.thrust_counter -= 1 END
227: END
228:
229: FUNCTION ship_postprocess(element: &mut Element, ship: &mut Starship)
230:   LET desc = ship.race_desc.as_mut()
231:   LET status = ship.cur_status_flags
232:
233:   // Weapon fire
234:   IF status.WEAPON AND NOT ship.weapon_cooling THEN
235:     IF ship.energy_level >= desc.characteristics.weapon_energy_cost THEN
236:       IF ship.weapon_counter == 0 THEN
237:         LET weapons = desc.behavior.init_weapon(ship_state, battle_ctx)?
238:         FOR w IN weapons
239:           add_element_to_display_list(w)
240:         END
241:         ship.energy_level -= desc.characteristics.weapon_energy_cost
242:         ship.weapon_counter = desc.characteristics.weapon_wait
243:         process_sound(WEAPON_SOUND)
244:       END
245:     END
246:   END
247:
248:   // Special activation
249:   IF status.SPECIAL AND NOT ship.special_cooling THEN
250:     IF ship.energy_level >= desc.characteristics.special_energy_cost THEN
251:       IF ship.special_counter == 0 THEN
252:         // Race postprocess handles special behavior
253:         ship.energy_level -= desc.characteristics.special_energy_cost
254:         ship.special_counter = desc.characteristics.special_wait
255:       END
256:     END
257:   END
258:
259:   // Race-specific postprocess
260:   desc.behavior.postprocess(ship_state, battle_ctx)?
261:
262:   // Cooldown updates
263:   IF ship.weapon_counter > 0 THEN ship.weapon_counter -= 1 END
264:   IF ship.special_counter > 0 THEN ship.special_counter -= 1 END
265: END
```

## Component 6: Ship Spawn

```text
270: FUNCTION spawn_ship(starship: &mut Starship) -> Result<()>
271:   // Load battle-ready descriptor
272:   LET desc = load_ship(starship.species_id, BattleReady)?
273:
274:   // Patch crew from queue entry
275:   desc.ship_info.crew_level = starship.crew_level
276:
277:   // Bind descriptor to queue entry
278:   starship.race_desc = Some(desc)
279:
280:   // Allocate ship element
281:   LET element = alloc_element()
282:   IF element IS None THEN RETURN Err(SpawnFailed) END
283:
284:   // Configure element
285:   element.playerNr = starship.playerNr
286:   element.state_flags = APPEARING | PLAYER_SHIP | FINITE_LIFE
287:   element.mass_points = desc.characteristics.ship_mass
288:   element.current.image = desc.ship_data.ship[current_resolution]
289:   element.life_span = NORMAL_LIFE
290:
291:   // Bind callbacks
292:   element.preprocess_func = ship_preprocess
293:   element.postprocess_func = ship_postprocess
294:   element.death_func = ship_death
295:   element.collision_func = desc.behavior.collision_override()
296:     .unwrap_or(default_ship_collision)
297:
298:   // Register element
299:   starship.hShip = insert_element(element)
300:   set_element_starship(element, starship)
301:
302:   RETURN Ok(())
303: END
```

## Component 7: Ship Death & Crew Writeback

```text
310: FUNCTION ship_death(element: &Element, ship: &mut Starship)
311:   // Execute death-specific behavior (explosion, crew scatter)
312:   spawn_explosion_elements(element)
313:   scatter_crew_elements(element, ship.race_desc.ship_info.crew_level)
314:   play_death_sound()
315: END
316:
317: FUNCTION new_ship_transition(dead_ship: &mut Starship)
318:   // Stop audio
319:   stop_ship_sounds()
320:
321:   // Free dead ship descriptor
322:   IF dead_ship.race_desc.is_some() THEN
323:     free_ship(dead_ship.race_desc.take(), true, true)
324:   END
325:
326:   // Write back crew (0 for dead ship)
327:   update_ship_frag_crew(dead_ship, 0)
328:
329:   // Mark inactive
330:   dead_ship.state_flags |= DEAD_SHIP
331:
332:   // Request next ship from battle engine
333:   // (battle engine owns selection policy)
334: END
335:
336: FUNCTION update_ship_frag_crew(ship: &Starship, crew: u16)
337:   // Find matching fragment by queue order and species
338:   FOR frag IN ship_fragment_queue
339:     IF frag.species_id == ship.species_id AND NOT frag.processed THEN
340:       frag.crew_level = crew
341:       frag.processed = true
342:       RETURN
343:     END
344:   END
345: END
346:
347: FUNCTION battle_teardown_writeback(race_q: &Queue<Starship>)
348:   // Count floating crew elements
349:   LET floating_crew = count_floating_crew_elements()
350:
351:   FOR ship IN race_q
352:     IF ship.race_desc.is_some() THEN
353:       LET final_crew = ship.race_desc.ship_info.crew_level
354:       update_ship_frag_crew(ship, final_crew)
355:       free_ship(ship.race_desc.take(), true, true)
356:     END
357:   END
358: END
```

## Component 8: Battle Lifecycle

```text
360: FUNCTION init_ships() -> Result<u32>
361:   init_space()?
362:
363:   // Initialize display list
364:   init_display_list()
365:
366:   // Initialize galaxy/background
367:   init_galaxy()
368:
369:   // Mode-specific setup
370:   IF hyperspace_mode() THEN
371:     setup_sis_for_hyperspace()
372:   ELSE
373:     setup_battle_space()
374:     IF has_asteroids() THEN spawn_asteroids() END
375:     IF has_planet() THEN spawn_planet() END
376:   END
377:
378:   RETURN Ok(NUM_SIDES)
379: END
380:
381: FUNCTION uninit_ships()
382:   stop_all_audio()
383:   uninit_space()
384:
385:   // Count floating crew
386:   LET floating = count_floating_crew_elements()
387:
388:   // Free all active ships and write back crew
389:   FOR side IN 0..NUM_SIDES
390:     battle_teardown_writeback(&race_q[side])
391:   END
392:
393:   // Clear battle state
394:   clear_in_battle()
395:   reinit_queues()
396: END
```

## Component 9: Per-Race Ship Behavior (Template)

```text
400: STRUCT ExampleShip {
401:   // Per-instance private state
402:   mode: ShipMode,
403:   counter: u16,
404: }
405:
406: IMPL ShipBehavior FOR ExampleShip
407:   FUNCTION descriptor_template() -> RaceDescTemplate
408:     RETURN RaceDescTemplate {
409:       ship_info: ShipInfo { flags: FIRES_FORE, cost: 15, max_crew: 20, ... },
410:       fleet: FleetStuff { ... },
411:       characteristics: Characteristics { max_thrust: 24, ... },
412:       intel: IntelStuff { maneuverability: 0, weapon_range: LASER_RANGE / 2 },
413:     }
414:   END
415:
416:   FUNCTION preprocess(ship: &mut ShipState, ctx: &BattleContext) -> Result<()>
417:     // Race-specific per-frame preprocess logic
418:     IF self.mode == Blazer THEN
419:       ship.characteristics.max_thrust = BLAZER_THRUST
420:       // ... mode-specific behavior
421:     END
422:     RETURN Ok(())
423:   END
424:
425:   FUNCTION postprocess(ship: &mut ShipState, ctx: &BattleContext) -> Result<()>
426:     // Race-specific postprocess
427:     RETURN Ok(())
428:   END
429:
430:   FUNCTION init_weapon(ship: &ShipState, ctx: &BattleContext) -> Result<Vec<WeaponElement>>
431:     LET laser = initialize_laser(ship.position, ship.facing, LASER_RANGE)
432:     RETURN Ok(vec![laser])
433:   END
434:
435:   FUNCTION intelligence(ship: &ShipState, ctx: &BattleContext) -> StatusFlags
436:     // AI decision logic
437:     LET flags = StatusFlags::empty()
438:     IF target_in_range(ship, ctx) THEN flags |= WEAPON END
439:     IF should_thrust(ship, ctx) THEN flags |= THRUST END
440:     RETURN flags
441:   END
442:
443:   FUNCTION uninit(&mut self)
444:     // Cleanup private state (automatic in Rust via Drop)
445:   END
446: END
```

## Pseudocode Summary

| Component | Lines | Referenced By Phase |
|-----------|-------|-------------------|
| Ship Registry / Dispatch | 01-24 | P04 |
| Two-Tier Ship Loader | 30-76 | P05 |
| Master Ship Catalog | 80-124 | P06 |
| Queue & Build Primitives | 130-162 | P07 |
| Shared Runtime Pipeline | 170-265 | P08 |
| Ship Spawn | 270-303 | P09 |
| Ship Death & Crew Writeback | 310-358 | P10 |
| Battle Lifecycle | 360-396 | P09 |
| Per-Race Behavior Template | 400-446 | P11-P13 |
