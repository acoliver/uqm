//! Live Thraddash callbacks, using the engine queue, frames, velocity and RNG.

#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
#[path = "support/special_hooks.rs"]
mod special_hooks;
use battle::{prim, Battle, FrameSet, PoolElement, PoolShip, Ship};
use std::{ffi::CStr, mem, ptr};
use uqm_rust::collections::observation;
use uqm_rust::graphics::context_abi::_GraphicsStatusFlags;
use uqm_rust::ships::native::{abi::*, cosine, facing_to_angle, frame_index, sine};

unsafe fn burn(ship: &mut Ship<'_>) -> *mut ElementRecord {
    ship.ship_mut().cur_status_flags |= SPECIAL as u16;
    let head = battle::head();
    ship.preprocess();
    let trail = battle::head();
    assert_ne!(trail, head);
    battle::element(trail)
}
unsafe fn fire(ship: &mut Ship<'_>) -> *mut std::ffi::c_void {
    ship.weapon_handles(1)[0]
}
unsafe fn queued() -> Vec<*mut std::ffi::c_void> {
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
    let (velocity_x, velocity_y) = ((*e).velocity).get_current_components();
    let (x, y) = (velocity_x as i16, velocity_y as i16);
    (x, y)
}

unsafe fn with_rng<T>(seed: DWORD, used: usize, action: impl FnOnce() -> T) -> T {
    TFB_SeedRandom(seed);
    for _ in 0..used {
        TFB_Random();
    }
    let expected = TFB_Random();
    TFB_SeedRandom(seed);
    let result = action();
    assert_eq!(TFB_Random(), expected, "RNG consumption: {used}");
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

unsafe fn think(ship: &mut Ship, c: &mut [EVALUATE_DESC; 4], input: u8, count: COUNT) -> u8 {
    ship.ship_mut().ship_input_state = input;
    ship.think(ship.ptr(), c, count);
    ship.ship_mut().ship_input_state
}

#[test]
fn descriptor_matches_all_c_initializer_fields_and_resources() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let d = battle::descriptor(SPECIES_ID_THRADDASH_ID);
            assert_eq!(d.ship_info.ship_flags, FIRES_FORE as u16);
            assert_eq!(d.ship_info.ship_cost, 10);
            assert_eq!((d.ship_info.crew_level, d.ship_info.max_crew), (8, 8));
            assert_eq!((d.ship_info.energy_level, d.ship_info.max_energy), (24, 24));
            assert_eq!(
                CStr::from_ptr(d.ship_info.race_strings_rsc),
                c"ship.thraddash.text"
            );
            assert_eq!(
                CStr::from_ptr(d.ship_info.icons_rsc),
                c"ship.thraddash.icons"
            );
            assert_eq!(
                CStr::from_ptr(d.ship_info.melee_icon_rsc),
                c"ship.thraddash.meleeicons"
            );
            assert!(
                d.ship_info.race_strings.is_null()
                    && d.ship_info.icons.is_null()
                    && d.ship_info.melee_icon.is_null()
            );
            assert_eq!(d.fleet.strength, 150);
            assert_eq!((d.fleet.known_loc.x, d.fleet.known_loc.y), (2535, 8358));
            let c = d.characteristics;
            assert_eq!(
                (c.max_thrust, c.thrust_increment, c.energy_regeneration),
                (28, 7, 1)
            );
            assert_eq!(
                (c.weapon_energy_cost, c.special_energy_cost, c.energy_wait),
                (2, 1, 6)
            );
            assert_eq!(
                (
                    c.turn_wait,
                    c.thrust_wait,
                    c.weapon_wait,
                    c.special_wait,
                    c.ship_mass
                ),
                (1, 0, 12, 0, 7)
            );
            for (i, size) in ["large", "medium", "small"].iter().enumerate() {
                for (rsc, name) in [
                    (d.ship_data.ship_rsc[i], "torch"),
                    (d.ship_data.weapon_rsc[i], "horn"),
                    (d.ship_data.special_rsc[i], "napalm"),
                ] {
                    assert_eq!(
                        CStr::from_ptr(rsc).to_str().unwrap(),
                        format!("ship.thraddash.graphics.{name}.{size}")
                    );
                }
                assert!(
                    d.ship_data.ship.view(i).is_absent()
                        && d.ship_data.weapon.view(i).is_absent()
                        && d.ship_data.special.view(i).is_absent()
                );
            }
            let captain = &d.ship_data.captain_control;
            assert_eq!(
                CStr::from_ptr(captain.captain_rsc),
                c"ship.thraddash.graphics.captain"
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
                c"ship.thraddash.ditty"
            );
            assert_eq!(
                CStr::from_ptr(d.ship_data.ship_sounds_rsc),
                c"ship.thraddash.sounds"
            );
            assert!(d.ship_data.victory_ditty.is_null() && d.ship_data.ship_sounds.is_null());
            assert_eq!(
                (
                    d.cyborg_control.ManeuverabilityIndex,
                    d.cyborg_control.WeaponRange
                ),
                (0, 900)
            );
            assert!(d.callbacks()[1] != 0 && d.callbacks()[3] != 0 && d.callbacks()[4] != 0);
            assert!(d.callbacks()[2] == 0 && d.callbacks()[0] == 0);
            let mut owned = battle::descriptor(SPECIES_ID_THRADDASH_ID);
            owned.ship_info.energy_level = 17;
            owned.race_callbacks.pre = uqm_rust::engine::battle::Selection::Disabled;
            let fresh = battle::descriptor(SPECIES_ID_THRADDASH_ID);
            assert_eq!(fresh.callbacks(), d.callbacks());
            assert_eq!(fresh.ship_info.energy_level, 24);
            assert_eq!(owned.ship_info.energy_level, 17);
            assert_eq!(owned.callbacks()[1], 0);
        }
    })
}

#[test]
fn horn_uses_next_location_exact_facing_speed_and_offsets_without_inheriting_velocity() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_THRADDASH_ID, 1, [16, 16, 10], [4, 4]);
            ((*ship.ptr()).velocity).set_components(1000, -1000);
            (*ship.ptr()).next.location = POINT {
                x: i16::MAX - 5,
                y: i16::MIN + 5,
            };
            for facing in 0..16 {
                ship.ship_mut().ship_facing = facing;
                let h = with_rng(101, 0, || fire(&mut ship));
                assert!(!h.is_null());
                let e = battle::element(h);
                let angle = facing_to_angle(i32::from(facing));
                let vx = cosine(angle, 120 << VELOCITY_SHIFT);
                let vy = sine(angle, 120 << VELOCITY_SHIFT);
                assert_eq!(velocity(e), (vx, vy));
                assert_eq!(
                    (*e).current.location.x,
                    (i32::from(i16::MAX) - 5 + i32::from(cosine(angle, 36))
                        - (i32::from(vx) >> VELOCITY_SHIFT)) as i16
                );
                assert_eq!(
                    (*e).current.location.y,
                    (i32::from(i16::MIN) + 5 + i32::from(sine(angle, 36))
                        - (i32::from(vy) >> VELOCITY_SHIFT)) as i16
                );
                assert_eq!(
                    (*e).current.image.frame.raw(),
                    ship.raw_frame(1, facing as usize)
                );
                assert_eq!((*e).current.image.frames, ship.frame_set(1));
                assert_eq!(
                    (*e).state_flags,
                    (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
                );
                assert_eq!((*e).life, 15);
                assert_eq!((*e).hits, 2);
                assert_eq!((*e).mass, 1);
                assert_eq!((*e).thrust, 3);
                assert_eq!((*e).playerNr, 1);
                assert!(
                    (*e).preprocess_func.is_none()
                        && (*e).postprocess_func.is_none()
                        && (*e).death_func.is_none()
                );
                assert!((*e).collision_func.is_some());
                assert!((*e).parent == uqm_rust::engine::battle::Parent::None);
                assert_eq!((*prim(e)).Type, gfx_object_STAMP_PRIM as u8);
                assert_eq!(queued(), vec![ship.handle]);
                battle_context::dispose(h);
            }
        }
    })
}

#[test]
fn failed_horn_and_napalm_allocation_keep_the_c_return_and_energy_semantics() {
    battle_context::run(|| {
        let _battle = Battle::new(1);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_THRADDASH_ID, 0, [16, 16, 10], [4, 4]);
            assert!(with_rng(2, 0, || fire(&mut ship)).is_null());
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            with_rng(2, 0, || ship.preprocess());
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 23);
            assert_eq!(ship.ship_mut().energy_counter, 6);
            assert_eq!(velocity(ship.ptr()), (0, -12 * 32));
            assert_eq!((*ship.ptr()).thrust, 1);
            assert_eq!(queued(), vec![ship.handle]);
            assert_eq!(
                (
                    ship.descriptor_mut().characteristics.max_thrust,
                    ship.descriptor_mut().characteristics.thrust_increment
                ),
                (28, 7)
            );
        }
    })
}

#[test]
fn afterburner_inserts_owned_invisible_napalm_at_the_head_in_newest_first_order() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_THRADDASH_ID, 1, [16, 16, 10], [4, 4]);
            ship.ship_mut().ship_facing = 7;
            (*ship.ptr()).next.location = POINT { x: -9, y: 13 };
            observation::enable();
            observation::drain();
            let first = with_rng(31, 0, || burn(&mut ship));
            assert_eq!(special_hooks::count(&observation::drain(), "thraddash"), 1);
            assert_eq!(queued(), vec![first.cast(), ship.handle]);
            let second = with_rng(31, 0, || burn(&mut ship));
            assert_eq!(special_hooks::count(&observation::drain(), "thraddash"), 1);
            assert_eq!(queued(), vec![second.cast(), first.cast(), ship.handle]);
            for e in [first, second] {
                assert_eq!(
                    (*e).parent,
                    uqm_rust::engine::battle::Parent::Ship(ship.ship_id)
                );
                assert!((*e).target.is_none());
                assert_eq!(
                    (*e).state_flags,
                    (APPEARING | FINITE_LIFE | IGNORE_SIMILAR | NONSOLID | PRE_PROCESS) as u16
                );
                assert_eq!((*e).life, 48);
                assert_eq!((*e).hits, 1);
                assert_eq!((*e).mass, 2);
                assert_eq!((*e).turn, 5);
                assert_eq!((*e).thrust, 0);
                assert_eq!((*e).playerNr, 1);
                assert_eq!((*e).current.image.frame.raw(), ship.raw_frame(2, 9));
                assert_eq!((*e).next.image.frame.raw(), (*e).current.image.frame.raw());
                assert_eq!((*e).next.image.frames, (*e).current.image.frames);
                assert_eq!(((*e).current.location.x, (*e).current.location.y), (-9, 13));
                assert_eq!(((*e).next.location.x, (*e).next.location.y), (-9, 13));
                assert_eq!(velocity_fields(&(*e).velocity), [0; 9]);
                assert_eq!((*prim(e)).Type, gfx_object_NUM_PRIMS as u8);
                assert!((*e).preprocess_func.is_some() && (*e).collision_func.is_some());
            }
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 22);
        }
    })
}

#[test]
fn boost_uses_temporary_characteristics_and_preserves_the_inertial_thrust_result() {
    battle_context::run(|| {
        let _battle = Battle::new(64);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_THRADDASH_ID, 0, [16, 16, 10], [4, 4]);
            for (speed, face, old, status, wait) in [
                (0, 0, 0, 0, 0),
                (28, 0, 0, SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED, 0),
                (72, 0, SPECIAL, SHIP_AT_MAX_SPEED, 3),
                (
                    80,
                    4,
                    SPECIAL,
                    SHIP_AT_MAX_SPEED | SHIP_IN_GRAVITY_WELL,
                    255,
                ),
            ] {
                ship.ship_mut().ship_facing = face;
                SetVelocityVector(&raw mut (*ship.ptr()).velocity, speed, 0);
                ship.ship_mut().old_status_flags = old as u16;
                ship.ship_mut().cur_status_flags = (SPECIAL | THRUST | status) as u16;
                (*ship.ptr()).thrust = wait;
                let mut expected_desc = battle::descriptor(SPECIES_ID_THRADDASH_ID);
                expected_desc.characteristics = ship.descriptor_mut().characteristics;
                expected_desc.ship_info = ship.descriptor_mut().ship_info;
                expected_desc.characteristics.max_thrust = 72;
                expected_desc.characteristics.thrust_increment = 12;
                let mut expected_ss = PoolShip::new(0);
                expected_ss.SpeciesID = ship.ship_mut().SpeciesID;
                expected_ss.hShip = ship.ship_mut().hShip;
                uqm_rust::app::bridge::enter(|cx| {
                    let state = cx.ship_state(ship.ship_id);
                    *cx.ship_state_mut(expected_ss.ship_id) = state;
                });
                expected_ss.descriptor = Some(Box::new(expected_desc));
                if old == 0 {
                    expected_ss.cur_status_flags &=
                        !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16);
                }
                let mut expected = PoolElement::new();
                let thrust_status = uqm_rust::app::bridge::enter(|cx| {
                    let id = cx
                        .elements()
                        .id(std::ptr::NonNull::new((&raw mut *expected).cast()))
                        .unwrap();
                    cx.copy_allocated_element(id, ship.id);
                    expected.parent = uqm_rust::engine::battle::Parent::Ship(expected_ss.ship_id);
                    inertial_thrust(cx, id)
                });
                with_rng(7, 0, || burn(&mut ship));
                assert_eq!(
                    velocity_fields(&(*ship.ptr()).velocity),
                    velocity_fields(&expected.velocity)
                );
                assert_eq!(
                    ship.ship_mut().cur_status_flags,
                    (SPECIAL | THRUST) as u16 | thrust_status
                );
                assert_eq!((*ship.ptr()).thrust, if wait == 0 { 1 } else { wait });
                assert_eq!(
                    (
                        ship.descriptor_mut().characteristics.max_thrust,
                        ship.descriptor_mut().characteristics.thrust_increment
                    ),
                    (28, 7)
                );
            }
        }
    })
}

#[test]
fn repeated_boost_reaches_72_and_release_marks_coasting_beyond_normal_speed() {
    battle_context::run(|| {
        let _battle = Battle::new(16);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_THRADDASH_ID, 0, [16, 16, 10], [4, 4]);
            for frame in 1..=7 {
                with_rng(45, 0, || burn(&mut ship));
                assert_eq!(velocity(ship.ptr()), (0, -32 * (frame.min(6) * 12)));
                assert_eq!(
                    ship.ship_mut().cur_status_flags & SHIP_AT_MAX_SPEED as u16 != 0,
                    frame == 7
                );
                ship.ship_mut().old_status_flags = ship.ship_mut().cur_status_flags;
            }
            let before = queued();
            ship.ship_mut().cur_status_flags &= !(SPECIAL as u16);
            ship.preprocess();
            assert_eq!(
                ship.ship_mut().cur_status_flags
                    & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16,
                (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16
            );
            assert_eq!(queued(), before);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 17);
            assert_eq!(velocity(ship.ptr()), (0, -72 * 32));
        }
    })
}

#[test]
fn no_key_and_exhausted_energy_leave_velocity_and_queue_alone() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_THRADDASH_ID, 0, [16, 16, 10], [4, 4]);
            ((*ship.ptr()).velocity).set_components(123, -456);
            let v = velocity_fields(&(*ship.ptr()).velocity);
            observation::enable();
            for (current, old, expected) in [
                (0, SPECIAL, 0),
                (SHIP_AT_MAX_SPEED, 0, SHIP_AT_MAX_SPEED),
                (
                    SHIP_AT_MAX_SPEED,
                    SPECIAL,
                    SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED,
                ),
            ] {
                ship.ship_mut().cur_status_flags = current as u16;
                ship.ship_mut().old_status_flags = old as u16;
                observation::drain();
                ship.preprocess();
                assert_eq!(special_hooks::count(&observation::drain(), "thraddash"), 0);
                assert_eq!(ship.ship_mut().cur_status_flags, expected as u16);
                assert_eq!(ship.descriptor_mut().ship_info.energy_level, 24);
            }
            ship.descriptor_mut().ship_info.energy_level = 0;
            ship.ship_mut().energy_counter = 17;
            ship.ship_mut().cur_status_flags = (SPECIAL | SHIP_AT_MAX_SPEED) as u16;
            observation::drain();
            with_rng(8, 0, || ship.preprocess());
            // Held SPECIAL with no energy to pay for it does not activate the afterburner.
            assert_eq!(special_hooks::count(&observation::drain(), "thraddash"), 0);
            assert_eq!(
                ship.ship_mut().cur_status_flags,
                (SPECIAL | SHIP_AT_MAX_SPEED | (1 << 5)) as u16
            );
            assert_eq!(ship.ship_mut().energy_counter, 17);
            assert_eq!((*ship.ptr()).thrust, 0);
            assert_eq!(velocity_fields(&(*ship.ptr()).velocity), v);
            assert_eq!(queued(), vec![ship.handle]);
        }
    })
}

#[test]
fn napalm_materializes_without_decaying_and_initializes_the_intersection_from_next_image() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_THRADDASH_ID, 0, [16, 16, 10], [4, 4]);
            let e = burn(&mut ship);
            let next_frames = FrameSet::new(10);
            (*e).next.image.frames = uqm_rust::engine::battle::frames::Frames::shared(
                uqm_rust::engine::battle::frames::SharedFrames::Fixture(0),
                next_frames.array(),
            );
            (*e).next.image.frame = next_frames.selection(7);
            (*e).current.location = POINT { x: -9, y: 13 };
            (*e).next.location = POINT { x: 42, y: -5 };
            ((*e).velocity).set_components(1001, -4003);
            (*e).velocity.error.width = 19;
            let callback = (*e)
                .preprocess_func
                .unwrap_or_else(|| panic!("installed napalm callback"));
            with_rng(66, 0, || hooks::invoke_element_pointer(callback, e));
            assert_eq!(velocity_fields(&(*e).velocity), [0; 9]);
            assert_eq!((*e).state_flags & NONSOLID as u16, 0);
            assert_ne!((*e).state_flags & APPEARING as u16, 0);
            assert_eq!((*e).state_flags & CHANGING as u16, 0);
            assert_eq!((*prim(e)).Type, gfx_object_STAMP_PRIM as u8);
            let i = (*e).IntersectControl;
            assert_eq!(
                (i.IntersectStamp.origin.x, i.IntersectStamp.origin.y),
                (-3, 3)
            );
            assert_eq!((i.EndPoint.x, i.EndPoint.y), (10, -2));
            assert_eq!(i.IntersectStamp.frame, next_frames.frame(7));
            assert_eq!((*e).turn, 5);
            assert_eq!((*e).current.image.frame.raw(), ship.raw_frame(2, 9));
            assert_eq!((*e).next.image.frame.raw(), next_frames.frame(7));
        }
    })
}

#[test]
fn napalm_decay_wait_and_loop_boundary_match_c_including_last_frame_wrap() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_THRADDASH_ID, 0, [16, 16, 10], [4, 4]);
            let e = burn(&mut ship);
            let callback = (*e)
                .preprocess_func
                .unwrap_or_else(|| panic!("installed napalm callback"));
            hooks::invoke_element_pointer(callback, e);
            (*e).state_flags &= !(APPEARING as u16);
            for wait in (0..5).rev() {
                ((*e).velocity).set_components(77, 99);
                with_rng(3, 0, || hooks::invoke_element_pointer(callback, e));
                assert_eq!((*e).turn, wait);
                assert_eq!((*e).next.image.frame.raw(), ship.raw_frame(2, 9));
                assert_eq!((*e).state_flags & CHANGING as u16, 0);
                assert_eq!(velocity_fields(&(*e).velocity), [0; 9]);
            }
            for (frame, life, expected) in [
                (9, 48, 8),
                (7, 37, 6),
                (6, 37, 9),
                (6, 36, 5),
                (6, 0, 5),
                (0, 1, 9),
            ] {
                (*e).current.image.frame = ship.frame(2, frame);
                (*e).next.image.frame = ship.frame(2, 2);
                (*e).life = life;
                (*e).turn = 0;
                (*e).state_flags &= !(CHANGING as u16);
                with_rng(3, 0, || hooks::invoke_element_pointer(callback, e));
                assert_eq!(frame_index((*e).next.image.frame.raw()), expected);
                assert_eq!((*e).turn, 5);
                assert_ne!((*e).state_flags & CHANGING as u16, 0);
                assert_eq!((*e).life, life);
            }
        }
    })
}

#[test]
fn intelligence_classifies_stationary_threshold_and_turn_limit_without_rng() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_THRADDASH_ID, 0, [16, 16, 10], [4, 4]);
            let enemy =
                Ship::with_frames(&_battle, SPECIES_ID_THRADDASH_ID, 1, [16, 16, 10], [4, 4]);
            ship.ship_mut().special_counter = 1;
            for (x, y, turn, movement) in [
                (0, 0, 8, MOVEMENT_STATE_PURSUE),
                (512, 0, 8, MOVEMENT_STATE_PURSUE),
                (-512, 0, 8, MOVEMENT_STATE_PURSUE),
                (513, 0, 8, MOVEMENT_STATE_ENTICE),
                (512, 1, 8, MOVEMENT_STATE_ENTICE),
                (0, -513, 9, MOVEMENT_STATE_PURSUE),
                (i16::MAX, i16::MIN, 8, MOVEMENT_STATE_ENTICE),
            ] {
                ((*enemy.ptr()).velocity).set_components(i32::from(x), i32::from(y));
                let mut c = concerns();
                c[0].ObjectPtr = enemy.ptr();
                c[0].which_turn = turn;
                let input = with_rng(4, 0, || {
                    think(&mut ship, &mut c, (SPECIAL | WEAPON) as u8, 0)
                });
                assert_eq!(c[0].MoveState, movement, "velocity {x},{y}, turn {turn}");
                assert_eq!(
                    input, SPECIAL as u8,
                    "shared pass clears a ready weapon before special-counter exit"
                );
            }
        }
    })
}

#[test]
fn pursuit_boost_requires_three_energy_ready_turn_and_no_active_speed_limit() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_THRADDASH_ID, 0, [16, 16, 10], [4, 4]);
            let enemy =
                Ship::with_frames(&_battle, SPECIES_ID_THRADDASH_ID, 1, [16, 16, 10], [4, 4]);
            for (energy, wait, input, current, boost) in [
                (2, 0, 0, 0, false),
                (3, 0, 0, 0, true),
                (3, 1, 0, 0, false),
                (3, 0, LEFT, 0, false),
                (3, 0, RIGHT, 0, false),
                (3, 0, 0, SPECIAL | SHIP_AT_MAX_SPEED, false),
                (3, 0, 0, SPECIAL | SHIP_BEYOND_MAX_SPEED, false),
                (3, 0, 0, SPECIAL, true),
                (3, 0, 0, SHIP_AT_MAX_SPEED, true),
            ] {
                ship.descriptor_mut().ship_info.energy_level = energy;
                ship.ship_mut().cur_status_flags = current as u16;
                (*ship.ptr()).turn = wait;
                let mut c = concerns();
                c[0].ObjectPtr = enemy.ptr();
                c[0].which_turn = 9;
                let result = with_rng(5, 0, || {
                    think(&mut ship, &mut c, (input | THRUST | SPECIAL) as u8, 0)
                });
                assert_eq!(result & SPECIAL as u8 != 0, boost);
                assert_eq!(result & THRUST as u8 != 0, !boost);
            }
            let mut c = concerns();
            assert_eq!(think(&mut ship, &mut c, SPECIAL as u8, 0), 0);
        }
    })
}

#[test]
fn enticing_weapon_takes_priority_and_uses_rounded_wrapped_travel_angle() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_THRADDASH_ID, 0, [16, 16, 10], [4, 4]);
            let enemy =
                Ship::with_frames(&_battle, SPECIES_ID_THRADDASH_ID, 1, [16, 16, 10], [4, 4]);
            let weapon =
                Ship::with_frames(&_battle, SPECIES_ID_THRADDASH_ID, 1, [16, 16, 10], [4, 4]);
            (*ship.ptr()).turn = 1;
            for facing in 0..16 {
                ship.ship_mut().ship_facing = facing;
                for angle in 0..64 {
                    (*weapon.ptr()).velocity.travel_angle = angle;
                    let mut c = concerns();
                    c[0].ObjectPtr = enemy.ptr();
                    c[0].which_turn = 9;
                    c[ENEMY_WEAPON_INDEX as usize].ObjectPtr = weapon.ptr();
                    c[ENEMY_WEAPON_INDEX as usize].MoveState = MOVEMENT_STATE_ENTICE;
                    let input = with_rng(73, 0, || think(&mut ship, &mut c, 0, 0));
                    let expected =
                        ((((i32::from(angle) + 32 + 8 + 2) >> 2) - i32::from(facing)) & 15) > 4;
                    assert_eq!(
                        input & SPECIAL as u8 != 0,
                        expected,
                        "facing {facing}, angle {angle}"
                    );
                }
            }
            (*weapon.ptr()).velocity.travel_angle = 32;
            ship.ship_mut().ship_facing = 0;
            for (energy, wait, input, expected) in [
                (0, 1, THRUST, THRUST | SPECIAL),
                (1, 1, THRUST, SPECIAL),
                (1, 0, 0, SPECIAL),
                (1, 0, LEFT, LEFT),
            ] {
                ship.descriptor_mut().ship_info.energy_level = energy;
                (*ship.ptr()).turn = wait;
                let mut c = concerns();
                c[ENEMY_WEAPON_INDEX as usize].ObjectPtr = weapon.ptr();
                c[ENEMY_WEAPON_INDEX as usize].MoveState = MOVEMENT_STATE_ENTICE;
                assert_eq!(think(&mut ship, &mut c, input as u8, 0), expected as u8);
            }
        }
    })
}

#[test]
fn entice_ship_geometry_uses_next_locations_and_size_truncation() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_THRADDASH_ID, 0, [16, 16, 10], [4, 4]);
            let enemy =
                Ship::with_frames(&_battle, SPECIES_ID_THRADDASH_ID, 1, [16, 16, 10], [4, 4]);
            ((*enemy.ptr()).velocity).set_components(513, 0);
            for (ship_y, enemy_y, boost) in [
                (4000, 4400, true),
                (4000, 3600, false),
                (i16::MAX - 1, i16::MIN + 1, true),
            ] {
                (*ship.ptr()).next.location = POINT { x: 4000, y: ship_y };
                (*enemy.ptr()).next.location = POINT {
                    x: 4000,
                    y: enemy_y,
                };
                let mut c = concerns();
                c[0].ObjectPtr = enemy.ptr();
                c[0].which_turn = 8;
                let input = with_rng(19, 0, || think(&mut ship, &mut c, THRUST as u8, 0));
                assert_eq!(c[0].MoveState, MOVEMENT_STATE_ENTICE);
                assert_eq!(input & SPECIAL as u8 != 0, boost);
                assert_eq!(input & THRUST as u8 != 0, !boost);
            }
        }
    })
}

#[test]
fn shared_ai_steers_before_thraddash_decides_whether_to_boost() {
    battle_context::run(|| {
        let _battle = Battle::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_THRADDASH_ID, 0, [16, 16, 10], [4, 4]);
            let enemy =
                Ship::with_frames(&_battle, SPECIES_ID_THRADDASH_ID, 1, [16, 16, 10], [4, 4]);
            (*enemy.ptr()).current.location = POINT { x: 4400, y: 4000 };
            (*enemy.ptr()).next = (*enemy.ptr()).current.clone();
            ship.ship_mut().weapon_counter = 1;
            let mut c = concerns();
            c[0].ObjectPtr = enemy.ptr();
            c[0].which_turn = 9;
            let input = with_rng(8, 0, || think(&mut ship, &mut c, 0, 1));
            assert_ne!(input & RIGHT as u8, 0);
            assert_eq!(input & (LEFT | SPECIAL) as u8, 0);
            assert_eq!(frame_index((*ship.ptr()).next.image.frame.raw()), 1);
        }
    })
}

#[test]
fn owner_cleanup_during_collision_scan_invalidates_the_active_napalm() {
    battle_context::run(|| {
        let _battle = Battle::new(16);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_THRADDASH_ID, 0, [16, 16, 10], [4, 4]);
            let mut enemy =
                Ship::with_frames(&_battle, SPECIES_ID_THRADDASH_ID, 1, [16, 16, 10], [4, 4]);
            let trail = burn(&mut ship);
            hooks::invoke_element_pointer(
                (*trail)
                    .preprocess_func
                    .unwrap_or_else(|| panic!("installed napalm callback")),
                trail,
            );
            (*trail).state_flags &= !(APPEARING as u16);
            (*trail).state_flags |= PRE_PROCESS as u16;
            (*trail).life = 20;
            // Avoid a spawn overlap, so the unfixed traversal reaches its callback.
            (*enemy.ptr()).state_flags |= (PRE_PROCESS | FINITE_LIFE) as u16;
            (*enemy.ptr()).collision_func = Some(uqm_rust::engine::battle::CollisionHook::Native(
                uqm_rust::battle::ship_native::collision,
            ));
            (*enemy.ptr()).current.location = (*trail).current.location;
            (*enemy.ptr()).next = (*enemy.ptr()).current.clone();
            ship.make_opaque(2);
            enemy.make_opaque(0);
            (*enemy.ptr()).IntersectControl = (*trail).IntersectControl;
            (*enemy.ptr()).IntersectControl.IntersectStamp.frame = enemy.raw_frame(0, 0);
            (*ship.ptr()).life = 0;
            (*ship.ptr()).death_func = Some(uqm_rust::engine::battle::ElementHook::Native(
                uqm_rust::battle::tactrans_native::cleanup_dead_ship,
            ));
            _GraphicsStatusFlags |= 4;
            let result = uqm_rust::app::bridge::enter(|cx| {
                let next = cx.elements().id(std::ptr::NonNull::new(ship.handle));
                let element = cx
                    .elements()
                    .id(std::ptr::NonNull::new(trail.cast()))
                    .unwrap_or_else(|| panic!("live trail fixture"));
                uqm_rust::battle::process_native::process_collisions(
                    cx,
                    next,
                    element,
                    MAX_TIME_VALUE as TIME_VALUE,
                    PRE_PROCESS as u16,
                )
            });
            assert_ne!(result & COLLISION as u16, 0);
            assert!((*trail).collision_func.is_none());
            assert_ne!((*trail).state_flags & (NONSOLID | DISAPPEARING) as u16, 0);
            assert_eq!(enemy.descriptor_mut().ship_info.crew_level, 8);
        }
    })
}

#[test]
fn napalm_and_horn_collide_with_battle_objects_without_optional_callback_skips() {
    battle_context::run(|| {
        // The non-solid categories deliberately have no collision callback, just
        // like their constructors. Solid categories use the live collision code.
        for (flags, hp, mass, damage) in [
            (PLAYER_SHIP, 8, 7, true),
            (0, 1, 3, true),           // asteroid
            (0, 200, 200, false),      // planet
            (FINITE_LIFE, 3, 1, true), // missile
            (FINITE_LIFE, 1, 1, true), // laser
            (FINITE_LIFE | CREW_OBJECT, 1, 0, true),
            (FINITE_LIFE | NONSOLID, 1, 1, false), // blast/rubble/ion trail
            (FINITE_LIFE | DISAPPEARING, 1, 1, false),
        ] {
            for napalm in [false, true] {
                let _battle = Battle::new(16);
                unsafe {
                    let mut ship = Ship::with_frames(
                        &_battle,
                        SPECIES_ID_THRADDASH_ID,
                        0,
                        [16, 16, 10],
                        [4, 4],
                    );
                    let mut target = Ship::with_frames(
                        &_battle,
                        SPECIES_ID_THRADDASH_ID,
                        1,
                        [16, 16, 10],
                        [4, 4],
                    );
                    let weapon = if napalm {
                        let trail = burn(&mut ship);
                        hooks::invoke_element_pointer(
                            (*trail)
                                .preprocess_func
                                .unwrap_or_else(|| panic!("installed napalm callback")),
                            trail,
                        );
                        ship.make_opaque(2);
                        trail
                    } else {
                        let horn = battle::element(fire(&mut ship));
                        (*horn).next = (*horn).current.clone();
                        ship.make_opaque(1);
                        horn
                    };
                    (*weapon).state_flags &= !(APPEARING as u16);
                    (*weapon).state_flags |= PRE_PROCESS as u16;
                    (*weapon).IntersectControl.IntersectStamp.origin = POINT { x: 1000, y: 1000 };
                    (*weapon).IntersectControl.EndPoint = POINT { x: 1000, y: 1000 };
                    (*weapon).IntersectControl.IntersectStamp.frame =
                        (*weapon).next.image.frame.raw();
                    target.make_opaque(0);
                    let t = target.ptr();
                    (*t).state_flags = flags as u16 | PRE_PROCESS as u16;
                    (*t).life = NORMAL_LIFE as u16;
                    (*t).hits = hp;
                    (*t).mass = mass;
                    (*t).collision_func = if flags & (NONSOLID | DISAPPEARING) == 0 {
                        Some(uqm_rust::engine::battle::CollisionHook::Native(
                            uqm_rust::battle::ship_native::collision,
                        ))
                    } else {
                        None
                    };
                    (*t).IntersectControl = (*weapon).IntersectControl;
                    (*t).IntersectControl.IntersectStamp.frame = target.raw_frame(0, 0);
                    let mut context: uqm_rust::graphics::native_abi::context_desc = mem::zeroed();
                    let old_context = uqm_rust::graphics::context_abi::SetContext(&mut context);
                    uqm_rust::app::bridge::enter(|cx| {
                        let next = cx.elements().id(std::ptr::NonNull::new(target.handle));
                        let element = cx
                            .elements()
                            .id(std::ptr::NonNull::new(weapon.cast()))
                            .unwrap_or_else(|| panic!("live weapon fixture"));
                        uqm_rust::battle::process_native::process_collisions(
                            cx,
                            next,
                            element,
                            MAX_TIME_VALUE as TIME_VALUE,
                            PRE_PROCESS as u16,
                        )
                    });
                    uqm_rust::graphics::context_abi::SetContext(old_context);
                    let expected = if damage {
                        hp - if napalm { 2.min(hp) } else { 1 }
                    } else {
                        hp
                    };
                    assert_eq!((*t).hits, expected, "flags={flags:x} napalm={napalm}");
                    assert!((*weapon).collision_func.is_some());
                }
            }
        }
    })
}

#[path = "support/hooks.rs"]
mod hooks;
