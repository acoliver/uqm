// Integration tests for battle engine Phase 2/3
// @plan PLAN-20260320-BATTLEPT2.P14
// @requirement REQ-E2E-INTEGRATION

#[cfg(test)]
mod tests {
    use crate::battle::display_list::DisplayList;
    use crate::battle::element::*;
    use crate::battle::lifecycle::*;
    use crate::battle::process_loop::*;
    use crate::battle::ship_runtime::*;
    use crate::battle::tactical::*;
    use crate::battle::velocity::VelocityDesc;

    const DISPLAY_LIST_CAPACITY: usize = 64;

    // ---- Test 1: Shared asset ref-counting ----

    #[test]
    fn test_shared_asset_ref_counting_lifecycle() {
        unsafe {
            let mut assets = SharedAssetState::new();
            assert!(assets.acquire());
            assert!(!assets.acquire());
            assert_eq!(assets.ref_count, 2);
            assert!(!assets.release());
            assert!(assets.release());
            assert!(!assets.loaded);
            assert!(assets.acquire()); // re-acquire loads again
            assert!(assets.release());
        }
    }

    // ---- Test 2: Input mapping → ship runtime ----

    #[test]
    fn test_input_mapping_to_ship_flags() {
        unsafe {
            let input = BattleInputState(
                BattleInputState::LEFT.0 | BattleInputState::THRUST.0 | BattleInputState::WEAPON.0,
            );
            let flags = map_battle_input(input);
            assert_ne!(flags & (LEFT as u32), 0);
            assert_ne!(flags & (THRUST as u32), 0);
            assert_ne!(flags & (WEAPON as u32), 0);
            assert_eq!(flags & (RIGHT as u32), 0);
        }
    }

    #[test]
    fn test_escape_triggers_flee_check() {
        unsafe {
            let input = BattleInputState(BattleInputState::ESCAPE.0);
            assert!(has_escape_input(input));
            assert!(run_away_allowed(IN_ENCOUNTER, true, false));
            assert!(!run_away_allowed(SUPER_MELEE, true, false));
        }
    }

    // ---- Test 3: Music selection ----

    #[test]
    fn test_music_selection_all_contexts() {
        unsafe {
            assert_eq!(
                select_battle_music(true, false),
                BattleMusicType::HyperSpace
            );
            assert_eq!(
                select_battle_music(false, true),
                BattleMusicType::QuasiSpace
            );
            assert_eq!(select_battle_music(false, false), BattleMusicType::Normal);
        }
    }

    // ---- Test 4: Element alloc → setup → free ----

    #[test]
    fn test_element_alloc_setup_free_cycle() {
        unsafe {
            let mut list = DisplayList::new(DISPLAY_LIST_CAPACITY);
            let handle = alloc_element(&mut list).expect("alloc should succeed");

            if let Some(elem) = list.get_mut(handle) {
                elem.state_flags = ElementFlags::APPEARING;
                elem.current.location.x = 100;
                elem.current.location.y = 200;
                elem.life_span = 10;
                setup_element(elem);
                assert_eq!(elem.next.location.x, 100);
                assert_eq!(elem.next.location.y, 200);
            }

            free_element(&mut list, handle);
        }
    }

    // ---- Test 5: Pre-process applies velocity ----

    #[test]
    fn test_pre_process_velocity_integration() {
        unsafe {
            let mut list = DisplayList::new(DISPLAY_LIST_CAPACITY);
            let handle = alloc_element(&mut list).expect("alloc should succeed");

            if let Some(elem) = list.get_mut(handle) {
                elem.state_flags = ElementFlags::PRE_PROCESS | ElementFlags::FINITE_LIFE;
                elem.life_span = 10;
                elem.current.location.x = 1000;
                elem.current.location.y = 2000;
                elem.next.location.x = 1000;
                elem.next.location.y = 2000;
                elem.velocity.set_components(256, 512);
            }

            unsafe { pre_process(handle, &mut list) };

            if let Some(elem) = list.get(handle) {
                assert_eq!(elem.life_span, 9);
                // Velocity applied (fixed-point: actual delta depends on VELOCITY_SHIFT)
                assert_ne!(elem.next.location.x, 1000); // moved
                assert_ne!(elem.next.location.y, 2000); // moved
            }
        }
    }

    // ---- Test 6: Zoom step mode ----

    #[test]
    fn test_zoom_step_mode_close() {
        unsafe {
            let r = calc_reduction_step(10, 10, 0, false, false);
            assert_eq!(r, 0);
        }
    }

    // ---- Test 7: Zoom continuous mode ----

    #[test]
    fn test_zoom_continuous_close() {
        unsafe {
            let r = calc_reduction_continuous(10, 10, false, false);
            assert!(r >= 0);
        }
    }

    // ---- Test 8: Display coordinate conversion ----

    #[test]
    fn test_display_coord_step_mode() {
        unsafe {
            assert_eq!(calc_display_coord_step(1000, 500, 0), 500);
            assert_eq!(calc_display_coord_step(1000, 500, 1), 250);
        }
    }

    #[test]
    fn test_display_coord_continuous_mode() {
        unsafe {
            let result = calc_display_coord_continuous(1000, 500, 1 << ZOOM_SHIFT);
            assert_eq!(result, 500);
        }
    }

    // ---- Test 9: Explosion fragment schedule ----

    #[test]
    fn test_explosion_fragment_schedule() {
        unsafe {
            // explosion_fragment_count returns (count, hide_ship, clear_preprocess)
            assert_eq!(explosion_fragment_count(0).0, 1);
            assert_eq!(explosion_fragment_count(3).0, 2);
            assert_eq!(explosion_fragment_count(10).0, 3);
            assert_eq!(explosion_fragment_count(15).0, 3);
            assert!(explosion_fragment_count(15).1); // hide_ship at tick 15
            assert_eq!(explosion_fragment_count(20).0, 1);
            assert!(explosion_fragment_count(25).2); // clear_preprocess at tick 25
        }
    }

    // ---- Test 10: Cleanup life_span ----

    #[test]
    fn test_cleanup_life_span_variants() {
        unsafe {
            let ls_with = compute_cleanup_life_span(true, false);
            let ls_without = compute_cleanup_life_span(false, false);
            let ls_winner = compute_cleanup_life_span(true, true);
            assert!(ls_with >= MIN_DITTY_FRAME_COUNT);
            assert_eq!(ls_without, 2);
            assert!(ls_winner > ls_with);
        }
    }

    // ---- Test 11: Flee allowance full matrix ----

    #[test]
    fn test_flee_allowance_matrix() {
        unsafe {
            assert!(run_away_allowed(IN_ENCOUNTER, true, false));
            assert!(run_away_allowed(IN_LAST_BATTLE, true, false));
            assert!(!run_away_allowed(SUPER_MELEE, true, false));
            assert!(!run_away_allowed(IN_ENCOUNTER, false, false));
            assert!(!run_away_allowed(IN_ENCOUNTER, true, true));
        }
    }

    // ---- Test 12: Battle counter decrement ----

    #[test]
    fn test_battle_counter_decrement() {
        unsafe {
            // MAX_SHIP_MASS = 10; normal ships have mass <= 10
            assert!(should_decrement_battle_counter(5));
            assert!(should_decrement_battle_counter(MAX_SHIP_MASS as u8));
            // Fleeing ships have mass > MAX_SHIP_MASS
            assert!(!should_decrement_battle_counter(MAX_SHIP_MASS as u8 + 1));
        }
    }

    // ---- Test 13: Ion trail colors ----

    #[test]
    fn test_ion_trail_colors_valid() {
        unsafe {
            assert_eq!(ION_TRAIL_COLOR_TABLE.len(), ION_TRAIL_LIFE as usize);
        }
    }

    // ---- Test 14: Player order ----

    #[test]
    fn test_player_order() {
        unsafe {
            assert_eq!(get_player_order(false, 0), (0, 1));
            assert_eq!(get_player_order(true, 1), (1, 0));
        }
    }

    // ---- Test 15: Instant victory + counters ----

    #[test]
    fn test_instant_victory_and_counters() {
        unsafe {
            assert_eq!(check_instant_victory(true), Some([1, 0]));
            assert!(check_instant_victory(false).is_none());
            assert_eq!(compute_battle_counters(3, 4), [3, 4]);
        }
    }

    // ---- Test 16: AI dispatch ----

    #[test]
    fn test_ai_dispatch_integration() {
        unsafe {
            use crate::battle::ai::*;
            // Missile evasion takes priority
            assert_eq!(
                select_dispatch_path(true, false, false),
                AiDispatchPath::MissileEvasion
            );
            // Flee when health low
            assert_eq!(
                select_dispatch_path(false, true, false),
                AiDispatchPath::FleeConsideration
            );
            // Special weapon when available and no higher priority
            assert_eq!(
                select_dispatch_path(false, false, true),
                AiDispatchPath::SpecialWeapon
            );
            // Standard combat as default
            assert_eq!(
                select_dispatch_path(false, false, false),
                AiDispatchPath::StandardCombat
            );
        }
    }

    // ---- Test 17: Inertial thrust ----

    #[test]
    fn test_inertial_thrust_basic() {
        unsafe {
            let mut vel = VelocityDesc::default();
            let _result = inertial_thrust(&mut vel, 0, 0, 64, 1024, 0u32, 0);
        }
    }

    // ---- Test 18: FFI exports callable ----

    #[test]
    fn test_all_ffi_exports_callable() {
        unsafe {
            use crate::battle::ffi::*;
            let _ = rust_battle_entry();
            let _ = rust_battle_frame();
            let _ = rust_battle_init_ships();
            rust_battle_uninit_ships();
            rust_battle_init_space();
            rust_battle_uninit_space();
            rust_battle_song(0);
            rust_free_battle_song();
            let _ = rust_get_player_order();
        }
    }

    // ---- Test 19: Element flag transitions ----

    #[test]
    fn test_element_flag_transitions() {
        unsafe {
            let mut elem = Element::default();
            elem.state_flags = ElementFlags::APPEARING;
            elem.state_flags.insert(ElementFlags::PRE_PROCESS);
            elem.state_flags.insert(ElementFlags::COLLISION);
            // Asymmetric clearing
            elem.state_flags.remove(ElementFlags::COLLISION);
            assert!(!elem.state_flags.contains(ElementFlags::COLLISION));
            assert!(elem.state_flags.contains(ElementFlags::PRE_PROCESS));
        }
    }

    // ---- Test 20: Battle sequence states ----

    #[test]
    fn test_battle_sequence_states() {
        unsafe {
            let states = [
                BattleSequenceState::Idle,
                BattleSequenceState::Initializing,
                BattleSequenceState::ShipsReady,
                BattleSequenceState::Selecting,
                BattleSequenceState::InBattle,
                BattleSequenceState::Aborting,
                BattleSequenceState::Finishing,
            ];
            for i in 0..states.len() {
                for j in (i + 1)..states.len() {
                    assert_ne!(states[i], states[j]);
                }
            }
        }
    }
}
