//! Exercise Slylandro's live callbacks, target selection, and RNG order.
#![forbid(unsafe_code)]

#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
use battle::{Battle as PoolBattle, PoolElement, Ship};
use uqm_engine::battle::ai_constants::FIRST_EMPTY_INDEX;
use uqm_engine::battle::ai_constants::IGNORE_SIMILAR;
use uqm_engine::battle::ai_constants::LEFT;
use uqm_engine::battle::ai_constants::MOVEMENT_STATE_AVOID;
use uqm_engine::battle::ai_constants::MOVEMENT_STATE_ENTICE;
use uqm_engine::battle::ai_constants::MOVEMENT_STATE_PURSUE;
use uqm_engine::battle::ai_constants::RIGHT;
use uqm_engine::battle::ai_constants::SEEKING_WEAPON;
use uqm_engine::battle::ai_constants::SPECIAL;
use uqm_engine::battle::ai_constants::THRUST;
use uqm_engine::battle::ai_constants::WEAPON;
use uqm_engine::battle::flags::APPEARING;
use uqm_engine::battle::flags::CHANGING;
use uqm_engine::battle::flags::COLLISION;
use uqm_engine::battle::flags::CREW_IMMUNE;
use uqm_engine::battle::flags::DISAPPEARING;
use uqm_engine::battle::flags::FINITE_LIFE;
use uqm_engine::battle::flags::NONSOLID;
use uqm_engine::battle::flags::PLAYER_SHIP;
use uqm_engine::battle::flags::SHIP_AT_MAX_SPEED;
use uqm_engine::battle::flags::SHIP_IN_GRAVITY_WELL;
use uqm_engine::battle::Concern;
use uqm_engine::battle::ElementId;
use uqm_engine::battle::EMPTY_PRIMITIVE;
use uqm_engine::battle::LINE_PRIMITIVE;
use uqm_engine::graphics::geometry::Color;
use uqm_engine::graphics::geometry::Point;
use uqm_engine::math::battle_types::ONE_SHIFT;
use uqm_engine::math::random;
use uqm_engine::math::seed_random;
use uqm_engine::math::velocity::VelocityDesc;
use uqm_engine::math::velocity::VELOCITY_SHIFT;
use uqm_engine::species::species_id::SPECIES_ID_SLYLANDRO_ID;
use uqm_engine::world::activity::activity_kinds::IN_ENCOUNTER;

#[path = "support/special_hooks.rs"]
mod special_hooks;
use uqm_content::ships::native::{cosine, facing_to_angle, normalize_facing, sine};
use uqm_engine::collections::observation;

struct Battle {
    _pool: PoolBattle,
    activity: u16,
}
impl Battle {
    fn new(capacity: u16) -> Self {
        let activity = crate::battle_context::with(|cx| cx.world.activity());
        let pool = PoolBattle::new(capacity);
        crate::battle_context::with(|cx| cx.world.set_activity(0));
        Self {
            _pool: pool,
            activity,
        }
    }
}
impl Drop for Battle {
    fn drop(&mut self) {
        crate::battle_context::with(|cx| cx.world.set_activity(self.activity));
    }
}

fn rgba(c: Color) -> (u8, u8, u8, u8) {
    (c.r, c.g, c.b, c.a)
}
fn velocity(v: &VelocityDesc) -> [i32; 9] {
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
fn vector(speed: i16, facing: u16) -> [i32; 9] {
    let mut v = VelocityDesc::new();
    uqm_modes::battle::velocity::set_velocity_vector(&mut v, speed, facing);
    velocity(&v)
}
fn with_rng<T>(seed: u32, draws: usize, action: impl FnOnce() -> T) -> T {
    seed_random(seed);
    for _ in 0..draws {
        random();
    }
    let next = random();
    seed_random(seed);
    let result = action();
    assert_eq!(random(), next, "seed {seed}: expected {draws} draws");
    result
}
fn randoms(seed: u32) -> (u32, u32) {
    seed_random(seed);
    (random(), random())
}
fn bolt_color(random: u32) -> (u8, u8, u8, u8) {
    match (random >> 8) & 3 {
        0 => (255, 255, 255, 255),
        1 => (181, 189, 255, 255),
        2 => (49, 57, 255, 255),
        _ => (0, 0, 198, 255),
    }
}
fn bolt_velocity(angle: u16, delta: i16, random: u32) -> [i32; 9] {
    let angle = match delta {
        -1 | 8 => angle.wrapping_add(random as u16),
        0 => {
            if random & 1 != 0 {
                angle.wrapping_sub(1)
            } else {
                angle.wrapping_add(1)
            }
        }
        1..=7 => angle.wrapping_add(random as u16 & 15),
        _ => angle.wrapping_sub(random as u16 & 15),
    };
    let length = ((((random >> 16) & 31) + 4) << (ONE_SHIFT + VELOCITY_SHIFT)) as i16;
    let mut v = VelocityDesc::default();
    (v).set_components(
        i32::from(cosine(i32::from(angle), i32::from(length))),
        i32::from(sine(i32::from(angle), i32::from(length))),
    );
    velocity(&v)
}
fn queue_len() -> usize {
    let mut cursor = battle::head();
    let mut n = 0;
    while let Some(h) = cursor {
        n += 1;
        assert!(n <= 128);
        cursor = battle::succ(h);
    }
    n
}
fn junk(ship: &Ship<'_>, dx: i32, dy: i32, flags: u16, mass: u8) -> PoolElement {
    let fixture = PoolElement::new();
    let e = fixture.id;
    battle::get_mut(e).player_nr = -1;
    battle::get_mut(e).state_flags = flags;
    battle::get_mut(e).life = 50;
    battle::get_mut(e).hits = 10;
    battle::get_mut(e).mass = mass;
    battle::get_mut(e).next.location = Point {
        x: (i32::from(battle::get(ship.id).next.location.x) + dx) as i16,
        y: (i32::from(battle::get(ship.id).next.location.y) + dy) as i16,
    };
    battle::get_mut(e).current = battle::get(e).next.clone();
    battle::put(e);
    fixture
}

#[test]
fn native_weapon_uses_the_projected_source_without_mutating_its_live_target() {
    battle_context::run(|_| {
        let battle = Battle::new(8);
        let ship = Ship::with_frames(&battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        battle::get_mut(ship.id).target = Some(ship.id);
        let projected = with_rng(99, 1, || ship.initialize_from(ship.id, 1)[0]).unwrap();
        assert_eq!(battle::get(ship.id).target, Some(ship.id));
        let direct = with_rng(99, 1, || fire_from(ship.id)).unwrap();
        assert!(battle::get(ship.id).target.is_none());
        assert_eq!(
            velocity(&battle::get(projected).velocity),
            velocity(&battle::get(direct).velocity)
        );
        assert_eq!(
            rgba(battle::prim(projected).color),
            rgba(battle::prim(direct).color)
        );
        assert_eq!(battle::get(projected).turn, battle::get(direct).turn);
        battle_context::dispose(projected);
        battle_context::dispose(direct);
    });
}

#[test]
fn descriptor_matches_all_c_stats_resources_and_callbacks() {
    battle_context::run(|_| {
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
            d.ship_info.race_strings_rsc.unwrap(),
            c"ship.slylandro.text"
        );
        assert_eq!(d.ship_info.icons_rsc.unwrap(), c"ship.slylandro.icons");
        assert_eq!(
            d.ship_info.melee_icon_rsc.unwrap(),
            c"ship.slylandro.meleeicons"
        );
        assert!(
            d.ship_info.race_strings.is_none()
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
                d.ship_data.ship_rsc[i].unwrap().to_str().unwrap(),
                format!("ship.slylandro.graphics.probe.{size}")
            );
            assert!(d.ship_data.weapon_rsc[i].is_none() && d.ship_data.special_rsc[i].is_none());
            assert!(
                d.ship_data.ship.view(i).is_absent()
                    && d.ship_data.weapon.view(i).is_absent()
                    && d.ship_data.special.view(i).is_absent()
            );
        }
        let cap = &d.ship_data.captain_control;
        assert_eq!(cap.captain_rsc.unwrap(), c"ship.slylandro.graphics.captain");
        assert!(
            cap.background.is_null()
                && cap.turn.is_null()
                && cap.thrust.is_null()
                && cap.weapon.is_null()
                && cap.special.is_absent()
        );
        assert_eq!(
            d.ship_data.victory_ditty_rsc.unwrap(),
            c"ship.slylandro.ditty"
        );
        assert_eq!(
            d.ship_data.ship_sounds_rsc.unwrap(),
            c"ship.slylandro.sounds"
        );
        assert!(d.ship_data.victory_ditty.is_null() && d.ship_data.ship_sounds.is_none());
        assert_eq!(
            (
                d.cyborg_control.maneuverability_index,
                d.cyborg_control.weapon_range
            ),
            (0, 400)
        );
        assert!(
            d.race.unwrap().intelligence.is_some()
                && d.race.unwrap().preprocess.is_some()
                && d.race.unwrap().postprocess.is_some()
                && d.race.unwrap().init_weapon.is_some()
        );
        assert!(d.race.unwrap().uninit.is_none());
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
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        let mut ship =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        ship.ship_mut().ship_facing = 15;
        battle::get_mut(ship.id).next.location = Point { x: 321, y: -654 };
        let mut colors_seen = [false; 4];
        for seed in 1..65 {
            let (random, _) = randoms(seed);
            colors_seen[((random >> 8) & 3) as usize] = true;
            for counter in [0u8, 1, 8, 9, 16, 17, 255] {
                ship.ship_mut().weapon_counter = counter;
                battle::get_mut(ship.id).target = Some(ship.id);
                let h = with_rng(seed, 1, || fire_from(ship.id)).unwrap();
                let bolt = h;
                let wait = if counter == 0 { 17u8 } else { counter };
                let wait = if wait > 8 {
                    17u8.wrapping_sub(wait)
                } else {
                    wait
                };
                assert_eq!(battle::get(bolt).turn, wait);
                assert_eq!(
                    battle::get(bolt).state_flags,
                    (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
                );
                assert_eq!(
                    (
                        battle::get(bolt).life,
                        battle::get(bolt).hits,
                        battle::get(bolt).mass,
                        battle::get(bolt).thrust
                    ),
                    (1, 1, 1, 1)
                );
                assert_eq!(battle::get(bolt).player_nr, 0);
                assert_eq!(battle::get(bolt).parent, battle::get(ship.id).parent);
                assert!(
                    battle::get(bolt).preprocess_func.is_none()
                        && battle::get(bolt).postprocess_func.is_some()
                        && battle::get(bolt).collision_func.is_some()
                );
                assert_eq!(
                    (
                        battle::get(bolt).current.location.x,
                        battle::get(bolt).current.location.y
                    ),
                    (321, -654)
                );
                assert_eq!(battle::prim(bolt).kind, LINE_PRIMITIVE);
                assert_eq!(rgba(battle::prim(bolt).color), bolt_color(random));
                assert_eq!(
                    velocity(&battle::get(bolt).velocity),
                    bolt_velocity(60, -1, random)
                );
                assert!(
                    battle::get(ship.id).target.is_none(),
                    "ship targeting starts fresh and is cleared after tracking"
                );
                battle_context::dispose(h);
            }
        }
        assert!(colors_seen.into_iter().all(|seen| seen));
    })
}

#[test]
fn lightning_tracks_all_turn_directions_and_draws_the_bolt_random_before_opposite_turn() {
    battle_context::run(|_| {
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
                let h =
                    with_rng(seed, if delta == 8 { 2 } else { 1 }, || fire_from(ship.id)).unwrap();
                let bolt = h;
                assert_eq!(
                    velocity(&battle::get(bolt).velocity),
                    bolt_velocity(facing_to_angle(facing as i32) as u16, delta, random)
                );
                assert_eq!(rgba(battle::prim(bolt).color), bolt_color(random));
                assert!(battle::get(ship.id).target.is_none());
                battle_context::dispose(h);
            }
        }
    })
}

#[test]
fn roots_choose_nearest_live_visible_opponent_not_a_cached_target_ally_or_dead_ship() {
    battle_context::run(|_| {
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
        battle::prim_mut(cloaked.id).kind = EMPTY_PRIMITIVE;
        let mut far =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 1, [16; 3], [4, 4]);
        at(&mut far, 4000, 2000);
        let mut near =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 1, [16; 3], [4, 4]);
        at(&mut near, 4500, 4000);
        battle::get_mut(ship.id).target = Some(far.id);
        let (random, _) = randoms(313);
        let h = with_rng(313, 1, || fire_from(ship.id)).unwrap();
        assert_eq!(
            velocity(&battle::get(h).velocity),
            bolt_velocity(4, 4, random)
        );
        assert!(battle::get(ship.id).target.is_none());
        battle_context::dispose(h);
        battle::get_mut(ship.id).state_flags |= APPEARING as u16;
        let h = with_rng(313, 1, || fire_from(ship.id)).unwrap();
        assert_eq!(
            velocity(&battle::get(h).velocity),
            bolt_velocity(0, 0, random),
            "appearing ship can target cloaked opponents"
        );
        battle_context::dispose(h);
        battle::get_mut(ship.id).state_flags &= !(APPEARING as u16);
        battle::get_mut(near.id).life = 0;
        let h = with_rng(313, 1, || fire_from(ship.id)).unwrap();
        assert_eq!(
            velocity(&battle::get(h).velocity),
            bolt_velocity(0, 0, random)
        );
        battle_context::dispose(h);
    })
}

#[test]
fn child_segments_keep_unrounded_travel_angle_inherit_color_and_retain_selected_target() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            let ship =
                Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
            let mut enemy =
                Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 1, [16; 3], [4, 4]);
            let mut cached =
                Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 1, [16; 3], [4, 4]);
            at(&mut cached, 6000, 4000);
            let h = fire_from(ship.id).unwrap();
            let parent = h;
            battle::get_mut(parent).current.location = Point { x: 4000, y: 4000 };
            battle::get_mut(parent).next.location = Point { x: 4100, y: 4200 };
            battle::get_mut(parent).turn = 4;
            battle::prim_mut(parent).color = Color {
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
                battle::get_mut(parent).velocity.travel_angle = angle;
                battle::get_mut(parent).target = None;
                let (random, _) = randoms(77);
                let child_h =
                    with_rng(77, if delta == 8 { 2 } else { 1 }, || fire_from(parent)).unwrap();
                let child = child_h;
                assert_eq!(
                    velocity(&battle::get(child).velocity),
                    bolt_velocity(angle, delta, random),
                    "angle {angle}, delta {delta}"
                );
                assert_eq!(rgba(battle::prim(child).color), (3, 77, 129, 43));
                assert_eq!(battle::get(child).turn, 3);
                assert_eq!(battle::get(child).parent, battle::get(parent).parent);
                assert_eq!(
                    (
                        battle::get(child).current.location.x,
                        battle::get(child).current.location.y
                    ),
                    (4100, 4200)
                );
                assert_eq!(battle::get(parent).target, Some(enemy.id));
                battle_context::dispose(child_h);
            }
            // A child with a cached target uses that target even if a nearer ship exists.
            cached.descriptor_mut().ship_info.crew_level = 12;
            at(&mut enemy, 4000, 3999);
            battle::get_mut(parent).velocity.travel_angle = 1;
            battle::get_mut(parent).target = Some(cached.id);
            let (random, _) = randoms(99);
            let child_h = with_rng(99, 1, || fire_from(parent)).unwrap();
            assert_eq!(battle::get(parent).target, Some(cached.id));
            assert_eq!(
                velocity(&battle::get(child_h).velocity),
                bolt_velocity(1, 4, random)
            );
            battle_context::dispose(child_h);
            // A dead cached target is cleared without starting a fresh queue scan.
            cached.descriptor_mut().ship_info.crew_level = 0;
            battle::get_mut(parent).target = Some(cached.id);
            let child_h = with_rng(99, 1, || fire_from(parent)).unwrap();
            assert!(battle::get(parent).target.is_none());
            assert_eq!(
                velocity(&battle::get(child_h).velocity),
                bolt_velocity(1, -1, random)
            );
            battle_context::dispose(child_h);
            enemy.descriptor_mut().ship_info.crew_level = 0;
            battle::get_mut(parent).turn = 0;
            let child_h = with_rng(99, 1, || fire_from(parent)).unwrap();
            assert_eq!(battle::get(child_h).turn, 255);
            assert_eq!(
                velocity(&battle::get(child_h).velocity),
                bolt_velocity(1, -1, random)
            );
            battle_context::dispose(child_h);
            battle_context::dispose(h);
        }
    })
}

#[test]
fn lightning_postprocess_extends_chains_until_zero_or_collision() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        let mut ship =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        ship.ship_mut().weapon_counter = 8;
        let h = fire_from(ship.id).unwrap();
        battle::put(h);
        let mut parent = h;
        for wait in (0..8).rev() {
            battle::get_mut(parent).next = battle::get(parent).current.clone();
            let before = queue_len();
            with_rng(56, 1, || {
                hooks::invoke_element(battle::get(parent).postprocess_func.unwrap(), parent)
            });
            assert_eq!(queue_len(), before + 1);
            let child = battle::tail().unwrap();
            assert_ne!(child, parent);
            assert_eq!(battle::get(child).turn, wait);
            assert_eq!(
                rgba(battle::prim(child).color),
                rgba(battle::prim(parent).color)
            );
            parent = child;
        }
        let before = queue_len();
        with_rng(56, 0, || {
            hooks::invoke_element(battle::get(parent).postprocess_func.unwrap(), parent)
        });
        assert_eq!(queue_len(), before);
        battle::get_mut(parent).turn = 5;
        battle::get_mut(parent).state_flags |= COLLISION as u16;
        with_rng(56, 0, || {
            hooks::invoke_element(battle::get(parent).postprocess_func.unwrap(), parent)
        });
        assert_eq!(queue_len(), before);
    })
}

#[test]
fn allocation_failure_consumes_no_lightning_random_and_keeps_initializer_count() {
    battle_context::run(|_| {
        let _battle = Battle::new(2);
        let mut ship =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        ship.ship_mut().weapon_counter = 8;
        let h = fire_from(ship.id).unwrap();
        battle::put(h);
        let parent = h;
        assert!(with_rng(7, 0, || fire_from(ship.id)).is_none());
        with_rng(7, 0, || {
            hooks::invoke_element(battle::get(parent).postprocess_func.unwrap(), parent)
        });
        assert_eq!({ battle::tail() }, Some(h));
        with_rng(7, 0, || ship.postprocess());
        assert_eq!({ battle::tail() }, Some(h));
    })
}

#[test]
fn lightning_collision_folds_and_subtracts_counter_then_calls_generic_weapon_collision() {
    battle_context::run(|_| {
        let _battle = Battle::new(32);
        let mut ship =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        let h = fire_from(ship.id).unwrap();
        let beam = h;
        let collide = battle::get(beam).collision_func.unwrap();
        let mut pa = Point { x: 10, y: 20 };
        let mut pb = Point { x: 30, y: 40 };
        let target = PoolElement::new();
        battle::get_mut(target.id).life = 2;
        battle::get_mut(target.id).hits = 3;
        battle::get_mut(target.id).mass = 2;
        battle::get_mut(target.id).state_flags = FINITE_LIFE as u16;
        with_rng(13, 0, || {
            hooks::invoke_collision(collide, beam, &mut pa, target.id, &mut pb)
        });
        assert_eq!(
            battle::get(target.id).hits,
            2,
            "generic weapon damage applied"
        );
        assert_ne!(
            battle::get(beam).state_flags & (COLLISION | NONSOLID) as u16,
            0
        );
        assert_eq!(battle::get(beam).hits, 0);
        assert_eq!(battle::get(beam).life, 0);
        assert_eq!(
            battle::get(beam).state_flags & DISAPPEARING as u16,
            0,
            "line prims do not disappear"
        );
        let tail = battle::tail();
        for counter in [0u8, 1, 8, 9, 16, 17, 255] {
            for wait in [0u8, 1, 8, 255] {
                ship.ship_mut().weapon_counter = counter;
                battle::get_mut(beam).turn = wait;
                let folded = if counter > 8 {
                    17u8.wrapping_sub(counter)
                } else {
                    counter
                };
                with_rng(13, 0, || {
                    hooks::invoke_collision(collide, beam, &mut pa, target.id, &mut pb)
                });
                assert_eq!(ship.ship_mut().weapon_counter, folded.wrapping_sub(wait));
                assert_eq!(battle::get(beam).turn, 0);
                assert_eq!(
                    { battle::tail() },
                    tail,
                    "generic collision ignores an already-collided beam"
                );
            }
        }
        battle_context::dispose(h);
    })
}

#[test]
fn ship_postprocess_emits_only_during_the_sixteen_active_weapon_ticks() {
    battle_context::run(|_| {
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
                assert_eq!(
                    battle::get(battle::tail().unwrap()).parent,
                    battle::get(ship.id).parent
                );
            }
            assert_eq!(ship.ship_mut().weapon_counter, counter);
        }
    })
}

#[test]
fn movement_reverses_on_thrust_press_turns_with_left_priority_and_animates_next_frame() {
    battle_context::run(|_| {
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
            ship.ship_mut().ship_facing = initial;
            ship.ship_mut().old_status_flags = old as u16;
            ship.ship_mut().cur_status_flags = (current | SHIP_IN_GRAVITY_WELL) as u16;
            battle::get_mut(ship.id).state_flags = PLAYER_SHIP as u16;
            battle::get_mut(ship.id).turn = turn_wait;
            battle::get_mut(ship.id).thrust = thrust_wait;
            battle::get_mut(ship.id).velocity = VelocityDesc::new();
            battle::get_mut(ship.id).current.image.frame = ship.frame(0, 3);
            battle::get_mut(ship.id).next.image.frame = ship.frame(0, 15);
            with_rng(64, 0, || ship.preprocess());
            assert_eq!(ship.ship_mut().ship_facing, facing);
            assert_eq!(
                battle::get(ship.id).turn,
                if turn_wait == 0 { 1 } else { turn_wait }
            );
            assert_eq!(
                battle::get(ship.id).thrust,
                if thrust_wait == 0 { 1 } else { thrust_wait }
            );
            assert_eq!((battle::get(ship.id).next.image.frame.raw()).index(), 0);
            assert_ne!(battle::get(ship.id).state_flags & CHANGING as u16, 0);
            if thrust_wait == 0 {
                assert_eq!(velocity(&battle::get(ship.id).velocity), vector(60, facing));
                assert_ne!(
                    ship.ship_mut().cur_status_flags & SHIP_AT_MAX_SPEED as u16,
                    0
                );
                assert_eq!(
                    ship.ship_mut().cur_status_flags & SHIP_IN_GRAVITY_WELL as u16,
                    0
                );
            } else {
                assert_eq!(velocity(&battle::get(ship.id).velocity), [0; 9]);
                assert_ne!(
                    ship.ship_mut().cur_status_flags & SHIP_IN_GRAVITY_WELL as u16,
                    0
                );
            }
        }
        ship.descriptor_mut().characteristics.turn_wait = 255;
        ship.descriptor_mut().characteristics.thrust_wait = 255;
        battle::get_mut(ship.id).turn = 0;
        battle::get_mut(ship.id).thrust = 0;
        ship.preprocess();
        assert_eq!(battle::get(ship.id).turn, 0);
        assert_eq!(battle::get(ship.id).thrust, 0);
        for flags in [APPEARING, NONSOLID, APPEARING | NONSOLID] {
            battle::get_mut(ship.id).state_flags = flags as u16;
            let facing = ship.ship_mut().ship_facing;
            let frame = battle::get(ship.id).next.image.frame.raw();
            let v = velocity(&battle::get(ship.id).velocity);
            with_rng(64, 0, || ship.preprocess());
            assert_eq!(ship.ship_mut().ship_facing, facing);
            assert_eq!(battle::get(ship.id).next.image.frame.raw(), frame);
            assert_eq!(velocity(&battle::get(ship.id).velocity), v);
            assert_eq!(battle::get(ship.id).state_flags, flags as u16);
        }
    })
}

#[test]
fn harvester_filters_all_junk_flags_collision_rules_and_inclusive_distance_boundaries() {
    battle_context::run(|_| {
        let _battle = Battle::new(64);
        let mut ship =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        ship.ship_mut().cur_status_flags = SPECIAL as u16;
        battle::get_mut(ship.id).state_flags |= COLLISION as u16;
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
            let fixture = junk(&ship, dx, dy, flags as u16, mass);
            objects.push((fixture.id, harvest));
            fixtures.push(fixture);
        }
        let owned_fixture = junk(&ship, 0, 0, 0, 1);
        let owned = owned_fixture.id;
        battle::get_mut(owned).player_nr = 0;
        objects.push((owned, false));
        let foreign_fixture = junk(&ship, 0, 0, 0, 1);
        let foreign = foreign_fixture.id;
        battle::get_mut(foreign).player_nr = 1;
        objects.push((foreign, false));
        ship.descriptor_mut().ship_info.energy_level = 3;
        with_rng(11, 0, || ship.postprocess());
        for (e, harvest) in objects {
            assert_eq!(battle::get(e).life, if harvest { 0 } else { 50 });
            if harvest {
                assert_ne!(battle::get(e).state_flags & NONSOLID as u16, 0);
            }
            assert_eq!(
                battle::get(e).hits,
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
    battle_context::run(|_| {
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
            battle::get_mut(ship.id).state_flags |= ship_flags as u16;
            battle::get_mut(ship.id).mass = ship_mass;
            let e_fixture = junk(&ship, 0, 0, obj_flags as u16, obj_mass);
            let e = e_fixture.id;
            if same_parent {
                battle::get_mut(e).parent = battle::get(ship.id).parent;
            }
            with_rng(2, 0, || ship.postprocess());
            assert_eq!(battle::get(e).life == 0, harvest);
            assert_eq!(
                ship.ship_mut().special_counter,
                if harvest { 20 } else { 0 }
            );
        }
    })
}

#[test]
fn harvester_requires_key_and_ready_counter_but_can_run_at_full_energy() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            let mut ship =
                Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
            for (key, counter, energy, harvest) in [
                (false, 0, 0, false),
                (true, 1, 0, false),
                (true, 0, 20, true),
            ] {
                let e_fixture = junk(&ship, 0, 0, 0, 1);
                let e = e_fixture.id;
                ship.ship_mut().cur_status_flags = if key { SPECIAL as u16 } else { 0 };
                ship.ship_mut().special_counter = counter;
                ship.descriptor_mut().ship_info.energy_level = energy;
                observation::enable();
                observation::drain();
                with_rng(6, 0, || ship.postprocess());
                assert_eq!(
                    special_hooks::count(&observation::drain(), "slylandro"),
                    usize::from(harvest)
                );
                assert_eq!(battle::get(e).life == 0, harvest);
                assert_eq!(
                    ship.ship_mut().special_counter,
                    if harvest { 20 } else { counter }
                );
                assert_eq!(
                    ship.descriptor_mut().ship_info.energy_level,
                    if harvest { 20 } else { energy }
                );
                // Exclude this object from later cases.
                battle::get_mut(e).state_flags |= NONSOLID as u16;
            }
            ship.ship_mut().special_counter = 0;
            ship.descriptor_mut().ship_info.energy_level = 0;
            ship.postprocess();
            assert_eq!(special_hooks::count(&observation::drain(), "slylandro"), 0);
            assert_eq!(ship.ship_mut().special_counter, 0);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 0);
            at(&mut ship, -32760, 4000);
            let e_fixture = junk(&ship, 0, 0, 0, 1);
            let e = e_fixture.id;
            battle::get_mut(e).next.location.x = 32760;
            ship.postprocess();
            assert_eq!(
                battle::get(e).life,
                0,
                "subtraction truncates to i16 before absolute value"
            );
        }
    })
}

#[test]
fn ai_encounter_weapon_range_recharge_range_and_close_pursuit_boundaries() {
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        let mut ship =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        let enemy = Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 1, [16; 3], [4, 4]);
        let mut concerns: [Concern; 5] = [Concern::default(); 5];
        concerns[0].object = Some(enemy.id);
        concerns[2].object = Some(enemy.id);
        concerns[4].object = Some(enemy.id);
        for (activity, turns, energy, special_turns, movement, expected_move) in [
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
            crate::battle_context::with(|cx| cx.world.set_activity(activity));
            concerns[0].which_turn = turns;
            concerns[0].movement = movement;
            concerns[2].object = Some(enemy.id);
            concerns[4].which_turn = special_turns;
            ship.descriptor_mut().ship_info.energy_level = energy;
            ship.ship_mut().ship_input_state = (WEAPON | SPECIAL) as u8;
            battle::get_mut(ship.id).thrust = 255;
            with_rng(17, 0, || think(&mut ship, &mut concerns, 0));
            assert_eq!(concerns[0].movement, expected_move);
            assert_eq!(
                ship.ship_mut().ship_input_state & WEAPON as u8 != 0,
                turns <= 14
            );
            assert_eq!(
                ship.ship_mut().ship_input_state & SPECIAL as u8 != 0,
                energy < 20 && special_turns <= 14
            );
            assert_eq!(
                concerns[2].object.is_none(),
                activity as u8 == IN_ENCOUNTER as u8
            );
            assert_eq!(battle::get(ship.id).thrust, 255);
        }
        concerns[0].object = None;
        concerns[4].object = None;
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
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        let mut ship =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        let e_fixture = junk(&ship, 1000, 0, 0, 1);
        let e = e_fixture.id;
        let mut concerns: [Concern; 5] = [Concern::default(); 5];
        concerns[4].object = Some(e);
        concerns[4].which_turn = 10;
        concerns[4].movement = MOVEMENT_STATE_PURSUE;
        for (energy, cooldown, gravity, turns) in [
            (0, 0, false, true),
            (1, 0, false, false),
            (0, 1, false, false),
            (0, 0, true, false),
        ] {
            ship.descriptor_mut().ship_info.energy_level = energy;
            ship.ship_mut().special_counter = cooldown;
            ship.ship_mut().ship_input_state = 0;
            ship.ship_mut().ship_facing = 0;
            battle::get_mut(ship.id).turn = 0;
            battle::get_mut(ship.id).thrust = 0;
            concerns[3].object = if gravity { Some(e) } else { None };
            with_rng(8, 0, || think(&mut ship, &mut concerns, 0));
            assert_eq!(ship.ship_mut().ship_input_state & RIGHT as u8 != 0, turns);
            assert_eq!(ship.ship_mut().ship_facing, if turns { 1 } else { 0 });
            assert_eq!(ship.ship_mut().ship_input_state & THRUST as u8, 0);
            assert_eq!(battle::get(ship.id).thrust, 0);
            assert_eq!(
                ship.ship_mut().ship_input_state & SPECIAL as u8,
                SPECIAL as u8
            );
        }
    })
}

#[test]
fn native_ai_preserves_fifth_concern_and_expands_active_count() {
    battle_context::run(|_| {
        use uqm_engine::battle::{Concern, Concerns};
        let battle = Battle::new(8);
        let mut ship =
            Ship::with_frames(&battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        let debris = junk(&ship, 1000, 0, 0, 1);
        let debris = debris.id;
        ship.descriptor_mut().ship_info.energy_level = 0;
        ship.ship_mut().special_counter = 0;
        with_rng(8, 0, || {
            crate::battle_context::with(|cx| {
                let junk_id = debris;
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
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        let mut ship =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 0, [16; 3], [4, 4]);
        let mut enemy =
            Ship::with_frames(&_battle._pool, SPECIES_ID_SLYLANDRO_ID, 1, [16; 3], [4, 4]);
        at(&mut enemy, 5000, 4000);
        let mut concerns: [Concern; 5] = [Concern::default(); 5];
        concerns[0].object = Some(enemy.id);
        concerns[0].movement = MOVEMENT_STATE_PURSUE;
        concerns[0].which_turn = 10;
        ship.descriptor_mut().ship_info.energy_level = 1;
        let before = velocity(&battle::get(ship.id).velocity);
        with_rng(71, 0, || think(&mut ship, &mut concerns, 1));
        assert_eq!(ship.ship_mut().ship_facing, 1);
        assert_ne!(ship.ship_mut().ship_input_state & RIGHT as u8, 0);
        assert_eq!(ship.ship_mut().ship_input_state & THRUST as u8, 0);
        assert_eq!(battle::get(ship.id).thrust, 0);
        assert_eq!(velocity(&battle::get(ship.id).velocity), before);
    })
}

fn at(ship: &mut Ship<'_>, x: i16, y: i16) {
    ship.element_mut().current.location = Point { x, y };
    ship.element_mut().next.location = Point { x, y };
}
fn fire_from(source: ElementId) -> Option<ElementId> {
    crate::battle_context::with(|cx| uqm_content::ships::races::lightning_segment(cx, source))
}
fn think(ship: &mut Ship<'_>, concerns: &mut [Concern; 5], count: u16) {
    ship.think(ship.id, concerns, count);
}

#[path = "support/hooks.rs"]
mod hooks;
