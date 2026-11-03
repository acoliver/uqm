//! Linked callback checks against the Chmmr C rules, without a running renderer.

use uqm_rust::engine::battle::schema::HLINK;
#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
#[path = "support/special_hooks.rs"]
mod special_hooks;
use battle::{prim, PoolElement, Ship};
use uqm_rust::collections::observation;

use std::{ffi::CStr, mem, ptr};
use uqm_rust::ships::native::{abi::*, cosine, facing_to_angle, frame_index, sine};

use battle::Battle as Battlefield;

// Layouts from libs/graphics/drawable.h. These are real frame arrays and parent
// descriptors, so SetAbsFrameIndex and GetFrameRect exercise the engine ABI.

fn rgba(c: Color) -> [u8; 4] {
    [c.r, c.g, c.b, c.a]
}
fn point(p: POINT) -> (i16, i16) {
    (p.x, p.y)
}

unsafe fn velocity(el: *mut ElementRecord) -> (SIZE, SIZE) {
    let (vx, vy) = ((*el).velocity).get_current_components();
    let (mut x, mut y) = (vx as i16, vy as i16);
    (x, y)
}

fn velocity_fields(v: &VELOCITY_DESC) -> [i16; 9] {
    [
        v.travel_angle as i16,
        v.vector.width,
        v.vector.height,
        v.fract.width,
        v.fract.height,
        v.error.width,
        v.error.height,
        v.incr.width,
        v.incr.height,
    ]
}

unsafe fn assert_rng_after(seed: u32, draws: usize) {
    let actual = TFB_Random();
    TFB_SeedRandom(seed);
    for _ in 0..draws {
        TFB_Random();
    }
    assert_eq!(actual, TFB_Random(), "RNG draws for seed {seed}");
}

unsafe fn exhaust() -> Vec<HLINK> {
    let mut handles = Vec::new();
    loop {
        let h = uqm_rust::app::bridge::enter(|cx| {
            cx.alloc_element()
                .map_or(std::ptr::null_mut(), |id| cx.elements().raw(id))
        });
        if h.is_null() {
            return handles;
        }
        handles.push(h);
    }
}

#[test]
fn descriptor_matches_the_entire_c_template_and_init_reinstalls_callbacks() {
    battle_context::run(|| {
        let _battle = Battlefield::new(8);
        unsafe {
            let d = battle::descriptor(SPECIES_ID_CHMMR_ID);
            assert_eq!(
                d.ship_info.ship_flags,
                (FIRES_FORE | IMMEDIATE_WEAPON | SEEKING_SPECIAL | POINT_DEFENSE) as u16
            );
            assert_eq!(d.ship_info.ship_cost, 30);
            assert_eq!((d.ship_info.crew_level, d.ship_info.max_crew), (42, 42));
            assert_eq!((d.ship_info.energy_level, d.ship_info.max_energy), (42, 42));
            assert_eq!(
                CStr::from_ptr(d.ship_info.race_strings_rsc),
                c"ship.chmmr.text"
            );
            assert_eq!(CStr::from_ptr(d.ship_info.icons_rsc), c"ship.chmmr.icons");
            assert_eq!(
                CStr::from_ptr(d.ship_info.melee_icon_rsc),
                c"ship.chmmr.meleeicons"
            );
            assert!(d.ship_info.race_strings.is_null());
            assert!(d.ship_info.icons.is_null());
            assert!(d.ship_info.melee_icon.is_null());
            assert_eq!((d.fleet.strength, point(d.fleet.known_loc)), (0, (0, 0)));
            let c = d.characteristics;
            assert_eq!(
                (c.max_thrust, c.thrust_increment, c.energy_regeneration),
                (35, 7, 1)
            );
            assert_eq!(
                (c.weapon_energy_cost, c.special_energy_cost, c.energy_wait),
                (2, 1, 1)
            );
            assert_eq!(
                (
                    c.turn_wait,
                    c.thrust_wait,
                    c.weapon_wait,
                    c.special_wait,
                    c.ship_mass
                ),
                (3, 5, 0, 0, 10)
            );
            for (i, size) in ["large", "medium", "small"].iter().enumerate() {
                assert_eq!(
                    CStr::from_ptr(d.ship_data.ship_rsc[i]).to_str().unwrap(),
                    format!("ship.chmmr.graphics.avatar.{size}")
                );
                assert_eq!(
                    CStr::from_ptr(d.ship_data.weapon_rsc[i]).to_str().unwrap(),
                    format!("ship.chmmr.graphics.muzzle.{size}")
                );
                assert_eq!(
                    CStr::from_ptr(d.ship_data.special_rsc[i]).to_str().unwrap(),
                    format!("ship.chmmr.graphics.satellite.{size}")
                );
                assert!(d.ship_data.ship.view(i).is_absent());
                assert!(d.ship_data.weapon.view(i).is_absent());
                assert!(d.ship_data.special.view(i).is_absent());
            }
            let captain = &d.ship_data.captain_control;
            assert_eq!(
                CStr::from_ptr(captain.captain_rsc),
                c"ship.chmmr.graphics.captain"
            );
            assert!([
                captain.background,
                captain.turn,
                captain.thrust,
                captain.weapon,
                captain.special.raw()
            ]
            .iter()
            .all(|p| p.is_null()));
            assert_eq!(
                CStr::from_ptr(d.ship_data.victory_ditty_rsc),
                c"ship.chmmr.ditty"
            );
            assert_eq!(
                CStr::from_ptr(d.ship_data.ship_sounds_rsc),
                c"ship.chmmr.sounds"
            );
            assert!(d.ship_data.victory_ditty.is_null());
            assert!(d.ship_data.ship_sounds.is_null());
            assert_eq!(
                (
                    d.cyborg_control.ManeuverabilityIndex,
                    d.cyborg_control.WeaponRange
                ),
                (0, 200)
            );
            assert!(d.race.unwrap().uninit.is_none());

            assert!(d.race.unwrap().preprocess.is_some());
            assert!(d.race.unwrap().postprocess.is_some());
            assert!(d.race.unwrap().init_weapon.is_some());
            assert!(d.race.unwrap().intelligence.is_some());
            let fresh = battle::descriptor(SPECIES_ID_CHMMR_ID);
            assert!(fresh.race.unwrap().preprocess.is_some());
            assert!(fresh.race.unwrap().postprocess.is_some());
            assert!(fresh.race.unwrap().init_weapon.is_some());
            assert!(fresh.race.unwrap().intelligence.is_some());
        }
    })
}

#[test]
fn laser_uses_frame_hotspot_alignment_facing_rgba_and_cycle() {
    battle_context::run(|| {
        let _battle = Battlefield::new(16);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            let colors = [
                [189, 0, 0, 255],
                [255, 24, 0, 255],
                [255, 140, 0, 255],
                [255, 24, 0, 255],
            ];
            for face in 0..16u16 {
                ship.ship_mut().ship_facing = face;
                ship.set_hot_spot(1, face as usize, [3, -5]);
                (*ship.ptr()).next.location = POINT { x: -101, y: 32767 };
                for counter in 0..4u8 {
                    ship.ship_mut().special_counter = counter;
                    TFB_SeedRandom(55);
                    let h = laser(&mut ship);
                    assert!(!h.is_null());
                    let laser = battle::element(h);
                    assert_eq!(point((*laser).current.location), (-116, -32752));
                    assert_eq!((*laser).hits, 1);
                    assert_eq!((*laser).mass, 2);
                    assert_eq!((*laser).life, 1);
                    assert_eq!((*laser).turn, (counter + 1) % 4);
                    assert!((*laser).death_func.is_some());
                    assert_eq!((*prim(laser)).Type, gfx_object_LINE_PRIM as u8);
                    assert_eq!(rgba((*prim(laser)).color), colors[counter as usize]);
                    let a = facing_to_angle(i32::from(face));
                    assert_eq!(
                        velocity(laser),
                        (
                            (i32::from(cosine(a, 600)) << VELOCITY_SHIFT) as SIZE,
                            (i32::from(sine(a, 600)) << VELOCITY_SHIFT) as SIZE
                        )
                    );
                    assert_rng_after(55, 0);
                    battle_context::dispose(h);
                }
            }
        }
    })
}

#[test]
fn weapon_allocation_failure_still_returns_one_null_slot() {
    battle_context::run(|| {
        let _battle = Battlefield::new(2);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            let held = exhaust();
            TFB_SeedRandom(9);
            assert!(laser(&mut ship).is_null());
            assert_rng_after(9, 0);
            for h in held {
                battle_context::dispose(h);
            }
        }
    })
}

#[test]
fn laser_death_obeys_range_byte_gate_count_draw_and_animation() {
    battle_context::run(|| {
        let _battle = Battlefield::new(16);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            let h = laser(&mut ship);
            let laser = battle::element(h);
            let death = (*laser).death_func.unwrap();
            let origin = (*ship.ptr()).current.location;
            let mut gates = [false; 2];
            for seed in 1..64 {
                for distance in [111i16, 112, 600] {
                    TFB_SeedRandom(seed);
                    let gate = TFB_Random() as u8 & 7 != 0;
                    let draw = TFB_Random() as u16;
                    gates[gate as usize] = true;
                    (*laser).current.location = POINT {
                        x: origin.x + distance,
                        y: origin.y,
                    };
                    ship.ship_mut().special_counter = 0;
                    let tail = battle::tail();
                    TFB_SeedRandom(seed);
                    hooks::invoke_element_pointer(death, laser);
                    assert_eq!(ship.ship_mut().special_counter, 1);
                    if gate && distance >= 112 {
                        let spot_handle = battle::tail();
                        assert_ne!(spot_handle, tail);
                        let spot = battle::element(spot_handle);
                        let magnitude = draw % ((distance as u16 + 1) - 112) + 112;
                        assert_eq!(
                            point((*spot).current.location),
                            (origin.x + magnitude as i16, origin.y)
                        );
                        assert_eq!(frame_index((*spot).current.image.frame.raw()), 32);
                        assert_eq!((*spot).current.image.frames, ship.frame_set(1));
                        assert_eq!((*spot).life, 9);
                        assert_eq!((*spot).thrust, 0);
                        assert_eq!((*prim(spot)).Type, gfx_object_STAMP_PRIM as u8);
                        assert_eq!(
                            (*spot).parent,
                            uqm_rust::engine::battle::Parent::Ship(ship.ship_id)
                        );
                        let animate = (*spot).preprocess_func.unwrap();
                        (*spot).turn = 1;
                        (*spot).thrust = 3;
                        hooks::invoke_element_pointer(animate, spot);
                        assert_eq!((*spot).turn, 0);
                        assert!((*spot).next.image.frame.is_absent());
                        hooks::invoke_element_pointer(animate, spot);
                        assert_eq!(frame_index((*spot).next.image.frame.raw()), 33);
                        assert_eq!((*spot).turn, 3);
                        assert_ne!((*spot).state_flags & CHANGING as u16, 0);
                        assert_rng_after(seed, 2);
                        battle_context::unlink(spot_handle);
                        battle_context::dispose(spot_handle);
                    } else {
                        assert_eq!({ battle::tail() }, tail);
                        assert_rng_after(seed, 1);
                    }
                }
            }
            assert_eq!(gates, [true, true]);
            ship.ship_mut().hShip = None;
            TFB_SeedRandom(123);
            hooks::invoke_element_pointer(death, laser);
            assert_rng_after(123, 0);
            ship.ship_mut().hShip = Some(ship.id);
            (*laser).current.location.x = origin.x + 600;
            let held = exhaust();
            TFB_SeedRandom(7);
            hooks::invoke_element_pointer(death, laser);
            assert_rng_after(7, 1);
            for h in held {
                battle_context::dispose(h);
            }
            battle_context::dispose(h);
        }
    })
}

#[test]
fn muzzle_flash_requires_friendly_appearing_tail_line_and_even_cycle() {
    battle_context::run(|| {
        let _battle = Battlefield::new(16);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            ship.ship_mut().ship_facing = 5;
            ship.ship_mut().cur_status_flags = WEAPON as u16;
            ship.postprocess(); // tail is the ship itself
            assert_eq!({ battle::tail() }, ship.handle);
            for (counter, player, appearing, kind, expected) in [
                (0, 0, true, gfx_object_LINE_PRIM, true),
                (2, 0, true, gfx_object_LINE_PRIM, true),
                (1, 0, true, gfx_object_LINE_PRIM, false),
                (3, 0, true, gfx_object_LINE_PRIM, false),
                (0, 1, true, gfx_object_LINE_PRIM, false),
                (0, 0, false, gfx_object_LINE_PRIM, false),
                (0, 0, true, gfx_object_STAMP_PRIM, false),
            ] {
                ship.ship_mut().special_counter = counter;
                let h = laser(&mut ship);
                let laser = battle::element(h);
                (*laser).playerNr = player;
                if !appearing {
                    (*laser).state_flags &= !(APPEARING as u16);
                }
                (*prim(laser)).Type = kind as u8;
                battle::put(h);
                ship.postprocess();
                assert_eq!(ship.ship_mut().special_counter, 0);
                if expected {
                    let flash_handle = battle::tail();
                    assert_ne!(flash_handle, h);
                    let flash = battle::element(flash_handle);
                    assert_eq!(
                        point((*flash).current.location),
                        point((*ship.ptr()).next.location)
                    );
                    assert_eq!(frame_index((*flash).current.image.frame.raw()), 21);
                    assert_eq!((*flash).life, 1);
                    assert_eq!((*prim(flash)).Type, gfx_object_STAMP_PRIM as u8);
                    assert_eq!(
                        (*flash).parent,
                        uqm_rust::engine::battle::Parent::Ship(ship.ship_id)
                    );
                    battle_context::unlink(flash_handle);
                    battle_context::dispose(flash_handle);
                } else {
                    assert_eq!({ battle::tail() }, h);
                }
                battle_context::unlink(h);
                battle_context::dispose(h);
            }
        }
    })
}

#[test]
fn delayed_satellites_spawn_around_live_ship_with_three_ordered_rng_draws() {
    battle_context::run(|| {
        let _battle = Battlefield::new(16);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 1, [16, 41, 12], [16, 12]);
            let delay = delayed_spawn(&mut ship);
            let transit = battle::element(delay);
            assert!(matches!(
                ship.descriptor_mut().race_callbacks.pre,
                uqm_rust::engine::battle::Selection::Disabled
            ));
            assert_eq!((*transit).life, 16);
            assert_eq!((*prim(transit)).Type, gfx_object_NUM_PRIMS as u8);
            assert_eq!(
                (*transit).state_flags,
                (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR | APPEARING) as u16
            );
            // The delayed element's position and player are not the spawn source.
            (*transit).playerNr = 0;
            (*transit).next.location = POINT { x: -1000, y: -2000 };
            (*ship.ptr()).next.location = POINT { x: 7000, y: 6000 };
            TFB_SeedRandom(77);
            let indices = [
                TFB_Random() as u16 & 7,
                TFB_Random() as u16 & 7,
                TFB_Random() as u16 & 7,
            ];
            TFB_SeedRandom(77);
            hooks::invoke_element_pointer((*transit).death_func.unwrap(), transit);
            let mut h = battle::succ(transit);
            for (i, index) in indices.into_iter().enumerate() {
                assert!(!h.is_null());
                let s = battle::element(h);
                let angle = (i as i32 * 64 + 1) / 3;
                assert_eq!((*s).playerNr, 1);
                assert_eq!((*s).turn, angle as u8);
                assert_eq!((*s).life, 2);
                assert_eq!((*s).hits, 10);
                assert_eq!((*s).mass, 10);
                assert_eq!(
                    point((*s).current.location),
                    (7000 + cosine(angle, 256), 6000 + sine(angle, 256))
                );
                assert_eq!(frame_index((*s).current.image.frame.raw()), index);
                assert_eq!((*prim(s)).Type, gfx_object_STAMP_PRIM as u8);
                assert!((*s).preprocess_func.is_some());
                assert!((*s).postprocess_func.is_some());
                assert!((*s).death_func.is_some());
                assert!((*s).collision_func.is_some());
                h = battle::succ(s);
            }
            assert!(h.is_null());
            assert_rng_after(77, 3);
        }
    })
}

#[test]
fn spawn_and_preprocess_allocation_failures_do_not_draw_rng_or_rearm() {
    battle_context::run(|| {
        let _battle = Battlefield::new(3);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            let delay = delayed_spawn(&mut ship);
            let death = (*battle::element(delay)).death_func.unwrap();
            TFB_SeedRandom(88);
            hooks::invoke_element_pointer(death, battle::element(delay)); // only one free slot
            assert_eq!((*battle::element(battle::tail())).hits, 10);
            assert_rng_after(88, 1);
            TFB_SeedRandom(88);
            hooks::invoke_element_pointer(death, battle::element(delay));
            assert_rng_after(88, 0);
            ship.ship_mut().hShip = None;
            TFB_SeedRandom(88);
            hooks::invoke_element_pointer(death, battle::element(delay));
            assert_rng_after(88, 0);
            ship.ship_mut().hShip = Some(ship.id);
            ship.descriptor_mut().race_callbacks.pre = uqm_rust::engine::battle::Selection::Default;
            let tail = battle::tail();
            ship.preprocess();
            assert!(matches!(
                ship.descriptor_mut().race_callbacks.pre,
                uqm_rust::engine::battle::Selection::Disabled
            ));
            assert_eq!({ battle::tail() }, tail);
        }
    })
}

#[test]
fn satellites_wrap_space_cap_speed_and_preserve_half_space_ties() {
    battle_context::run(|| {
        let _battle = Battlefield::new(16);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            let satellites = satellites(&mut ship);
            let s = battle::element(satellites[0]);
            let preprocess = (*s).preprocess_func.unwrap();
            for (dx, dy, wx, wy) in [
                (80, 0, 80, 0),
                (81, 0, 81, 0),
                (48, 64, 48, 64),
                (48, 65, 48, 65),
                (8190, 7677, -2, -3),
                (-8190, -7677, 2, 3),
                (4096, 3840, 4096, 3840),
                (-4096, -3840, -4096, -3840),
                (4097, 3841, -4095, -3839),
                (-4097, -3841, 4095, 3839),
            ] {
                (*s).turn = 63;
                (*s).life = u16::MAX;
                (*s).current.image.frame = ship.frame(2, 7);
                let desired_x =
                    i32::from((*ship.ptr()).next.location.x) + i32::from(cosine(0, 256));
                let desired_y = i32::from((*ship.ptr()).next.location.y) + i32::from(sine(0, 256));
                (*s).current.location = POINT {
                    x: (desired_x - dx) as i16,
                    y: (desired_y - dy) as i16,
                };
                ship.descriptor_mut().ship_info.ship_flags &= !(POINT_DEFENSE as u16);
                hooks::invoke_element_pointer(preprocess, s);
                assert_eq!((*s).life, 0);
                assert_eq!((*s).turn, 0);
                assert_eq!(frame_index((*s).next.image.frame.raw()), 0);
                assert_ne!(
                    ship.descriptor_mut().ship_info.ship_flags & POINT_DEFENSE as u16,
                    0
                );
                let expected =
                    if i64::from(wx) * i64::from(wx) + i64::from(wy) * i64::from(wy) <= 6400 {
                        (
                            (wx << VELOCITY_SHIFT) as SIZE,
                            (wy << VELOCITY_SHIFT) as SIZE,
                        )
                    } else {
                        let a = i32::from(ARCTAN(wx as SIZE, wy as SIZE));
                        (cosine(a, 2560), sine(a, 2560))
                    };
                assert_eq!(velocity(s), expected, "delta {dx}/{dy}");
            }
            ship.ship_mut().hShip = None;
            let before = velocity_fields(&(*s).velocity);
            hooks::invoke_element_pointer(preprocess, s);
            assert_eq!(velocity_fields(&(*s).velocity), before);
        }
    })
}

#[test]
fn satellite_collision_is_noop_and_death_installs_four_tick_animation() {
    battle_context::run(|| {
        let _battle = Battlefield::new(16);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            let satellites = satellites(&mut ship);
            let s = battle::element(satellites[0]);
            let mut other = PoolElement::new();
            let mut p = POINT { x: 1, y: 2 };
            hooks::invoke_collision_pointer(
                (*s).collision_func.unwrap(),
                s,
                &mut p,
                &mut *other,
                &mut POINT { x: 0, y: 0 },
                hooks::CollisionCall::Hook,
            );
            assert_eq!((*s).hits, 10);
            assert_eq!(point(p), (1, 2));
            (*s).state_flags |= DISAPPEARING as u16;
            hooks::invoke_element_pointer((*s).death_func.unwrap(), s);
            assert_eq!(
                ship.descriptor_mut().ship_info.ship_flags & POINT_DEFENSE as u16,
                0
            );
            assert_eq!((*s).state_flags & DISAPPEARING as u16, 0);
            assert_eq!(
                (*s).state_flags & (NONSOLID | FINITE_LIFE | CHANGING) as u16,
                (NONSOLID | FINITE_LIFE | CHANGING) as u16
            );
            assert_eq!((*s).life, 4);
            assert_eq!((*s).turn, 1);
            assert_eq!((*s).thrust, 0);
            assert_eq!(frame_index((*s).next.image.frame.raw()), 8);
            assert!((*s).postprocess_func.is_none());
            assert!((*s).collision_func.is_none());
            assert!((*s).death_func.is_none());
            (*s).current = (*s).next.clone();
            let animate = (*s).preprocess_func.unwrap();
            hooks::invoke_element_pointer(animate, s);
            assert_eq!(frame_index((*s).next.image.frame.raw()), 8);
            hooks::invoke_element_pointer(animate, s);
            assert_eq!(frame_index((*s).next.image.frame.raw()), 9);
            // A surviving satellite restores point defense on its next preprocess.
            let survivor = battle::element(satellites[1]);
            hooks::invoke_element_pointer((*survivor).preprocess_func.unwrap(), survivor);
            assert_ne!(
                ship.descriptor_mut().ship_info.ship_flags & POINT_DEFENSE as u16,
                0
            );
            // Reinitializing the template does not mutate this ship's descriptor.
            ship.descriptor_mut().ship_info.ship_flags &= !(POINT_DEFENSE as u16);
            assert_ne!(
                battle::descriptor(SPECIES_ID_CHMMR_ID).ship_info.ship_flags & POINT_DEFENSE as u16,
                0
            );
        }
    })
}

unsafe fn target(satellite: *mut ElementRecord, dx: i16, dy: i16, hp: u16) -> PoolElement {
    let mut fixture = PoolElement::new();
    let e = &raw mut *fixture;
    (*e).playerNr = 1;
    (*e).hits = hp;
    (*e).mass = 1;
    (*e).next.location = POINT {
        x: (*satellite).next.location.x.wrapping_add(dx),
        y: (*satellite).next.location.y.wrapping_add(dy),
    };
    (*prim(e)).Type = gfx_object_STAMP_PRIM as u8;
    battle::put(e.cast());
    fixture
}

unsafe fn defense(satellite: *mut ElementRecord) -> HLINK {
    hooks::invoke_element_pointer((*satellite).postprocess_func.unwrap(), satellite);
    let h = battle::tail();
    assert_eq!(
        (*battle::element(h)).target,
        Some(battle::id(satellite.cast()))
    );
    assert_eq!((*battle::element(h)).life, 0);
    h
}

#[test]
fn point_defense_uses_backward_weakest_nearest_and_stable_tie_order() {
    battle_context::run(|| {
        let _battle = Battlefield::new(32);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            let satellites = satellites(&mut ship);
            let s = battle::element(satellites[0]);
            (*s).next = (*s).current.clone();
            let mut strong_fixture = target(s, 4, 0, 3);
            let strong: HLINK = (&raw mut *strong_fixture).cast();
            let mut far_weak_fixture = target(s, 200, 0, 1);
            let far_weak: HLINK = (&raw mut *far_weak_fixture).cast();
            let mut near_weak_fixture = target(s, 40, 0, 1);
            let near_weak: HLINK = (&raw mut *near_weak_fixture).cast();
            let mut tie_fixture = target(s, -40, 0, 1);
            let tie: HLINK = (&raw mut *tie_fixture).cast();
            let d = defense(s);
            // An even weaker candidate appended after the defense is not traversed.
            let mut after_fixture = target(s, 0, 4, 0);
            let after: HLINK = (&raw mut *after_fixture).cast();
            let before = battle::tail();
            TFB_SeedRandom(12);
            hooks::invoke_element_pointer(
                (*battle::element(d)).death_func.unwrap(),
                battle::element(d),
            );
            assert_ne!({ battle::tail() }, before);
            let beam = battle::element(battle::tail());
            assert_eq!(point((*beam).current.location), point((*s).next.location));
            assert_eq!(velocity(beam), (-40 << VELOCITY_SHIFT, 0));
            assert_eq!(rgba((*prim(beam)).color), [0, 8, 255, 255]);
            assert_eq!((*beam).parent, (*s).parent);
            assert!((*beam).target.is_none());
            assert_eq!((*s).thrust, 2);
            assert_rng_after(12, 0);
            for h in [strong, far_weak, near_weak, tie, after] {
                assert!(!h.is_null());
            }
        }
    })
}

#[test]
fn point_defense_respects_collision_cloak_range_and_full_width_hitpoints() {
    battle_context::run(|| {
        // (world dx, dy, hit points, excluded condition, should shoot)
        for (dx, dy, hp, exclusion, shoot) in [
            (256, 0, 1, 0, true),
            (259, 0, 1, 0, true),
            (260, 0, 1, 0, false),
            (180, 180, 1, 0, true),
            (184, 184, 1, 0, false),
            (40, 0, 256, 0, false),
            (32, 0, 255, 0, true),
            (36, 0, 255, 0, false),
            (40, 0, 1, 1, false),
            (40, 0, 1, 2, false),
            (40, 0, 1, 3, false),
            (40, 0, 1, 4, false),
            (40, 0, 1, 5, false),
            (40, 0, 1, 6, false),
            (40, 0, 1, 7, false),
            (40, 0, 1, 8, false),
            (40, 0, 1, 9, false),
            (40, 0, 1, 10, true),
        ] {
            let _battle = Battlefield::new(16);
            unsafe {
                let mut ship =
                    Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
                let satellites = satellites(&mut ship);
                let s = battle::element(satellites[0]);
                (*s).next = (*s).current.clone();
                let mut h_fixture = target(s, dx, dy, hp);
                let h: HLINK = (&raw mut *h_fixture).cast();
                let obj = battle::element(h);
                match exclusion {
                    1 => (*obj).playerNr = 0,
                    2 => (*obj).playerNr = -1,
                    3 => (*obj).state_flags = NONSOLID as u16,
                    4 => (*obj).state_flags = DISAPPEARING as u16,
                    5 => (*prim(obj)).Type = gfx_object_NUM_PRIMS as u8,
                    6 => {
                        (*prim(obj)).Type = gfx_object_STAMPFILL_PRIM as u8;
                        (*prim(obj)).color = Color {
                            r: 0,
                            g: 0,
                            b: 0,
                            a: 255,
                        };
                    }
                    7 => {
                        (*obj).state_flags = COLLISION as u16;
                        (*ship.ptr()).state_flags |= COLLISION as u16;
                    }
                    8 => {
                        (*obj).state_flags = IGNORE_SIMILAR as u16;
                        (*ship.ptr()).state_flags |= IGNORE_SIMILAR as u16;
                        (*obj).parent = (*ship.ptr()).parent;
                    }
                    9 => {
                        (*obj).mass = 0;
                        (*ship.ptr()).mass = 0;
                    }
                    10 => (*ship.ptr()).state_flags |= NONSOLID as u16,
                    _ => {}
                }
                let d = defense(s);
                hooks::invoke_element_pointer(
                    (*battle::element(d)).death_func.unwrap(),
                    battle::element(d),
                );
                assert_eq!(
                    battle::tail() != d,
                    shoot,
                    "dx {dx} dy {dy} hp {hp} exclusion {exclusion}"
                );
                assert_eq!((*s).thrust, if shoot { 2 } else { 0 });
            }
        }
    })
}

#[test]
fn satellite_defense_wait_underflows_on_dead_zero_wait_and_handles_exhaustion() {
    battle_context::run(|| {
        let _battle = Battlefield::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            let satellites = satellites(&mut ship);
            let s = battle::element(satellites[0]);
            let post = (*s).postprocess_func.unwrap();
            (*s).thrust = 2;
            let tail = battle::tail();
            hooks::invoke_element_pointer(post, s);
            assert_eq!((*s).thrust, 1);
            hooks::invoke_element_pointer(post, s);
            assert_eq!((*s).thrust, 0);
            assert_eq!({ battle::tail() }, tail);
            (*s).life = 0;
            hooks::invoke_element_pointer(post, s);
            assert_eq!((*s).thrust, 255);
            (*s).life = 2;
            (*s).thrust = 0;
            let held = exhaust();
            hooks::invoke_element_pointer(post, s);
            assert_eq!({ battle::tail() }, tail);
            assert_eq!((*s).thrust, 0);
            for h in held {
                battle_context::dispose(h);
            }
            (*s).next = (*s).current.clone();
            let _target = target(s, 40, 0, 1);
            let d = defense(s);
            let held = exhaust();
            hooks::invoke_element_pointer(
                (*battle::element(d)).death_func.unwrap(),
                battle::element(d),
            );
            assert_eq!({ battle::tail() }, d);
            assert_eq!((*s).thrust, 0);
            for h in held {
                battle_context::dispose(h);
            }
        }
    })
}

#[test]
fn tractor_applies_mass_division_speed_flags_and_head_inserted_blue_shadows() {
    battle_context::run(|| {
        for (mass, initial_vx, expected_flags) in [
            (10u8, 0i16, 0u32),
            (10, 1082, SHIP_AT_MAX_SPEED),
            (10, 1083, SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED),
            (99, 0, 0),
        ] {
            let _battle = Battlefield::new(32);
            unsafe {
                let ship =
                    Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
                let mut enemy =
                    Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 1, [16, 41, 12], [16, 12]);
                (*enemy.ptr()).mass = mass;
                (*enemy.ptr()).current.location = POINT { x: 3000, y: 3728 };
                (*enemy.ptr()).next = (*enemy.ptr()).current.clone();
                (*ship.ptr()).target = Some(battle::id((enemy.handle).cast()));
                ((*enemy.ptr()).velocity).set_components(i32::from(initial_vx), i32::from(0));
                enemy.ship_mut().cur_status_flags =
                    (LEFT | SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16;
                let mut ship = ship;
                ship.ship_mut().cur_status_flags = SPECIAL as u16;
                ship.ship_mut().special_counter = 3;
                observation::enable();
                observation::drain();
                TFB_SeedRandom(99);
                ship.postprocess();
                assert_eq!(special_hooks::count(&observation::drain(), "chmmr"), 1);
                let vx = initial_vx + (384 / i32::from(mass)) as i16;
                assert_eq!(velocity(enemy.ptr()), (vx, 0));
                assert_eq!(
                    enemy.ship_mut().cur_status_flags,
                    (LEFT | expected_flags) as u16
                );
                assert_eq!(ship.descriptor_mut().ship_info.energy_level, 41);
                assert_eq!(ship.ship_mut().energy_counter, 1);
                assert_eq!(ship.ship_mut().special_counter, 0);
                let mut h = battle::head();
                for (offset, blue) in [(60, 57), (42, 74), (28, 99), (17, 115), (8, 132)] {
                    let shadow = battle::element(h);
                    assert_ne!(h, ship.handle);
                    assert_eq!((*shadow).playerNr, 1);
                    assert_eq!(
                        (*shadow).parent,
                        uqm_rust::engine::battle::Parent::Ship(enemy.ship_id)
                    );
                    assert_eq!(
                        (*shadow).state_flags,
                        (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR | POST_PROCESS | CHANGING) as u16
                    );
                    assert_eq!((*shadow).life, 1);
                    assert_eq!(point((*shadow).current.location), (3000 + offset * 4, 3728));
                    assert_eq!(
                        point((*shadow).next.location),
                        point((*shadow).current.location)
                    );
                    assert_eq!(
                        (*shadow).current.image.frame.raw(),
                        (*enemy.ptr()).next.image.frame.raw()
                    );
                    assert_eq!(velocity(shadow), (vx, 0));
                    assert_eq!((*prim(shadow)).Type, gfx_object_STAMPFILL_PRIM as u8);
                    assert_eq!(rgba((*prim(shadow)).color), [0, 0, blue, 255]);
                    h = battle::succ(shadow);
                }
                assert_eq!(h, ship.handle);
                assert_eq!({ battle::tail() }, enemy.handle);
                assert_rng_after(99, 0);
            }
        }
    })
}

#[test]
fn tractor_energy_target_gravity_and_allocation_boundaries() {
    battle_context::run(|| {
        for (energy, mass, has_target, exhaust_queue) in [
            (0u8, 10u8, true, false),
            (1, 100, true, false),
            (1, 255, true, false),
            (1, 10, false, false),
            (1, 10, true, true),
        ] {
            let _battle = Battlefield::new(16);
            unsafe {
                let mut ship =
                    Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
                let enemy =
                    Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 1, [16, 41, 12], [16, 12]);
                (*enemy.ptr()).mass = mass;
                (*enemy.ptr()).current.location = POINT { x: 3000, y: 3728 };
                (*enemy.ptr()).next = (*enemy.ptr()).current.clone();
                if has_target {
                    (*ship.ptr()).target = Some(battle::id((enemy.handle).cast()));
                } else {
                    // TrackShip must see no target, including through its queue scan.
                    (*enemy.ptr()).state_flags = 0;
                }
                ship.descriptor_mut().ship_info.energy_level = energy;
                ship.ship_mut().cur_status_flags = SPECIAL as u16;
                ship.ship_mut().special_counter = 3;
                let held = if exhaust_queue { exhaust() } else { Vec::new() };
                let tail = battle::tail();
                observation::enable();
                observation::drain();
                TFB_SeedRandom(22);
                ship.postprocess();
                // Energy is paid before TrackShip, so a targetless tractor still fires.
                assert_eq!(
                    special_hooks::count(&observation::drain(), "chmmr"),
                    usize::from(energy != 0)
                );
                assert_eq!(ship.ship_mut().special_counter, 0);
                assert_eq!(ship.descriptor_mut().ship_info.energy_level, 0);
                assert_eq!({ battle::tail() }, tail);
                assert_eq!({ battle::head() }, ship.handle);
                let affected = energy != 0 && mass < 100 && has_target;
                assert_eq!(
                    velocity(enemy.ptr()),
                    if affected { (38, 0) } else { (0, 0) }
                );
                assert_rng_after(22, 0);
                for h in held {
                    battle_context::dispose(h);
                }
            }
        }
    })
}

#[test]
fn ai_exact_travel_angle_and_turn_gates_follow_shared_intelligence() {
    battle_context::run(|| {
        let _battle = Battlefield::new(16);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            let enemy = Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 1, [16, 41, 12], [16, 12]);
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            concerns[0].ObjectPtr = enemy.ptr();
            concerns[0].which_turn = 13;

            for own_angle in 0..64u16 {
                for enemy_angle in 0..64u16 {
                    (*ship.ptr()).velocity.travel_angle = own_angle;
                    (*enemy.ptr()).velocity.travel_angle = enemy_angle;
                    ship.ship_mut().ship_input_state = (LEFT | SPECIAL | WEAPON) as u8;
                    // Shared intelligence clears WEAPON when weapon_counter is zero.
                    ship.intelligence(&mut *ship.ptr(), &mut concerns, 0);
                    let angle = (i32::from(own_angle) - (i32::from(enemy_angle) + 32) + 16) & 63;
                    assert_eq!(
                        ship.ship_mut().ship_input_state,
                        LEFT as u8 | if angle > 32 { SPECIAL as u8 } else { 0 },
                        "angles {own_angle}/{enemy_angle}"
                    );
                }
            }
            (*ship.ptr()).velocity.travel_angle = 0;
            (*enemy.ptr()).velocity.travel_angle = 0;
            for (turn, counter, weapon_counter, has_enemy, expected) in [
                (12, 0, 0, true, false),
                (13, 0, 0, true, true),
                (13, 1, 0, true, false),
                (13, 0, 1, true, false),
                (13, 0, 0, false, false),
            ] {
                concerns[0].which_turn = turn;
                concerns[0].ObjectPtr = if has_enemy {
                    enemy.ptr()
                } else {
                    ptr::null_mut()
                };
                ship.ship_mut().special_counter = counter;
                ship.ship_mut().weapon_counter = weapon_counter;
                ship.ship_mut().ship_input_state = (WEAPON | SPECIAL) as u8;
                TFB_SeedRandom(101);
                ship.intelligence(&mut *ship.ptr(), &mut concerns, 0);
                assert_eq!(
                    ship.ship_mut().ship_input_state & SPECIAL as u8 != 0,
                    expected
                );
                assert_eq!(
                    ship.ship_mut().ship_input_state & WEAPON as u8 != 0,
                    weapon_counter != 0
                );
                assert_rng_after(101, 0);
            }
        }
    })
}

#[test]
fn tractor_tracking_consumes_only_the_half_turn_rng_before_shadow_allocation() {
    battle_context::run(|| {
        for seed in [1, 17, 12345] {
            let _battle = Battlefield::new(4);
            unsafe {
                let mut ship =
                    Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
                let enemy =
                    Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 1, [16, 41, 12], [16, 12]);
                // South is exactly half a circle from TrackShip's initial facing.
                (*enemy.ptr()).current.location = POINT { x: 4000, y: 5000 };
                (*enemy.ptr()).next = (*enemy.ptr()).current.clone();
                ship.ship_mut().cur_status_flags = SPECIAL as u16;
                assert!((*ship.ptr()).target.is_none());
                TFB_SeedRandom(seed);
                ship.postprocess();
                assert_eq!(
                    (*ship.ptr()).target,
                    Some(battle::id((enemy.handle).cast()))
                );
                assert_eq!(velocity(enemy.ptr()), (0, -38));
                assert_eq!(ship.descriptor_mut().ship_info.energy_level, 41);
                // Two free slots produce offsets 8 then 17; later allocations fail.
                let second = battle::element(battle::head());
                let first = battle::element(battle::succ(second));
                assert_eq!(point((*second).current.location), (4000, 5000 - 17 * 4));
                assert_eq!(point((*first).current.location), (4000, 5000 - 8 * 4));
                assert_eq!(battle::succ(first), ship.handle);
                assert_rng_after(seed, 1);
            }
        }
    })
}

#[test]
fn laser_death_preserves_size_subtraction_and_unsigned_count_magnitude() {
    battle_context::run(|| {
        let _battle = Battlefield::new(8);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            let h = laser(&mut ship);
            let laser = battle::element(h);
            (*ship.ptr()).current.location = POINT { x: 32760, y: 32760 };
            (*laser).current.location = POINT { x: -8, y: -8 };
            (*laser).turn = 255;
            let mut saw_large_magnitude = false;
            for seed in 1..128 {
                TFB_SeedRandom(seed);
                let gate = TFB_Random() as u8 & 7 != 0;
                let draw = TFB_Random() as COUNT;
                let distance_squared = 2_147_483_648u32;
                let magnitude = (i32::from(draw)
                    % (i32::from(square_root(distance_squared)) + 1 - 112)
                    + 112) as COUNT;
                let tail = battle::tail();
                TFB_SeedRandom(seed);
                hooks::invoke_element_pointer((*laser).death_func.unwrap(), laser);
                assert_eq!(ship.ship_mut().special_counter, 255);
                if gate {
                    saw_large_magnitude |= magnitude > i16::MAX as u16;
                    let spot_handle = battle::tail();
                    let spot = battle::element(spot_handle);
                    let a = i32::from(ARCTAN(i16::MIN, i16::MIN));
                    assert_eq!(
                        point((*spot).current.location),
                        (
                            32760i16.wrapping_add(cosine(a, i32::from(magnitude))),
                            32760i16.wrapping_add(sine(a, i32::from(magnitude))),
                        )
                    );
                    assert_rng_after(seed, 2);
                    battle_context::unlink(spot_handle);
                    battle_context::dispose(spot_handle);
                } else {
                    assert_eq!({ battle::tail() }, tail);
                    assert_rng_after(seed, 1);
                }
            }
            assert!(saw_large_magnitude);
            battle_context::dispose(h);
        }
    })
}

#[test]
fn unrequested_special_and_failed_muzzle_allocation_only_reset_cycle() {
    battle_context::run(|| {
        let _battle = Battlefield::new(3);
        unsafe {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            ship.ship_mut().special_counter = 3;
            TFB_SeedRandom(81);
            ship.postprocess();
            assert_eq!(ship.ship_mut().special_counter, 0);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 42);
            assert_eq!({ battle::head() }, ship.handle);
            assert_rng_after(81, 0);
            let h = laser(&mut ship);
            battle::put(h);
            let held = exhaust();
            ship.ship_mut().cur_status_flags = WEAPON as u16;
            ship.ship_mut().special_counter = 2;
            ship.postprocess();
            assert_eq!(ship.ship_mut().special_counter, 0);
            assert_eq!({ battle::tail() }, h);
            for h in held {
                battle_context::dispose(h);
            }
        }
    })
}

unsafe fn laser(ship: &mut Ship<'_>) -> HLINK {
    let h = ship.initialize_from(ship.ptr(), 1)[0];
    if !h.is_null() {
        (*battle::element(h)).parent = (*ship.ptr()).parent;
    }
    h
}
unsafe fn delayed_spawn(ship: &mut Ship<'_>) -> HLINK {
    ship.preprocess();
    battle::tail()
}
unsafe fn satellites(ship: &mut Ship<'_>) -> Vec<HLINK> {
    let delay = delayed_spawn(ship);
    let death = (*battle::element(delay)).death_func.unwrap();
    battle_context::unlink(delay);
    battle_context::dispose(delay);
    let tail = battle::tail();
    hooks::invoke_element_pointer(death, ship.ptr());
    let mut h = battle::succ(tail);
    let mut out = Vec::new();
    while !h.is_null() {
        out.push(h);
        h = battle::succ(h);
    }
    out
}

#[path = "support/hooks.rs"]
mod hooks;
