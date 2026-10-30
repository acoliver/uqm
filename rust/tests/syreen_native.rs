//! Exercise Syreen descriptor registration and callbacks against the linked engine.

#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
use battle::*;

use std::{ffi::CStr, mem, ptr};
use uqm_rust::ships::native::{abi::*, element};

/// The queue holds exactly the elements a test creates, so a test can also
/// exhaust the allocator.
const QUEUE_SLOTS: u16 = 6;

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
            let desc = descriptor(SPECIES_ID_SYREEN_ID);
            assert!(std::ptr::eq(
                desc.race.unwrap(),
                descriptor(SPECIES_ID_SYREEN_ID).race.unwrap()
            ));
            assert_eq!(desc.ship_info.ship_flags, FIRES_FORE as u16);
            assert_eq!(desc.ship_info.ship_cost, 13);
            assert_eq!(desc.ship_info.crew_level, 12);
            assert_eq!(desc.ship_info.max_crew, 42);
            assert_eq!(desc.ship_info.energy_level, 16);
            assert_eq!(desc.ship_info.max_energy, 16);
            let text = |p| CStr::from_ptr(p);
            assert_eq!(text(desc.ship_info.race_strings_rsc), c"ship.syreen.text");
            assert_eq!(text(desc.ship_info.icons_rsc), c"ship.syreen.icons");
            assert_eq!(
                text(desc.ship_info.melee_icon_rsc),
                c"ship.syreen.meleeicons"
            );
            assert_eq!(desc.fleet.strength, 0);
            assert_eq!((desc.fleet.known_loc.x, desc.fleet.known_loc.y), (0, 0));

            let c = desc.characteristics;
            assert_eq!(
                (c.max_thrust, c.thrust_increment, c.energy_regeneration),
                (36, 9, 1)
            );
            assert_eq!(
                (c.weapon_energy_cost, c.special_energy_cost, c.energy_wait),
                (1, 5, 6)
            );
            assert_eq!((c.turn_wait, c.thrust_wait, c.weapon_wait), (1, 1, 8));
            assert_eq!((c.special_wait, c.ship_mass), (20, 2));

            for (i, size) in ["large", "medium", "small"].iter().enumerate() {
                assert_eq!(
                    text(desc.ship_data.ship_rsc[i]).to_str().unwrap(),
                    format!("ship.syreen.graphics.penetrator.{size}")
                );
                assert_eq!(
                    text(desc.ship_data.weapon_rsc[i]).to_str().unwrap(),
                    format!("ship.syreen.graphics.dagger.{size}")
                );
                assert!(desc.ship_data.special_rsc[i].is_null());
            }
            assert_eq!(
                text(desc.ship_data.captain_control.captain_rsc),
                c"ship.syreen.graphics.captain"
            );
            assert_eq!(text(desc.ship_data.victory_ditty_rsc), c"ship.syreen.ditty");
            assert_eq!(text(desc.ship_data.ship_sounds_rsc), c"ship.syreen.sounds");

            // MISSILE_SPEED * MISSILE_LIFE * 2 / 3 with MISSILE_SPEED = 120
            assert_eq!(desc.cyborg_control.WeaponRange, 800);
            assert_eq!(desc.cyborg_control.ManeuverabilityIndex, 0);
            assert!(desc.callbacks()[4] != 0);
            assert!(desc.callbacks()[2] != 0);
            assert!(desc.callbacks()[3] != 0);
            assert!(desc.callbacks()[1] == 0);
            assert!(desc.callbacks()[0] == 0);
            assert!(desc.data.is_null());
            assert!(desc.code_ref.is_null());
        }
    })
}

#[test]
fn dagger_is_launched_with_the_c_missile_block() {
    battle_context::run(|| {
        let _battle = Battlefield::new(QUEUE_SLOTS + 5);
        unsafe {
            let mut ship = Ship::bare(&_battle, SPECIES_ID_SYREEN_ID, 0);
            ship.ship_mut().ShipFacing = 4;
            ship.element_mut().next.location = POINT { x: 1000, y: 2000 };
            let slots = ship.weapon(1);
            let daggers = [ship.weapon_record(slots[0]).cast()];

            let dagger = element(daggers[0]);
            assert!(!dagger.is_null());
            assert_eq!((*dagger).__bindgen_anon_1.life_span, 10);
            assert_eq!((*dagger).__bindgen_anon_2.hit_points, 1);
            assert_eq!((*dagger).__bindgen_anon_3.mass_points, 2);
            assert_eq!((*dagger).__bindgen_anon_5.blast_offset, 3);
            assert_ne!((*dagger).state_flags & IGNORE_SIMILAR as u16, 0);
            assert!((*dagger).preprocess_func.is_none());
            // DISPLAY_TO_WORLD (30) world units per tick along the ship facing.
            assert_eq!(
                velocity_fields(&(*dagger).velocity),
                velocity_toward(120, 4)
            );
            FreeElement(daggers[0]);
        }
    })
}

#[test]
fn song_needs_the_special_key_a_ready_cooldown_and_energy() {
    battle_context::run(|| {
        let _battle = Battlefield::new(QUEUE_SLOTS + 5);
        unsafe {
            let mut ship = Ship::bare(&_battle, SPECIES_ID_SYREEN_ID, 0);

            ship.ship_mut().cur_status_flags = 0;
            ship.postprocess();
            assert!(disp_q.tail.is_null());
            assert_eq!(ship.ship_mut().special_counter, 0);

            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.ship_mut().special_counter = 3;
            ship.postprocess();
            assert!(disp_q.tail.is_null());
            assert_eq!(ship.ship_mut().special_counter, 3);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 16);

            // Too little energy: the engine records the shortage and nothing starts.
            ship.ship_mut().special_counter = 0;
            ship.descriptor_mut().ship_info.energy_level = 4;
            ship.postprocess();
            assert!(disp_q.tail.is_null());
            assert_eq!(ship.ship_mut().special_counter, 0);
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 4);
        }
    })
}

#[test]
fn song_pays_energy_and_queues_the_deferred_scan() {
    battle_context::run(|| {
        let _battle = Battlefield::new(QUEUE_SLOTS + 5);
        unsafe {
            let mut ship = Ship::bare(&_battle, SPECIES_ID_SYREEN_ID, 0);
            ship.element_mut().next.location = POINT { x: 1234, y: 4321 };
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.postprocess();

            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 16 - 5);
            assert_eq!(ship.ship_mut().special_counter, 20);
            let scan = element(disp_q.tail);
            assert!(!scan.is_null());
            assert_eq!(
                (*scan).state_flags,
                (APPEARING | NONSOLID | FINITE_LIFE) as u16
            );
            assert_eq!((*scan).__bindgen_anon_1.life_span, 0);
            assert_eq!((*scan).playerNr, 0);
            assert_eq!(
                ((*scan).next.location.x, (*scan).next.location.y),
                (1234, 4321)
            );
            assert_eq!((*scan).pParent, ship.element_mut().pParent);
            assert!((*scan).hTarget.is_null());
            assert!((*scan).death_func.is_some());

            // The song cannot be restarted during the cooldown.
            ship.postprocess();
            assert_eq!(ship.descriptor_mut().ship_info.energy_level, 16 - 5);
            assert_eq!(element(disp_q.tail), scan);
        }
    })
}

unsafe fn enter_battle(ship: &mut Ship<'_>, location: POINT, crew: u16) {
    ship.element_mut().next.location = location;
    ship.element_mut().set_hit_points(crew);
    ship.descriptor_mut().ship_info.crew_level = crew;
    ship.queue();
}

/// The scan the song queues, ready to be run as the element's death function.
unsafe fn queued_scan(ship: &mut Ship) -> (*mut ELEMENT, uqm_rust::engine::battle::ElementHook) {
    ship.ship_mut().cur_status_flags = SPECIAL as u16;
    ship.postprocess();
    let scan = element(disp_q.tail);
    (scan, (*scan).death_func.unwrap())
}

#[test]
fn deferred_scan_requeues_nothing_and_leaves_out_of_range_crews_alone() {
    battle_context::run(|| {
        let _battle = Battlefield::new(QUEUE_SLOTS + 5);
        unsafe {
            let mut ship = Ship::bare(&_battle, SPECIES_ID_SYREEN_ID, 0);
            ship.element_mut().next.location = POINT { x: 5000, y: 5000 };
            let (scan, run_scan) = queued_scan(&mut ship);

            // Queue candidates the scan must skip: a far ship, a friendly ship, a
            // ship with a single crew member, and a non-ship element.
            let mut far = Ship::bare(&_battle, SPECIES_ID_SYREEN_ID, 1);
            enter_battle(
                &mut far,
                POINT {
                    x: 5000 + 4 * 209,
                    y: 5000,
                },
                10,
            );
            let mut friend = Ship::bare(&_battle, SPECIES_ID_SYREEN_ID, 0);
            enter_battle(&mut friend, POINT { x: 5100, y: 5000 }, 10);
            let mut last_crew = Ship::bare(&_battle, SPECIES_ID_SYREEN_ID, 1);
            enter_battle(&mut last_crew, POINT { x: 5100, y: 5000 }, 1);
            let mut shot = Ship::bare(&_battle, SPECIES_ID_SYREEN_ID, 1);
            shot.element_mut().state_flags = 0;
            enter_battle(&mut shot, POINT { x: 5100, y: 5000 }, 10);

            let before = (disp_q.head, disp_q.tail);
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(run_scan, scan);
            assert_eq!((disp_q.head, disp_q.tail), before, "nothing was queued");
            for handle in walk_queue() {
                let queued = element(handle);
                if (*queued).state_flags & PLAYER_SHIP as u16 != 0 {
                    let crew = (*queued).__bindgen_anon_2.crew_level;
                    assert!(matches!(crew, 1 | 10), "crew {crew}");
                }
            }
        }
    })
}

unsafe fn walk_queue() -> Vec<HELEMENT> {
    let mut handles = Vec::new();
    let mut handle = disp_q.head;
    while !handle.is_null() {
        handles.push(handle);
        handle = (*element(handle)).succ;
    }
    handles
}

#[test]
fn deferred_scan_survives_a_ship_in_range_when_no_crew_elements_are_available() {
    battle_context::run(|| {
        let _battle = Battlefield::new(QUEUE_SLOTS + 5);
        unsafe {
            let mut ship = Ship::bare(&_battle, SPECIES_ID_SYREEN_ID, 0);
            ship.element_mut().next.location = POINT { x: 5000, y: 5000 };
            let (scan, run_scan) = queued_scan(&mut ship);

            // A ship at the edge of the abandon range reaches AbandonShip. With
            // the allocator exhausted it has no element to spawn crew members
            // with, so the scan has to finish with the queue and crew untouched.
            let mut victim = Ship::bare(&_battle, SPECIES_ID_SYREEN_ID, 1);
            enter_battle(
                &mut victim,
                POINT {
                    x: 5000 + 4 * 208,
                    y: 5000,
                },
                10,
            );
            let mut ghosts = Vec::new();
            loop {
                let handle = AllocElement();
                if handle.is_null() {
                    break;
                }
                ghosts.push(handle);
            }
            let before = walk_queue();
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(run_scan, scan);
            assert_eq!(walk_queue(), before);
            assert_eq!((*element(before[1])).__bindgen_anon_2.crew_level, 10);
            for handle in ghosts {
                FreeElement(handle);
            }
        }
    })
}

struct Duel<'a> {
    ship: Ship<'a>,
    enemy: Ship<'a>,
    concerns: [EVALUATE_DESC; 4],
}

impl<'a> Duel<'a> {
    unsafe fn new(_battle: &'a Battle, which_turn: u16) -> Self {
        let mut duel = Self {
            ship: Ship::bare(_battle, SPECIES_ID_SYREEN_ID, 0),
            enemy: Ship::bare(_battle, SPECIES_ID_SYREEN_ID, 1),
            concerns: unsafe { mem::zeroed() },
        };
        duel.enemy.element_mut().__bindgen_anon_2.crew_level = 5;
        let slot = &mut duel.concerns[ENEMY_SHIP_INDEX as usize];
        slot.ObjectPtr = duel.enemy.el();
        slot.which_turn = which_turn;
        duel
    }

    fn run(&mut self) {
        // A zero concern count leaves only the Syreen decision to run.
        unsafe {
            self.ship
                .intelligence(&mut *self.ship.ptr(), &mut self.concerns, 0);
        }
    }

    unsafe fn input(&mut self) -> u8 {
        self.ship.ship_mut().ship_input_state
    }
}

#[test]
fn ai_sings_only_when_it_can_drain_a_reachable_crew() {
    battle_context::run(|| {
        let _battle = Battlefield::new(QUEUE_SLOTS + 5);
        unsafe {
            let special = SPECIAL as u8;

            let mut duel = Duel::new(&_battle, 14);
            duel.run();
            assert_eq!(duel.input(), special);

            drop(duel);
            let mut duel = Duel::new(&_battle, 15);
            duel.ship.ship_mut().ship_input_state = special;
            duel.run();
            assert_eq!(duel.input(), 0, "too many turns away drops the request");

            drop(duel);
            let mut duel = Duel::new(&_battle, 14);
            duel.ship.ship_mut().special_counter = 1;
            duel.ship.ship_mut().ship_input_state = special;
            duel.run();
            assert_eq!(duel.input(), 0, "cooling down");

            drop(duel);
            let mut duel = Duel::new(&_battle, 14);
            duel.enemy.element_mut().__bindgen_anon_2.crew_level = 1;
            duel.ship.ship_mut().ship_input_state = special;
            duel.run();
            assert_eq!(duel.input(), 0, "nothing to take from a last crew member");

            drop(duel);
            let mut duel = Duel::new(&_battle, 14);
            duel.enemy.descriptor_mut().ship_info.ship_flags = CREW_IMMUNE as u16;
            duel.ship.ship_mut().ship_input_state = special;
            duel.run();
            assert_eq!(duel.input(), 0, "crew immune");

            // Without a target the request is left alone.
            drop(duel);
            let mut duel = Duel::new(&_battle, 14);
            duel.concerns[ENEMY_SHIP_INDEX as usize].ObjectPtr = ptr::null_mut();
            duel.ship.ship_mut().ship_input_state = special;
            duel.run();
            assert_eq!(duel.input(), special);
        }
    })
}
