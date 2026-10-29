#[path = "support/battle_context.rs"]
mod battle_context;

#[path = "support/battle.rs"]
mod battle;
use battle::*;
use std::mem;
use uqm_rust::ships::native::{abi::*, element, frame_index};
#[test]
fn descriptor_and_twin_missile_launch() {
    battle_context::run(|| {
        let _battle = Battle::new(16);
        unsafe {
            let mut ship = Ship::new(&_battle, SPECIES_ID_YEHAT_ID, 0);
            assert_eq!(
                ship.descriptor_mut().ship_info.ship_flags,
                (FIRES_FORE | SHIELD_DEFENSE) as u16
            );
            assert_eq!(
                (
                    ship.descriptor_mut().ship_info.crew_level,
                    ship.descriptor_mut().ship_info.energy_level
                ),
                (20, 10)
            );
            assert_eq!(ship.descriptor_mut().cyborg_control.WeaponRange, 266);
            assert_eq!(
                (
                    ship.descriptor_mut().fleet.known_loc.x,
                    ship.descriptor_mut().fleet.known_loc.y
                ),
                (4970, 40)
            );
            let out = with_rng(1, 0, || {
                ship.weapon(2)
                    .into_iter()
                    .map(|id| {
                        id.map_or(std::ptr::null_mut(), |id| {
                            ship.weapon_record(Some(id)).cast()
                        })
                    })
                    .collect::<Vec<HELEMENT>>()
            });
            for (i, &h) in out.iter().enumerate() {
                let e = element(h);
                assert_eq!(
                    (
                        (*e).__bindgen_anon_1.life_span,
                        (*e).__bindgen_anon_2.hit_points,
                        (*e).__bindgen_anon_3.mass_points
                    ),
                    (10, 1, 1)
                );
                assert_eq!(
                    (*e).current.location.x,
                    4000 + if i == 0 { 32 } else { -32 }
                );
                assert_eq!((*e).current.location.y, 4000 - 64 + 80);
                assert_eq!(velocity(&(*e).velocity), toward(80, 0));
                FreeElement(h);
            }
        }
    })
}
#[test]
fn shield_uses_equivalent_facing_expires_and_recharges_on_postprocess() {
    battle_context::run(|| {
        let _battle = Battle::new(16);
        unsafe {
            let mut ship = Ship::new(&_battle, SPECIES_ID_YEHAT_ID, 0);
            let e = ship.ptr();
            (*e).next.image.frame = SetAbsFrameIndex(ship.descriptor_mut().ship_data.ship[0], 7);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            with_rng(2, 0, || ship.preprocess());
            assert_eq!((*e).__bindgen_anon_1.life_span, 11);
            assert_eq!(ship.ship_mut().special_counter, 2);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 10);
            assert_eq!(
                (*e).next.image.farray,
                (&raw mut ship.descriptor_mut().ship_data.special).cast()
            );
            assert_eq!(frame_index((*e).next.image.frame), 7);
            ship.postprocess();
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 7);
            ship.ship_mut().special_counter = 0;
            ship.preprocess();
            assert_eq!((*e).__bindgen_anon_1.life_span, 11);
            assert_eq!(ship.ship_mut().special_counter, 2);
            ship.postprocess();
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 4);
            ship.ship_mut().cur_status_flags = 0;
            for _ in 0..10 {
                ship.preprocess();
            }
            assert_eq!((*e).__bindgen_anon_1.life_span, 1);
            assert_eq!(
                (*e).next.image.farray,
                (&raw mut ship.descriptor_mut().ship_data.ship).cast()
            );
            assert_eq!(frame_index((*e).next.image.frame), 7);
            ship.postprocess();
            assert_eq!(ship.ship_mut().special_counter, 0);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 4);
        }
    })
}
#[test]
fn low_energy_flashes_without_shield_and_appearing_or_nonsolid_skips_callbacks() {
    battle_context::run(|| {
        let _battle = Battle::new(16);
        unsafe {
            let mut ship = Ship::new(&_battle, SPECIES_ID_YEHAT_ID, 0);
            ship.descriptor_mut().ship_info.energy_level = 2;
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.preprocess();
            assert_eq!((*ship.ptr()).__bindgen_anon_1.life_span, 1);
            assert_eq!(ship.ship_mut().special_counter, 0);
            assert_ne!(ship.ship_mut().cur_status_flags & (1 << 5), 0);
            ship.descriptor_mut().ship_info.energy_level = 10;
            (*ship.ptr()).state_flags |= APPEARING as u16;
            ship.preprocess();
            assert_eq!(ship.ship_mut().special_counter, 0);
            ship.ship_mut().special_counter = 2;
            (*ship.ptr()).state_flags |= NONSOLID as u16;
            ship.postprocess();
            assert_eq!(ship.ship_mut().special_counter, 2);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 10);
        }
    })
}
#[test]
fn ai_doubles_nonfinite_crew_turns_halves_finite_turns_and_obeys_life_boundary() {
    battle_context::run(|| {
        let _battle = Battle::new(16);
        unsafe {
            let mut ship = Ship::new(&_battle, SPECIES_ID_YEHAT_ID, 0);
            let mut threat_fixture = Object::new(&_battle);
            let threat = threat_fixture.ptr();
            for finite in [false, true] {
                for life in [2, 3] {
                    for turn in [0, 1, 2, 4, 6] {
                        for seed in 1..5 {
                            (*threat).state_flags =
                                (CREW_OBJECT | if finite { FINITE_LIFE } else { 0 }) as u16;
                            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
                            concerns[ENEMY_WEAPON_INDEX as usize] = EVALUATE_DESC {
                                ObjectPtr: threat,
                                facing: 0,
                                which_turn: turn,
                                MoveState: MOVEMENT_STATE_ENTICE,
                            };
                            (*ship.ptr()).__bindgen_anon_1.life_span = life;
                            ship.ship_mut().ship_input_state = 0;
                            let transformed = if finite { (turn >> 1).max(1) } else { turn * 2 };
                            TFB_SeedRandom(seed);
                            let shield = TFB_Random() & 3 != 0;
                            with_rng(seed, usize::from(life <= 2 && transformed <= 2), || {
                                ship.intelligence(&mut *ship.ptr(), &mut concerns, 0)
                            });
                            assert_eq!(
                                concerns[ENEMY_WEAPON_INDEX as usize].which_turn,
                                transformed
                            );
                            assert!(!concerns[ENEMY_WEAPON_INDEX as usize].ObjectPtr.is_null());
                            assert_eq!(
                                ship.ship_mut().ship_input_state & SPECIAL as u8 != 0,
                                life <= 2 && transformed <= 2 && shield
                            );
                        }
                    }
                }
            }
        }
    })
}
