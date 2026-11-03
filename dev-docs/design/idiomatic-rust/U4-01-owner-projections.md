# U4-01 owner-projection retirement table

Measured with `cargo xtask quality report owner_projections --details`. These are exact caller scopes for the approval request on #839. They do not grant an allowance for unsafe, lint, LOC or layer growth.

The 18 previously unassigned calls in navigation events and its string-key helpers have been converted to explicit World arguments. Legacy callers now acquire an owner at their own measured boundary rather than hiding that acquisition in the helpers.

Battle and ship scopes follow the latest #839 orchestrator assignment. Both budgets end at phase 2. Other references below are line numbers in work-items.md. U6-04 assignments cite the front-end/Game conversion and deletion of the remaining bridge, not a battle or ship exemption.

The #839 review identified 16 unsupported assignments. Fixture execution now borrows Cx owners (8 calls removed); scene entry receives coordinator-owned wiring (2 calls relocated); the legacy loader receives World from the live loader (1 call relocated). The nine cargo/report/sphere and glue/speech scopes belong to U6-04: its reserved src/planets/ and src/comm/ paths (line 1967) move to forbid(unsafe_code) modes (line 1974), so these raw-owner users must convert before that move.

The updated ledger follows the Rust-only function renames and scoped transaction reductions. Five new acquisition boundaries replace the removed encounter queue-header accessor and the clock raw-pointer adapter. The four battle callers retire with #837; with_game_clock is reserved by U4-02 at work-items.md:1479. No other retirement assignment changes.

The final cleanup reduces the ledger to 1,130 calls in 542 scopes. Retirement assignments are unchanged; remaining acquisitions stay at the reserved live boundaries.

| Retirement | Budget | Calls | Scopes | Work-item Change/Deletes |
| --- | --- | ---: | ---: | --- |
| U2-10a | `u2_10a_projection` | 76 | 38 | #837; #839 scope decision, ending phase-2 |
| U2-10b | `u2_10b_projection` | 15 | 8 | #838; #839 scope decision, ending phase-2 |
| U3-02 | `u3_02_world_projection` | 90 | 42 | 1379, 1383 |
| U4-02 | `u4_02_world_projection` | 48 | 15 | 1479–1481 |
| U4-03 | `u4_03_world_projection` | 25 | 19 | 1526, 1530–1532 |
| U4-04 | `u4_04_world_projection` | 69 | 57 | 1572, 1576–1578 |
| U5-01 | `u5_01_world_projection` | 4 | 4 | 1631–1632 |
| U5-02 | `u5_02_world_projection` | 257 | 100 | 1680–1682 |
| U5-03 | `u5_03_world_projection` | 20 | 14 | 1730–1731 |
| U5-04 | `u5_04_world_projection` | 44 | 8 | 1775 |
| U6-01 | `u6_01_world_projection` | 58 | 32 | 1824–1827 |
| U6-02 | `u6_02_world_projection` | 44 | 35 | 1875–1876 |
| U6-03 | `u6_03_world_projection` | 262 | 107 | 1925 |
| U6-04 | `u6_04_world_projection` | 118 | 63 | 1967, 1971–1974 |
| Total | | 1130 | 542 | |

Each caller has an exact scope, its current count, source file, and a retirement work-item reference.

## `u2_10a_projection`

Work-items.md Change/Deletes: #837; #839 scope decision, ending phase-2.

| Caller identity | Source | Ceiling |
| --- | --- | ---: |
| `uqm::battle::battle_native::engine::Battle` | `src/battle/battle_native.rs` | 3 |
| `uqm::battle::battle_native::engine::BattleSong` | `src/battle/battle_native.rs` | 5 |
| `uqm::battle::battle_native::engine::FreeBattleSong` | `src/battle/battle_native.rs` | 2 |
| `uqm::battle::battle_native::engine::activity` | `src/battle/battle_native.rs` | 1 |
| `uqm::battle::battle_native::engine::game_state` | `src/battle/battle_native.rs` | 1 |
| `uqm::battle::battle_native::engine::player_control` | `src/battle/battle_native.rs` | 1 |
| `uqm::battle::battle_native::engine::player_input` | `src/battle/battle_native.rs` | 1 |
| `uqm::battle::battle_native::engine::set_activity` | `src/battle/battle_native.rs` | 1 |
| `uqm::battle::battle_native::engine::set_game_state` | `src/battle/battle_native.rs` | 1 |
| `uqm::battle::battle_native::engine::set_player_control` | `src/battle/battle_native.rs` | 1 |
| `uqm::battle::battle_native::engine::set_player_input` | `src/battle/battle_native.rs` | 1 |
| `uqm::battle::build_native::engine::AddEscortShips` | `src/battle/build_native.rs` | 1 |
| `uqm::battle::build_native::engine::CalculateEscortsWorth` | `src/battle/build_native.rs` | 1 |
| `uqm::battle::build_native::engine::CheckAlliance` | `src/battle/build_native.rs` | 1 |
| `uqm::battle::build_native::engine::CountEscortShips` | `src/battle/build_native.rs` | 1 |
| `uqm::battle::build_native::engine::EscortFeasibilityStudy` | `src/battle/build_native.rs` | 1 |
| `uqm::battle::build_native::engine::GetEscortByStarShipIndex` | `src/battle/build_native.rs` | 1 |
| `uqm::battle::build_native::engine::RemoveSomeEscortShips` | `src/battle/build_native.rs` | 1 |
| `uqm::battle::build_native::engine::SetEscortCrewComplement` | `src/battle/build_native.rs` | 1 |
| `uqm::battle::context_access::reader_legacy::update_ship_frag_crew` | `src/battle/context_access/reader_legacy.rs` | 1 |
| `uqm::battle::encount_native::engine::BuildBattle` | `src/battle/encount_native.rs` | 5 |
| `uqm::battle::encount_native::engine::EncounterBattle` | `src/battle/encount_native.rs` | 9 |
| `uqm::battle::encount_native::engine::FleetIsInfinite` | `src/battle/encount_native.rs` | 1 |
| `uqm::battle::encount_native::engine::InitEncounter` | `src/battle/encount_native.rs` | 6 |
| `uqm::battle::encount_native::engine::UninitEncounter` | `src/battle/encount_native.rs` | 12 |
| `uqm::battle::encount_native::engine::encounter_race_exists` | `src/battle/encount_native.rs` | 1 |
| `uqm::battle::encount_native::engine::encounter_star` | `src/battle/encount_native.rs` | 1 |
| `uqm::battle::encount_native::engine::game_string` | `src/battle/encount_native.rs` | 1 |
| `uqm::battle::encount_native::engine::planet_name` | `src/battle/encount_native.rs` | 1 |
| `uqm::battle::galaxy::engine::blocks` | `src/battle/galaxy.rs` | 1 |
| `uqm::battle::galaxy::engine::stars` | `src/battle/galaxy.rs` | 1 |
| `uqm::battle::ship_native::engine::GetInitialStarShips` | `src/battle/ship_native.rs` | 1 |
| `uqm::battle::sounds::engine::PlayMenuSound` | `src/battle/sounds.rs` | 1 |
| `uqm::battle::sounds::engine::remove_sounds` | `src/battle/sounds.rs` | 1 |
| `uqm::battle::status_native::engine::DrawBattleCrewAmount` | `src/battle/status_native.rs` | 1 |
| `uqm::battle::tactrans_native::engine::ResetWinnerStarShip` | `src/battle/tactrans_native.rs` | 1 |
| `uqm::battle::tactrans_native::engine::StopDitty` | `src/battle/tactrans_native.rs` | 2 |
| `uqm::battle::tactrans_native::engine::ditty_playing` | `src/battle/tactrans_native.rs` | 3 |

## `u2_10b_projection`

Work-items.md Change/Deletes: #838; #839 scope decision, ending phase-2.

| Caller identity | Source | Ceiling |
| --- | --- | ---: |
| `uqm::ships::code_resources::descriptor` | `src/ships/code_resources.rs` | 1 |
| `uqm::ships::master_native::cost` | `src/ships/master_native.rs` | 1 |
| `uqm::ships::master_native::engine::FreeMasterShipList` | `src/ships/master_native.rs` | 1 |
| `uqm::ships::master_native::engine::LoadMasterShipList` | `src/ships/master_native.rs` | 1 |
| `uqm::ships::master_native::engine::with_catalog` | `src/ships/master_native.rs` | 1 |
| `uqm::ships::native_adapters::live::rust_bridge_init_battle_arena` | `src/ships/native_adapters.rs` | 8 |
| `uqm::ships::native_adapters::live::uqm_get_current_activity_lobyte` | `src/ships/native_adapters.rs` | 1 |
| `uqm::ships::native_adapters::uninit_ships` | `src/ships/native_adapters.rs` | 1 |

## `u3_02_world_projection`

Work-items.md Change/Deletes: 1379, 1383.

| Caller identity | Source | Ceiling |
| --- | --- | ---: |
| `uqm::pickship::ffi::change_selection` | `src/pickship/ffi.rs` | 2 |
| `uqm::pickship::ffi::draw_armada_pick_ship` | `src/pickship/ffi.rs` | 4 |
| `uqm::pickship::ffi::encounter_ship` | `src/pickship/ffi.rs` | 2 |
| `uqm::pickship::ffi::get_armada_star_ship` | `src/pickship/ffi.rs` | 1 |
| `uqm::supermelee::native::battle_picker::live::BuildPickMeleeFrame` | `src/supermelee/native/battle_picker.rs` | 1 |
| `uqm::supermelee::native::battle_picker::live::FillPickMeleeFrame` | `src/supermelee/native/battle_picker.rs` | 2 |
| `uqm::supermelee::native::battle_picker::live::MeleeGameOver` | `src/supermelee/native/battle_picker.rs` | 3 |
| `uqm::supermelee::native::battle_picker::live::MeleeShipDeath` | `src/supermelee/native/battle_picker.rs` | 2 |
| `uqm::supermelee::native::battle_picker::live::choose` | `src/supermelee/native/battle_picker.rs` | 2 |
| `uqm::supermelee::native::battle_picker::live::draw` | `src/supermelee/native/battle_picker.rs` | 1 |
| `uqm::supermelee::native::battle_picker::live::get_ships` | `src/supermelee/native/battle_picker.rs` | 4 |
| `uqm::supermelee::native::battle_picker::live::value` | `src/supermelee/native/battle_picker.rs` | 2 |
| `uqm::supermelee::native::build_picker::live::build_build_pick_frame` | `src/supermelee/native/build_picker.rs` | 1 |
| `uqm::supermelee::native::build_picker::live::pick` | `src/supermelee/native/build_picker.rs` | 1 |
| `uqm::supermelee::native::load_teams::live::DoLoadTeam` | `src/supermelee/native/load_teams.rs` | 1 |
| `uqm::supermelee::native::load_teams::live::DoSaveTeam` | `src/supermelee/native/load_teams.rs` | 1 |
| `uqm::supermelee::native::load_teams::live::draw` | `src/supermelee/native/load_teams.rs` | 2 |
| `uqm::supermelee::native::load_teams::live::flash` | `src/supermelee/native/load_teams.rs` | 1 |
| `uqm::supermelee::native::load_teams::live::select` | `src/supermelee/native/load_teams.rs` | 2 |
| `uqm::supermelee::native::menu::live::DoMelee` | `src/supermelee/native/menu.rs` | 4 |
| `uqm::supermelee::native::menu::live::Melee` | `src/supermelee/native/menu.rs` | 10 |
| `uqm::supermelee::native::menu::live::battle::confirm` | `src/supermelee/native/menu/live/battle.rs` | 2 |
| `uqm::supermelee::native::menu::live::battle::connecting` | `src/supermelee/native/menu/live/battle.rs` | 3 |
| `uqm::supermelee::native::menu::live::battle::disconnects` | `src/supermelee/native/menu/live/battle.rs` | 2 |
| `uqm::supermelee::native::menu::live::battle::next_control` | `src/supermelee/native/menu/live/battle.rs` | 2 |
| `uqm::supermelee::native::menu::live::battle::ready` | `src/supermelee/native/menu/live/battle.rs` | 1 |
| `uqm::supermelee::native::menu::live::battle::start` | `src/supermelee/native/menu/live/battle.rs` | 5 |
| `uqm::supermelee::native::menu::live::battle::start_button` | `src/supermelee/native/menu/live/battle.rs` | 7 |
| `uqm::supermelee::native::menu::live::boundary::set_player_input_all` | `src/supermelee/native/menu/live/boundary.rs` | 1 |
| `uqm::supermelee::native::menu::live::changes::resetFeedback` | `src/supermelee/native/menu/live/changes.rs` | 1 |
| `uqm::supermelee::native::menu::live::drawing::DrawMeleeShipStrings` | `src/supermelee/native/menu/live/drawing.rs` | 2 |
| `uqm::supermelee::native::menu::live::drawing::Melee_flashSelection` | `src/supermelee/native/menu/live/drawing.rs` | 1 |
| `uqm::supermelee::native::menu::live::drawing::RepairMeleeFrame` | `src/supermelee/native/menu/live/drawing.rs` | 1 |
| `uqm::supermelee::native::menu::live::drawing::controls` | `src/supermelee/native/menu/live/drawing.rs` | 1 |
| `uqm::supermelee::native::menu::live::drawing::init_screen` | `src/supermelee/native/menu/live/drawing.rs` | 2 |
| `uqm::supermelee::native::menu::live::drawing::name` | `src/supermelee/native/menu/live/drawing.rs` | 1 |
| `uqm::supermelee::native::menu::live::drawing::status` | `src/supermelee/native/menu/live/drawing.rs` | 2 |
| `uqm::supermelee::native::menu::live::drawing::worth` | `src/supermelee/native/menu/live/drawing.rs` | 1 |
| `uqm::supermelee::native::menu::live::editing::edit` | `src/supermelee/native/menu/live/editing.rs` | 1 |
| `uqm::supermelee::native::menu::live::load_config` | `src/supermelee/native/menu.rs` | 3 |
| `uqm::supermelee::native::menu::live::select_option` | `src/supermelee/native/menu.rs` | 1 |
| `uqm::supermelee::native::menu::live::write_config` | `src/supermelee/native/menu.rs` | 1 |

## `u4_02_world_projection`

Work-items.md Change/Deletes: 1479–1481.

| Caller identity | Source | Ceiling |
| --- | --- | ---: |
| `uqm::navigation::groups::live::BuildGroups` | `src/navigation/groups.rs` | 6 |
| `uqm::navigation::groups::live::GetGroupInfo` | `src/navigation/groups.rs` | 20 |
| `uqm::navigation::groups::live::PutGroupInfo` | `src/navigation/groups.rs` | 5 |
| `uqm::navigation::groups::live::fleet_queue` | `src/navigation/groups.rs` | 1 |
| `uqm::navigation::groups::live::flush` | `src/navigation/groups.rs` | 6 |
| `uqm::navigation::groups::live::ip_queue` | `src/navigation/groups.rs` | 1 |
| `uqm::navigation::groups::live::npc_queue` | `src/navigation/groups.rs` | 1 |
| `uqm::navigation::groups::live::open` | `src/navigation/groups.rs` | 1 |
| `uqm::navigation::groups::live::seek` | `src/navigation/groups.rs` | 1 |
| `uqm::navigation::groups::read` | `src/navigation/groups.rs` | 1 |
| `uqm::navigation::groups::write` | `src/navigation/groups.rs` | 1 |
| `uqm::time::clock_abi::live::AddEvent` | `src/time/clock_abi.rs` | 1 |
| `uqm::time::clock_abi::live::GetGameClock` | `src/time/clock_abi.rs` | 1 |
| `uqm::time::clock_abi::live::finish_clock_day` | `src/time/clock_abi.rs` | 1 |
| `uqm::time::clock_bridge::with_game_clock` | `src/time/clock_bridge.rs` | 1 |

## `u4_03_world_projection`

Work-items.md Change/Deletes: 1526, 1530–1532.

| Caller identity | Source | Ceiling |
| --- | --- | ---: |
| `uqm::automation::campaign_state::<LiveCampaign as StateSource>::read` | `src/automation/campaign_state.rs` | 1 |
| `uqm::automation::coordinator::<Coordinator>::activate_start_scene` | `src/automation/coordinator.rs` | 2 |
| `uqm::automation::coordinator::<Coordinator>::service_energy` | `src/automation/coordinator.rs` | 2 |
| `uqm::automation::coordinator::<Coordinator>::service_patrol_contact` | `src/automation/coordinator.rs` | 1 |
| `uqm::automation::coordinator::terminal::<Coordinator>::set_terminal` | `src/automation/coordinator/terminal.rs` | 1 |
| `uqm::automation::input_ffi::contact_race` | `src/automation/input_ffi.rs` | 1 |
| `uqm::automation::input_ffi::contact_snapshot` | `src/automation/input_ffi.rs` | 1 |
| `uqm::automation::input_ffi::cyborg_settings` | `src/automation/input_ffi.rs` | 1 |
| `uqm::automation::input_ffi::escort_roster` | `src/automation/input_ffi.rs` | 2 |
| `uqm::automation::input_ffi::game_date` | `src/automation/input_ffi.rs` | 1 |
| `uqm::automation::input_ffi::hyperspace_position` | `src/automation/input_ffi.rs` | 1 |
| `uqm::automation::input_ffi::hyperspace_stopped` | `src/automation/input_ffi.rs` | 1 |
| `uqm::automation::input_ffi::read_flight_state` | `src/automation/input_ffi.rs` | 1 |
| `uqm::automation::input_ffi::service_input` | `src/automation/input_ffi.rs` | 1 |
| `uqm::automation::input_ffi::ship_element_amounts` | `src/automation/input_ffi.rs` | 1 |
| `uqm::automation::input_ffi::ship_resources` | `src/automation/input_ffi.rs` | 1 |
| `uqm::automation::input_ffi::sis_crew_and_landers` | `src/automation/input_ffi.rs` | 1 |
| `uqm::automation::input_ffi::sis_fittings` | `src/automation/input_ffi.rs` | 1 |
| `uqm::automation::input_ffi::star_flight_observation` | `src/automation/input_ffi.rs` | 4 |

## `u4_04_world_projection`

Work-items.md Change/Deletes: 1572, 1576–1578.

| Caller identity | Source | Ceiling |
| --- | --- | ---: |
| `uqm::engine::battle::access::record::ship_queue` | `src/engine/battle/record.rs` | 1 |
| `uqm::mainloop::ffi::get_chmmr_bomb_state` | `src/mainloop/ffi.rs` | 1 |
| `uqm::mainloop::ffi::get_crew_enlisted` | `src/mainloop/ffi.rs` | 1 |
| `uqm::mainloop::ffi::get_current_activity` | `src/mainloop/ffi.rs` | 1 |
| `uqm::mainloop::ffi::get_global_flags_and_data` | `src/mainloop/ffi.rs` | 1 |
| `uqm::mainloop::ffi::get_kohr_ah_killed_all` | `src/mainloop/ffi.rs` | 1 |
| `uqm::mainloop::ffi::get_last_activity` | `src/mainloop/ffi.rs` | 1 |
| `uqm::mainloop::ffi::get_starbase_available` | `src/mainloop/ffi.rs` | 1 |
| `uqm::mainloop::ffi::load_kernel` | `src/mainloop/ffi.rs` | 2 |
| `uqm::mainloop::ffi::set_chmmr_bomb_state` | `src/mainloop/ffi.rs` | 1 |
| `uqm::mainloop::ffi::set_current_activity` | `src/mainloop/ffi.rs` | 1 |
| `uqm::mainloop::ffi::set_last_activity` | `src/mainloop/ffi.rs` | 1 |
| `uqm::mainloop::ffi::set_player_input_all_or_explode` | `src/mainloop/ffi.rs` | 1 |
| `uqm::mainloop::ffi::zero_global_velocity` | `src/mainloop/ffi.rs` | 1 |
| `uqm::mainloop::game_loop::cffi::<CffiOps as GameLoopOps>::add_initial_game_events` | `src/mainloop/game_loop.rs` | 1 |
| `uqm::mainloop::game_loop::cffi::<CffiOps as GameLoopOps>::clear_player_input_all` | `src/mainloop/game_loop.rs` | 1 |
| `uqm::mainloop::game_loop::cffi::<CffiOps as GameLoopOps>::free_kernel` | `src/mainloop/game_loop.rs` | 1 |
| `uqm::mainloop::game_loop::cffi::<CffiOps as GameLoopOps>::get_current_activity` | `src/mainloop/game_loop.rs` | 1 |
| `uqm::mainloop::game_loop::cffi::<CffiOps as GameLoopOps>::load_kernel` | `src/mainloop/game_loop.rs` | 2 |
| `uqm::mainloop::input_abi::live::DoInput` | `src/mainloop/input_abi.rs` | 1 |
| `uqm::mainloop::lifecycle_abi::background_init_kernel` | `src/mainloop/lifecycle_abi.rs` | 3 |
| `uqm::mainloop::lifecycle_abi::on_battle_frame` | `src/mainloop/lifecycle_abi.rs` | 2 |
| `uqm::mainloop::lifecycle_abi::signal_stop_main_thread` | `src/mainloop/lifecycle_abi.rs` | 1 |
| `uqm::mainloop::restart_menu::menu_abi::uqm_do_popup_window_msg` | `src/mainloop/restart_menu/menu_abi.rs` | 1 |
| `uqm::mainloop::restart_menu::menu_abi::uqm_set_player_control` | `src/mainloop/restart_menu/menu_abi.rs` | 1 |
| `uqm::mainloop::restart_menu::render_abi::draw_restart_menu_graphic` | `src/mainloop/restart_menu/render_abi.rs` | 1 |
| `uqm::mainloop::restart_menu::restart_ops::<CffiOps as RestartMenuOps>::create_flash_overlay` | `src/mainloop/restart_menu/restart_ops.rs` | 1 |
| `uqm::mainloop::restart_menu::restart_ops::<CffiOps as RestartMenuOps>::get_current_activity` | `src/mainloop/restart_menu/restart_ops.rs` | 1 |
| `uqm::mainloop::restart_menu::restart_ops::<CffiOps as RestartMenuOps>::set_screen_context` | `src/mainloop/restart_menu/restart_ops.rs` | 1 |
| `uqm::mainloop::state_abi::get_last_activity` | `src/mainloop/state_abi.rs` | 1 |
| `uqm::mainloop::state_abi::rust_get_avail_race_queue` | `src/mainloop/state_abi.rs` | 1 |
| `uqm::mainloop::state_abi::rust_get_built_ship_queue` | `src/mainloop/state_abi.rs` | 1 |
| `uqm::mainloop::state_abi::rust_get_encounter_queue` | `src/mainloop/state_abi.rs` | 1 |
| `uqm::mainloop::state_abi::rust_get_ip_group_queue` | `src/mainloop/state_abi.rs` | 1 |
| `uqm::mainloop::state_abi::rust_get_npc_built_ship_queue` | `src/mainloop/state_abi.rs` | 1 |
| `uqm::mainloop::state_abi::rust_set_cur_star_desc_ptr_null` | `src/mainloop/state_abi.rs` | 1 |
| `uqm::mainloop::state_abi::set_current_activity` | `src/mainloop/state_abi.rs` | 1 |
| `uqm::mainloop::state_abi::set_last_activity` | `src/mainloop/state_abi.rs` | 1 |
| `uqm::mainloop::state_abi::uqm_get_chmmr_bomb_state` | `src/mainloop/state_abi.rs` | 1 |
| `uqm::mainloop::state_abi::uqm_get_crew_enlisted` | `src/mainloop/state_abi.rs` | 1 |
| `uqm::mainloop::state_abi::uqm_get_global_flags_and_data` | `src/mainloop/state_abi.rs` | 1 |
| `uqm::mainloop::state_abi::uqm_get_kohr_ah_killed_all` | `src/mainloop/state_abi.rs` | 1 |
| `uqm::mainloop::state_abi::uqm_get_starbase_available` | `src/mainloop/state_abi.rs` | 1 |
| `uqm::mainloop::state_abi::uqm_set_chmmr_bomb_state` | `src/mainloop/state_abi.rs` | 1 |
| `uqm::mainloop::state_abi::uqm_set_player_input_all_or_explode` | `src/mainloop/state_abi.rs` | 1 |
| `uqm::mainloop::state_abi::uqm_zero_global_velocity` | `src/mainloop/state_abi.rs` | 1 |
| `uqm::savegame::live::load::LoadGame` | `src/savegame/live/load.rs` | 1 |
| `uqm::savegame::live::load::finish_load` | `src/savegame/live/load.rs` | 2 |
| `uqm::savegame::live::load::install` | `src/savegame/live/load.rs` | 1 |
| `uqm::savegame::live::save::capture` | `src/savegame/live/save.rs` | 4 |
| `uqm::savegame::runtime::live::capture_live_star` | `src/savegame/runtime/live.rs` | 1 |
| `uqm::savegame::runtime::live::capture_live_summary` | `src/savegame/runtime/live.rs` | 2 |
| `uqm::savegame::runtime::live::prepare_live_hyper_state` | `src/savegame/runtime/live.rs` | 1 |
| `uqm::savegame::runtime::live::prepare_live_solar_location` | `src/savegame/runtime/live.rs` | 3 |
| `uqm::savegame::runtime::location::persist_planet_change` | `src/savegame/runtime/location.rs` | 1 |
| `uqm::savegame::runtime::statefiles::init_group_info` | `src/savegame/runtime/statefiles.rs` | 1 |
| `uqm::savegame::runtime::summary::capture_summary` | `src/savegame/runtime/summary.rs` | 1 |

## `u5_01_world_projection`

Work-items.md Change/Deletes: 1631–1632.

| Caller identity | Source | Ceiling |
| --- | --- | ---: |
| `uqm::planets::generate::special::<LiveStory as Story>::flag` | `src/planets/generate/special.rs` | 1 |
| `uqm::planets::generate::special::<LiveStory as Story>::set_flag` | `src/planets/generate/special.rs` | 1 |
| `uqm::planets::generate::special::get_game_state_byte` | `src/planets/generate/special.rs` | 1 |
| `uqm::planets::generate::special::set_game_state_byte` | `src/planets/generate/special.rs` | 1 |

## `u5_02_world_projection`

Work-items.md Change/Deletes: 1680–1682.

| Caller identity | Source | Ceiling |
| --- | --- | ---: |
| `uqm::planets::c_api::<LiveGame as FleetGroups>::battle_group_ref` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::<LiveGame as FleetGroups>::head_ip_group` | `src/planets/c_api.rs` | 2 |
| `uqm::planets::c_api::<LiveGame as FleetGroups>::set_battle_group_ref` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::<LiveGame as Npcs>::clear_battle_group_ref` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::<LiveGame as Npcs>::for_each_ip_group` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::<LiveGame as Npcs>::init_ip_groups` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::<LiveGame as Npcs>::reinit_ip_group_q` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::<LiveGame as Npcs>::reinit_npc_built_ship_q` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::<LiveGame as Npcs>::yehat_civil_war` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::<LiveGame as PlanetNames>::set_battle_planet` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::<LiveGame as PlanetNames>::set_planet_name` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::<LiveGame as crate::planets::generate::sam::GuardSite>::for_each_ip_group` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::<LiveGame as crate::planets::generate::sam::GuardSite>::ip_location` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::<LiveGame as crate::planets::generate::sam::GuardSite>::set_and_transform_ip_location` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::begin_urquan_encounter` | `src/planets/c_api.rs` | 4 |
| `uqm::planets::c_api::built_ship_count` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::built_ship_queue_has_ships` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::clone_npc_ship` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::current_star_index` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::current_star_point` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::fuel_on_board` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::glob` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::star_indexes_before_current` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::start_sphere_tracking` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::c_api::take_encounter` | `src/planets/c_api.rs` | 1 |
| `uqm::planets::devices::DevicesMenu` | `src/planets/devices.rs` | 5 |
| `uqm::planets::devices::InventoryDevices` | `src/planets/devices.rs` | 1 |
| `uqm::planets::devices::draw` | `src/planets/devices.rs` | 1 |
| `uqm::planets::devices::effect_state` | `src/planets/devices.rs` | 3 |
| `uqm::planets::devices::frame` | `src/planets/devices.rs` | 1 |
| `uqm::planets::devices::invoke` | `src/planets/devices.rs` | 23 |
| `uqm::planets::devices::reset_groups` | `src/planets/devices.rs` | 1 |
| `uqm::planets::devices::use_caster` | `src/planets/devices.rs` | 13 |
| `uqm::planets::ffi::current_star_type` | `src/planets/ffi.rs` | 1 |
| `uqm::planets::orbit::live::FreePlanet` | `src/planets/orbit.rs` | 1 |
| `uqm::planets::orbit::live::LoadPlanet` | `src/planets/orbit.rs` | 1 |
| `uqm::planets::orbit::live::create_context` | `src/planets/orbit.rs` | 2 |
| `uqm::planets::orbit::live::do_orbit` | `src/planets/orbit.rs` | 6 |
| `uqm::planets::orbit::live::draw_display` | `src/planets/orbit.rs` | 1 |
| `uqm::planets::pstarmap::DrawStarMap` | `src/planets/pstarmap.rs` | 3 |
| `uqm::planets::pstarmap::StarMap` | `src/planets/pstarmap.rs` | 8 |
| `uqm::planets::pstarmap::UpdateCursorInfo` | `src/planets/pstarmap.rs` | 2 |
| `uqm::planets::pstarmap::UpdateCursorLocation` | `src/planets/pstarmap.rs` | 2 |
| `uqm::planets::pstarmap::current_location` | `src/planets/pstarmap.rs` | 4 |
| `uqm::planets::pstarmap::flashCurrentLocation` | `src/planets/pstarmap.rs` | 1 |
| `uqm::planets::pstarmap::fleet_queue` | `src/planets/pstarmap.rs` | 1 |
| `uqm::planets::pstarmap::game_string` | `src/planets/pstarmap.rs` | 1 |
| `uqm::planets::pstarmap::portal_open` | `src/planets/pstarmap.rs` | 1 |
| `uqm::planets::pstarmap::sphere_text` | `src/planets/pstarmap.rs` | 1 |
| `uqm::planets::pstarmap::universeToDispx` | `src/planets/pstarmap.rs` | 1 |
| `uqm::planets::pstarmap::universeToDispy` | `src/planets/pstarmap.rs` | 1 |
| `uqm::planets::pstarmap::which_space` | `src/planets/pstarmap.rs` | 1 |
| `uqm::planets::roster::live::RosterMenu` | `src/planets/roster.rs` | 1 |
| `uqm::planets::roster::live::delta_crew` | `src/planets/roster.rs` | 1 |
| `uqm::planets::roster::live::modify` | `src/planets/roster.rs` | 3 |
| `uqm::planets::roster::live::support_ship` | `src/planets/roster.rs` | 1 |
| `uqm::planets::scan::live::GetScanContext` | `src/planets/scan/live.rs` | 2 |
| `uqm::planets::scan::live::cursor_frame` | `src/planets/scan/live.rs` | 1 |
| `uqm::planets::scan::live::dispatch_lander` | `src/planets/scan/live.rs` | 5 |
| `uqm::planets::scan::live::display::pc` | `src/planets/scan/live/display.rs` | 2 |
| `uqm::planets::scan::live::display::three_do` | `src/planets/scan/live/display.rs` | 1 |
| `uqm::planets::scan::live::pick_input` | `src/planets/scan/live.rs` | 1 |
| `uqm::planets::scan::live::scan_input` | `src/planets/scan/live.rs` | 4 |
| `uqm::planets::scan::live::scan_planet` | `src/planets/scan/live.rs` | 1 |
| `uqm::planets::scan::live::spawn_fwiffo` | `src/planets/scan/live.rs` | 1 |
| `uqm::planets::solarsys::drawing::create_star_background` | `src/planets/solarsys/drawing.rs` | 3 |
| `uqm::planets::solarsys::drawing::draw_inner_system` | `src/planets/solarsys/drawing.rs` | 1 |
| `uqm::planets::solarsys::drawing::draw_outer_system` | `src/planets/solarsys/drawing.rs` | 1 |
| `uqm::planets::solarsys::drawing::draw_system` | `src/planets/solarsys/drawing.rs` | 2 |
| `uqm::planets::solarsys::flight::check_intersect` | `src/planets/solarsys/flight.rs` | 1 |
| `uqm::planets::solarsys::flight::check_ship_location` | `src/planets/solarsys/flight.rs` | 2 |
| `uqm::planets::solarsys::flight::check_zoom_level` | `src/planets/solarsys/flight.rs` | 1 |
| `uqm::planets::solarsys::flight::enter_inner_system` | `src/planets/solarsys/flight.rs` | 1 |
| `uqm::planets::solarsys::flight::enter_orbital` | `src/planets/solarsys/flight.rs` | 1 |
| `uqm::planets::solarsys::flight::get_collision_frame` | `src/planets/solarsys/flight.rs` | 1 |
| `uqm::planets::solarsys::flight::glob` | `src/planets/solarsys/flight.rs` | 1 |
| `uqm::planets::solarsys::flight::ip_frame` | `src/planets/solarsys/flight.rs` | 1 |
| `uqm::planets::solarsys::flight::leave_inner_system` | `src/planets/solarsys/flight.rs` | 1 |
| `uqm::planets::solarsys::flight::process_ship_controls` | `src/planets/solarsys/flight.rs` | 1 |
| `uqm::planets::solarsys::flight::scale_system` | `src/planets/solarsys/flight.rs` | 1 |
| `uqm::planets::solarsys::lifecycle::ExploreSolarSys` | `src/planets/solarsys/lifecycle.rs` | 7 |
| `uqm::planets::solarsys::lifecycle::GetNamedPlanetaryBody` | `src/planets/solarsys/lifecycle.rs` | 2 |
| `uqm::planets::solarsys::lifecycle::SaveSolarSysLocation` | `src/planets/solarsys/lifecycle.rs` | 5 |
| `uqm::planets::solarsys::lifecycle::do_ip_flight` | `src/planets/solarsys/lifecycle.rs` | 3 |
| `uqm::planets::solarsys::lifecycle::do_solar_sys_menu` | `src/planets/solarsys/lifecycle.rs` | 7 |
| `uqm::planets::solarsys::lifecycle::end_interplanetary` | `src/planets/solarsys/lifecycle.rs` | 3 |
| `uqm::planets::solarsys::lifecycle::enter_planet_orbit` | `src/planets/solarsys/lifecycle.rs` | 11 |
| `uqm::planets::solarsys::lifecycle::free_solar_sys` | `src/planets/solarsys/lifecycle.rs` | 1 |
| `uqm::planets::solarsys::lifecycle::game_string` | `src/planets/solarsys/lifecycle.rs` | 1 |
| `uqm::planets::solarsys::lifecycle::glob` | `src/planets/solarsys/lifecycle.rs` | 1 |
| `uqm::planets::solarsys::lifecycle::init_solar_sys` | `src/planets/solarsys/lifecycle.rs` | 16 |
| `uqm::planets::solarsys::lifecycle::init_solar_sys_sis_characteristics` | `src/planets/solarsys/lifecycle.rs` | 2 |
| `uqm::planets::solarsys::lifecycle::load_solar_sys` | `src/planets/solarsys/lifecycle.rs` | 16 |
| `uqm::planets::solarsys::lifecycle::reset_solar_sys` | `src/planets/solarsys/lifecycle.rs` | 1 |
| `uqm::planets::solarsys::lifecycle::save_non_orbital_location` | `src/planets/solarsys/lifecycle.rs` | 6 |
| `uqm::planets::solarsys::lifecycle::solar_sys_menu` | `src/planets/solarsys/lifecycle.rs` | 1 |
| `uqm::planets::solarsys::lifecycle::uninit_solar_sys` | `src/planets/solarsys/lifecycle.rs` | 5 |
| `uqm::planets::starmap::FindStar` | `src/planets/starmap.rs` | 1 |
| `uqm::planets::starmap::naming::GetClusterName` | `src/planets/starmap.rs` | 2 |
| `uqm::planets::starmap_cursor::DoMoveCursor` | `src/planets/starmap_cursor.rs` | 7 |

## `u5_03_world_projection`

Work-items.md Change/Deletes: 1730–1731.

| Caller identity | Source | Ceiling |
| --- | --- | ---: |
| `uqm::planet_side::adapters::<CffiShipStatus as ShipStatusPort>::apply` | `src/planet_side/adapters.rs` | 2 |
| `uqm::planet_side::adapters::<CffiShipStatus>::snapshot` | `src/planet_side/adapters.rs` | 1 |
| `uqm::planet_side::adapters::read_player_one_input` | `src/planet_side/adapters.rs` | 1 |
| `uqm::planet_side::generation_adapter::<CffiSurfaceGenerator as SurfaceGenerator>::can_node` | `src/planet_side/generation_adapter.rs` | 1 |
| `uqm::planet_side::generation_adapter::<CffiSurfaceGenerator as SurfaceGenerator>::generate` | `src/planet_side/generation_adapter.rs` | 1 |
| `uqm::planet_side::generation_adapter::<CffiSurfaceGenerator as SurfaceGenerator>::pickup` | `src/planet_side/generation_adapter.rs` | 2 |
| `uqm::planet_side::generation_adapter::<CffiSurfaceGenerator>::node_key` | `src/planet_side/generation_adapter.rs` | 1 |
| `uqm::planet_side::graphics_adapter::<CffiPlanetSideGraphics<'_,A> as PlanetSideGraphics>::render` | `src/planet_side/graphics_adapter.rs` | 1 |
| `uqm::planet_side::init_lander::InitLander` | `src/planet_side/init_lander.rs` | 1 |
| `uqm::planet_side::init_lander::draw_lander_status` | `src/planet_side/init_lander.rs` | 2 |
| `uqm::planet_side::init_lander::sis_num_landers` | `src/planet_side/init_lander.rs` | 1 |
| `uqm::planet_side::init_lander::sis_total_element_mass` | `src/planet_side/init_lander.rs` | 1 |
| `uqm::planet_side::report_adapter::<CffiDiscoveryReport as DiscoveryReportPort>::present` | `src/planet_side/report_adapter.rs` | 1 |
| `uqm::planet_side::session_factory::production_upgrades` | `src/planet_side/session_factory.rs` | 4 |

## `u5_04_world_projection`

Work-items.md Change/Deletes: 1775.

| Caller identity | Source | Ceiling |
| --- | --- | ---: |
| `uqm::navigation::hyper_data::FreeHyperspace` | `src/navigation/hyper_data.rs` | 1 |
| `uqm::navigation::hyper_data::LoadHyperspace` | `src/navigation/hyper_data.rs` | 13 |
| `uqm::navigation::hyper_data::free_hyper_data` | `src/navigation/hyper_data.rs` | 1 |
| `uqm::navigation::hyper_data::load` | `src/navigation/hyper_data.rs` | 6 |
| `uqm::navigation::hyper_menu::hyperspace_menu` | `src/navigation/hyper_menu.rs` | 4 |
| `uqm::navigation::hyper_menu::input` | `src/navigation/hyper_menu.rs` | 5 |
| `uqm::navigation::hyperspace::live::MoveSIS` | `src/navigation/hyperspace.rs` | 13 |
| `uqm::navigation::hyperspace::live::rust_reset_hyper_distance` | `src/navigation/hyperspace.rs` | 1 |

## `u6_01_world_projection`

Work-items.md Change/Deletes: 1824–1827.

| Caller identity | Source | Ceiling |
| --- | --- | ---: |
| `uqm::comm::commglue_abi::runtime::GetAllianceName` | `src/comm/commglue_abi.rs` | 1 |
| `uqm::comm::commglue_abi::runtime::get` | `src/comm/commglue_abi.rs` | 1 |
| `uqm::comm::commglue_abi::runtime::getSegue` | `src/comm/commglue_abi.rs` | 3 |
| `uqm::comm::commglue_abi::runtime::set` | `src/comm/commglue_abi.rs` | 1 |
| `uqm::comm::commglue_abi::runtime::setSegue` | `src/comm/commglue_abi.rs` | 3 |
| `uqm::comm::dispatch::avail_race_queue` | `src/comm/dispatch.rs` | 1 |
| `uqm::comm::dispatch::built_ship_queue` | `src/comm/dispatch.rs` | 1 |
| `uqm::comm::dispatch::encounter_queue` | `src/comm/dispatch.rs` | 1 |
| `uqm::comm::dispatch::init_communication` | `src/comm/dispatch.rs` | 4 |
| `uqm::comm::dispatch::init_communication_inner` | `src/comm/dispatch.rs` | 8 |
| `uqm::comm::dispatch::npc_built_ship_queue` | `src/comm/dispatch.rs` | 1 |
| `uqm::comm::dispatch::place_flagship_at_star` | `src/comm/dispatch.rs` | 1 |
| `uqm::comm::dispatch::prepare_samatra_dev_fixture` | `src/comm/dispatch.rs` | 1 |
| `uqm::comm::dispatch::rust_init_race_dispatch` | `src/comm/dispatch.rs` | 2 |
| `uqm::comm::dispatch::rust_race_communication` | `src/comm/dispatch.rs` | 4 |
| `uqm::comm::ffi::get_alliance_name` | `src/comm/ffi.rs` | 1 |
| `uqm::comm::ffi::get_alliance_name_full` | `src/comm/ffi.rs` | 2 |
| `uqm::comm::ffi::rust_npc_phrase_cb` | `src/comm/ffi.rs` | 1 |
| `uqm::comm::hail::game_string` | `src/comm/hail.rs` | 1 |
| `uqm::comm::hail::hail_alien` | `src/comm/hail.rs` | 3 |
| `uqm::comm::hail::is_starbase_conversation` | `src/comm/hail.rs` | 2 |
| `uqm::comm::hail::planet_name` | `src/comm/hail.rs` | 1 |
| `uqm::comm::oscill_abi::runtime::DrawOscilloscope` | `src/comm/oscill_abi.rs` | 2 |
| `uqm::comm::sis_graphics::<CommWindow as CommWindowGfx>::draw_scroll_indicator` | `src/comm/sis_graphics.rs` | 1 |
| `uqm::comm::sis_graphics::<CommWindow as CommWindowGfx>::space_context` | `src/comm/sis_graphics.rs` | 1 |
| `uqm::comm::sis_graphics::draw_summary_page_contents` | `src/comm/sis_graphics.rs` | 2 |
| `uqm::comm::sis_graphics::game_string` | `src/comm/sis_graphics.rs` | 1 |
| `uqm::comm::talk_segue::c_bridge::clear_last_activity_load_flag` | `src/comm/talk_segue.rs` | 1 |
| `uqm::comm::talk_segue::dinput::do_last_replay_cb` | `src/comm/talk_segue.rs` | 2 |
| `uqm::comm::talk_segue::init_speech_graphics` | `src/comm/talk_segue.rs` | 1 |
| `uqm::comm::talk_segue::set_slider_image_frame` | `src/comm/talk_segue.rs` | 1 |
| `uqm::comm::talk_segue::update_speech_graphics` | `src/comm/talk_segue.rs` | 2 |

## `u6_02_world_projection`

Work-items.md Change/Deletes: 1875–1876.

| Caller identity | Source | Ceiling |
| --- | --- | ---: |
| `uqm::comm::races::alliance::live::<LiveScript<H> as AllianceApi<H>>::set_allied` | `src/comm/races/alliance.rs` | 1 |
| `uqm::comm::races::alliance::live::<LiveScript<H> as AllianceApi<H>>::track` | `src/comm/races/alliance.rs` | 1 |
| `uqm::comm::races::blackur::api::<super::super::script::live::LiveScript<Handler> as BlackurApi>::battle_group` | `src/comm/races/blackur/api.rs` | 2 |
| `uqm::comm::races::chmmr::api::<super::super::script::live::LiveScript<Handler> as ChmmrApi>::refit` | `src/comm/races/chmmr/api.rs` | 1 |
| `uqm::comm::races::comandr::<ComandrDialogue as super::RaceDialogue>::init` | `src/comm/races/comandr.rs` | 1 |
| `uqm::comm::races::druuge::api::<crate::comm::races::script::live::LiveScript<Handler> as DruugeApi>::crew_cost` | `src/comm/races/druuge/api.rs` | 1 |
| `uqm::comm::races::druuge::api::<crate::comm::races::script::live::LiveScript<Handler> as DruugeApi>::has_escorts` | `src/comm/races/druuge/api.rs` | 1 |
| `uqm::comm::races::druuge::api::<crate::comm::races::script::live::LiveScript<Handler> as DruugeApi>::set_crew_cost` | `src/comm/races/druuge/api.rs` | 1 |
| `uqm::comm::races::druuge::api::<crate::comm::races::script::live::LiveScript<Handler> as DruugeApi>::ship` | `src/comm/races/druuge/api.rs` | 1 |
| `uqm::comm::races::melnorm::api::live::<LiveScript<Handler> as MelnormApi>::module_cost` | `src/comm/races/melnorm/api.rs` | 1 |
| `uqm::comm::races::melnorm::api::live::<LiveScript<Handler> as MelnormApi>::set_module_cost` | `src/comm/races/melnorm/api.rs` | 1 |
| `uqm::comm::races::melnorm::api::live::<LiveScript<Handler> as MelnormApi>::set_ship` | `src/comm/races/melnorm/api.rs` | 1 |
| `uqm::comm::races::melnorm::api::live::<LiveScript<Handler> as MelnormApi>::ship` | `src/comm/races/melnorm/api.rs` | 1 |
| `uqm::comm::races::pkunk::api::<crate::comm::races::script::live::LiveScript<Handler> as PkunkApi>::date` | `src/comm/races/pkunk/api.rs` | 1 |
| `uqm::comm::races::rebel::api::<super::super::script::live::LiveScript<Handler> as RebelApi>::date` | `src/comm/races/rebel/api.rs` | 1 |
| `uqm::comm::races::script::live::<LiveScript<H> as ScriptApi<H>>::add_lander` | `src/comm/races/script.rs` | 1 |
| `uqm::comm::races::script::live::<LiveScript<H> as ScriptApi<H>>::get` | `src/comm/races/script.rs` | 1 |
| `uqm::comm::races::script::live::<LiveScript<H> as ScriptApi<H>>::response` | `src/comm/races/script.rs` | 1 |
| `uqm::comm::races::script::live::<LiveScript<H> as ScriptApi<H>>::set` | `src/comm/races/script.rs` | 1 |
| `uqm::comm::races::script::live::<LiveScript<H> as ScriptApi<H>>::supplies` | `src/comm/races/script.rs` | 3 |
| `uqm::comm::races::slyland::api::live::<LiveScript<Handler> as SlylandApi>::position` | `src/comm/races/slyland/api.rs` | 2 |
| `uqm::comm::races::starbas::api::live::<LiveScript<Handler> as StarbaseApi>::date` | `src/comm/races/starbas/api.rs` | 1 |
| `uqm::comm::races::starbas::api::live::<LiveScript<Handler> as StarbaseApi>::mineral_amount` | `src/comm/races/starbas/api.rs` | 1 |
| `uqm::comm::races::starbas::api::live::<LiveScript<Handler> as StarbaseApi>::mineral_worth` | `src/comm/races/starbas/api.rs` | 1 |
| `uqm::comm::races::starbas::api::live::<LiveScript<Handler> as StarbaseApi>::remove_minerals` | `src/comm/races/starbas/api.rs` | 2 |
| `uqm::comm::races::starbas::api::live::<LiveScript<Handler> as StarbaseApi>::ship` | `src/comm/races/starbas/api.rs` | 1 |
| `uqm::comm::races::starbas::api::live::<LiveScript<Handler> as StarbaseApi>::skip_sale` | `src/comm/races/starbas/api.rs` | 1 |
| `uqm::comm::races::starbas::api::live::<LiveScript<Handler> as StarbaseApi>::using_speech` | `src/comm/races/starbas/api.rs` | 1 |
| `uqm::comm::races::syreen::api::<super::super::script::live::LiveScript<Handler> as SyreenApi>::using_speech` | `src/comm/races/syreen/api.rs` | 1 |
| `uqm::comm::races::talkpet::api::<super::super::script::live::LiveScript<Handler> as TalkpetApi>::flags` | `src/comm/races/talkpet/api.rs` | 1 |
| `uqm::comm::races::talkpet::api::<super::super::script::live::LiveScript<Handler> as TalkpetApi>::npc_ships` | `src/comm/races/talkpet/api.rs` | 1 |
| `uqm::comm::races::talkpet::api::<super::super::script::live::LiveScript<Handler> as TalkpetApi>::umgah_alive` | `src/comm/races/talkpet/api.rs` | 1 |
| `uqm::comm::races::umgah::api::<super::super::script::live::LiveScript<Handler> as UmgahApi>::add_biomass` | `src/comm/races/umgah/api.rs` | 2 |
| `uqm::comm::races::urquan::api::<super::super::script::live::LiveScript<Handler> as UrquanApi>::group_battle` | `src/comm/races/urquan/api.rs` | 3 |
| `uqm::comm::races::zoqfot::api::live::<LiveScript<Handler> as ZoqApi>::date` | `src/comm/races/zoqfot/api.rs` | 2 |

## `u6_03_world_projection`

Work-items.md Change/Deletes: 1925.

| Caller identity | Source | Ceiling |
| --- | --- | ---: |
| `uqm::starbase::cffi::<CffiPlatform as Gfx>::game_string` | `src/starbase/cffi.rs` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Gfx>::play_frame` | `src/starbase/cffi.rs` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Gfx>::set_font_effect` | `src/starbase/cffi.rs` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Platform>::crew_cost` | `src/starbase/cffi.rs` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Platform>::game_state` | `src/starbase/cffi.rs` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Platform>::set_crew_cost` | `src/starbase/cffi.rs` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Platform>::set_drive_slot` | `src/starbase/cffi.rs` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Platform>::set_fuel_on_board` | `src/starbase/cffi.rs` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Platform>::set_game_state` | `src/starbase/cffi.rs` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Platform>::set_jet_slot` | `src/starbase/cffi.rs` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Platform>::set_module_slot` | `src/starbase/cffi.rs` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Platform>::set_num_landers` | `src/starbase/cffi.rs` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Platform>::sis` | `src/starbase/cffi.rs` | 1 |
| `uqm::starbase::cffi::<CffiPlatform>::context_ptr` | `src/starbase/cffi.rs` | 1 |
| `uqm::starbase::cffi::<CffiPlatform>::font_ptr` | `src/starbase/cffi.rs` | 1 |
| `uqm::starbase::cffi::avail_race_queue` | `src/starbase/cffi.rs` | 1 |
| `uqm::starbase::cffi::built_ship_queue` | `src/starbase/cffi.rs` | 1 |
| `uqm::ui::border::draw_sisframe` | `src/ui/border.rs` | 2 |
| `uqm::ui::confirm::DoConfirmExit` | `src/ui/confirm.rs` | 4 |
| `uqm::ui::confirm::DoPopupWindow` | `src/ui/confirm.rs` | 2 |
| `uqm::ui::confirm::do_popup` | `src/ui/confirm.rs` | 1 |
| `uqm::ui::confirm::draw_confirmation` | `src/ui/confirm.rs` | 1 |
| `uqm::ui::confirm::game_string` | `src/ui/confirm.rs` | 1 |
| `uqm::ui::getchar::DoTextEntry` | `src/ui/getchar.rs` | 1 |
| `uqm::ui::menu::DoMenuChooser` | `src/ui/menu.rs` | 1 |
| `uqm::ui::menu::DrawMenuStateStrings` | `src/ui/menu.rs` | 5 |
| `uqm::ui::menu::draw_pc_menu` | `src/ui/menu.rs` | 3 |
| `uqm::ui::menu::game_string` | `src/ui/menu.rs` | 1 |
| `uqm::ui::menu::previous_menu_state` | `src/ui/menu.rs` | 1 |
| `uqm::ui::outfit::DoOutfit` | `src/ui/outfit.rs` | 11 |
| `uqm::ui::outfit::change_fuel_quantity` | `src/ui/outfit.rs` | 6 |
| `uqm::ui::outfit::display_landers` | `src/ui/outfit.rs` | 2 |
| `uqm::ui::outfit::do_install_module` | `src/ui/outfit.rs` | 10 |
| `uqm::ui::outfit::draw_module_strings` | `src/ui/outfit.rs` | 3 |
| `uqm::ui::outfit::game_string` | `src/ui/outfit.rs` | 1 |
| `uqm::ui::outfit::redistribute_fuel` | `src/ui/outfit.rs` | 7 |
| `uqm::ui::outfit::slot_piece` | `src/ui/outfit.rs` | 4 |
| `uqm::ui::settings::PlayMusic` | `src/ui/settings.rs` | 1 |
| `uqm::ui::settings::PlaySoundEffect` | `src/ui/settings.rs` | 2 |
| `uqm::ui::settings::toggle_music` | `src/ui/settings.rs` | 2 |
| `uqm::ui::settings::toggle_sound_effect` | `src/ui/settings.rs` | 1 |
| `uqm::ui::shipstat::DrawCrewFuelString` | `src/ui/shipstat.rs` | 2 |
| `uqm::ui::shipstat::draw_ship_name_string` | `src/ui/shipstat.rs` | 1 |
| `uqm::ui::shipstat::draw_status_title` | `src/ui/shipstat.rs` | 2 |
| `uqm::ui::shipstat::init_ship_status` | `src/ui/shipstat.rs` | 2 |
| `uqm::ui::shipyard::DoShipyard` | `src/ui/shipyard.rs` | 4 |
| `uqm::ui::shipyard::crew_transaction` | `src/ui/shipyard.rs` | 4 |
| `uqm::ui::shipyard::dms_dismiss_escort_ship_crew` | `src/ui/shipyard.rs` | 1 |
| `uqm::ui::shipyard::dms_dismiss_flag_ship_crew` | `src/ui/shipyard.rs` | 1 |
| `uqm::ui::shipyard::dms_flash_escort_ship_crew_count` | `src/ui/shipyard.rs` | 1 |
| `uqm::ui::shipyard::dms_flash_flag_ship_crew_count` | `src/ui/shipyard.rs` | 2 |
| `uqm::ui::shipyard::dms_hire_escort_ship_crew` | `src/ui/shipyard.rs` | 1 |
| `uqm::ui::shipyard::dms_hire_flag_ship_crew` | `src/ui/shipyard.rs` | 3 |
| `uqm::ui::shipyard::dms_scrap_escort_ship` | `src/ui/shipyard.rs` | 2 |
| `uqm::ui::shipyard::dms_spin_ship` | `src/ui/shipyard.rs` | 2 |
| `uqm::ui::shipyard::dms_try_add_escort_ship` | `src/ui/shipyard.rs` | 1 |
| `uqm::ui::shipyard::do_modify_ships` | `src/ui/shipyard.rs` | 2 |
| `uqm::ui::shipyard::draw_blue_print` | `src/ui/shipyard.rs` | 14 |
| `uqm::ui::shipyard::draw_race_strings` | `src/ui/shipyard.rs` | 3 |
| `uqm::ui::shipyard::get_available_race_count` | `src/ui/shipyard.rs` | 1 |
| `uqm::ui::shipyard::get_available_race_from_index` | `src/ui/shipyard.rs` | 1 |
| `uqm::ui::shipyard::on_input_frame` | `src/ui/shipyard.rs` | 1 |
| `uqm::ui::shipyard::purchased_crew` | `src/ui/shipyard.rs` | 2 |
| `uqm::ui::shipyard::show_combat_ship` | `src/ui/shipyard.rs` | 2 |
| `uqm::ui::shipyard::show_ship_crew` | `src/ui/shipyard.rs` | 1 |
| `uqm::ui::sis::ClearSISRect` | `src/ui/sis.rs` | 1 |
| `uqm::ui::sis::DeltaSISGauges` | `src/ui/sis.rs` | 4 |
| `uqm::ui::sis::DrawAutoPilotMessage` | `src/ui/sis.rs` | 6 |
| `uqm::ui::sis::DrawFlagshipName` | `src/ui/sis.rs` | 6 |
| `uqm::ui::sis::DrawSISMessageEx` | `src/ui/sis.rs` | 6 |
| `uqm::ui::sis::DrawSISTitle` | `src/ui/sis.rs` | 3 |
| `uqm::ui::sis::DrawStatusMessage` | `src/ui/sis.rs` | 6 |
| `uqm::ui::sis::DrawStorageBays` | `src/ui/sis.rs` | 2 |
| `uqm::ui::sis::SetFlashRect` | `src/ui/sis.rs` | 2 |
| `uqm::ui::sis::capacity::CountSISPieces` | `src/ui/sis/capacity.rs` | 1 |
| `uqm::ui::sis::capacity::GetCPodCapacity` | `src/ui/sis/capacity.rs` | 1 |
| `uqm::ui::sis::capacity::GetCrewCount` | `src/ui/sis/capacity.rs` | 1 |
| `uqm::ui::sis::capacity::GetCrewPodCapacity` | `src/ui/sis/capacity.rs` | 1 |
| `uqm::ui::sis::capacity::GetFTankCapacity` | `src/ui/sis/capacity.rs` | 2 |
| `uqm::ui::sis::capacity::GetFuelTankCapacity` | `src/ui/sis/capacity.rs` | 1 |
| `uqm::ui::sis::capacity::GetSBayCapacity` | `src/ui/sis/capacity.rs` | 1 |
| `uqm::ui::sis::capacity::GetStorageBayCapacity` | `src/ui/sis/capacity.rs` | 1 |
| `uqm::ui::sis::capacity::element_mass` | `src/ui/sis/capacity.rs` | 1 |
| `uqm::ui::sis::capacity::fuel_slot_for` | `src/ui/sis/capacity.rs` | 1 |
| `uqm::ui::sis::delta_crew` | `src/ui/sis.rs` | 2 |
| `uqm::ui::sis::delta_fuel` | `src/ui/sis.rs` | 1 |
| `uqm::ui::sis::delta_res` | `src/ui/sis.rs` | 1 |
| `uqm::ui::sis::draw_captains_name` | `src/ui/sis.rs` | 3 |
| `uqm::ui::sis::draw_flagship_stats` | `src/ui/sis.rs` | 5 |
| `uqm::ui::sis::draw_landers` | `src/ui/sis.rs` | 3 |
| `uqm::ui::sis::draw_pc_sis` | `src/ui/sis.rs` | 3 |
| `uqm::ui::sis::draw_slots` | `src/ui/sis.rs` | 1 |
| `uqm::ui::sis::draw_support_ships` | `src/ui/sis.rs` | 1 |
| `uqm::ui::sis::game_state` | `src/ui/sis.rs` | 1 |
| `uqm::ui::sis::game_string` | `src/ui/sis.rs` | 1 |
| `uqm::ui::sis::repair_sisborder` | `src/ui/sis.rs` | 1 |
| `uqm::ui::sis::sis` | `src/ui/sis.rs` | 1 |
| `uqm::ui::starbase::DoStarBase` | `src/ui/starbase.rs` | 4 |
| `uqm::ui::starbase::DrawShipPiece` | `src/ui/starbase.rs` | 2 |
| `uqm::ui::starbase::cleanup_after_star_base` | `src/ui/starbase.rs` | 3 |
| `uqm::ui::starbase::draw_base_state_strings` | `src/ui/starbase.rs` | 2 |
| `uqm::ui::starbase::exit_star_base` | `src/ui/starbase.rs` | 3 |
| `uqm::ui::starbase::game_string` | `src/ui/starbase.rs` | 1 |
| `uqm::ui::starbase::install_bomb_at_earth` | `src/ui/starbase.rs` | 4 |
| `uqm::ui::starbase::visit_star_base` | `src/ui/starbase.rs` | 17 |
| `uqm::ui::util::live::PauseGame` | `src/ui/util.rs` | 8 |
| `uqm::ui::util::live::wait_until` | `src/ui/util.rs` | 1 |

## `u6_04_world_projection`

Work-items.md Change/Deletes: 1967, 1971–1974.

| Caller identity | Source | Ceiling |
| --- | --- | ---: |
| `uqm::comm::glue::resolve_phrase` | `src/comm/glue.rs` | 1 |
| `uqm::comm::speech_graphics::<SpeechGraphics>::update` | `src/comm/speech_graphics.rs` | 2 |
| `uqm::comm::speech_graphics::activity_frame` | `src/comm/speech_graphics.rs` | 1 |
| `uqm::front_end::cnctdlg::do_melee_connect_dialog` | `src/front_end/cnctdlg.rs` | 1 |
| `uqm::front_end::cnctdlg::game_string` | `src/front_end/cnctdlg.rs` | 1 |
| `uqm::front_end::cnctdlg::mcd_draw_button` | `src/front_end/cnctdlg.rs` | 1 |
| `uqm::front_end::credits::runtime::Credits` | `src/front_end/credits.rs` | 3 |
| `uqm::front_end::credits::runtime::credits_input` | `src/front_end/credits.rs` | 1 |
| `uqm::front_end::credits::runtime::outtakes` | `src/front_end/credits.rs` | 1 |
| `uqm::front_end::fmv::runtime::SplashScreen` | `src/front_end/fmv.rs` | 4 |
| `uqm::front_end::gameopt::runtime::ConfirmSaveLoad` | `src/front_end/gameopt.rs` | 1 |
| `uqm::front_end::gameopt::runtime::GameOptions` | `src/front_end/gameopt.rs` | 3 |
| `uqm::front_end::gameopt::runtime::do_game_options` | `src/front_end/gameopt.rs` | 1 |
| `uqm::front_end::gameopt::runtime::do_pick_game` | `src/front_end/gameopt.rs` | 2 |
| `uqm::front_end::gameopt::runtime::do_settings` | `src/front_end/gameopt.rs` | 6 |
| `uqm::front_end::gameopt::runtime::draw_game_selection` | `src/front_end/gameopt.rs` | 1 |
| `uqm::front_end::gameopt::runtime::draw_name_string` | `src/front_end/gameopt.rs` | 3 |
| `uqm::front_end::gameopt::runtime::draw_save_name_string` | `src/front_end/gameopt.rs` | 2 |
| `uqm::front_end::gameopt::runtime::draw_savegame_cargo` | `src/front_end/gameopt.rs` | 3 |
| `uqm::front_end::gameopt::runtime::draw_savegame_summary` | `src/front_end/gameopt.rs` | 14 |
| `uqm::front_end::gameopt::runtime::feedback_setting` | `src/front_end/gameopt.rs` | 1 |
| `uqm::front_end::gameopt::runtime::game_string` | `src/front_end/gameopt.rs` | 1 |
| `uqm::front_end::gameopt::runtime::name_captain_or_ship` | `src/front_end/gameopt.rs` | 2 |
| `uqm::front_end::gameopt::runtime::pick_game` | `src/front_end/gameopt.rs` | 5 |
| `uqm::front_end::intro::runtime::do_presentation` | `src/front_end/intro.rs` | 1 |
| `uqm::front_end::intro::runtime::fade_clear_screen` | `src/front_end/intro.rs` | 1 |
| `uqm::front_end::intro::runtime::generate_sis` | `src/front_end/intro.rs` | 2 |
| `uqm::front_end::intro::runtime::show_slides` | `src/front_end/intro.rs` | 1 |
| `uqm::front_end::intro::runtime::video_input` | `src/front_end/intro.rs` | 1 |
| `uqm::front_end::setupmenu::runtime::SetupMenu` | `src/front_end/setupmenu.rs` | 1 |
| `uqm::front_end::setupmenu::runtime::do_setup_menu` | `src/front_end/setupmenu.rs` | 2 |
| `uqm::front_end::uqmdebug::runtime::activate_all_ships` | `src/front_end/uqmdebug.rs` | 1 |
| `uqm::front_end::uqmdebug::runtime::calculate_mineral_value` | `src/front_end/uqmdebug.rs` | 1 |
| `uqm::front_end::uqmdebug::runtime::clear_escorts` | `src/front_end/uqmdebug.rs` | 1 |
| `uqm::front_end::uqmdebug::runtime::do_instant_move` | `src/front_end/uqmdebug.rs` | 5 |
| `uqm::front_end::uqmdebug::runtime::draw_context` | `src/front_end/uqmdebug.rs` | 1 |
| `uqm::front_end::uqmdebug::runtime::dump_events` | `src/front_end/uqmdebug.rs` | 1 |
| `uqm::front_end::uqmdebug::runtime::dump_planet` | `src/front_end/uqmdebug.rs` | 1 |
| `uqm::front_end::uqmdebug::runtime::dump_strings` | `src/front_end/uqmdebug.rs` | 1 |
| `uqm::front_end::uqmdebug::runtime::equip_ship` | `src/front_end/uqmdebug.rs` | 4 |
| `uqm::front_end::uqmdebug::runtime::for_all_planets` | `src/front_end/uqmdebug.rs` | 1 |
| `uqm::front_end::uqmdebug::runtime::forward_to_next_event` | `src/front_end/uqmdebug.rs` | 1 |
| `uqm::front_end::uqmdebug::runtime::game_string` | `src/front_end/uqmdebug.rs` | 1 |
| `uqm::front_end::uqmdebug::runtime::get_screen` | `src/front_end/uqmdebug.rs` | 1 |
| `uqm::front_end::uqmdebug::runtime::give_devices` | `src/front_end/uqmdebug.rs` | 1 |
| `uqm::front_end::uqmdebug::runtime::is_context_visible` | `src/front_end/uqmdebug.rs` | 1 |
| `uqm::front_end::uqmdebug::runtime::moon_recurse` | `src/front_end/uqmdebug.rs` | 1 |
| `uqm::front_end::uqmdebug::runtime::planet_recurse` | `src/front_end/uqmdebug.rs` | 1 |
| `uqm::front_end::uqmdebug::runtime::put_screen` | `src/front_end/uqmdebug.rs` | 1 |
| `uqm::front_end::uqmdebug::runtime::reset_crew_battle` | `src/front_end/uqmdebug.rs` | 2 |
| `uqm::front_end::uqmdebug::runtime::reset_energy_battle` | `src/front_end/uqmdebug.rs` | 2 |
| `uqm::front_end::uqmdebug::runtime::show_spheres` | `src/front_end/uqmdebug.rs` | 1 |
| `uqm::front_end::uqmdebug::runtime::star_recurse` | `src/front_end/uqmdebug.rs` | 3 |
| `uqm::front_end::uqmdebug::runtime::universe_recurse` | `src/front_end/uqmdebug.rs` | 2 |
| `uqm::mainloop::restart_menu::menu_abi::uqm_get_utwig_bomb` | `src/mainloop/restart_menu/menu_abi.rs` | 1 |
| `uqm::mainloop::restart_menu::menu_abi::uqm_get_utwig_bomb_on_ship` | `src/mainloop/restart_menu/menu_abi.rs` | 1 |
| `uqm::mainloop::restart_menu::menu_abi::uqm_set_utwig_bomb_on_ship` | `src/mainloop/restart_menu/menu_abi.rs` | 1 |
| `uqm::planets::cargo::live::ShowRemainingCapacity` | `src/planets/cargo.rs` | 1 |
| `uqm::planets::cargo::live::discard` | `src/planets/cargo.rs` | 3 |
| `uqm::planets::cargo::live::display` | `src/planets/cargo.rs` | 1 |
| `uqm::planets::cargo::live::element_amount` | `src/planets/cargo.rs` | 3 |
| `uqm::planets::report::live::make_report` | `src/planets/report.rs` | 1 |
| `uqm::planets::sphere::live::tint_frame` | `src/planets/sphere.rs` | 1 |

