# Phase 02: Pseudocode

## Phase ID
`PLAN-20260314-SUPERMELEE.P02`

## Prerequisites
- Required: Phase 01a (Analysis Verification) completed and passed

## Purpose

Produce algorithmic pseudocode for all major SuperMelee-owned components. Implementation phases reference specific line ranges from this pseudocode.

## Component 001: Team Model (`meleesetup.c` -> `team.rs`)

```text
001: FUNCTION melee_team_serialize(team: &MeleeTeam, buffer: &mut [u8])
002:   FOR i IN 0..MELEE_FLEET_SIZE
003:     buffer[i] = team.ships[i] as u8
004:   COPY bounded team-name bytes to buffer[MELEE_FLEET_SIZE..]
005: END

006: FUNCTION melee_team_deserialize(buffer: &[u8]) -> Result<MeleeTeam>
007:   VALIDATE buffer length >= MELEE_FLEET_SIZE + name_size
008:   team = MeleeTeam::default()
009:   FOR i IN 0..MELEE_FLEET_SIZE
010:     ship_id = buffer[i]
011:     IF ship_id is valid MeleeShip THEN team.ships[i] = ship_id
012:     ELSE team.ships[i] = normalize_invalid_ship_id(ship_id)
013:   COPY name bytes into bounded storage
014:   ENSURE valid termination/normalization for UI + persistence contract
015:   RETURN Ok(team)
016: END

017: FUNCTION melee_setup_set_ship(setup: &mut MeleeSetup, side: usize, slot: usize, ship: MeleeShip)
018:   old_ship = setup.teams[side].ships[slot]
019:   IF old_ship == ship THEN RETURN
020:   setup.fleet_value[side] -= ship_cost(old_ship)
021:   setup.fleet_value[side] += ship_cost(ship)
022:   setup.teams[side].ships[slot] = ship
023: END

024: FUNCTION melee_setup_replace_team(setup: &mut MeleeSetup, side: usize, team: &MeleeTeam)
025:   setup.teams[side] = clone(team)
026:   setup.fleet_value[side] = recompute_value(team)
027: END
```

## Component 002: Team Persistence (`loadmele.c` -> `persistence.rs` / `config.rs`)

```text
028: FUNCTION load_team_image(filename: &str) -> Result<MeleeTeam>
029:   file = open_file(melee_dir / filename)?
030:   data = read_all(file)?
031:   RETURN melee_team_deserialize(data)
032: END

033: FUNCTION save_team(team: &MeleeTeam) -> Result<()>
034:   filename = format!("{}.mle", sanitize(team.name))
035:   temp_path = begin_safe_write(melee_dir / filename)?
036:   buffer = melee_team_serialize(team)
037:   IF write_all(temp_path, buffer).is_err() THEN
038:     cleanup_partial_save(temp_path)
039:     RETURN Err
040:   finalize_safe_write(temp_path)
041:   RETURN Ok
042: END

043: FUNCTION init_prebuilt_teams() -> Vec<MeleeTeam>
044:   teams = load_builtin_catalog_definition()
045:   RETURN teams
046: END

047: FUNCTION load_team_list(load_state: &mut TeamBrowserState) -> Result<()>
048:   load_state.entries.clear()
049:   FOR built_in IN prebuilt_catalog
050:     load_state.entries.push(BuiltInEntry(built_in))
051:   FOR entry IN read_dir(melee_dir)?
052:     IF entry.extension() == ".mle" THEN
053:       load_state.entries.push(FileEntry(entry.file_name()))
054:   RETURN Ok
055: END

056: FUNCTION load_melee_config(setup: &mut MeleeSetup, player_control: &mut [Control; NUM_SIDES]) -> bool
057:   file = open_file(config_dir / "melee.cfg")?
058:   IF file shape invalid THEN RETURN false
059:   FOR side IN 0..NUM_SIDES
060:     control = read_control(file)?
061:     player_control[side] = sanitize_startup_control(control)
062:     team = deserialize_team(file)?
063:     setup.replace_team(side, team)
064:   RETURN true
065: END

066: FUNCTION write_melee_config(setup: &MeleeSetup, player_control: &[Control; NUM_SIDES]) -> Result<()>
067:   file = create_file(config_dir / "melee.cfg")?
068:   FOR side IN 0..NUM_SIDES
069:     write_control(file, player_control[side])?
070:     serialize_team(file, setup.team[side])?
071:   RETURN Ok
072: END
```

## Component 003: Setup/Menu Orchestration (`melee.c` -> `melee.rs`)

```text
073: FUNCTION melee()
074:   state = MeleeState::new()
075:   state.random = RandomContext::new(time_counter())
076:   state.browser = init_team_browser_state()
077:   SET activity = SUPER_MELEE
078:   load_melee_info(&mut state)
079:   IF NOT load_melee_config(&mut state.setup, &mut state.player_control) THEN
080:     set_default_teams_from_builtin_catalog(&mut state)
081:   ENTER menu_loop(state)
082:   write_melee_config(&state.setup, &state.player_control)
083:   free_melee_info(&mut state)
084: END

085: FUNCTION menu_loop(state: &mut MeleeState)
086:   WHILE state.running
087:     input = read_menu_input()
088:     MATCH state.subview
089:       None => handle_main_menu_input(state, input)
090:       BuildPick => handle_build_pick_input(state, input)
091:       TeamBrowse => handle_team_browser_input(state, input)
092:       TeamNameEdit => handle_name_edit_input(state, input)
093:   END WHILE
094: END

095: FUNCTION start_melee_button_pressed(state: &mut MeleeState) -> bool
096:   IF NOT both_sides_playable(state.setup) THEN RETURN false
097:   IF netplay_enabled(state) AND NOT netplay_start_ready(state.netplay) THEN RETURN false
098:   start_melee(state)
099:   RETURN true
100: END

101: FUNCTION start_melee(state: &mut MeleeState)
102:   prepare_setup_to_battle_transition()
103:   prepare_player_input_for_battle(state.player_control)
104:   initial_selection = get_initial_melee_combatants(state)?
105:   invoke_battle_with_supermelee_handoff(initial_selection)
106:   restore_supermelee_post_battle_state(state)
107: END
```

## Component 004: Fleet-Edit Ship Picker (`buildpick.c` -> `build_pick.rs`)

```text
108: FUNCTION build_pick_ship(state: &mut MeleeState, side: usize, slot: usize) -> PickResult
109:   picker = BuildPickState::new(side, slot)
110:   LOOP
111:     draw_pick_ui(picker)
112:     input = read_picker_input()
113:     IF input == CANCEL THEN RETURN Cancel
114:     IF input navigates THEN move_picker_cursor(picker, input)
115:     IF input == CONFIRM THEN
116:       chosen_ship = picker.current_ship()
117:       RETURN Confirm(chosen_ship)
118:   END LOOP
119: END

120: FUNCTION apply_build_pick_result(state: &mut MeleeState, result: PickResult)
121:   IF result is Confirm(ship) THEN set_ship(active_side, active_slot, ship)
122:   ELSE leave team state unchanged
123: END
```

## Component 005: Battle-Facing Combatant Selection (`pickmele.c` -> `pick_melee.rs`)

```text
124: FUNCTION get_initial_melee_combatants(state: &mut MeleeState) -> Result<BattleStartSelection>
125:   left = select_combatant_for_side(state, LEFT, initial=true)?
126:   right = select_combatant_for_side(state, RIGHT, initial=true)?
127:   RETURN BattleStartSelection { left, right }
128: END

129: FUNCTION get_next_melee_combatant(state: &mut MeleeState, side: usize, last_loss: CombatantLossInfo) -> Result<Option<BattleReadyCombatant>>
130:   mark_previous_combatant_unavailable(state, side, last_loss)
131:   RETURN select_combatant_for_side(state, side, initial=false)
132: END

133: FUNCTION select_combatant_for_side(state: &mut MeleeState, side: usize, initial: bool) -> Result<Option<BattleReadyCombatant>>
134:   candidate_slot = determine_selection_mode(state, side)
135:   IF local_human_selection_required THEN
136:     slot = prompt_local_fleet_selection(state, side)?
137:   ELSE IF local_auto_selection_required THEN
138:     slot = auto_select_slot(state, side)?
139:   ELSE IF remote_selection_required THEN
140:     slot = await_remote_selection_commit(state, side)?
141:   IF no valid slot available THEN RETURN Ok(None)
142:   ship = state.setup.teams[side].ships[slot]
143:   battle_entry = battle_ship_factory.create_combatant_for_slot(side, slot, ship)?
144:   commit_selection(state, side, slot, ship, battle_entry)
145:   expose_local_selection_to_netplay_if_needed(state, side, slot, ship)
146:   RETURN Ok(Some(battle_entry))
147: END
```

## Component 006: Netplay Boundary (`netplay_boundary.rs`)

```text
148: FUNCTION notify_local_ship_slot_change(state: &mut NetplayBoundaryState, side: usize, slot: usize, ship: MeleeShip)
149:   IF netplay not enabled THEN RETURN
150:   emit_setup_sync_event(ShipSlotChanged { side, slot, ship })
151: END

152: FUNCTION notify_local_team_name_change(state: &mut NetplayBoundaryState, side: usize, name: TeamName)
153:   IF netplay not enabled THEN RETURN
154:   emit_setup_sync_event(TeamNameChanged { side, name })
155: END

156: FUNCTION emit_whole_team_sync(state: &mut NetplayBoundaryState, side: usize, team: &MeleeTeam)
157:   IF netplay not enabled THEN RETURN
158:   emit_setup_sync_event(WholeTeamSync { side, team })
159: END

160: FUNCTION validate_start_preconditions(state: &NetplayBoundaryState) -> bool
161:   IF netplay not enabled THEN RETURN true
162:   RETURN state.connection_ready AND state.local_ready AND state.remote_ready AND state.local_confirmed AND state.remote_confirmed
163: END

164: FUNCTION validate_remote_selection(selection: RemoteSelection, setup: &MeleeSetup, selection_state: &CombatantSelectionState) -> Result<ValidatedSelection>
165:   IF selected slot/ship not present in remote fleet THEN RETURN Err(SemanticInvalid)
166:   IF slot already consumed/eliminated THEN RETURN Err(SemanticInvalid)
167:   RETURN Ok(validated)
168: END

169: FUNCTION accept_remote_selection(state: &mut MeleeState, selection: RemoteSelection) -> Result<BattleReadyCombatant>
170:   validated = validate_remote_selection(selection, &state.setup, &state.selection_state)?
171:   battle_entry = battle_ship_factory.create_combatant_for_slot(validated.side, validated.slot, validated.ship)?
172:   commit_selection(state, validated.side, validated.slot, validated.ship, battle_entry)
173:   RETURN Ok(battle_entry)
174: END
```

## Component 007: Compatibility Audit Decision Points

```text
175: FUNCTION evaluate_builtin_team_exactness_requirement(audit_inputs) -> CompatibilityDecision
176:   IF audit proves exact built-in names/compositions are externally significant THEN RETURN ExactParityRequired
177:   ELSE RETURN SemanticCatalogRequired
178: END

179: FUNCTION evaluate_save_format_exactness_requirement(audit_inputs) -> CompatibilityDecision
180:   IF audit proves byte-for-byte save compatibility is required THEN RETURN ExactParityRequired
181:   ELSE RETURN SemanticReloadabilityRequired
182: END

183: FUNCTION evaluate_ui_timing_exactness_requirement(audit_inputs) -> CompatibilityDecision
184:   IF audit proves setup UI navigation/timing/audio are compatibility-significant THEN RETURN ExactParityRequired
185:   ELSE RETURN SemanticBehaviorRequired
186: END
```

## Verification Commands

```bash
# No code changes in this phase - verify pseudocode artifact exists
ls -la project-plans/20260311/supermelee/plan/02-pseudocode.md
```

## Success Criteria
- [ ] Pseudocode covers all scoped SuperMelee-owned components
- [ ] Battle-engine/runtime internals are not treated as SuperMelee-owned implementation components
- [ ] Combatant-selection pseudocode preserves battle-ready handoff objects rather than bare IDs
- [ ] Netplay pseudocode includes setup sync, start gating, remote-selection validation, and commit semantics
- [ ] Compatibility-sensitive obligations are represented as audit decisions, not pre-assumed exactness
- [ ] Line numbers are consistent and referenceable
