#[path = "support/battle_context.rs"]
mod battle_context;

#[path = "support/battle.rs"]
mod battle;
#[path = "support/special_hooks.rs"]
mod special_hooks;
use battle::*;
use uqm_rust::collections::observation;
use uqm_rust::engine::battle::ai_constants::ENEMY_WEAPON_INDEX;
use uqm_rust::engine::battle::ai_constants::FIRES_FORE;
use uqm_rust::engine::battle::ai_constants::MOVEMENT_STATE_ENTICE;
use uqm_rust::engine::battle::ai_constants::SHIELD_DEFENSE;
use uqm_rust::engine::battle::ai_constants::SPECIAL;
use uqm_rust::engine::battle::flags::APPEARING;
use uqm_rust::engine::battle::flags::CREW_OBJECT;
use uqm_rust::engine::battle::flags::FINITE_LIFE;
use uqm_rust::engine::battle::flags::NONSOLID;
use uqm_rust::engine::battle::Concern;
use uqm_rust::engine::battle::ElementId;
use uqm_rust::engine::species::species_id::SPECIES_ID_YEHAT_ID;
use uqm_rust::math::random;
use uqm_rust::math::seed_random;
#[test]
fn descriptor_and_twin_missile_launch() {
    battle_context::run(|_| {
        let _battle = Battle::new(16);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_YEHAT_ID, 0);
            assert_eq!(
                ship.descriptor_mut().ship_info.ship_flags,
                (FIRES_FORE | SHIELD_DEFENSE) as u16
            );
            {
                let descriptor = ship.descriptor_mut();
                assert_eq!(
                    (
                        descriptor.ship_info.crew_level,
                        descriptor.ship_info.energy_level
                    ),
                    (20, 10)
                );
            }
            assert_eq!(ship.descriptor_mut().cyborg_control.weapon_range, 266);
            {
                let descriptor = ship.descriptor_mut();
                assert_eq!(
                    (descriptor.fleet.known_loc.x, descriptor.fleet.known_loc.y),
                    (4970, 40)
                );
            }
            let out = with_rng(1, 0, || {
                ship.weapon(2)
                    .into_iter()
                    .map(|id| id.expect("allocated weapon"))
                    .collect::<Vec<ElementId>>()
            });
            for (i, &h) in out.iter().enumerate() {
                let e = h;
                assert_eq!(
                    (
                        battle::get(e).life,
                        battle::get(e).hits,
                        battle::get(e).mass
                    ),
                    (10, 1, 1)
                );
                assert_eq!(
                    battle::get(e).current.location.x,
                    4000 + if i == 0 { 32 } else { -32 }
                );
                assert_eq!(battle::get(e).current.location.y, 4000 - 64 + 80);
                assert_eq!(velocity(&battle::get(e).velocity), toward(80, 0));
                battle_context::dispose(h);
            }
        }
    })
}
#[test]
fn shield_uses_equivalent_facing_expires_and_recharges_on_postprocess() {
    battle_context::run(|_| {
        let _battle = Battle::new(16);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_YEHAT_ID, 0);
            let e = ship.id;
            battle::get_mut(e).next.image.frame = ship.frame(0, 7);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            observation::enable();
            observation::drain();
            with_rng(2, 0, || ship.preprocess());
            assert_eq!(special_hooks::count(&observation::drain(), "yehat"), 1);
            assert_eq!(battle::get(e).life, 11);
            assert_eq!(ship.ship_mut().special_counter, 2);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 10);
            assert_eq!(battle::get(e).next.image.frames, ship.frame_set(2));
            assert_eq!((battle::get(e).next.image.frame.raw()).index(), 7);
            ship.postprocess();
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 7);
            ship.ship_mut().special_counter = 0;
            ship.preprocess();
            assert_eq!(special_hooks::count(&observation::drain(), "yehat"), 1);
            assert_eq!(battle::get(e).life, 11);
            assert_eq!(ship.ship_mut().special_counter, 2);
            ship.postprocess();
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 4);
            ship.ship_mut().cur_status_flags = 0;
            for _ in 0..10 {
                ship.preprocess();
            }
            assert_eq!(battle::get(e).life, 1);
            assert_eq!(battle::get(e).next.image.frames, ship.frame_set(0));
            assert_eq!((battle::get(e).next.image.frame.raw()).index(), 7);
            ship.postprocess();
            assert_eq!(ship.ship_mut().special_counter, 0);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 4);
        }
    })
}
#[test]
fn low_energy_flashes_without_shield_and_appearing_or_nonsolid_skips_callbacks() {
    battle_context::run(|_| {
        let _battle = Battle::new(16);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_YEHAT_ID, 0);
            ship.descriptor_mut().ship_info.energy_level = 2;
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            observation::enable();
            observation::drain();
            ship.preprocess();
            assert_eq!(battle::get(ship.id).life, 1);
            assert_eq!(ship.ship_mut().special_counter, 0);
            assert_ne!(ship.ship_mut().cur_status_flags & (1 << 5), 0);
            ship.descriptor_mut().ship_info.energy_level = 10;
            battle::get_mut(ship.id).state_flags |= APPEARING as u16;
            ship.preprocess();
            // Low energy only flashes, and an appearing ship skips the shield.
            assert_eq!(special_hooks::count(&observation::drain(), "yehat"), 0);
            assert_eq!(ship.ship_mut().special_counter, 0);
            ship.ship_mut().special_counter = 2;
            battle::get_mut(ship.id).state_flags |= NONSOLID as u16;
            ship.postprocess();
            assert_eq!(ship.ship_mut().special_counter, 2);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 10);
        }
    })
}
#[test]
fn ai_doubles_nonfinite_crew_turns_halves_finite_turns_and_obeys_life_boundary() {
    battle_context::run(|_| {
        let _battle = Battle::new(16);
        {
            let mut ship = Ship::new(&_battle, SPECIES_ID_YEHAT_ID, 0);
            let threat_fixture = Object::new(&_battle);
            let threat = threat_fixture.id;
            for finite in [false, true] {
                for life in [2, 3] {
                    for turn in [0, 1, 2, 4, 6] {
                        for seed in 1..5 {
                            battle::get_mut(threat).state_flags =
                                (CREW_OBJECT | if finite { FINITE_LIFE } else { 0 }) as u16;
                            let mut concerns: [Concern; 4] = [Concern::default(); 4];
                            concerns[ENEMY_WEAPON_INDEX as usize] = Concern {
                                object: Some(threat),
                                facing: 0,
                                which_turn: turn,
                                movement: MOVEMENT_STATE_ENTICE,
                            };
                            battle::get_mut(ship.id).life = life;
                            ship.ship_mut().ship_input_state = 0;
                            let transformed = if finite { (turn >> 1).max(1) } else { turn * 2 };
                            seed_random(seed);
                            let shield = random() & 3 != 0;
                            with_rng(seed, usize::from(life <= 2 && transformed <= 2), || {
                                ship.think(ship.id, &mut concerns, 0)
                            });
                            assert_eq!(
                                concerns[ENEMY_WEAPON_INDEX as usize].which_turn,
                                transformed
                            );
                            assert!(!concerns[ENEMY_WEAPON_INDEX as usize].object.is_none());
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
