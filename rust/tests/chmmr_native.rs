//! Linked callback checks against the Chmmr C rules, without a running renderer.
#![forbid(unsafe_code)]

#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
#[path = "support/special_hooks.rs"]
mod special_hooks;
use battle::Battle as Battlefield;
use battle::{PoolElement, Ship};
use uqm_content::ships::native::{cosine, facing_to_angle, sine};
use uqm_engine::battle::ai_constants::FIRES_FORE;
use uqm_engine::battle::ai_constants::IGNORE_SIMILAR;
use uqm_engine::battle::ai_constants::IMMEDIATE_WEAPON;
use uqm_engine::battle::ai_constants::LEFT;
use uqm_engine::battle::ai_constants::POINT_DEFENSE;
use uqm_engine::battle::ai_constants::SEEKING_SPECIAL;
use uqm_engine::battle::ai_constants::SPECIAL;
use uqm_engine::battle::ai_constants::WEAPON;
use uqm_engine::battle::flags::APPEARING;
use uqm_engine::battle::flags::CHANGING;
use uqm_engine::battle::flags::COLLISION;
use uqm_engine::battle::flags::DISAPPEARING;
use uqm_engine::battle::flags::FINITE_LIFE;
use uqm_engine::battle::flags::NONSOLID;
use uqm_engine::battle::flags::POST_PROCESS;
use uqm_engine::battle::flags::SHIP_AT_MAX_SPEED;
use uqm_engine::battle::flags::SHIP_BEYOND_MAX_SPEED;
use uqm_engine::battle::flags::STAMPFILL_PRIMITIVE;
use uqm_engine::battle::Concern;
use uqm_engine::battle::ElementId;
use uqm_engine::battle::EMPTY_PRIMITIVE;
use uqm_engine::battle::LINE_PRIMITIVE;
use uqm_engine::battle::STAMP_PRIMITIVE;
use uqm_engine::collections::observation;
use uqm_engine::graphics::geometry::Color;
use uqm_engine::graphics::geometry::Point;
use uqm_engine::math::arctan;
use uqm_engine::math::random;
use uqm_engine::math::seed_random;
use uqm_engine::math::square_root;
use uqm_engine::math::velocity::VelocityDesc;
use uqm_engine::math::velocity::VELOCITY_SHIFT;
use uqm_engine::species::species_id::SPECIES_ID_CHMMR_ID;

// Layouts from libs/graphics/drawable.h. These are real frame arrays and parent
// descriptors, so SetAbsFrameIndex and GetFrameRect exercise the engine ABI.

fn rgba(c: Color) -> [u8; 4] {
    [c.r, c.g, c.b, c.a]
}
fn point(p: Point) -> (i16, i16) {
    (p.x, p.y)
}

fn velocity(el: ElementId) -> (i16, i16) {
    let (velocity_x, velocity_y) = (battle::get(el).velocity).get_current_components();
    let (x, y) = (velocity_x as i16, velocity_y as i16);
    (x, y)
}

fn velocity_fields(v: &VelocityDesc) -> [i16; 9] {
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

fn assert_rng_after(seed: u32, draws: usize) {
    let actual = random();
    seed_random(seed);
    for _ in 0..draws {
        random();
    }
    assert_eq!(actual, random(), "RNG draws for seed {seed}");
}

fn exhaust() -> Vec<ElementId> {
    let mut handles = Vec::new();
    while let Some(h) = crate::battle_context::with(|cx| cx.alloc_element()) {
        handles.push(h);
    }
    handles
}

#[test]
fn descriptor_matches_the_entire_c_template_and_init_reinstalls_callbacks() {
    battle_context::run(|_| {
        let _battle = Battlefield::new(8);
        {
            let d = battle::descriptor(SPECIES_ID_CHMMR_ID);
            assert_eq!(
                d.ship_info.ship_flags,
                (FIRES_FORE | IMMEDIATE_WEAPON | SEEKING_SPECIAL | POINT_DEFENSE) as u16
            );
            assert_eq!(d.ship_info.ship_cost, 30);
            assert_eq!((d.ship_info.crew_level, d.ship_info.max_crew), (42, 42));
            assert_eq!((d.ship_info.energy_level, d.ship_info.max_energy), (42, 42));
            assert_eq!(d.ship_info.race_strings_rsc.unwrap(), c"ship.chmmr.text");
            assert_eq!(d.ship_info.icons_rsc.unwrap(), c"ship.chmmr.icons");
            assert_eq!(
                d.ship_info.melee_icon_rsc.unwrap(),
                c"ship.chmmr.meleeicons"
            );
            assert!(d.ship_info.race_strings.is_none());
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
                    d.ship_data.ship_rsc[i].unwrap().to_str().unwrap(),
                    format!("ship.chmmr.graphics.avatar.{size}")
                );
                assert_eq!(
                    d.ship_data.weapon_rsc[i].unwrap().to_str().unwrap(),
                    format!("ship.chmmr.graphics.muzzle.{size}")
                );
                assert_eq!(
                    d.ship_data.special_rsc[i].unwrap().to_str().unwrap(),
                    format!("ship.chmmr.graphics.satellite.{size}")
                );
                assert!(d.ship_data.ship.view(i).is_absent());
                assert!(d.ship_data.weapon.view(i).is_absent());
                assert!(d.ship_data.special.view(i).is_absent());
            }
            let captain = &d.ship_data.captain_control;
            assert_eq!(captain.captain_rsc.unwrap(), c"ship.chmmr.graphics.captain");
            assert!([
                captain.background,
                captain.turn,
                captain.thrust,
                captain.weapon,
                captain.special.raw()
            ]
            .iter()
            .all(|p| p.is_null()));
            assert_eq!(d.ship_data.victory_ditty_rsc.unwrap(), c"ship.chmmr.ditty");
            assert_eq!(d.ship_data.ship_sounds_rsc.unwrap(), c"ship.chmmr.sounds");
            assert!(d.ship_data.victory_ditty.is_null());
            assert!(d.ship_data.ship_sounds.is_none());
            assert_eq!(
                (
                    d.cyborg_control.maneuverability_index,
                    d.cyborg_control.weapon_range
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
    battle_context::run(|_| {
        let _battle = Battlefield::new(16);
        {
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
                battle::get_mut(ship.id).next.location = Point { x: -101, y: 32767 };
                for counter in 0..4u8 {
                    ship.ship_mut().special_counter = counter;
                    seed_random(55);
                    let h = laser(&mut ship).unwrap();
                    let laser = h;
                    assert_eq!(point(battle::get(laser).current.location), (-116, -32752));
                    assert_eq!(battle::get(laser).hits, 1);
                    assert_eq!(battle::get(laser).mass, 2);
                    assert_eq!(battle::get(laser).life, 1);
                    assert_eq!(battle::get(laser).turn, (counter + 1) % 4);
                    assert!(battle::get(laser).death_func.is_some());
                    assert_eq!(battle::prim(laser).kind, LINE_PRIMITIVE);
                    assert_eq!(rgba(battle::prim(laser).color), colors[counter as usize]);
                    let a = facing_to_angle(i32::from(face));
                    assert_eq!(
                        velocity(laser),
                        (
                            (i32::from(cosine(a, 600)) << VELOCITY_SHIFT) as i16,
                            (i32::from(sine(a, 600)) << VELOCITY_SHIFT) as i16
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
    battle_context::run(|_| {
        let _battle = Battlefield::new(2);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            let held = exhaust();
            seed_random(9);
            assert!(laser(&mut ship).is_none());
            assert_rng_after(9, 0);
            for h in held {
                battle_context::dispose(h);
            }
        }
    })
}

#[test]
fn laser_death_obeys_range_byte_gate_count_draw_and_animation() {
    battle_context::run(|_| {
        let _battle = Battlefield::new(16);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            let h = laser(&mut ship).unwrap();
            let laser = h;
            let death = battle::get(laser).death_func.unwrap();
            let origin = battle::get(ship.id).current.location;
            let mut gates = [false; 2];
            for seed in 1..64 {
                for distance in [111i16, 112, 600] {
                    seed_random(seed);
                    let gate = random() as u8 & 7 != 0;
                    let draw = random() as u16;
                    gates[gate as usize] = true;
                    battle::get_mut(laser).current.location = Point {
                        x: origin.x + distance,
                        y: origin.y,
                    };
                    ship.ship_mut().special_counter = 0;
                    let tail = battle::tail();
                    seed_random(seed);
                    hooks::invoke_element(death, laser);
                    assert_eq!(ship.ship_mut().special_counter, 1);
                    if gate && distance >= 112 {
                        let spot_handle = battle::tail().unwrap();
                        assert_ne!(Some(spot_handle), tail);
                        let spot = spot_handle;
                        let magnitude = draw % ((distance as u16 + 1) - 112) + 112;
                        assert_eq!(
                            point(battle::get(spot).current.location),
                            (origin.x + magnitude as i16, origin.y)
                        );
                        assert_eq!((battle::get(spot).current.image.frame.raw()).index(), 32);
                        assert_eq!(battle::get(spot).current.image.frames, ship.frame_set(1));
                        assert_eq!(battle::get(spot).life, 9);
                        assert_eq!(battle::get(spot).thrust, 0);
                        assert_eq!(battle::prim(spot).kind, STAMP_PRIMITIVE);
                        assert_eq!(
                            battle::get(spot).parent,
                            uqm_engine::battle::Parent::Ship(ship.ship_id)
                        );
                        let animate = battle::get(spot).preprocess_func.unwrap();
                        battle::get_mut(spot).turn = 1;
                        battle::get_mut(spot).thrust = 3;
                        hooks::invoke_element(animate, spot);
                        assert_eq!(battle::get(spot).turn, 0);
                        assert!(battle::get(spot).next.image.frame.is_absent());
                        hooks::invoke_element(animate, spot);
                        assert_eq!((battle::get(spot).next.image.frame.raw()).index(), 33);
                        assert_eq!(battle::get(spot).turn, 3);
                        assert_ne!(battle::get(spot).state_flags & CHANGING as u16, 0);
                        assert_rng_after(seed, 2);
                        battle_context::unlink(spot_handle);
                        battle_context::dispose(spot_handle);
                    } else {
                        assert_eq!(battle::tail(), tail);
                        assert_rng_after(seed, 1);
                    }
                }
            }
            assert_eq!(gates, [true, true]);
            ship.ship_mut().ship_element = None;
            seed_random(123);
            hooks::invoke_element(death, laser);
            assert_rng_after(123, 0);
            ship.ship_mut().ship_element = Some(ship.id);
            battle::get_mut(laser).current.location.x = origin.x + 600;
            let held = exhaust();
            seed_random(7);
            hooks::invoke_element(death, laser);
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
    battle_context::run(|_| {
        let _battle = Battlefield::new(16);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            ship.ship_mut().ship_facing = 5;
            ship.ship_mut().cur_status_flags = WEAPON as u16;
            ship.postprocess(); // tail is the ship itself
            assert_eq!({ battle::tail() }, Some(ship.id));
            for (counter, player, appearing, kind, expected) in [
                (0, 0, true, LINE_PRIMITIVE, true),
                (2, 0, true, LINE_PRIMITIVE, true),
                (1, 0, true, LINE_PRIMITIVE, false),
                (3, 0, true, LINE_PRIMITIVE, false),
                (0, 1, true, LINE_PRIMITIVE, false),
                (0, 0, false, LINE_PRIMITIVE, false),
                (0, 0, true, STAMP_PRIMITIVE, false),
            ] {
                ship.ship_mut().special_counter = counter;
                let h = laser(&mut ship).unwrap();
                let laser = h;
                battle::get_mut(laser).player_nr = player;
                if !appearing {
                    battle::get_mut(laser).state_flags &= !(APPEARING as u16);
                }
                battle::prim_mut(laser).kind = kind;
                battle::put(h);
                ship.postprocess();
                assert_eq!(ship.ship_mut().special_counter, 0);
                if expected {
                    let flash_handle = battle::tail().unwrap();
                    assert_ne!(flash_handle, h);
                    let flash = flash_handle;
                    assert_eq!(
                        point(battle::get(flash).current.location),
                        point(battle::get(ship.id).next.location)
                    );
                    assert_eq!((battle::get(flash).current.image.frame.raw()).index(), 21);
                    assert_eq!(battle::get(flash).life, 1);
                    assert_eq!(battle::prim(flash).kind, STAMP_PRIMITIVE);
                    assert_eq!(
                        battle::get(flash).parent,
                        uqm_engine::battle::Parent::Ship(ship.ship_id)
                    );
                    battle_context::unlink(flash_handle);
                    battle_context::dispose(flash_handle);
                } else {
                    assert_eq!({ battle::tail() }, Some(h));
                }
                battle_context::unlink(h);
                battle_context::dispose(h);
            }
        }
    })
}

#[test]
fn delayed_satellites_spawn_around_live_ship_with_three_ordered_rng_draws() {
    battle_context::run(|_| {
        let _battle = Battlefield::new(16);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 1, [16, 41, 12], [16, 12]);
            let delay = delayed_spawn(&mut ship);
            let transit = delay;
            assert!(matches!(
                ship.descriptor_mut().race_callbacks.pre,
                uqm_engine::battle::Selection::Disabled
            ));
            assert_eq!(battle::get(transit).life, 16);
            assert_eq!(battle::prim(transit).kind, EMPTY_PRIMITIVE);
            assert_eq!(
                battle::get(transit).state_flags,
                (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR | APPEARING) as u16
            );
            // The delayed element's position and player are not the spawn source.
            battle::get_mut(transit).player_nr = 0;
            battle::get_mut(transit).next.location = Point { x: -1000, y: -2000 };
            battle::get_mut(ship.id).next.location = Point { x: 7000, y: 6000 };
            seed_random(77);
            let indices = [
                random() as u16 & 7,
                random() as u16 & 7,
                random() as u16 & 7,
            ];
            seed_random(77);
            hooks::invoke_element(battle::get(transit).death_func.unwrap(), transit);
            let mut h = battle::succ(transit);
            for (i, index) in indices.into_iter().enumerate() {
                assert!(h.is_some());
                let s = h.unwrap();
                let angle = (i as i32 * 64 + 1) / 3;
                assert_eq!(battle::get(s).player_nr, 1);
                assert_eq!(battle::get(s).turn, angle as u8);
                assert_eq!(battle::get(s).life, 2);
                assert_eq!(battle::get(s).hits, 10);
                assert_eq!(battle::get(s).mass, 10);
                assert_eq!(
                    point(battle::get(s).current.location),
                    (7000 + cosine(angle, 256), 6000 + sine(angle, 256))
                );
                assert_eq!((battle::get(s).current.image.frame.raw()).index(), index);
                assert_eq!(battle::prim(s).kind, STAMP_PRIMITIVE);
                assert!(battle::get(s).preprocess_func.is_some());
                assert!(battle::get(s).postprocess_func.is_some());
                assert!(battle::get(s).death_func.is_some());
                assert!(battle::get(s).collision_func.is_some());
                h = battle::succ(s);
            }
            assert!(h.is_none());
            assert_rng_after(77, 3);
        }
    })
}

#[test]
fn spawn_and_preprocess_allocation_failures_do_not_draw_rng_or_rearm() {
    battle_context::run(|_| {
        let _battle = Battlefield::new(3);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            let delay = delayed_spawn(&mut ship);
            let death = battle::get(delay).death_func.unwrap();
            seed_random(88);
            hooks::invoke_element(death, delay); // only one free slot
            assert_eq!(battle::get(battle::tail().unwrap()).hits, 10);
            assert_rng_after(88, 1);
            seed_random(88);
            hooks::invoke_element(death, delay);
            assert_rng_after(88, 0);
            ship.ship_mut().ship_element = None;
            seed_random(88);
            hooks::invoke_element(death, delay);
            assert_rng_after(88, 0);
            ship.ship_mut().ship_element = Some(ship.id);
            ship.descriptor_mut().race_callbacks.pre = uqm_engine::battle::Selection::Default;
            let tail = battle::tail();
            ship.preprocess();
            assert!(matches!(
                ship.descriptor_mut().race_callbacks.pre,
                uqm_engine::battle::Selection::Disabled
            ));
            assert_eq!(battle::tail(), tail);
        }
    })
}

#[test]
fn satellites_wrap_space_cap_speed_and_preserve_half_space_ties() {
    battle_context::run(|_| {
        let _battle = Battlefield::new(16);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            let satellites = satellites(&mut ship);
            let s = satellites[0];
            let preprocess = battle::get(s).preprocess_func.unwrap();
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
                battle::get_mut(s).turn = 63;
                battle::get_mut(s).life = u16::MAX;
                battle::get_mut(s).current.image.frame = ship.frame(2, 7);
                let desired_x =
                    i32::from(battle::get(ship.id).next.location.x) + i32::from(cosine(0, 256));
                let desired_y =
                    i32::from(battle::get(ship.id).next.location.y) + i32::from(sine(0, 256));
                battle::get_mut(s).current.location = Point {
                    x: (desired_x - dx) as i16,
                    y: (desired_y - dy) as i16,
                };
                ship.descriptor_mut().ship_info.ship_flags &= !(POINT_DEFENSE as u16);
                hooks::invoke_element(preprocess, s);
                assert_eq!(battle::get(s).life, 0);
                assert_eq!(battle::get(s).turn, 0);
                assert_eq!((battle::get(s).next.image.frame.raw()).index(), 0);
                assert_ne!(
                    ship.descriptor_mut().ship_info.ship_flags & POINT_DEFENSE as u16,
                    0
                );
                let expected =
                    if i64::from(wx) * i64::from(wx) + i64::from(wy) * i64::from(wy) <= 6400 {
                        ((wx << VELOCITY_SHIFT) as i16, (wy << VELOCITY_SHIFT) as i16)
                    } else {
                        let a = i32::from(arctan(wx as i16, wy as i16));
                        (cosine(a, 2560), sine(a, 2560))
                    };
                assert_eq!(velocity(s), expected, "delta {dx}/{dy}");
            }
            ship.ship_mut().ship_element = None;
            let before = velocity_fields(&battle::get(s).velocity);
            hooks::invoke_element(preprocess, s);
            assert_eq!(velocity_fields(&battle::get(s).velocity), before);
        }
    })
}

#[test]
fn satellite_collision_is_noop_and_death_installs_four_tick_animation() {
    battle_context::run(|_| {
        let _battle = Battlefield::new(16);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            let satellites = satellites(&mut ship);
            let s = satellites[0];
            let other = PoolElement::new();
            let mut p = Point { x: 1, y: 2 };
            hooks::invoke_collision(
                battle::get(s).collision_func.unwrap(),
                s,
                &mut p,
                other.id,
                &mut Point { x: 0, y: 0 },
            );
            assert_eq!(battle::get(s).hits, 10);
            assert_eq!(point(p), (1, 2));
            battle::get_mut(s).state_flags |= DISAPPEARING as u16;
            hooks::invoke_element(battle::get(s).death_func.unwrap(), s);
            assert_eq!(
                ship.descriptor_mut().ship_info.ship_flags & POINT_DEFENSE as u16,
                0
            );
            assert_eq!(battle::get(s).state_flags & DISAPPEARING as u16, 0);
            assert_eq!(
                battle::get(s).state_flags & (NONSOLID | FINITE_LIFE | CHANGING) as u16,
                (NONSOLID | FINITE_LIFE | CHANGING) as u16
            );
            assert_eq!(battle::get(s).life, 4);
            assert_eq!(battle::get(s).turn, 1);
            assert_eq!(battle::get(s).thrust, 0);
            assert_eq!((battle::get(s).next.image.frame.raw()).index(), 8);
            assert!(battle::get(s).postprocess_func.is_none());
            assert!(battle::get(s).collision_func.is_none());
            assert!(battle::get(s).death_func.is_none());
            battle::get_mut(s).current = battle::get(s).next.clone();
            let animate = battle::get(s).preprocess_func.unwrap();
            hooks::invoke_element(animate, s);
            assert_eq!((battle::get(s).next.image.frame.raw()).index(), 8);
            hooks::invoke_element(animate, s);
            assert_eq!((battle::get(s).next.image.frame.raw()).index(), 9);
            // A surviving satellite restores point defense on its next preprocess.
            let survivor = satellites[1];
            hooks::invoke_element(battle::get(survivor).preprocess_func.unwrap(), survivor);
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

fn target(satellite: ElementId, dx: i16, dy: i16, hp: u16) -> PoolElement {
    let fixture = PoolElement::new();
    let e = fixture.id;
    battle::get_mut(e).player_nr = 1;
    battle::get_mut(e).hits = hp;
    battle::get_mut(e).mass = 1;
    battle::get_mut(e).next.location = Point {
        x: battle::get(satellite).next.location.x.wrapping_add(dx),
        y: battle::get(satellite).next.location.y.wrapping_add(dy),
    };
    battle::prim_mut(e).kind = STAMP_PRIMITIVE;
    battle::put(e);
    fixture
}

fn defense(satellite: ElementId) -> ElementId {
    hooks::invoke_element(battle::get(satellite).postprocess_func.unwrap(), satellite);
    let h = battle::tail().unwrap();
    assert_eq!(battle::get(h).target, Some(satellite));
    assert_eq!(battle::get(h).life, 0);
    h
}

#[test]
fn point_defense_uses_backward_weakest_nearest_and_stable_tie_order() {
    battle_context::run(|_| {
        let _battle = Battlefield::new(32);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            let satellites = satellites(&mut ship);
            let s = satellites[0];
            battle::get_mut(s).next = battle::get(s).current.clone();
            let strong_fixture = target(s, 4, 0, 3);
            let strong: ElementId = strong_fixture.id;
            let far_weak_fixture = target(s, 200, 0, 1);
            let far_weak: ElementId = far_weak_fixture.id;
            let near_weak_fixture = target(s, 40, 0, 1);
            let near_weak: ElementId = near_weak_fixture.id;
            let tie_fixture = target(s, -40, 0, 1);
            let tie: ElementId = tie_fixture.id;
            let d = defense(s);
            // An even weaker candidate appended after the defense is not traversed.
            let after_fixture = target(s, 0, 4, 0);
            let after: ElementId = after_fixture.id;
            let before = battle::tail();
            seed_random(12);
            hooks::invoke_element(battle::get(d).death_func.unwrap(), d);
            assert_ne!({ battle::tail() }, before);
            let beam = battle::tail().unwrap();
            assert_eq!(
                point(battle::get(beam).current.location),
                point(battle::get(s).next.location)
            );
            assert_eq!(velocity(beam), (-40 << VELOCITY_SHIFT, 0));
            assert_eq!(rgba(battle::prim(beam).color), [0, 8, 255, 255]);
            assert_eq!(battle::get(beam).parent, battle::get(s).parent);
            assert!(battle::get(beam).target.is_none());
            assert_eq!(battle::get(s).thrust, 2);
            assert_rng_after(12, 0);
            for _h in [strong, far_weak, near_weak, tie, after] {}
        }
    })
}

#[test]
fn point_defense_respects_collision_cloak_range_and_full_width_hitpoints() {
    battle_context::run(|_| {
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
            {
                let mut ship =
                    Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
                let satellites = satellites(&mut ship);
                let s = satellites[0];
                battle::get_mut(s).next = battle::get(s).current.clone();
                let h_fixture = target(s, dx, dy, hp);
                let h: ElementId = h_fixture.id;
                let obj = h;
                match exclusion {
                    1 => battle::get_mut(obj).player_nr = 0,
                    2 => battle::get_mut(obj).player_nr = -1,
                    3 => battle::get_mut(obj).state_flags = NONSOLID as u16,
                    4 => battle::get_mut(obj).state_flags = DISAPPEARING as u16,
                    5 => battle::prim_mut(obj).kind = EMPTY_PRIMITIVE,
                    6 => {
                        battle::prim_mut(obj).kind = STAMPFILL_PRIMITIVE;
                        battle::prim_mut(obj).color = Color {
                            r: 0,
                            g: 0,
                            b: 0,
                            a: 255,
                        };
                    }
                    7 => {
                        battle::get_mut(obj).state_flags = COLLISION as u16;
                        battle::get_mut(ship.id).state_flags |= COLLISION as u16;
                    }
                    8 => {
                        battle::get_mut(obj).state_flags = IGNORE_SIMILAR as u16;
                        battle::get_mut(ship.id).state_flags |= IGNORE_SIMILAR as u16;
                        battle::get_mut(obj).parent = battle::get(ship.id).parent;
                    }
                    9 => {
                        battle::get_mut(obj).mass = 0;
                        battle::get_mut(ship.id).mass = 0;
                    }
                    10 => battle::get_mut(ship.id).state_flags |= NONSOLID as u16,
                    _ => {}
                }
                let d = defense(s);
                hooks::invoke_element(battle::get(d).death_func.unwrap(), d);
                assert_eq!(
                    battle::tail() != Some(d),
                    shoot,
                    "dx {dx} dy {dy} hp {hp} exclusion {exclusion}"
                );
                assert_eq!(battle::get(s).thrust, if shoot { 2 } else { 0 });
            }
        }
    })
}

#[test]
fn satellite_defense_wait_underflows_on_dead_zero_wait_and_handles_exhaustion() {
    battle_context::run(|_| {
        let _battle = Battlefield::new(8);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            let satellites = satellites(&mut ship);
            let s = satellites[0];
            let post = battle::get(s).postprocess_func.unwrap();
            battle::get_mut(s).thrust = 2;
            let tail = battle::tail();
            hooks::invoke_element(post, s);
            assert_eq!(battle::get(s).thrust, 1);
            hooks::invoke_element(post, s);
            assert_eq!(battle::get(s).thrust, 0);
            assert_eq!(battle::tail(), tail);
            battle::get_mut(s).life = 0;
            hooks::invoke_element(post, s);
            assert_eq!(battle::get(s).thrust, 255);
            battle::get_mut(s).life = 2;
            battle::get_mut(s).thrust = 0;
            let held = exhaust();
            hooks::invoke_element(post, s);
            assert_eq!(battle::tail(), tail);
            assert_eq!(battle::get(s).thrust, 0);
            for h in held {
                battle_context::dispose(h);
            }
            battle::get_mut(s).next = battle::get(s).current.clone();
            let _target = target(s, 40, 0, 1);
            let d = defense(s);
            let held = exhaust();
            hooks::invoke_element(battle::get(d).death_func.unwrap(), d);
            assert_eq!({ battle::tail() }, Some(d));
            assert_eq!(battle::get(s).thrust, 0);
            for h in held {
                battle_context::dispose(h);
            }
        }
    })
}

#[test]
fn tractor_applies_mass_division_speed_flags_and_head_inserted_blue_shadows() {
    battle_context::run(|_| {
        for (mass, initial_vx, expected_flags) in [
            (10u8, 0i16, 0u32),
            (10, 1082, SHIP_AT_MAX_SPEED),
            (10, 1083, SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED),
            (99, 0, 0),
        ] {
            let _battle = Battlefield::new(32);
            {
                let ship =
                    Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
                let mut enemy =
                    Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 1, [16, 41, 12], [16, 12]);
                battle::get_mut(enemy.id).mass = mass;
                battle::get_mut(enemy.id).current.location = Point { x: 3000, y: 3728 };
                battle::get_mut(enemy.id).next = battle::get(enemy.id).current.clone();
                battle::get_mut(ship.id).target = Some(enemy.id);
                battle::get_mut(enemy.id)
                    .velocity
                    .set_components(i32::from(initial_vx), 0);
                enemy.ship_mut().cur_status_flags =
                    (LEFT | SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16;
                let mut ship = ship;
                ship.ship_mut().cur_status_flags = SPECIAL as u16;
                ship.ship_mut().special_counter = 3;
                observation::enable();
                observation::drain();
                seed_random(99);
                ship.postprocess();
                assert_eq!(special_hooks::count(&observation::drain(), "chmmr"), 1);
                let vx = initial_vx + (384 / i32::from(mass)) as i16;
                assert_eq!(velocity(enemy.id), (vx, 0));
                assert_eq!(
                    enemy.ship_mut().cur_status_flags,
                    (LEFT | expected_flags) as u16
                );
                assert_eq!(ship.descriptor_mut().ship_info.energy_level, 41);
                assert_eq!(ship.ship_mut().energy_counter, 1);
                assert_eq!(ship.ship_mut().special_counter, 0);
                let mut h = battle::head();
                for (offset, blue) in [(60, 57), (42, 74), (28, 99), (17, 115), (8, 132)] {
                    let shadow = h.unwrap();
                    assert_ne!(shadow, ship.id);
                    assert_eq!(battle::get(shadow).player_nr, 1);
                    assert_eq!(
                        battle::get(shadow).parent,
                        uqm_engine::battle::Parent::Ship(enemy.ship_id)
                    );
                    assert_eq!(
                        battle::get(shadow).state_flags,
                        (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR | POST_PROCESS | CHANGING) as u16
                    );
                    assert_eq!(battle::get(shadow).life, 1);
                    assert_eq!(
                        point(battle::get(shadow).current.location),
                        (3000 + offset * 4, 3728)
                    );
                    assert_eq!(
                        point(battle::get(shadow).next.location),
                        point(battle::get(shadow).current.location)
                    );
                    assert_eq!(
                        battle::get(shadow).current.image.frame.raw(),
                        battle::get(enemy.id).next.image.frame.raw()
                    );
                    assert_eq!(velocity(shadow), (vx, 0));
                    assert_eq!(battle::prim(shadow).kind, STAMPFILL_PRIMITIVE);
                    assert_eq!(rgba(battle::prim(shadow).color), [0, 0, blue, 255]);
                    h = battle::succ(shadow);
                }
                assert_eq!(h, Some(ship.id));
                assert_eq!({ battle::tail() }, Some(enemy.id));
                assert_rng_after(99, 0);
            }
        }
    })
}

#[test]
fn tractor_energy_target_gravity_and_allocation_boundaries() {
    battle_context::run(|_| {
        for (energy, mass, has_target, exhaust_queue) in [
            (0u8, 10u8, true, false),
            (1, 100, true, false),
            (1, 255, true, false),
            (1, 10, false, false),
            (1, 10, true, true),
        ] {
            let _battle = Battlefield::new(16);
            {
                let mut ship =
                    Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
                let enemy =
                    Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 1, [16, 41, 12], [16, 12]);
                battle::get_mut(enemy.id).mass = mass;
                battle::get_mut(enemy.id).current.location = Point { x: 3000, y: 3728 };
                battle::get_mut(enemy.id).next = battle::get(enemy.id).current.clone();
                if has_target {
                    battle::get_mut(ship.id).target = Some(enemy.id);
                } else {
                    // TrackShip must see no target, including through its queue scan.
                    battle::get_mut(enemy.id).state_flags = 0;
                }
                ship.descriptor_mut().ship_info.energy_level = energy;
                ship.ship_mut().cur_status_flags = SPECIAL as u16;
                ship.ship_mut().special_counter = 3;
                let held = if exhaust_queue { exhaust() } else { Vec::new() };
                let tail = battle::tail();
                observation::enable();
                observation::drain();
                seed_random(22);
                ship.postprocess();
                // Energy is paid before TrackShip, so a targetless tractor still fires.
                assert_eq!(
                    special_hooks::count(&observation::drain(), "chmmr"),
                    usize::from(energy != 0)
                );
                assert_eq!(ship.ship_mut().special_counter, 0);
                assert_eq!(ship.descriptor_mut().ship_info.energy_level, 0);
                assert_eq!(battle::tail(), tail);
                assert_eq!({ battle::head() }, Some(ship.id));
                let affected = energy != 0 && mass < 100 && has_target;
                assert_eq!(velocity(enemy.id), if affected { (38, 0) } else { (0, 0) });
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
    battle_context::run(|_| {
        let _battle = Battlefield::new(16);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            let enemy = Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 1, [16, 41, 12], [16, 12]);
            let mut concerns: [Concern; 4] = [Concern::default(); 4];
            concerns[0].object = Some(enemy.id);
            concerns[0].which_turn = 13;

            for own_angle in 0..64u16 {
                for enemy_angle in 0..64u16 {
                    battle::get_mut(ship.id).velocity.travel_angle = own_angle;
                    battle::get_mut(enemy.id).velocity.travel_angle = enemy_angle;
                    ship.ship_mut().ship_input_state = (LEFT | SPECIAL | WEAPON) as u8;
                    // Shared intelligence clears WEAPON when weapon_counter is zero.
                    ship.think(ship.id, &mut concerns, 0);
                    let angle = (i32::from(own_angle) - (i32::from(enemy_angle) + 32) + 16) & 63;
                    assert_eq!(
                        ship.ship_mut().ship_input_state,
                        LEFT as u8 | if angle > 32 { SPECIAL as u8 } else { 0 },
                        "angles {own_angle}/{enemy_angle}"
                    );
                }
            }
            battle::get_mut(ship.id).velocity.travel_angle = 0;
            battle::get_mut(enemy.id).velocity.travel_angle = 0;
            for (turn, counter, weapon_counter, has_enemy, expected) in [
                (12, 0, 0, true, false),
                (13, 0, 0, true, true),
                (13, 1, 0, true, false),
                (13, 0, 1, true, false),
                (13, 0, 0, false, false),
            ] {
                concerns[0].which_turn = turn;
                concerns[0].object = if has_enemy { Some(enemy.id) } else { None };
                ship.ship_mut().special_counter = counter;
                ship.ship_mut().weapon_counter = weapon_counter;
                ship.ship_mut().ship_input_state = (WEAPON | SPECIAL) as u8;
                seed_random(101);
                ship.think(ship.id, &mut concerns, 0);
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
    battle_context::run(|_| {
        for seed in [1, 17, 12345] {
            let _battle = Battlefield::new(4);
            {
                let mut ship =
                    Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
                let enemy =
                    Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 1, [16, 41, 12], [16, 12]);
                // South is exactly half a circle from TrackShip's initial facing.
                battle::get_mut(enemy.id).current.location = Point { x: 4000, y: 5000 };
                battle::get_mut(enemy.id).next = battle::get(enemy.id).current.clone();
                ship.ship_mut().cur_status_flags = SPECIAL as u16;
                assert!(battle::get(ship.id).target.is_none());
                seed_random(seed);
                ship.postprocess();
                assert_eq!(battle::get(ship.id).target, Some(enemy.id));
                assert_eq!(velocity(enemy.id), (0, -38));
                assert_eq!(ship.descriptor_mut().ship_info.energy_level, 41);
                // Two free slots produce offsets 8 then 17; later allocations fail.
                let second = battle::head().unwrap();
                let first = battle::succ(second).unwrap();
                assert_eq!(
                    point(battle::get(second).current.location),
                    (4000, 5000 - 17 * 4)
                );
                assert_eq!(
                    point(battle::get(first).current.location),
                    (4000, 5000 - 8 * 4)
                );
                assert_eq!(battle::succ(first), Some(ship.id));
                assert_rng_after(seed, 1);
            }
        }
    })
}

#[test]
fn laser_death_preserves_size_subtraction_and_unsigned_count_magnitude() {
    battle_context::run(|_| {
        let _battle = Battlefield::new(8);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            let h = laser(&mut ship).unwrap();
            let laser = h;
            battle::get_mut(ship.id).current.location = Point { x: 32760, y: 32760 };
            battle::get_mut(laser).current.location = Point { x: -8, y: -8 };
            battle::get_mut(laser).turn = 255;
            let mut saw_large_magnitude = false;
            for seed in 1..128 {
                seed_random(seed);
                let gate = random() as u8 & 7 != 0;
                let draw = random() as u16;
                let distance_squared = 2_147_483_648u32;
                let magnitude = (i32::from(draw)
                    % (i32::from(square_root(distance_squared)) + 1 - 112)
                    + 112) as u16;
                let tail = battle::tail();
                seed_random(seed);
                hooks::invoke_element(battle::get(laser).death_func.unwrap(), laser);
                assert_eq!(ship.ship_mut().special_counter, 255);
                if gate {
                    saw_large_magnitude |= magnitude > i16::MAX as u16;
                    let spot_handle = battle::tail().unwrap();
                    let spot = spot_handle;
                    let a = i32::from(arctan(i16::MIN, i16::MIN));
                    assert_eq!(
                        point(battle::get(spot).current.location),
                        (
                            32760i16.wrapping_add(cosine(a, i32::from(magnitude))),
                            32760i16.wrapping_add(sine(a, i32::from(magnitude))),
                        )
                    );
                    assert_rng_after(seed, 2);
                    battle_context::unlink(spot_handle);
                    battle_context::dispose(spot_handle);
                } else {
                    assert_eq!(battle::tail(), tail);
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
    battle_context::run(|_| {
        let _battle = Battlefield::new(3);
        {
            let mut ship =
                Ship::with_frames(&_battle, SPECIES_ID_CHMMR_ID, 0, [16, 41, 12], [16, 12]);
            ship.ship_mut().special_counter = 3;
            seed_random(81);
            ship.postprocess();
            assert_eq!(ship.ship_mut().special_counter, 0);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 42);
            assert_eq!({ battle::head() }, Some(ship.id));
            assert_rng_after(81, 0);
            let h = laser(&mut ship).unwrap();
            battle::put(h);
            let held = exhaust();
            ship.ship_mut().cur_status_flags = WEAPON as u16;
            ship.ship_mut().special_counter = 2;
            ship.postprocess();
            assert_eq!(ship.ship_mut().special_counter, 0);
            assert_eq!({ battle::tail() }, Some(h));
            for h in held {
                battle_context::dispose(h);
            }
        }
    })
}

fn laser(ship: &mut Ship<'_>) -> Option<ElementId> {
    let h = ship.initialize_from(ship.id, 1)[0];
    if let Some(h) = h {
        battle::get_mut(h).parent = battle::get(ship.id).parent;
    }
    h
}
fn delayed_spawn(ship: &mut Ship<'_>) -> ElementId {
    ship.preprocess();
    battle::tail().unwrap()
}
fn satellites(ship: &mut Ship<'_>) -> Vec<ElementId> {
    let delay = delayed_spawn(ship);
    let death = battle::get(delay).death_func.unwrap();
    battle_context::unlink(delay);
    battle_context::dispose(delay);
    let tail = battle::tail().unwrap();
    hooks::invoke_element(death, ship.id);
    let mut cursor = battle::succ(tail);
    let mut out = Vec::new();
    while let Some(h) = cursor {
        out.push(h);
        cursor = battle::succ(h);
    }
    out
}

#[path = "support/hooks.rs"]
mod hooks;
