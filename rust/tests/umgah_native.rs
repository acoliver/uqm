//! Live Umgah callbacks, private cone state, collision cleanup and AI RNG order.

use uqm_rust::engine::battle::schema::HLINK;
#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
use battle::{prim, Battle, PoolElement, Ship};
#[path = "support/special_hooks.rs"]
mod special_hooks;

use std::{ffi::CStr, ptr};
use uqm_rust::collections::observation;
use uqm_rust::ships::native::{abi::*, cosine, facing_to_angle, frame_index, sine};

// FRAME_DESC and DRAWABLE_DESC layouts from drawable.h.

unsafe fn queued() -> Vec<HLINK> {
    let mut result = Vec::new();
    let mut h = battle::head();
    while !h.is_null() {
        result.push(h);
        assert!(result.len() <= 64, "display queue cycle");
        h = battle::succ(h);
    }
    result
}

fn velocity_fields(v: &VELOCITY_DESC) -> [i32; 9] {
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

unsafe fn velocity(e: *mut ElementRecord) -> (SIZE, SIZE) {
    let (vx, vy) = ((*e).velocity).get_current_components();
    let (mut x, mut y) = (vx as i16, vy as i16);
    (x, y)
}

unsafe fn draws(seed: DWORD, n: usize) -> Vec<DWORD> {
    TFB_SeedRandom(seed);
    (0..n).map(|_| TFB_Random()).collect()
}

unsafe fn seed_where(accept: impl Fn(&[DWORD]) -> bool) -> DWORD {
    (1..100_000).find(|&seed| accept(&draws(seed, 3))).unwrap()
}

unsafe fn with_rng<T>(seed: DWORD, used: usize, action: impl FnOnce() -> T) -> T {
    let sequence = draws(seed, used + 1);
    TFB_SeedRandom(seed);
    let result = action();
    assert_eq!(
        TFB_Random(),
        sequence[used],
        "RNG consumption: {used}, seed {seed}"
    );
    result
}

fn concerns() -> [EVALUATE_DESC; 4] {
    [EVALUATE_DESC {
        ObjectPtr: ptr::null_mut(),
        facing: 0,
        which_turn: 0,
        MoveState: MOVEMENT_STATE_NO_MOVEMENT,
    }; 4]
}

unsafe fn enemy_concern(enemy: &Ship<'_>, turn: COUNT) -> [EVALUATE_DESC; 4] {
    let mut c = concerns();
    c[0].ObjectPtr = enemy.ptr();
    c[0].which_turn = turn;
    c[0].MoveState = MOVEMENT_STATE_PURSUE;
    c
}

unsafe fn think(ship: &mut Ship<'_>, c: &mut [EVALUATE_DESC; 4], input: u8, count: COUNT) -> u8 {
    ship.ship_mut().ship_input_state = input;
    ship.intelligence(&mut *ship.ptr(), c, count);
    ship.ship_mut().ship_input_state
}

#[test]
fn descriptor_matches_all_c_initializer_fields_and_resources() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let d = battle::descriptor(SPECIES_ID_UMGAH_ID);
            assert_eq!(
                d.ship_info.ship_flags,
                (FIRES_FORE | IMMEDIATE_WEAPON) as u16
            );
            assert_eq!(d.ship_info.ship_cost, 7);
            assert_eq!((d.ship_info.crew_level, d.ship_info.max_crew), (10, 10));
            assert_eq!((d.ship_info.energy_level, d.ship_info.max_energy), (30, 30));
            assert_eq!(
                CStr::from_ptr(d.ship_info.race_strings_rsc),
                c"ship.umgah.text"
            );
            assert_eq!(CStr::from_ptr(d.ship_info.icons_rsc), c"ship.umgah.icons");
            assert_eq!(
                CStr::from_ptr(d.ship_info.melee_icon_rsc),
                c"ship.umgah.meleeicons"
            );
            assert!(
                d.ship_info.race_strings.is_null()
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
                    CStr::from_ptr(d.ship_data.ship_rsc[i]).to_str().unwrap(),
                    format!("ship.umgah.graphics.drone.{size}")
                );
                assert_eq!(
                    CStr::from_ptr(d.ship_data.special_rsc[i]).to_str().unwrap(),
                    format!("ship.umgah.graphics.cone.{size}")
                );
                assert!(
                    d.ship_data.ship.view(i).is_absent()
                        && d.ship_data.weapon.view(i).is_absent()
                        && d.ship_data.special.view(i).is_absent()
                );
            }
            assert_eq!(
                CStr::from_ptr(d.ship_data.weapon_rsc[0]),
                c"ship.umgah.graphics.spritz"
            );
            assert!(d.ship_data.weapon_rsc[1].is_null() && d.ship_data.weapon_rsc[2].is_null());
            let captain = &d.ship_data.captain_control;
            assert_eq!(
                CStr::from_ptr(captain.captain_rsc),
                c"ship.umgah.graphics.captain"
            );
            assert!(
                captain.background.is_null()
                    && captain.turn.is_null()
                    && captain.thrust.is_null()
                    && captain.weapon.is_null()
                    && captain.special.is_absent()
            );
            assert_eq!(
                CStr::from_ptr(d.ship_data.victory_ditty_rsc),
                c"ship.umgah.ditty"
            );
            assert_eq!(
                CStr::from_ptr(d.ship_data.ship_sounds_rsc),
                c"ship.umgah.sounds"
            );
            assert!(d.ship_data.victory_ditty.is_null() && d.ship_data.ship_sounds.is_null());
            assert_eq!(
                (
                    d.cyborg_control.ManeuverabilityIndex,
                    d.cyborg_control.WeaponRange
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
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut first =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            first.ship_mut().ship_facing = 5;
            let h = fire(&mut first);
            let data = previous_facing(&mut first);
            assert!(data.is_some());
            hooks::invoke_element_pointer(
                (*battle::element(h)).preprocess_func.unwrap(),
                battle::element(h),
            );
            assert_eq!(
                frame_index(first.descriptor_mut().ship_data.special.view(0).raw()),
                21
            );
            first.descriptor_mut().ship_info.energy_level = 1;
            first.descriptor_mut().characteristics.special_wait = 77;
            let mut second =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 1, [16, 16, 48], [4, 4]);
            assert_ne!(
                first.descriptor_mut() as *mut _,
                second.descriptor_mut() as *mut _
            );
            assert!(previous_facing(&mut second).is_none());
            assert_eq!(second.descriptor_mut().ship_info.energy_level, 30);
            assert_eq!(second.descriptor_mut().characteristics.special_wait, 2);
            assert_eq!(
                frame_index(second.descriptor_mut().ship_data.special.view(0).raw()),
                0
            );
            second.ship_mut().ship_facing = 9;
            let other = fire(&mut second);
            assert_ne!(previous_facing(&mut first), previous_facing(&mut second));
            assert_eq!(previous_facing(&mut first), data);
            assert_eq!(
                frame_index(first.descriptor_mut().ship_data.special.view(0).raw()),
                21
            );
            first.uninit();
            assert!(previous_facing(&mut first).is_none());
            assert!(!previous_facing(&mut second).is_none());
            hooks::invoke_element_pointer(
                (*battle::element(other)).preprocess_func.unwrap(),
                battle::element(other),
            );
            assert_eq!(
                frame_index(second.descriptor_mut().ship_data.special.view(0).raw()),
                25
            );
            battle_context::dispose(h);
            battle_context::dispose(other);
            let mut template = battle::descriptor(SPECIES_ID_UMGAH_ID);
            template.ship_info.energy_level = 0;
            template.characteristics.special_wait = 0;
            template.race_callbacks.pre = uqm_rust::engine::battle::Selection::Disabled;
            assert_eq!(template.ship_info.energy_level, 0);
            assert_eq!(template.characteristics.special_wait, 0);
            assert!(matches!(
                template.race_callbacks.pre,
                uqm_rust::engine::battle::Selection::Disabled
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
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 1, [16, 16, 48], [4, 4]);
            ship.ship_mut().ship_facing = 5;
            (*ship.ptr()).next.location = POINT { x: -9, y: 13 };
            ((*ship.ptr()).velocity).set_components(i32::from(321), i32::from(-654));
            let h = with_rng(8, 0, || fire(&mut ship));
            assert!(!h.is_null());
            let cone = battle::element(h);
            assert_eq!((*cone).state_flags, (FINITE_LIFE | IGNORE_SIMILAR) as u16);
            assert_eq!((*cone).life, 1);
            assert_eq!((*cone).hits, 100);
            assert_eq!((*cone).mass, 1);
            assert_eq!((*cone).thrust, 0);
            assert_eq!((*cone).playerNr, 1);
            assert_eq!(velocity_fields(&(*cone).velocity), [0; 9]);
            assert_eq!(
                ((*cone).current.location.x, (*cone).current.location.y),
                (-9, 13)
            );
            assert_eq!(((*cone).next.location.x, (*cone).next.location.y), (-9, 13));
            assert_eq!((*cone).current.image.frame.raw(), ship.raw_frame(2, 5));
            assert_eq!(
                (*cone).next.image.frame.raw(),
                (*cone).current.image.frame.raw()
            );
            assert_eq!((*cone).next.image.frames, ship.frame_set(2));
            let i = (*cone).IntersectControl;
            assert_eq!(
                (i.IntersectStamp.origin.x, i.IntersectStamp.origin.y),
                (-3, 3)
            );
            assert_eq!((i.EndPoint.x, i.EndPoint.y), (-3, 3));
            assert_eq!(i.IntersectStamp.frame, ship.raw_frame(2, 5));
            assert_eq!((*prim(cone)).Type, gfx_object_STAMP_PRIM as u8);
            assert!((*cone).preprocess_func.is_some() && (*cone).collision_func.is_some());
            assert!((*cone).postprocess_func.is_none() && (*cone).death_func.is_none());
            assert_eq!(queued(), vec![ship.handle]);
            battle_context::dispose(h);
        }
    })
}

#[test]
fn held_cone_advances_by_16_preserves_phase_and_resets_only_when_facing_changes() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            ship.ship_mut().ship_facing = 5;
            for index in [5, 21, 37, 5] {
                let h = with_rng(3, 0, || fire(&mut ship));
                let cone = battle::element(h);
                let data = previous_facing(&mut ship);
                assert!(data.is_some());
                assert_eq!(data, Some(5));
                assert_eq!(frame_index((*cone).current.image.frame.raw()), index);
                let old_frame = (*cone).next.image.frame.raw();
                with_rng(3, 0, || {
                    hooks::invoke_element_pointer((*cone).preprocess_func.unwrap(), cone)
                });
                assert_eq!(
                    frame_index(ship.descriptor_mut().ship_data.special.view(0).raw()),
                    (index + 16) % 48
                );
                assert_eq!((*cone).next.image.frame.raw(), old_frame);
                assert_eq!(
                    (*cone).state_flags,
                    (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
                );
                let next = fire(&mut ship);
                assert_eq!(
                    previous_facing(&mut ship),
                    data,
                    "holding facing keeps the saved facing"
                );
                assert_eq!(
                    frame_index((*battle::element(next)).current.image.frame.raw()),
                    (index + 16) % 48
                );
                battle_context::dispose(next);
                battle_context::dispose(h);
            }
            ship.ship_mut().ship_facing = 9;
            let h = fire(&mut ship);
            assert_eq!(previous_facing(&mut ship).unwrap(), 9);
            assert_eq!(
                frame_index((*battle::element(h)).current.image.frame.raw()),
                9
            );
            assert_eq!(
                (*battle::element(h)).IntersectControl.IntersectStamp.frame,
                ship.raw_frame(2, 9)
            );
            battle_context::dispose(h);
        }
    })
}

#[test]
fn appearance_and_uninit_free_only_the_instance_custom_data_and_are_repeatable() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let h = fire(&mut ship);
            battle_context::dispose(h);
            assert!(!previous_facing(&mut ship).is_none());
            ((*ship.ptr()).velocity).set_components(i32::from(123), i32::from(-456));
            let before = velocity_fields(&(*ship.ptr()).velocity);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.ship_mut().special_counter = 9;
            (*ship.ptr()).state_flags |= APPEARING as u16;
            with_rng(4, 0, || ship.preprocess());
            assert!(previous_facing(&mut ship).is_none());
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 30);
            assert_eq!(ship.ship_mut().special_counter, 9);
            assert_eq!(velocity_fields(&(*ship.ptr()).velocity), before);
            ship.preprocess();
            let h = fire(&mut ship);
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
    battle_context::run(|| {
        let _battle = Battle::new(1);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            ship.ship_mut().ship_facing = 15;
            assert!(with_rng(5, 0, || fire(&mut ship)).is_null());
            assert!(!previous_facing(&mut ship).is_none());
            assert_eq!(previous_facing(&mut ship).unwrap(), 15);
            assert_eq!(
                frame_index(ship.descriptor_mut().ship_data.special.view(0).raw()),
                15
            );
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 30);
            ship.ship_mut().ship_facing = 4;
            assert!(fire(&mut ship).is_null());
            assert_eq!(previous_facing(&mut ship).unwrap(), 4);
            assert_eq!(
                frame_index(ship.descriptor_mut().ship_data.special.view(0).raw()),
                4
            );
            assert_eq!(queued(), vec![ship.handle]);
        }
    })
}

unsafe fn target(finite: bool) -> PoolElement {
    let mut e = PoolElement::new();
    e.playerNr = 1;
    e.state_flags = if finite { FINITE_LIFE as u16 } else { 0 };
    e.life = 2;
    e.hits = 3;
    e.mass = 1;
    e
}

#[test]
fn collision_removes_and_frees_the_blast_but_does_not_restore_cone_life_or_solidity() {
    battle_context::run(|| {
        let _battle = Battle::new(4);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let h = fire(&mut ship);
            let cone = battle::element(h);
            let mut hit = target(false);
            let mut a = POINT { x: 10, y: 20 };
            let mut b = POINT { x: 30, y: 40 };
            let before = queued();
            with_rng(1, 0, || {
                hooks::invoke_collision_pointer(
                    (*cone).collision_func.unwrap(),
                    cone,
                    &mut a,
                    &mut *hit,
                    &mut b,
                    hooks::CollisionCall::Hook,
                )
            });
            assert_eq!(queued(), before, "the generic collision blast was unlinked");
            assert_eq!(
                (*cone).state_flags,
                (FINITE_LIFE | IGNORE_SIMILAR | COLLISION | NONSOLID) as u16
            );
            assert_eq!((*cone).life, 0);
            assert_eq!((*cone).hits, 0);
            assert_eq!(hit.hits, 3);
            let freed_blast = uqm_rust::app::bridge::enter(|cx| {
                cx.alloc_element()
                    .map_or(std::ptr::null_mut(), |id| cx.elements().raw(id))
            });
            assert!(
                !freed_blast.is_null(),
                "blast element returned to the queue free list"
            );
            assert!(uqm_rust::app::bridge::enter(|cx| cx
                .alloc_element()
                .map_or(std::ptr::null_mut(), |id| cx.elements().raw(id)))
            .is_null());
            battle_context::dispose(freed_blast);
            battle_context::dispose(h);
        }
    })
}

#[test]
fn collision_preserves_disappearing_when_blast_allocation_fails() {
    battle_context::run(|| {
        let _battle = Battle::new(3);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let h = fire(&mut ship);
            let cone = battle::element(h);
            let mut hit = target(false);
            let mut p = POINT { x: 0, y: 0 };
            hooks::invoke_collision_pointer(
                (*cone).collision_func.unwrap(),
                cone,
                &mut p,
                &mut *hit,
                &mut POINT { x: 0, y: 0 },
                hooks::CollisionCall::Hook,
            );
            assert_eq!(
                (*cone).state_flags & (DISAPPEARING | COLLISION | NONSOLID) as u16,
                (DISAPPEARING | COLLISION | NONSOLID) as u16
            );
            assert_eq!((*cone).life, 0);
            assert_eq!(queued(), vec![ship.handle]);
            battle_context::dispose(h);
        }
    })
}

#[test]
fn finite_weapon_hit_damages_once_without_a_blast_and_does_not_clear_disappearing() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let h = fire(&mut ship);
            let cone = battle::element(h);
            let mut hit = target(true);
            (*cone).state_flags |= DISAPPEARING as u16;
            let collide = (*cone).collision_func.unwrap();
            with_rng(7, 0, || {
                hooks::invoke_collision_pointer(
                    collide,
                    cone,
                    &mut POINT { x: 0, y: 0 },
                    &mut *hit,
                    &mut POINT { x: 0, y: 0 },
                    hooks::CollisionCall::Hook,
                )
            });
            assert_eq!(hit.hits, 2);
            assert_eq!((*cone).hits, 100);
            assert_eq!((*cone).life, 1);
            assert_ne!((*cone).state_flags & COLLISION as u16, 0);
            assert_ne!((*cone).state_flags & DISAPPEARING as u16, 0);
            hooks::invoke_collision_pointer(
                collide,
                cone,
                &mut POINT { x: 0, y: 0 },
                &mut *hit,
                &mut POINT { x: 0, y: 0 },
                hooks::CollisionCall::Hook,
            );
            assert_eq!(hit.hits, 2);
            assert_eq!(queued(), vec![ship.handle]);
            battle_context::dispose(h);
        }
    })
}

#[test]
fn retropropulsion_adds_the_exact_reverse_vector_for_every_facing_then_stops_all_velocity_fields() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
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
                ((*ship.ptr()).velocity).set_components(i32::from(123), i32::from(-456));
                let mut expected = (*ship.ptr()).velocity;
                let a = facing_to_angle(i32::from(facing)) + 32;
                (expected)
                    .delta_components(i32::from(cosine(a, 160 * 32)), i32::from(sine(a, 160 * 32)));
                observation::enable();
                observation::drain();
                with_rng(8, 0, || ship.preprocess());
                assert_eq!(special_hooks::count(&observation::drain(), "umgah"), 1);
                assert_eq!(
                    velocity_fields(&(*ship.ptr()).velocity),
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
                assert_eq!(velocity_fields(&(*ship.ptr()).velocity), [0; 9]);
            }
            ship.ship_mut().ship_facing = 0;
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            observation::drain();
            ship.preprocess();
            assert_eq!(special_hooks::count(&observation::drain(), "umgah"), 1);
            assert_eq!(velocity(ship.ptr()), (0, 160 * 32));
        }
    })
}

#[test]
fn retropropulsion_gates_on_appearance_thrust_wait_key_and_energy() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            for (appearing, wait, key, energy, low) in [
                (true, 0, true, 30, false),
                (false, 1, true, 30, false),
                (false, 255, true, 0, false),
                (false, 0, false, 30, false),
                (false, 0, true, 0, true),
            ] {
                (*ship.ptr()).state_flags =
                    (PLAYER_SHIP | if appearing { APPEARING } else { 0 }) as u16;
                (*ship.ptr()).thrust = wait;
                ship.ship_mut().cur_status_flags = if key { SPECIAL as u16 } else { 0 };
                ship.ship_mut().special_counter = 0;
                ship.ship_mut().energy_counter = 19;
                ship.descriptor_mut().ship_info.energy_level = energy;
                ((*ship.ptr()).velocity).set_components(i32::from(333), i32::from(-555));
                let before = velocity_fields(&(*ship.ptr()).velocity);
                observation::enable();
                observation::drain();
                with_rng(4, 0, || ship.preprocess());
                assert_eq!(special_hooks::count(&observation::drain(), "umgah"), 0);
                assert_eq!(velocity_fields(&(*ship.ptr()).velocity), before);
                assert_eq!(ship.descriptor_mut().ship_info.energy_level, energy);
                assert_eq!(ship.ship_mut().special_counter, 0);
                assert_eq!(ship.ship_mut().energy_counter, 19);
                assert_eq!(ship.ship_mut().cur_status_flags & (1 << 5) != 0, low);
                ship.postprocess();
                assert_eq!(
                    velocity_fields(&(*ship.ptr()).velocity),
                    before,
                    "counter zero coasts unchanged"
                );
            }
            (*ship.ptr()).state_flags = PLAYER_SHIP as u16;
            (*ship.ptr()).thrust = 0;
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
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
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
                (*threat.ptr()).state_flags = flags as u16;
                let mut c = concerns();
                let w = &mut c[ENEMY_WEAPON_INDEX as usize];
                w.ObjectPtr = threat.ptr();
                w.which_turn = turn;
                w.MoveState = MOVEMENT_STATE_ENTICE;
                with_rng(7, 0, || think(&mut ship, &mut c, SPECIAL as u8, 0));
                assert_eq!(!c[ENEMY_WEAPON_INDEX as usize].ObjectPtr.is_null(), keep);
                assert_eq!(c[ENEMY_WEAPON_INDEX as usize].MoveState, movement);
            }
            let mut c = concerns();
            c[ENEMY_WEAPON_INDEX as usize].ObjectPtr = threat.ptr();
            c[ENEMY_WEAPON_INDEX as usize].MoveState = MOVEMENT_STATE_AVOID;
            with_rng(7, 0, || think(&mut ship, &mut c, 0, 0));
            assert_eq!(c[ENEMY_WEAPON_INDEX as usize].ObjectPtr, threat.ptr());
            assert_eq!(
                c[ENEMY_WEAPON_INDEX as usize].MoveState,
                MOVEMENT_STATE_AVOID
            );
        }
    })
}

#[test]
fn short_range_ai_resets_special_wait_and_fires_only_before_turn_16() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let enemy = Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 1, [16, 16, 48], [4, 4]);
            for reason in 0..3 {
                for turn in [15, 16] {
                    ship.ship_mut().special_counter = if reason == 0 { 1 } else { 0 };
                    let mut c = enemy_concern(&enemy, turn);
                    if reason == 1 {
                        c[GRAVITY_MASS_INDEX as usize].ObjectPtr = enemy.ptr();
                    }
                    if reason == 2 {
                        c[0].ObjectPtr = ptr::null_mut();
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
                    assert_eq!(ship.descriptor_mut().cyborg_control.WeaponRange, 200);
                    assert_eq!(ship.descriptor_mut().characteristics.special_wait, 255);
                }
            }
        }
    })
}

#[test]
fn rearward_zip_uses_one_draw_with_both_random_turn_directions() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let enemy = Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 1, [16, 16, 48], [4, 4]);
            (*enemy.ptr()).next.location = POINT { x: 4000, y: 4400 };
            for odd in [false, true] {
                let seed = seed_where(|d| (d[0] & 1 != 0) == odd);
                let mut c = enemy_concern(&enemy, 8);
                let input = with_rng(seed, 1, || think(&mut ship, &mut c, THRUST as u8, 0));
                assert_eq!(input, (SPECIAL | if odd { LEFT } else { RIGHT }) as u8);
                assert_eq!(ship.descriptor_mut().characteristics.special_wait, 8);
                assert_eq!(ship.descriptor_mut().cyborg_control.WeaponRange, 32000);
            }
        }
    })
}

#[test]
fn ai_rng_follows_shared_steering_and_uses_old_turn_flags_for_alignment() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let enemy = Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 1, [16, 16, 48], [4, 4]);
            (*enemy.ptr()).current.location = POINT { x: 4000, y: 4400 };
            (*enemy.ptr()).next = (*enemy.ptr()).current.clone();
            ship.ship_mut().weapon_counter = 1;
            for first_odd in [false, true] {
                let seed =
                    seed_where(|d| (d[0] & 1 != 0) == first_odd && (d[1] & 1 != 0) != first_odd);
                ship.ship_mut().ship_facing = 0;
                (*ship.ptr()).current.image.frame = ship.frame(0, 0);
                (*ship.ptr()).next = (*ship.ptr()).current.clone();
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
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let enemy = Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 1, [16, 16, 48], [4, 4]);
            (*enemy.ptr()).next.location = POINT { x: 4000, y: 4400 };
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
                (*ship.ptr()).turn = wait;
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
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let enemy = Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 1, [16, 16, 48], [4, 4]);
            (*enemy.ptr()).next.location = POINT { x: 4000, y: 4400 };
            (*ship.ptr()).turn = 1;
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
                    ship.descriptor_mut().cyborg_control.WeaponRange,
                    if boost { 32000 } else { 200 }
                );
                assert_eq!(input & THRUST as u8 != 0, !boost);
            }
        }
    })
}

#[test]
fn ai_uses_enemy_maneuverability_and_inclusive_16_through_24_range() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let mut enemy =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 1, [16, 16, 48], [4, 4]);
            (*enemy.ptr()).next.location = POINT { x: 4000, y: 3600 };
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
                enemy.descriptor_mut().cyborg_control.ManeuverabilityIndex = maneuver;
                let mut c = enemy_concern(&enemy, turn);
                let input = with_rng(9, 0, || think(&mut ship, &mut c, THRUST as u8, 0));
                assert_eq!(
                    ship.descriptor_mut().cyborg_control.WeaponRange,
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
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let enemy = Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 1, [16, 16, 48], [4, 4]);
            (*enemy.ptr()).next.location = POINT { x: 4000, y: 3600 };
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
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 0, [16, 16, 48], [4, 4]);
            let enemy = Ship::with_frames(&_battle, SPECIES_ID_UMGAH_ID, 1, [16, 16, 48], [4, 4]);
            (*ship.ptr()).turn = 1;
            for facing in 0..16 {
                ship.ship_mut().ship_facing = facing;
                for angle in 0..64 {
                    let dx = cosine(angle, 1000);
                    let dy = sine(angle, 1000);
                    (*enemy.ptr()).next.location = POINT {
                        x: 4000 + dx,
                        y: 4000 + dy,
                    };
                    let direction = ARCTAN(dx, dy);
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
            (*ship.ptr()).next.location = POINT {
                x: 4000,
                y: i16::MAX - 1,
            };
            (*enemy.ptr()).next.location = POINT {
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

unsafe fn fire(ship: &mut Ship<'_>) -> HLINK {
    let h = ship.initialize_from(ship.ptr(), 1)[0];
    if !h.is_null() {
        assert!((*battle::element(h)).parent == uqm_rust::engine::battle::Parent::None);
        (*battle::element(h)).parent = (*ship.ptr()).parent;
    }
    h
}
unsafe fn previous_facing(ship: &mut Ship<'_>) -> Option<u16> {
    let uqm_rust::engine::battle::RaceState::Umgah { previous_facing } =
        ship.descriptor_mut().race_state
    else {
        panic!("Umgah fixture state")
    };
    previous_facing
}

#[path = "support/hooks.rs"]
mod hooks;
