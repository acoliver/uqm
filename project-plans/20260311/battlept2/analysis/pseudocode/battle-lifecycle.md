# battle lifecycle pseudocode (phase p02)

## conventions
- numbered lines are local to each function block.
- snake_case identifiers are used for pseudocode.
- markers:
  - `[ffi/phase1]` = c-side callable boundary or existing phase-1 compatible seam.
  - `[validate]` = explicit invariant/state checkpoint.

---

## `battle.c`

### 1) `run_away_allowed()` (battle.c:63-67)

```text
01: function run_away_allowed() -> bool
02:     in_encounter_or_last_battle = (
03:         low_byte(global_current_activity) == in_encounter
04:         or low_byte(global_current_activity) == in_last_battle
05:     )
06:     has_starbase = get_game_state(starbase_available)
07:     carrying_bomb = get_game_state(bomb_carrier)
08:     return in_encounter_or_last_battle and has_starbase and not carrying_bomb
09: end
```

### 2) `process_input()` (battle.c:145-177)

```text
01: function process_input() -> void
02:     #if netplay
03:     net_input()  # [ffi/phase1]
04:     #endif
05:
06:     can_run_away = run_away_allowed()
07:
08:     for side_i in 0 .. num_sides-1:
09:         cur_player = battle_input_order[side_i]
10:         battle_ship = head_link(race_q[cur_player])
11:
12:         while battle_ship != 0:
13:             star_ship = lock_star_ship(race_q[cur_player], battle_ship)
14:             next_ship = get_succ_link(star_ship)
15:
16:             if star_ship.h_ship != 0:
17:                 star_ship.control = player_control[cur_player]
18:                 input_state = player_input[cur_player].handlers.frame_input(
19:                     player_input[cur_player], star_ship
20:                 )  # [ffi/phase1]
21:
22:                 #if create_journal
23:                 journal_input(input_state)
24:                 #endif
25:
26:                 #if netplay
27:                 if not (player_control[cur_player] has network_control):
28:                     bib = get_battle_input_buffer(cur_player)
29:                     netplay_notify_all_battle_input(input_state)
30:                     flush_packet_queues()
31:                     battle_input_buffer_push(bib, input_state)
32:                     battle_input_buffer_pop(bib, out input_state)
33:                 #endif
34:
35:                 star_ship.ship_input_state = 0
36:
37:                 if star_ship.race_desc.ship_info.crew_level != 0:
38:                     if input_state has battle_left:
39:                         star_ship.ship_input_state |= left
40:                     else if input_state has battle_right:
41:                         star_ship.ship_input_state |= right
42:
43:                     if input_state has battle_thrust:
44:                         star_ship.ship_input_state |= thrust
45:                     if input_state has battle_weapon:
46:                         star_ship.ship_input_state |= weapon
47:                     if input_state has battle_special:
48:                         star_ship.ship_input_state |= special
49:
50:                     if can_run_away and cur_player == 0 and (input_state has battle_escape):
51:                         do_run_away(star_ship)
52:             end_if
53:
54:             unlock_star_ship(race_q[cur_player], battle_ship)
55:             battle_ship = next_ship
56:         end_while
57:     end_for
58:
59:     #if netplay
60:     flush_packet_queues()
61:     #endif
62:
63:     if global_current_activity has (check_load or check_abort):
64:         global_current_activity &= ~in_battle
65:     [validate] in_battle flag cleared on load/abort requests
66: end
```

### 3) `setup_battle_input_order()` (battle.c:180-220)

```text
01: function setup_battle_input_order() -> void
02:     #if not netplay
03:     for i in 0 .. num_sides-1:
04:         battle_input_order[i] = i
05:     #else
06:     i = 0
07:
08:     # local-controlled sides first
09:     for j in 0 .. num_sides-1:
10:         if not (player_control[j] has network_control):
11:             battle_input_order[i] = j
12:             i += 1
13:
14:     # network-controlled sides last
15:     for j in 0 .. num_sides-1:
16:         if player_control[j] has network_control:
17:             battle_input_order[i] = j
18:             i += 1
19:     #endif
20:
21:     [validate] battle_input_order contains each side index exactly once
22: end
```

### 4) `battle_song(do_play)` (battle.c:222-262)

```text
01: function battle_song(do_play: bool) -> void
02:     if battle_ref == 0:
03:         if in_hyper_space():
04:             battle_ref = load_music(hyperspace_music)
05:         else if in_quasi_space():
06:             battle_ref = load_music(quasispace_music)
07:         else:
08:             battle_ref = load_music(battle_music)
09:         [validate] battle_ref loaded once and cached
10:
11:     if do_play:
12:         play_music(battle_ref, loop=true, priority=1)
13: end
```

### 5) `free_battle_song()` (battle.c:264-280)

```text
01: function free_battle_song() -> void
02:     destroy_music(battle_ref)
03:     battle_ref = 0
04:     [validate] music handle reset for next battle lifecycle
05: end
```

### 6) `select_all_ships(num_ships)` (battle.c:282-320)

```text
01: function select_all_ships(num_ships: size) -> bool
02:     if num_ships == 1:
03:         # hyperspace full-game path
04:         return get_next_star_ship(null, 0)  # [ffi/phase1]
05:
06:     #if netplay
07:     if (player_control[0] has network_control) and (player_control[1] has network_control):
08:         log_error("only one side may be network-controlled")
09:         return false
10:     #endif
11:
12:     return get_initial_star_ships()  # [ffi/phase1]
13: end
```

### 7) `get_player_order(i)` (battle.c:380-394, netplay)

```text
01: function get_player_order(i: count) -> count
02:     # if local side has my_turn discriminant, local goes first
03:     side0_local_first = (
04:         (player_control[0] has network_control)
05:         and not net_connection_get_discriminant(net_connections[0])
06:     )
07:     side1_local_first = (
08:         (player_control[1] has network_control)
09:         and net_connection_get_discriminant(net_connections[1])
10:     )
11:
12:     if side0_local_first or side1_local_first:
13:         return i
14:     else:
15:         return 1 - i
16: end
```

### 8) `battle(callback)` (battle.c:396-516) — full entry/exit lifecycle

```text
01: function battle(callback: battle_frame_callback*) -> bool
02:     # rng seed policy
03:     #if not (demo_mode or create_journal)
04:     if low_byte(global_current_activity) != super_melee:
05:         tfb_seed_random(get_time_counter())
06:     # super_melee keeps pre-initialized rng
07:     #else
08:     if battle_seed == 0:
09:         battle_seed = tfb_random()
10:     tfb_seed_random(battle_seed)
11:     battle_seed = tfb_random()  # precompute next seed
12:     #endif
13:
14:     battle_song(do_play=false)  # preload/cached only
15:
16:     num_ships = init_ships()  # [ffi/phase1]
17:
18:     if instant_victory:
19:         num_ships = 0
20:         battle_counter[0] = 1
21:         battle_counter[1] = 0
22:         instant_victory = false
23:
24:     if num_ships != 0:
25:         bs = new battle_state()
26:
27:         global_current_activity |= in_battle
28:         battle_counter[0] = count_links(race_q[0])
29:         battle_counter[1] = count_links(race_q[1])
30:
31:         if opt_melee_scale != tfb_scale_step:
32:             set_graphic_scale_mode(opt_melee_scale)
33:
34:         setup_battle_input_order()
35:
36:         #if netplay
37:         init_battle_input_buffers()
38:         #if netplay_checksum
39:         init_checksum_buffers()
40:         #endif
41:         battle_frame_count = 0
42:         reset_winner_star_ship()
43:         set_battle_state_connections(&bs)
44:         #endif
45:
46:         if not select_all_ships(num_ships):
47:             global_current_activity |= check_abort
48:             goto abort_battle
49:
50:         battle_song(do_play=true)
51:         bs.next_time = 0
52:
53:         #if netplay
54:         init_battle_state_data_connections()
55:         all_ok = negotiate_ready_connections(true, net_state_in_battle)
56:         if not all_ok:
57:             global_current_activity |= check_abort
58:             goto abort_battle
59:         #endif
60:
61:         bs.input_func = do_battle
62:         bs.frame_cb = callback
63:         bs.first_time = in_hq_space()
64:
65:         do_input(&bs, false)  # main battle loop driver [ffi/phase1]
66:
67: abort_battle:
68:         if low_byte(global_current_activity) == super_melee:
69:             if global_current_activity has check_abort:
70:                 #if netplay
71:                 wait_reset_connections(net_state_in_setup)
72:                 #endif
73:                 global_current_activity &= ~check_abort
74:             else:
75:                 melee_game_over()
76:
77:         #if netplay
78:         uninit_battle_input_buffers()
79:         #if netplay_checksum
80:         uninit_checksum_buffers()
81:         #endif
82:         set_battle_state_connections(null)
83:         #endif
84:
85:         stop_ditty()
86:         stop_music()
87:         stop_sound()
88:     end_if
89:
90:     uninit_ships()   # [ffi/phase1]
91:     free_battle_song()
92:
93:     [validate] in_battle cleared by teardown path(s)
94:     [validate] audio halted and music handle released
95:
96:     return (num_ships < 0)
97: end
```

Branch variants explicitly represented in lifecycle:
- `demo_mode/create_journal` deterministic seed branch vs normal seed branch.
- `super_melee` abort handling (`check_abort` cleared, no forced jump to main menu) and `melee_game_over` non-abort branch.
- `netplay` setup/ready/teardown buffers and connection-state transitions.
- `check_abort` and `check_load` propagate through `process_input` / loop exit / abort label.

---

## `init.c`

### 9) `init_space()` (init.c:115-153) — ref-counted

```text
01: function init_space() -> bool
02:     first_init = (space_ini_cnt == 0)
03:     space_ini_cnt += 1
04:
05:     if first_init and (low_byte(global_current_activity) <= in_encounter):
06:         stars_in_space = capture_drawable(load_graphic(star_mask_pmap_anim))
07:         if stars_in_space == null:
08:             return false
09:
10:         if not load_animation(explosion, boom_big_mask_pmap_anim, boom_med_mask_pmap_anim, boom_sml_mask_pmap_anim):
11:             return false
12:         if not load_animation(blast, blast_big_mask_pmap_anim, blast_med_mask_pmap_anim, blast_sml_mask_pmap_anim):
13:             return false
14:         if not load_animation(asteroid, asteroid_big_mask_pmap_anim, asteroid_med_mask_pmap_anim, asteroid_sml_mask_pmap_anim):
15:             return false
16:     end_if
17:
18:     [validate] refcount incremented even on non-first callers
19:     return true
20: end
```

### 10) `uninit_space()` (init.c:155-182) — ref-counted

```text
01: function uninit_space() -> void
02:     if space_ini_cnt != 0:
03:         space_ini_cnt -= 1
04:         if space_ini_cnt == 0:
05:             free_image(blast)
06:             free_image(explosion)
07:             free_image(asteroid)
08:             destroy_drawable(release_drawable(stars_in_space))
09:             stars_in_space = 0
10:             [validate] shared space assets fully released on final unref
11:         end_if
12:     end_if
13: end
```

### 11) `init_ships()` (init.c:186-277) — full init sequence, `use_rust_ships` split

```text
01: function init_ships() -> size
02:     #if use_rust_ships
03:     return rust_ships_init()  # [ffi/phase1]
04:     #else
05:     init_space()
06:
07:     set_context(status_context)
08:     set_context(space_context)
09:
10:     init_display_list()
11:     init_galaxy()
12:
13:     if in_hq_space():
14:         reinit_queue(race_q[0])
15:         reinit_queue(race_q[1])
16:
17:         build_sis()
18:         load_hyperspace()
19:
20:         num_ships = 1
21:     else:
22:         set_context_fg_frame(screen)
23:         set_context_clip_rect(rect(safe_x, safe_y, space_width, space_height))
24:
25:         set_context_background_color(black_color)
26:         old_context = set_context(screen_context)
27:         set_context_background_color(black_color)
28:         clear_drawable()
29:         set_context(old_context)
30:
31:         if low_byte(global_current_activity) == in_last_battle:
32:             free_gravity_well()
33:         else:
34:             repeat 5 times: spawn_asteroid(null)
35:             repeat 1 time: spawn_planet()
36:
37:         num_ships = num_sides
38:     end_if
39:
40:     return num_ships
41:     #endif
42: end
```

### 12) `count_crew_elements()` (init.c:253-274) — static helper

```text
01: function count_crew_elements() -> count
02:     result = 0
03:     h_element = get_head_element()
04:
05:     while h_element != 0:
06:         element = lock_element(h_element)
07:         h_next = get_succ_element(element)
08:
09:         if element.state_flags has crew_object:
10:             result += 1
11:
12:         unlock_element(h_element)
13:         h_element = h_next
14:     end_while
15:
16:     return result
17: end
```

### 13) `uninit_ships()` (init.c:279-363) — full teardown, `use_rust_ships` split

```text
01: function uninit_ships() -> void
02:     #if use_rust_ships
03:     rust_ships_uninit()  # [ffi/phase1]
04:     return
05:     #else
06:     stop_sound()
07:     uninit_space()
08:
09:     for i in 0 .. num_players-1:
10:         s_ptr[i] = null
11:
12:     crew_retrieved = count_crew_elements()
13:
14:     h_element = get_head_element()
15:     while h_element != 0:
16:         element = lock_element(h_element)
17:         h_next = get_succ_element(element)
18:
19:         if (element.state_flags has player_ship) or (element.death_func == new_ship):
20:             star_ship = get_element_star_ship(element)
21:
22:             if star_ship.race_desc.ship_info.crew_level != 0:
23:                 missing = star_ship.race_desc.ship_info.max_crew - star_ship.race_desc.ship_info.crew_level
24:                 if crew_retrieved >= missing:
25:                     star_ship.race_desc.ship_info.crew_level = star_ship.race_desc.ship_info.max_crew
26:                 else:
27:                     star_ship.race_desc.ship_info.crew_level += crew_retrieved
28:
29:             star_ship.crew_level = star_ship.race_desc.ship_info.crew_level
30:             s_ptr[star_ship.player_nr] = star_ship
31:             free_ship(star_ship.race_desc, true, true)
32:             star_ship.race_desc = 0
33:         end_if
34:
35:         unlock_element(h_element)
36:         h_element = h_next
37:     end_while
38:
39:     global_current_activity &= ~in_battle
40:
41:     if low_byte(global_current_activity) == in_encounter
42:        and not (global_current_activity has check_abort):
43:         for i from num_players-1 down_to 0:
44:             if s_ptr[i] != null and not fleet_is_infinite(i):
45:                 update_ship_frag_crew(s_ptr[i])
46:         [validate] encounter survivors persist post-battle crew/frags
47:     end_if
48:
49:     if low_byte(global_current_activity) != in_encounter:
50:         for i in 0 .. num_players-1:
51:             reinit_queue(race_q[i])
52:
53:         if in_hq_space():
54:             free_hyperspace()
55:     end_if
56:     #endif
57: end
```

---

## `intel.c`

### 14) `computer_intelligence(context, star_ship_ptr)` (intel.c:1-76) — 4 dispatch paths

```text
01: function computer_intelligence(context, star_ship_ptr) -> battle_input_state
02:     # path 1: sa-matra / last-battle special case
03:     if low_byte(global_current_activity) == in_last_battle:
04:         return 0
05:
06:     if star_ship_ptr != null:
07:         # path 2: cyborg-controlled in-battle ship
08:         if star_ship_ptr.control has cyborg_control:
09:             input_state = tactical_intelligence(context, star_ship_ptr)  # [ffi/phase1]
10:
11:             # preserve human warp-escape override for rpg player
12:             if star_ship_ptr.player_nr == rpg_player_num:
13:                 input_state |= (current_input_to_battle_input(context.player_nr) and battle_escape)
14:         else:
15:             # path 3: non-cyborg in-battle ship (human/raw mapped input)
16:             input_state = current_input_to_battle_input(context.player_nr)
17:     else if not (player_control[context.player_nr] has psytron_control):
18:         # path 4a: no ship + non-psytron => idle
19:         input_state = 0
20:     else:
21:         # path 4b: no ship + psytron (menu/selection automation)
22:         switch low_byte(global_current_activity):
23:             case super_melee:
24:                 sleep_thread(one_second >> 1)
25:                 input_state = battle_weapon  # pick random ship behavior
26:             default:
27:                 log_warning("unexpected state in computer_intelligence")
28:                 input_state = 0
29:         end_switch
30:
31:     [validate] exactly one dispatch path assigns returned input_state
32:     return input_state
33: end
```
