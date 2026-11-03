# U4-01 owner-projection retirement table

Measured with `cargo xtask quality report owner_projections --details`. These are exact caller scopes for the approval request on #839. They do not grant an allowance for unsafe, lint, LOC or layer growth.

The 18 previously unassigned calls in navigation events and its string-key helpers have been converted to explicit World arguments. Legacy callers now acquire an owner at their own measured boundary rather than hiding that acquisition in the helpers.

Battle and ship scopes follow the latest #839 orchestrator assignment. Both budgets end at phase 2. Other references below are line numbers in work-items.md. U6-04 assignments cite the front-end/Game conversion and deletion of the remaining bridge, not a battle or ship exemption.

| Retirement | Budget | Calls | Scopes | Work-item Change/Deletes |
| --- | --- | ---: | ---: | --- |
| U2-10a | `u2_10a_projection` | 75 | 35 | #837; #839 orchestrator scope decision |
| U2-10b | `u2_10b_projection` | 15 | 8 | #838; #839 orchestrator scope decision |
| U3-02 | `u3_02_world_projection` | 91 | 42 | 1379, 1383 |
| U4-02 | `u4_02_world_projection` | 45 | 14 | 1479–1481 |
| U4-03 | `u4_03_world_projection` | 35 | 25 | 1530–1532 |
| U4-04 | `u4_04_world_projection` | 69 | 57 | 1576–1578 |
| U5-01 | `u5_01_world_projection` | 4 | 4 | 1631–1632 |
| U5-02 | `u5_02_world_projection` | 273 | 106 | 1680–1682 |
| U5-03 | `u5_03_world_projection` | 20 | 14 | 1730–1731 |
| U5-04 | `u5_04_world_projection` | 44 | 8 | 1775 |
| U6-01 | `u6_01_world_projection` | 64 | 35 | 1824–1827 |
| U6-02 | `u6_02_world_projection` | 44 | 35 | 1875–1876 |
| U6-03 | `u6_03_world_projection` | 281 | 107 | 1925 |
| U6-04 | `u6_04_world_projection` | 129 | 54 | 1971–1974 |
| Total | | 1189 | 544 | |

Each row below is a `ffi|owner_projections` identity with the exact proposed ceiling.

## U2-10a: `u2_10a_projection`

Work-items.md Change/Deletes: #837; #839 orchestrator scope decision.

| Caller identity | Ceiling |
| --- | ---: |
| `uqm::battle::battle_native::engine::Battle` | 3 |
| `uqm::battle::battle_native::engine::BattleSong` | 5 |
| `uqm::battle::battle_native::engine::FreeBattleSong` | 2 |
| `uqm::battle::battle_native::engine::activity` | 1 |
| `uqm::battle::battle_native::engine::game_state` | 1 |
| `uqm::battle::battle_native::engine::player_control` | 1 |
| `uqm::battle::battle_native::engine::player_input` | 1 |
| `uqm::battle::battle_native::engine::set_activity` | 1 |
| `uqm::battle::battle_native::engine::set_game_state` | 1 |
| `uqm::battle::battle_native::engine::set_player_control` | 1 |
| `uqm::battle::battle_native::engine::set_player_input` | 1 |
| `uqm::battle::build_native::engine::AddEscortShips` | 1 |
| `uqm::battle::build_native::engine::CalculateEscortsWorth` | 1 |
| `uqm::battle::build_native::engine::CheckAlliance` | 1 |
| `uqm::battle::build_native::engine::CountEscortShips` | 1 |
| `uqm::battle::build_native::engine::EscortFeasibilityStudy` | 1 |
| `uqm::battle::build_native::engine::GetEscortByStarShipIndex` | 1 |
| `uqm::battle::build_native::engine::RemoveSomeEscortShips` | 1 |
| `uqm::battle::build_native::engine::SetEscortCrewComplement` | 1 |
| `uqm::battle::encount_native::engine::BuildBattle` | 4 |
| `uqm::battle::encount_native::engine::EncounterBattle` | 9 |
| `uqm::battle::encount_native::engine::GetShipFragQueueForPlayer` | 1 |
| `uqm::battle::encount_native::engine::InitEncounter` | 7 |
| `uqm::battle::encount_native::engine::UninitEncounter` | 14 |
| `uqm::battle::encount_native::engine::game_string` | 1 |
| `uqm::battle::encount_native::engine::planet_name` | 1 |
| `uqm::battle::galaxy::engine::blocks` | 1 |
| `uqm::battle::galaxy::engine::stars` | 1 |
| `uqm::battle::ship_native::engine::GetInitialStarShips` | 1 |
| `uqm::battle::sounds::engine::PlayMenuSound` | 1 |
| `uqm::battle::sounds::engine::remove_sounds` | 1 |
| `uqm::battle::status_native::engine::DrawBattleCrewAmount` | 1 |
| `uqm::battle::tactrans_native::engine::ResetWinnerStarShip` | 1 |
| `uqm::battle::tactrans_native::engine::StopDitty` | 2 |
| `uqm::battle::tactrans_native::engine::ditty_playing` | 3 |

## U2-10b: `u2_10b_projection`

Work-items.md Change/Deletes: #838; #839 orchestrator scope decision.

| Caller identity | Ceiling |
| --- | ---: |
| `uqm::ships::code_resources::descriptor` | 1 |
| `uqm::ships::master_native::cost` | 1 |
| `uqm::ships::master_native::engine::FreeMasterShipList` | 1 |
| `uqm::ships::master_native::engine::LoadMasterShipList` | 1 |
| `uqm::ships::master_native::engine::with_catalog` | 1 |
| `uqm::ships::native_adapters::live::rust_bridge_init_battle_arena` | 8 |
| `uqm::ships::native_adapters::live::uqm_get_current_activity_lobyte` | 1 |
| `uqm::ships::native_adapters::uninit_ships` | 1 |

## U3-02: `u3_02_world_projection`

Work-items.md Change/Deletes: 1379, 1383.

| Caller identity | Ceiling |
| --- | ---: |
| `uqm::pickship::ffi::change_selection` | 2 |
| `uqm::pickship::ffi::draw_armada_pick_ship` | 4 |
| `uqm::pickship::ffi::encounter_ship` | 2 |
| `uqm::pickship::ffi::get_armada_star_ship` | 1 |
| `uqm::supermelee::native::battle_picker::live::BuildPickMeleeFrame` | 1 |
| `uqm::supermelee::native::battle_picker::live::FillPickMeleeFrame` | 2 |
| `uqm::supermelee::native::battle_picker::live::MeleeGameOver` | 3 |
| `uqm::supermelee::native::battle_picker::live::MeleeShipDeath` | 2 |
| `uqm::supermelee::native::battle_picker::live::choose` | 2 |
| `uqm::supermelee::native::battle_picker::live::draw` | 1 |
| `uqm::supermelee::native::battle_picker::live::get_ships` | 4 |
| `uqm::supermelee::native::battle_picker::live::value` | 2 |
| `uqm::supermelee::native::build_picker::live::BuildBuildPickFrame` | 1 |
| `uqm::supermelee::native::build_picker::live::pick` | 1 |
| `uqm::supermelee::native::load_teams::live::DoLoadTeam` | 1 |
| `uqm::supermelee::native::load_teams::live::DoSaveTeam` | 1 |
| `uqm::supermelee::native::load_teams::live::draw` | 2 |
| `uqm::supermelee::native::load_teams::live::flash` | 1 |
| `uqm::supermelee::native::load_teams::live::select` | 2 |
| `uqm::supermelee::native::menu::live::DoMelee` | 5 |
| `uqm::supermelee::native::menu::live::Melee` | 10 |
| `uqm::supermelee::native::menu::live::battle::confirm` | 2 |
| `uqm::supermelee::native::menu::live::battle::connecting` | 3 |
| `uqm::supermelee::native::menu::live::battle::disconnects` | 2 |
| `uqm::supermelee::native::menu::live::battle::next_control` | 2 |
| `uqm::supermelee::native::menu::live::battle::ready` | 1 |
| `uqm::supermelee::native::menu::live::battle::start` | 5 |
| `uqm::supermelee::native::menu::live::battle::start_button` | 7 |
| `uqm::supermelee::native::menu::live::boundary::set_player_input_all` | 1 |
| `uqm::supermelee::native::menu::live::changes::resetFeedback` | 1 |
| `uqm::supermelee::native::menu::live::drawing::DrawMeleeShipStrings` | 2 |
| `uqm::supermelee::native::menu::live::drawing::Melee_flashSelection` | 1 |
| `uqm::supermelee::native::menu::live::drawing::RepairMeleeFrame` | 1 |
| `uqm::supermelee::native::menu::live::drawing::controls` | 1 |
| `uqm::supermelee::native::menu::live::drawing::init_screen` | 2 |
| `uqm::supermelee::native::menu::live::drawing::name` | 1 |
| `uqm::supermelee::native::menu::live::drawing::status` | 2 |
| `uqm::supermelee::native::menu::live::drawing::worth` | 1 |
| `uqm::supermelee::native::menu::live::editing::edit` | 1 |
| `uqm::supermelee::native::menu::live::load_config` | 3 |
| `uqm::supermelee::native::menu::live::select_option` | 1 |
| `uqm::supermelee::native::menu::live::write_config` | 1 |

## U4-02: `u4_02_world_projection`

Work-items.md Change/Deletes: 1479–1481.

| Caller identity | Ceiling |
| --- | ---: |
| `uqm::navigation::groups::live::BuildGroups` | 5 |
| `uqm::navigation::groups::live::GetGroupInfo` | 19 |
| `uqm::navigation::groups::live::PutGroupInfo` | 5 |
| `uqm::navigation::groups::live::fleet_queue` | 1 |
| `uqm::navigation::groups::live::flush` | 6 |
| `uqm::navigation::groups::live::ip_queue` | 1 |
| `uqm::navigation::groups::live::npc_queue` | 1 |
| `uqm::navigation::groups::live::open` | 1 |
| `uqm::navigation::groups::live::seek` | 1 |
| `uqm::navigation::groups::read` | 1 |
| `uqm::navigation::groups::write` | 1 |
| `uqm::time::clock_abi::live::AddEvent` | 1 |
| `uqm::time::clock_abi::live::GetGameClock` | 1 |
| `uqm::time::clock_abi::live::finish_clock_day` | 1 |

## U4-03: `u4_03_world_projection`

Work-items.md Change/Deletes: 1530–1532.

| Caller identity | Ceiling |
| --- | ---: |
| `uqm::automation::campaign_state::<LiveCampaign as StateSource>::read` | 1 |
| `uqm::automation::coordinator::<Coordinator>::service_energy` | 2 |
| `uqm::automation::coordinator::<Coordinator>::service_navigation` | 2 |
| `uqm::automation::coordinator::<Coordinator>::service_patrol_contact` | 1 |
| `uqm::automation::coordinator::terminal::<Coordinator>::set_terminal` | 1 |
| `uqm::automation::input_ffi::contact_race` | 1 |
| `uqm::automation::input_ffi::contact_snapshot` | 1 |
| `uqm::automation::input_ffi::cyborg_settings` | 1 |
| `uqm::automation::input_ffi::escort_roster` | 2 |
| `uqm::automation::input_ffi::game_date` | 1 |
| `uqm::automation::input_ffi::hyperspace_position` | 1 |
| `uqm::automation::input_ffi::hyperspace_stopped` | 1 |
| `uqm::automation::input_ffi::read_flight_state` | 1 |
| `uqm::automation::input_ffi::service_input` | 1 |
| `uqm::automation::input_ffi::ship_element_amounts` | 1 |
| `uqm::automation::input_ffi::ship_resources` | 1 |
| `uqm::automation::input_ffi::sis_crew_and_landers` | 1 |
| `uqm::automation::input_ffi::sis_fittings` | 1 |
| `uqm::automation::input_ffi::star_flight_observation` | 4 |
| `uqm::automation::melee_fixture::live::production::apply` | 1 |
| `uqm::automation::melee_fixture::live::production::borrow_world` | 1 |
| `uqm::automation::melee_fixture::live::production::configure` | 2 |
| `uqm::automation::melee_fixture::live::production::execute` | 4 |
| `uqm::automation::scenario::<LinkedSceneEntry as SceneEntryOps>::clear_screen` | 1 |
| `uqm::automation::scenario::activate` | 1 |

## U4-04: `u4_04_world_projection`

Work-items.md Change/Deletes: 1576–1578.

| Caller identity | Ceiling |
| --- | ---: |
| `uqm::engine::battle::access::record::ship_queue` | 1 |
| `uqm::mainloop::ffi::get_chmmr_bomb_state` | 1 |
| `uqm::mainloop::ffi::get_crew_enlisted` | 1 |
| `uqm::mainloop::ffi::get_current_activity` | 1 |
| `uqm::mainloop::ffi::get_global_flags_and_data` | 1 |
| `uqm::mainloop::ffi::get_kohr_ah_killed_all` | 1 |
| `uqm::mainloop::ffi::get_last_activity` | 1 |
| `uqm::mainloop::ffi::get_starbase_available` | 1 |
| `uqm::mainloop::ffi::load_kernel` | 2 |
| `uqm::mainloop::ffi::set_chmmr_bomb_state` | 1 |
| `uqm::mainloop::ffi::set_current_activity` | 1 |
| `uqm::mainloop::ffi::set_last_activity` | 1 |
| `uqm::mainloop::ffi::set_player_input_all_or_explode` | 1 |
| `uqm::mainloop::ffi::zero_global_velocity` | 1 |
| `uqm::mainloop::game_loop::cffi::<CffiOps as GameLoopOps>::add_initial_game_events` | 1 |
| `uqm::mainloop::game_loop::cffi::<CffiOps as GameLoopOps>::clear_player_input_all` | 1 |
| `uqm::mainloop::game_loop::cffi::<CffiOps as GameLoopOps>::free_kernel` | 1 |
| `uqm::mainloop::game_loop::cffi::<CffiOps as GameLoopOps>::get_current_activity` | 1 |
| `uqm::mainloop::game_loop::cffi::<CffiOps as GameLoopOps>::load_kernel` | 2 |
| `uqm::mainloop::input_abi::live::DoInput` | 1 |
| `uqm::mainloop::lifecycle_abi::SignalStopMainThread` | 1 |
| `uqm::mainloop::lifecycle_abi::background_init_kernel` | 3 |
| `uqm::mainloop::lifecycle_abi::on_battle_frame` | 2 |
| `uqm::mainloop::restart_menu::menu_abi::uqm_do_popup_window_msg` | 1 |
| `uqm::mainloop::restart_menu::menu_abi::uqm_set_player_control` | 1 |
| `uqm::mainloop::restart_menu::render_abi::DrawRestartMenuGraphic` | 1 |
| `uqm::mainloop::restart_menu::restart_ops::<CffiOps as RestartMenuOps>::create_flash_overlay` | 1 |
| `uqm::mainloop::restart_menu::restart_ops::<CffiOps as RestartMenuOps>::get_current_activity` | 1 |
| `uqm::mainloop::restart_menu::restart_ops::<CffiOps as RestartMenuOps>::set_screen_context` | 1 |
| `uqm::mainloop::state_abi::get_last_activity` | 1 |
| `uqm::mainloop::state_abi::rust_get_avail_race_queue` | 1 |
| `uqm::mainloop::state_abi::rust_get_built_ship_queue` | 1 |
| `uqm::mainloop::state_abi::rust_get_encounter_queue` | 1 |
| `uqm::mainloop::state_abi::rust_get_ip_group_queue` | 1 |
| `uqm::mainloop::state_abi::rust_get_npc_built_ship_queue` | 1 |
| `uqm::mainloop::state_abi::rust_set_cur_star_desc_ptr_null` | 1 |
| `uqm::mainloop::state_abi::set_current_activity` | 1 |
| `uqm::mainloop::state_abi::set_last_activity` | 1 |
| `uqm::mainloop::state_abi::uqm_get_chmmr_bomb_state` | 1 |
| `uqm::mainloop::state_abi::uqm_get_crew_enlisted` | 1 |
| `uqm::mainloop::state_abi::uqm_get_global_flags_and_data` | 1 |
| `uqm::mainloop::state_abi::uqm_get_kohr_ah_killed_all` | 1 |
| `uqm::mainloop::state_abi::uqm_get_starbase_available` | 1 |
| `uqm::mainloop::state_abi::uqm_set_chmmr_bomb_state` | 1 |
| `uqm::mainloop::state_abi::uqm_set_player_input_all_or_explode` | 1 |
| `uqm::mainloop::state_abi::uqm_zero_global_velocity` | 1 |
| `uqm::savegame::legacy::live::LoadLegacyGame` | 1 |
| `uqm::savegame::live::load::finish_load` | 2 |
| `uqm::savegame::live::load::install` | 1 |
| `uqm::savegame::live::save::capture` | 4 |
| `uqm::savegame::runtime::live::capture_live_star` | 1 |
| `uqm::savegame::runtime::live::capture_live_summary` | 2 |
| `uqm::savegame::runtime::live::prepare_live_hyper_state` | 1 |
| `uqm::savegame::runtime::live::prepare_live_solar_location` | 3 |
| `uqm::savegame::runtime::location::persist_planet_change` | 1 |
| `uqm::savegame::runtime::statefiles::init_group_info` | 1 |
| `uqm::savegame::runtime::summary::capture_summary` | 1 |

## U5-01: `u5_01_world_projection`

Work-items.md Change/Deletes: 1631–1632.

| Caller identity | Ceiling |
| --- | ---: |
| `uqm::planets::generate::special::<LiveStory as Story>::flag` | 1 |
| `uqm::planets::generate::special::<LiveStory as Story>::set_flag` | 1 |
| `uqm::planets::generate::special::get_game_state_byte` | 1 |
| `uqm::planets::generate::special::set_game_state_byte` | 1 |

## U5-02: `u5_02_world_projection`

Work-items.md Change/Deletes: 1680–1682.

| Caller identity | Ceiling |
| --- | ---: |
| `uqm::planets::c_api::<LiveGame as FleetGroups>::battle_group_ref` | 1 |
| `uqm::planets::c_api::<LiveGame as FleetGroups>::head_ip_group` | 2 |
| `uqm::planets::c_api::<LiveGame as FleetGroups>::set_battle_group_ref` | 1 |
| `uqm::planets::c_api::<LiveGame as Npcs>::clear_battle_group_ref` | 1 |
| `uqm::planets::c_api::<LiveGame as Npcs>::for_each_ip_group` | 1 |
| `uqm::planets::c_api::<LiveGame as Npcs>::init_ip_groups` | 1 |
| `uqm::planets::c_api::<LiveGame as Npcs>::reinit_ip_group_q` | 1 |
| `uqm::planets::c_api::<LiveGame as Npcs>::reinit_npc_built_ship_q` | 1 |
| `uqm::planets::c_api::<LiveGame as Npcs>::yehat_civil_war` | 1 |
| `uqm::planets::c_api::<LiveGame as PlanetNames>::set_battle_planet` | 1 |
| `uqm::planets::c_api::<LiveGame as PlanetNames>::set_planet_name` | 1 |
| `uqm::planets::c_api::<LiveGame as crate::planets::generate::sam::GuardSite>::for_each_ip_group` | 1 |
| `uqm::planets::c_api::<LiveGame as crate::planets::generate::sam::GuardSite>::ip_location` | 1 |
| `uqm::planets::c_api::<LiveGame as crate::planets::generate::sam::GuardSite>::set_and_transform_ip_location` | 1 |
| `uqm::planets::c_api::begin_urquan_encounter` | 4 |
| `uqm::planets::c_api::built_ship_count` | 1 |
| `uqm::planets::c_api::built_ship_queue_has_ships` | 1 |
| `uqm::planets::c_api::clone_npc_ship` | 1 |
| `uqm::planets::c_api::current_star_index` | 1 |
| `uqm::planets::c_api::current_star_point` | 1 |
| `uqm::planets::c_api::fuel_on_board` | 1 |
| `uqm::planets::c_api::glob` | 1 |
| `uqm::planets::c_api::star_indexes_before_current` | 1 |
| `uqm::planets::c_api::start_sphere_tracking` | 1 |
| `uqm::planets::c_api::take_encounter` | 1 |
| `uqm::planets::cargo::live::ShowRemainingCapacity` | 1 |
| `uqm::planets::cargo::live::discard` | 3 |
| `uqm::planets::cargo::live::display` | 1 |
| `uqm::planets::cargo::live::element_amount` | 3 |
| `uqm::planets::devices::DevicesMenu` | 5 |
| `uqm::planets::devices::InventoryDevices` | 1 |
| `uqm::planets::devices::draw` | 1 |
| `uqm::planets::devices::effect_state` | 3 |
| `uqm::planets::devices::frame` | 1 |
| `uqm::planets::devices::invoke` | 27 |
| `uqm::planets::devices::reset_groups` | 1 |
| `uqm::planets::devices::use_caster` | 13 |
| `uqm::planets::ffi::current_star_type` | 1 |
| `uqm::planets::orbit::live::FreePlanet` | 1 |
| `uqm::planets::orbit::live::LoadPlanet` | 1 |
| `uqm::planets::orbit::live::create_context` | 2 |
| `uqm::planets::orbit::live::do_orbit` | 6 |
| `uqm::planets::orbit::live::draw_display` | 1 |
| `uqm::planets::pstarmap::DrawStarMap` | 3 |
| `uqm::planets::pstarmap::StarMap` | 8 |
| `uqm::planets::pstarmap::UpdateCursorInfo` | 2 |
| `uqm::planets::pstarmap::UpdateCursorLocation` | 2 |
| `uqm::planets::pstarmap::current_location` | 4 |
| `uqm::planets::pstarmap::flashCurrentLocation` | 1 |
| `uqm::planets::pstarmap::fleet_queue` | 1 |
| `uqm::planets::pstarmap::game_string` | 1 |
| `uqm::planets::pstarmap::portal_open` | 1 |
| `uqm::planets::pstarmap::sphere_text` | 1 |
| `uqm::planets::pstarmap::universeToDispx` | 1 |
| `uqm::planets::pstarmap::universeToDispy` | 1 |
| `uqm::planets::pstarmap::which_space` | 1 |
| `uqm::planets::report::live::make_report` | 1 |
| `uqm::planets::roster::live::RosterMenu` | 1 |
| `uqm::planets::roster::live::delta_crew` | 1 |
| `uqm::planets::roster::live::modify` | 3 |
| `uqm::planets::roster::live::support_ship` | 1 |
| `uqm::planets::scan::live::GetScanContext` | 2 |
| `uqm::planets::scan::live::cursor_frame` | 1 |
| `uqm::planets::scan::live::dispatch_lander` | 5 |
| `uqm::planets::scan::live::display::pc` | 2 |
| `uqm::planets::scan::live::display::three_do` | 1 |
| `uqm::planets::scan::live::pick_input` | 1 |
| `uqm::planets::scan::live::scan_input` | 4 |
| `uqm::planets::scan::live::scan_planet` | 1 |
| `uqm::planets::scan::live::spawn_fwiffo` | 1 |
| `uqm::planets::solarsys::drawing::create_star_background` | 3 |
| `uqm::planets::solarsys::drawing::draw_inner_system` | 1 |
| `uqm::planets::solarsys::drawing::draw_outer_system` | 1 |
| `uqm::planets::solarsys::drawing::draw_system` | 2 |
| `uqm::planets::solarsys::flight::check_intersect` | 1 |
| `uqm::planets::solarsys::flight::check_ship_location` | 2 |
| `uqm::planets::solarsys::flight::check_zoom_level` | 1 |
| `uqm::planets::solarsys::flight::enter_inner_system` | 1 |
| `uqm::planets::solarsys::flight::enter_orbital` | 1 |
| `uqm::planets::solarsys::flight::get_collision_frame` | 1 |
| `uqm::planets::solarsys::flight::glob` | 1 |
| `uqm::planets::solarsys::flight::ip_frame` | 1 |
| `uqm::planets::solarsys::flight::leave_inner_system` | 1 |
| `uqm::planets::solarsys::flight::process_ship_controls` | 1 |
| `uqm::planets::solarsys::flight::scale_system` | 1 |
| `uqm::planets::solarsys::lifecycle::ExploreSolarSys` | 7 |
| `uqm::planets::solarsys::lifecycle::GetNamedPlanetaryBody` | 2 |
| `uqm::planets::solarsys::lifecycle::SaveSolarSysLocation` | 5 |
| `uqm::planets::solarsys::lifecycle::do_ip_flight` | 3 |
| `uqm::planets::solarsys::lifecycle::do_solar_sys_menu` | 7 |
| `uqm::planets::solarsys::lifecycle::end_interplanetary` | 3 |
| `uqm::planets::solarsys::lifecycle::enter_planet_orbit` | 11 |
| `uqm::planets::solarsys::lifecycle::free_solar_sys` | 1 |
| `uqm::planets::solarsys::lifecycle::game_string` | 1 |
| `uqm::planets::solarsys::lifecycle::glob` | 1 |
| `uqm::planets::solarsys::lifecycle::init_solar_sys` | 16 |
| `uqm::planets::solarsys::lifecycle::init_solar_sys_sis_characteristics` | 2 |
| `uqm::planets::solarsys::lifecycle::load_solar_sys` | 18 |
| `uqm::planets::solarsys::lifecycle::reset_solar_sys` | 1 |
| `uqm::planets::solarsys::lifecycle::save_non_orbital_location` | 6 |
| `uqm::planets::solarsys::lifecycle::solar_sys_menu` | 1 |
| `uqm::planets::solarsys::lifecycle::uninit_solar_sys` | 5 |
| `uqm::planets::sphere::live::tint_frame` | 1 |
| `uqm::planets::starmap::FindStar` | 1 |
| `uqm::planets::starmap::naming::GetClusterName` | 2 |
| `uqm::planets::starmap_cursor::DoMoveCursor` | 7 |

## U5-03: `u5_03_world_projection`

Work-items.md Change/Deletes: 1730–1731.

| Caller identity | Ceiling |
| --- | ---: |
| `uqm::planet_side::adapters::<CffiShipStatus as ShipStatusPort>::apply` | 2 |
| `uqm::planet_side::adapters::<CffiShipStatus>::snapshot` | 1 |
| `uqm::planet_side::adapters::read_player_one_input` | 1 |
| `uqm::planet_side::generation_adapter::<CffiSurfaceGenerator as SurfaceGenerator>::can_node` | 1 |
| `uqm::planet_side::generation_adapter::<CffiSurfaceGenerator as SurfaceGenerator>::generate` | 1 |
| `uqm::planet_side::generation_adapter::<CffiSurfaceGenerator as SurfaceGenerator>::pickup` | 2 |
| `uqm::planet_side::generation_adapter::<CffiSurfaceGenerator>::node_key` | 1 |
| `uqm::planet_side::graphics_adapter::<CffiPlanetSideGraphics<'_,A> as PlanetSideGraphics>::render` | 1 |
| `uqm::planet_side::init_lander::InitLander` | 1 |
| `uqm::planet_side::init_lander::draw_lander_status` | 2 |
| `uqm::planet_side::init_lander::sis_num_landers` | 1 |
| `uqm::planet_side::init_lander::sis_total_element_mass` | 1 |
| `uqm::planet_side::report_adapter::<CffiDiscoveryReport as DiscoveryReportPort>::present` | 1 |
| `uqm::planet_side::session_factory::production_upgrades` | 4 |

## U5-04: `u5_04_world_projection`

Work-items.md Change/Deletes: 1775.

| Caller identity | Ceiling |
| --- | ---: |
| `uqm::navigation::hyper_data::FreeHyperData` | 1 |
| `uqm::navigation::hyper_data::FreeHyperspace` | 1 |
| `uqm::navigation::hyper_data::LoadHyperspace` | 13 |
| `uqm::navigation::hyper_data::load` | 6 |
| `uqm::navigation::hyper_menu::HyperspaceMenu` | 4 |
| `uqm::navigation::hyper_menu::input` | 5 |
| `uqm::navigation::hyperspace::live::MoveSIS` | 13 |
| `uqm::navigation::hyperspace::live::rust_reset_hyper_distance` | 1 |

## U6-01: `u6_01_world_projection`

Work-items.md Change/Deletes: 1824–1827.

| Caller identity | Ceiling |
| --- | ---: |
| `uqm::comm::commglue_abi::runtime::GetAllianceName` | 1 |
| `uqm::comm::commglue_abi::runtime::get` | 1 |
| `uqm::comm::commglue_abi::runtime::getSegue` | 3 |
| `uqm::comm::commglue_abi::runtime::set` | 1 |
| `uqm::comm::commglue_abi::runtime::setSegue` | 3 |
| `uqm::comm::dispatch::avail_race_queue` | 1 |
| `uqm::comm::dispatch::built_ship_queue` | 1 |
| `uqm::comm::dispatch::encounter_queue` | 1 |
| `uqm::comm::dispatch::init_communication` | 4 |
| `uqm::comm::dispatch::init_communication_inner` | 8 |
| `uqm::comm::dispatch::npc_built_ship_queue` | 1 |
| `uqm::comm::dispatch::place_flagship_at_star` | 1 |
| `uqm::comm::dispatch::prepare_samatra_dev_fixture` | 1 |
| `uqm::comm::dispatch::rust_init_race_dispatch` | 2 |
| `uqm::comm::dispatch::rust_race_communication` | 6 |
| `uqm::comm::ffi::get_alliance_name` | 1 |
| `uqm::comm::ffi::get_alliance_name_full` | 2 |
| `uqm::comm::ffi::rust_npc_phrase_cb` | 1 |
| `uqm::comm::glue::resolve_phrase` | 1 |
| `uqm::comm::hail::game_string` | 1 |
| `uqm::comm::hail::hail_alien` | 3 |
| `uqm::comm::hail::is_starbase_conversation` | 2 |
| `uqm::comm::hail::planet_name` | 1 |
| `uqm::comm::oscill_abi::runtime::DrawOscilloscope` | 2 |
| `uqm::comm::sis_graphics::<CommWindow as CommWindowGfx>::draw_scroll_indicator` | 1 |
| `uqm::comm::sis_graphics::<CommWindow as CommWindowGfx>::space_context` | 1 |
| `uqm::comm::sis_graphics::draw_summary_page_contents` | 2 |
| `uqm::comm::sis_graphics::game_string` | 1 |
| `uqm::comm::speech_graphics::<SpeechGraphics>::update` | 2 |
| `uqm::comm::speech_graphics::activity_frame` | 1 |
| `uqm::comm::talk_segue::c_bridge::clear_last_activity_load_flag` | 1 |
| `uqm::comm::talk_segue::dinput::do_last_replay_cb` | 2 |
| `uqm::comm::talk_segue::init_speech_graphics` | 1 |
| `uqm::comm::talk_segue::set_slider_image_frame` | 1 |
| `uqm::comm::talk_segue::update_speech_graphics` | 2 |

## U6-02: `u6_02_world_projection`

Work-items.md Change/Deletes: 1875–1876.

| Caller identity | Ceiling |
| --- | ---: |
| `uqm::comm::races::alliance::live::<LiveScript<H> as AllianceApi<H>>::set_allied` | 1 |
| `uqm::comm::races::alliance::live::<LiveScript<H> as AllianceApi<H>>::track` | 1 |
| `uqm::comm::races::blackur::api::<super::super::script::live::LiveScript<Handler> as BlackurApi>::battle_group` | 2 |
| `uqm::comm::races::chmmr::api::<super::super::script::live::LiveScript<Handler> as ChmmrApi>::refit` | 1 |
| `uqm::comm::races::comandr::<ComandrDialogue as super::RaceDialogue>::init` | 1 |
| `uqm::comm::races::druuge::api::<crate::comm::races::script::live::LiveScript<Handler> as DruugeApi>::crew_cost` | 1 |
| `uqm::comm::races::druuge::api::<crate::comm::races::script::live::LiveScript<Handler> as DruugeApi>::has_escorts` | 1 |
| `uqm::comm::races::druuge::api::<crate::comm::races::script::live::LiveScript<Handler> as DruugeApi>::set_crew_cost` | 1 |
| `uqm::comm::races::druuge::api::<crate::comm::races::script::live::LiveScript<Handler> as DruugeApi>::ship` | 1 |
| `uqm::comm::races::melnorm::api::live::<LiveScript<Handler> as MelnormApi>::module_cost` | 1 |
| `uqm::comm::races::melnorm::api::live::<LiveScript<Handler> as MelnormApi>::set_module_cost` | 1 |
| `uqm::comm::races::melnorm::api::live::<LiveScript<Handler> as MelnormApi>::set_ship` | 1 |
| `uqm::comm::races::melnorm::api::live::<LiveScript<Handler> as MelnormApi>::ship` | 1 |
| `uqm::comm::races::pkunk::api::<crate::comm::races::script::live::LiveScript<Handler> as PkunkApi>::date` | 1 |
| `uqm::comm::races::rebel::api::<super::super::script::live::LiveScript<Handler> as RebelApi>::date` | 1 |
| `uqm::comm::races::script::live::<LiveScript<H> as ScriptApi<H>>::add_lander` | 1 |
| `uqm::comm::races::script::live::<LiveScript<H> as ScriptApi<H>>::get` | 1 |
| `uqm::comm::races::script::live::<LiveScript<H> as ScriptApi<H>>::response` | 1 |
| `uqm::comm::races::script::live::<LiveScript<H> as ScriptApi<H>>::set` | 1 |
| `uqm::comm::races::script::live::<LiveScript<H> as ScriptApi<H>>::supplies` | 3 |
| `uqm::comm::races::slyland::api::live::<LiveScript<Handler> as SlylandApi>::position` | 2 |
| `uqm::comm::races::starbas::api::live::<LiveScript<Handler> as StarbaseApi>::date` | 1 |
| `uqm::comm::races::starbas::api::live::<LiveScript<Handler> as StarbaseApi>::mineral_amount` | 1 |
| `uqm::comm::races::starbas::api::live::<LiveScript<Handler> as StarbaseApi>::mineral_worth` | 1 |
| `uqm::comm::races::starbas::api::live::<LiveScript<Handler> as StarbaseApi>::remove_minerals` | 2 |
| `uqm::comm::races::starbas::api::live::<LiveScript<Handler> as StarbaseApi>::ship` | 1 |
| `uqm::comm::races::starbas::api::live::<LiveScript<Handler> as StarbaseApi>::skip_sale` | 1 |
| `uqm::comm::races::starbas::api::live::<LiveScript<Handler> as StarbaseApi>::using_speech` | 1 |
| `uqm::comm::races::syreen::api::<super::super::script::live::LiveScript<Handler> as SyreenApi>::using_speech` | 1 |
| `uqm::comm::races::talkpet::api::<super::super::script::live::LiveScript<Handler> as TalkpetApi>::flags` | 1 |
| `uqm::comm::races::talkpet::api::<super::super::script::live::LiveScript<Handler> as TalkpetApi>::npc_ships` | 1 |
| `uqm::comm::races::talkpet::api::<super::super::script::live::LiveScript<Handler> as TalkpetApi>::umgah_alive` | 1 |
| `uqm::comm::races::umgah::api::<super::super::script::live::LiveScript<Handler> as UmgahApi>::add_biomass` | 2 |
| `uqm::comm::races::urquan::api::<super::super::script::live::LiveScript<Handler> as UrquanApi>::group_battle` | 3 |
| `uqm::comm::races::zoqfot::api::live::<LiveScript<Handler> as ZoqApi>::date` | 2 |

## U6-03: `u6_03_world_projection`

Work-items.md Change/Deletes: 1925.

| Caller identity | Ceiling |
| --- | ---: |
| `uqm::starbase::cffi::<CffiPlatform as Gfx>::game_string` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Gfx>::play_frame` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Gfx>::set_font_effect` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Platform>::crew_cost` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Platform>::game_state` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Platform>::set_crew_cost` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Platform>::set_drive_slot` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Platform>::set_fuel_on_board` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Platform>::set_game_state` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Platform>::set_jet_slot` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Platform>::set_module_slot` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Platform>::set_num_landers` | 1 |
| `uqm::starbase::cffi::<CffiPlatform as Platform>::sis` | 1 |
| `uqm::starbase::cffi::<CffiPlatform>::context_ptr` | 1 |
| `uqm::starbase::cffi::<CffiPlatform>::font_ptr` | 1 |
| `uqm::starbase::cffi::avail_race_queue` | 1 |
| `uqm::starbase::cffi::built_ship_queue` | 1 |
| `uqm::ui::border::DrawSISFrame` | 2 |
| `uqm::ui::confirm::DoConfirmExit` | 4 |
| `uqm::ui::confirm::DoPopupWindow` | 2 |
| `uqm::ui::confirm::do_popup` | 1 |
| `uqm::ui::confirm::draw_confirmation` | 1 |
| `uqm::ui::confirm::game_string` | 1 |
| `uqm::ui::getchar::DoTextEntry` | 1 |
| `uqm::ui::menu::DoMenuChooser` | 1 |
| `uqm::ui::menu::DrawMenuStateStrings` | 5 |
| `uqm::ui::menu::PreviousMenuState` | 1 |
| `uqm::ui::menu::draw_pc_menu` | 3 |
| `uqm::ui::menu::game_string` | 1 |
| `uqm::ui::outfit::ChangeFuelQuantity` | 6 |
| `uqm::ui::outfit::DisplayLanders` | 2 |
| `uqm::ui::outfit::DoInstallModule` | 22 |
| `uqm::ui::outfit::DoOutfit` | 13 |
| `uqm::ui::outfit::DrawModuleStrings` | 3 |
| `uqm::ui::outfit::RedistributeFuel` | 7 |
| `uqm::ui::outfit::SlotPiece` | 4 |
| `uqm::ui::outfit::game_string` | 1 |
| `uqm::ui::settings::PlayMusic` | 1 |
| `uqm::ui::settings::PlaySoundEffect` | 2 |
| `uqm::ui::settings::ToggleMusic` | 2 |
| `uqm::ui::settings::ToggleSoundEffect` | 1 |
| `uqm::ui::shipstat::DrawCrewFuelString` | 2 |
| `uqm::ui::shipstat::draw_ship_name_string` | 1 |
| `uqm::ui::shipstat::draw_status_title` | 2 |
| `uqm::ui::shipstat::init_ship_status` | 2 |
| `uqm::ui::shipyard::CrewTransaction` | 4 |
| `uqm::ui::shipyard::DMS_DismissEscortShipCrew` | 1 |
| `uqm::ui::shipyard::DMS_DismissFlagShipCrew` | 1 |
| `uqm::ui::shipyard::DMS_FlashEscortShipCrewCount` | 1 |
| `uqm::ui::shipyard::DMS_FlashFlagShipCrewCount` | 2 |
| `uqm::ui::shipyard::DMS_HireEscortShipCrew` | 1 |
| `uqm::ui::shipyard::DMS_HireFlagShipCrew` | 3 |
| `uqm::ui::shipyard::DMS_ScrapEscortShip` | 2 |
| `uqm::ui::shipyard::DMS_SpinShip` | 2 |
| `uqm::ui::shipyard::DMS_TryAddEscortShip` | 1 |
| `uqm::ui::shipyard::DoModifyShips` | 2 |
| `uqm::ui::shipyard::DoShipyard` | 6 |
| `uqm::ui::shipyard::DrawBluePrint` | 14 |
| `uqm::ui::shipyard::DrawRaceStrings` | 3 |
| `uqm::ui::shipyard::GetAvailableRaceCount` | 1 |
| `uqm::ui::shipyard::GetAvailableRaceFromIndex` | 1 |
| `uqm::ui::shipyard::ShowCombatShip` | 2 |
| `uqm::ui::shipyard::ShowShipCrew` | 1 |
| `uqm::ui::shipyard::on_input_frame` | 1 |
| `uqm::ui::shipyard::purchased_crew` | 2 |
| `uqm::ui::sis::ClearSISRect` | 1 |
| `uqm::ui::sis::DeltaSISGauges` | 4 |
| `uqm::ui::sis::DrawAutoPilotMessage` | 6 |
| `uqm::ui::sis::DrawCaptainsName` | 3 |
| `uqm::ui::sis::DrawFlagshipName` | 6 |
| `uqm::ui::sis::DrawFlagshipStats` | 5 |
| `uqm::ui::sis::DrawLanders` | 3 |
| `uqm::ui::sis::DrawSISMessageEx` | 6 |
| `uqm::ui::sis::DrawSISTitle` | 3 |
| `uqm::ui::sis::DrawStatusMessage` | 8 |
| `uqm::ui::sis::DrawStorageBays` | 2 |
| `uqm::ui::sis::RepairSISBorder` | 1 |
| `uqm::ui::sis::SetFlashRect` | 2 |
| `uqm::ui::sis::capacity::CountSISPieces` | 1 |
| `uqm::ui::sis::capacity::GetCPodCapacity` | 1 |
| `uqm::ui::sis::capacity::GetCrewCount` | 1 |
| `uqm::ui::sis::capacity::GetCrewPodCapacity` | 1 |
| `uqm::ui::sis::capacity::GetFTankCapacity` | 2 |
| `uqm::ui::sis::capacity::GetFuelTankCapacity` | 1 |
| `uqm::ui::sis::capacity::GetSBayCapacity` | 1 |
| `uqm::ui::sis::capacity::GetStorageBayCapacity` | 1 |
| `uqm::ui::sis::capacity::element_mass` | 1 |
| `uqm::ui::sis::capacity::fuel_slot_for` | 1 |
| `uqm::ui::sis::delta_crew` | 2 |
| `uqm::ui::sis::delta_fuel` | 1 |
| `uqm::ui::sis::delta_res` | 1 |
| `uqm::ui::sis::draw_pc_sis` | 3 |
| `uqm::ui::sis::draw_slots` | 1 |
| `uqm::ui::sis::draw_support_ships` | 1 |
| `uqm::ui::sis::game_state` | 1 |
| `uqm::ui::sis::game_string` | 1 |
| `uqm::ui::sis::sis` | 1 |
| `uqm::ui::starbase::CleanupAfterStarBase` | 3 |
| `uqm::ui::starbase::DoStarBase` | 4 |
| `uqm::ui::starbase::DrawBaseStateStrings` | 2 |
| `uqm::ui::starbase::DrawShipPiece` | 3 |
| `uqm::ui::starbase::ExitStarBase` | 3 |
| `uqm::ui::starbase::InstallBombAtEarth` | 4 |
| `uqm::ui::starbase::VisitStarBase` | 17 |
| `uqm::ui::starbase::game_string` | 1 |
| `uqm::ui::util::live::PauseGame` | 8 |
| `uqm::ui::util::live::wait_until` | 1 |

## U6-04: `u6_04_world_projection`

Work-items.md Change/Deletes: 1971–1974.

| Caller identity | Ceiling |
| --- | ---: |
| `uqm::front_end::cnctdlg::do_melee_connect_dialog` | 1 |
| `uqm::front_end::cnctdlg::game_string` | 1 |
| `uqm::front_end::cnctdlg::mcd_draw_button` | 1 |
| `uqm::front_end::credits::runtime::Credits` | 3 |
| `uqm::front_end::credits::runtime::credits_input` | 1 |
| `uqm::front_end::credits::runtime::outtakes` | 1 |
| `uqm::front_end::fmv::runtime::SplashScreen` | 4 |
| `uqm::front_end::gameopt::runtime::ConfirmSaveLoad` | 1 |
| `uqm::front_end::gameopt::runtime::DoGameOptions` | 1 |
| `uqm::front_end::gameopt::runtime::DoPickGame` | 2 |
| `uqm::front_end::gameopt::runtime::DoSettings` | 6 |
| `uqm::front_end::gameopt::runtime::DrawGameSelection` | 1 |
| `uqm::front_end::gameopt::runtime::DrawNameString` | 3 |
| `uqm::front_end::gameopt::runtime::DrawSaveNameString` | 4 |
| `uqm::front_end::gameopt::runtime::DrawSavegameCargo` | 3 |
| `uqm::front_end::gameopt::runtime::DrawSavegameSummary` | 15 |
| `uqm::front_end::gameopt::runtime::FeedbackSetting` | 3 |
| `uqm::front_end::gameopt::runtime::GameOptions` | 3 |
| `uqm::front_end::gameopt::runtime::NameCaptainOrShip` | 2 |
| `uqm::front_end::gameopt::runtime::PickGame` | 5 |
| `uqm::front_end::gameopt::runtime::game_string` | 1 |
| `uqm::front_end::intro::runtime::do_presentation` | 1 |
| `uqm::front_end::intro::runtime::fade_clear_screen` | 1 |
| `uqm::front_end::intro::runtime::generate_sis` | 2 |
| `uqm::front_end::intro::runtime::show_slides` | 1 |
| `uqm::front_end::intro::runtime::video_input` | 1 |
| `uqm::front_end::setupmenu::runtime::DoSetupMenu` | 2 |
| `uqm::front_end::setupmenu::runtime::SetupMenu` | 1 |
| `uqm::front_end::uqmdebug::runtime::UniverseRecurse` | 2 |
| `uqm::front_end::uqmdebug::runtime::activateAllShips` | 1 |
| `uqm::front_end::uqmdebug::runtime::calculateMineralValue` | 1 |
| `uqm::front_end::uqmdebug::runtime::clearEscorts` | 1 |
| `uqm::front_end::uqmdebug::runtime::doInstantMove` | 5 |
| `uqm::front_end::uqmdebug::runtime::drawContext` | 1 |
| `uqm::front_end::uqmdebug::runtime::dumpEvents` | 1 |
| `uqm::front_end::uqmdebug::runtime::dumpPlanet` | 1 |
| `uqm::front_end::uqmdebug::runtime::dumpStrings` | 1 |
| `uqm::front_end::uqmdebug::runtime::equipShip` | 24 |
| `uqm::front_end::uqmdebug::runtime::forAllPlanets` | 1 |
| `uqm::front_end::uqmdebug::runtime::forwardToNextEvent` | 1 |
| `uqm::front_end::uqmdebug::runtime::game_string` | 1 |
| `uqm::front_end::uqmdebug::runtime::getScreen` | 1 |
| `uqm::front_end::uqmdebug::runtime::giveDevices` | 1 |
| `uqm::front_end::uqmdebug::runtime::isContextVisible` | 1 |
| `uqm::front_end::uqmdebug::runtime::moonRecurse` | 1 |
| `uqm::front_end::uqmdebug::runtime::planetRecurse` | 1 |
| `uqm::front_end::uqmdebug::runtime::putScreen` | 1 |
| `uqm::front_end::uqmdebug::runtime::resetCrewBattle` | 2 |
| `uqm::front_end::uqmdebug::runtime::resetEnergyBattle` | 2 |
| `uqm::front_end::uqmdebug::runtime::showSpheres` | 1 |
| `uqm::front_end::uqmdebug::runtime::starRecurse` | 3 |
| `uqm::mainloop::restart_menu::menu_abi::uqm_get_utwig_bomb` | 1 |
| `uqm::mainloop::restart_menu::menu_abi::uqm_get_utwig_bomb_on_ship` | 1 |
| `uqm::mainloop::restart_menu::menu_abi::uqm_set_utwig_bomb_on_ship` | 1 |

