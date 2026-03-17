# Phase 02: Pseudocode

## Phase ID
`PLAN-20260314-PLANET-SOLARSYS.P02`

## Prerequisites
- Required: Phase 01a (Analysis Verification) completed

## Purpose

Define algorithmic pseudocode for every major component. Implementation phases reference specific line ranges from this pseudocode. Covers: planetary analysis, surface generation, scan/node flow, orbit entry, solar-system lifecycle, save-location encoding, generation-handler dispatch by class, and FFI-aware world identity handling.

---

## Component 1: Planetary Analysis (`calc.rs`)

```text
001: FUNCTION do_planetary_analysis(sys_info, planet_desc, star_desc)
002:   seed SysGenRNG from planet_desc.rand_seed
003:   star_energy = SUN_DATA[star_desc.Type].energy
004:   intensity = derive_intensity(star_desc.Type)
005:
006:   orbital_distance = compute_orbital_distance(planet_desc.radius)
007:   base_temperature = compute_temperature(star_energy, orbital_distance)
008:
009:   plan_data = PLAN_DATA[planet_desc.data_index]
010:   density = derive_density(plan_data, rng)
011:   radius = derive_radius(plan_data, rng)
012:   rotation_period = derive_rotation(plan_data, rng)
013:   gravity = compute_gravity(density, radius)
014:   tilt = derive_tilt(rng)
015:
016:   atmo_density = derive_atmosphere(plan_data, rng)
017:   IF atmo_density > 0
018:     greenhouse_adjust = compute_greenhouse(atmo_density, orbital_distance)
019:     surface_temp = base_temperature + greenhouse_adjust
020:   ELSE
021:     surface_temp = base_temperature
022:   END IF
023:
024:   tectonics = derive_tectonics(plan_data, density, rng)
025:   weather = derive_weather(atmo_density, surface_temp, rng)
026:   life_chance = derive_life_chance(plan_data, surface_temp, atmo_density, rng)
027:
028:   STORE all computed values into sys_info.PlanetInfo
029:   RETURN sys_info
030: END FUNCTION
031:
032: FUNCTION compute_temp_color(temperature)
033:   // Maps temperature to display color for solar-system view
034:   // Preserves greenhouse quirk: uses pre-greenhouse temperature for color
035:   SELECT temperature range
036:     CASE frozen: blue tint
037:     CASE cold: cyan tint
038:     CASE temperate: green tint
039:     CASE warm: yellow tint
040:     CASE hot: red tint
041:     CASE inferno: white/bright tint
042:   END SELECT
043:   RETURN color
044: END FUNCTION
```

## Component 2: Surface Generation (`surface.rs`, `gentopo.rs`)

```text
050: FUNCTION generate_planet_surface(world_ref, orbit_assets, surf_def_frame)
051:   seed SysGenRNG from world_ref.rand_seed
052:   init_planet_orbit_buffers(orbit_assets)
053:
054:   IF surf_def_frame IS PROVIDED
055:     load_predefined_surface(orbit_assets, surf_def_frame)
056:     RETURN renderable assets ready
057:   END IF
058:
059:   algo = select_algorithm(world_ref.data_index)
060:   MATCH algo
061:     GAS_GIANT_ALGO => generate_gas_giant_topo(orbit_assets.lpTopoData, rng)
062:     TOPO_ALGO => generate_topo_surface(orbit_assets.lpTopoData, rng)
063:     CRATERED_ALGO => generate_cratered_surface(orbit_assets.lpTopoData, rng)
064:   END MATCH
065:
066:   render_topography_frame(orbit_assets)
067:   build_sphere_rendering_assets(orbit_assets)
068: END FUNCTION
069:
070: FUNCTION delta_topography(num_iterations, depth_array, rect, depth_delta)
071:   FOR i IN 0..num_iterations
072:     random_line = generate_random_bisecting_line(rect, rng)
073:     FOR each pixel in rect
074:       IF pixel is on positive side of line
075:         depth_array[pixel] += depth_delta
076:       ELSE
077:         depth_array[pixel] -= depth_delta
078:       END IF
079:     END FOR
080:     depth_delta = adjust_delta(depth_delta, i, num_iterations)
081:   END FOR
082: END FUNCTION
```

## Component 3: Scan Flow & Node Materialization (`scan.rs`)

```text
090: FUNCTION scan_system(state, world_ref)
091:   prepare_scan_context(world_ref)
092:   restrictions = determine_scan_restrictions(world_ref)
093:   // Gas giant: restrict to certain scan types
094:   // Shielded: no scan access
095:   init_planet_location_display()
096:   draw_scanned_objects(existing_nodes)
097:   print_coarse_scan(state.sys_info.PlanetInfo)
098:   run_scan_input_loop(restrictions)
099:   cleanup_scan_display()
100: END FUNCTION
101:
102: FUNCTION generate_planet_side(state, world_ref)
103:   init_display_list()
104:   IF world_is_shielded(world_ref)
105:     RETURN  // no nodes on shielded worlds
106:   END IF
107:
108:   FOR scan_type IN [BIOLOGICAL, ENERGY, MINERAL]
109:     node_count = dispatch_data_provider_count(state.gen_dispatch, scan_type, world_ref)
110:     FOR node_index IN 0..node_count
111:       IF is_node_retrieved(state.scan_masks, scan_type, node_index)
112:         CONTINUE  // already picked up
113:       END IF
114:       node_info = dispatch_data_provider_node(state.gen_dispatch, scan_type, world_ref, node_index)
115:       element = allocate_display_element()
116:       populate_element(element, scan_type, node_info)
117:     END FOR
118:   END FOR
119: END FUNCTION
120:
121: FUNCTION is_node_retrieved(scan_masks, scan_type, node_index)
122:   mask = scan_masks[scan_type]
123:   RETURN (mask >> node_index) & 1 == 1
124: END FUNCTION
```

## Component 4: Orbit Entry & Orbital Menu (`orbit.rs`)

```text
130: FUNCTION enter_planet_orbit(state, orbit_target)
131:   IF entered_via_ip_collision
132:     free_ip_flight_assets()
133:   END IF
134:
135:   position_ship_stamp(orbit_target)
136:
137:   // Load persisted scan state BEFORE orbit-content processing
138:   identity = classify_world_for_persistence(state, orbit_target)
139:   scan_masks = persistence.get_planet_info(identity.star, identity.planet, identity.moon, identity.planet_num_moons)
140:   store_scan_masks(state.SysInfo, scan_masks)
141:
142:   // Dispatch orbit-content hook using audited override/fallback semantics
143:   orbital_result = dispatch_orbit_content(state.gen_dispatch, state, orbit_target)
144:   IF orbital_result == NOT_HANDLED
145:     default_generate_orbital(state, orbit_target)
146:   END IF
147:
148:   // Check activity interrupts
149:   IF activity_interrupt_active()
150:     RETURN OrbitalOutcome::Interrupted
151:   END IF
152:
153:   // Check orbital readiness by observable assets/flags, not by internal return convention
154:   IF state.TopoFrame IS NONE
155:     RETURN OrbitalOutcome::NoTopo
156:   END IF
157:
158:   // Planet loading (post-readiness)
159:   load_planet(state, orbit_target)
160:   planet_orbit_menu(state, orbit_target)
161:   free_planet(state)
162:
163:   // Post-orbit: reload system
164:   reload_solar_system()
165:   revalidate_orbits()
166:   RETURN OrbitalOutcome::Normal
167: END FUNCTION
168:
169: FUNCTION planet_orbit_menu(state, orbit_target)
170:   setup_rotating_planet_display()
171:   LOOP
172:     action = get_menu_input()
173:     MATCH action
174:       SCAN => scan_system(state, orbit_target)
175:       EQUIP_DEVICE => devices_menu()      // external
176:       CARGO => cargo_menu()               // external
177:       ROSTER => roster_menu()             // external
178:       GAME_MENU => game_options()         // external
179:       STARMAP | NAVIGATION => BREAK       // leave orbit
180:     END MATCH
181:   END LOOP
182: END FUNCTION
```

## Component 5: Solar-System Lifecycle (`solarsys.rs`)

```text
190: FUNCTION explore_solar_sys()
191:   star = resolve_current_star()
192:   update_sis_coordinates(star)
193:
194:   solar_sys = SolarSysState::new()
195:   gen_dispatch = get_generate_dispatch(star.Index)
196:   solar_sys.gen_dispatch = gen_dispatch
197:
198:   orbit_target = load_solar_sys(solar_sys, star)
199:
200:   IF orbit_target IS SOME
201:     enter_planet_orbit(solar_sys, orbit_target)
202:   END IF
203:
204:   do_ip_flight(solar_sys)
205:
206:   uninit_solar_sys(solar_sys)
207: END FUNCTION
208:
209: FUNCTION load_solar_sys(solar_sys, star)
210:   seed_sys_gen_rng(get_random_seed_for_star(star))
211:   setup_sun_descriptor(solar_sys, star)
212:
213:   planets_result = dispatch_planet_generation(solar_sys.gen_dispatch, solar_sys)
214:   IF planets_result == NOT_HANDLED
215:     default_generate_planets(solar_sys)
216:   END IF
217:
218:   // Commit pending planetary changes
219:   IF planetary_change_flag_set()
220:     assert_persistence_window_is_legal()
221:     persistence.put_planet_info(...)
222:     clear_planetary_change_flag()
223:   END IF
224:
225:   // Planetary analysis for temperature colors
226:   FOR EACH planet IN solar_sys.PlanetDesc[0..planet_count]
227:     do_planetary_analysis(solar_sys.SysInfo, planet, star)
228:     planet.temp_color = compute_temp_color(solar_sys.SysInfo.PlanetInfo.Temperature)
229:   END FOR
230:
231:   sort_planets_by_display_position(solar_sys)
232:
233:   IF saved_position_is_inner_system()
234:     target = make_planet_ref(saved_planet_index)
235:     moons_result = dispatch_moon_generation(solar_sys.gen_dispatch, solar_sys, target)
236:     IF moons_result == NOT_HANDLED
237:       default_generate_moons(solar_sys, target)
238:     END IF
239:     init_inner_system(solar_sys, target)
240:     IF saved_position_is_in_orbit()
241:       RETURN decode_orbit_target(solar_sys)
242:     END IF
243:   ELSE
244:     init_outer_system(solar_sys)
245:   END IF
246:   RETURN NONE
247: END FUNCTION
```

## Component 6: Save-Location Encoding (`save_location.rs`)

```text
250: FUNCTION save_solar_sys_location(state)
251:   IF NOT state.in_orbit
252:     save_non_orbital_location()
253:     RETURN
254:   END IF
255:
256:   // Commit pending scan changes inside the host-guaranteed persistence window
257:   IF planetary_change_flag_set()
258:     assert_persistence_window_is_legal()
259:     persistence.put_planet_info(...)
260:     clear_planetary_change_flag()
261:   END IF
262:
263:   target = current_orbit_target(state)
264:   identity = classify_world_for_persistence(state, target)
265:
266:   IF identity.moon == 0
267:     in_orbit_value = 1
268:   ELSE
269:     in_orbit_value = 1 + identity.moon
270:   END IF
271:
272:   store_global(in_orbit, in_orbit_value)
273: END FUNCTION
274:
275: FUNCTION decode_orbit_target(state)
276:   in_orbit_value = load_global(in_orbit)
277:   IF in_orbit_value == 0
278:     RETURN NONE
279:   ELSE IF in_orbit_value == 1
280:     RETURN current_inner_system_planet_ref(state)
281:   ELSE
282:     moon_slot = in_orbit_value - 1
283:     RETURN moon_ref_for_slot(state, moon_slot)
284:   END IF
285: END FUNCTION
```

## Component 7: Generation-handler dispatch (`generate.rs`)

```text
290: TYPE GenerateDispatch
291:   override_fallback_handlers
292:   data_provider_handlers
293:   side_effect_handlers
294: END TYPE
295:
296: FUNCTION dispatch_planet_generation(gen_dispatch, state)
297:   RETURN call_override_fallback_handler(gen_dispatch.planets, state)
298: END FUNCTION
299:
300: FUNCTION dispatch_moon_generation(gen_dispatch, state, planet_ref)
301:   RETURN call_override_fallback_handler(gen_dispatch.moons, state, planet_ref)
302: END FUNCTION
303:
304: FUNCTION dispatch_name_generation(gen_dispatch, state, world_ref)
305:   RETURN call_override_fallback_handler(gen_dispatch.name, state, world_ref)
306: END FUNCTION
307:
308: FUNCTION dispatch_orbit_content(gen_dispatch, state, world_ref)
309:   RETURN call_override_fallback_handler(gen_dispatch.orbit_content, state, world_ref)
310: END FUNCTION
311:
312: FUNCTION dispatch_data_provider_count(gen_dispatch, scan_type, world_ref)
313:   SELECT scan_type
314:     CASE MINERAL => RETURN call_count_provider(gen_dispatch.minerals, world_ref)
315:     CASE ENERGY => RETURN call_count_provider(gen_dispatch.energy, world_ref)
316:     CASE BIOLOGICAL => RETURN call_count_provider(gen_dispatch.life, world_ref)
317:   END SELECT
318: END FUNCTION
319:
320: FUNCTION dispatch_data_provider_node(gen_dispatch, scan_type, world_ref, node_index)
321:   SELECT scan_type
322:     CASE MINERAL => RETURN call_node_provider(gen_dispatch.minerals, world_ref, node_index)
323:     CASE ENERGY => RETURN call_node_provider(gen_dispatch.energy, world_ref, node_index)
324:     CASE BIOLOGICAL => RETURN call_node_provider(gen_dispatch.life, world_ref, node_index)
325:   END SELECT
326: END FUNCTION
327:
328: FUNCTION dispatch_side_effect_hook(gen_dispatch, hook_type, args)
329:   call_hook_for_effect(gen_dispatch[hook_type], args)
330: END FUNCTION
331:
332: FUNCTION get_generate_dispatch(star_index)
333:   IF star_index HAS dedicated table
334:     RETURN wrapped_c_dispatch_for_star(star_index)
335:   ELSE
336:     RETURN default_generate_dispatch()
337:   END IF
338: END FUNCTION
```

## Component 8: World Classification (`world_class.rs`)

```text
340: TYPE WorldRef = PlanetRef(planet_index) | MoonRef(planet_index, moon_index)
341:
342: FUNCTION world_is_planet(world_ref) -> bool
343:   RETURN world_ref IS PlanetRef
344: END FUNCTION
345:
346: FUNCTION world_is_moon(world_ref) -> bool
347:   RETURN world_ref IS MoonRef
348: END FUNCTION
349:
350: FUNCTION planet_index(world_ref) -> usize
351:   MATCH world_ref
352:     PlanetRef(planet_i) => RETURN planet_i
353:     MoonRef(planet_i, moon_i) => RETURN planet_i
354:   END MATCH
355: END FUNCTION
356:
357: FUNCTION moon_index(world_ref) -> Option<usize>
358:   MATCH world_ref
359:     PlanetRef(_) => RETURN NONE
360:     MoonRef(_, moon_i) => RETURN moon_i
361:   END MATCH
362: END FUNCTION
363:
364: FUNCTION match_world(world_ref, planet_i, moon_i) -> bool
365:   IF moon_i == MATCH_PLANET
366:     RETURN world_ref == PlanetRef(planet_i)
367:   ELSE
368:     RETURN world_ref == MoonRef(planet_i, moon_i)
369:   END IF
370: END FUNCTION
```

## Component 9: Navigation (`navigation.rs`)

```text
380: FUNCTION do_ip_flight(solar_sys)
381:   solar_sys.InIpFlight = true
382:   LOOP
383:     process_input(solar_sys)
384:     update_ship_position(solar_sys)
385:
386:     FOR EACH body_ref IN current_body_set(solar_sys)
387:       IF ship_intersects(body_ref) AND NOT gated(body_ref, solar_sys.WaitIntersect)
388:         IF in_outer_system
389:           enter_inner_system(solar_sys, body_ref)
390:         ELSE
391:           enter_planet_orbit(solar_sys, body_ref)
392:         END IF
393:       END IF
394:     END FOR
395:
396:     IF leaving_system_boundary
397:       BREAK
398:     END IF
399:
400:     render_system_view(solar_sys)
401:   END LOOP
402:   solar_sys.InIpFlight = false
403: END FUNCTION
404:
405: FUNCTION enter_inner_system(solar_sys, planet_ref)
406:   moons_result = dispatch_moon_generation(solar_sys.gen_dispatch, solar_sys, planet_ref)
407:   IF moons_result == NOT_HANDLED
408:     default_generate_moons(solar_sys, planet_ref)
409:   END IF
410:   solar_sys.base_desc = moons_for(planet_ref)
411:   solar_sys.orbital_target = planet_ref
412:   switch_to_inner_view()
413: END FUNCTION
414:
415: FUNCTION leave_inner_system(solar_sys)
416:   solar_sys.base_desc = planets
417:   solar_sys.orbital_target = NONE
418:   switch_to_outer_view()
419: END FUNCTION
```
