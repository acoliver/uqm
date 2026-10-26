// Integration tests for battle engine Phase 2/3
// @plan PLAN-20260320-BATTLEPT2.P14
// @requirement REQ-E2E-INTEGRATION

#[cfg(test)]
mod tests {
    use crate::battle::element::*;
    use crate::battle::lifecycle::*;
    use crate::battle::ship_runtime::*;
    use crate::battle::velocity::VelocityDesc;

    // ---- Test 1: Shared asset ref-counting ----

    #[test]
    fn test_shared_asset_ref_counting_lifecycle() {
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

    // ---- Test 2: Input mapping → ship runtime ----

    #[test]
    fn test_input_mapping_to_ship_flags() {
        let input = BattleInputState(
            BattleInputState::LEFT.0 | BattleInputState::THRUST.0 | BattleInputState::WEAPON.0,
        );
        let flags = map_battle_input(input);
        assert_ne!(flags & LEFT, 0);
        assert_ne!(flags & THRUST, 0);
        assert_ne!(flags & WEAPON, 0);
        assert_eq!(flags & RIGHT, 0);
    }

    #[test]
    fn test_escape_triggers_flee_check() {
        let input = BattleInputState(BattleInputState::ESCAPE.0);
        assert!(has_escape_input(input));
    }

    // ---- Test 3: Music selection ----

    #[test]
    fn test_music_selection_all_contexts() {
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

    // ---- Test 14: Player order ----

    #[test]
    fn test_player_order() {
        assert_eq!(get_player_order(false, 0), (0, 1));
        assert_eq!(get_player_order(true, 1), (1, 0));
    }

    // ---- Test 15: Instant victory + counters ----

    #[test]
    fn test_instant_victory_and_counters() {
        assert_eq!(check_instant_victory(true), Some([1, 0]));
        assert!(check_instant_victory(false).is_none());
        assert_eq!(compute_battle_counters(3, 4), [3, 4]);
    }

    // ---- Test 16: AI dispatch ----

    #[test]
    fn test_ai_dispatch_integration() {
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

    // ---- Test 17: Inertial thrust ----

    #[test]
    fn test_inertial_thrust_basic() {
        let mut vel = VelocityDesc::default();
        let _result = inertial_thrust(&mut vel, 0, 0, 64, 1024, 0u32, 0);
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
        let mut elem = Element {
            state_flags: ElementFlags::APPEARING,
            ..Element::default()
        };
        elem.state_flags.insert(ElementFlags::PRE_PROCESS);
        elem.state_flags.insert(ElementFlags::COLLISION);
        // Asymmetric clearing
        elem.state_flags.remove(ElementFlags::COLLISION);
        assert!(!elem.state_flags.contains(ElementFlags::COLLISION));
        assert!(elem.state_flags.contains(ElementFlags::PRE_PROCESS));
    }

    // ---- Test 20: Battle sequence states ----

    #[test]
    fn test_battle_sequence_states() {
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
