//! Live Umgah callbacks, private cone state, collision cleanup and AI RNG order.
#![forbid(unsafe_code)]

#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
use battle::{Battle, PoolElement, Ship};
use uqm_engine::battle::ai_constants::ENEMY_WEAPON_INDEX;
use uqm_engine::battle::ai_constants::FIRES_FORE;
use uqm_engine::battle::ai_constants::GRAVITY_MASS_INDEX;
use uqm_engine::battle::ai_constants::IGNORE_SIMILAR;
use uqm_engine::battle::ai_constants::IMMEDIATE_WEAPON;
use uqm_engine::battle::ai_constants::LEFT;
use uqm_engine::battle::ai_constants::MOVEMENT_STATE_AVOID;
use uqm_engine::battle::ai_constants::MOVEMENT_STATE_ENTICE;
use uqm_engine::battle::ai_constants::MOVEMENT_STATE_NO_MOVEMENT;
use uqm_engine::battle::ai_constants::MOVEMENT_STATE_PURSUE;
use uqm_engine::battle::ai_constants::RIGHT;
use uqm_engine::battle::ai_constants::SPECIAL;
use uqm_engine::battle::ai_constants::THRUST;
use uqm_engine::battle::ai_constants::WEAPON;
use uqm_engine::battle::flags::APPEARING;
use uqm_engine::battle::flags::COLLISION;
use uqm_engine::battle::flags::CREW_OBJECT;
use uqm_engine::battle::flags::DISAPPEARING;
use uqm_engine::battle::flags::FINITE_LIFE;
use uqm_engine::battle::flags::NONSOLID;
use uqm_engine::battle::flags::PLAYER_SHIP;
use uqm_engine::battle::flags::SHIP_AT_MAX_SPEED;
use uqm_engine::battle::flags::SHIP_BEYOND_MAX_SPEED;
use uqm_engine::battle::flags::SHIP_IN_GRAVITY_WELL;
use uqm_engine::battle::Concern;
use uqm_engine::battle::ElementId;
use uqm_engine::battle::STAMP_PRIMITIVE;
use uqm_engine::graphics::geometry::Point;
use uqm_engine::math::arctan;
use uqm_engine::math::random;
use uqm_engine::math::seed_random;
use uqm_engine::math::velocity::VelocityDesc;
use uqm_engine::species::species_id::SPECIES_ID_UMGAH_ID;
#[path = "support/special_hooks.rs"]
mod special_hooks;

use uqm_content::ships::native::{cosine, facing_to_angle, sine};
use uqm_engine::collections::observation;

// FRAME_DESC and DRAWABLE_DESC layouts from drawable.h.

fn queued() -> Vec<ElementId> {
    let mut result = Vec::new();
    let mut cursor = battle::head();
    while let Some(h) = cursor {
        result.push(h);
        assert!(result.len() <= 64, "display queue cycle");
        cursor = battle::succ(h);
    }
    result
}

fn velocity_fields(v: &VelocityDesc) -> [i32; 9] {
    [
        v.travel_angle.into(),
        v.vector.width.into(),
        v.vector.height.into(),
        v.fract.width.into(),
        v.fract.height.into(),
        v.error.width.into(),
        v.error.height.into(),
        v.incr.width.into(),
        v.incr.height.into(),
    ]
}

fn velocity(e: ElementId) -> (i16, i16) {
    let (velocity_x, velocity_y) = (battle::get(e).velocity).get_current_components();
    let (x, y) = (velocity_x as i16, velocity_y as i16);
    (x, y)
}

fn draws(seed: u32, n: usize) -> Vec<u32> {
    seed_random(seed);
    (0..n).map(|_| random()).collect()
}

fn seed_where(accept: impl Fn(&[u32]) -> bool) -> u32 {
    (1..100_000).find(|&seed| accept(&draws(seed, 3))).unwrap()
}

fn with_rng<T>(seed: u32, used: usize, action: impl FnOnce() -> T) -> T {
    let sequence = draws(seed, used + 1);
    seed_random(seed);
    let result = action();
    assert_eq!(
        random(),
        sequence[used],
        "RNG consumption: {used}, seed {seed}"
    );
    result
}

fn concerns() -> [Concern; 4] {
    [Concern {
        object: None,
        facing: 0,
        which_turn: 0,
        movement: MOVEMENT_STATE_NO_MOVEMENT,
    }; 4]
}

fn enemy_concern(enemy: &Ship<'_>, turn: u16) -> [Concern; 4] {
    let mut c = concerns();
    c[0].object = Some(enemy.id);
    c[0].which_turn = turn;
    c[0].movement = MOVEMENT_STATE_PURSUE;
    c
}

fn think(ship: &mut Ship<'_>, c: &mut [Concern; 4], input: u8, count: u16) -> u8 {
    ship.ship_mut().ship_input_state = input;
    ship.think(ship.id, c, count);
    ship.ship_mut().ship_input_state
}

#[test]
fn descriptor_matches_all_c_initializer_fields_and_resources() {
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        {
            let d = battle::descriptor(SPECIES_ID_UMGAH_ID);
            assert_eq!(
                d.ship_info.ship_flags,
                (FIRES_FORE | IMMEDIATE_WEAPON) as u16
            );
            assert_eq!(d.ship_info.ship_cost, 7);
            assert_eq!((d.ship_info.crew_level, d.ship_info.max_crew), (10, 10));
            assert_eq!((d.ship_info.energy_level, d.ship_info.max_energy), (30, 30));
            assert_eq!(d.ship_info.race_strings_rsc.unwrap(), c"ship.umgah.text");
            assert_eq!(d.ship_info.icons_rsc.unwrap(), c"ship.umgah.icons");
            assert_eq!(
                d.ship_info.melee_icon_rsc.unwrap(),
                c"ship.umgah.meleeicons"
            );
            assert!(
                d.ship_info.race_strings.is_none()
                    && d.ship_info.icons.is_null()
                    && d.ship_info.melee_icon.is_null()
            );
            assert_eq!(d.fleet.strength, 150);
            assert_eq!((d.fleet.known_loc.x, d.fleet.known_loc.y), (1798, 6000));
            let c = d.characteristics;
            assert_eq!(
                (c.max_thrust, c.thrust_increment, c.energy_regeneration),
                (18, 6, 30)
            );
            assert_eq!(
                (c.weapon_energy_cost, c.special_energy_cost, c.energy_wait),
                (0, 1, 150)
            );
            assert_eq!(
                (
                    c.turn_wait,
                    c.thrust_wait,
                    c.weapon_wait,
                    c.special_wait,
                    c.ship_mass
                ),
                (4, 3, 0, 2, 1)
            );
            for (i, size) in ["large", "medium", "small"].iter().enumerate() {
                assert_eq!(
                    d.ship_data.ship_rsc[i].unwrap().to_str().unwrap(),
                    format!("ship.umgah.graphics.drone.{size}")
                );
                assert_eq!(
                    d.ship_data.special_rsc[i].unwrap().to_str().unwrap(),
                    format!("ship.umgah.graphics.cone.{size}")
                );
                assert!(
                    d.ship_data.ship.view(i).is_absent()
                        && d.ship_data.weapon.view(i).is_absent()
                        && d.ship_data.special.view(i).is_absent()
                );
            }
            assert_eq!(
                d.ship_data.weapon_rsc[0].unwrap(),
                c"ship.umgah.graphics.spritz"
            );
            assert!(d.ship_data.weapon_rsc[1].is_none() && d.ship_data.weapon_rsc[2].is_none());
            let captain = &d.ship_data.captain_control;
            assert_eq!(captain.captain_rsc.unwrap(), c"ship.umgah.graphics.captain");
            assert!(
                captain.background.is_null()
                    && captain.turn.is_null()
                    && captain.thrust.is_null()
                    && captain.weapon.is_null()
                    && captain.special.is_absent()
            );
            assert_eq!(d.ship_data.victory_ditty_rsc.unwrap(), c"ship.umgah.ditty");
            assert_eq!(d.ship_data.ship_sounds_rsc.unwrap(), c"ship.umgah.sounds");
            assert!(d.ship_data.victory_ditty.is_null() && d.ship_data.ship_sounds.is_none());
            assert_eq!(
                (
                    d.cyborg_control.maneuverability_index,
                    d.cyborg_control.weapon_range
                ),
                (0, 16000)
            );
            assert!(
                d.race.unwrap().preprocess.is_some()
                    && d.race.unwrap().postprocess.is_some()
                    && d.race.unwrap().init_weapon.is_some()
                    && d.race.unwrap().uninit.is_some()
                    && d.race.unwrap().intelligence.is_some()
            );
        }
    })
}

#[test]
fn per_init_descriptor_copies_isolate_energy_resources_animation_and_custom_data() {
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        {
            let mut first =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            first.ship_mut().ship_facing = 5;
            let h = fire(&mut first).unwrap();
            let data = previous_facing(&mut first);
            assert!(data.is_some());
            hooks::invoke_element(battle::get(h).preprocess_func.unwrap(), h);
            assert_eq!(
                (first.descriptor_mut().ship_data.special.view(0).raw()).index(),
                21
            );
            first.descriptor_mut().ship_info.energy_level = 1;
            first.descriptor_mut().characteristics.special_wait = 77;
            let mut second =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 1, [16, 16, 48], [4, 4]);
            let first_descriptor = std::ptr::from_ref(&*first.descriptor_mut());
            let second_descriptor = std::ptr::from_ref(&*second.descriptor_mut());
            assert_ne!(first_descriptor, second_descriptor);
            assert!(previous_facing(&mut second).is_none());
            assert_eq!(second.descriptor_mut().ship_info.energy_level, 30);
            assert_eq!(second.descriptor_mut().characteristics.special_wait, 2);
            assert_eq!(
                (second.descriptor_mut().ship_data.special.view(0).raw()).index(),
                0
            );
            second.ship_mut().ship_facing = 9;
            let other = fire(&mut second).unwrap();
            assert_ne!(previous_facing(&mut first), previous_facing(&mut second));
            assert_eq!(previous_facing(&mut first), data);
            assert_eq!(
                (first.descriptor_mut().ship_data.special.view(0).raw()).index(),
                21
            );
            first.uninit();
            assert!(previous_facing(&mut first).is_none());
            assert!(!previous_facing(&mut second).is_none());
            hooks::invoke_element(battle::get(other).preprocess_func.unwrap(), other);
            assert_eq!(
                (second.descriptor_mut().ship_data.special.view(0).raw()).index(),
                25
            );
            battle_context::dispose(h);
            battle_context::dispose(other);
            let mut template = battle::descriptor(SPECIES_ID_UMGAH_ID);
            template.ship_info.energy_level = 0;
            template.characteristics.special_wait = 0;
            template.race_callbacks.pre = uqm_engine::battle::Selection::Disabled;
            assert_eq!(template.ship_info.energy_level, 0);
            assert_eq!(template.characteristics.special_wait, 0);
            assert!(matches!(
                template.race_callbacks.pre,
                uqm_engine::battle::Selection::Disabled
            ));
            let fresh = battle::descriptor(SPECIES_ID_UMGAH_ID);
            assert_eq!(fresh.ship_info.energy_level, 30);
            assert_eq!(fresh.characteristics.special_wait, 2);
            assert!(fresh.race.unwrap().preprocess.is_some());
        }
    })
}

#[test]
fn cone_initialization_fixes_intersection_and_does_not_inherit_velocity() {
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 1, [16, 16, 48], [4, 4]);
            ship.ship_mut().ship_facing = 5;
            battle::get_mut(ship.id).next.location = Point { x: -9, y: 13 };
            (battle::get_mut(ship.id).velocity).set_components(321, -654);
            let h = with_rng(8, 0, || fire(&mut ship)).unwrap();
            let cone = h;
            assert_eq!(
                battle::get(cone).state_flags,
                (FINITE_LIFE | IGNORE_SIMILAR) as u16
            );
            assert_eq!(battle::get(cone).life, 1);
            assert_eq!(battle::get(cone).hits, 100);
            assert_eq!(battle::get(cone).mass, 1);
            assert_eq!(battle::get(cone).thrust, 0);
            assert_eq!(battle::get(cone).player_nr, 1);
            assert_eq!(velocity_fields(&battle::get(cone).velocity), [0; 9]);
            assert_eq!(
                (
                    battle::get(cone).current.location.x,
                    battle::get(cone).current.location.y
                ),
                (-9, 13)
            );
            assert_eq!(
                (
                    battle::get(cone).next.location.x,
                    battle::get(cone).next.location.y
                ),
                (-9, 13)
            );
            assert_eq!(
                battle::get(cone).current.image.frame.raw(),
                ship.raw_frame(2, 5)
            );
            assert_eq!(
                battle::get(cone).next.image.frame.raw(),
                battle::get(cone).current.image.frame.raw()
            );
            assert_eq!(battle::get(cone).next.image.frames, ship.frame_set(2));
            let i = battle::get(cone).intersect_control;
            assert_eq!(
                (i.intersect_stamp.origin.x, i.intersect_stamp.origin.y),
                (-3, 3)
            );
            assert_eq!((i.end_point.x, i.end_point.y), (-3, 3));
            assert_eq!(i.intersect_stamp.frame, ship.raw_frame(2, 5));
            assert_eq!(battle::prim(cone).kind, STAMP_PRIMITIVE);
            assert!(
                battle::get(cone).preprocess_func.is_some()
                    && battle::get(cone).collision_func.is_some()
            );
            assert!(
                battle::get(cone).postprocess_func.is_none()
                    && battle::get(cone).death_func.is_none()
            );
            assert_eq!(queued(), vec![ship.id]);
            battle_context::dispose(h);
        }
    })
}

#[test]
fn held_cone_advances_by_16_preserves_phase_and_resets_only_when_facing_changes() {
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            ship.ship_mut().ship_facing = 5;
            for index in [5, 21, 37, 5] {
                let h = with_rng(3, 0, || fire(&mut ship)).unwrap();
                let cone = h;
                let data = previous_facing(&mut ship);
                assert!(data.is_some());
                assert_eq!(data, Some(5));
                assert_eq!((battle::get(cone).current.image.frame.raw()).index(), index);
                let old_frame = battle::get(cone).next.image.frame.raw();
                with_rng(3, 0, || {
                    hooks::invoke_element(battle::get(cone).preprocess_func.unwrap(), cone)
                });
                assert_eq!(
                    (ship.descriptor_mut().ship_data.special.view(0).raw()).index(),
                    (index + 16) % 48
                );
                assert_eq!(battle::get(cone).next.image.frame.raw(), old_frame);
                assert_eq!(
                    battle::get(cone).state_flags,
                    (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
                );
                let next = fire(&mut ship).unwrap();
                assert_eq!(
                    previous_facing(&mut ship),
                    data,
                    "holding facing keeps the saved facing"
                );
                assert_eq!(
                    (battle::get(next).current.image.frame.raw()).index(),
                    (index + 16) % 48
                );
                battle_context::dispose(next);
                battle_context::dispose(h);
            }
            ship.ship_mut().ship_facing = 9;
            let h = fire(&mut ship).unwrap();
            assert_eq!(previous_facing(&mut ship).unwrap(), 9);
            assert_eq!((battle::get(h).current.image.frame.raw()).index(), 9);
            assert_eq!(
                battle::get(h).intersect_control.intersect_stamp.frame,
                ship.raw_frame(2, 9)
            );
            battle_context::dispose(h);
        }
    })
}

#[test]
fn appearance_and_uninit_free_only_the_instance_custom_data_and_are_repeatable() {
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let h = fire(&mut ship).unwrap();
            battle_context::dispose(h);
            assert!(!previous_facing(&mut ship).is_none());
            (battle::get_mut(ship.id).velocity).set_components(123, -456);
            let before = velocity_fields(&battle::get(ship.id).velocity);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.ship_mut().special_counter = 9;
            battle::get_mut(ship.id).state_flags |= APPEARING as u16;
            with_rng(4, 0, || ship.preprocess());
            assert!(previous_facing(&mut ship).is_none());
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 30);
            assert_eq!(ship.ship_mut().special_counter, 9);
            assert_eq!(velocity_fields(&battle::get(ship.id).velocity), before);
            ship.preprocess();
            let h = fire(&mut ship).unwrap();
            battle_context::dispose(h);
            assert!(!previous_facing(&mut ship).is_none());
            let frame = ship.descriptor_mut().ship_data.special.view(0).raw();
            ship.uninit();
            ship.uninit();
            assert!(previous_facing(&mut ship).is_none());
            assert_eq!(ship.descriptor_mut().ship_data.special.view(0).raw(), frame);
        }
    })
}

#[test]
fn failed_cone_allocation_still_updates_private_facing_and_returns_one_slot() {
    battle_context::run(|_| {
        let _battle = Battle::new(1);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            ship.ship_mut().ship_facing = 15;
            assert!(with_rng(5, 0, || fire(&mut ship)).is_none());
            assert!(!previous_facing(&mut ship).is_none());
            assert_eq!(previous_facing(&mut ship).unwrap(), 15);
            assert_eq!(
                (ship.descriptor_mut().ship_data.special.view(0).raw()).index(),
                15
            );
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 30);
            ship.ship_mut().ship_facing = 4;
            assert!(fire(&mut ship).is_none());
            assert_eq!(previous_facing(&mut ship).unwrap(), 4);
            assert_eq!(
                (ship.descriptor_mut().ship_data.special.view(0).raw()).index(),
                4
            );
            assert_eq!(queued(), vec![ship.id]);
        }
    })
}

fn target(finite: bool) -> PoolElement {
    let e = PoolElement::new();
    battle::get_mut(e.id).player_nr = 1;
    battle::get_mut(e.id).state_flags = if finite { FINITE_LIFE as u16 } else { 0 };
    battle::get_mut(e.id).life = 2;
    battle::get_mut(e.id).hits = 3;
    battle::get_mut(e.id).mass = 1;
    e
}

#[test]
fn collision_removes_and_frees_the_blast_but_does_not_restore_cone_life_or_solidity() {
    battle_context::run(|_| {
        let _battle = Battle::new(4);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let h = fire(&mut ship).unwrap();
            let cone = h;
            let hit = target(false);
            let mut a = Point { x: 10, y: 20 };
            let mut b = Point { x: 30, y: 40 };
            let before = queued();
            with_rng(1, 0, || {
                hooks::invoke_collision(
                    battle::get(cone).collision_func.unwrap(),
                    cone,
                    &mut a,
                    hit.id,
                    &mut b,
                )
            });
            assert_eq!(queued(), before, "the generic collision blast was unlinked");
            assert_eq!(
                battle::get(cone).state_flags,
                (FINITE_LIFE | IGNORE_SIMILAR | COLLISION | NONSOLID) as u16
            );
            assert_eq!(battle::get(cone).life, 0);
            assert_eq!(battle::get(cone).hits, 0);
            assert_eq!(battle::get(hit.id).hits, 3);
            let freed_blast = crate::battle_context::with(|cx| cx.alloc_element())
                .unwrap_or_else(|| panic!("blast element returned to the queue free list"));
            assert!(crate::battle_context::with(|cx| cx.alloc_element()).is_none());
            battle_context::dispose(freed_blast);
            battle_context::dispose(h);
        }
    })
}

#[test]
fn collision_preserves_disappearing_when_blast_allocation_fails() {
    battle_context::run(|_| {
        let _battle = Battle::new(3);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let h = fire(&mut ship).unwrap();
            let cone = h;
            let hit = target(false);
            let mut p = Point { x: 0, y: 0 };
            hooks::invoke_collision(
                battle::get(cone).collision_func.unwrap(),
                cone,
                &mut p,
                hit.id,
                &mut Point { x: 0, y: 0 },
            );
            assert_eq!(
                battle::get(cone).state_flags & (DISAPPEARING | COLLISION | NONSOLID) as u16,
                (DISAPPEARING | COLLISION | NONSOLID) as u16
            );
            assert_eq!(battle::get(cone).life, 0);
            assert_eq!(queued(), vec![ship.id]);
            battle_context::dispose(h);
        }
    })
}

#[test]
fn finite_weapon_hit_damages_once_without_a_blast_and_does_not_clear_disappearing() {
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let h = fire(&mut ship).unwrap();
            let cone = h;
            let hit = target(true);
            battle::get_mut(cone).state_flags |= DISAPPEARING as u16;
            let collide = battle::get(cone).collision_func.unwrap();
            with_rng(7, 0, || {
                hooks::invoke_collision(
                    collide,
                    cone,
                    &mut Point { x: 0, y: 0 },
                    hit.id,
                    &mut Point { x: 0, y: 0 },
                )
            });
            assert_eq!(battle::get(hit.id).hits, 2);
            assert_eq!(battle::get(cone).hits, 100);
            assert_eq!(battle::get(cone).life, 1);
            assert_ne!(battle::get(cone).state_flags & COLLISION as u16, 0);
            assert_ne!(battle::get(cone).state_flags & DISAPPEARING as u16, 0);
            hooks::invoke_collision(
                collide,
                cone,
                &mut Point { x: 0, y: 0 },
                hit.id,
                &mut Point { x: 0, y: 0 },
            );
            assert_eq!(battle::get(hit.id).hits, 2);
            assert_eq!(queued(), vec![ship.id]);
            battle_context::dispose(h);
        }
    })
}

#[test]
fn retropropulsion_adds_the_exact_reverse_vector_for_every_facing_then_stops_all_velocity_fields() {
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            for facing in 0..16 {
                ship.ship_mut().ship_facing = facing;
                ship.ship_mut().cur_status_flags = (SPECIAL
                    | SHIP_AT_MAX_SPEED
                    | SHIP_BEYOND_MAX_SPEED
                    | SHIP_IN_GRAVITY_WELL
                    | WEAPON) as u16;
                ship.ship_mut().special_counter = 17;
                ship.descriptor_mut().ship_info.energy_level = 30;
                (battle::get_mut(ship.id).velocity).set_components(123, -456);
                let mut expected = battle::get(ship.id).velocity;
                let a = facing_to_angle(i32::from(facing)) + 32;
                (expected)
                    .delta_components(i32::from(cosine(a, 160 * 32)), i32::from(sine(a, 160 * 32)));
                observation::enable();
                observation::drain();
                with_rng(8, 0, || ship.preprocess());
                assert_eq!(special_hooks::count(&observation::drain(), "umgah"), 1);
                assert_eq!(
                    velocity_fields(&battle::get(ship.id).velocity),
                    velocity_fields(&expected)
                );
                assert_eq!(ship.descriptor_mut().ship_info.energy_level, 29);
                assert_eq!(ship.ship_mut().energy_counter, 150);
                assert_eq!(
                    ship.ship_mut().special_counter,
                    2,
                    "special counter does not gate retropropulsion"
                );
                assert_eq!(
                    ship.ship_mut().cur_status_flags,
                    (SPECIAL | SHIP_IN_GRAVITY_WELL | WEAPON) as u16
                );
                with_rng(8, 0, || ship.postprocess());
                assert_eq!(ship.ship_mut().special_counter, 0);
                assert_eq!(velocity_fields(&battle::get(ship.id).velocity), [0; 9]);
            }
            ship.ship_mut().ship_facing = 0;
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            observation::drain();
            ship.preprocess();
            assert_eq!(special_hooks::count(&observation::drain(), "umgah"), 1);
            assert_eq!(velocity(ship.id), (0, 160 * 32));
        }
    })
}

#[test]
fn retropropulsion_gates_on_appearance_thrust_wait_key_and_energy() {
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            for (appearing, wait, key, energy, low) in [
                (true, 0, true, 30, false),
                (false, 1, true, 30, false),
                (false, 255, true, 0, false),
                (false, 0, false, 30, false),
                (false, 0, true, 0, true),
            ] {
                battle::get_mut(ship.id).state_flags =
                    (PLAYER_SHIP | if appearing { APPEARING } else { 0 }) as u16;
                battle::get_mut(ship.id).thrust = wait;
                ship.ship_mut().cur_status_flags = if key { SPECIAL as u16 } else { 0 };
                ship.ship_mut().special_counter = 0;
                ship.ship_mut().energy_counter = 19;
                ship.descriptor_mut().ship_info.energy_level = energy;
                (battle::get_mut(ship.id).velocity).set_components(333, -555);
                let before = velocity_fields(&battle::get(ship.id).velocity);
                observation::enable();
                observation::drain();
                with_rng(4, 0, || ship.preprocess());
                assert_eq!(special_hooks::count(&observation::drain(), "umgah"), 0);
                assert_eq!(velocity_fields(&battle::get(ship.id).velocity), before);
                assert_eq!(ship.descriptor_mut().ship_info.energy_level, energy);
                assert_eq!(ship.ship_mut().special_counter, 0);
                assert_eq!(ship.ship_mut().energy_counter, 19);
                assert_eq!(ship.ship_mut().cur_status_flags & (1 << 5) != 0, low);
                ship.postprocess();
                assert_eq!(
                    velocity_fields(&battle::get(ship.id).velocity),
                    before,
                    "counter zero coasts unchanged"
                );
            }
            battle::get_mut(ship.id).state_flags = PLAYER_SHIP as u16;
            battle::get_mut(ship.id).thrust = 0;
            ship.descriptor_mut().ship_info.energy_level = 1;
            ship.preprocess();
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 0);
            assert_eq!(ship.ship_mut().special_counter, 2);
            assert_eq!(ship.ship_mut().cur_status_flags & (1 << 5), 0);
        }
    })
}

#[test]
fn intelligence_filters_enticing_weapons_before_the_shared_pass() {
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let threat = Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 1, [16, 16, 48], [4, 4]);
            for (turn, old, flags, keep, movement) in [
                (4, 0, FINITE_LIFE, false, MOVEMENT_STATE_ENTICE),
                (3, SPECIAL, FINITE_LIFE, false, MOVEMENT_STATE_ENTICE),
                (3, 0, FINITE_LIFE, true, MOVEMENT_STATE_AVOID),
                (3, 0, FINITE_LIFE | CREW_OBJECT, true, MOVEMENT_STATE_PURSUE),
                (1, 0, 0, true, MOVEMENT_STATE_PURSUE),
            ] {
                ship.ship_mut().old_status_flags = old as u16;
                battle::get_mut(threat.id).state_flags = flags as u16;
                let mut c = concerns();
                let w = &mut c[ENEMY_WEAPON_INDEX as usize];
                w.object = Some(threat.id);
                w.which_turn = turn;
                w.movement = MOVEMENT_STATE_ENTICE;
                with_rng(7, 0, || think(&mut ship, &mut c, SPECIAL as u8, 0));
                assert_eq!(!c[ENEMY_WEAPON_INDEX as usize].object.is_none(), keep);
                assert_eq!(c[ENEMY_WEAPON_INDEX as usize].movement, movement);
            }
            let mut c = concerns();
            c[ENEMY_WEAPON_INDEX as usize].object = Some(threat.id);
            c[ENEMY_WEAPON_INDEX as usize].movement = MOVEMENT_STATE_AVOID;
            with_rng(7, 0, || think(&mut ship, &mut c, 0, 0));
            assert_eq!(c[ENEMY_WEAPON_INDEX as usize].object, Some(threat.id));
            assert_eq!(
                c[ENEMY_WEAPON_INDEX as usize].movement,
                MOVEMENT_STATE_AVOID
            );
        }
    })
}

#[test]
fn short_range_ai_resets_special_wait_and_fires_only_before_turn_16() {
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let enemy = Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 1, [16, 16, 48], [4, 4]);
            for reason in 0..3 {
                for turn in [15, 16] {
                    ship.ship_mut().special_counter = if reason == 0 { 1 } else { 0 };
                    let mut c = enemy_concern(&enemy, turn);
                    if reason == 1 {
                        c[GRAVITY_MASS_INDEX as usize].object = Some(enemy.id);
                    }
                    if reason == 2 {
                        c[0].object = None;
                    }
                    let input = with_rng(4, 0, || {
                        think(
                            &mut ship,
                            &mut c,
                            (SPECIAL | THRUST | LEFT | WEAPON) as u8,
                            0,
                        )
                    });
                    assert_eq!(
                        input,
                        (THRUST | LEFT | if turn < 16 { WEAPON } else { 0 }) as u8
                    );
                    assert_eq!(ship.descriptor_mut().cyborg_control.weapon_range, 200);
                    assert_eq!(ship.descriptor_mut().characteristics.special_wait, 255);
                }
            }
        }
    })
}

#[test]
fn rearward_zip_uses_one_draw_with_both_random_turn_directions() {
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let enemy = Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 1, [16, 16, 48], [4, 4]);
            battle::get_mut(enemy.id).next.location = Point { x: 4000, y: 4400 };
            for odd in [false, true] {
                let seed = seed_where(|d| (d[0] & 1 != 0) == odd);
                let mut c = enemy_concern(&enemy, 8);
                let input = with_rng(seed, 1, || think(&mut ship, &mut c, THRUST as u8, 0));
                assert_eq!(input, (SPECIAL | if odd { LEFT } else { RIGHT }) as u8);
                assert_eq!(ship.descriptor_mut().characteristics.special_wait, 8);
                assert_eq!(ship.descriptor_mut().cyborg_control.weapon_range, 32000);
            }
        }
    })
}

#[test]
fn ai_rng_follows_shared_steering_and_uses_old_turn_flags_for_alignment() {
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let enemy = Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 1, [16, 16, 48], [4, 4]);
            battle::get_mut(enemy.id).current.location = Point { x: 4000, y: 4400 };
            battle::get_mut(enemy.id).next = battle::get(enemy.id).current.clone();
            ship.ship_mut().weapon_counter = 1;
            for first_odd in [false, true] {
                let seed =
                    seed_where(|d| (d[0] & 1 != 0) == first_odd && (d[1] & 1 != 0) != first_odd);
                ship.ship_mut().ship_facing = 0;
                battle::get_mut(ship.id).current.image.frame = ship.frame(0, 0);
                battle::get_mut(ship.id).next = battle::get(ship.id).current.clone();
                let mut c = enemy_concern(&enemy, 8);
                let input = with_rng(seed, 2, || think(&mut ship, &mut c, 0, 1));
                assert_eq!(ship.ship_mut().ship_facing, if first_odd { 15 } else { 1 });
                assert_eq!(
                    input,
                    (SPECIAL | LEFT | RIGHT) as u8,
                    "second draw is Umgah's turn, after the shared turn"
                );
                assert_eq!(ship.descriptor_mut().characteristics.special_wait, 8);
            }
            ship.ship_mut().ship_facing = 0;
            ship.ship_mut().old_status_flags = LEFT as u16;
            let mut c = enemy_concern(&enemy, 8);
            assert_eq!(
                with_rng(8, 0, || think(&mut ship, &mut c, THRUST as u8, 0)),
                SPECIAL as u8
            );
        }
    })
}

#[test]
fn rear_zip_alignment_and_randomness_change_at_turns_8_and_16() {
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let enemy = Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 1, [16, 16, 48], [4, 4]);
            battle::get_mut(enemy.id).next.location = Point { x: 4000, y: 4400 };
            for (turn, wait, old, boost, used) in [
                (8, 0, 0, true, 1),
                (9, 0, 0, true, 0),
                (8, 1, 0, true, 0),
                (8, 0, LEFT, true, 0),
                (15, 1, 0, true, 0),
                (16, 1, 0, false, 0),
                (16, 0, LEFT, false, 0),
                (16, 0, 0, true, 0),
            ] {
                ship.ship_mut().old_status_flags = old as u16;
                battle::get_mut(ship.id).turn = wait;
                let mut c = enemy_concern(&enemy, turn);
                let input = with_rng(99, used, || think(&mut ship, &mut c, THRUST as u8, 0));
                assert_eq!(
                    input & SPECIAL as u8 != 0,
                    boost,
                    "turn {turn}, wait {wait}, old {old}"
                );
                assert_eq!(input & THRUST as u8, 0);
                assert_eq!(
                    ship.descriptor_mut().characteristics.special_wait,
                    if boost { turn as u8 } else { 255 }
                );
            }
        }
    })
}

#[test]
fn ai_energy_test_is_strict_and_uses_world_to_turn_shift() {
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let enemy = Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 1, [16, 16, 48], [4, 4]);
            battle::get_mut(enemy.id).next.location = Point { x: 4000, y: 4400 };
            battle::get_mut(ship.id).turn = 1;
            for (energy, turn, boost) in [
                (0, 0, false),
                (1, 1, true),
                (1, 2, false),
                (2, 4, true),
                (2, 5, false),
                (3, 6, true),
                (3, 7, false),
            ] {
                ship.descriptor_mut().ship_info.energy_level = energy;
                let mut c = enemy_concern(&enemy, turn);
                let input = with_rng(6, 0, || think(&mut ship, &mut c, THRUST as u8, 0));
                assert_eq!(input & SPECIAL as u8 != 0, boost);
                assert_eq!(
                    ship.descriptor_mut().cyborg_control.weapon_range,
                    if boost { 32000 } else { 200 }
                );
                assert_eq!(input & THRUST as u8 != 0, !boost);
            }
        }
    })
}

#[test]
fn ai_uses_enemy_maneuverability_and_inclusive_16_through_24_range() {
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let mut enemy =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 1, [16, 16, 48], [4, 4]);
            battle::get_mut(enemy.id).next.location = Point { x: 4000, y: 3600 };
            for (turn, maneuver, long) in [
                (6, 25, false),
                (7, 25, true),
                (7, 26, false),
                (15, 26, false),
                (16, 26, true),
                (24, 26, true),
                (25, 26, false),
                (25, 25, true),
            ] {
                enemy.descriptor_mut().cyborg_control.maneuverability_index = maneuver;
                let mut c = enemy_concern(&enemy, turn);
                let input = with_rng(9, 0, || think(&mut ship, &mut c, THRUST as u8, 0));
                assert_eq!(
                    ship.descriptor_mut().cyborg_control.weapon_range,
                    if long { 32000 } else { 200 }
                );
                assert_eq!(input & THRUST as u8 != 0, !long);
                assert_eq!(input & WEAPON as u8 != 0, turn < 16);
                assert_eq!(input & SPECIAL as u8, 0);
                assert_eq!(ship.descriptor_mut().characteristics.special_wait, 255);
            }
        }
    })
}

#[test]
fn active_zip_continues_on_nonincreasing_turns_then_exits_with_thrust_and_no_steering() {
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let enemy = Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 1, [16, 16, 48], [4, 4]);
            battle::get_mut(enemy.id).next.location = Point { x: 4000, y: 3600 };
            ship.ship_mut().old_status_flags = SPECIAL as u16;
            for (turn, previous, continuing) in [(9, 10, true), (10, 10, true), (11, 10, false)] {
                ship.descriptor_mut().characteristics.special_wait = previous;
                let mut c = enemy_concern(&enemy, turn);
                let input = with_rng(3, 0, || {
                    think(
                        &mut ship,
                        &mut c,
                        (SPECIAL | LEFT | RIGHT | THRUST) as u8,
                        0,
                    )
                });
                assert_eq!(
                    input,
                    if continuing {
                        (SPECIAL | LEFT | RIGHT | WEAPON) as u8
                    } else {
                        (THRUST | WEAPON) as u8
                    }
                );
                assert_eq!(
                    ship.descriptor_mut().characteristics.special_wait,
                    if continuing { turn as u8 } else { 255 }
                );
            }
            ship.descriptor_mut().ship_info.energy_level = 255;
            for turn in [255, 256, u16::MAX] {
                ship.descriptor_mut().characteristics.special_wait = 255;
                let mut c = enemy_concern(&enemy, turn);
                let input = with_rng(3, 0, || think(&mut ship, &mut c, THRUST as u8, 0));
                assert_eq!(input, SPECIAL as u8);
                assert_eq!(
                    ship.descriptor_mut().characteristics.special_wait,
                    255,
                    "turn estimate saturates to a byte"
                );
            }
        }
    })
}

#[test]
fn rear_arc_uses_unwrapped_next_location_difference_and_inclusive_angle_bounds() {
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let enemy = Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 1, [16, 16, 48], [4, 4]);
            battle::get_mut(ship.id).turn = 1;
            for facing in 0..16 {
                ship.ship_mut().ship_facing = facing;
                for angle in 0..64 {
                    let dx = cosine(angle, 1000);
                    let dy = sine(angle, 1000);
                    battle::get_mut(enemy.id).next.location = Point {
                        x: 4000 + dx,
                        y: 4000 + dy,
                    };
                    let direction = arctan(dx, dy);
                    let rear =
                        ((i32::from(direction) - (i32::from(facing) * 4 + 32) + 10) & 63) <= 20;
                    let mut c = enemy_concern(&enemy, 7);
                    let input = with_rng(11, 0, || think(&mut ship, &mut c, 0, 0));
                    assert_eq!(
                        input & SPECIAL as u8 != 0,
                        rear,
                        "facing {facing}, angle {direction}"
                    );
                    assert_eq!(input & WEAPON as u8 != 0, !rear);
                }
            }
            ship.ship_mut().ship_facing = 0;
            battle::get_mut(ship.id).next.location = Point {
                x: 4000,
                y: i16::MAX - 1,
            };
            battle::get_mut(enemy.id).next.location = Point {
                x: 4000,
                y: i16::MIN + 1,
            };
            let mut c = enemy_concern(&enemy, 7);
            assert_eq!(
                with_rng(1, 0, || think(&mut ship, &mut c, 0, 0)),
                SPECIAL as u8
            );
        }
    })
}

fn fire(ship: &mut Ship<'_>) -> Option<ElementId> {
    let h = ship.initialize_from(ship.id, 1)[0];
    if let Some(h) = h {
        assert!(battle::get(h).parent == uqm_engine::battle::Parent::None);
        battle::get_mut(h).parent = battle::get(ship.id).parent;
    }
    h
}
fn previous_facing(ship: &mut Ship<'_>) -> Option<u16> {
    let uqm_engine::battle::RaceState::Umgah { previous_facing } = ship.descriptor_mut().race_state
    else {
        panic!("Umgah fixture state")
    };
    previous_facing
}

#[path = "support/hooks.rs"]
mod hooks;
