//! Chenjesu fixtures retain the original integer, order and RNG expectations.

use uqm_rust::battle::ContextOps;
#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
use battle::{
    prim, queued, toward as velocity_toward, velocity as velocity_fields, with_rng, Battle, Object,
    PoolElement, Ship,
};
use std::{ffi::CStr, mem};
use uqm_rust::ships::native::{
    abi::*, angle_to_facing, cosine, facing_to_angle, frame_index, normalize_facing, sine,
};

unsafe fn crystal(ship: &mut Ship<'_>) -> *mut std::ffi::c_void {
    // SAFETY: the fixture owns the published battle, ship and returned missile.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            let origin = cx.weapon_origin(ship.id);
            let out = cx.spawn_weapons(ship.ship_id, origin);
            assert_eq!(out.len(), 1);
            let id = out[0].unwrap_or_else(|| panic!("crystal allocation"));
            assert_eq!(cx.parent(id), uqm_rust::engine::battle::Parent::None);
            cx.set_parent(id, Some(ship.ship_id));
            cx.copy_visual_to_next(id, cx.element(id).current);
            cx.set_intersect_frame(id, cx.element(id).current.image.frame);
            cx.legacy_element(id).cast()
        })
    }
}
unsafe fn doggy(ship: &mut Ship<'_>) -> *mut ElementRecord {
    // SAFETY: the fixture retains the serialized ship and its resources.
    unsafe {
        ship.ship_mut().cur_status_flags = SPECIAL as u16;
        ship.ship_mut().special_counter = 0;
        ship.descriptor_mut().ship_info.energy_level = 30;
        let old_tail = battle::tail();
        ship.postprocess();
        let tail = battle::tail();
        assert_ne!(tail, old_tail);
        let dog = battle::element(tail);
        (*dog).next = (*dog).current.clone();
        dog
    }
}
unsafe fn at(ship: &mut Ship<'_>, x: COORD, y: COORD) {
    // SAFETY: no record borrow crosses a hook call.
    unsafe {
        ship.element_mut().current.location = POINT { x, y };
        ship.element_mut().next.location = POINT { x, y };
    }
}
unsafe fn intelligence(ship: &mut Ship<'_>, concerns: &mut [EVALUATE_DESC; 4]) {
    // SAFETY: the fixture owns all projected records and no borrow crosses dispatch.
    unsafe {
        ship.intelligence(&mut *ship.ptr(), concerns, 0);
    }
}

fn sentinel(_: &mut uqm_rust::engine::battle::Cx<'_>, _: uqm_rust::engine::battle::ElementId) {}

#[test]
fn descriptor_matches_every_c_initializer_field() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        unsafe {
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
            assert_eq!(
                CStr::from_ptr(d.ship_info.race_strings_rsc),
                c"ship.chenjesu.text"
            );
            assert_eq!(
                CStr::from_ptr(d.ship_info.icons_rsc),
                c"ship.chenjesu.icons"
            );
            assert_eq!(
                CStr::from_ptr(d.ship_info.melee_icon_rsc),
                c"ship.chenjesu.meleeicons"
            );
            assert!(d.ship_info.race_strings.is_null());
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
                    CStr::from_ptr(d.ship_data.ship_rsc[i]).to_str().unwrap(),
                    format!("ship.chenjesu.graphics.broodhome.{size}")
                );
                assert_eq!(
                    CStr::from_ptr(d.ship_data.weapon_rsc[i]).to_str().unwrap(),
                    format!("ship.chenjesu.graphics.spark.{size}")
                );
                assert_eq!(
                    CStr::from_ptr(d.ship_data.special_rsc[i]).to_str().unwrap(),
                    format!("ship.chenjesu.graphics.doggy.{size}")
                );
                assert!(d.ship_data.ship.view(i).is_absent());
                assert!(d.ship_data.weapon.view(i).is_absent());
                assert!(d.ship_data.special.view(i).is_absent());
            }
            let captain = &d.ship_data.captain_control;
            assert_eq!(
                CStr::from_ptr(captain.captain_rsc),
                c"ship.chenjesu.graphics.captain"
            );
            assert!(captain.background.is_null());
            assert!(captain.turn.is_null());
            assert!(captain.thrust.is_null());
            assert!(captain.weapon.is_null());
            assert!(captain.special.is_absent());
            assert_eq!(
                CStr::from_ptr(d.ship_data.victory_ditty_rsc),
                c"ship.chenjesu.ditty"
            );
            assert_eq!(
                CStr::from_ptr(d.ship_data.ship_sounds_rsc),
                c"ship.chenjesu.sounds"
            );
            assert!(d.ship_data.victory_ditty.is_null());
            assert!(d.ship_data.ship_sounds.is_null());
            assert_eq!(d.cyborg_control.ManeuverabilityIndex, 0);
            assert_eq!(d.cyborg_control.WeaponRange, 4000);

            assert!(race.intelligence.is_some());

            assert!(race.preprocess.is_some());

            assert!(race.postprocess.is_some());

            assert!(race.init_weapon.is_some());

            assert!(race.uninit.is_none());
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
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            ship.ship_mut().ship_facing = 7;
            at(&mut ship, 321, -654);
            let h = crystal(&mut ship);
            let crystal = battle::element(h);
            assert_eq!((*crystal).life, 90);
            assert_eq!((*crystal).hits, 10);
            assert_eq!((*crystal).mass, 6);
            assert_eq!((*crystal).thrust, 0);
            assert_eq!(
                (*crystal).state_flags,
                (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
            );
            assert_eq!((*crystal).playerNr, 0);
            assert_eq!(
                (*crystal).current.image.frame.raw(),
                SetAbsFrameIndex(ship.descriptor_mut().ship_data.weapon.view(0).raw(), 0)
            );
            assert_eq!((*crystal).current.image.frames, ship.frame_set(1));
            assert_eq!(
                velocity_fields(&(*crystal).velocity),
                velocity_toward(64, 7)
            );
            let angle = facing_to_angle(7);
            let vx = cosine(angle, 64 << VELOCITY_SHIFT);
            let vy = sine(angle, 64 << VELOCITY_SHIFT);
            assert_eq!(
                (*crystal).current.location.x,
                (321 + i32::from(cosine(angle, 64)) - (i32::from(vx) >> VELOCITY_SHIFT)) as i16
            );
            assert_eq!(
                (*crystal).current.location.y,
                (-654 + i32::from(sine(angle, 64)) - (i32::from(vy) >> VELOCITY_SHIFT)) as i16
            );
            assert!((*crystal).collision_func.is_some());
            assert!((*crystal).postprocess_func.is_none());
            let preprocess = (*crystal).preprocess_func.unwrap();
            ship.ship_mut().cur_status_flags = WEAPON as u16;
            with_rng(21, 0, || hooks::invoke_element_pointer(preprocess, crystal));
            assert_eq!((*crystal).life, 91);
            (*crystal).life = u16::MAX;
            hooks::invoke_element_pointer(preprocess, crystal);
            assert_eq!((*crystal).life, 0);
            ship.ship_mut().cur_status_flags = 0;
            with_rng(21, 0, || hooks::invoke_element_pointer(preprocess, crystal));
            assert_eq!((*crystal).life, 1);
            let release = (*crystal).postprocess_func.unwrap().address();
            ship.ship_mut().cur_status_flags = WEAPON as u16;
            hooks::invoke_element_pointer(preprocess, crystal);
            assert_eq!((*crystal).life, 2);
            assert_eq!((*crystal).postprocess_func.unwrap().address(), release);
            battle_context::dispose(h);
        }
    })
}

#[test]
fn released_crystal_queues_eight_fragments_in_facing_order() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 1, [16, 10, 6], [4, 4]);
            let h = crystal(&mut ship);
            let crystal = battle::element(h);
            (*crystal).next.location = POINT { x: 5000, y: 6000 };
            hooks::invoke_element_pointer((*crystal).preprocess_func.unwrap(), crystal);
            let before = queued().len();
            with_rng(90, 0, || {
                hooks::invoke_element_pointer((*crystal).postprocess_func.unwrap(), crystal)
            });
            let all = queued();
            assert_eq!(all.len(), before + 8);
            for (i, &fragment_handle) in all[before..].iter().enumerate() {
                let fragment = battle::element(fragment_handle);
                let face = i as u16 * 2;
                assert_eq!(
                    (*fragment).parent,
                    uqm_rust::engine::battle::Parent::Ship(ship.ship_id)
                );
                assert_eq!((*fragment).playerNr, 1);
                assert_eq!(
                    (*fragment).state_flags,
                    (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
                );
                assert_eq!((*fragment).life, 10);
                assert_eq!((*fragment).hits, 1);
                assert_eq!((*fragment).mass, 2);
                assert_eq!((*fragment).thrust, 2);
                assert_eq!(
                    (*fragment).current.image.frame.raw(),
                    SetAbsFrameIndex(ship.descriptor_mut().ship_data.weapon.view(0).raw(), 1)
                );
                assert_eq!(
                    (*fragment).current.image.frames,
                    (*crystal).next.image.frames
                );
                assert!((*fragment).preprocess_func.is_none());
                assert!((*fragment).postprocess_func.is_none());
                assert!((*fragment).death_func.is_none());
                assert!((*fragment).collision_func.is_some());
                assert_eq!(
                    velocity_fields(&(*fragment).velocity),
                    velocity_toward(64, face)
                );
                let a = facing_to_angle(i32::from(face));
                let dx = i32::from(cosine(a, 64 << VELOCITY_SHIFT)) >> VELOCITY_SHIFT;
                let dy = i32::from(sine(a, 64 << VELOCITY_SHIFT)) >> VELOCITY_SHIFT;
                assert_eq!(
                    (
                        (*fragment).current.location.x,
                        (*fragment).current.location.y
                    ),
                    ((5000 - dx) as i16, (6000 - dy) as i16)
                );
                assert_eq!(battle::pred(fragment), all[before + i - 1]);
            }
            assert!(battle::succ(battle::tail()).is_null());
            battle_context::dispose(h);
        }
    })
}

#[test]
fn allocation_failure_preserves_weapon_count_and_partial_fragment_order() {
    battle_context::run(|_| {
        let _battle = Battle::new(4);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let crystal_handle = crystal(&mut ship);
            let crystal = battle::element(crystal_handle);
            hooks::invoke_element_pointer((*crystal).preprocess_func.unwrap(), crystal);
            with_rng(3, 0, || {
                hooks::invoke_element_pointer((*crystal).postprocess_func.unwrap(), crystal)
            });
            let all = queued();
            assert_eq!(all.len(), 3);
            assert_eq!(
                velocity_fields(&(*battle::element(all[1])).velocity),
                velocity_toward(64, 0)
            );
            assert_eq!(
                velocity_fields(&(*battle::element(all[2])).velocity),
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
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let h = crystal(&mut ship);
            let crystal = battle::element(h);
            let mut target = PoolElement::new();
            target.playerNr = 1;
            target.current.location = POINT { x: 1234, y: -4321 };
            // Not NORMAL_LIFE: the generic weapon collision must not damage it.
            target.life = 2;
            let mut pa = POINT { x: 20, y: 30 };
            let mut pb = POINT { x: 40, y: 50 };
            let collide = (*crystal).collision_func.unwrap();
            with_rng(17, 0, || {
                hooks::invoke_collision_pointer(
                    collide,
                    crystal,
                    &mut pa,
                    &mut *target,
                    &mut pb,
                    hooks::CollisionCall::Hook,
                )
            });
            let sparkle = battle::element(battle::tail());
            assert_ne!(sparkle, ship.ptr());
            assert_eq!(
                ((*sparkle).current.location.x, (*sparkle).current.location.y),
                (1234, -4321)
            );
            assert_eq!((*sparkle).life, 8);
            assert_eq!((*sparkle).turn, 0);
            assert_eq!((*sparkle).thrust, 0);
            assert_eq!(
                (*sparkle).current.image.frames,
                (*crystal).next.image.frames
            );
            assert_eq!(
                (*sparkle).current.image.frame.raw(),
                SetAbsFrameIndex(ship.descriptor_mut().ship_data.weapon.view(0).raw(), 2)
            );
            assert_eq!((*crystal).life, 0);
            assert_eq!((*crystal).hits, 0);
            assert_eq!(
                (*crystal).state_flags & (COLLISION | NONSOLID | DISAPPEARING) as u16,
                (COLLISION | NONSOLID | DISAPPEARING) as u16
            );
            let animate = (*sparkle).preprocess_func.unwrap();
            (*sparkle).next.image.frame = ship.frame(1, 8);
            with_rng(17, 0, || hooks::invoke_element_pointer(animate, sparkle));
            assert_eq!(frame_index((*sparkle).next.image.frame.raw()), 3);
            assert_ne!((*sparkle).state_flags & CHANGING as u16, 0);
            (*sparkle).turn = 2;
            (*sparkle).thrust = 3;
            (*sparkle).state_flags &= !(CHANGING as u16);
            hooks::invoke_element_pointer(animate, sparkle);
            assert_eq!((*sparkle).turn, 1);
            assert_eq!((*sparkle).state_flags & CHANGING as u16, 0);
            hooks::invoke_element_pointer(animate, sparkle);
            assert_eq!((*sparkle).turn, 0);
            (*sparkle).current.image.frame = ship.frame(1, 9);
            hooks::invoke_element_pointer(animate, sparkle);
            assert_eq!(
                (*sparkle).next.image.frame.raw(),
                SetAbsFrameIndex(ship.descriptor_mut().ship_data.weapon.view(0).raw(), 0)
            );
            assert_eq!((*sparkle).turn, 3);
            let tail = battle::tail();
            hooks::invoke_collision_pointer(
                collide,
                crystal,
                &mut pa,
                &mut *target,
                &mut pb,
                hooks::CollisionCall::Hook,
            );
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
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            for facing in [0, 4, 7, 8, 15, u16::MAX] {
                ship.ship_mut().ship_facing = facing;
                at(&mut ship, i16::MAX - 10, i16::MIN + 10);
                let dog = with_rng(7, 0, || doggy(&mut ship));
                let angle = (facing_to_angle(i32::from(facing)) + HALF_CIRCLE as i32) as COUNT;
                let expected_facing = normalize_facing(angle_to_facing(i32::from(angle)));
                assert_eq!((*dog).playerNr, 0);
                assert_eq!((*dog).state_flags, APPEARING as u16);
                assert_eq!((*dog).life, 1);
                assert_eq!((*dog).hits, 3);
                assert_eq!((*dog).mass, 4);
                assert_eq!((*dog).thrust, 0);
                assert_eq!((*prim(dog)).kind, gfx_object_STAMP_PRIM as GRAPHICS_PRIM);
                assert_eq!((*dog).current.image.frames, ship.frame_set(2));
                assert_eq!(
                    (*dog).current.image.frame.raw(),
                    SetAbsFrameIndex(ship.descriptor_mut().ship_data.special.view(0).raw(), 0)
                );
                assert_eq!(
                    (*dog).parent,
                    uqm_rust::engine::battle::Parent::Ship(ship.ship_id)
                );
                assert!((*dog).target.is_none());
                assert!((*dog).preprocess_func.is_some());
                assert!((*dog).postprocess_func.is_none());
                assert!((*dog).collision_func.is_some());
                assert!((*dog).death_func.is_some());
                assert_eq!(
                    velocity_fields(&(*dog).velocity),
                    velocity_toward(32, expected_facing)
                );
                let dx = cosine(i32::from(angle), 34 << ONE_SHIFT);
                let dy = sine(i32::from(angle), 34 << ONE_SHIFT);
                assert_eq!(
                    (*dog).current.location.x,
                    ((i32::from(i16::MAX) - 10) + i32::from(dx)) as i16
                );
                assert_eq!(
                    (*dog).current.location.y,
                    ((i32::from(i16::MIN) + 10) + i32::from(dy)) as i16
                );
                assert_ne!((*ship.ptr()).state_flags & DEFY_PHYSICS as u16, 0);
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
        unsafe {
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
                (*ship.ptr()).state_flags &= !(DEFY_PHYSICS as u16);
                with_rng(1, 0, || ship.postprocess());
                assert_eq!(queued().len(), before + usize::from(spawn));
                assert_eq!(ship.ship_mut().special_counter, 1);
                assert_eq!(
                    ship.descriptor_mut().ship_info.energy_level,
                    if spawn { 0 } else { energy }
                );
                assert_eq!(ship.ship_mut().cur_status_flags & (1 << 5) != 0, low);
                assert_eq!((*ship.ptr()).state_flags & DEFY_PHYSICS as u16 != 0, spawn);
            }
        }
    })
}

#[test]
fn failed_doggy_allocation_still_spends_the_energy() {
    battle_context::run(|_| {
        let _battle = Battle::new(1);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.postprocess();
            assert_eq!(queued(), vec![ship.handle]);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 0);
            assert_eq!(ship.ship_mut().energy_counter, 4);
            assert_eq!(ship.ship_mut().special_counter, 1);
            assert_eq!((*ship.ptr()).state_flags & DEFY_PHYSICS as u16, 0);
        }
    })
}

#[test]
fn ship_preprocess_matches_byte_promotion_and_wrap_boundaries() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        unsafe {
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
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let mut enemy =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 1, [16, 10, 6], [4, 4]);
            let dog = doggy(&mut ship);
            let collide = (*dog).collision_func.unwrap();
            let mut pa = POINT { x: 0, y: 0 };
            let mut pb = POINT { x: 0, y: 0 };
            for (energy, remaining) in [(0, 0), (9, 0), (10, 0), (11, 1), (30, 20)] {
                enemy.descriptor_mut().ship_info.energy_level = energy;
                enemy.ship_mut().energy_counter = 99;
                (*dog).thrust = 0;
                with_rng(101, 0, || {
                    hooks::invoke_collision_pointer(
                        collide,
                        dog,
                        &mut pa,
                        enemy.ptr(),
                        &mut pb,
                        hooks::CollisionCall::Hook,
                    )
                });
                assert_eq!(enemy.descriptor_mut().ship_info.energy_level, remaining);
                assert_eq!(
                    enemy.ship_mut().energy_counter,
                    4,
                    "even zero drain calls DeltaEnergy"
                );
                assert_ne!((*dog).state_flags & COLLISION as u16, 0);
                assert_eq!((*dog).thrust, 6);
            }
            for (wait, expected) in [(0, 6), (1, 7), (2, 8), (3, 9), (4, 4), (255, 255)] {
                (*dog).thrust = wait;
                ship.descriptor_mut().ship_info.energy_level = 17;
                hooks::invoke_collision_pointer(
                    collide,
                    dog,
                    &mut pa,
                    ship.ptr(),
                    &mut pb,
                    hooks::CollisionCall::Hook,
                );
                assert_eq!((*dog).thrust, expected);
                assert_eq!(ship.descriptor_mut().ship_info.energy_level, 17);
            }
            let mut fixture = Object::new(&_battle);
            let object = &mut *fixture.ptr();
            object.playerNr = 1;
            object.state_flags = FINITE_LIFE as u16;
            (*dog).state_flags &= !(COLLISION as u16);
            hooks::invoke_collision_pointer(
                collide,
                dog,
                &mut pa,
                object,
                &mut pb,
                hooks::CollisionCall::Hook,
            );
            assert_eq!((*dog).state_flags & COLLISION as u16, 0);
            // The generic collision runs first and damages the DOGI on planets.
            object.state_flags = 0;
            object.mass = 101;
            hooks::invoke_collision_pointer(
                collide,
                dog,
                &mut pa,
                object,
                &mut pb,
                hooks::CollisionCall::Hook,
            );
            assert_eq!((*dog).hits, 2);
            assert_ne!((*dog).state_flags & COLLISION as u16, 0);
        }
    })
}

#[test]
fn doggy_death_preserves_unrelated_fields_and_starts_six_frame_animation() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let dog = doggy(&mut ship);
            (*dog).state_flags |= (DISAPPEARING | COLLISION | DEFY_PHYSICS) as u16;
            (*dog).postprocess_func = Some(uqm_rust::engine::battle::ElementHook::Native(sentinel));
            (*dog).target = Some(battle::id((ship.handle).cast()));
            (*dog).turn = 9;
            (*dog).thrust = 7;
            let preprocess = (*dog).preprocess_func.unwrap().address();
            let old_flags = (*dog).state_flags;
            let counter = ship.ship_mut().special_counter;
            with_rng(88, 0, || {
                hooks::invoke_element_pointer((*dog).death_func.unwrap(), dog)
            });
            assert_eq!(
                (*dog).state_flags,
                (old_flags & !(DISAPPEARING as u16)) | (NONSOLID | FINITE_LIFE) as u16
            );
            assert_eq!((*dog).life, 6);
            assert_eq!(velocity_fields(&(*dog).velocity), [0; 9]);
            assert_eq!((*dog).turn, 0);
            assert_eq!((*dog).thrust, 0);
            assert!((*dog).death_func.is_none());
            assert!((*dog).collision_func.is_none());
            assert_ne!((*dog).preprocess_func.unwrap().address(), preprocess);
            assert_eq!(
                (*dog).postprocess_func.unwrap().address(),
                sentinel as *const () as usize
            );
            assert_eq!((*dog).target, Some(battle::id((ship.handle).cast())));
            assert_eq!(
                (*dog).parent,
                uqm_rust::engine::battle::Parent::Ship(ship.ship_id)
            );
            assert_eq!((*dog).hits, 3);
            assert_eq!((*dog).mass, 4);
            let animate = (*dog).preprocess_func.unwrap();
            for frame in 1..=6 {
                with_rng(88, 0, || hooks::invoke_element_pointer(animate, dog));
                assert_eq!(frame_index((*dog).next.image.frame.raw()), frame % 6);
                (*dog).current = (*dog).next.clone();
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
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let dog = doggy(&mut ship);
            let preprocess = (*dog).preprocess_func.unwrap();
            uqm_rust::battle::velocity::set_velocity_vector(&mut (*dog).velocity, 11, 3);
            let original = velocity_fields(&(*dog).velocity);
            (*dog).thrust = 1;
            ship.ship_mut().special_counter = 255;
            with_rng(42, 0, || hooks::invoke_element_pointer(preprocess, dog));
            assert_eq!(ship.ship_mut().special_counter, 0);
            assert_eq!((*dog).thrust, 0);
            assert_eq!(velocity_fields(&(*dog).velocity), original);
            for seed in 1..33 {
                uqm_rust::battle::velocity::set_velocity_vector(&mut (*dog).velocity, 11, 3);
                TFB_SeedRandom(seed);
                let face = (TFB_Random() & 15) as u16;
                with_rng(seed, 1, || hooks::invoke_element_pointer(preprocess, dog));
                assert_eq!(
                    velocity_fields(&(*dog).velocity),
                    if face == 3 {
                        original
                    } else {
                        velocity_toward(32, face)
                    }
                );
                assert!((*dog).target.is_none());
            }
        }
    })
}

#[test]
fn doggy_steering_uses_exact_rear_facing_boundaries_not_trackships_single_step() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let enemy = Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 1, [16, 10, 6], [4, 4]);
            let dog = doggy(&mut ship);
            let preprocess = (*dog).preprocess_func.unwrap();
            (*dog).current.location = POINT { x: 4000, y: 4500 };
            (*dog).next.location = (*dog).current.location;
            // Enemy due north: raw facing 0, TrackShip's one-step result is 3.
            for (delta, expected) in [(6, 0), (7, 4), (8, 12), (9, 12), (10, 0)] {
                (*enemy.ptr()).current.image.frame = enemy.frame(0, (16 - delta) as u16);
                (*dog).target = None;
                uqm_rust::battle::velocity::set_velocity_vector(&mut (*dog).velocity, 11, 4);
                let original = velocity_fields(&(*dog).velocity);
                with_rng(99, 0, || hooks::invoke_element_pointer(preprocess, dog));
                assert_eq!((*dog).target, Some(battle::id((enemy.handle).cast())));
                assert_eq!(
                    velocity_fields(&(*dog).velocity),
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
            (*enemy.ptr()).current.image.frame = enemy.frame(0, 0);
            for seed in 1..17 {
                uqm_rust::battle::velocity::set_velocity_vector(&mut (*dog).velocity, 32, 8);
                (*dog).target = Some(battle::id((enemy.handle).cast()));
                with_rng(seed, 1, || hooks::invoke_element_pointer(preprocess, dog));
                assert_eq!(velocity_fields(&(*dog).velocity), velocity_toward(32, 0));
            }
        }
    })
}

#[test]
fn doggy_uses_current_unwrapped_coordinates_and_truncates_arctan_arguments() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let mut enemy =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 1, [16, 10, 6], [4, 4]);
            let dog = doggy(&mut ship);
            let preprocess = (*dog).preprocess_func.unwrap();
            for (dog_x, enemy_x, expected) in
                [(-30000, 30000, 12), (8000, 100, 12), (30000, -30000, 4)]
            {
                (*dog).current.location = POINT { x: dog_x, y: 4000 };
                (*dog).next.location = POINT { x: 1000, y: 4000 };
                at(&mut enemy, enemy_x, 4000);
                (*enemy.ptr()).next.location = POINT { x: 2000, y: 4000 };
                (*enemy.ptr()).current.image.frame = enemy.frame(0, (expected as usize) as u16);
                (*dog).target = Some(battle::id((enemy.handle).cast()));
                uqm_rust::battle::velocity::set_velocity_vector(&mut (*dog).velocity, 32, 0);
                assert_eq!(
                    normalize_facing(angle_to_facing(i32::from(ARCTAN(
                        (i32::from(enemy_x) - i32::from(dog_x)) as SIZE,
                        0
                    )))),
                    expected
                );
                with_rng(61, 0, || hooks::invoke_element_pointer(preprocess, dog));
                assert_eq!(
                    velocity_fields(&(*dog).velocity),
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
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let mut far =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 1, [16, 10, 6], [4, 4]);
            let mut near =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 1, [16, 10, 6], [4, 4]);
            let dog = doggy(&mut ship);
            (*dog).current.location = POINT { x: 4000, y: 4000 };
            (*dog).next = (*dog).current.clone();
            at(&mut near, 4100, 4000);
            at(&mut far, 4000, 3500);
            let preprocess = (*dog).preprocess_func.unwrap();
            uqm_rust::battle::velocity::set_velocity_vector(&mut (*dog).velocity, 32, 0);
            with_rng(4, 0, || hooks::invoke_element_pointer(preprocess, dog));
            assert_eq!((*dog).target, Some(battle::id((near.handle).cast())));
            assert_eq!(velocity_fields(&(*dog).velocity), velocity_toward(32, 4));
            (*prim(near.ptr())).kind = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
            uqm_rust::battle::velocity::set_velocity_vector(&mut (*dog).velocity, 32, 0);
            with_rng(4, 0, || hooks::invoke_element_pointer(preprocess, dog));
            assert_eq!(
                (*dog).target,
                Some(battle::id((near.handle).cast())),
                "TrackShip retains a live prior target even when cloaked"
            );
            (*dog).target = None;
            uqm_rust::battle::velocity::set_velocity_vector(&mut (*dog).velocity, 32, 0);
            with_rng(4, 0, || hooks::invoke_element_pointer(preprocess, dog));
            assert_eq!((*dog).target, Some(battle::id((far.handle).cast())));
            assert_eq!(velocity_fields(&(*dog).velocity), velocity_toward(32, 0));
        }
    })
}

#[test]
fn ai_pursuit_boundaries_match_turn_maneuverability_range_and_seeking_flags() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        unsafe {
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
                enemy.descriptor_mut().cyborg_control.ManeuverabilityIndex = maneuver;
                enemy.descriptor_mut().cyborg_control.WeaponRange = range;
                enemy.descriptor_mut().ship_info.ship_flags =
                    if seeking { SEEKING_WEAPON as u16 } else { 0 };
                let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
                concerns[ENEMY_SHIP_INDEX as usize] = EVALUATE_DESC {
                    ObjectPtr: enemy.ptr(),
                    facing: 0,
                    which_turn: turn,
                    MoveState: MOVEMENT_STATE_ENTICE,
                };
                with_rng(6, 0, || intelligence(&mut ship, &mut concerns));
                assert_eq!(
                    concerns[ENEMY_SHIP_INDEX as usize].MoveState,
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
        unsafe {
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
                        let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
                        concerns[ENEMY_SHIP_INDEX as usize].ObjectPtr = enemy.ptr();
                        with_rng(81, 0, || intelligence(&mut ship, &mut concerns));
                        assert_eq!(
                            ship.ship_mut().ship_input_state & SPECIAL as u8 != 0,
                            counter < 4 && energy <= 30 && !weapon,
                            "counter {counter}, energy {energy}, weapon {weapon}"
                        );
                    }
                }
            }
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
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
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let mut threat = PoolElement::new();
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
                let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
                concerns[ENEMY_WEAPON_INDEX as usize] = EVALUATE_DESC {
                    ObjectPtr: &mut *threat,
                    facing: 0,
                    which_turn: turn,
                    MoveState: state,
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
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            concerns[ENEMY_WEAPON_INDEX as usize] = EVALUATE_DESC {
                ObjectPtr: &mut *threat,
                facing: 0,
                which_turn: 8,
                MoveState: MOVEMENT_STATE_ENTICE,
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
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let mut enemy =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 1, [16, 10, 6], [4, 4]);
            let h = crystal(&mut ship);
            let crystal = battle::element(h);
            battle::put(h);
            (*crystal).current.location = POINT { x: 4000, y: 4000 };
            uqm_rust::battle::velocity::set_velocity_vector(&mut (*crystal).velocity, 64, 0);
            for (x, y, hold, immediate) in [
                (4000, 3000, true, false),
                (4320, 3000, true, false),
                (4321, 3000, false, false),
                (4000, 3800, false, true),
            ] {
                at(&mut enemy, x, y);
                let intercept = battle::plot_intercept(crystal, enemy.ptr(), 90, 320);
                if immediate {
                    assert_eq!(intercept, 1);
                    assert_eq!(battle::plot_intercept(crystal, enemy.ptr(), 90, 0), 0);
                } else {
                    assert_eq!(intercept > 1, hold, "({x}, {y}) intercept {intercept}");
                }
                ship.ship_mut().old_status_flags = WEAPON as u16;
                ship.ship_mut().weapon_counter = 0;
                ship.ship_mut().special_counter = 4;
                ship.ship_mut().ship_input_state = (SPECIAL | WEAPON) as u8;
                let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
                concerns[ENEMY_SHIP_INDEX as usize].ObjectPtr = enemy.ptr();
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
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 0, [16, 10, 6], [4, 4]);
            let mut enemy =
                Ship::with_frames(&_battle, SPECIES_ID_CHENJESU_ID, 1, [16, 10, 6], [4, 4]);
            at(&mut enemy, 4000, 3000);
            let miss_h = crystal(&mut ship);
            let miss = battle::element(miss_h);
            (*miss).current.location = POINT { x: 1000, y: 4000 };
            uqm_rust::battle::velocity::set_velocity_vector(&mut (*miss).velocity, 64, 0);
            battle::put(miss_h);
            let hit_h = crystal(&mut ship);
            let hit = battle::element(hit_h);
            (*hit).current.location = POINT { x: 4000, y: 4000 };
            uqm_rust::battle::velocity::set_velocity_vector(&mut (*hit).velocity, 64, 0);
            battle::put(hit_h);
            let mut invalid_fixtures = Vec::new();
            for invalid in 0..5 {
                let mut fixture = Object::new(&_battle);
                let e = fixture.ptr();
                let h = e.cast();
                let prim_index = (*e).PrimIndex;
                *e = (*hit).clone();
                (*e).PrimIndex = prim_index;
                match invalid {
                    0 => (*e).state_flags |= NONSOLID as u16,
                    1 => (*e).next.image.frames = ship.frame_set(2),
                    2 => (*e).preprocess_func = None,
                    3 => (*e).life = 0,
                    4 => (*e).playerNr = 1,
                    _ => unreachable!(),
                }
                battle::put(h);
                invalid_fixtures.push(fixture);
            }
            ship.ship_mut().old_status_flags = WEAPON as u16;
            ship.ship_mut().special_counter = 4;
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            concerns[ENEMY_SHIP_INDEX as usize].ObjectPtr = enemy.ptr();
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
        unsafe {
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
                let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
                concerns[ENEMY_SHIP_INDEX as usize].ObjectPtr = enemy.ptr();
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
            let mut threat = PoolElement::new();
            threat.current.location = POINT { x: 4000, y: 3000 };
            threat.next = threat.current.clone();
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            concerns[ENEMY_WEAPON_INDEX as usize] = EVALUATE_DESC {
                ObjectPtr: &mut *threat,
                facing: 0,
                which_turn: 8,
                MoveState: MOVEMENT_STATE_ENTICE,
            };
            with_rng(8, 0, || intelligence(&mut ship, &mut concerns));
            assert_ne!(ship.ship_mut().ship_input_state & WEAPON as u8, 0);
            assert_eq!(ship.ship_mut().ship_input_state & SPECIAL as u8, 0);
        }
    })
}

#[path = "support/hooks.rs"]
mod hooks;
