//! Exercise Orz's native callbacks with the linked engine queue and physics.

use uqm_rust::engine::battle::ai_constants::ENEMY_SHIP_INDEX;
use uqm_rust::engine::battle::ai_constants::FIRES_FORE;
use uqm_rust::engine::battle::ai_constants::IGNORE_SIMILAR;
use uqm_rust::engine::battle::ai_constants::LEFT;
use uqm_rust::engine::battle::ai_constants::MOVEMENT_STATE_ENTICE;
use uqm_rust::engine::battle::ai_constants::POINT_DEFENSE;
use uqm_rust::engine::battle::ai_constants::RIGHT;
use uqm_rust::engine::battle::ai_constants::SEEKING_SPECIAL;
use uqm_rust::engine::battle::ai_constants::SPECIAL;
use uqm_rust::engine::battle::ai_constants::THRUST;
use uqm_rust::engine::battle::ai_constants::WEAPON;
use uqm_rust::engine::battle::ElementId;
use uqm_rust::math::seed_random;
use uqm_rust::ships::races::{intruder_preprocess, marine_collision};
#[path = "support/battle_context.rs"]
mod battle_context;

#[path = "support/battle.rs"]
mod battle;
use battle::{Battle, Ship};
use uqm_rust::engine::battle::flags::{
    APPEARING, CHANGING, COLLISION, CREW_OBJECT, DISAPPEARING, FINITE_LIFE, NEUTRAL_PLAYER_NUM,
    NONSOLID, PLAYER_SHIP, POINT_PRIMITIVE, POST_PROCESS, PRE_PROCESS, SHIP_AT_MAX_SPEED,
    SHIP_BEYOND_MAX_SPEED, SHIP_IN_GRAVITY_WELL, STAMPFILL_PRIMITIVE,
};
use uqm_rust::engine::battle::Concern;
use uqm_rust::engine::battle::{CollisionFn, ElementFn};
use uqm_rust::engine::battle::{EMPTY_PRIMITIVE, STAMP_PRIMITIVE};
use uqm_rust::engine::species::species_id::SPECIES_ID_ORZ_ID;
use uqm_rust::graphics::geometry::Color;
use uqm_rust::graphics::geometry::Point;
use uqm_rust::math::velocity::{VelocityDesc, VELOCITY_SHIFT};
use uqm_rust::math::{arctan, random};
use uqm_rust::ships::native::{angle_to_facing, cosine, facing_to_angle, normalize_facing, sine};

fn orz(battle: &Battle, player: i16) -> Ship<'_> {
    let mut ship = Ship::with_frames(battle, SPECIES_ID_ORZ_ID, player, [16, 16, 32], [4, 4]);
    ship.descriptor_mut().ship_info.icons = ship.raw_frame(0, 0);
    ship
}

trait OrzFixture {
    fn at(&mut self, x: i16, y: i16);
    fn install_turret(&mut self) -> ElementId;
    fn tick_turret(&mut self) -> ElementId;
    fn launch(&mut self) -> ElementId;
    fn missile(&mut self) -> ElementId;
}
impl OrzFixture for Ship<'_> {
    fn at(&mut self, x: i16, y: i16) {
        battle::get_mut(self.id).current.location = Point { x, y };
        battle::get_mut(self.id).next.location = Point { x, y };
    }

    fn install_turret(&mut self) -> ElementId {
        battle::get_mut(self.id).state_flags |= APPEARING as u16;
        self.preprocess();
        battle::get_mut(self.id).state_flags &= !(APPEARING as u16);
        let turret = battle::succ(self.id).unwrap();
        assert_eq!(
            battle::get(turret).parent,
            uqm_rust::engine::battle::Parent::Ship(self.ship_id)
        );
        turret
    }

    fn tick_turret(&mut self) -> ElementId {
        let old = battle::succ(self.id).unwrap();
        let e = old;
        battle::get_mut(e).life = 0;
        hooks::invoke_element(battle::get(e).postprocess_func.unwrap(), e);
        battle_context::unlink(old);
        battle_context::dispose(old);
        battle::succ(self.id).unwrap()
    }

    fn launch(&mut self) -> ElementId {
        self.ship_mut().cur_status_flags = (SPECIAL | WEAPON) as u16;
        self.ship_mut().special_counter = 0;
        let tail = battle::tail().unwrap();
        // Earlier objects have completed their appearing frame before this shot.
        battle::get_mut(tail).state_flags &= !(APPEARING as u16);
        self.tick_turret();
        assert_ne!(battle::tail(), Some(tail));
        let marine = battle::tail().unwrap();
        assert_eq!(
            battle::get(marine).collision_func.unwrap().address(),
            marine_collision as *const () as usize
        );
        battle::get_mut(marine).next = battle::get(marine).current.clone();
        marine
    }

    fn missile(&mut self) -> ElementId {
        let out = self.weapon_handles(1)[0].unwrap();
        let e = out;
        assert!(battle::get(e).parent == uqm_rust::engine::battle::Parent::None);
        battle::get_mut(e).parent = uqm_rust::engine::battle::Parent::Ship(self.ship_id);
        battle::get_mut(e).next = battle::get(e).current.clone();
        out
    }
}

fn queued() -> Vec<ElementId> {
    let mut result = Vec::new();
    let mut cursor = battle::head();
    let mut previous = None;
    while let Some(h) = cursor {
        assert_eq!(battle::pred(h), previous);
        result.push(h);
        assert!(result.len() <= 255, "display queue cycle");
        previous = Some(h);
        cursor = battle::succ(h);
    }
    assert_eq!(previous, { battle::tail() });
    result
}

fn with_rng<T>(seed: u32, draws: usize, run: impl FnOnce() -> T) -> T {
    seed_random(seed);
    for _ in 0..draws {
        random();
    }
    let expected = random();
    seed_random(seed);
    let result = run();
    assert_eq!(random(), expected, "RNG draws {draws}, seed {seed}");
    result
}

fn seed_for_byte(byte: u8) -> u32 {
    (1..100_000)
        .find(|&seed| {
            seed_random(seed);
            random() as u8 == byte
        })
        .expect("seed for each intruder outcome boundary")
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

fn velocity_toward(speed: i16, facing: u16) -> [i32; 9] {
    let mut v = VelocityDesc::new();
    uqm_rust::battle::velocity::set_velocity_vector(&mut v, speed, facing);
    velocity_fields(&v)
}

fn collide(marine: ElementId, target: ElementId) {
    crate::battle_context::with(|cx| {
        let a = marine;
        let b = target;
        marine_collision(
            cx,
            a,
            &mut Point { x: 0, y: 0 },
            b,
            &mut Point { x: 0, y: 0 },
        );
    });
}

fn intruder(e: ElementId) {
    crate::battle_context::with(|cx| {
        let id = e;
        intruder_preprocess(cx, id);
    });
}

fn intelligence(ship: &mut Ship<'_>, concerns: &mut [Concern; 4]) {
    // Isolate Orz's decisions after the real shared pass, with no movement concerns.
    ship.think(ship.id, concerns, 0);
}

#[test]
fn descriptor_preserves_c_resources_characteristics_and_callback_abi() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let d = battle::descriptor(SPECIES_ID_ORZ_ID);
            let race = d.race.unwrap();
            assert_eq!(
                d.ship_info.ship_flags,
                (FIRES_FORE | SEEKING_SPECIAL) as u16
            );
            assert_eq!(d.ship_info.ship_cost, 23);
            assert_eq!((d.ship_info.crew_level, d.ship_info.max_crew), (16, 16));
            assert_eq!((d.ship_info.energy_level, d.ship_info.max_energy), (20, 20));
            assert_eq!(d.ship_info.race_strings_rsc.unwrap(), c"ship.orz.text");
            assert_eq!(d.ship_info.icons_rsc.unwrap(), c"ship.orz.icons");
            assert_eq!(d.ship_info.melee_icon_rsc.unwrap(), c"ship.orz.meleeicons");
            assert!(d.ship_info.race_strings.is_none());
            assert!(d.ship_info.icons.is_null());
            assert!(d.ship_info.melee_icon.is_null());
            assert_eq!(d.fleet.strength, 60);
            assert_eq!((d.fleet.known_loc.x, d.fleet.known_loc.y), (3608, 2637));
            let c = d.characteristics;
            assert_eq!(
                (c.max_thrust, c.thrust_increment, c.energy_regeneration),
                (35, 5, 1)
            );
            assert_eq!(
                (c.weapon_energy_cost, c.special_energy_cost, c.energy_wait),
                (6, 0, 6)
            );
            assert_eq!(
                (
                    c.turn_wait,
                    c.thrust_wait,
                    c.weapon_wait,
                    c.special_wait,
                    c.ship_mass
                ),
                (1, 0, 4, 12, 4)
            );
            for (i, size) in ["large", "medium", "small"].iter().enumerate() {
                assert_eq!(
                    d.ship_data.ship_rsc[i].unwrap().to_str().unwrap(),
                    format!("ship.orz.graphics.nemesis.{size}")
                );
                assert_eq!(
                    d.ship_data.weapon_rsc[i].unwrap().to_str().unwrap(),
                    format!("ship.orz.graphics.howitzer.{size}")
                );
                assert_eq!(
                    d.ship_data.special_rsc[i].unwrap().to_str().unwrap(),
                    format!("ship.orz.graphics.turret.{size}")
                );
                assert!(d.ship_data.ship.view(i).is_absent());
                assert!(d.ship_data.weapon.view(i).is_absent());
                assert!(d.ship_data.special.view(i).is_absent());
            }
            assert_eq!(
                d.ship_data.captain_control.captain_rsc.unwrap(),
                c"ship.orz.graphics.captain"
            );
            assert_eq!(d.ship_data.victory_ditty_rsc.unwrap(), c"ship.orz.ditty");
            assert_eq!(d.ship_data.ship_sounds_rsc.unwrap(), c"ship.orz.sounds");
            assert_eq!(d.cyborg_control.weapon_range, 1440);
            assert_eq!(d.cyborg_control.maneuverability_index, 0);
            assert!(race.preprocess.is_some());
            assert!(race.init_weapon.is_some());
            assert!(race.intelligence.is_some());
            assert!(race.postprocess.is_none());
            assert!(race.uninit.is_none());
            let _: ElementFn = intruder_preprocess;
            let _: CollisionFn = marine_collision;
            let mut changed = battle::descriptor(SPECIES_ID_ORZ_ID);
            changed.characteristics.turn_wait = 99;
            changed.ship_info.crew_level = 0;
            assert_eq!(changed.characteristics.turn_wait, 99);
            assert_eq!(changed.ship_info.crew_level, 0);
            let again = battle::descriptor(SPECIES_ID_ORZ_ID);
            assert_eq!(again.characteristics.turn_wait, 1);
            assert_eq!(again.ship_info.crew_level, 16);
            assert_eq!(again.callbacks(), d.callbacks());
        }
    })
}

#[test]
fn appearing_turret_is_inserted_immediately_after_ship_and_inputs_suppress_hull_and_gun() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            let enemy = orz(&_battle, 1);
            ship.ship_mut().ship_facing = 7;
            let turret = with_rng(2, 0, || ship.install_turret());
            assert_eq!(queued(), vec![ship.id, turret, enemy.id]);
            assert_eq!(
                battle::get(turret).state_flags,
                (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR) as u16
            );
            assert_eq!(battle::get(turret).life, 1);
            assert_eq!(
                battle::get(turret).current.image.frame.raw(),
                ship.raw_frame(2, 7)
            );
            assert_eq!(battle::get(turret).current.image.frames, ship.frame_set(2));
            assert!(battle::get(turret).postprocess_func.is_some());
            assert!(battle::get(turret).preprocess_func.is_none());
            assert!(battle::get(turret).collision_func.is_none());
            for (current, old, turn, weapon, expected_turn, expected_weapon) in [
                (SPECIAL | RIGHT | WEAPON, 0, 0, 0, 1, 1),
                (LEFT, SPECIAL, 0, 0, 1, 0),
                (SPECIAL | LEFT | WEAPON, 0, 2, 9, 2, 9),
                (SPECIAL | WEAPON, 0, 0, 0, 0, 1),
                (RIGHT | WEAPON, 0, 0, 0, 0, 0),
            ] {
                ship.ship_mut().cur_status_flags = current as u16;
                ship.ship_mut().old_status_flags = old as u16;
                ship.ship_mut().weapon_counter = weapon;
                battle::get_mut(ship.id).turn = turn;
                with_rng(2, 0, || ship.preprocess());
                assert_eq!(battle::get(ship.id).turn, expected_turn);
                assert_eq!(ship.ship_mut().weapon_counter, expected_weapon);
            }
        }
    })
}

#[test]
fn howitzer_rotates_before_firing_with_byte_wrap_and_exact_missile_fields() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 1);
            let turret = ship.install_turret();
            ship.at(i16::MAX - 10, i16::MIN + 10);
            battle::get_mut(ship.id).next.location = Point { x: -1234, y: 2345 };
            for (status, offset, wait, expected_offset, expected_wait) in [
                (SPECIAL | RIGHT, 255, 0, 0, 4),
                (SPECIAL | LEFT, 0, 0, 255, 4),
                (SPECIAL | LEFT | RIGHT, 3, 0, 4, 4),
                (SPECIAL | RIGHT, 9, 1, 9, 1),
                (RIGHT, 9, 0, 9, 0),
            ] {
                ship.ship_mut().ship_facing = 15;
                ship.ship_mut().cur_status_flags = status as u16;
                battle::get_mut(turret).turn = wait;
                battle::get_mut(turret).thrust = offset;
                let h = with_rng(11, 0, || ship.missile());
                let missile = h;
                let face = normalize_facing(15 + i32::from(expected_offset));
                assert_eq!(battle::get(turret).turn, expected_wait);
                assert_eq!(battle::get(turret).thrust, expected_offset);
                assert_eq!(battle::get(missile).life, 12);
                assert_eq!(battle::get(missile).hits, 2);
                assert_eq!(battle::get(missile).mass, 3);
                assert_eq!(battle::get(missile).thrust, 1);
                assert_eq!(battle::get(missile).player_nr, 1);
                assert_eq!(
                    battle::get(missile).state_flags,
                    (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
                );
                assert_eq!(
                    battle::get(missile).current.image.frame.raw(),
                    ship.raw_frame(1, face as usize)
                );
                assert_eq!(battle::get(missile).current.image.frames, ship.frame_set(1));
                assert_eq!(
                    velocity_fields(&battle::get(missile).velocity),
                    velocity_toward(120, face)
                );
                let a = facing_to_angle(i32::from(face));
                let dx = i32::from(cosine(a, 120 << VELOCITY_SHIFT)) >> VELOCITY_SHIFT;
                let dy = i32::from(sine(a, 120 << VELOCITY_SHIFT)) >> VELOCITY_SHIFT;
                assert_eq!(
                    battle::get(missile).current.location.x,
                    (-1234 + i32::from(cosine(a, 56)) - dx) as i16
                );
                assert_eq!(
                    battle::get(missile).current.location.y,
                    (2345 + i32::from(sine(a, 56)) - dy) as i16
                );
                assert!(battle::get(missile).preprocess_func.is_none());
                assert!(battle::get(missile).postprocess_func.is_none());
                battle_context::dispose(h);
            }
        }
    })
}

#[test]
fn howitzer_ignores_friendlies_but_damages_enemies_through_engine_collision() {
    battle_context::run(|_| {
        let _battle = Battle::new(4);
        {
            let mut ship = orz(&_battle, 0);
            ship.install_turret();
            let mut enemy = orz(&_battle, 1);
            let h = ship.missile();
            let missile = h;
            let collision = battle::get(missile).collision_func.unwrap();
            let mut point = Point { x: 10, y: 20 };
            with_rng(17, 0, || {
                hooks::invoke_collision(
                    collision,
                    missile,
                    &mut point,
                    ship.id,
                    &mut Point { x: 0, y: 0 },
                )
            });
            assert_eq!(battle::get(ship.id).hits, 16);
            assert_eq!(battle::get(missile).life, 12);
            with_rng(17, 0, || {
                hooks::invoke_collision(
                    collision,
                    missile,
                    &mut point,
                    enemy.id,
                    &mut Point { x: 0, y: 0 },
                )
            });
            assert_eq!(battle::get(enemy.id).hits, 13);
            assert_eq!(enemy.descriptor_mut().ship_info.crew_level, 13);
            assert_eq!(battle::get(missile).life, 0);
            assert_eq!(battle::get(missile).hits, 0);
            assert_ne!(
                battle::get(missile).state_flags & (COLLISION | NONSOLID | DISAPPEARING) as u16,
                0
            );
            battle_context::dispose(h);
        }
    })
}

#[test]
fn turret_replacement_decrements_wait_once_rotates_and_updates_firing_quadrant() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            ship.install_turret();
            ship.ship_mut().ship_facing = 15;
            ship.ship_mut().cur_status_flags = (SPECIAL | RIGHT) as u16;
            let tint = Color {
                r: 3,
                g: 7,
                b: 11,
                a: 99,
            };
            battle::prim_mut(ship.id).kind = STAMPFILL_PRIMITIVE;
            battle::prim_mut(ship.id).color = tint;
            battle::get_mut(ship.id).next.location = Point { x: 123, y: -456 };
            for (expected_offset, expected_wait) in [(1, 3), (1, 2), (1, 1), (1, 0), (2, 3)] {
                let turret = with_rng(5, 0, || ship.tick_turret());
                assert_eq!(battle::get(turret).thrust, expected_offset);
                assert_eq!(battle::get(turret).turn, expected_wait);
                assert_eq!(battle::get(turret).life, 1);
                assert_eq!(
                    battle::get(turret).state_flags,
                    (FINITE_LIFE
                        | NONSOLID
                        | IGNORE_SIMILAR
                        | CHANGING
                        | PRE_PROCESS
                        | POST_PROCESS) as u16
                );
                assert_eq!(
                    (
                        battle::get(turret).current.location.x,
                        battle::get(turret).current.location.y
                    ),
                    (123, -456)
                );
                assert_eq!(
                    battle::get(turret).next.image.frame.raw(),
                    battle::get(turret).current.image.frame.raw()
                );
                assert_eq!(
                    (battle::get(turret).current.image.frame.raw()).index(),
                    normalize_facing(15 + i32::from(expected_offset))
                );
                assert_eq!(battle::prim(turret).kind, STAMPFILL_PRIMITIVE);
                assert_eq!(
                    (
                        battle::prim(turret).color.r,
                        battle::prim(turret).color.g,
                        battle::prim(turret).color.b,
                        battle::prim(turret).color.a
                    ),
                    (3, 7, 11, 99)
                );
            }
            ship.ship_mut().cur_status_flags = 0;
            for face in 0..16 {
                ship.ship_mut().ship_facing = face;
                let old = battle::succ(ship.id).unwrap();
                battle::get_mut(old).thrust = 0;
                ship.tick_turret();
                let expected = (FIRES_FORE << (normalize_facing(i32::from(face) + 2) / 4)) as u16;
                assert_eq!(
                    ship.descriptor_mut().ship_info.ship_flags,
                    SEEKING_SPECIAL as u16 | expected
                );
            }
        }
    })
}

#[test]
fn muzzle_flash_requires_an_appearing_friendly_stamp_and_animates_from_current() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            ship.install_turret();
            ship.ship_mut().ship_facing = 4;
            ship.ship_mut().cur_status_flags = WEAPON as u16;
            let h = ship.missile();
            battle::put(h);
            let turret = with_rng(20, 0, || ship.tick_turret());
            let flash = battle::tail().unwrap();
            assert_ne!(flash, h);
            assert_eq!(battle::get(flash).life, 4);
            assert_eq!(
                battle::get(flash).current.image.frame.raw(),
                ship.raw_frame(2, 16)
            );
            assert_eq!(
                battle::get(flash).state_flags,
                (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR | APPEARING) as u16
            );
            assert_eq!(
                battle::get(flash).parent,
                uqm_rust::engine::battle::Parent::Ship(ship.ship_id)
            );
            let a = facing_to_angle(4);
            assert_eq!(
                battle::get(turret).current.location.x,
                (4000 - i32::from(cosine(a, 8))) as i16
            );
            assert_eq!(
                battle::get(turret).current.location.y,
                (4000 - i32::from(sine(a, 8))) as i16
            );
            assert_eq!(
                battle::get(flash).current.location.x,
                (i32::from(battle::get(turret).current.location.x) + i32::from(cosine(a, 56)))
                    as i16
            );
            assert_eq!(
                battle::get(flash).current.location.y,
                (i32::from(battle::get(turret).current.location.y) + i32::from(sine(a, 56))) as i16
            );
            let animate = battle::get(flash).preprocess_func.unwrap();
            battle::get_mut(flash).next.image.frame = ship.frame(2, 25);
            battle::get_mut(flash).thrust = 2;
            with_rng(20, 0, || hooks::invoke_element(animate, flash));
            assert_eq!(
                battle::get(flash).next.image.frame.raw(),
                ship.raw_frame(2, 17)
            );
            assert_eq!(battle::get(flash).turn, 2);
            hooks::invoke_element(animate, flash);
            assert_eq!(battle::get(flash).turn, 1);
            assert_eq!(
                battle::get(flash).next.image.frame.raw(),
                ship.raw_frame(2, 17)
            );
            battle::get_mut(flash).state_flags &= !(APPEARING as u16);
            let before = queued().len();
            ship.tick_turret();
            assert_eq!(queued().len(), before);
        }
    })
}

#[test]
fn marines_launch_at_rear_with_exact_state_and_cost_only_after_allocation() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            ship.install_turret();
            let mut enemy = orz(&_battle, 1);
            enemy.at(4500, 4000);
            for facing in [0, 4, 7, 15] {
                ship.ship_mut().ship_facing = facing;
                ship.at(i16::MAX - 10, i16::MIN + 10);
                let old_crew = ship.descriptor_mut().ship_info.crew_level;
                let marine = with_rng(50, 0, || ship.launch());
                assert_eq!(
                    battle::get(marine).state_flags,
                    (IGNORE_SIMILAR | APPEARING | CREW_OBJECT) as u16
                );
                assert_eq!(battle::get(marine).life, 1);
                assert_eq!(battle::get(marine).hits, 3);
                assert_eq!(battle::get(marine).mass, 1);
                assert_eq!(battle::get(marine).thrust, 0);
                assert_eq!(
                    battle::get(marine).turn,
                    (normalize_facing(i32::from(facing) + 8) << 4) as u8
                );
                assert_eq!(
                    battle::get(marine).current.image.frame.raw(),
                    ship.raw_frame(2, 20)
                );
                assert_eq!(battle::get(marine).current.image.frames, ship.frame_set(2));
                assert_eq!(
                    battle::get(marine).parent,
                    uqm_rust::engine::battle::Parent::Ship(ship.ship_id)
                );
                assert_eq!(battle::prim(marine).kind, STAMP_PRIMITIVE);
                assert_eq!(velocity_fields(&battle::get(marine).velocity), [0; 9]);
                assert!(battle::get(marine).preprocess_func.is_some());
                assert!(battle::get(marine).postprocess_func.is_none());
                assert!(battle::get(marine).death_func.is_none());
                let a = facing_to_angle(i32::from(facing));
                assert_eq!(
                    battle::get(marine).current.location.x,
                    (i32::from(i16::MAX) - 10 - i32::from(cosine(a, 56))) as i16
                );
                assert_eq!(
                    battle::get(marine).current.location.y,
                    (i32::from(i16::MIN) + 10 - i32::from(sine(a, 56))) as i16
                );
                assert_eq!(ship.descriptor_mut().ship_info.crew_level, old_crew - 1);
                assert_eq!(battle::get(ship.id).hits, old_crew - 1);
                assert_eq!(ship.ship_mut().special_counter, 12);
                assert_eq!(ship.descriptor_mut().ship_info.energy_level, 20);
                // Retain an aligned cached target so launch eligibility never ties.
                battle::get_mut(ship.id).target = Some(enemy.id);
            }
        }
    })
}

#[test]
fn marine_limit_counts_only_live_matching_nonfinite_nondisappearing_elements() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            ship.install_turret();
            let mut enemy = orz(&_battle, 1);
            enemy.at(4500, 4000);
            let mut marines = Vec::new();
            for _ in 0..8 {
                marines.push(ship.launch());
            }
            assert_eq!(ship.descriptor_mut().ship_info.crew_level, 8);
            ship.ship_mut().special_counter = 0;
            battle::get_mut(battle::tail().unwrap()).state_flags &= !(APPEARING as u16);
            let before = queued().len();
            with_rng(5, 0, || ship.tick_turret());
            assert_eq!(queued().len(), before);
            assert_eq!(ship.ship_mut().special_counter, 0);
            for (i, flags) in [(0, FINITE_LIFE), (1, DISAPPEARING)] {
                battle::get_mut(marines[i]).state_flags |= flags as u16;
                ship.launch();
            }
            battle::get_mut(marines[2]).life = 0;
            ship.launch();
            battle::get_mut(marines[3]).current.image.frames = enemy.frame_set(2);
            ship.launch();
            assert_eq!(ship.descriptor_mut().ship_info.crew_level, 4);
        }
    })
}

#[test]
fn launch_gates_require_both_keys_counter_zero_crew_and_a_trackable_enemy() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            ship.install_turret();
            let enemy = orz(&_battle, 1);
            for (status, counter, crew, cloaked) in [
                (SPECIAL, 0, 16, false),
                (WEAPON, 0, 16, false),
                (SPECIAL | WEAPON, 1, 16, false),
                (SPECIAL | WEAPON, 0, 1, false),
                (SPECIAL | WEAPON, 0, 16, true),
            ] {
                ship.ship_mut().cur_status_flags = status as u16;
                ship.ship_mut().special_counter = counter;
                ship.descriptor_mut().ship_info.crew_level = crew;
                battle::get_mut(ship.id).hits = crew;
                battle::get_mut(ship.id).target = None;
                battle::prim_mut(enemy.id).kind = if cloaked {
                    EMPTY_PRIMITIVE
                } else {
                    STAMP_PRIMITIVE
                };
                let before = queued().len();
                // No appearing tail stamp, so this cannot create a muzzle flash.
                battle::get_mut(enemy.id).state_flags &= !(APPEARING as u16);
                with_rng(3, 0, || ship.tick_turret());
                assert_eq!(queued().len(), before);
                assert_eq!(ship.descriptor_mut().ship_info.crew_level, crew);
                assert_eq!(ship.ship_mut().special_counter, counter);
            }
        }
    })
}

#[test]
fn boarding_claims_first_unused_slot_and_immediately_kills_one_enemy_crew_without_rng() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            ship.install_turret();
            let mut enemy = orz(&_battle, 1);
            enemy.at(4500, 4000);
            let mut marines = Vec::new();
            for slot in 0..3 {
                let marine = ship.launch();
                battle::get_mut(enemy.id).state_flags |= COLLISION as u16;
                with_rng(10, 0, || collide(marine, enemy.id));
                assert_eq!(battle::get(marine).turn, slot);
                assert_eq!(battle::get(marine).thrust, 12);
                assert_eq!(
                    battle::get(marine).next.image.frame.raw(),
                    ship.raw_frame(2, 22 + slot as usize)
                );
                assert_eq!(
                    battle::get(marine).preprocess_func.unwrap().address(),
                    intruder_preprocess as *const () as usize
                );
                assert_eq!(
                    battle::get(marine).state_flags & (NONSOLID | CREW_OBJECT | COLLISION) as u16,
                    NONSOLID as u16
                );
                assert_eq!(battle::prim(marine).kind, EMPTY_PRIMITIVE);
                assert_eq!(battle::get(enemy.id).state_flags & COLLISION as u16, 0);
                marines.push(marine);
            }
            assert_eq!(enemy.descriptor_mut().ship_info.crew_level, 13);
            battle::get_mut(marines[1]).life = 0;
            battle::get_mut(marines[1]).state_flags |= DISAPPEARING as u16;
            let marine = ship.launch();
            collide(marine, enemy.id);
            assert_eq!(battle::get(marine).turn, 1);
            assert_eq!(
                battle::get(marine).next.image.frame.raw(),
                ship.raw_frame(2, 23)
            );
            let crew = enemy.descriptor_mut().ship_info.crew_level;
            collide(marine, enemy.id);
            assert_eq!(
                enemy.descriptor_mut().ship_info.crew_level,
                crew,
                "nonsolid intruder cannot board twice"
            );
        }
    })
}

#[test]
fn marine_collision_handles_return_planets_nonships_and_last_enemy_crew() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            ship.install_turret();
            let mut enemy = orz(&_battle, 1);
            enemy.at(4500, 4000);
            let marine = ship.launch();
            with_rng(8, 0, || collide(marine, ship.id));
            assert_eq!(ship.descriptor_mut().ship_info.crew_level, 16);
            assert_eq!(battle::get(marine).life, 0);
            assert_eq!(battle::get(marine).hits, 0);
            assert_ne!(
                battle::get(marine).state_flags & (DISAPPEARING | NONSOLID | FINITE_LIFE) as u16,
                0
            );
            let marine = ship.launch();
            let object = battle::PoolElement::new();
            battle::get_mut(object.id).player_nr = 1;
            battle::get_mut(object.id).mass = 100;
            battle::get_mut(marine).turn = 0xa0;
            battle::get_mut(marine).thrust = 7;
            collide(marine, object.id);
            assert_eq!(battle::get(marine).turn, 0xa5);
            assert_eq!(battle::get(marine).thrust, 4);
            assert_ne!(battle::get(marine).state_flags & COLLISION as u16, 0);
            assert_eq!(battle::get(marine).life, 1);
            battle::get_mut(marine).state_flags &= !(COLLISION as u16);
            battle::get_mut(object.id).mass = 101;
            collide(marine, object.id);
            assert_eq!(battle::get(marine).life, 0);
            assert_eq!(battle::get(marine).hits, 0);
            assert_eq!(
                battle::get(marine).state_flags & (NONSOLID | FINITE_LIFE) as u16,
                (NONSOLID | FINITE_LIFE) as u16
            );
            let marine = ship.launch();
            enemy.descriptor_mut().ship_info.crew_level = 1;
            battle::get_mut(enemy.id).hits = 1;
            with_rng(8, 0, || collide(marine, enemy.id));
            assert_eq!(battle::get(enemy.id).hits, 0);
            assert_eq!(battle::get(enemy.id).life, 0);
            assert_eq!(battle::get(marine).state_flags & NONSOLID as u16, 0);
            assert_ne!(
                battle::get(marine).preprocess_func.unwrap().address(),
                intruder_preprocess as *const () as usize
            );
            assert_eq!(battle::get(marine).turn & 15, 5);
        }
    })
}

#[test]
fn marine_collision_ignores_finite_targets_dead_marines_and_in_progress_collisions() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            ship.install_turret();
            let mut enemy = orz(&_battle, 1);
            enemy.at(4500, 4000);
            let marine = ship.launch();
            for (life, marine_flags, target_flags) in [
                (0, IGNORE_SIMILAR | CREW_OBJECT, PLAYER_SHIP),
                (1, IGNORE_SIMILAR | CREW_OBJECT | NONSOLID, PLAYER_SHIP),
                (1, IGNORE_SIMILAR | CREW_OBJECT | COLLISION, PLAYER_SHIP),
                (1, IGNORE_SIMILAR | CREW_OBJECT, PLAYER_SHIP | FINITE_LIFE),
            ] {
                battle::get_mut(marine).life = life;
                battle::get_mut(marine).state_flags = marine_flags as u16;
                battle::get_mut(marine).turn = 0x90;
                battle::get_mut(enemy.id).state_flags = target_flags as u16;
                with_rng(5, 0, || collide(marine, enemy.id));
                assert_eq!(enemy.descriptor_mut().ship_info.crew_level, 16);
                assert_eq!(battle::get(marine).state_flags, marine_flags as u16);
                assert_eq!(battle::get(marine).turn, 0x90);
            }
        }
    })
}

#[test]
fn intruder_rng_byte_boundaries_distinguish_death_damage_and_no_action() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            ship.install_turret();
            let mut enemy = orz(&_battle, 1);
            enemy.at(4500, 4000);
            let marine = ship.launch();
            collide(marine, enemy.id);
            let intruder_frame = battle::get(marine).next.image.frame.raw();
            for byte in [0, 15, 16, 143, 144, 255] {
                battle::get_mut(marine).life = 1;
                battle::get_mut(marine).state_flags = (IGNORE_SIMILAR | NONSOLID) as u16;
                battle::get_mut(marine).thrust = 1;
                enemy.descriptor_mut().ship_info.crew_level = 10;
                battle::get_mut(enemy.id).hits = 10;
                battle::get_mut(enemy.id).next.location = Point { x: -123, y: 456 };
                let seed = seed_for_byte(byte);
                with_rng(seed, 1, || intruder(marine));
                assert_eq!(battle::get(marine).life, u16::from(byte >= 16));
                assert_eq!(
                    battle::get(marine).state_flags & DISAPPEARING as u16 != 0,
                    byte < 16
                );
                let damaged = (16..144).contains(&byte);
                assert_eq!(
                    enemy.descriptor_mut().ship_info.crew_level,
                    if damaged { 9 } else { 10 }
                );
                assert_eq!(battle::get(marine).thrust, if damaged { 13 } else { 12 });
                assert_eq!(
                    battle::get(marine).next.image.frame.raw(),
                    intruder_frame,
                    "silhouette changes do not replace marine identity frame"
                );
                assert_eq!(
                    (
                        battle::get(marine).next.location.x,
                        battle::get(marine).next.location.y
                    ),
                    (-123, 456)
                );
                assert_eq!(
                    battle::get(marine).preprocess_func.unwrap().address(),
                    intruder_preprocess as *const () as usize
                );
            }
        }
    })
}

#[test]
fn intruder_wait_resets_attack_silhouette_without_rng_then_counts_down() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            ship.install_turret();
            let mut enemy = orz(&_battle, 1);
            enemy.at(4500, 4000);
            let marine = ship.launch();
            collide(marine, enemy.id);
            battle::get_mut(marine).thrust = 13;
            with_rng(8, 0, || intruder(marine));
            assert_eq!(battle::get(marine).thrust, 11);
            assert_eq!(enemy.descriptor_mut().ship_info.crew_level, 15);
            for expected in (1..=10).rev() {
                with_rng(8, 0, || intruder(marine));
                assert_eq!(battle::get(marine).thrust, expected);
            }
            let seed = seed_for_byte(16);
            enemy.descriptor_mut().ship_info.crew_level = 1;
            battle::get_mut(enemy.id).hits = 1;
            with_rng(seed, 1, || intruder(marine));
            assert_eq!(battle::get(enemy.id).life, 0);
            assert_eq!(
                battle::get(marine).life,
                1,
                "return begins on the next preprocess"
            );
            with_rng(seed, 1, || intruder(marine));
            assert_eq!(
                battle::get(marine).next.image.frame.raw(),
                ship.raw_frame(2, 21)
            );
            assert_eq!(battle::get(marine).state_flags & NONSOLID as u16, 0);
        }
    })
}

#[test]
fn intruder_returns_when_enemy_is_dead_or_absent_and_randomizes_facing_once() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            ship.install_turret();
            let mut enemy = orz(&_battle, 1);
            enemy.at(4500, 4000);
            let marine = ship.launch();
            collide(marine, enemy.id);
            for absent in [false, true] {
                battle::get_mut(enemy.id).hits = 0;
                if absent {
                    battle_context::unlink(enemy.id);
                }
                for seed in 1..17 {
                    battle::get_mut(marine).state_flags = (IGNORE_SIMILAR | NONSOLID) as u16;
                    battle::get_mut(marine).thrust = 0;
                    battle::get_mut(marine).preprocess_func = Some(
                        uqm_rust::engine::battle::ElementHook::Native(intruder_preprocess),
                    );
                    battle::prim_mut(marine).kind = EMPTY_PRIMITIVE;
                    seed_random(seed);
                    let facing = normalize_facing(i32::from(random() as u8));
                    with_rng(seed, 1, || intruder(marine));
                    assert_eq!(battle::get(marine).turn, (facing << 4) as u8);
                    assert_eq!(battle::get(marine).thrust, 0);
                    assert_eq!(
                        battle::get(marine).state_flags,
                        (IGNORE_SIMILAR | CHANGING | CREW_OBJECT) as u16
                    );
                    assert_eq!(battle::prim(marine).kind, STAMP_PRIMITIVE);
                    assert_eq!(
                        battle::get(marine).current.image.frame.raw(),
                        ship.raw_frame(2, 21)
                    );
                    assert_eq!(
                        battle::get(marine).next.image.frame.raw(),
                        ship.raw_frame(2, 21)
                    );
                    assert_ne!(
                        battle::get(marine).preprocess_func.unwrap().address(),
                        intruder_preprocess as *const () as usize
                    );
                }
                if absent {
                    battle::put(enemy.id);
                }
            }
        }
    })
}

#[test]
fn returning_marine_tracks_owner_cannot_board_again_and_replenishes_crew() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            ship.install_turret();
            let mut enemy = orz(&_battle, 1);
            enemy.at(4500, 4000);
            let marine = ship.launch();
            collide(marine, enemy.id);
            battle::get_mut(enemy.id).hits = 0;
            intruder(marine);
            battle::get_mut(marine).current.location = Point { x: 4000, y: 4500 };
            battle::get_mut(marine).turn = 0;
            with_rng(10, 0, || {
                hooks::invoke_element(battle::get(marine).preprocess_func.unwrap(), marine)
            });
            assert_eq!(battle::get(marine).state_flags & IGNORE_SIMILAR as u16, 0);
            assert_eq!(
                battle::get(marine).next.image.frame.raw(),
                ship.raw_frame(2, 21)
            );
            battle::get_mut(enemy.id).hits = 15;
            collide(marine, enemy.id);
            assert_eq!(enemy.descriptor_mut().ship_info.crew_level, 15);
            assert_eq!(battle::get(marine).state_flags & NONSOLID as u16, 0);
            with_rng(10, 0, || collide(marine, ship.id));
            assert_eq!(ship.descriptor_mut().ship_info.crew_level, 16);
            assert_eq!(battle::get(ship.id).hits, 16);
            assert_eq!(battle::get(marine).life, 0);
        }
    })
}

#[test]
fn owner_death_kills_both_free_and_boarded_marines_without_rng() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            ship.install_turret();
            let mut enemy = orz(&_battle, 1);
            enemy.at(4500, 4000);
            let free = ship.launch();
            let boarded = ship.launch();
            collide(boarded, enemy.id);
            battle::get_mut(ship.id).hits = 0;
            battle::get_mut(ship.id).life = 1;
            battle::get_mut(ship.id).state_flags |= (FINITE_LIFE | NONSOLID) as u16;
            with_rng(9, 0, || {
                hooks::invoke_element(battle::get(free).preprocess_func.unwrap(), free)
            });
            assert_eq!(battle::get(free).life, 0);
            assert_eq!(battle::get(free).turn, 0);
            assert_eq!(
                battle::get(free).state_flags & (DISAPPEARING | NONSOLID) as u16,
                (DISAPPEARING | NONSOLID) as u16
            );
            with_rng(9, 0, || intruder(boarded));
            assert_eq!(battle::get(boarded).life, 0);
            assert_ne!(battle::get(boarded).state_flags & DISAPPEARING as u16, 0);
            assert_eq!(
                battle::get(boarded).preprocess_func.unwrap().address(),
                intruder_preprocess as *const () as usize
            );
        }
    })
}

#[test]
fn marine_thrust_restores_owner_state_and_inserts_preprocessed_five_color_ion_trail() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            ship.install_turret();
            let mut enemy = orz(&_battle, 1);
            enemy.at(4500, 4000);
            let marine = ship.launch();
            battle::get_mut(marine).current.location = Point { x: 4000, y: 4500 };
            // Returning marine heads straight at its owner and ignores the enemy.
            battle::get_mut(marine).state_flags &= !(IGNORE_SIMILAR as u16);
            battle::get_mut(marine).turn = 0;
            ship.ship_mut().ship_facing = 7;
            ship.ship_mut().cur_status_flags = (SPECIAL | SHIP_IN_GRAVITY_WELL) as u16;
            let preprocess = battle::get(marine).preprocess_func.unwrap();
            for (speed, trail) in [(8, true), (16, true), (24, true), (32, true), (32, false)] {
                let before = queued().len();
                with_rng(9, 0, || hooks::invoke_element(preprocess, marine));
                assert_eq!(
                    velocity_fields(&battle::get(marine).velocity),
                    velocity_toward(speed, 0)
                );
                assert_eq!(queued().len(), before + usize::from(trail));
                assert_eq!(ship.ship_mut().ship_facing, 7);
                assert_eq!(
                    ship.ship_mut().cur_status_flags,
                    (SPECIAL | SHIP_IN_GRAVITY_WELL) as u16
                );
                assert_eq!(ship.descriptor_mut().characteristics.max_thrust, 35);
                assert_eq!(ship.descriptor_mut().characteristics.thrust_increment, 5);
            }
            assert_eq!(battle::get(marine).thrust, (SHIP_AT_MAX_SPEED >> 6) as u8);
            let ion = battle::head().unwrap();
            assert_eq!(battle::get(ion).player_nr, NEUTRAL_PLAYER_NUM as i16);
            assert_eq!(
                battle::get(ion).parent,
                uqm_rust::engine::battle::Parent::Ship(ship.ship_id)
            );
            assert_eq!(
                battle::get(ion).state_flags,
                (APPEARING | FINITE_LIFE | NONSOLID | PRE_PROCESS) as u16
            );
            assert_eq!(battle::get(ion).life, 0);
            assert_eq!(battle::get(ion).thrust, 1);
            assert_eq!(battle::get(ion).color_cycle_index, 0);
            assert_eq!(battle::prim(ion).kind, POINT_PRIMITIVE);
            assert_eq!(
                (
                    battle::prim(ion).color.r,
                    battle::prim(ion).color.g,
                    battle::prim(ion).color.b
                ),
                (255, 173, 0)
            );
            assert_eq!(
                battle::get(ion).current.location.x,
                (4000 + i32::from(cosine(32, 8))) as i16
            );
            assert_eq!(
                battle::get(ion).current.location.y,
                (4500 + i32::from(sine(32, 8))) as i16
            );
            assert_eq!(
                battle::get(ion).next.location.y,
                battle::get(ion).current.location.y
            );
            let cycle = battle::get(ion).death_func.unwrap();
            for (index, rgb) in [
                (1, (255, 115, 0)),
                (2, (255, 24, 0)),
                (3, (222, 0, 0)),
                (4, (156, 0, 0)),
            ] {
                battle::get_mut(ion).state_flags |= DISAPPEARING as u16;
                with_rng(9, 0, || hooks::invoke_element(cycle, ion));
                assert_eq!(battle::get(ion).color_cycle_index, index);
                assert_eq!(battle::get(ion).life, 1);
                assert_eq!(battle::get(ion).state_flags & DISAPPEARING as u16, 0);
                assert_ne!(battle::get(ion).state_flags & CHANGING as u16, 0);
                assert_eq!(
                    (
                        battle::prim(ion).color.r,
                        battle::prim(ion).color.g,
                        battle::prim(ion).color.b
                    ),
                    rgb
                );
            }
            battle::get_mut(ion).life = 0;
            battle::get_mut(ion).state_flags |= DISAPPEARING as u16;
            with_rng(9, 0, || hooks::invoke_element(cycle, ion));
            assert_eq!(battle::get(ion).color_cycle_index, 5);
            assert_eq!(battle::get(ion).life, 0);
            assert_ne!(battle::get(ion).state_flags & DISAPPEARING as u16, 0);
        }
    })
}

#[test]
fn marine_collision_delay_precedes_tracking_and_half_circle_turn_uses_one_draw() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            ship.install_turret();
            let mut enemy = orz(&_battle, 1);
            enemy.at(4500, 4000);
            let marine = ship.launch();
            let preprocess = battle::get(marine).preprocess_func.unwrap();
            battle::get_mut(marine).current.location = Point { x: 4000, y: 4500 };
            battle::get_mut(marine).state_flags &= !(IGNORE_SIMILAR as u16);
            battle::get_mut(marine).turn = 0x85;
            battle::get_mut(marine).thrust = 7;
            let before = queued().len();
            for expected in (0x80..=0x84).rev() {
                with_rng(5, 0, || hooks::invoke_element(preprocess, marine));
                assert_eq!(battle::get(marine).turn, expected);
                assert_eq!(battle::get(marine).thrust, 7);
                assert_eq!(queued().len(), before);
                assert_eq!(velocity_fields(&battle::get(marine).velocity), [0; 9]);
            }
            battle::get_mut(marine).thrust = 0;
            for seed in 1..17 {
                battle::get_mut(marine).turn = 0x80;
                seed_random(seed);
                let step = ((random() as u8 & 1) as i32) * 2 - 1;
                with_rng(seed, 1, || hooks::invoke_element(preprocess, marine));
                assert_eq!(
                    battle::get(marine).turn,
                    (normalize_facing(8 + step) << 4) as u8
                );
                assert_eq!(queued().len(), before);
            }
        }
    })
}

#[test]
fn marine_prediction_uses_wrapped_distance_but_unwrapped_future_position_with_size_casts() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            ship.install_turret();
            let mut enemy = orz(&_battle, 1);
            enemy.at(4500, 4000);
            let marine = ship.launch();
            let preprocess = battle::get(marine).preprocess_func.unwrap();
            battle::get_mut(marine).state_flags &= !(IGNORE_SIMILAR as u16);
            for (marine_x, owner_x) in [(8000, 100), (-30000, 30000), (30000, -30000)] {
                battle::get_mut(marine).current.location = Point {
                    x: marine_x,
                    y: 4000,
                };
                ship.at(owner_x, 4000);
                battle::get_mut(marine).turn = 0;
                battle::get_mut(marine).thrust = 0;
                let desired = normalize_facing(angle_to_facing(i32::from(arctan(
                    (i32::from(owner_x) - i32::from(marine_x)) as i16,
                    0,
                ))));
                assert!(desired == 4 || desired == 12);
                with_rng(4, 0, || hooks::invoke_element(preprocess, marine));
                let expected = if desired == 4 { 1 } else { 15 };
                assert_eq!(battle::get(marine).turn, expected << 4);
            }
            ship.at(4000, 4000);
            battle::get_mut(marine).current.location = Point { x: 4000, y: 4500 };
            uqm_rust::battle::velocity::set_velocity_vector(
                &mut battle::get_mut(ship.id).velocity,
                64,
                4,
            );
            battle::get_mut(marine).turn = 0;
            with_rng(4, 0, || hooks::invoke_element(preprocess, marine));
            // floor(500 / 64) = 7 turns of eastward target movement.
            assert_eq!(battle::get(marine).turn, 0x10);
        }
    })
}

#[test]
fn marine_gravity_avoidance_uses_mass_threshold_distance_and_one_draw_for_head_on() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            ship.install_turret();
            let mut enemy = orz(&_battle, 1);
            enemy.at(4500, 4000);
            let marine = ship.launch();
            battle::get_mut(marine).current.location = Point { x: 4000, y: 4000 };
            let planet_h = crate::battle_context::with(|cx| cx.alloc_element()).unwrap();
            let planet = planet_h;
            battle::get_mut(planet).mass = 101;
            battle::get_mut(planet).current.location = Point { x: 4100, y: 4000 };
            battle::put(planet_h);
            let face = normalize_facing(angle_to_facing(i32::from(arctan(100, 0))));
            let preprocess = battle::get(marine).preprocess_func.unwrap();
            let before = queued().len();
            for seed in 1..17 {
                battle::get_mut(marine).turn = (face << 4) as u8;
                battle::get_mut(marine).thrust = 0;
                uqm_rust::battle::velocity::set_velocity_vector(
                    &mut battle::get_mut(marine).velocity,
                    8,
                    face,
                );
                seed_random(seed);
                let step = ((random() as i16 & 1) as i32) * 2 - 1;
                with_rng(seed, 1, || hooks::invoke_element(preprocess, marine));
                assert_eq!(
                    battle::get(marine).turn,
                    (normalize_facing(i32::from(face) + step) << 4) as u8
                );
                assert_ne!(
                    battle::get(marine).thrust & (SHIP_IN_GRAVITY_WELL >> 6) as u8,
                    0
                );
                assert_eq!(queued().len(), before);
            }
            for (distance, in_well) in [(1020, true), (1021, false)] {
                battle::get_mut(planet).current.location.x = 4000 + distance;
                battle::get_mut(marine).turn = (face << 4) as u8;
                battle::get_mut(marine).thrust = 0;
                let draws = usize::from(in_well);
                with_rng(3, draws, || hooks::invoke_element(preprocess, marine));
                assert_eq!(
                    battle::get(marine).thrust & (SHIP_IN_GRAVITY_WELL >> 6) as u8 != 0,
                    in_well
                );
            }
        }
    })
}

#[test]
fn ai_turret_half_circle_consumes_trackship_draw_then_its_own_tie_break() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            let turret = ship.install_turret();
            let mut enemy = orz(&_battle, 1);
            enemy.at(4000, 4500);
            battle::get_mut(turret).current.location = Point { x: 4000, y: 4000 };
            ship.ship_mut().special_counter = 2;
            ship.ship_mut().weapon_counter = 1;
            ship.ship_mut().ship_facing = 0;
            for seed in 1..33 {
                battle::get_mut(turret).turn = 0;
                battle::get_mut(turret).thrust = 0;
                battle::get_mut(turret).target = None;
                ship.ship_mut().ship_input_state = 0;
                let mut concerns: [Concern; 4] = [Concern::default(); 4];
                concerns[ENEMY_SHIP_INDEX as usize].object = Some(enemy.id);
                seed_random(seed);
                random();
                let second_bit = random() as u8 & 1;
                with_rng(seed, 2, || intelligence(&mut ship, &mut concerns));
                assert_eq!(
                    ship.ship_mut().ship_input_state,
                    (SPECIAL | if second_bit == 0 { RIGHT } else { LEFT }) as u8
                );
                assert_eq!(battle::get(turret).turn, 0);
                assert_eq!(battle::get(turret).target, Some(enemy.id));
            }
        }
    })
}

#[test]
fn ai_launch_budget_obeys_crew_point_defense_maneuverability_turn_and_marine_count() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            ship.install_turret();
            let mut enemy = orz(&_battle, 1);
            enemy.at(4500, 4000);
            let a = ship.launch();
            let b = ship.launch();
            let turret = battle::succ(ship.id).unwrap();
            battle::get_mut(turret).turn = 1;
            ship.ship_mut().weapon_counter = 1;
            for (counter, crew, defense, maneuver, turn, active, expected) in [
                (0, 5, false, 24, 13, 2, true),
                (0, 5, false, 25, 13, 2, false),
                (0, 5, false, 25, 12, 2, true),
                (0, 5, false, 25, 13, 1, true),
                (0, 4, false, 0, 1, 1, false),
                (0, 5, true, 0, 1, 1, false),
                (2, 5, false, 0, 1, 1, false),
            ] {
                ship.ship_mut().special_counter = counter;
                ship.ship_mut().ship_input_state = SPECIAL as u8;
                ship.descriptor_mut().ship_info.crew_level = crew;
                enemy.descriptor_mut().ship_info.ship_flags =
                    if defense { POINT_DEFENSE as u16 } else { 0 };
                enemy.descriptor_mut().cyborg_control.maneuverability_index = maneuver;
                battle::get_mut(a).life = 1;
                battle::get_mut(b).life = u16::from(active == 2);
                let mut concerns: [Concern; 4] = [Concern::default(); 4];
                concerns[ENEMY_SHIP_INDEX as usize] = Concern {
                    object: Some(enemy.id),
                    facing: 0,
                    which_turn: turn,
                    movement: MOVEMENT_STATE_ENTICE,
                };
                with_rng(2, 0, || intelligence(&mut ship, &mut concerns));
                assert_eq!(
                    ship.ship_mut().ship_input_state,
                    if expected {
                        (SPECIAL | WEAPON) as u8
                    } else {
                        0
                    }
                );
            }
            ship.ship_mut().special_counter = 1;
            ship.ship_mut().ship_input_state = (SPECIAL | LEFT) as u8;
            let mut concerns: [Concern; 4] = [Concern::default(); 4];
            concerns[ENEMY_SHIP_INDEX as usize].object = Some(enemy.id);
            with_rng(2, 0, || intelligence(&mut ship, &mut concerns));
            assert_eq!(ship.ship_mut().ship_input_state, (SPECIAL | LEFT) as u8);
            concerns[ENEMY_SHIP_INDEX as usize].object = None;
            with_rng(2, 0, || intelligence(&mut ship, &mut concerns));
            assert_eq!(ship.ship_mut().ship_input_state, LEFT as u8);
        }
    })
}

#[test]
fn ai_entice_coasting_clears_hull_turn_only_at_c_angle_boundaries() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            let turret = ship.install_turret();
            let mut enemy = orz(&_battle, 1);
            enemy.at(4500, 4000);
            battle::get_mut(turret).turn = 1;
            ship.ship_mut().special_counter = 2;
            ship.ship_mut().weapon_counter = 1;
            let direction = arctan(500, 0);
            for (difference, turn, thrust, fast, expected_clear) in [
                (11, 23, false, true, false),
                (12, 23, false, true, true),
                (51, 23, false, true, true),
                (52, 23, false, true, false),
                (53, 23, false, true, false),
                (32, 24, false, true, false),
                (32, 23, true, true, false),
                (32, 23, false, false, false),
            ] {
                battle::get_mut(ship.id).velocity.travel_angle =
                    ((u32::from(direction) + difference) & 63) as u16;
                ship.ship_mut().cur_status_flags = if fast { SHIP_AT_MAX_SPEED as u16 } else { 0 };
                ship.ship_mut().ship_input_state = (LEFT | if thrust { THRUST } else { 0 }) as u8;
                let mut concerns: [Concern; 4] = [Concern::default(); 4];
                concerns[ENEMY_SHIP_INDEX as usize] = Concern {
                    object: Some(enemy.id),
                    facing: 0,
                    which_turn: turn,
                    movement: MOVEMENT_STATE_ENTICE,
                };
                with_rng(2, 0, || intelligence(&mut ship, &mut concerns));
                assert_eq!(
                    ship.ship_mut().ship_input_state & LEFT as u8 == 0,
                    expected_clear
                );
            }
        }
    })
}

#[test]
fn exhausted_queue_preserves_weapon_return_count_and_still_rotates_turret() {
    battle_context::run(|_| {
        let _battle = Battle::new(2);
        {
            let mut ship = orz(&_battle, 0);
            let turret = ship.install_turret();
            ship.ship_mut().cur_status_flags = (SPECIAL | LEFT) as u16;
            let mut out = Some(ship.id);
            with_rng(6, 0, || {
                assert_eq!(
                    {
                        out = ship.weapon_handles(1)[0];
                        1
                    },
                    1
                )
            });
            assert!(out.is_none());
            assert_eq!(battle::get(turret).turn, 4);
            assert_eq!(battle::get(turret).thrust, 255);
            assert_eq!(queued(), vec![ship.id, turret]);
        }
    })
}

#[test]
fn failed_initial_turret_allocation_does_not_change_ship_or_counters() {
    battle_context::run(|_| {
        let _battle = Battle::new(1);
        {
            let mut ship = orz(&_battle, 0);
            battle::get_mut(ship.id).state_flags |= APPEARING as u16;
            ship.ship_mut().weapon_counter = 7;
            ship.ship_mut().special_counter = 9;
            with_rng(4, 0, || ship.preprocess());
            assert_eq!(queued(), vec![ship.id]);
            assert_eq!(
                battle::get(ship.id).state_flags,
                (PLAYER_SHIP | APPEARING) as u16
            );
            assert_eq!(ship.ship_mut().weapon_counter, 7);
            assert_eq!(ship.ship_mut().special_counter, 9);
        }
    })
}

#[test]
fn failed_replacement_and_marine_allocation_leave_crew_and_special_budget_untouched() {
    battle_context::run(|_| {
        for capacity in [3, 4] {
            let _battle = Battle::new(capacity);
            {
                let mut ship = orz(&_battle, 0);
                let old = ship.install_turret();
                let mut enemy = orz(&_battle, 1);
                enemy.at(4500, 4000);
                ship.ship_mut().cur_status_flags = (SPECIAL | WEAPON) as u16;
                battle::get_mut(old).life = 0;
                // Facing 0 when replacement allocation fails; facing 0's angle when it succeeds.
                with_rng(3, 0, || {
                    hooks::invoke_element(battle::get(old).postprocess_func.unwrap(), old)
                });
                assert_eq!(ship.descriptor_mut().ship_info.crew_level, 16);
                assert_eq!(battle::get(ship.id).hits, 16);
                assert_eq!(ship.ship_mut().special_counter, 0);
                assert_eq!(battle::prim(old).kind, EMPTY_PRIMITIVE);
                assert_eq!(queued().len(), capacity as usize);
                assert_eq!(
                    battle::get(ship.id).target,
                    Some(enemy.id),
                    "tracking eligibility runs before failed marine allocation"
                );
            }
        }
    })
}

#[test]
fn failed_muzzle_flash_allocation_does_not_recoil_and_failed_ion_allocation_still_thrusts() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(4);
            {
                let mut ship = orz(&_battle, 0);
                ship.install_turret();
                ship.ship_mut().cur_status_flags = WEAPON as u16;
                let h = ship.missile();
                battle::put(h);
                let turret = ship.tick_turret();
                assert_eq!(queued().len(), 3);
                assert_eq!(
                    (
                        battle::get(turret).current.location.x,
                        battle::get(turret).current.location.y
                    ),
                    (4000, 4000)
                );
                assert_eq!({ battle::tail() }, Some(h));
            }
        }
        {
            let _battle = Battle::new(5);
            {
                let mut ship = orz(&_battle, 0);
                ship.install_turret();
                let mut enemy = orz(&_battle, 1);
                enemy.at(4500, 4000);
                let marine = ship.launch();
                let filler = crate::battle_context::with(|cx| cx.alloc_element()).unwrap();
                battle::put(filler);
                battle::get_mut(marine).state_flags &= !(IGNORE_SIMILAR as u16);
                battle::get_mut(marine).current.location = Point { x: 4000, y: 4500 };
                battle::get_mut(marine).turn = 0;
                let before = queued();
                with_rng(1, 0, || {
                    hooks::invoke_element(battle::get(marine).preprocess_func.unwrap(), marine)
                });
                assert_eq!(queued(), before);
                assert_eq!(
                    velocity_fields(&battle::get(marine).velocity),
                    velocity_toward(8, 0)
                );
                assert_eq!(ship.descriptor_mut().characteristics.max_thrust, 35);
                assert_eq!(ship.descriptor_mut().characteristics.thrust_increment, 5);
            }
        }
    })
}

#[test]
fn marine_prefers_visible_enemy_then_returns_to_visible_owner_or_keeps_heading() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            ship.install_turret();
            let mut enemy = orz(&_battle, 1);
            enemy.at(4500, 4000);
            let marine = ship.launch();
            enemy.at(4500, 4500);
            battle::get_mut(marine).current.location = Point { x: 4000, y: 4500 };
            let preprocess = battle::get(marine).preprocess_func.unwrap();
            for (enemy_visible, owner_visible, expected_facing, returning) in [
                (true, true, 1, false),
                (false, true, 0, true),
                (false, false, 0, false),
            ] {
                battle::prim_mut(enemy.id).kind = if enemy_visible {
                    STAMP_PRIMITIVE
                } else {
                    EMPTY_PRIMITIVE
                };
                battle::prim_mut(ship.id).kind = if owner_visible {
                    STAMP_PRIMITIVE
                } else {
                    EMPTY_PRIMITIVE
                };
                battle::get_mut(marine).state_flags = (IGNORE_SIMILAR | CREW_OBJECT) as u16;
                battle::get_mut(marine).turn = 0;
                battle::get_mut(marine).thrust = 0;
                battle::get_mut(marine).velocity = VelocityDesc::new();
                battle::get_mut(marine).next.image.frame = ship.frame(2, 20);
                let before = queued().len();
                with_rng(8, 0, || hooks::invoke_element(preprocess, marine));
                assert_eq!(battle::get(marine).turn, expected_facing << 4);
                assert_eq!(
                    battle::get(marine).state_flags & IGNORE_SIMILAR as u16 == 0,
                    returning
                );
                assert_eq!(
                    battle::get(marine).next.image.frame.raw(),
                    ship.raw_frame(2, if returning { 21 } else { 20 })
                );
                assert_eq!(queued().len(), before + usize::from(returning));
                assert_eq!(
                    velocity_fields(&battle::get(marine).velocity),
                    if returning {
                        velocity_toward(8, 0)
                    } else {
                        [0; 9]
                    }
                );
            }
        }
    })
}

#[test]
fn marine_without_target_clears_stale_gravity_and_decelerates_only_when_overspeed() {
    battle_context::run(|_| {
        let _battle = Battle::new(128);
        {
            let mut ship = orz(&_battle, 0);
            ship.install_turret();
            let mut enemy = orz(&_battle, 1);
            enemy.at(4500, 4000);
            let marine = ship.launch();
            battle::prim_mut(ship.id).kind = EMPTY_PRIMITIVE;
            battle::prim_mut(enemy.id).kind = EMPTY_PRIMITIVE;
            let preprocess = battle::get(marine).preprocess_func.unwrap();
            for overspeed in [false, true] {
                battle::get_mut(marine).turn = 0;
                battle::get_mut(marine).thrust = ((SHIP_IN_GRAVITY_WELL
                    | if overspeed { SHIP_BEYOND_MAX_SPEED } else { 0 })
                    >> 6) as u8;
                uqm_rust::battle::velocity::set_velocity_vector(
                    &mut battle::get_mut(marine).velocity,
                    40,
                    8,
                );
                let before = queued().len();
                with_rng(9, 0, || hooks::invoke_element(preprocess, marine));
                assert_eq!(
                    battle::get(marine).thrust & (SHIP_IN_GRAVITY_WELL >> 6) as u8,
                    0
                );
                assert_eq!(battle::get(marine).turn, 0);
                assert_eq!(
                    velocity_fields(&battle::get(marine).velocity),
                    velocity_toward(if overspeed { 32 } else { 40 }, 8)
                );
                assert_eq!(queued().len(), before + usize::from(overspeed));
                assert_eq!(ship.ship_mut().ship_facing, 0);
                assert_eq!(ship.descriptor_mut().characteristics.max_thrust, 35);
            }
        }
    })
}

#[path = "support/hooks.rs"]
mod hooks;
