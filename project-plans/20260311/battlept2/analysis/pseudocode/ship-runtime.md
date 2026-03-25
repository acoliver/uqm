# Ship Runtime Pseudocode (`sc2/src/uqm/ship.c`)

## 1) `animation_preprocess()` (`ship.c:46-87`)

```text
001 function animation_preprocess(element_ptr: element_t*) -> void
002     if element_ptr.turn_wait > 0 then
003         element_ptr.turn_wait = element_ptr.turn_wait - 1
004     else
005         element_ptr.next.image.frame = inc_frame_index(element_ptr.current.image.frame)  // ffi_call
006         element_ptr.state_flags = element_ptr.state_flags OR changing_flag
007         element_ptr.turn_wait = element_ptr.next_turn
008     end if
009 end function
```

Phase 1 type usage:
- `element_t*` maps to C `ELEMENT*`.

FFI calls:
- `inc_frame_index(...)` (C: `IncFrameIndex`).

---

## 2) `inertial_thrust()` (`ship.c:89-157`)

```text
001 function inertial_thrust(element_ptr: element_t*) -> status_flags_t
002     const max_allowed_speed = world_to_velocity(display_to_world(18))  // ffi_call
003     const max_allowed_speed_sqr = max_allowed_speed * max_allowed_speed
004
005     velocity_ptr = &element_ptr.velocity
006
007     starship_ptr: starship_t* = null
008     get_element_starship(element_ptr, &starship_ptr)  // ffi_call
009
010     current_angle = facing_to_angle(starship_ptr.ship_facing)  // ffi_call
011     travel_angle = get_velocity_travel_angle(velocity_ptr)  // ffi_call
012
013     thrust_increment = starship_ptr.race_desc_ptr.characteristics.thrust_increment
014     max_thrust = starship_ptr.race_desc_ptr.characteristics.max_thrust
015
016     if thrust_increment == max_thrust then
017         // inertialess acceleration (example: skiff)
018         set_velocity_vector(velocity_ptr, max_thrust, starship_ptr.ship_facing)  // ffi_call
019         return ship_at_max_speed
020     end if
021
022     if travel_angle == current_angle
023        and (starship_ptr.cur_status_flags has_any (ship_at_max_speed, ship_beyond_max_speed))
024        and not (starship_ptr.cur_status_flags has ship_in_gravity_well) then
025         return (starship_ptr.cur_status_flags AND (ship_at_max_speed OR ship_beyond_max_speed))
026     end if
027
028     thrust_increment_v = world_to_velocity(thrust_increment)  // ffi_call
029
030     cur_dx, cur_dy = get_current_velocity_components(velocity_ptr)  // ffi_call
031     current_speed = velocity_squared(cur_dx, cur_dy)  // ffi_call
032
033     dx = cur_dx + cosine(current_angle, thrust_increment_v)  // ffi_call
034     dy = cur_dy + sine(current_angle, thrust_increment_v)  // ffi_call
035     desired_speed = velocity_squared(dx, dy)  // ffi_call
036     max_speed = velocity_squared(world_to_velocity(max_thrust), 0)  // ffi_call
037
038     if desired_speed <= max_speed then
039         // normal acceleration
040         set_velocity_components(velocity_ptr, dx, dy)  // ffi_call
041         return 0
042     end if
043
044     if ((starship_ptr.cur_status_flags has ship_in_gravity_well) and desired_speed <= max_allowed_speed_sqr)
045         or (desired_speed < current_speed) then
046         // gravity-well allowed overspeed OR deceleration after whip
047         set_velocity_components(velocity_ptr, dx, dy)  // ffi_call
048         return (ship_at_max_speed OR ship_beyond_max_speed)
049     end if
050
051     if travel_angle == current_angle then
052         // saturated acceleration, same vector
053         if current_speed <= max_speed then
054             set_velocity_vector(velocity_ptr, max_thrust, starship_ptr.ship_facing)  // ffi_call
055         end if
056         return ship_at_max_speed
057     end if
058
059     // saturated acceleration at angle: rotate vector without increasing true speed
060     v = copy(*velocity_ptr)
061     delta_velocity_components(
062         &v,
063         cosine(current_angle, thrust_increment_v >> 1) - cosine(travel_angle, thrust_increment_v),  // ffi_call
064         sine(current_angle, thrust_increment_v >> 1) - sine(travel_angle, thrust_increment_v)       // ffi_call
065     )  // ffi_call
066
067     cur_dx2, cur_dy2 = get_current_velocity_components(&v)  // ffi_call
068     desired_speed2 = velocity_squared(cur_dx2, cur_dy2)  // ffi_call
069
070     if desired_speed2 > max_speed then
071         if desired_speed2 < current_speed then
072             *velocity_ptr = v
073         end if
074         return (ship_at_max_speed OR ship_beyond_max_speed)
075     end if
076
077     *velocity_ptr = v
078     return 0
079 end function
```

Phase 1 type usage:
- `element_t*` (`ELEMENT*`), `starship_t*` (`STARSHIP*`), `velocity_desc_t` (`VELOCITY_DESC`).
- return `status_flags_t` (`STATUS_FLAGS`).

FFI calls:
- `world_to_velocity`, `display_to_world`, `get_element_starship`, `facing_to_angle`, `get_velocity_travel_angle`, `get_current_velocity_components`, `velocity_squared`, `cosine`, `sine`, `set_velocity_vector`, `set_velocity_components`, `delta_velocity_components`.

---

## 3) `ship_preprocess()` (`ship.c:159-293`) — 7-stage pipeline

`USE_RUST_SHIPS` variants:
- Rust build path: immediate delegate `rust_ships_preprocess(element_ptr)` and return.
- C build path: full 7-stage logic below.

```text
001 function ship_preprocess(element_ptr: element_t*) -> void
002 #if use_rust_ships
003     rust_ships_preprocess(element_ptr)  // ffi_call (rust bridge)
004     return
005 #else
006     // stage_1: resolve ship context and input baseline
007     starship_ptr: starship_t* = null
008     get_element_starship(element_ptr, &starship_ptr)  // ffi_call
009     rd_ptr = starship_ptr.race_desc_ptr
010
011     cur_status_flags = starship_ptr.cur_status_flags AND NOT (left_flag OR right_flag OR thrust_flag OR weapon_flag OR special_flag)
012
013     if not (element_ptr.state_flags has appearing_flag) then
014         cur_status_flags = cur_status_flags OR (starship_ptr.ship_input_state AND (left_flag OR right_flag OR thrust_flag OR weapon_flag OR special_flag))
015     else
016         // stage_2: first-frame spawn/appearance initialization
017         element_ptr.crew_level = rd_ptr.ship_info.crew_level
018
019         if element_ptr.player_nr == npc_player_num and low_byte(global_current_activity()) == in_last_battle then
020             // sa-matra status backdrop draw path
021             old_context = set_context(status_context)  // ffi_call
022             stamp.origin = (0,0)
023             stamp.frame = rd_ptr.ship_data.captain_control.background
024             draw_stamp(&stamp)  // ffi_call
025             destroy_drawable(release_drawable(stamp.frame))  // ffi_call
026             rd_ptr.ship_data.captain_control.background = 0
027             set_context(old_context)  // ffi_call
028
029         else if low_byte(global_current_activity()) <= in_encounter then
030             // normal encounter HUD init + possible ship transition
031             init_ship_status(&rd_ptr.ship_info, starship_ptr, null)  // ffi_call
032             old_context = set_context(status_context)  // ffi_call
033             draw_captains_window(starship_ptr)  // ffi_call
034             set_context(old_context)  // ffi_call
035
036             if rd_ptr.preprocess_func != null then
037                 rd_ptr.preprocess_func(element_ptr)  // ffi_call (species hook)
038             end if
039
040             if element_ptr.h_target == 0 then
041                 ship_transition(element_ptr)  // ffi_call
042             else
043                 // pkunk reincarnation path
044                 element_ptr.h_target = 0
045                 if not plr_playing(all_music_ref) and opponent_alive(starship_ptr) then  // ffi_call
046                     battle_song(true)  // ffi_call
047                 end if
048             end if
049             return
050
051         else
052             // hyperspace/other placement bootstrap
053             element_ptr.current.location = (log_space_width/2, log_space_height/2)
054             element_ptr.next.location = element_ptr.current.location
055             init_intersect_start_point(element_ptr)  // ffi_call
056             init_intersect_end_point(element_ptr)  // ffi_call
057
058             if hyper_transition(element_ptr) then  // ffi_call
059                 return
060             end if
061         end if
062     end if
063
064     // stage_3: commit status flags and energy regeneration tick
065     starship_ptr.cur_status_flags = cur_status_flags
066
067     if starship_ptr.energy_counter > 0 then
068         starship_ptr.energy_counter = starship_ptr.energy_counter - 1
069     else if rd_ptr.ship_info.energy_level < rd_ptr.ship_info.max_energy
070          or rd_ptr.characteristics.energy_regeneration < 0 then
071         delta_energy(element_ptr, rd_ptr.characteristics.energy_regeneration)  // ffi_call
072     end if
073
074     // stage_4: species preprocess hook (can mutate status flags)
075     if rd_ptr.preprocess_func != null then
076         rd_ptr.preprocess_func(element_ptr)  // ffi_call
077         cur_status_flags = starship_ptr.cur_status_flags
078     end if
079
080     // stage_5: turning and frame update
081     if element_ptr.turn_wait > 0 then
082         element_ptr.turn_wait = element_ptr.turn_wait - 1
083     else if cur_status_flags has_any (left_flag, right_flag) then
084         if cur_status_flags has left_flag then
085             starship_ptr.ship_facing = normalize_facing(starship_ptr.ship_facing - 1)  // ffi_call
086         else
087             starship_ptr.ship_facing = normalize_facing(starship_ptr.ship_facing + 1)  // ffi_call
088         end if
089
090         element_ptr.next.image.frame = set_abs_frame_index(element_ptr.next.image.frame, starship_ptr.ship_facing)  // ffi_call
091         element_ptr.state_flags = element_ptr.state_flags OR changing_flag
092         element_ptr.turn_wait = rd_ptr.characteristics.turn_wait
093     end if
094
095     // stage_6: thrust cadence, velocity integration, ion trail spawn
096     if element_ptr.thrust_wait > 0 then
097         element_ptr.thrust_wait = element_ptr.thrust_wait - 1
098     else if cur_status_flags has thrust_flag then
099         thrust_status = inertial_thrust(element_ptr)
100
101         starship_ptr.cur_status_flags = starship_ptr.cur_status_flags AND NOT (ship_at_max_speed OR ship_beyond_max_speed OR ship_in_gravity_well)
102         starship_ptr.cur_status_flags = starship_ptr.cur_status_flags OR thrust_status
103
104         element_ptr.thrust_wait = rd_ptr.characteristics.thrust_wait
105
106         if not object_cloaked(element_ptr) and low_byte(global_current_activity()) <= in_encounter then
107             spawn_ion_trail(element_ptr)  // ffi_call
108         end if
109     end if
110
111     // stage_7: per-frame encounter HUD/status post-preprocess
112     if low_byte(global_current_activity()) <= in_encounter then
113         preprocess_status(element_ptr)  // ffi_call
114     end if
115 #endif
116 end function
```

Phase 1 type usage:
- `element_t*`, `starship_t*`, `race_desc_t*`, `status_flags_t`, `context_t`, `stamp_t`.

FFI calls (major):
- `rust_ships_preprocess` (rust branch), `get_element_starship`, `set_context`, `draw_stamp`, `release_drawable`, `destroy_drawable`, `init_ship_status`, `draw_captains_window`, species `preprocess_func`, `ship_transition`, `plr_playing`, `opponent_alive`, `battle_song`, `init_intersect_start_point`, `init_intersect_end_point`, `hyper_transition`, `delta_energy`, `normalize_facing`, `set_abs_frame_index`, `object_cloaked`, `spawn_ion_trail`, `preprocess_status`.

---

## 4) `ship_postprocess()` (`ship.c:295-391`)

`USE_RUST_SHIPS` variants:
- Rust build path: `rust_ships_postprocess(element_ptr)` and return.
- C build path: weapon/special counters, weapon spawn, status UI.

```text
001 function ship_postprocess(element_ptr: element_t*) -> void
002 #if use_rust_ships
003     rust_ships_postprocess(element_ptr)  // ffi_call (rust bridge)
004     return
005 #else
006     if element_ptr.crew_level == 0 then
007         return
008     end if
009
010     starship_ptr: starship_t* = null
011     get_element_starship(element_ptr, &starship_ptr)  // ffi_call
012     rd_ptr = starship_ptr.race_desc_ptr
013
014     if starship_ptr.weapon_counter > 0 then
015         starship_ptr.weapon_counter = starship_ptr.weapon_counter - 1
016     else if (starship_ptr.cur_status_flags has weapon_flag)
017          and delta_energy(element_ptr, -rd_ptr.characteristics.weapon_energy_cost) then  // ffi_call
018
019         weapon_handles[6] = {0}
020         num_weapons = rd_ptr.init_weapon_func(element_ptr, weapon_handles)  // ffi_call
021
022         if num_weapons > 0 then
023             get_element_starship(element_ptr, &starship_ptr)  // ffi_call (refresh in case hook mutated refs)
024             played_sfx = false
025
026             for each weapon_handle in first num_weapons of weapon_handles do
027                 if weapon_handle != 0 then
028                     weapon_element_ptr: element_t* = null
029                     lock_element(weapon_handle, &weapon_element_ptr)  // ffi_call
030                     set_element_starship(weapon_element_ptr, starship_ptr)  // ffi_call
031
032                     if not played_sfx then
033                         process_sound(rd_ptr.ship_data.ship_sounds, weapon_element_ptr)  // ffi_call
034                         played_sfx = true
035                     end if
036
037                     unlock_element(weapon_handle)  // ffi_call
038                     put_element(weapon_handle)  // ffi_call
039                 end if
040             end for
041
042             if not played_sfx then
043                 process_sound(rd_ptr.ship_data.ship_sounds, element_ptr)  // ffi_call
044             end if
045         end if
046
047         starship_ptr.weapon_counter = rd_ptr.characteristics.weapon_wait
048     end if
049
050     if starship_ptr.special_counter > 0 then
051         starship_ptr.special_counter = starship_ptr.special_counter - 1
052     end if
053
054     if rd_ptr.postprocess_func != null then
055         rd_ptr.postprocess_func(element_ptr)  // ffi_call (species hook)
056     end if
057
058     if low_byte(global_current_activity()) <= in_encounter then
059         postprocess_status(element_ptr)  // ffi_call
060     end if
061 #endif
062 end function
```

Phase 1 type usage:
- `element_t*`, `starship_t*`, `race_desc_t*`, `helement_t` array (weapon handles).

FFI calls:
- `rust_ships_postprocess`, `get_element_starship`, `delta_energy`, `init_weapon_func`, `lock_element`, `set_element_starship`, `process_sound`, `unlock_element`, `put_element`, species `postprocess_func`, `postprocess_status`.

---

## 5) `collision()` (`ship.c:393-461`)

```text
001 function collision(element_ptr0: element_t*, p_pt0: point_t*, element_ptr1: element_t*, p_pt1: point_t*) -> void
002     if not (element_ptr1.state_flags has finite_life_flag) then
003         element_ptr0.state_flags = element_ptr0.state_flags OR collision_flag
004
005         if gravity_mass(element_ptr1.mass_points) then  // ffi_call
006             // collision with planet-like body
007             damage = element_ptr0.hit_points >> 2
008             if damage == 0 then
009                 damage = 1
010             end if
011
012             do_damage(element_ptr0, damage)  // ffi_call
013
014             sfx_index = target_damaged_for_1_pt + (damage >> 1)
015             if sfx_index > target_damaged_for_6_plus_pt then
016                 sfx_index = target_damaged_for_6_plus_pt
017             end if
018
019             process_sound(set_abs_sound_index(game_sounds, sfx_index), element_ptr0)  // ffi_call
020         end if
021     end if
022
023     // p_pt0/p_pt1 unused in implementation
024 end function
```

Phase 1 type usage:
- `element_t*`, `point_t*` (`POINT*`), integer damage/sfx indices.

FFI calls:
- `gravity_mass`, `do_damage`, `set_abs_sound_index`, `process_sound`.

---

## 6) `spawn_ship()` (`ship.c:463-515`)

`USE_RUST_SHIPS` variants:
- Rust build path: `return rust_ships_spawn(starship_ptr)`.
- C build path: full load/init/element-construction path.

```text
001 function spawn_ship(starship_ptr: starship_t*) -> bool
002 #if use_rust_ships
003     return rust_ships_spawn(starship_ptr)  // ffi_call (rust bridge)
004 #else
005     rd_ptr = load_ship(starship_ptr.species_id, true)  // ffi_call
006     if rd_ptr == null then
007         return false
008     end if
009
010     starship_ptr.race_desc_ptr = rd_ptr
011     starship_ptr.ship_input_state = 0
012     starship_ptr.cur_status_flags = 0
013     starship_ptr.old_status_flags = 0
014
015     if low_byte(global_current_activity()) == in_encounter
016        or low_byte(global_current_activity()) == in_last_battle then
017         if starship_ptr.crew_level == 0 then
018             // flagship crew comes from sis flow elsewhere; leave rd crew as-is
019         else
020             rd_ptr.ship_info.crew_level = starship_ptr.crew_level
021         end if
022
023         if rd_ptr.ship_info.crew_level > rd_ptr.ship_info.max_crew then
024             rd_ptr.ship_info.crew_level = rd_ptr.ship_info.max_crew
025         end if
026     end if
027
028     starship_ptr.energy_counter = 0
029     starship_ptr.weapon_counter = 0
030     starship_ptr.special_counter = 0
031
032     h_ship = starship_ptr.h_ship
033     if h_ship == 0 then
034         h_ship = alloc_element()  // ffi_call
035         if h_ship != 0 then
036             insert_element(h_ship, get_head_element())  // ffi_call
037         end if
038     end if
039
040     starship_ptr.h_ship = h_ship
041     if starship_ptr.h_ship == 0 then
042         return false
043     end if
044
045     ship_element_ptr: element_t* = null
046     lock_element(h_ship, &ship_element_ptr)  // ffi_call
047
048     ship_element_ptr.player_nr = starship_ptr.player_nr
049     ship_element_ptr.crew_level = 0
050     ship_element_ptr.mass_points = rd_ptr.characteristics.ship_mass
051     ship_element_ptr.state_flags = appearing_flag OR player_ship_flag OR ignore_similar_flag
052     ship_element_ptr.turn_wait = 0
053     ship_element_ptr.thrust_wait = 0
054     ship_element_ptr.life_span = normal_life
055     ship_element_ptr.color_cycle_index = 0
056
057     set_prim_type(&display_array[ship_element_ptr.prim_index], stamp_prim)  // ffi_call
058     ship_element_ptr.current.image.farray = rd_ptr.ship_data.ship
059
060     if ship_element_ptr.player_nr == npc_player_num
061        and low_byte(global_current_activity()) == in_last_battle then
062         // sa-matra spawn
063         starship_ptr.ship_facing = 0
064         ship_element_ptr.current.image.frame = set_abs_frame_index(rd_ptr.ship_data.ship[0], starship_ptr.ship_facing)  // ffi_call
065         ship_element_ptr.current.location = (log_space_width/2, log_space_height/2)
066         ship_element_ptr.life_span = ship_element_ptr.life_span + 1
067     else
068         starship_ptr.ship_facing = normalize_facing(tfb_random())  // ffi_call
069
070         if in_hq_space() then  // ffi_call
071             facing = global_ship_facing()
072             if facing > 0 then
073                 facing = facing - 1
074             end if
075             starship_ptr.ship_facing = facing
076         end if
077
078         ship_element_ptr.current.image.frame = set_abs_frame_index(rd_ptr.ship_data.ship[0], starship_ptr.ship_facing)  // ffi_call
079
080         repeat
081             ship_element_ptr.current.location.x = wrap_x(display_align_x(tfb_random()))  // ffi_call
082             ship_element_ptr.current.location.y = wrap_y(display_align_y(tfb_random()))  // ffi_call
083         until (not calculate_gravity(ship_element_ptr)) and (not time_space_matter_conflict(ship_element_ptr))  // ffi_call
084     end if
085
086     ship_element_ptr.preprocess_func = ship_preprocess
087     ship_element_ptr.postprocess_func = ship_postprocess
088     ship_element_ptr.death_func = ship_death  // rust variant via compile-time dispatch inside ship_death path
089     ship_element_ptr.collision_func = collision
090     zero_velocity_components(&ship_element_ptr.velocity)  // ffi_call
091
092     set_element_starship(ship_element_ptr, starship_ptr)  // ffi_call
093     ship_element_ptr.h_target = 0
094
095     unlock_element(h_ship)  // ffi_call
096     return true
097 #endif
098 end function
```

Phase 1 type usage:
- `starship_t*`, `race_desc_t*`, `element_t*`, `helement_t` handle.

FFI calls:
- `rust_ships_spawn`, `load_ship`, `alloc_element`, `insert_element`, `get_head_element`, `lock_element`, `set_prim_type`, `set_abs_frame_index`, `normalize_facing`, `tfb_random`, `in_hq_space`, `wrap_x`, `wrap_y`, `display_align_x`, `display_align_y`, `calculate_gravity`, `time_space_matter_conflict`, `zero_velocity_components`, `set_element_starship`, `unlock_element`.

---

## 7) `GetNextStarShip()` (`ship.c:518-552`)

```text
001 function get_next_star_ship(last_starship_ptr: starship_t*, which_side: count_t) -> bool
002     h_battle_ship = get_encounter_star_ship(last_starship_ptr, which_side)  // ffi_call
003
004     if h_battle_ship != 0 then
005         starship_ptr = lock_starship(&race_q[which_side], h_battle_ship)  // ffi_call
006
007         if last_starship_ptr != null then
008             if starship_ptr == last_starship_ptr then
009                 // recycled ship handle case (infinite-ship contexts)
010                 last_starship_ptr = null
011             else
012                 starship_ptr.h_ship = last_starship_ptr.h_ship
013             end if
014         end if
015
016         if not spawn_ship(starship_ptr) then
017             unlock_starship(&race_q[which_side], h_battle_ship)  // ffi_call
018             return false
019         end if
020
021         unlock_starship(&race_q[which_side], h_battle_ship)  // ffi_call
022     end if
023
024     if last_starship_ptr != null then
025         last_starship_ptr.h_ship = 0
026     end if
027
028     return (h_battle_ship != 0)
029 end function
```

Phase 1 type usage:
- `starship_t*`, `hstarship_t`, `count_t`.

FFI calls:
- `get_encounter_star_ship`, `lock_starship`, `unlock_starship`.

---

## 8) `GetInitialStarShips()` (`ship.c:554-592`)

```text
001 function get_initial_star_ships() -> bool
002     if low_byte(global_current_activity()) == super_melee then
003         ships[0..num_players-1]: hstarship_t
004
005         if not get_initial_melee_star_ships(ships) then  // ffi_call
006             return false
007         end if
008
009         for i in 0 to num_players - 1 do
010             player_i = get_player_order(i)  // ffi_call
011             starship_ptr = lock_starship(&race_q[player_i], ships[player_i])  // ffi_call
012
013             if not spawn_ship(starship_ptr) then
014                 unlock_starship(&race_q[player_i], ships[player_i])  // ffi_call
015                 return false
016             end if
017
018             unlock_starship(&race_q[player_i], ships[player_i])  // ffi_call
019         end for
020
021         return true
022     else
023         for i from num_players down_to 1 do
024             if not get_next_star_ship(null, i - 1) then
025                 return false
026             end if
027         end for
028
029         return true
030     end if
031 end function
```

Phase 1 type usage:
- `hstarship_t[]`, `starship_t*`, `count_t`, player index/count types.

FFI calls:
- `get_initial_melee_star_ships`, `get_player_order`, `lock_starship`, `unlock_starship`, and transitively `spawn_ship`/`get_next_star_ship` dependencies.

---

## Notes on branch variants and runtime ownership

- `USE_RUST_SHIPS` short-circuits key lifecycle hooks (`ship_preprocess`, `ship_postprocess`, `spawn_ship`) to Rust bridge entry points. The C side remains the default full pipeline when the macro is disabled.
- `ship_death` dispatch is assigned in `spawn_ship`; this file references a compile-time variant via includes/externs.
- Handle-based ownership patterns (`HELEMENT`, `HSTARSHIP`) are lock/unlock disciplined; pseudocode keeps this explicit to preserve ordering semantics.
