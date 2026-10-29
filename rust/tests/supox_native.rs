//! Exercise the Supox entry point and registered callbacks against the linked engine.

#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
use battle::*;

use std::{ffi::CStr, mem};
use uqm_rust::ships::native::{abi::*, element};

/// The element queue and RNG are process-wide engine state.
use battle::Battle as Battlefield;

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

fn velocity_toward(speed: SIZE, facing: COUNT) -> [i32; 9] {
    unsafe {
        let mut v: VELOCITY_DESC = mem::zeroed();
        SetVelocityVector(&mut v, speed, facing);
        velocity_fields(&v)
    }
}

#[test]
fn descriptor_matches_the_c_race_data() {
    battle_context::run(|| {
        unsafe {
            let desc = descriptor(SPECIES_ID_SUPOX_ID);
            assert!(std::ptr::eq(
                desc.race.unwrap(),
                descriptor(SPECIES_ID_SUPOX_ID).race.unwrap()
            ));
            assert_eq!(desc.ship_info.ship_flags, FIRES_FORE as u16);
            assert_eq!(desc.ship_info.ship_cost, 16);
            assert_eq!(desc.ship_info.crew_level, 12);
            assert_eq!(desc.ship_info.max_crew, 12);
            assert_eq!(desc.ship_info.energy_level, 16);
            assert_eq!(desc.ship_info.max_energy, 16);
            let text = |p| CStr::from_ptr(p);
            assert_eq!(text(desc.ship_info.race_strings_rsc), c"ship.supox.text");
            assert_eq!(text(desc.ship_info.icons_rsc), c"ship.supox.icons");
            assert_eq!(
                text(desc.ship_info.melee_icon_rsc),
                c"ship.supox.meleeicons"
            );
            // 333 / SPHERE_RADIUS_INCREMENT * 2
            assert_eq!(desc.fleet.strength, 60);
            assert_eq!(
                (desc.fleet.known_loc.x, desc.fleet.known_loc.y),
                (7468, 9246)
            );

            let c = desc.characteristics;
            assert_eq!(
                (c.max_thrust, c.thrust_increment, c.energy_regeneration),
                (40, 8, 1)
            );
            assert_eq!(
                (c.weapon_energy_cost, c.special_energy_cost, c.energy_wait),
                (1, 1, 4)
            );
            assert_eq!((c.turn_wait, c.thrust_wait, c.weapon_wait), (1, 0, 2));
            assert_eq!((c.special_wait, c.ship_mass), (0, 4));

            for (i, size) in ["large", "medium", "small"].iter().enumerate() {
                assert_eq!(
                    text(desc.ship_data.ship_rsc[i]).to_str().unwrap(),
                    format!("ship.supox.graphics.blade.{size}")
                );
                assert_eq!(
                    text(desc.ship_data.weapon_rsc[i]).to_str().unwrap(),
                    format!("ship.supox.graphics.glob.{size}")
                );
                assert!(desc.ship_data.special_rsc[i].is_null());
            }
            assert_eq!(
                text(desc.ship_data.captain_control.captain_rsc),
                c"ship.supox.graphics.captain"
            );
            assert_eq!(text(desc.ship_data.victory_ditty_rsc), c"ship.supox.ditty");
            assert_eq!(text(desc.ship_data.ship_sounds_rsc), c"ship.supox.sounds");

            // (MISSILE_SPEED * MISSILE_LIFE) >> 1 with MISSILE_SPEED = 120
            assert_eq!(desc.cyborg_control.WeaponRange, 600);
            assert_eq!(desc.cyborg_control.ManeuverabilityIndex, 0);
            assert!(desc.callbacks()[4] != 0);
            assert!(desc.callbacks()[1] != 0);
            assert!(desc.callbacks()[3] != 0);
            assert!(desc.callbacks()[0] == 0);
            assert!(desc.callbacks()[2] == 0);
            assert!(desc.data.is_null());
            assert!(desc.code_ref.is_null());
        }
    })
}

#[test]
fn gob_is_launched_with_the_c_missile_block() {
    battle_context::run(|| {
        let _battle = Battlefield::new(8);
        unsafe {
            let mut ship = Ship::bare(&_battle, SPECIES_ID_SUPOX_ID, 0);
            ship.ship_mut().ShipFacing = 4;
            ship.element_mut().next.location = POINT { x: 100, y: 200 };
            let slots = ship.weapon(1);
            let horns = [ship.weapon_record(slots[0]).cast()];

            let gob = element(horns[0]);
            assert!(!gob.is_null());
            assert_eq!((*gob).__bindgen_anon_1.life_span, 10);
            assert_eq!((*gob).__bindgen_anon_2.hit_points, 1);
            assert_eq!((*gob).__bindgen_anon_3.mass_points, 1);
            assert_eq!((*gob).__bindgen_anon_5.blast_offset, 2);
            assert_ne!((*gob).state_flags & IGNORE_SIMILAR as u16, 0);
            assert!((*gob).preprocess_func.is_none());
            // DISPLAY_TO_WORLD (30) world units per tick along the ship facing.
            assert_eq!(velocity_fields(&(*gob).velocity), velocity_toward(120, 4));
            FreeElement(horns[0]);
        }
    })
}

/// Thrusts a second ship at `expected_facing` and returns what the engine
/// computes for that heading, as the oracle for the lateral/reverse maths.
unsafe fn thrust_heading(_battle: &Battle, heading: COUNT, flags: u16) -> ([i32; 9], u16) {
    let mut oracle = Ship::bare(_battle, SPECIES_ID_SUPOX_ID, 0);
    // SAFETY: the caller holds the serialized battle through these fixture projections.
    unsafe {
        oracle.ship_mut().ShipFacing = heading;
        oracle.ship_mut().cur_status_flags = flags;
        let status = inertial_thrust(oracle.el());
        (velocity_fields(&oracle.element_mut().velocity), status)
    }
}

#[test]
fn special_thrusts_laterally_and_in_reverse_then_restores_the_facing() {
    battle_context::run(|| {
        let _battle = Battlefield::new(8);
        unsafe {
            // (held controls, ship facing, heading the thrust is applied along)
            let cases: [(u32, COUNT, COUNT); 8] = [
                (SPECIAL | LEFT, 4, 0),
                (SPECIAL | RIGHT, 4, 8),
                (SPECIAL | THRUST, 4, 12),
                (SPECIAL | THRUST | LEFT, 4, 14),
                (SPECIAL | THRUST | RIGHT, 4, 10),
                (SPECIAL | LEFT, 0, 12),
                (SPECIAL | RIGHT, 15, 3),
                (SPECIAL | THRUST | LEFT | RIGHT, 1, 11),
            ];
            for (held, facing, heading) in cases {
                let mut ship = Ship::bare(&_battle, SPECIES_ID_SUPOX_ID, 0);
                ship.ship_mut().ShipFacing = facing;
                ship.ship_mut().cur_status_flags = held as u16;
                ship.preprocess();

                let (velocity, status) = thrust_heading(&_battle, heading, held as u16);
                assert_ne!(velocity, [0; 9], "case {held:#x}/{facing}");
                assert_eq!(
                    velocity_fields(&ship.element_mut().velocity),
                    velocity,
                    "case {held:#x}/{facing}"
                );
                assert_eq!(
                    ship.ship_mut().ShipFacing,
                    facing,
                    "the real facing is restored"
                );
                let thrust_flags =
                    (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED | SHIP_IN_GRAVITY_WELL) as u16;
                assert_eq!(ship.ship_mut().cur_status_flags, held as u16 | status);
                assert_eq!(status & !thrust_flags, 0);
            }
        }
    })
}

#[test]
fn special_arms_the_wait_counters_and_normal_flight_is_untouched() {
    battle_context::run(|| {
        let _battle = Battlefield::new(8);
        unsafe {
            let mut ship = Ship::bare(&_battle, SPECIES_ID_SUPOX_ID, 0);
            ship.ship_mut().ShipFacing = 4;
            ship.ship_mut().cur_status_flags = (SPECIAL | THRUST | LEFT) as u16;
            ship.preprocess();
            assert_eq!(ship.element_mut().__bindgen_anon_5.thrust_wait, 1);
            assert_eq!(ship.element_mut().__bindgen_anon_4.turn_wait, 1);

            // Existing waits are not rearmed.
            ship.element_mut().__bindgen_anon_5.thrust_wait = 3;
            ship.element_mut().__bindgen_anon_4.turn_wait = 5;
            ship.ship_mut().cur_status_flags = (SPECIAL | THRUST | RIGHT) as u16;
            ship.preprocess();
            assert_eq!(ship.element_mut().__bindgen_anon_5.thrust_wait, 3);
            assert_eq!(ship.element_mut().__bindgen_anon_4.turn_wait, 5);

            // Without SPECIAL, or with SPECIAL alone, nothing is changed.
            for held in [THRUST | LEFT, SPECIAL] {
                let mut ship = Ship::bare(&_battle, SPECIES_ID_SUPOX_ID, 0);
                ship.ship_mut().ShipFacing = 4;
                ship.ship_mut().cur_status_flags = held as u16;
                ship.preprocess();
                assert_eq!(velocity_fields(&ship.element_mut().velocity), [0; 9]);
                assert_eq!(ship.ship_mut().cur_status_flags, held as u16);
                assert_eq!(ship.ship_mut().ShipFacing, 4);
                assert_eq!(ship.element_mut().__bindgen_anon_5.thrust_wait, 0);
                assert_eq!(ship.element_mut().__bindgen_anon_4.turn_wait, 0);
            }
        }
    })
}

/// An enemy ship placed `distance` units due north of a Supox at the origin.
struct Chase<'a> {
    ship: Ship<'a>,
    enemy: Ship<'a>,
    concerns: [EVALUATE_DESC; 4],
}

impl<'a> Chase<'a> {
    unsafe fn new(_battle: &'a Battle, which_turn: u16, enemy_facing: COUNT) -> Self {
        let mut chase = Self {
            ship: Ship::bare(_battle, SPECIES_ID_SUPOX_ID, 0),
            enemy: Ship::bare(_battle, SPECIES_ID_SUPOX_ID, 1),
            concerns: unsafe { mem::zeroed() },
        };
        // SAFETY: both fixture slots belong to the caller's serialized battle.
        unsafe {
            chase.ship.element_mut().next.location = POINT { x: 5000, y: 5000 };
            chase.enemy.element_mut().next.location = POINT { x: 5000, y: 4000 };
        }
        let slot = &mut chase.concerns[ENEMY_SHIP_INDEX as usize];
        slot.ObjectPtr = chase.enemy.el();
        slot.which_turn = which_turn;
        slot.facing = enemy_facing;
        // Pursuit is only chosen by the burst decision, so start elsewhere.
        slot.MoveState = MOVEMENT_STATE_AVOID;
        chase
    }

    fn run(&mut self) {
        // A zero concern count leaves only the Supox decisions to run.
        unsafe {
            self.ship
                .intelligence(&mut *self.ship.ptr(), &mut self.concerns, 0);
        }
    }

    unsafe fn input(&mut self) -> u8 {
        // SAFETY: the caller retains the serialized battle while reading its ship slot.
        unsafe { self.ship.ship_mut().ship_input_state }
    }

    fn enemy_state(&self) -> MOVEMENT_STATE {
        self.concerns[ENEMY_SHIP_INDEX as usize].MoveState
    }
}

#[test]
fn ai_lines_up_the_reverse_burst_with_the_c_conditions() {
    battle_context::run(|| {
        let _battle = Battlefield::new(8);
        assert_eq!(ARCTAN(0, -1000), 0, "north is angle 0");
        unsafe {
            let special = SPECIAL as u8;
            let burst = (SPECIAL | THRUST | WEAPON) as u8;
            // Head-on: the enemy is north, the Supox faces north and the enemy faces
            // back at it (angle 32).
            let mut chase = Chase::new(&_battle, 10, 32);
            chase.run();
            assert_eq!(chase.input(), burst);
            assert_eq!(chase.enemy_state(), MOVEMENT_STATE_PURSUE);

            // Between 13 and 20 turns away a held special request is kept but a new
            // one is not started.
            drop(chase);
            let mut chase = Chase::new(&_battle, 15, 32);
            chase.run();
            assert_eq!(chase.input(), 0);
            assert_eq!(chase.enemy_state(), MOVEMENT_STATE_AVOID);
            drop(chase);
            let mut chase = Chase::new(&_battle, 15, 32);
            chase.ship.ship_mut().ship_input_state = special;
            chase.run();
            assert_eq!(chase.input(), burst);
            assert_eq!(chase.enemy_state(), MOVEMENT_STATE_PURSUE);

            // Too far, wrong enemy heading, wrong own heading, or cooling down.
            drop(chase);
            let mut chase = Chase::new(&_battle, 21, 32);
            chase.ship.ship_mut().ship_input_state = special;
            chase.run();
            assert_eq!(chase.input(), 0);
            assert_eq!(chase.enemy_state(), MOVEMENT_STATE_AVOID);
            drop(chase);
            let mut chase = Chase::new(&_battle, 10, 0);
            chase.run();
            assert_eq!(chase.input(), 0);
            assert_eq!(chase.enemy_state(), MOVEMENT_STATE_AVOID);
            drop(chase);
            let mut chase = Chase::new(&_battle, 10, 32);
            chase.ship.ship_mut().ShipFacing = 8;
            chase.ship.ship_mut().ship_input_state = special;
            chase.run();
            assert_eq!(chase.input(), 0);
            drop(chase);
            let mut chase = Chase::new(&_battle, 10, 32);
            chase.ship.ship_mut().special_counter = 1;
            chase.ship.ship_mut().ship_input_state = special;
            chase.run();
            assert_eq!(chase.input(), 0);

            // The enemy's facing window is +-8 around the reverse of the Supox facing.
            for (enemy_facing, expected) in [(24, burst), (40, burst), (23, 0), (41, 0)] {
                let mut chase = Chase::new(&_battle, 10, enemy_facing);
                chase.run();
                assert_eq!(chase.input(), expected, "enemy facing {enemy_facing}");
            }
        }
    })
}

#[test]
fn ai_without_an_enemy_ship_drops_the_special_request() {
    battle_context::run(|| {
        let _battle = Battlefield::new(8);
        unsafe {
            let mut ship = Ship::bare(&_battle, SPECIES_ID_SUPOX_ID, 0);
            ship.ship_mut().ship_input_state = SPECIAL as u8;
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            ship.intelligence(&mut *ship.ptr(), &mut concerns, 0);
            assert_eq!(ship.ship_mut().ship_input_state, 0);
        }
    })
}

/// An incoming weapon the AI wants to dodge.
fn dodge(ship: &mut Ship, concerns: &mut [EVALUATE_DESC; 4], weapon: *mut ELEMENT) {
    let slot = &mut concerns[ENEMY_WEAPON_INDEX as usize];
    slot.ObjectPtr = weapon;
    slot.MoveState = MOVEMENT_STATE_AVOID;
    unsafe {
        ship.intelligence(&mut *ship.ptr(), concerns, 0);
    }
}

#[test]
fn ai_dodges_with_the_held_turn_or_a_random_side_from_the_byte_draw() {
    battle_context::run(|| {
        let _battle = Battlefield::new(8);
        unsafe {
            let mut missile_fixture = Object::new(&_battle);
            let missile = missile_fixture.ptr();

            // No turn held: the side comes from bit 0 of the low byte of one draw.
            let mut seen = [false; 2];
            for seed in 1..64u32 {
                TFB_SeedRandom(seed);
                let bit = (TFB_Random() as u8 & 1) as usize;
                let mut ship = Ship::bare(&_battle, SPECIES_ID_SUPOX_ID, 0);
                ship.ship_mut().ship_input_state = THRUST as u8;
                let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
                TFB_SeedRandom(seed);
                dodge(&mut ship, &mut concerns, missile);
                let expected = (SPECIAL as u8) | (1u8 << bit);
                assert_eq!(ship.ship_mut().ship_input_state, expected, "seed {seed}");
                assert_eq!(expected & (LEFT | RIGHT) as u8, 1 << bit);
                // Exactly one draw was used.
                let after = TFB_Random();
                TFB_SeedRandom(seed);
                TFB_Random();
                assert_eq!(after, TFB_Random(), "seed {seed}");
                seen[bit] = true;
            }
            assert_eq!(seen, [true, true], "both turn directions are reachable");

            // A held turn is reused and consumes no random numbers.
            for held in [LEFT, RIGHT] {
                let mut ship = Ship::bare(&_battle, SPECIES_ID_SUPOX_ID, 0);
                ship.ship_mut().cur_status_flags = held as u16;
                ship.ship_mut().ship_input_state = THRUST as u8;
                let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
                TFB_SeedRandom(9);
                dodge(&mut ship, &mut concerns, missile);
                assert_eq!(ship.ship_mut().ship_input_state, SPECIAL as u8 | held as u8);
                let after = TFB_Random();
                TFB_SeedRandom(9);
                assert_eq!(after, TFB_Random());
            }

            // Not while the special is cooling down, while the turn is blocked, or
            // when the incoming object is not being avoided.
            let mut ship = Ship::bare(&_battle, SPECIES_ID_SUPOX_ID, 0);
            ship.ship_mut().special_counter = 1;
            ship.ship_mut().ship_input_state = THRUST as u8;
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            dodge(&mut ship, &mut concerns, missile);
            assert_eq!(ship.ship_mut().ship_input_state, THRUST as u8);

            let mut ship = Ship::bare(&_battle, SPECIES_ID_SUPOX_ID, 0);
            ship.element_mut().__bindgen_anon_4.turn_wait = 1;
            ship.ship_mut().ship_input_state = THRUST as u8;
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            dodge(&mut ship, &mut concerns, missile);
            assert_eq!(ship.ship_mut().ship_input_state, THRUST as u8);

            let mut ship = Ship::bare(&_battle, SPECIES_ID_SUPOX_ID, 0);
            ship.ship_mut().ship_input_state = THRUST as u8;
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            concerns[ENEMY_WEAPON_INDEX as usize].ObjectPtr = missile;
            concerns[ENEMY_WEAPON_INDEX as usize].MoveState = MOVEMENT_STATE_ENTICE;
            ship.intelligence(&mut *ship.ptr(), &mut concerns, 0);
            assert_eq!(ship.ship_mut().ship_input_state, THRUST as u8);
        }
    })
}
