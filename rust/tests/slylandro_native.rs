//! Exercise Slylandro's live callbacks, target selection, and RNG order.
#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
use battle::{prim, Battle as PoolBattle, PoolElement, Ship};

use std::{ffi::CStr, mem, ptr};
use uqm_rust::ships::native::{
    abi::*, cosine, element, facing_to_angle, frame_index, normalize_facing, put_element, sine,
};

struct Battle {
    _pool: PoolBattle,
    activity: u16,
}
impl Battle {
    unsafe fn new(capacity: COUNT) -> Self {
        let activity = GlobData.Game_state.CurrentActivity;
        let pool = PoolBattle::new(capacity);
        GlobData.Game_state.CurrentActivity = 0;
        Self {
            _pool: pool,
            activity,
        }
    }
}
impl Drop for Battle {
    fn drop(&mut self) {
        unsafe {
            GlobData.Game_state.CurrentActivity = self.activity;
        }
    }
}

fn rgba(c: Color) -> (u8, u8, u8, u8) {
    (c.r, c.g, c.b, c.a)
}
fn velocity(v: &VELOCITY_DESC) -> [i32; 9] {
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
unsafe fn vector(speed: SIZE, facing: COUNT) -> [i32; 9] {
    let mut v = mem::zeroed();
    SetVelocityVector(&mut v, speed, facing);
    velocity(&v)
}
unsafe fn with_rng<T>(seed: DWORD, draws: usize, action: impl FnOnce() -> T) -> T {
    TFB_SeedRandom(seed);
    for _ in 0..draws {
        TFB_Random();
    }
    let next = TFB_Random();
    TFB_SeedRandom(seed);
    let result = action();
    assert_eq!(TFB_Random(), next, "seed {seed}: expected {draws} draws");
    result
}
unsafe fn randoms(seed: DWORD) -> (DWORD, DWORD) {
    TFB_SeedRandom(seed);
    (TFB_Random(), TFB_Random())
}
fn bolt_color(random: DWORD) -> (u8, u8, u8, u8) {
    match (random >> 8) & 3 {
        0 => (255, 255, 255, 255),
        1 => (181, 189, 255, 255),
        2 => (49, 57, 255, 255),
        _ => (0, 0, 198, 255),
    }
}
unsafe fn bolt_velocity(angle: COUNT, delta: SIZE, random: DWORD) -> [i32; 9] {
    let angle = match delta {
        -1 | 8 => angle.wrapping_add(random as COUNT),
        0 => {
            if random & 1 != 0 {
                angle.wrapping_sub(1)
            } else {
                angle.wrapping_add(1)
            }
        }
        1..=7 => angle.wrapping_add(random as COUNT & 15),
        _ => angle.wrapping_sub(random as COUNT & 15),
    };
    let length = ((((random >> 16) & 31) + 4) << (ONE_SHIFT + VELOCITY_SHIFT)) as SIZE;
    let mut v = mem::zeroed();
    SetVelocityComponents(
        &mut v,
        cosine(i32::from(angle), i32::from(length)),
        sine(i32::from(angle), i32::from(length)),
    );
    velocity(&v)
}
unsafe fn queue_len() -> usize {
    let mut h = disp_q.head;
    let mut n = 0;
    while !h.is_null() {
        n += 1;
        assert!(n <= 128);
        h = (*element(h)).succ;
    }
    n
}
unsafe fn junk(ship: &Ship<'_>, dx: i32, dy: i32, flags: u16, mass: BYTE) -> PoolElement {
    let mut fixture = PoolElement::new();
    let e = &raw mut *fixture;
    (*e).playerNr = -1;
    (*e).state_flags = flags;
    (*e).__bindgen_anon_1.life_span = 50;
    (*e).__bindgen_anon_2.hit_points = 10;
    (*e).__bindgen_anon_3.mass_points = mass;
    (*e).next.location = POINT {
        x: (i32::from((*ship.ptr()).next.location.x) + dx) as COORD,
        y: (i32::from((*ship.ptr()).next.location.y) + dy) as COORD,
    };
    (*e).current = (*e).next;
    put_element(e.cast());
    fixture
}

#[test]
fn native_weapon_uses_the_projected_source_without_mutating_its_live_target() {
    // SAFETY: the serialized fixture owns the source, target, and frame resources.
    battle_context::run(|| unsafe {
        let battle = Battle::new(8);
        let ship = Ship::with_frames(&battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        (*ship.ptr()).hTarget = ship.handle;
        let projected = with_rng(99, 1, || ship.initialize_from(ship.ptr(), 1)[0]);
        assert!(!projected.is_null());
        assert_eq!((*ship.ptr()).hTarget, ship.handle);
        let direct = with_rng(99, 1, || fire_from(ship.ptr()));
        assert!(!direct.is_null());
        assert!((*ship.ptr()).hTarget.is_null());
        assert_eq!(
            velocity(&(*element(projected)).velocity),
            velocity(&(*element(direct)).velocity)
        );
        assert_eq!(
            rgba((*prim(element(projected))).color),
            rgba((*prim(element(direct))).color)
        );
        assert_eq!(
            (*element(projected)).__bindgen_anon_4.turn_wait,
            (*element(direct)).__bindgen_anon_4.turn_wait
        );
        FreeElement(projected);
        FreeElement(direct);
    });
}

#[test]
fn descriptor_matches_all_c_stats_resources_and_callbacks() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(8);
        let d = battle::descriptor(SPECIES_ID_SLYLANDRO_ID);
        assert_eq!(
            d.ship_info.ship_flags,
            (SEEKING_WEAPON | CREW_IMMUNE) as u16
        );
        assert_eq!(d.ship_info.ship_cost, 17);
        assert_eq!(
            (
                d.ship_info.crew_level,
                d.ship_info.max_crew,
                d.ship_info.energy_level,
                d.ship_info.max_energy
            ),
            (12, 12, 20, 20)
        );
        assert_eq!(
            CStr::from_ptr(d.ship_info.race_strings_rsc),
            c"ship.slylandro.text"
        );
        assert_eq!(
            CStr::from_ptr(d.ship_info.icons_rsc),
            c"ship.slylandro.icons"
        );
        assert_eq!(
            CStr::from_ptr(d.ship_info.melee_icon_rsc),
            c"ship.slylandro.meleeicons"
        );
        assert!(
            d.ship_info.race_strings.is_null()
                && d.ship_info.icons.is_null()
                && d.ship_info.melee_icon.is_null()
        );
        assert_eq!(
            (d.fleet.strength, d.fleet.known_loc.x, d.fleet.known_loc.y),
            (65535, 333, 9812)
        );
        let c = d.characteristics;
        assert_eq!(
            (
                c.max_thrust,
                c.thrust_increment,
                c.energy_regeneration,
                c.energy_wait
            ),
            (60, 60, 0, 10)
        );
        assert_eq!(
            (
                c.weapon_energy_cost,
                c.special_energy_cost,
                c.turn_wait,
                c.thrust_wait,
                c.weapon_wait,
                c.special_wait,
                c.ship_mass
            ),
            (2, 0, 0, 0, 17, 20, 1)
        );
        for (i, size) in ["large", "medium", "small"].iter().enumerate() {
            assert_eq!(
                CStr::from_ptr(d.ship_data.ship_rsc[i]).to_str().unwrap(),
                format!("ship.slylandro.graphics.probe.{size}")
            );
            assert!(d.ship_data.weapon_rsc[i].is_null() && d.ship_data.special_rsc[i].is_null());
            assert!(
                d.ship_data.ship[i].is_null()
                    && d.ship_data.weapon[i].is_null()
                    && d.ship_data.special[i].is_null()
            );
        }
        let cap = d.ship_data.captain_control;
        assert_eq!(
            CStr::from_ptr(cap.captain_rsc),
            c"ship.slylandro.graphics.captain"
        );
        assert!(
            cap.background.is_null()
                && cap.turn.is_null()
                && cap.thrust.is_null()
                && cap.weapon.is_null()
                && cap.special.is_null()
        );
        assert_eq!(
            CStr::from_ptr(d.ship_data.victory_ditty_rsc),
            c"ship.slylandro.ditty"
        );
        assert_eq!(
            CStr::from_ptr(d.ship_data.ship_sounds_rsc),
            c"ship.slylandro.sounds"
        );
        assert!(d.ship_data.victory_ditty.is_null() && d.ship_data.ship_sounds.is_null());
        assert_eq!(
            (
                d.cyborg_control.ManeuverabilityIndex,
                d.cyborg_control.WeaponRange
            ),
            (0, 400)
        );
        assert!(
            d.race.unwrap().intelligence.is_some()
                && d.race.unwrap().preprocess.is_some()
                && d.race.unwrap().postprocess.is_some()
                && d.race.unwrap().init_weapon.is_some()
        );
        assert!(d.race.unwrap().uninit.is_none() && d.data.is_null() && d.code_ref.is_null());
        let fresh = battle::descriptor(SPECIES_ID_SLYLANDRO_ID);
        assert_eq!(
            fresh.race.unwrap().preprocess.map(|f| f as usize),
            d.race.unwrap().preprocess.map(|f| f as usize)
        );
        assert_eq!(
            fresh.race.unwrap().postprocess.map(|f| f as usize),
            d.race.unwrap().postprocess.map(|f| f as usize)
        );
        assert_eq!(
            fresh.race.unwrap().init_weapon.map(|f| f as usize),
            d.race.unwrap().init_weapon.map(|f| f as usize)
        );
        assert_eq!(
            fresh.race.unwrap().intelligence.map(|f| f as usize),
            d.race.unwrap().intelligence.map(|f| f as usize)
        );
    })
}

#[test]
fn root_bolts_have_exact_random_color_length_angle_and_folded_chain_length() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(8);
        let mut ship =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        ship.ship_mut().ShipFacing = 15;
        (*ship.ptr()).next.location = POINT { x: 321, y: -654 };
        let mut colors_seen = [false; 4];
        for seed in 1..65 {
            let (random, _) = randoms(seed);
            colors_seen[((random >> 8) & 3) as usize] = true;
            for counter in [0u8, 1, 8, 9, 16, 17, 255] {
                ship.ship_mut().weapon_counter = counter;
                (*ship.ptr()).hTarget = ship.handle;
                let h = with_rng(seed, 1, || fire_from(ship.ptr()));
                assert!(!h.is_null());
                let bolt = element(h);
                let wait = if counter == 0 { 17u8 } else { counter };
                let wait = if wait > 8 {
                    17u8.wrapping_sub(wait)
                } else {
                    wait
                };
                assert_eq!((*bolt).__bindgen_anon_4.turn_wait, wait);
                assert_eq!(
                    (*bolt).state_flags,
                    (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
                );
                assert_eq!(
                    (
                        (*bolt).__bindgen_anon_1.life_span,
                        (*bolt).__bindgen_anon_2.hit_points,
                        (*bolt).__bindgen_anon_3.mass_points,
                        (*bolt).__bindgen_anon_5.blast_offset
                    ),
                    (1, 1, 1, 1)
                );
                assert_eq!((*bolt).playerNr, 0);
                assert_eq!((*bolt).pParent, (*ship.ptr()).pParent);
                assert!(
                    (*bolt).preprocess_func.is_none()
                        && (*bolt).postprocess_func.is_some()
                        && (*bolt).collision_func.is_some()
                );
                assert_eq!(
                    ((*bolt).current.location.x, (*bolt).current.location.y),
                    (321, -654)
                );
                assert_eq!((*prim(bolt)).Type, gfx_object_LINE_PRIM as u8);
                assert_eq!(rgba((*prim(bolt)).color), bolt_color(random));
                assert_eq!(velocity(&(*bolt).velocity), bolt_velocity(60, -1, random));
                assert!(
                    (*ship.ptr()).hTarget.is_null(),
                    "ship targeting starts fresh and is cleared after tracking"
                );
                FreeElement(h);
            }
        }
        assert!(colors_seen.into_iter().all(|seen| seen));
    })
}

#[test]
fn lightning_tracks_all_turn_directions_and_draws_the_bolt_random_before_opposite_turn() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(8);
        let ship = Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        let mut enemy =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 1, [16; 3], [4, 4]);
        for (dx, dy, delta, facing) in [
            (0, -1000, 0, 0),
            (1000, 0, 4, 1),
            (-1000, 0, 12, 15),
            (0, 1000, 8, 0),
        ] {
            at(&mut enemy, 4000 + dx, 4000 + dy);
            for seed in [1, 2, 7, 44, 1000] {
                let (random, turn_random) = randoms(seed);
                let facing = if delta == 8 {
                    normalize_facing(((turn_random as u8 & 1) as i32 * 2) - 1)
                } else {
                    facing
                };
                let h = with_rng(seed, if delta == 8 { 2 } else { 1 }, || {
                    fire_from(ship.ptr())
                });
                let bolt = element(h);
                assert_eq!(
                    velocity(&(*bolt).velocity),
                    bolt_velocity(facing_to_angle(facing as i32) as COUNT, delta, random)
                );
                assert_eq!(rgba((*prim(bolt)).color), bolt_color(random));
                assert!((*ship.ptr()).hTarget.is_null());
                FreeElement(h);
            }
        }
    })
}

#[test]
fn roots_choose_nearest_live_visible_opponent_not_a_cached_target_ally_or_dead_ship() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(16);
        let ship = Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        let mut ally =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        at(&mut ally, 4000, 3999);
        let mut dead =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 1, [16; 3], [4, 4]);
        at(&mut dead, 4000, 3998);
        dead.descriptor_mut().ship_info.crew_level = 0;
        let mut cloaked =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 1, [16; 3], [4, 4]);
        at(&mut cloaked, 4000, 3997);
        (*prim(cloaked.ptr())).Type = gfx_object_NUM_PRIMS as u8;
        let mut far =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 1, [16; 3], [4, 4]);
        at(&mut far, 4000, 2000);
        let mut near =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 1, [16; 3], [4, 4]);
        at(&mut near, 4500, 4000);
        (*ship.ptr()).hTarget = far.handle;
        let (random, _) = randoms(313);
        let h = with_rng(313, 1, || fire_from(ship.ptr()));
        assert_eq!(
            velocity(&(*element(h)).velocity),
            bolt_velocity(4, 4, random)
        );
        assert!((*ship.ptr()).hTarget.is_null());
        FreeElement(h);
        (*ship.ptr()).state_flags |= APPEARING as u16;
        let h = with_rng(313, 1, || fire_from(ship.ptr()));
        assert_eq!(
            velocity(&(*element(h)).velocity),
            bolt_velocity(0, 0, random),
            "appearing ship can target cloaked opponents"
        );
        FreeElement(h);
        (*ship.ptr()).state_flags &= !(APPEARING as u16);
        (*near.ptr()).__bindgen_anon_1.life_span = 0;
        let h = with_rng(313, 1, || fire_from(ship.ptr()));
        assert_eq!(
            velocity(&(*element(h)).velocity),
            bolt_velocity(0, 0, random)
        );
        FreeElement(h);
    })
}

#[test]
fn child_segments_keep_unrounded_travel_angle_inherit_color_and_retain_selected_target() {
    battle_context::run(|| {
        unsafe {
            let _battle = Battle::new(16);
            let ship =
                Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
            let mut enemy =
                Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 1, [16; 3], [4, 4]);
            let mut cached =
                Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 1, [16; 3], [4, 4]);
            at(&mut cached, 6000, 4000);
            let h = fire_from(ship.ptr());
            let parent = element(h);
            (*parent).current.location = POINT { x: 4000, y: 4000 };
            (*parent).next.location = POINT { x: 4100, y: 4200 };
            (*parent).__bindgen_anon_4.turn_wait = 4;
            (*prim(parent)).color = Color {
                r: 3,
                g: 77,
                b: 129,
                a: 43,
            };
            for (angle, dx, dy, delta) in [
                (1, 0, -1000, 0),
                (1, 1000, 0, 4),
                (1, -1000, 0, 12),
                (1, 0, 1000, 8),
                (5, 0, -1000, 15),
            ] {
                at(&mut enemy, 4000 + dx, 4000 + dy);
                cached.descriptor_mut().ship_info.crew_level = 0;
                (*parent).velocity.travel_angle = angle;
                (*parent).hTarget = ptr::null_mut();
                let (random, _) = randoms(77);
                let child_h = with_rng(77, if delta == 8 { 2 } else { 1 }, || fire_from(parent));
                let child = element(child_h);
                assert_eq!(
                    velocity(&(*child).velocity),
                    bolt_velocity(angle, delta, random),
                    "angle {angle}, delta {delta}"
                );
                assert_eq!(rgba((*prim(child)).color), (3, 77, 129, 43));
                assert_eq!((*child).__bindgen_anon_4.turn_wait, 3);
                assert_eq!((*child).pParent, (*parent).pParent);
                assert_eq!(
                    ((*child).current.location.x, (*child).current.location.y),
                    (4100, 4200)
                );
                assert_eq!((*parent).hTarget, enemy.handle);
                FreeElement(child_h);
            }
            // A child with a cached target uses that target even if a nearer ship exists.
            cached.descriptor_mut().ship_info.crew_level = 12;
            at(&mut enemy, 4000, 3999);
            (*parent).velocity.travel_angle = 1;
            (*parent).hTarget = cached.handle;
            let (random, _) = randoms(99);
            let child_h = with_rng(99, 1, || fire_from(parent));
            assert_eq!((*parent).hTarget, cached.handle);
            assert_eq!(
                velocity(&(*element(child_h)).velocity),
                bolt_velocity(1, 4, random)
            );
            FreeElement(child_h);
            // A dead cached target is cleared without starting a fresh queue scan.
            cached.descriptor_mut().ship_info.crew_level = 0;
            (*parent).hTarget = cached.handle;
            let child_h = with_rng(99, 1, || fire_from(parent));
            assert!((*parent).hTarget.is_null());
            assert_eq!(
                velocity(&(*element(child_h)).velocity),
                bolt_velocity(1, -1, random)
            );
            FreeElement(child_h);
            enemy.descriptor_mut().ship_info.crew_level = 0;
            (*parent).__bindgen_anon_4.turn_wait = 0;
            let child_h = with_rng(99, 1, || fire_from(parent));
            assert_eq!((*element(child_h)).__bindgen_anon_4.turn_wait, 255);
            assert_eq!(
                velocity(&(*element(child_h)).velocity),
                bolt_velocity(1, -1, random)
            );
            FreeElement(child_h);
            FreeElement(h);
        }
    })
}

#[test]
fn lightning_postprocess_extends_chains_until_zero_or_collision() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(32);
        let mut ship =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        ship.ship_mut().weapon_counter = 8;
        let h = fire_from(ship.ptr());
        put_element(h);
        let mut parent = element(h);
        for wait in (0..8).rev() {
            (*parent).next = (*parent).current;
            let before = queue_len();
            with_rng(56, 1, || {
                uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                    (*parent).postprocess_func.unwrap(),
                    parent,
                )
            });
            assert_eq!(queue_len(), before + 1);
            let child = element(disp_q.tail);
            assert_ne!(child, parent);
            assert_eq!((*child).__bindgen_anon_4.turn_wait, wait);
            assert_eq!(rgba((*prim(child)).color), rgba((*prim(parent)).color));
            parent = child;
        }
        let before = queue_len();
        with_rng(56, 0, || {
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                (*parent).postprocess_func.unwrap(),
                parent,
            )
        });
        assert_eq!(queue_len(), before);
        (*parent).__bindgen_anon_4.turn_wait = 5;
        (*parent).state_flags |= COLLISION as u16;
        with_rng(56, 0, || {
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                (*parent).postprocess_func.unwrap(),
                parent,
            )
        });
        assert_eq!(queue_len(), before);
    })
}

#[test]
fn allocation_failure_consumes_no_lightning_random_and_keeps_initializer_count() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(2);
        let mut ship =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        ship.ship_mut().weapon_counter = 8;
        let h = fire_from(ship.ptr());
        put_element(h);
        let parent = element(h);
        assert!(with_rng(7, 0, || fire_from(ship.ptr())).is_null());
        with_rng(7, 0, || {
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                (*parent).postprocess_func.unwrap(),
                parent,
            )
        });
        assert_eq!({ disp_q.tail }, h);
        with_rng(7, 0, || ship.postprocess());
        assert_eq!({ disp_q.tail }, h);
    })
}

#[test]
fn lightning_collision_folds_and_subtracts_counter_then_calls_generic_weapon_collision() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(32);
        let mut ship =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        let h = fire_from(ship.ptr());
        let beam = element(h);
        let collide = (*beam).collision_func.unwrap();
        let mut pa = POINT { x: 10, y: 20 };
        let mut pb = POINT { x: 30, y: 40 };
        let mut target = PoolElement::new();
        target.__bindgen_anon_1.life_span = 2;
        target.__bindgen_anon_2.hit_points = 3;
        target.__bindgen_anon_3.mass_points = 2;
        target.state_flags = FINITE_LIFE as u16;
        with_rng(13, 0, || {
            uqm_rust::battle::process_native::invoke_collision::invoke_legacy_collision(
                collide,
                beam,
                &mut pa,
                &mut *target,
                &mut pb,
                uqm_rust::battle::process_native::invoke_collision::CollisionCall::Hook,
            )
        });
        assert_eq!(
            target.__bindgen_anon_2.hit_points, 2,
            "generic weapon damage applied"
        );
        assert_ne!((*beam).state_flags & (COLLISION | NONSOLID) as u16, 0);
        assert_eq!((*beam).__bindgen_anon_2.hit_points, 0);
        assert_eq!((*beam).__bindgen_anon_1.life_span, 0);
        assert_eq!(
            (*beam).state_flags & DISAPPEARING as u16,
            0,
            "line prims do not disappear"
        );
        let tail = disp_q.tail;
        for counter in [0u8, 1, 8, 9, 16, 17, 255] {
            for wait in [0u8, 1, 8, 255] {
                ship.ship_mut().weapon_counter = counter;
                (*beam).__bindgen_anon_4.turn_wait = wait;
                let folded = if counter > 8 {
                    17u8.wrapping_sub(counter)
                } else {
                    counter
                };
                with_rng(13, 0, || {
                    uqm_rust::battle::process_native::invoke_collision::invoke_legacy_collision(
                        collide,
                        beam,
                        &mut pa,
                        &mut *target,
                        &mut pb,
                        uqm_rust::battle::process_native::invoke_collision::CollisionCall::Hook,
                    )
                });
                assert_eq!(ship.ship_mut().weapon_counter, folded.wrapping_sub(wait));
                assert_eq!((*beam).__bindgen_anon_4.turn_wait, 0);
                assert_eq!(
                    { disp_q.tail },
                    tail,
                    "generic collision ignores an already-collided beam"
                );
            }
        }
        FreeElement(h);
    })
}

#[test]
fn ship_postprocess_emits_only_during_the_sixteen_active_weapon_ticks() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(32);
        let mut ship =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        for counter in [0u8, 17, 18, 255, 1, 8, 9, 16] {
            ship.ship_mut().weapon_counter = counter;
            let before = queue_len();
            let emits = counter != 0 && counter < 17;
            with_rng(303, usize::from(emits), || ship.postprocess());
            assert_eq!(queue_len(), before + usize::from(emits));
            if emits {
                assert_eq!((*element(disp_q.tail)).pParent, (*ship.ptr()).pParent);
            }
            assert_eq!(ship.ship_mut().weapon_counter, counter);
        }
    })
}

#[test]
fn movement_reverses_on_thrust_press_turns_with_left_priority_and_animates_next_frame() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(8);
        let mut ship =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        for (initial, old, current, turn_wait, thrust_wait, facing) in [
            (0, 0, 0, 0, 0, 0),
            (0, 0, THRUST, 0, 0, 8),
            (8, THRUST, THRUST, 0, 0, 8),
            (0, 0, THRUST | LEFT | RIGHT, 0, 0, 7),
            (0, 0, LEFT, 0, 0, 15),
            (15, 0, RIGHT, 0, 0, 0),
            (0, 0, THRUST | LEFT, 2, 0, 8),
            (0, 0, RIGHT, 0, 2, 1),
            (u16::MAX, 0, 0, 2, 2, 15),
        ] {
            ship.ship_mut().ShipFacing = initial;
            ship.ship_mut().old_status_flags = old as u16;
            ship.ship_mut().cur_status_flags = (current | SHIP_IN_GRAVITY_WELL) as u16;
            (*ship.ptr()).state_flags = PLAYER_SHIP as u16;
            (*ship.ptr()).__bindgen_anon_4.turn_wait = turn_wait;
            (*ship.ptr()).__bindgen_anon_5.thrust_wait = thrust_wait;
            (*ship.ptr()).velocity = mem::zeroed();
            (*ship.ptr()).current.image.frame = ship.raw_frame(0, 3);
            (*ship.ptr()).next.image.frame = ship.raw_frame(0, 15);
            with_rng(64, 0, || ship.preprocess());
            assert_eq!(ship.ship_mut().ShipFacing, facing);
            assert_eq!(
                (*ship.ptr()).__bindgen_anon_4.turn_wait,
                if turn_wait == 0 { 1 } else { turn_wait }
            );
            assert_eq!(
                (*ship.ptr()).__bindgen_anon_5.thrust_wait,
                if thrust_wait == 0 { 1 } else { thrust_wait }
            );
            assert_eq!(frame_index((*ship.ptr()).next.image.frame), 0);
            assert_ne!((*ship.ptr()).state_flags & CHANGING as u16, 0);
            if thrust_wait == 0 {
                assert_eq!(velocity(&(*ship.ptr()).velocity), vector(60, facing));
                assert_ne!(
                    ship.ship_mut().cur_status_flags & SHIP_AT_MAX_SPEED as u16,
                    0
                );
                assert_eq!(
                    ship.ship_mut().cur_status_flags & SHIP_IN_GRAVITY_WELL as u16,
                    0
                );
            } else {
                assert_eq!(velocity(&(*ship.ptr()).velocity), [0; 9]);
                assert_ne!(
                    ship.ship_mut().cur_status_flags & SHIP_IN_GRAVITY_WELL as u16,
                    0
                );
            }
        }
        ship.descriptor_mut().characteristics.turn_wait = 255;
        ship.descriptor_mut().characteristics.thrust_wait = 255;
        (*ship.ptr()).__bindgen_anon_4.turn_wait = 0;
        (*ship.ptr()).__bindgen_anon_5.thrust_wait = 0;
        ship.preprocess();
        assert_eq!((*ship.ptr()).__bindgen_anon_4.turn_wait, 0);
        assert_eq!((*ship.ptr()).__bindgen_anon_5.thrust_wait, 0);
        for flags in [APPEARING, NONSOLID, APPEARING | NONSOLID] {
            (*ship.ptr()).state_flags = flags as u16;
            let facing = ship.ship_mut().ShipFacing;
            let frame = (*ship.ptr()).next.image.frame;
            let v = velocity(&(*ship.ptr()).velocity);
            with_rng(64, 0, || ship.preprocess());
            assert_eq!(ship.ship_mut().ShipFacing, facing);
            assert_eq!((*ship.ptr()).next.image.frame, frame);
            assert_eq!(velocity(&(*ship.ptr()).velocity), v);
            assert_eq!((*ship.ptr()).state_flags, flags as u16);
        }
    })
}

#[test]
fn harvester_filters_all_junk_flags_collision_rules_and_inclusive_distance_boundaries() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(64);
        let mut ship =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        ship.ship_mut().cur_status_flags = SPECIAL as u16;
        (*ship.ptr()).state_flags |= COLLISION as u16;
        let mut objects = Vec::new();
        let mut fixtures = Vec::new();
        for (flags, mass, dx, dy, harvest) in [
            (0, 1, 0, 0, true),
            (0, 0, 0, 0, true),
            (0, 100, 0, 0, true),
            (0, 101, 0, 0, false),
            (APPEARING, 1, 0, 0, false),
            (PLAYER_SHIP, 1, 0, 0, false),
            (FINITE_LIFE, 1, 0, 0, false),
            (NONSOLID, 1, 0, 0, false),
            (DISAPPEARING, 1, 0, 0, false),
            (COLLISION, 1, 0, 0, false),
            (0, 1, 78 * 4, 0, true),
            (0, 1, -(78 * 4 + 3), 0, true),
            (0, 1, 79 * 4, 0, false),
            (0, 1, 55 * 4, -55 * 4, true),
            (0, 1, 56 * 4, 56 * 4, false),
        ] {
            let mut fixture = junk(&ship, dx, dy, flags as u16, mass);
            objects.push((&raw mut *fixture, harvest));
            fixtures.push(fixture);
        }
        let mut owned_fixture = junk(&ship, 0, 0, 0, 1);
        let owned = &raw mut *owned_fixture;
        (*owned).playerNr = 0;
        objects.push((owned, false));
        let mut foreign_fixture = junk(&ship, 0, 0, 0, 1);
        let foreign = &raw mut *foreign_fixture;
        (*foreign).playerNr = 1;
        objects.push((foreign, false));
        ship.descriptor_mut().ship_info.energy_level = 3;
        with_rng(11, 0, || ship.postprocess());
        for (e, harvest) in objects {
            assert_eq!(
                (*e).__bindgen_anon_1.life_span,
                if harvest { 0 } else { 50 }
            );
            if harvest {
                assert_ne!((*e).state_flags & NONSOLID as u16, 0);
            }
            assert_eq!(
                (*e).__bindgen_anon_2.hit_points,
                10,
                "harvesting changes life, not hit points"
            );
        }
        assert_eq!(ship.descriptor_mut().ship_info.energy_level, 20);
        assert_eq!(ship.ship_mut().energy_counter, 10);
        assert_eq!(ship.ship_mut().special_counter, 20);
    })
}

#[test]
fn harvester_same_parent_zero_mass_and_ship_solidity_match_collision_possible_macro() {
    battle_context::run(|| unsafe {
        for (ship_flags, obj_flags, same_parent, ship_mass, obj_mass, harvest) in [
            (IGNORE_SIMILAR, IGNORE_SIMILAR, true, 1, 1, false),
            (IGNORE_SIMILAR, IGNORE_SIMILAR, false, 1, 1, true),
            (0, IGNORE_SIMILAR, true, 1, 1, true),
            (0, 0, false, 0, 0, false),
            (0, 0, false, 0, 1, true),
            (NONSOLID, 0, false, 1, 1, true),
            (DISAPPEARING, 0, false, 1, 1, true),
        ] {
            let _battle = Battle::new(4);
            let mut ship =
                Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            (*ship.ptr()).state_flags |= ship_flags as u16;
            (*ship.ptr()).__bindgen_anon_3.mass_points = ship_mass;
            let mut e_fixture = junk(&ship, 0, 0, obj_flags as u16, obj_mass);
            let e = &raw mut *e_fixture;
            if same_parent {
                (*e).pParent = (*ship.ptr()).pParent;
            }
            with_rng(2, 0, || ship.postprocess());
            assert_eq!((*e).__bindgen_anon_1.life_span == 0, harvest);
            assert_eq!(
                ship.ship_mut().special_counter,
                if harvest { 20 } else { 0 }
            );
        }
    })
}

#[test]
fn harvester_requires_key_and_ready_counter_but_can_run_at_full_energy() {
    battle_context::run(|| {
        unsafe {
            let _battle = Battle::new(16);
            let mut ship =
                Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
            for (key, counter, energy, harvest) in [
                (false, 0, 0, false),
                (true, 1, 0, false),
                (true, 0, 20, true),
            ] {
                let mut e_fixture = junk(&ship, 0, 0, 0, 1);
                let e = &raw mut *e_fixture;
                ship.ship_mut().cur_status_flags = if key { SPECIAL as u16 } else { 0 };
                ship.ship_mut().special_counter = counter;
                ship.descriptor_mut().ship_info.energy_level = energy;
                with_rng(6, 0, || ship.postprocess());
                assert_eq!((*e).__bindgen_anon_1.life_span == 0, harvest);
                assert_eq!(
                    ship.ship_mut().special_counter,
                    if harvest { 20 } else { counter }
                );
                assert_eq!(
                    ship.descriptor_mut().ship_info.energy_level,
                    if harvest { 20 } else { energy }
                );
                // Exclude this object from later cases.
                (*e).state_flags |= NONSOLID as u16;
            }
            ship.ship_mut().special_counter = 0;
            ship.descriptor_mut().ship_info.energy_level = 0;
            ship.postprocess();
            assert_eq!(ship.ship_mut().special_counter, 0);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 0);
            at(&mut ship, -32760, 4000);
            let mut e_fixture = junk(&ship, 0, 0, 0, 1);
            let e = &raw mut *e_fixture;
            (*e).next.location.x = 32760;
            ship.postprocess();
            assert_eq!(
                (*e).__bindgen_anon_1.life_span,
                0,
                "subtraction truncates to SIZE before absolute value"
            );
        }
    })
}

#[test]
fn ai_encounter_weapon_range_recharge_range_and_close_pursuit_boundaries() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(8);
        let mut ship =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        let enemy = Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 1, [16; 3], [4, 4]);
        let mut concerns: [EVALUATE_DESC; 5] = mem::zeroed();
        concerns[0].ObjectPtr = enemy.ptr();
        concerns[2].ObjectPtr = enemy.ptr();
        concerns[4].ObjectPtr = enemy.ptr();
        for (activity, turns, energy, special_turns, move_state, expected_move) in [
            (0, 6, 19, 14, MOVEMENT_STATE_PURSUE, MOVEMENT_STATE_ENTICE),
            (0, 7, 19, 15, MOVEMENT_STATE_PURSUE, MOVEMENT_STATE_PURSUE),
            (
                0x102,
                14,
                19,
                14,
                MOVEMENT_STATE_PURSUE,
                MOVEMENT_STATE_PURSUE,
            ),
            (0, 15, 20, 14, MOVEMENT_STATE_PURSUE, MOVEMENT_STATE_PURSUE),
            (0, 6, 19, 14, MOVEMENT_STATE_AVOID, MOVEMENT_STATE_AVOID),
        ] {
            GlobData.Game_state.CurrentActivity = activity;
            concerns[0].which_turn = turns;
            concerns[0].MoveState = move_state;
            concerns[2].ObjectPtr = enemy.ptr();
            concerns[4].which_turn = special_turns;
            ship.descriptor_mut().ship_info.energy_level = energy;
            ship.ship_mut().ship_input_state = (WEAPON | SPECIAL) as u8;
            (*ship.ptr()).__bindgen_anon_5.thrust_wait = 255;
            with_rng(17, 0, || think(&mut ship, &mut concerns, 0));
            assert_eq!(concerns[0].MoveState, expected_move);
            assert_eq!(
                ship.ship_mut().ship_input_state & WEAPON as u8 != 0,
                turns <= 14
            );
            assert_eq!(
                ship.ship_mut().ship_input_state & SPECIAL as u8 != 0,
                energy < 20 && special_turns <= 14
            );
            assert_eq!(
                concerns[2].ObjectPtr.is_null(),
                activity as u8 == IN_ENCOUNTER as u8
            );
            assert_eq!((*ship.ptr()).__bindgen_anon_5.thrust_wait, 255);
        }
        concerns[0].ObjectPtr = ptr::null_mut();
        concerns[4].ObjectPtr = ptr::null_mut();
        ship.ship_mut().ship_input_state = (SPECIAL | WEAPON) as u8;
        with_rng(17, 0, || think(&mut ship, &mut concerns, 0));
        assert_eq!(
            ship.ship_mut().ship_input_state & (SPECIAL | WEAPON) as u8,
            0
        );
    })
}

#[test]
fn empty_energy_expands_concerns_only_without_gravity_and_cooldown_and_suppresses_thrust() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(8);
        let mut ship =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        let mut e_fixture = junk(&ship, 1000, 0, 0, 1);
        let e = &raw mut *e_fixture;
        let mut concerns: [EVALUATE_DESC; 5] = mem::zeroed();
        concerns[4].ObjectPtr = e;
        concerns[4].which_turn = 10;
        concerns[4].MoveState = MOVEMENT_STATE_PURSUE;
        for (energy, cooldown, gravity, turns) in [
            (0, 0, false, true),
            (1, 0, false, false),
            (0, 1, false, false),
            (0, 0, true, false),
        ] {
            ship.descriptor_mut().ship_info.energy_level = energy;
            ship.ship_mut().special_counter = cooldown;
            ship.ship_mut().ship_input_state = 0;
            ship.ship_mut().ShipFacing = 0;
            (*ship.ptr()).__bindgen_anon_4.turn_wait = 0;
            (*ship.ptr()).__bindgen_anon_5.thrust_wait = 0;
            concerns[3].ObjectPtr = if gravity { e } else { ptr::null_mut() };
            with_rng(8, 0, || think(&mut ship, &mut concerns, 0));
            assert_eq!(ship.ship_mut().ship_input_state & RIGHT as u8 != 0, turns);
            assert_eq!(ship.ship_mut().ShipFacing, if turns { 1 } else { 0 });
            assert_eq!(ship.ship_mut().ship_input_state & THRUST as u8, 0);
            assert_eq!((*ship.ptr()).__bindgen_anon_5.thrust_wait, 0);
            assert_eq!(
                ship.ship_mut().ship_input_state & SPECIAL as u8,
                SPECIAL as u8
            );
        }
    })
}

#[test]
fn native_ai_preserves_fifth_concern_and_expands_active_count() {
    // SAFETY: battle_context serializes the pool, ship, and loaded fixture resources.
    battle_context::run(|| unsafe {
        use uqm_rust::engine::battle::{Concern, Concerns};
        let battle = Battle::new(8);
        let mut ship =
            Ship::with_frames(&battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        let mut debris = junk(&ship, 1000, 0, 0, 1);
        let debris = &raw mut *debris;
        ship.descriptor_mut().ship_info.energy_level = 0;
        ship.ship_mut().special_counter = 0;
        with_rng(8, 0, || {
            uqm_rust::app::bridge::enter(|cx| {
                let junk_id = cx.elements().id(debris.cast()).unwrap();
                let mut concerns = Concerns {
                    subject: cx.weapon_origin(ship.id),
                    objects: [Concern {
                        object: None,
                        facing: 0,
                        which_turn: u16::MAX,
                        movement: 0,
                    }; 10],
                    count: FIRST_EMPTY_INDEX as u16,
                };
                concerns.objects[FIRST_EMPTY_INDEX as usize] = Concern {
                    object: Some(junk_id),
                    facing: 0,
                    which_turn: 10,
                    movement: MOVEMENT_STATE_PURSUE,
                };
                assert!(cx.race_intelligence(ship.ship_id, &mut concerns));
                assert_eq!(concerns.count, 5);
                assert_eq!(concerns.objects.len(), 10);
                assert_eq!(concerns.objects[4].object, Some(junk_id));
                assert_ne!(cx.ship_state(ship.ship_id).input & SPECIAL as u8, 0);
                assert_ne!(cx.ship_state(ship.ship_id).input & RIGHT as u8, 0);
            })
        });
    });
}

#[test]
fn shared_ai_turns_toward_a_ship_without_applying_generic_thrust() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(8);
        let mut ship =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        let mut enemy =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 1, [16; 3], [4, 4]);
        at(&mut enemy, 5000, 4000);
        let mut concerns: [EVALUATE_DESC; 5] = mem::zeroed();
        concerns[0].ObjectPtr = enemy.ptr();
        concerns[0].MoveState = MOVEMENT_STATE_PURSUE;
        concerns[0].which_turn = 10;
        ship.descriptor_mut().ship_info.energy_level = 1;
        let before = velocity(&(*ship.ptr()).velocity);
        with_rng(71, 0, || think(&mut ship, &mut concerns, 1));
        assert_eq!(ship.ship_mut().ShipFacing, 1);
        assert_ne!(ship.ship_mut().ship_input_state & RIGHT as u8, 0);
        assert_eq!(ship.ship_mut().ship_input_state & THRUST as u8, 0);
        assert_eq!((*ship.ptr()).__bindgen_anon_5.thrust_wait, 0);
        assert_eq!(velocity(&(*ship.ptr()).velocity), before);
    })
}

unsafe fn at(ship: &mut Ship<'_>, x: COORD, y: COORD) {
    ship.element_mut().current.location = POINT { x, y };
    ship.element_mut().next.location = POINT { x, y };
}
unsafe fn fire_from(source: *mut ELEMENT) -> HELEMENT {
    // SAFETY: the fixture retains the source element in the published battle pool.
    uqm_rust::app::bridge::enter(|cx| {
        let source = cx.elements().id(source.cast()).unwrap();
        uqm_rust::ships::races::lightning_segment(cx, source)
            .map_or(ptr::null_mut(), |id| cx.legacy_element(id).cast())
    })
}
unsafe fn think(ship: &mut Ship<'_>, concerns: &mut [EVALUATE_DESC; 5], count: COUNT) {
    ship.intelligence(&mut *ship.ptr(), concerns, count);
}
