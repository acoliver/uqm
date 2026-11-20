//! Chenjesu fixtures retain the original integer, order and RNG expectations.
#![forbid(unsafe_code)]

#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
use battle::{
    queued, toward as velocity_toward, velocity as velocity_fields, with_rng, Battle, Object,
    PoolElement, Ship,
};
use uqm_rust::engine::battle::ai_constants::ENEMY_SHIP_INDEX;
use uqm_rust::engine::battle::ai_constants::ENEMY_WEAPON_INDEX;
use uqm_rust::engine::battle::ai_constants::FIRES_FORE;
use uqm_rust::engine::battle::ai_constants::IGNORE_SIMILAR;
use uqm_rust::engine::battle::ai_constants::MOVEMENT_STATE_AVOID;
use uqm_rust::engine::battle::ai_constants::MOVEMENT_STATE_ENTICE;
use uqm_rust::engine::battle::ai_constants::MOVEMENT_STATE_PURSUE;
use uqm_rust::engine::battle::ai_constants::SEEKING_SPECIAL;
use uqm_rust::engine::battle::ai_constants::SEEKING_WEAPON;
use uqm_rust::engine::battle::ai_constants::SPECIAL;
use uqm_rust::engine::battle::ai_constants::WEAPON;
use uqm_rust::engine::battle::flags::APPEARING;
use uqm_rust::engine::battle::flags::CHANGING;
use uqm_rust::engine::battle::flags::COLLISION;
use uqm_rust::engine::battle::flags::DEFY_PHYSICS;
use uqm_rust::engine::battle::flags::DISAPPEARING;
use uqm_rust::engine::battle::flags::FINITE_LIFE;
use uqm_rust::engine::battle::flags::NONSOLID;
use uqm_rust::engine::battle::Concern;
use uqm_rust::engine::battle::ElementId;
use uqm_rust::engine::battle::EMPTY_PRIMITIVE;
use uqm_rust::engine::battle::STAMP_PRIMITIVE;
use uqm_rust::engine::species::species_id::SPECIES_ID_CHENJESU_ID;
use uqm_rust::graphics::geometry::Point;
use uqm_rust::math::arctan;
use uqm_rust::math::battle_types::HALF_CIRCLE;
use uqm_rust::math::battle_types::ONE_SHIFT;
use uqm_rust::math::random;
use uqm_rust::math::seed_random;
use uqm_rust::math::velocity::VELOCITY_SHIFT;
use uqm_rust::ships::native::{angle_to_facing, cosine, facing_to_angle, normalize_facing, sine};

fn crystal(ship: &mut Ship<'_>) -> ElementId {
    {
        crate::battle_context::with(|cx| {
            let origin = cx.weapon_origin(ship.id);
            let out = cx.spawn_weapons(ship.ship_id, origin);
            assert_eq!(out.len(), 1);
            let id = out[0].unwrap_or_else(|| panic!("crystal allocation"));
            assert_eq!(cx.parent(id), uqm_rust::engine::battle::Parent::None);
            cx.set_parent(id, Some(ship.ship_id));
            cx.copy_visual_to_next(id, cx.element(id).current);
            cx.set_intersect_frame(id, cx.element(id).current.image.frame);
            id
        })
    }
}
fn doggy(ship: &mut Ship<'_>) -> ElementId {
    {
        ship.ship_mut().cur_status_flags = SPECIAL as u16;
        ship.ship_mut().special_counter = 0;
        ship.descriptor_mut().ship_info.energy_level = 30;
        let old_tail = battle::tail();
        ship.postprocess();
        let tail = battle::tail();
        assert_ne!(tail, old_tail);
        let dog = tail.unwrap();
        battle::get_mut(dog).next = battle::get(dog).current.clone();
        dog
    }
}
fn at(ship: &mut Ship<'_>, x: i16, y: i16) {
    {
        ship.element_mut().current.location = Point { x, y };
        ship.element_mut().next.location = Point { x, y };
    }
}
fn intelligence(ship: &mut Ship<'_>, concerns: &mut [Concern; 4]) {
    {
        ship.think(ship.id, concerns, 0);
    }
}

fn sentinel(_: &mut uqm_rust::engine::battle::Cx<'_>, _: uqm_rust::engine::battle::ElementId) {}

#[test]
fn descriptor_matches_every_c_initializer_field() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship = Ship::bare(&_battle, SPECIES_ID_CHENJESU_ID, 0);
            let d = ship.descriptor_mut();
            let race = d
                .race
                .unwrap_or_else(|| panic!("Native Chenjesu descriptor"));
            assert_eq!(
                d.ship_info.ship_flags,
                (FIRES_FORE | SEEKING_SPECIAL | SEEKING_WEAPON) as u16
            );
            assert_eq!(d.ship_info.ship_cost, 28);
            assert_eq!((d.ship_info.crew_level, d.ship_info.max_crew), (36, 36));
            assert_eq!((d.ship_info.energy_level, d.ship_info.max_energy), (30, 30));
            assert_eq!(d.ship_info.race_strings_rsc.unwrap(), c"ship.chenjesu.text");
            assert_eq!(d.ship_info.icons_rsc.unwrap(), c"ship.chenjesu.icons");
            assert_eq!(
                d.ship_info.melee_icon_rsc.unwrap(),
                c"ship.chenjesu.meleeicons"
            );
            assert!(d.ship_info.race_strings.is_none());
            assert!(d.ship_info.icons.is_null());
            assert!(d.ship_info.melee_icon.is_null());
            assert_eq!(d.fleet.strength, 0);
            assert_eq!((d.fleet.known_loc.x, d.fleet.known_loc.y), (0, 0));
            let c = d.characteristics;
            assert_eq!(
                (c.max_thrust, c.thrust_increment, c.energy_regeneration),
                (27, 3, 1)
            );
            assert_eq!(
                (c.weapon_energy_cost, c.special_energy_cost, c.energy_wait),
                (5, 30, 4)
            );
            assert_eq!(
                (
                    c.turn_wait,
                    c.thrust_wait,
                    c.weapon_wait,
                    c.special_wait,
                    c.ship_mass
                ),
                (6, 4, 0, 0, 10)
            );
            for (i, size) in ["large", "medium", "small"].iter().enumerate() {
                assert_eq!(
                    d.ship_data.ship_rsc[i].unwrap().to_str().unwrap(),
                    format!("ship.chenjesu.graphics.broodhome.{size}")
                );
                assert_eq!(
                    d.ship_data.weapon_rsc[i].unwrap().to_str().unwrap(),
                    format!("ship.chenjesu.graphics.spark.{size}")
                );
                assert_eq!(
                    d.ship_data.special_rsc[i].unwrap().to_str().unwrap(),
                    format!("ship.chenjesu.graphics.doggy.{size}")
                );
                assert!(d.ship_data.ship.view(i).is_absent());
                assert!(d.ship_data.weapon.view(i).is_absent());
                assert!(d.ship_data.special.view(i).is_absent());
            }
            let captain = &d.ship_data.captain_control;
            assert_eq!(
                captain.captain_rsc.unwrap(),
                c"ship.chenjesu.graphics.captain"
            );
            assert!(captain.background.is_null());
            assert!(captain.turn.is_null());
            assert!(captain.thrust.is_null());
            assert!(captain.weapon.is_null());
            assert!(captain.special.is_absent());
            assert_eq!(
                d.ship_data.victory_ditty_rsc.unwrap(),
                c"ship.chenjesu.ditty"
            );
            assert_eq!(
                d.ship_data.ship_sounds_rsc.unwrap(),
                c"ship.chenjesu.sounds"
            );
            assert!(d.ship_data.victory_ditty.is_null());
            assert!(d.ship_data.ship_sounds.is_none());
            assert_eq!(d.cyborg_control.maneuverability_index, 0);
            assert_eq!(d.cyborg_control.weapon_range, 4000);

            assert!(race.intelligence.is_some());

            assert!(race.preprocess.is_some());

            assert!(race.postprocess.is_some());

            assert!(race.init_weapon.is_some());

            assert!(race.uninit.is_none());
            drop(d);
            let fresh = battle::descriptor(SPECIES_ID_CHENJESU_ID);
            assert!(std::ptr::eq(
                fresh.race.unwrap_or_else(|| panic!("fresh Native race")),
                race
            ));
        }
    })
}

#[test]
fn crystal_launch_hold_release_and_count_truncation_match_c() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            ship.ship_mut().ship_facing = 7;
            at(&mut ship, 321, -654);
            let h = crystal(&mut ship);
            let crystal = h;
            assert_eq!(battle::get(crystal).life, 90);
            assert_eq!(battle::get(crystal).hits, 10);
            assert_eq!(battle::get(crystal).mass, 6);
            assert_eq!(battle::get(crystal).thrust, 0);
            assert_eq!(
                battle::get(crystal).state_flags,
                (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
            );
            assert_eq!(battle::get(crystal).player_nr, 0);
            assert_eq!(
                battle::get(crystal).current.image.frame.raw(),
                (ship.descriptor_mut().ship_data.weapon.view(0).raw()).at(0)
            );
            assert_eq!(battle::get(crystal).current.image.frames, ship.frame_set(1));
            assert_eq!(
                velocity_fields(&battle::get(crystal).velocity),
                velocity_toward(64, 7)
            );
            let angle = facing_to_angle(7);
            let vx = cosine(angle, 64 << VELOCITY_SHIFT);
            let vy = sine(angle, 64 << VELOCITY_SHIFT);
            assert_eq!(
                battle::get(crystal).current.location.x,
                (321 + i32::from(cosine(angle, 64)) - (i32::from(vx) >> VELOCITY_SHIFT)) as i16
            );
            assert_eq!(
                battle::get(crystal).current.location.y,
                (-654 + i32::from(sine(angle, 64)) - (i32::from(vy) >> VELOCITY_SHIFT)) as i16
            );
            assert!(battle::get(crystal).collision_func.is_some());
            assert!(battle::get(crystal).postprocess_func.is_none());
            let preprocess = battle::get(crystal).preprocess_func.unwrap();
            ship.ship_mut().cur_status_flags = WEAPON as u16;
            with_rng(21, 0, || hooks::invoke_element(preprocess, crystal));
            assert_eq!(battle::get(crystal).life, 91);
            battle::get_mut(crystal).life = u16::MAX;
            hooks::invoke_element(preprocess, crystal);
            assert_eq!(battle::get(crystal).life, 0);
            ship.ship_mut().cur_status_flags = 0;
            with_rng(21, 0, || hooks::invoke_element(preprocess, crystal));
            assert_eq!(battle::get(crystal).life, 1);
            let release = battle::get(crystal).postprocess_func.unwrap().address();
            ship.ship_mut().cur_status_flags = WEAPON as u16;
            hooks::invoke_element(preprocess, crystal);
            assert_eq!(battle::get(crystal).life, 2);
            assert_eq!(
                battle::get(crystal).postprocess_func.unwrap().address(),
                release
            );
            battle_context::dispose(h);
        }
    })
}

#[test]
fn released_crystal_queues_eight_fragments_in_facing_order() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 1, [16, 10, 6], [4, 4]);
            let h = crystal(&mut ship);
            let crystal = h;
            battle::get_mut(crystal).next.location = Point { x: 5000, y: 6000 };
            hooks::invoke_element(battle::get(crystal).preprocess_func.unwrap(), crystal);
            let before = queued().len();
            with_rng(90, 0, || {
                hooks::invoke_element(battle::get(crystal).postprocess_func.unwrap(), crystal)
            });
            let all = queued();
            assert_eq!(all.len(), before + 8);
            for (i, &fragment_handle) in all[before..].iter().enumerate() {
                let fragment = fragment_handle;
                let face = i as u16 * 2;
                assert_eq!(
                    battle::get(fragment).parent,
                    uqm_rust::engine::battle::Parent::Ship(ship.ship_id)
                );
                assert_eq!(battle::get(fragment).player_nr, 1);
                assert_eq!(
                    battle::get(fragment).state_flags,
                    (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
                );
                assert_eq!(battle::get(fragment).life, 10);
                assert_eq!(battle::get(fragment).hits, 1);
                assert_eq!(battle::get(fragment).mass, 2);
                assert_eq!(battle::get(fragment).thrust, 2);
                assert_eq!(
                    battle::get(fragment).current.image.frame.raw(),
                    (ship.descriptor_mut().ship_data.weapon.view(0).raw()).at(1)
                );
                assert_eq!(
                    battle::get(fragment).current.image.frames,
                    battle::get(crystal).next.image.frames
                );
                assert!(battle::get(fragment).preprocess_func.is_none());
                assert!(battle::get(fragment).postprocess_func.is_none());
                assert!(battle::get(fragment).death_func.is_none());
                assert!(battle::get(fragment).collision_func.is_some());
                assert_eq!(
                    velocity_fields(&battle::get(fragment).velocity),
                    velocity_toward(64, face)
                );
                let a = facing_to_angle(i32::from(face));
                let dx = i32::from(cosine(a, 64 << VELOCITY_SHIFT)) >> VELOCITY_SHIFT;
                let dy = i32::from(sine(a, 64 << VELOCITY_SHIFT)) >> VELOCITY_SHIFT;
                assert_eq!(
                    (
                        battle::get(fragment).current.location.x,
                        battle::get(fragment).current.location.y
                    ),
                    ((5000 - dx) as i16, (6000 - dy) as i16)
                );
                assert_eq!(battle::pred(fragment), Some(all[before + i - 1]));
            }
            assert!(battle::succ(battle::tail().unwrap()).is_none());
            battle_context::dispose(h);
        }
    })
}

#[test]
fn allocation_failure_preserves_weapon_count_and_partial_fragment_order() {
    battle_context::run(|_| {
        let _battle = Battle::new(4);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let crystal_handle = crystal(&mut ship);
            let crystal = crystal_handle;
            hooks::invoke_element(battle::get(crystal).preprocess_func.unwrap(), crystal);
            with_rng(3, 0, || {
                hooks::invoke_element(battle::get(crystal).postprocess_func.unwrap(), crystal)
            });
            let all = queued();
            assert_eq!(all.len(), 3);
            assert_eq!(
                velocity_fields(&battle::get(all[1]).velocity),
                velocity_toward(64, 0)
            );
            assert_eq!(
                velocity_fields(&battle::get(all[2]).velocity),
                velocity_toward(64, 2)
            );
            let out = ship.weapon(1);
            assert_eq!(out, vec![None]);
            battle_context::dispose(crystal_handle);
        }
    })
}

#[test]
fn crystal_collision_creates_sparkles_at_the_target_and_animates_current_frame() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let h = crystal(&mut ship);
            let crystal = h;
            let target = PoolElement::new();
            battle::get_mut(target.id).player_nr = 1;
            battle::get_mut(target.id).current.location = Point { x: 1234, y: -4321 };
            // Not NORMAL_LIFE: the generic weapon collision must not damage it.
            battle::get_mut(target.id).life = 2;
            let mut pa = Point { x: 20, y: 30 };
            let mut pb = Point { x: 40, y: 50 };
            let collide = battle::get(crystal).collision_func.unwrap();
            with_rng(17, 0, || {
                hooks::invoke_collision(collide, crystal, &mut pa, target.id, &mut pb)
            });
            let sparkle = battle::tail().unwrap();
            assert_ne!(sparkle, ship.id);
            assert_eq!(
                (
                    battle::get(sparkle).current.location.x,
                    battle::get(sparkle).current.location.y
                ),
                (1234, -4321)
            );
            assert_eq!(battle::get(sparkle).life, 8);
            assert_eq!(battle::get(sparkle).turn, 0);
            assert_eq!(battle::get(sparkle).thrust, 0);
            assert_eq!(
                battle::get(sparkle).current.image.frames,
                battle::get(crystal).next.image.frames
            );
            assert_eq!(
                battle::get(sparkle).current.image.frame.raw(),
                (ship.descriptor_mut().ship_data.weapon.view(0).raw()).at(2)
            );
            assert_eq!(battle::get(crystal).life, 0);
            assert_eq!(battle::get(crystal).hits, 0);
            assert_eq!(
                battle::get(crystal).state_flags & (COLLISION | NONSOLID | DISAPPEARING) as u16,
                (COLLISION | NONSOLID | DISAPPEARING) as u16
            );
            let animate = battle::get(sparkle).preprocess_func.unwrap();
            battle::get_mut(sparkle).next.image.frame = ship.frame(1, 8);
            with_rng(17, 0, || hooks::invoke_element(animate, sparkle));
            assert_eq!((battle::get(sparkle).next.image.frame.raw()).index(), 3);
            assert_ne!(battle::get(sparkle).state_flags & CHANGING as u16, 0);
            battle::get_mut(sparkle).turn = 2;
            battle::get_mut(sparkle).thrust = 3;
            battle::get_mut(sparkle).state_flags &= !(CHANGING as u16);
            hooks::invoke_element(animate, sparkle);
            assert_eq!(battle::get(sparkle).turn, 1);
            assert_eq!(battle::get(sparkle).state_flags & CHANGING as u16, 0);
            hooks::invoke_element(animate, sparkle);
            assert_eq!(battle::get(sparkle).turn, 0);
            battle::get_mut(sparkle).current.image.frame = ship.frame(1, 9);
            hooks::invoke_element(animate, sparkle);
            assert_eq!(
                battle::get(sparkle).next.image.frame.raw(),
                (ship.descriptor_mut().ship_data.weapon.view(0).raw()).at(0)
            );
            assert_eq!(battle::get(sparkle).turn, 3);
            let tail = battle::tail();
            hooks::invoke_collision(collide, crystal, &mut pa, target.id, &mut pb);
            assert_eq!(
                { battle::tail() },
                tail,
                "already-collided crystal creates no second blast"
            );
            battle_context::dispose(h);
        }
    })
}

#[test]
fn doggy_spawns_behind_ship_with_exact_state_and_coordinate_truncation() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            for facing in [0, 4, 7, 8, 15, u16::MAX] {
                ship.ship_mut().ship_facing = facing;
                at(&mut ship, i16::MAX - 10, i16::MIN + 10);
                let dog = with_rng(7, 0, || doggy(&mut ship));
                let angle = (facing_to_angle(i32::from(facing)) + HALF_CIRCLE as i32) as u16;
                let expected_facing = normalize_facing(angle_to_facing(i32::from(angle)));
                assert_eq!(battle::get(dog).player_nr, 0);
                assert_eq!(battle::get(dog).state_flags, APPEARING as u16);
                assert_eq!(battle::get(dog).life, 1);
                assert_eq!(battle::get(dog).hits, 3);
                assert_eq!(battle::get(dog).mass, 4);
                assert_eq!(battle::get(dog).thrust, 0);
                assert_eq!(battle::prim(dog).kind, STAMP_PRIMITIVE);
                assert_eq!(battle::get(dog).current.image.frames, ship.frame_set(2));
                assert_eq!(
                    battle::get(dog).current.image.frame.raw(),
                    (ship.descriptor_mut().ship_data.special.view(0).raw()).at(0)
                );
                assert_eq!(
                    battle::get(dog).parent,
                    uqm_rust::engine::battle::Parent::Ship(ship.ship_id)
                );
                assert!(battle::get(dog).target.is_none());
                assert!(battle::get(dog).preprocess_func.is_some());
                assert!(battle::get(dog).postprocess_func.is_none());
                assert!(battle::get(dog).collision_func.is_some());
                assert!(battle::get(dog).death_func.is_some());
                assert_eq!(
                    velocity_fields(&battle::get(dog).velocity),
                    velocity_toward(32, expected_facing)
                );
                let dx = cosine(i32::from(angle), 34 << ONE_SHIFT);
                let dy = sine(i32::from(angle), 34 << ONE_SHIFT);
                assert_eq!(
                    battle::get(dog).current.location.x,
                    ((i32::from(i16::MAX) - 10) + i32::from(dx)) as i16
                );
                assert_eq!(
                    battle::get(dog).current.location.y,
                    ((i32::from(i16::MIN) + 10) + i32::from(dy)) as i16
                );
                assert_ne!(battle::get(ship.id).state_flags & DEFY_PHYSICS as u16, 0);
                assert_eq!(ship.descriptor_mut().ship_info.energy_level, 0);
                assert_eq!(ship.ship_mut().energy_counter, 4);
                assert_eq!(ship.ship_mut().special_counter, 1);
            }
        }
    })
}

#[test]
fn special_checks_key_counter_and_energy_but_always_resets_the_counter() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            for (key, counter, energy, spawn, low) in [
                (false, 0, 30, false, false),
                (true, 3, 30, true, false),
                (true, 4, 30, false, false),
                (true, 255, 30, false, false),
                (true, 0, 29, false, true),
                (true, 0, 0, false, true),
            ] {
                let before = queued().len();
                ship.ship_mut().cur_status_flags = if key { SPECIAL as u16 } else { 0 };
                ship.ship_mut().special_counter = counter;
                ship.descriptor_mut().ship_info.energy_level = energy;
                battle::get_mut(ship.id).state_flags &= !(DEFY_PHYSICS as u16);
                with_rng(1, 0, || ship.postprocess());
                assert_eq!(queued().len(), before + usize::from(spawn));
                assert_eq!(ship.ship_mut().special_counter, 1);
                assert_eq!(
                    ship.descriptor_mut().ship_info.energy_level,
                    if spawn { 0 } else { energy }
                );
                assert_eq!(ship.ship_mut().cur_status_flags & (1 << 5) != 0, low);
                assert_eq!(
                    battle::get(ship.id).state_flags & DEFY_PHYSICS as u16 != 0,
                    spawn
                );
            }
        }
    })
}

#[test]
fn failed_doggy_allocation_still_spends_the_energy() {
    battle_context::run(|_| {
        let _battle = Battle::new(1);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.postprocess();
            assert_eq!(queued(), vec![ship.id]);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 0);
            assert_eq!(ship.ship_mut().energy_counter, 4);
            assert_eq!(ship.ship_mut().special_counter, 1);
            assert_eq!(battle::get(ship.id).state_flags & DEFY_PHYSICS as u16, 0);
        }
    })
}

#[test]
fn ship_preprocess_matches_byte_promotion_and_wrap_boundaries() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);

            for (special, expected) in [(0, 0), (1, 1), (2, 6), (251, 255), (252, 0), (255, 3)] {
                ship.ship_mut().special_counter = special;
                ship.ship_mut().weapon_counter = 255;
                ship.ship_mut().cur_status_flags = WEAPON as u16;
                ship.ship_mut().old_status_flags = WEAPON as u16;
                with_rng(5, 0, || ship.preprocess());
                assert_eq!(ship.ship_mut().special_counter, expected);
                assert_eq!(ship.ship_mut().weapon_counter, 0);
            }
            for (current, old) in [(0, 0), (WEAPON, 0), (0, WEAPON), (SPECIAL, WEAPON)] {
                ship.ship_mut().cur_status_flags = current as u16;
                ship.ship_mut().old_status_flags = old as u16;
                ship.ship_mut().weapon_counter = 13;
                ship.preprocess();
                assert_eq!(ship.ship_mut().weapon_counter, 13);
            }
        }
    })
}

#[test]
fn doggy_collision_drains_only_enemy_ships_and_extends_only_short_waits() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let mut enemy =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 1, [16, 10, 6], [4, 4]);
            let dog = doggy(&mut ship);
            let collide = battle::get(dog).collision_func.unwrap();
            let mut pa = Point { x: 0, y: 0 };
            let mut pb = Point { x: 0, y: 0 };
            for (energy, remaining) in [(0, 0), (9, 0), (10, 0), (11, 1), (30, 20)] {
                enemy.descriptor_mut().ship_info.energy_level = energy;
                enemy.ship_mut().energy_counter = 99;
                battle::get_mut(dog).thrust = 0;
                with_rng(101, 0, || {
                    hooks::invoke_collision(collide, dog, &mut pa, enemy.id, &mut pb)
                });
                assert_eq!(enemy.descriptor_mut().ship_info.energy_level, remaining);
                assert_eq!(
                    enemy.ship_mut().energy_counter,
                    4,
                    "even zero drain calls DeltaEnergy"
                );
                assert_ne!(battle::get(dog).state_flags & COLLISION as u16, 0);
                assert_eq!(battle::get(dog).thrust, 6);
            }
            for (wait, expected) in [(0, 6), (1, 7), (2, 8), (3, 9), (4, 4), (255, 255)] {
                battle::get_mut(dog).thrust = wait;
                ship.descriptor_mut().ship_info.energy_level = 17;
                hooks::invoke_collision(collide, dog, &mut pa, ship.id, &mut pb);
                assert_eq!(battle::get(dog).thrust, expected);
                assert_eq!(ship.descriptor_mut().ship_info.energy_level, 17);
            }
            let fixture = Object::new(&_battle);
            let object = fixture.id;
            battle::get_mut(object).player_nr = 1;
            battle::get_mut(object).state_flags = FINITE_LIFE as u16;
            battle::get_mut(dog).state_flags &= !(COLLISION as u16);
            hooks::invoke_collision(collide, dog, &mut pa, object, &mut pb);
            assert_eq!(battle::get(dog).state_flags & COLLISION as u16, 0);
            // The generic collision runs first and damages the DOGI on planets.
            battle::get_mut(object).state_flags = 0;
            battle::get_mut(object).mass = 101;
            hooks::invoke_collision(collide, dog, &mut pa, object, &mut pb);
            assert_eq!(battle::get(dog).hits, 2);
            assert_ne!(battle::get(dog).state_flags & COLLISION as u16, 0);
        }
    })
}

#[test]
fn doggy_death_preserves_unrelated_fields_and_starts_six_frame_animation() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let dog = doggy(&mut ship);
            battle::get_mut(dog).state_flags |= (DISAPPEARING | COLLISION | DEFY_PHYSICS) as u16;
            battle::get_mut(dog).postprocess_func =
                Some(uqm_rust::engine::battle::ElementHook::Native(sentinel));
            battle::get_mut(dog).target = Some(ship.id);
            battle::get_mut(dog).turn = 9;
            battle::get_mut(dog).thrust = 7;
            let preprocess = battle::get(dog).preprocess_func.unwrap().address();
            let old_flags = battle::get(dog).state_flags;
            let counter = ship.ship_mut().special_counter;
            with_rng(88, 0, || {
                hooks::invoke_element(battle::get(dog).death_func.unwrap(), dog)
            });
            assert_eq!(
                battle::get(dog).state_flags,
                (old_flags & !(DISAPPEARING as u16)) | (NONSOLID | FINITE_LIFE) as u16
            );
            assert_eq!(battle::get(dog).life, 6);
            assert_eq!(velocity_fields(&battle::get(dog).velocity), [0; 9]);
            assert_eq!(battle::get(dog).turn, 0);
            assert_eq!(battle::get(dog).thrust, 0);
            assert!(battle::get(dog).death_func.is_none());
            assert!(battle::get(dog).collision_func.is_none());
            assert_ne!(
                battle::get(dog).preprocess_func.unwrap().address(),
                preprocess
            );
            assert_eq!(
                battle::get(dog).postprocess_func.unwrap().address(),
                sentinel as *const () as usize
            );
            assert_eq!(battle::get(dog).target, Some(ship.id));
            assert_eq!(
                battle::get(dog).parent,
                uqm_rust::engine::battle::Parent::Ship(ship.ship_id)
            );
            assert_eq!(battle::get(dog).hits, 3);
            assert_eq!(battle::get(dog).mass, 4);
            let animate = battle::get(dog).preprocess_func.unwrap();
            for frame in 1..=6 {
                with_rng(88, 0, || hooks::invoke_element(animate, dog));
                assert_eq!((battle::get(dog).next.image.frame.raw()).index(), frame % 6);
                battle::get_mut(dog).current = battle::get(dog).next.clone();
            }
            assert_eq!(
                ship.ship_mut().special_counter,
                counter,
                "dead DOGIs no longer count as active"
            );
        }
    })
}

#[test]
fn doggy_wait_counts_before_tracking_and_random_roaming_consumes_one_draw() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let dog = doggy(&mut ship);
            let preprocess = battle::get(dog).preprocess_func.unwrap();
            uqm_rust::battle::velocity::set_velocity_vector(
                &mut battle::get_mut(dog).velocity,
                11,
                3,
            );
            let original = velocity_fields(&battle::get(dog).velocity);
            battle::get_mut(dog).thrust = 1;
            ship.ship_mut().special_counter = 255;
            with_rng(42, 0, || hooks::invoke_element(preprocess, dog));
            assert_eq!(ship.ship_mut().special_counter, 0);
            assert_eq!(battle::get(dog).thrust, 0);
            assert_eq!(velocity_fields(&battle::get(dog).velocity), original);
            for seed in 1..33 {
                uqm_rust::battle::velocity::set_velocity_vector(
                    &mut battle::get_mut(dog).velocity,
                    11,
                    3,
                );
                seed_random(seed);
                let face = (random() & 15) as u16;
                with_rng(seed, 1, || hooks::invoke_element(preprocess, dog));
                assert_eq!(
                    velocity_fields(&battle::get(dog).velocity),
                    if face == 3 {
                        original
                    } else {
                        velocity_toward(32, face)
                    }
                );
                assert!(battle::get(dog).target.is_none());
            }
        }
    })
}

#[test]
fn doggy_steering_uses_exact_rear_facing_boundaries_not_trackships_single_step() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let enemy = Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 1, [16, 10, 6], [4, 4]);
            let dog = doggy(&mut ship);
            let preprocess = battle::get(dog).preprocess_func.unwrap();
            battle::get_mut(dog).current.location = Point { x: 4000, y: 4500 };
            battle::get_mut(dog).next.location = battle::get(dog).current.location;
            // Enemy due north: raw facing 0, TrackShip's one-step result is 3.
            for (delta, expected) in [(6, 0), (7, 4), (8, 12), (9, 12), (10, 0)] {
                battle::get_mut(enemy.id).current.image.frame = enemy.frame(0, (16 - delta) as u16);
                battle::get_mut(dog).target = None;
                uqm_rust::battle::velocity::set_velocity_vector(
                    &mut battle::get_mut(dog).velocity,
                    11,
                    4,
                );
                let original = velocity_fields(&battle::get(dog).velocity);
                with_rng(99, 0, || hooks::invoke_element(preprocess, dog));
                assert_eq!(battle::get(dog).target, Some(enemy.id));
                assert_eq!(
                    velocity_fields(&battle::get(dog).velocity),
                    if expected == 4 {
                        original
                    } else {
                        velocity_toward(32, expected)
                    },
                    "delta {delta}"
                );
            }
            // TrackShip itself draws once for a half-circle tie. DOGI recomputes
            // its heading afterward, so the random tie-break does not choose it.
            battle::get_mut(enemy.id).current.image.frame = enemy.frame(0, 0);
            for seed in 1..17 {
                uqm_rust::battle::velocity::set_velocity_vector(
                    &mut battle::get_mut(dog).velocity,
                    32,
                    8,
                );
                battle::get_mut(dog).target = Some(enemy.id);
                with_rng(seed, 1, || hooks::invoke_element(preprocess, dog));
                assert_eq!(
                    velocity_fields(&battle::get(dog).velocity),
                    velocity_toward(32, 0)
                );
            }
        }
    })
}

#[test]
fn doggy_uses_current_unwrapped_coordinates_and_truncates_arctan_arguments() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let mut enemy =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 1, [16, 10, 6], [4, 4]);
            let dog = doggy(&mut ship);
            let preprocess = battle::get(dog).preprocess_func.unwrap();
            for (dog_x, enemy_x, expected) in
                [(-30000, 30000, 12), (8000, 100, 12), (30000, -30000, 4)]
            {
                battle::get_mut(dog).current.location = Point { x: dog_x, y: 4000 };
                battle::get_mut(dog).next.location = Point { x: 1000, y: 4000 };
                at(&mut enemy, enemy_x, 4000);
                battle::get_mut(enemy.id).next.location = Point { x: 2000, y: 4000 };
                battle::get_mut(enemy.id).current.image.frame =
                    enemy.frame(0, (expected as usize) as u16);
                battle::get_mut(dog).target = Some(enemy.id);
                uqm_rust::battle::velocity::set_velocity_vector(
                    &mut battle::get_mut(dog).velocity,
                    32,
                    0,
                );
                assert_eq!(
                    normalize_facing(angle_to_facing(i32::from(arctan(
                        (i32::from(enemy_x) - i32::from(dog_x)) as i16,
                        0
                    )))),
                    expected
                );
                with_rng(61, 0, || hooks::invoke_element(preprocess, dog));
                assert_eq!(
                    velocity_fields(&battle::get(dog).velocity),
                    velocity_toward(32, expected)
                );
            }
        }
    })
}

#[test]
fn doggy_acquires_nearest_visible_ship_but_retains_an_existing_cloaked_target() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let mut far =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 1, [16, 10, 6], [4, 4]);
            let mut near =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 1, [16, 10, 6], [4, 4]);
            let dog = doggy(&mut ship);
            battle::get_mut(dog).current.location = Point { x: 4000, y: 4000 };
            battle::get_mut(dog).next = battle::get(dog).current.clone();
            at(&mut near, 4100, 4000);
            at(&mut far, 4000, 3500);
            let preprocess = battle::get(dog).preprocess_func.unwrap();
            uqm_rust::battle::velocity::set_velocity_vector(
                &mut battle::get_mut(dog).velocity,
                32,
                0,
            );
            with_rng(4, 0, || hooks::invoke_element(preprocess, dog));
            assert_eq!(battle::get(dog).target, Some(near.id));
            assert_eq!(
                velocity_fields(&battle::get(dog).velocity),
                velocity_toward(32, 4)
            );
            battle::prim_mut(near.id).kind = EMPTY_PRIMITIVE;
            uqm_rust::battle::velocity::set_velocity_vector(
                &mut battle::get_mut(dog).velocity,
                32,
                0,
            );
            with_rng(4, 0, || hooks::invoke_element(preprocess, dog));
            assert_eq!(
                battle::get(dog).target,
                Some(near.id),
                "TrackShip retains a live prior target even when cloaked"
            );
            battle::get_mut(dog).target = None;
            uqm_rust::battle::velocity::set_velocity_vector(
                &mut battle::get_mut(dog).velocity,
                32,
                0,
            );
            with_rng(4, 0, || hooks::invoke_element(preprocess, dog));
            assert_eq!(battle::get(dog).target, Some(far.id));
            assert_eq!(
                velocity_fields(&battle::get(dog).velocity),
                velocity_toward(32, 0)
            );
        }
    })
}

#[test]
fn ai_pursuit_boundaries_match_turn_maneuverability_range_and_seeking_flags() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let mut enemy =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 1, [16, 10, 6], [4, 4]);
            let cases = [
                (16, 45, 0, false, true),
                (17, 45, 0, false, false),
                (16, 44, 0, false, false),
                (0, 45, 0, false, true),
                (17, 25, 3000, true, true),
                (17, 26, 3000, true, false),
                (17, 25, 2999, true, false),
                (17, 25, 3000, false, false),
                (65535, 0, 4000, true, true),
                (17, 65535, 4000, true, false),
            ];
            // Avoid the launch probe while checking movement-state decisions.
            ship.ship_mut().weapon_counter = 1;
            for (turn, maneuver, range, seeking, pursue) in cases {
                enemy.descriptor_mut().cyborg_control.maneuverability_index = maneuver;
                enemy.descriptor_mut().cyborg_control.weapon_range = range;
                enemy.descriptor_mut().ship_info.ship_flags =
                    if seeking { SEEKING_WEAPON as u16 } else { 0 };
                let mut concerns: [Concern; 4] = [Concern::default(); 4];
                concerns[ENEMY_SHIP_INDEX as usize] = Concern {
                    object: Some(enemy.id),
                    facing: 0,
                    which_turn: turn,
                    movement: MOVEMENT_STATE_ENTICE,
                };
                with_rng(6, 0, || intelligence(&mut ship, &mut concerns));
                assert_eq!(
                    concerns[ENEMY_SHIP_INDEX as usize].movement,
                    if pursue {
                        MOVEMENT_STATE_PURSUE
                    } else {
                        MOVEMENT_STATE_ENTICE
                    },
                    "turn {turn}, maneuver {maneuver}, range {range}, seeking {seeking}"
                );
            }
        }
    })
}

#[test]
fn ai_special_budget_uses_less_than_four_and_energy_less_than_or_equal_to_cost() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let enemy = Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 1, [16, 10, 6], [4, 4]);
            for counter in [0, 1, 3, 4, 255] {
                for energy in [0, 29, 30, 31] {
                    for weapon in [false, true] {
                        ship.ship_mut().weapon_counter = 1;
                        ship.ship_mut().special_counter = counter;
                        ship.descriptor_mut().ship_info.energy_level = energy;
                        ship.ship_mut().ship_input_state =
                            SPECIAL as u8 | if weapon { WEAPON as u8 } else { 0 };
                        let mut concerns: [Concern; 4] = [Concern::default(); 4];
                        concerns[ENEMY_SHIP_INDEX as usize].object = Some(enemy.id);
                        with_rng(81, 0, || intelligence(&mut ship, &mut concerns));
                        assert_eq!(
                            ship.ship_mut().ship_input_state & SPECIAL as u8 != 0,
                            counter < 4 && energy <= 30 && !weapon,
                            "counter {counter}, energy {energy}, weapon {weapon}"
                        );
                    }
                }
            }
            let mut concerns: [Concern; 4] = [Concern::default(); 4];
            ship.ship_mut().ship_input_state = SPECIAL as u8;
            with_rng(81, 0, || intelligence(&mut ship, &mut concerns));
            assert_eq!(ship.ship_mut().ship_input_state & SPECIAL as u8, 0);
        }
    })
}

#[test]
fn ai_weapon_threat_selection_and_release_cooldown_use_exact_boundaries() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let threat = PoolElement::new();
            for (counter, turn, state, selected) in [
                (1, 8, MOVEMENT_STATE_ENTICE, true),
                (1, 9, MOVEMENT_STATE_ENTICE, false),
                (0, 8, MOVEMENT_STATE_ENTICE, false),
                (2, 8, MOVEMENT_STATE_ENTICE, false),
                (1, 8, MOVEMENT_STATE_AVOID, false),
            ] {
                ship.ship_mut().old_status_flags = WEAPON as u16;
                ship.ship_mut().ship_input_state = (SPECIAL | WEAPON) as u8;
                ship.ship_mut().weapon_counter = 1;
                ship.ship_mut().special_counter = counter;
                let mut concerns: [Concern; 4] = [Concern::default(); 4];
                concerns[ENEMY_WEAPON_INDEX as usize] = Concern {
                    object: Some(threat.id),
                    facing: 0,
                    which_turn: turn,
                    movement: state,
                };
                with_rng(51, 0, || intelligence(&mut ship, &mut concerns));
                assert_eq!(ship.ship_mut().weapon_counter, if selected { 3 } else { 1 });
                assert_eq!(
                    ship.ship_mut().ship_input_state & WEAPON as u8 != 0,
                    !selected
                );
                assert_eq!(
                    ship.ship_mut().ship_input_state & SPECIAL as u8 != 0,
                    selected
                );
            }
            // A matching crystal without an enemy ship terminates the backwards
            // scan, then follows the same no-crystal release path.
            let h = crystal(&mut ship);
            battle::put(h);
            ship.ship_mut().old_status_flags = WEAPON as u16;
            ship.ship_mut().weapon_counter = 0;
            ship.ship_mut().special_counter = 1;
            let mut concerns: [Concern; 4] = [Concern::default(); 4];
            concerns[ENEMY_WEAPON_INDEX as usize] = Concern {
                object: Some(threat.id),
                facing: 0,
                which_turn: 8,
                movement: MOVEMENT_STATE_ENTICE,
            };
            intelligence(&mut ship, &mut concerns);
            assert_eq!(ship.ship_mut().weapon_counter, 3);
            assert_eq!(ship.ship_mut().ship_input_state & WEAPON as u8, 0);
        }
    })
}

#[test]
fn ai_crystal_intercept_holds_or_releases_at_the_fragment_margin() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let mut enemy =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 1, [16, 10, 6], [4, 4]);
            let h = crystal(&mut ship);
            let crystal = h;
            battle::put(h);
            battle::get_mut(crystal).current.location = Point { x: 4000, y: 4000 };
            uqm_rust::battle::velocity::set_velocity_vector(
                &mut battle::get_mut(crystal).velocity,
                64,
                0,
            );
            for (x, y, hold, immediate) in [
                (4000, 3000, true, false),
                (4320, 3000, true, false),
                (4321, 3000, false, false),
                (4000, 3800, false, true),
            ] {
                at(&mut enemy, x, y);
                let intercept = battle::plot_intercept(crystal, enemy.id, 90, 320);
                if immediate {
                    assert_eq!(intercept, 1);
                    assert_eq!(battle::plot_intercept(crystal, enemy.id, 90, 0), 0);
                } else {
                    assert_eq!(intercept > 1, hold, "({x}, {y}) intercept {intercept}");
                }
                ship.ship_mut().old_status_flags = WEAPON as u16;
                ship.ship_mut().weapon_counter = 0;
                ship.ship_mut().special_counter = 4;
                ship.ship_mut().ship_input_state = (SPECIAL | WEAPON) as u8;
                let mut concerns: [Concern; 4] = [Concern::default(); 4];
                concerns[ENEMY_SHIP_INDEX as usize].object = Some(enemy.id);
                with_rng(63, 0, || intelligence(&mut ship, &mut concerns));
                assert_eq!(
                    ship.ship_mut().ship_input_state & WEAPON as u8 != 0,
                    hold,
                    "({x}, {y})"
                );
                assert_eq!(ship.ship_mut().ship_input_state & SPECIAL as u8, 0);
            }
        }
    })
}

#[test]
fn ai_scans_from_tail_skips_every_noncrystal_case_and_stops_at_the_first_match() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let mut enemy =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 1, [16, 10, 6], [4, 4]);
            at(&mut enemy, 4000, 3000);
            let miss_h = crystal(&mut ship);
            let miss = miss_h;
            battle::get_mut(miss).current.location = Point { x: 1000, y: 4000 };
            uqm_rust::battle::velocity::set_velocity_vector(
                &mut battle::get_mut(miss).velocity,
                64,
                0,
            );
            battle::put(miss_h);
            let hit_h = crystal(&mut ship);
            let hit = hit_h;
            battle::get_mut(hit).current.location = Point { x: 4000, y: 4000 };
            uqm_rust::battle::velocity::set_velocity_vector(
                &mut battle::get_mut(hit).velocity,
                64,
                0,
            );
            battle::put(hit_h);
            let mut invalid_fixtures = Vec::new();
            for invalid in 0..5 {
                let fixture = Object::new(&_battle);
                let e = fixture.id;
                let h = e;
                let prim_index = battle::get(e).prim_index;
                *battle::get_mut(e) = battle::get(hit).clone();
                battle::get_mut(e).prim_index = prim_index;
                match invalid {
                    0 => battle::get_mut(e).state_flags |= NONSOLID as u16,
                    1 => battle::get_mut(e).next.image.frames = ship.frame_set(2),
                    2 => battle::get_mut(e).preprocess_func = None,
                    3 => battle::get_mut(e).life = 0,
                    4 => battle::get_mut(e).player_nr = 1,
                    _ => unreachable!(),
                }
                battle::put(h);
                invalid_fixtures.push(fixture);
            }
            ship.ship_mut().old_status_flags = WEAPON as u16;
            ship.ship_mut().special_counter = 4;
            let mut concerns: [Concern; 4] = [Concern::default(); 4];
            concerns[ENEMY_SHIP_INDEX as usize].object = Some(enemy.id);
            with_rng(30, 0, || intelligence(&mut ship, &mut concerns));
            assert_ne!(
                ship.ship_mut().ship_input_state & WEAPON as u8,
                0,
                "newest matching crystal intercepts"
            );
            battle_context::unlink(hit_h);
            battle_context::dispose(hit_h);
            with_rng(30, 0, || intelligence(&mut ship, &mut concerns));
            assert_eq!(
                ship.ship_mut().ship_input_state & WEAPON as u8,
                0,
                "older matching crystal misses"
            );
        }
    })
}

#[test]
fn ai_launch_probe_uses_the_selected_target_and_respects_weapon_cooldown() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let mut enemy =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 1, [16, 10, 6], [4, 4]);
            ship.ship_mut().ship_facing = 0;
            ship.ship_mut().special_counter = 4;
            for (x, counter, fire) in [(4000, 0, true), (4400, 0, false), (4000, 1, false)] {
                at(&mut enemy, x, 3000);
                ship.ship_mut().old_status_flags = 0;
                ship.ship_mut().weapon_counter = counter;
                ship.ship_mut().ship_input_state = SPECIAL as u8;
                let mut concerns: [Concern; 4] = [Concern::default(); 4];
                concerns[ENEMY_SHIP_INDEX as usize].object = Some(enemy.id);
                let before = queued();
                with_rng(8, 0, || intelligence(&mut ship, &mut concerns));
                assert_eq!(ship.ship_mut().ship_input_state & WEAPON as u8 != 0, fire);
                assert_eq!(
                    queued(),
                    before,
                    "launch probes never enter the display queue"
                );
            }
            ship.ship_mut().weapon_counter = 0;
            ship.ship_mut().special_counter = 1;
            let threat = PoolElement::new();
            battle::get_mut(threat.id).current.location = Point { x: 4000, y: 3000 };
            battle::get_mut(threat.id).next = battle::get(threat.id).current.clone();
            let mut concerns: [Concern; 4] = [Concern::default(); 4];
            concerns[ENEMY_WEAPON_INDEX as usize] = Concern {
                object: Some(threat.id),
                facing: 0,
                which_turn: 8,
                movement: MOVEMENT_STATE_ENTICE,
            };
            with_rng(8, 0, || intelligence(&mut ship, &mut concerns));
            assert_ne!(ship.ship_mut().ship_input_state & WEAPON as u8, 0);
            assert_eq!(ship.ship_mut().ship_input_state & SPECIAL as u8, 0);
        }
    })
}

#[path = "support/hooks.rs"]
mod hooks;
