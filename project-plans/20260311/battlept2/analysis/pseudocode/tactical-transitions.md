# Tactical Transitions Pseudocode (Phase P02)

Source coverage:
- `sc2/src/uqm/tactrans.c` (full file)
- `sc2/src/uqm/battle.c:68-135` (`do_run_away`)

Conventions used:
- snake_case naming
- numbered lines
- `[FFI]` markers where C/runtime boundary details matter
- `[PHASE1]` markers for likely first-pass Rust-port behavior
- `[VALIDATE]` checkpoints for parity-critical behavior

---

## Critical callback chain (death/explosion path)

```text
01. ship_death(ship)
02.   -> start_ship_explosion(ship, play_sound=true)
03.      -> sets death_func = cleanup_dead_ship
04.      -> sets preprocess_func = explosion_preprocess
05.   -> later per-frame explosion_preprocess runs until lifespan expires
06.   -> on ship element death: cleanup_dead_ship(dead_ship)
07.      -> converts dead ship element into waiting sentinel
08.      -> sets death_func = new_ship
09.      -> sets preprocess_func = preprocess_dead_ship
10.   -> when sentinel life_span expires: new_ship(dead_ship)
11.      -> waits for ready_for_battle_end(); may extend life_span
12.      -> eventually tears down ship + opens next-ship transition
```

[VALIDATE]
- Ensure callback assignments and transitions occur in this exact order.
- Ensure winner-picks-last behavior via life_span coordination is preserved.

---

## P10 functions

### 1) opponent_alive(test_star_ship_ptr)

```text
01. function opponent_alive(test_star_ship_ptr) -> bool
02.   for each element handle h_element from head to tail:
03.     lock element
04.     cache successor handle h_succ_element
05.     star_ship_ptr = get_element_star_ship(element)
06.     unlock element
07.
08.     if star_ship_ptr exists
09.        and star_ship_ptr != test_star_ship_ptr
10.        and star_ship_ptr.race_desc.ship_info.crew_level == 0:
11.          return false
12.
13.   return true
```

[VALIDATE]
- Semantics are inverted vs name: returns `false` when “other ship exists and is dead”. Keep exact behavior.

---

### 2) find_alive_star_ship(dead_ship)

```text
01. function find_alive_star_ship(dead_ship) -> star_ship_or_null
02.   alive_ship = null
03.
04.   for each element from head to tail:
05.     lock element
06.     if element has PLAYER_SHIP
07.        and element != dead_ship
08.        and element.mass_points <= MAX_SHIP_MASS + 1:   // exclude running-away ships
09.          alive_ship = get_element_star_ship(element)
10.          assert alive_ship != null
11.
12.          if alive_ship.race_desc.ship_info.crew_level == 0
13.             and element.mass_points != MAX_SHIP_MASS + 1:   // pkunk reincarnation exception
14.               alive_ship = null
15.
16.          unlock element
17.          break
18.
19.     h_next = get_succ_element(element)
20.     unlock element
21.
22.   return alive_ship
```

[VALIDATE]
- Preserve `MAX_SHIP_MASS + 1` special case exactly.

---

### 3) reset_winner_star_ship()

```text
01. function reset_winner_star_ship()
02.   winner_star_ship = null
```

---

### 4) get_winner_star_ship()

```text
01. function get_winner_star_ship() -> star_ship_or_null
02.   return winner_star_ship
```

---

### 5) set_winner_star_ship(winner)

```text
01. function set_winner_star_ship(winner)
02.   if winner == null:
03.     return
04.
05.   winner.cur_status_flags |= PLAY_VICTORY_DITTY
06.
07.   if winner_star_ship == null:
08.     winner_star_ship = winner
```

[VALIDATE]
- First-call-wins policy in simultaneous death must be preserved.

---

### 6) flee_preprocess(element_ptr)

```text
01. function flee_preprocess(element_ptr)
02.   if (--element_ptr.turn_wait == 0):
03.     color_tab[20] = [
04.       0:  (0x0A,0,0), 1:(0x0E,0,0), 2:(0x13,0,0), 3:(0x17,0,0), 4:(0x1B,0,0),
05.       5:  (0x1F,0,0), 6:(0x1F,0x04,0x04), 7:(0x1F,0x0A,0x0A), 8:(0x1F,0x0F,0x0F),
06.       9:  (0x1F,0x13,0x13), 10:(0x1F,0x19,0x19),
07.       11:(0x1F,0x13,0x13), 12:(0x1F,0x0F,0x0F), 13:(0x1F,0x0A,0x0A),
08.       14:(0x1F,0x04,0x04), 15:(0x1F,0,0), 16:(0x1B,0,0), 17:(0x17,0,0),
09.       18:(0x13,0,0), 19:(0x0E,0,0)
10.     ]
11.
12.     element_ptr.color_cycle_index += 1
13.     if element_ptr.color_cycle_index == 20:
14.       element_ptr.color_cycle_index = 0
15.
16.     set_prim_color(element_ptr.prim, color_tab[element_ptr.color_cycle_index])
17.
18.     if element_ptr.color_cycle_index == 0:
19.       element_ptr.thrust_wait -= 1
20.
21.     element_ptr.turn_wait = element_ptr.thrust_wait
22.     if element_ptr.turn_wait != 0:
23.       element_ptr.turn_wait = ((element_ptr.turn_wait - 1) >> 1) + 1
24.     else if element_ptr.color_cycle_index != (20 / 2):
25.       element_ptr.turn_wait = 1
26.     else:
27.       // midpoint of pulse reached while fully collapsed timing
28.       element_ptr.death_func = cleanup_dead_ship
29.       element_ptr.crew_level = 0
30.
31.       element_ptr.life_span = HYPERJUMP_LIFE + 1
32.       element_ptr.preprocess_func = ship_transition
33.       element_ptr.postprocess_func = null
34.       set_prim_type(element_ptr.prim, NO_PRIM)
35.       element_ptr.state_flags |= NONSOLID | FINITE_LIFE | CHANGING
36.
37.   star_ship_ptr = get_element_star_ship(element_ptr)
38.   star_ship_ptr.cur_status_flags &= ~(LEFT|RIGHT|THRUST|WEAPON|SPECIAL)
39.   pre_process_status(element_ptr)
```

[VALIDATE]
- 20-color pulse order and midpoint trigger (`index==10`) must match exactly.
- `turn_wait` halving formula must be exact.

---

### 7) ship_transition(element_ptr)

```text
01. function ship_transition(element_ptr)
02.   if element_ptr has PLAYER_SHIP:
03.     if element_ptr has APPEARING:
04.       // initialize warp transition
05.       element_ptr.life_span = HYPERJUMP_LIFE
06.       element_ptr.preprocess_func = ship_transition
07.       element_ptr.postprocess_func = null
08.       set_prim_type(element_ptr.prim, NO_PRIM)
09.       element_ptr.state_flags |= NONSOLID | FINITE_LIFE | CHANGING
10.
11.     else if element_ptr.life_span < HYPERJUMP_LIFE:
12.       if element_ptr.life_span == NORMAL_LIFE and element_ptr.crew_level != 0:
13.         // re-materialize ship at end of warp-in
14.         element_ptr.current.image.frame = set_equ_frame_index(...)
15.         element_ptr.next.image.frame = element_ptr.current.image.frame
16.         set_prim_type(element_ptr.prim, STAMP_PRIM)
17.         init_intersect_start_point(element_ptr)
18.         init_intersect_end_point(element_ptr)
19.         init_intersect_frame(element_ptr)
20.         zero_velocity(element_ptr.velocity)
21.         element_ptr.state_flags &= ~(NONSOLID | FINITE_LIFE)
22.         element_ptr.state_flags |= CHANGING
23.         element_ptr.preprocess_func = ship_preprocess
24.         element_ptr.postprocess_func = ship_postprocess
25.       return
26.
27.   // spawn ghost image trail segments
28.   star_ship_ptr = get_element_star_ship(element_ptr)
29.   lock element at star_ship_ptr.hShip as ship_image_ptr
30.
31.   if ship_image_ptr not NONSOLID:
32.     element_ptr.preprocess_func = null
33.   else if alloc element h_ship_image succeeds:
34.     TRANSITION_SPEED = DISPLAY_TO_WORLD(40)
35.     TRANSITION_LIFE = 1
36.     put element
37.     angle = facing_to_angle(star_ship_ptr.ShipFacing)
38.
39.     lock new image element as ship_image_ptr
40.     ship_image_ptr.playerNr = NEUTRAL_PLAYER_NUM
41.     ship_image_ptr.state_flags = APPEARING | FINITE_LIFE | NONSOLID
42.     ship_image_ptr.thrust_wait = TRANSITION_LIFE
43.     ship_image_ptr.life_span = TRANSITION_LIFE
44.     set_prim_type(ship_image_ptr.prim, STAMPFILL_PRIM)
45.     set_prim_color(ship_image_ptr.prim, START_ION_COLOR)
46.     ship_image_ptr.color_cycle_index = 0
47.     ship_image_ptr.current.image = element_ptr.current.image
48.     ship_image_ptr.current.location = element_ptr.current.location
49.
50.     if element_ptr is not PLAYER_SHIP:
51.       // warp-out shadow projected forward
52.       ship_image_ptr.current.location += vector(angle, TRANSITION_SPEED)
53.       element_ptr.preprocess_func = null
54.     else if element_ptr.crew_level != 0:
55.       // warp-in ghost chain projected backward by life_span offset
56.       ship_image_ptr.current.location -= vector(angle, TRANSITION_SPEED) * (element_ptr.life_span - 1)
57.       ship_image_ptr.current.location = wrap_xy(ship_image_ptr.current.location)
58.
59.     ship_image_ptr.preprocess_func = ship_transition
60.     ship_image_ptr.death_func = cycle_ion_trail
61.     set_element_star_ship(ship_image_ptr, star_ship_ptr)
62.     unlock new image element
63.
64.   unlock star_ship_ptr.hShip
```

[VALIDATE]
- Ghost sequence should render the full 15-frame hyperjump progression (driven by `HYPERJUMP_LIFE`).
- Preserve different location offsets for warp-in vs warp-out branches.

---

### 8) do_run_away(star_ship_ptr)  // from battle.c:68-135

```text
01. function do_run_away(star_ship_ptr)
02.   lock element = star_ship_ptr.hShip
03.   if prim_type == STAMP_PRIM
04.      and element.life_span == NORMAL_LIFE
05.      and element not FINITE_LIFE
06.      and element.mass_points != MAX_SHIP_MASS * 10
07.      and element not APPEARING:
08.
09.       battle_counter[0] -= 1
10.
11.       element.turn_wait = 3
12.       element.thrust_wait = 4
13.       element.color_cycle_index = 0
14.       element.preprocess_func = flee_preprocess
15.       element.mass_points = MAX_SHIP_MASS * 10
16.       zero_velocity(element.velocity)
17.       star_ship_ptr.cur_status_flags &= ~(SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED)
18.
19.       set_prim_color(element.prim, BUILD_COLOR(0x0B,0,0))
20.       set_prim_type(element.prim, STAMPFILL_PRIM)
21.       star_ship_ptr.ship_input_state = 0
22.
23.   unlock element
```

[VALIDATE]
- Keep sentinel `mass_points = MAX_SHIP_MASS * 10` unchanged.
- Initial color differs slightly from flee table first entry (intentional legacy quirk).

---

## P09 functions

### 9) check_other_ship_life_span(dead_ship)

```text
01. function check_other_ship_life_span(dead_ship)
02.   dead_star_ship = get_element_star_ship(dead_ship)
03.
04.   if winner_star_ship != null
05.      and dead_star_ship != winner_star_ship
06.      and winner_star_ship.race_desc.ship_info.crew_level == 0:
07.       // opponent also died but is winner (e.g. glory device case)
08.       set_min_star_ship_life_span(winner_star_ship, dead_ship.life_span + 1)
09.
10.   else if winner_star_ship == null:
11.       // simultaneous death or loser already expired
12.       for each element in all elements:
13.         lock element
14.         star_ship = get_element_star_ship(element)
15.         if star_ship != null
16.            and element != dead_ship
17.            and star_ship.race_desc.ship_info.crew_level == 0:
18.              set_min_ship_life_span(element, dead_ship.life_span)
19.         unlock element
```

---

### 10) set_min_ship_life_span(ship, life_span)

```text
01. function set_min_ship_life_span(ship, life_span)
02.   if ship.death_func == new_ship:
03.     assert ship has FINITE_LIFE
04.     assert ship not DISAPPEARING
05.     if ship.life_span < life_span:
06.       ship.life_span = life_span
```

---

### 11) set_min_star_ship_life_span(star_ship, life_span)

```text
01. function set_min_star_ship_life_span(star_ship, life_span)
02.   lock ship element at star_ship.hShip
03.   set_min_ship_life_span(ship, life_span)
04.   unlock
```

---

### 12) ditty_playing()

```text
01. function ditty_playing() -> bool
02.   if not ditty_is_playing:
03.     return false
04.
05.   ditty_is_playing = plr_playing(MUSIC_REF_ALL_ONES)
06.   return ditty_is_playing
```

[FFI]
- `plr_playing((MUSIC_REF)~0)` exact sentinel must be mirrored.

---

### 13) play_ditty(ship)

```text
01. function play_ditty(ship)
02.   play_music(ship.race_desc.ship_data.victory_ditty, loop=false, priority=3)
03.   ditty_is_playing = true
```

---

### 14) stop_ditty()

```text
01. function stop_ditty()
02.   if ditty_is_playing:
03.     stop_music()
04.   ditty_is_playing = false
```

---

### 15) ready_for_battle_end()

```text
01. function ready_for_battle_end() -> bool
02.   if NETPLAY is disabled:
03.     if DEMO_MODE enabled:
04.       return true   // deterministic replay; do not trust plr timing
05.     else:
06.       return not ditty_playing()
07.
08.   else (NETPLAY enabled):
09.     if ditty_playing():
10.       return false
11.
12.     for player_i in [0 .. NUM_PLAYERS-1]:
13.       if not player_input[player_i].handlers.battle_end_ready(player_input[player_i]):
14.         return false
15.
16.     return true
```

[VALIDATE]
- Must preserve compile-time branch behavior: NETPLAY + DEMO_MODE differences.

---

### 16) preprocess_dead_ship(dead_ship_ptr)

```text
01. function preprocess_dead_ship(dead_ship_ptr)
02.   process_sound(SOUND_ALL_ONES, null)
03.   ignore dead_ship_ptr
```

[FFI]
- Sound pump call is intentionally retained as side-effect heartbeat.

---

### 17) cleanup_dead_ship(dead_ship_ptr)

```text
01. function cleanup_dead_ship(dead_ship_ptr)
02.   process_sound(SOUND_ALL_ONES, null)
03.   dead_star_ship = get_element_star_ship(dead_ship_ptr)
04.
05.   // record post-battle crew snapshot
06.   dead_star_ship.crew_level = dead_star_ship.race_desc.ship_info.crew_level
07.
08.   music_started = false
09.
10.   for each element in all elements:
11.     lock element
12.     star_ship = get_element_star_ship(element)
13.
14.     if star_ship == dead_star_ship:
15.       set_element_star_ship(element, null)
16.
17.       if element is not (CREW_OBJECT with preprocess==crew_preprocess):
18.         set_prim_type(element.prim, NO_PRIM)
19.         element.life_span = 0
20.         element.state_flags = NONSOLID | DISAPPEARING | FINITE_LIFE
21.         element.preprocess_func = null
22.         element.postprocess_func = null
23.         element.death_func = null
24.         element.collision_func = null
25.
26.     if star_ship exists and star_ship.cur_status_flags has PLAY_VICTORY_DITTY:
27.       music_started = true
28.       play_ditty(star_ship)
29.       star_ship.cur_status_flags &= ~PLAY_VICTORY_DITTY
30.
31.     unlock element
32.
33.   MIN_DITTY_FRAME_COUNT = (ONE_SECOND * 3) / BATTLE_FRAME_RATE
34.   dead_ship_ptr.life_span = (music_started ? MIN_DITTY_FRAME_COUNT : 1)
35.
36.   if dead_star_ship == winner_star_ship:
37.     dead_ship_ptr.life_span = MIN_DITTY_FRAME_COUNT + 1
38.
39.   dead_ship_ptr.death_func = new_ship
40.   dead_ship_ptr.preprocess_func = preprocess_dead_ship
41.   dead_ship_ptr.state_flags &= ~DISAPPEARING
42.   dead_ship_ptr.life_span += 1   // legacy framecount-preserving increment
43.   set_element_star_ship(dead_ship_ptr, dead_star_ship)
```

[VALIDATE]
- Multi-step life_span logic must remain exactly in this sequence:
  1) base by music_started, 2) winner override, 3) unconditional +1.

---

### 18) new_ship(dead_ship_ptr)

```text
01. function new_ship(dead_ship_ptr)
02.   dead_star_ship = get_element_star_ship(dead_ship_ptr)
03.
04.   if not ready_for_battle_end():
05.     dead_ship_ptr.state_flags &= ~DISAPPEARING
06.     dead_ship_ptr.life_span += 1
07.     check_other_ship_life_span(dead_ship_ptr)
08.     return
09.
10.   winner_star_ship = null
11.
12.   stop_ditty()
13.   stop_music()
14.   stop_sound()
15.
16.   set_element_star_ship(dead_ship_ptr, null)
17.   restart_music = opponent_alive(dead_star_ship)
18.
19.   free_ship(dead_star_ship.race_desc, free_icons=true, free_melee=true)
20.   dead_star_ship.race_desc = null
21.
22.   unbatch_graphics()
23.
24.   if NETPLAY:
25.     init_battle_state_data_connections()
26.     if not negotiate_ready_connections(true, NetState_interBattle):
27.       GLOBAL.CurrentActivity &= ~IN_BATTLE
28.       batch_graphics()
29.       return
30.
31.   if not fleet_is_infinite(dead_star_ship.playerNr):
32.     update_ship_frag_crew(dead_star_ship)
33.     dead_star_ship.SpeciesID = NO_ID
34.
35.   if get_next_star_ship(dead_star_ship, dead_star_ship.playerNr):
36.     if NETPLAY and not negotiate_ready_connections(true, NetState_inBattle):
37.       GLOBAL.CurrentActivity &= ~IN_BATTLE
38.       batch_graphics()
39.       return
40.
41.     if restart_music:
42.       battle_song(true)
43.
44.   else if battle_counter[0] == 0 or battle_counter[1] == 0:
45.     GLOBAL.CurrentActivity &= ~IN_BATTLE
46.
47.   else if NETPLAY:
48.     GLOBAL.CurrentActivity |= CHECK_ABORT
49.
50.   batch_graphics()
```

[PHASE1]
- Keep control-flow parity first; defer structural refactor until after golden tests.

---

### 19) explosion_preprocess(ship_ptr)

```text
01. function explosion_preprocess(ship_ptr)
02.   i = (NUM_EXPLOSION_FRAMES * 3) - ship_ptr.life_span
03.
04.   switch i:
05.     case 25:
06.       ship_ptr.preprocess_func = null
07.       // fallthrough
08.     case 0,1,2,20,21,22,23,24:
09.       i = 1
10.     case 3,4,5,18,19:
11.       i = 2
12.     case 15:
13.       set_prim_type(ship_ptr.prim, NO_PRIM)
14.       ship_ptr.state_flags |= CHANGING
15.       // fallthrough
16.     default:
17.       i = 3
18.
19.   do:
20.     h_element = alloc_element()
21.     if h_element exists:
22.       put_element(h_element)
23.       lock element
24.       element.playerNr = NEUTRAL_PLAYER_NUM
25.       element.state_flags = APPEARING | FINITE_LIFE | NONSOLID
26.       element.life_span = 9
27.       set_prim_type(element.prim, STAMP_PRIM)
28.       element.current.image.farray = explosion_frames
29.       element.current.image.frame = explosion_frames[0]
30.
31.       rand_val = tfb_random()
32.       angle = low_byte(high_word(rand_val))
33.       dist = DISPLAY_TO_WORLD(low_byte(low_word(rand_val)) % 8)
34.       if high_byte(low_word(rand_val)) < (256 * 1 / 3):
35.         dist += DISPLAY_TO_WORLD(8)
36.
37.       element.current.location = ship_ptr.current.location + polar(angle, dist)
38.
39.       element.preprocess_func = animation_preprocess
40.
41.       rand_val = tfb_random()
42.       angle = low_byte(low_word(rand_val))
43.       dist = WORLD_TO_VELOCITY(DISPLAY_TO_WORLD(high_byte(low_word(rand_val)) % 5))
44.       set_velocity_components(element.velocity, polar(angle, dist))
45.       unlock element
46.   while (--i)
```

[VALIDATE]
- Debris spawn schedule exactness:
  - `i in {0,1,2,20..24}` => 1 particle
  - `i in {3,4,5,18,19}` => 2 particles
  - all others => 3 particles
  - at `i==15` hide ship prim
  - at `i==25` disable future preprocess

---

### 20) start_ship_explosion(ship_ptr, play_sound)

```text
01. function start_ship_explosion(ship_ptr, play_sound)
02.   star_ship_ptr = get_element_star_ship(ship_ptr)
03.   zero_velocity(ship_ptr.velocity)
04.   delta_energy(ship_ptr, -star_ship_ptr.race_desc.ship_info.energy_level)
05.
06.   ship_ptr.life_span = NUM_EXPLOSION_FRAMES * 3
07.   ship_ptr.state_flags &= ~DISAPPEARING
08.   ship_ptr.state_flags |= FINITE_LIFE | NONSOLID
09.   ship_ptr.preprocess_func = explosion_preprocess
10.   ship_ptr.postprocess_func = PostProcessStatus
11.   ship_ptr.death_func = cleanup_dead_ship
12.   ship_ptr.hTarget = 0
13.
14.   if play_sound:
15.     play_sound_effect(SHIP_EXPLODES, calc_sound_position(ship_ptr), priority=GAME_SOUND_PRIORITY+1)
```

---

### 21) stop_all_battle_music()

```text
01. function stop_all_battle_music()
02.   stop_ditty()
03.   stop_music()
```

---

### 22) spawn_ion_trail(element_ptr)

```text
01. function spawn_ion_trail(element_ptr)
02.   assert element_ptr has PLAYER_SHIP
03.   h_ion = alloc_element()
04.   if h_ion exists:
05.     ION_LIFE = 1
06.     star_ship_ptr = get_element_star_ship(element_ptr)
07.     angle = facing_to_angle(star_ship_ptr.ShipFacing) + HALF_CIRCLE
08.     r = get_frame_rect(star_ship_ptr.race_desc.ship_data.ship[0])
09.     r.extent.height = DISPLAY_TO_WORLD(r.extent.height + r.corner.y)
10.
11.     insert_element_at_head(h_ion)
12.     lock ion
13.     ion.playerNr = NEUTRAL_PLAYER_NUM
14.     ion.state_flags = APPEARING | FINITE_LIFE | NONSOLID
15.     ion.thrust_wait = ION_LIFE
16.     ion.life_span = ION_LIFE
17.     set_prim_type(ion.prim, POINT_PRIM)
18.     set_prim_color(ion.prim, START_ION_COLOR)
19.     ion.color_cycle_index = 0
20.     ion.current.image.frame = dec_frame_index(stars_in_space)
21.     ion.current.image.farray = &stars_in_space
22.     ion.current.location = element_ptr.current.location + polar(angle, r.extent.height)
23.     ion.death_func = cycle_ion_trail
24.     set_element_star_ship(ion, star_ship_ptr)
25.
26.     // manual preprocess bootstrap due to head insertion ordering
27.     ion.next = ion.current
28.     ion.life_span -= 1
29.     ion.state_flags |= PRE_PROCESS
30.
31.     unlock ion
```

[VALIDATE]
- Manual bootstrap (`next=current`, `life_span--`, `PRE_PROCESS`) is required.

---

### 23) ship_death(ship_ptr)

```text
01. function ship_death(ship_ptr)
02.   star_ship_ptr = get_element_star_ship(ship_ptr)
03.
04.   stop_all_battle_music()
05.
06.   // suppress ditty if this winner later dies before ditty starts
07.   star_ship_ptr.cur_status_flags &= ~PLAY_VICTORY_DITTY
08.
09.   start_ship_explosion(ship_ptr, play_sound=true)
10.
11.   winner = find_alive_star_ship(ship_ptr)
12.   set_winner_star_ship(winner)
13.   record_ship_death(ship_ptr)
```

---

### 24) cycle_ion_trail(element_ptr)

```text
01. function cycle_ion_trail(element_ptr)
02.   color_tab[12] = warm_to_dark sequence
03.   assert element_ptr not PLAYER_SHIP
04.
05.   element_ptr.color_cycle_index += 1
06.   if element_ptr.color_cycle_index != 12:
07.     element_ptr.life_span = element_ptr.thrust_wait
08.     set_prim_color(element_ptr.prim, color_tab[element_ptr.color_cycle_index])
09.     element_ptr.state_flags &= ~DISAPPEARING
10.     element_ptr.state_flags |= CHANGING
11.   else:
12.     // allow disappearance
13.     no-op
```

---

### 25) record_ship_death(dead_ship)

```text
01. function record_ship_death(dead_ship)
02.   dead_star_ship = get_element_star_ship(dead_ship)
03.   assert dead_star_ship != null
04.
05.   if dead_ship.mass_points <= MAX_SHIP_MASS:
06.     // not running away (runaway already counted in do_run_away)
07.     assert dead_star_ship.playerNr >= 0
08.     battle_counter[dead_star_ship.playerNr] -= 1
09.
10.   if low_byte(GLOBAL.CurrentActivity) == SUPER_MELEE:
11.     melee_ship_death(dead_star_ship)
```

[VALIDATE]
- Keep mass-point guard so runaway deaths are not double-counted.

---

## NETPLAY / DEMO_MODE branch summary

```text
01. ready_for_battle_end:
02.   - no NETPLAY + DEMO_MODE: always true
03.   - no NETPLAY + normal: !ditty_playing()
04.   - NETPLAY: !ditty_playing() AND all player battle_end_ready handlers true
05.
06. new_ship includes NETPLAY checkpoints:
07.   - negotiate_ready_connections(..., NetState_interBattle)
08.   - negotiate_ready_connections(..., NetState_inBattle)
09.   - on failure: clear IN_BATTLE and return
```

[FFI]
- Networking readiness negotiation must remain synchronous and frame-safe.
