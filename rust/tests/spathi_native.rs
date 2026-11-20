//! Exercise the Spathi live descriptor and registered callbacks against the linked engine.
#![forbid(unsafe_code)]

#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
use battle::{Battle, FrameSet, Ship};
use uqm_engine::battle::ai_constants::{
    DONT_CHASE, ENEMY_SHIP_INDEX, FIRES_AFT, FIRES_FORE, IGNORE_SIMILAR, SEEKING_SPECIAL, SPECIAL,
};
use uqm_engine::battle::flags::{
    APPEARING, CHANGING, FINITE_LIFE, SHIP_BEYOND_MAX_SPEED, SHIP_IN_GRAVITY_WELL,
};
use uqm_engine::battle::Concern;
use uqm_engine::battle::ElementId;
use uqm_engine::graphics::drawable::FrameId;
use uqm_engine::graphics::geometry::Point;
use uqm_engine::species::species_id::SPECIES_ID_SPATHI_ID;

/// Mirrors `FRAME_DESC` from `drawable.h` so the engine's frame functions can
/// walk a drawable built here.
fn index_of(frame: FrameId) -> u16 {
    frame.index()
}
fn queue_tail() -> ElementId {
    battle::tail().unwrap_or_else(|| panic!("queued element"))
}
#[test]
fn eluder_descriptor_matches_the_c_template() {
    battle_context::run(|_| {
        {
            let d = battle::descriptor(SPECIES_ID_SPATHI_ID);
            let desc = &d;
            assert_eq!(
                desc.ship_info.ship_flags,
                (FIRES_FORE | FIRES_AFT | SEEKING_SPECIAL | DONT_CHASE) as u16
            );
            assert_eq!(desc.ship_info.ship_cost, 18);
            assert_eq!(
                (desc.ship_info.crew_level, desc.ship_info.max_crew),
                (30, 30)
            );
            assert_eq!(
                (desc.ship_info.energy_level, desc.ship_info.max_energy),
                (10, 10)
            );
            assert_eq!(
                desc.ship_info.race_strings_rsc.unwrap(),
                c"ship.spathi.text"
            );
            assert_eq!(desc.ship_info.icons_rsc.unwrap(), c"ship.spathi.icons");
            assert_eq!(
                desc.ship_info.melee_icon_rsc.unwrap(),
                c"ship.spathi.meleeicons"
            );
            // 1000 / SPHERE_RADIUS_INCREMENT * 2
            assert_eq!(desc.fleet.strength, 180);
            assert_eq!(
                (desc.fleet.known_loc.x, desc.fleet.known_loc.y),
                (2549, 3600)
            );
            let c = &desc.characteristics;
            assert_eq!(
                (c.max_thrust, c.thrust_increment, c.energy_regeneration),
                (48, 12, 1)
            );
            assert_eq!(
                (c.weapon_energy_cost, c.special_energy_cost, c.energy_wait),
                (2, 3, 10)
            );
            assert_eq!((c.turn_wait, c.thrust_wait, c.weapon_wait), (1, 1, 0));
            assert_eq!((c.special_wait, c.ship_mass), (7, 5));

            let data = &desc.ship_data;
            let expected_ship = [
                c"ship.spathi.graphics.eluder.large",
                c"ship.spathi.graphics.eluder.medium",
                c"ship.spathi.graphics.eluder.small",
            ];
            let expected_weapon = [
                c"ship.spathi.graphics.missile.large",
                c"ship.spathi.graphics.missile.medium",
                c"ship.spathi.graphics.missile.small",
            ];
            let expected_special = [
                c"ship.spathi.graphics.butt.large",
                c"ship.spathi.graphics.butt.medium",
                c"ship.spathi.graphics.butt.small",
            ];
            for i in 0..3 {
                assert_eq!(data.ship_rsc[i].unwrap(), expected_ship[i]);
                assert_eq!(data.weapon_rsc[i].unwrap(), expected_weapon[i]);
                assert_eq!(data.special_rsc[i].unwrap(), expected_special[i]);
                assert!(data.ship.view(i).is_absent());
                assert!(data.weapon.view(i).is_absent());
                assert!(data.special.view(i).is_absent());
            }
            assert_eq!(
                data.captain_control.captain_rsc.unwrap(),
                c"ship.spathi.graphics.captain"
            );
            assert_eq!(data.victory_ditty_rsc.unwrap(), c"ship.spathi.ditty");
            assert_eq!(data.ship_sounds_rsc.unwrap(), c"ship.spathi.sounds");

            // MISSILE_SPEED * MISSILE_LIFE: the cyborg's weapon range.
            assert_eq!(desc.cyborg_control.maneuverability_index, 0);
            assert_eq!(desc.cyborg_control.weapon_range, 1200);
            assert!(desc.race.unwrap().intelligence.is_some());
            assert!(desc.race.unwrap().postprocess.is_some());
            assert!(desc.race.unwrap().init_weapon.is_some());
            assert!(desc.race.unwrap().preprocess.is_none());
            assert!(desc.race.unwrap().uninit.is_none());
        }
    })
}

#[test]
fn forward_missile_leaves_the_nose_along_the_ship_facing() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            let mut fleet = Ship::new(&_battle, SPECIES_ID_SPATHI_ID, 0);
            let missile_frames = FrameSet::new(16);
            fleet.descriptor_mut().ship_data.weapon = missile_frames.array();
            fleet.ship_mut().player_nr = 0;
            battle::get_mut(fleet.id).player_nr = 0;
            battle::get_mut(fleet.id).current.location = Point { x: 400, y: 500 };
            battle::get_mut(fleet.id).next.location = Point { x: 400, y: 500 };
            let ship = fleet.id;
            fleet.ship_mut().ship_facing = 4;

            let mut out = [None];
            assert_eq!(
                {
                    out[0] = fleet.initialize_from(ship, 1)[0];
                    1
                },
                1
            );
            let m = out[0].unwrap();
            assert_eq!(battle::get(m).life, 10);
            assert_eq!(battle::get(m).hits, 1);
            assert_eq!(battle::get(m).mass, 1);
            assert_eq!(battle::get(m).thrust, 1);
            assert_eq!(battle::get(m).player_nr, 0);
            assert_eq!(
                battle::get(m).state_flags,
                (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
            );
            assert!(battle::get(m).preprocess_func.is_none());
            assert_eq!(index_of(battle::get(m).current.image.frame.raw()), 4);
            // Facing 4 is angle 16, along +x (angle 0 points up the screen): the nose offset of
            // 16 display pixels, pulled back by one frame of travel at speed 120.
            assert_eq!(battle::get(m).velocity.travel_angle, 16);
            assert_eq!(
                (
                    battle::get(m).current.location.x,
                    battle::get(m).current.location.y
                ),
                (400 + 16 * 4 - 120, 500)
            );
        }
    })
}

#[test]
fn butt_leaves_the_tail_not_the_nose() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            let mut fleet = Ship::new(&_battle, SPECIES_ID_SPATHI_ID, 0);
            let butt_frames = FrameSet::new(16);
            fleet.descriptor_mut().ship_data.special = butt_frames.array();
            fleet.ship_mut().player_nr = 0;
            battle::get_mut(fleet.id).player_nr = 0;
            battle::get_mut(fleet.id).current.location = Point { x: 400, y: 500 };
            battle::get_mut(fleet.id).next.location = Point { x: 400, y: 500 };
            let ship = fleet.id;
            fleet.ship_mut().cur_status_flags = SPECIAL as u16;
            fleet.ship_mut().ship_facing = 4;

            fleet.postprocess();

            // ANGLE_TO_FACING (HALF_CIRCLE) is eight facings, so the butt flies
            // out at facing 12 (angle 48, along -x).
            let butt = queue_tail();
            assert_ne!(butt, ship);
            assert_eq!(
                battle::get(butt).parent,
                uqm_engine::battle::Parent::Ship(fleet.ship_id)
            );
            assert_eq!(index_of(battle::get(butt).current.image.frame.raw()), 12);
            assert_eq!(battle::get(butt).velocity.travel_angle, 48);
            assert_eq!(
                battle::get(butt).state_flags,
                (APPEARING | FINITE_LIFE) as u16
            );
            assert_eq!(battle::get(butt).life, 30);
            assert_eq!(battle::get(butt).hits, 1);
            assert_eq!(battle::get(butt).mass, 2);
            assert_eq!(battle::get(butt).turn, 1);
            assert_eq!(battle::get(butt).thrust, 4);
            assert!(battle::get(butt).preprocess_func.is_some());
            assert_eq!(
                (
                    battle::get(butt).current.location.x,
                    battle::get(butt).current.location.y
                ),
                (400 - 20 * 4 + 32, 500)
            );
            assert_eq!(fleet.descriptor_mut().ship_info.energy_level, 7);
            assert_eq!(fleet.ship_mut().special_counter, 7);
        }
    })
}

#[test]
fn butt_rear_facing_wraps_through_sixteen_facings() {
    battle_context::run(|_| {
        let _battle = Battle::new(16);
        let mut fleet = Ship::new(&_battle, SPECIES_ID_SPATHI_ID, 0);
        let butt_frames = FrameSet::new(16);
        fleet.descriptor_mut().ship_data.special = butt_frames.array();
        fleet.ship_mut().player_nr = 0;
        battle::get_mut(fleet.id).player_nr = 0;
        battle::get_mut(fleet.id).current.location = Point { x: 0, y: 0 };
        battle::get_mut(fleet.id).next.location = Point { x: 0, y: 0 };
        fleet.ship_mut().cur_status_flags = SPECIAL as u16;
        for (ship_facing, rear) in [(0, 8), (3, 11), (8, 0), (15, 7)] {
            fleet.ship_mut().ship_facing = ship_facing;
            fleet.ship_mut().special_counter = 0;
            fleet.descriptor_mut().ship_info.energy_level = 10;
            fleet.postprocess();
            let butt = queue_tail();
            assert_eq!(index_of(battle::get(butt).current.image.frame.raw()), rear);
        }
    })
}

#[test]
fn butt_needs_the_special_key_a_ready_counter_and_energy() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            let mut fleet = Ship::new(&_battle, SPECIES_ID_SPATHI_ID, 0);
            fleet.ship_mut().player_nr = 0;
            battle::get_mut(fleet.id).player_nr = 0;
            battle::get_mut(fleet.id).current.location = Point { x: 0, y: 0 };
            battle::get_mut(fleet.id).next.location = Point { x: 0, y: 0 };
            let tail = queue_tail();

            // Key not held.
            fleet.ship_mut().cur_status_flags = 0;
            fleet.postprocess();
            assert_eq!(queue_tail(), tail);
            assert_eq!(fleet.descriptor_mut().ship_info.energy_level, 10);

            // Counter still running.
            fleet.ship_mut().cur_status_flags = SPECIAL as u16;
            fleet.ship_mut().special_counter = 1;
            fleet.postprocess();
            assert_eq!(queue_tail(), tail);
            assert_eq!(fleet.descriptor_mut().ship_info.energy_level, 10);

            // Not enough energy for SPECIAL_ENERGY_COST (3).
            fleet.ship_mut().special_counter = 0;
            fleet.descriptor_mut().ship_info.energy_level = 2;
            fleet.postprocess();
            assert_eq!(queue_tail(), tail);
            assert_eq!(fleet.descriptor_mut().ship_info.energy_level, 2);
            assert_eq!(fleet.ship_mut().special_counter, 0);
        }
    })
}

#[test]
fn butt_homes_one_facing_step_per_tracking_pass() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            let mut spathi = Ship::new(&_battle, SPECIES_ID_SPATHI_ID, 0);
            let butt_frames = FrameSet::new(16);
            spathi.descriptor_mut().ship_data.special = butt_frames.array();
            spathi.ship_mut().player_nr = 0;
            battle::get_mut(spathi.id).player_nr = 0;
            battle::get_mut(spathi.id).current.location = Point { x: 400, y: 500 };
            battle::get_mut(spathi.id).next.location = Point { x: 400, y: 500 };
            spathi.ship_mut().cur_status_flags = SPECIAL as u16;
            spathi.ship_mut().ship_facing = 4;
            spathi.postprocess();
            let butt = queue_tail();
            // The engine starts each frame with `next` equal to `current`.
            battle::get_mut(butt).next = battle::get(butt).current.clone();
            let preprocess = battle::get(butt).preprocess_func.unwrap();

            // TRACK_WAIT: the first pass only counts the delay down.
            assert_eq!(battle::get(butt).turn, 1);
            hooks::invoke_element(preprocess, butt);
            assert_eq!(battle::get(butt).turn, 0);
            assert_eq!(index_of(battle::get(butt).next.image.frame.raw()), 12);
            assert_eq!(battle::get(butt).state_flags & CHANGING as u16, 0);

            // With no enemy to track, the delay is reloaded and nothing turns.
            hooks::invoke_element(preprocess, butt);
            assert_eq!(battle::get(butt).turn, 1);
            assert_eq!(index_of(battle::get(butt).next.image.frame.raw()), 12);
            assert_eq!(battle::get(butt).state_flags & CHANGING as u16, 0);

            // An enemy straight up the screen is four facings away from facing 12, so
            // one pass steps the butt to facing 13 and re-aims its velocity.
            let mut other = Ship::new(&_battle, SPECIES_ID_SPATHI_ID, 1);
            let enemy_x = battle::get(butt).current.location.x;
            let enemy_y = battle::get(butt).current.location.y - 1000;
            other.ship_mut().player_nr = 1;
            battle::get_mut(other.id).player_nr = 1;
            battle::get_mut(other.id).current.location = Point {
                x: enemy_x,
                y: enemy_y,
            };
            battle::get_mut(other.id).next.location = Point {
                x: enemy_x,
                y: enemy_y,
            };

            hooks::invoke_element(preprocess, butt);
            hooks::invoke_element(preprocess, butt);
            assert_eq!(index_of(battle::get(butt).next.image.frame.raw()), 13);
            assert_ne!(battle::get(butt).state_flags & CHANGING as u16, 0);
            assert_eq!(battle::get(butt).velocity.travel_angle, 52);
            assert_eq!(battle::get(butt).turn, 1);
        }
    })
}

/// Runs `spathi_intelligence` with no concerns for `ship_intelligence` to
/// act on, so only the Spathi decision is observed.
fn intelligence_sets_special(
    fleet: &mut Ship<'_>,
    ship: ElementId,
    target: Option<ElementId>,
    which_turn: u16,
) -> bool {
    let mut concerns: [Concern; 4] = [Concern::default(); 4];
    concerns[ENEMY_SHIP_INDEX as usize].object = target;
    concerns[ENEMY_SHIP_INDEX as usize].which_turn = which_turn;
    // A stale SPECIAL must always be cleared first.
    fleet.ship_mut().ship_input_state = SPECIAL as u8;
    fleet.think(ship, &mut concerns, 0);
    fleet.ship_mut().ship_input_state & SPECIAL as u8 != 0
}

#[test]
fn intelligence_fires_the_butt_only_at_targets_behind_the_ship() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            let mut fleet = Ship::new(&_battle, SPECIES_ID_SPATHI_ID, 0);
            fleet.ship_mut().player_nr = 0;
            battle::get_mut(fleet.id).player_nr = 0;
            battle::get_mut(fleet.id).current.location = Point { x: 2000, y: 2000 };
            battle::get_mut(fleet.id).next.location = Point { x: 2000, y: 2000 };
            let ship = fleet.id;
            let mut other = Ship::new(&_battle, SPECIES_ID_SPATHI_ID, 1);
            other.ship_mut().player_nr = 1;
            battle::get_mut(other.id).player_nr = 1;
            battle::get_mut(other.id).current.location = Point { x: 2000, y: 3000 };
            battle::get_mut(other.id).next.location = Point { x: 2000, y: 3000 };
            let behind = other.id;
            let ahead_fixture = Ship::new(&_battle, SPECIES_ID_SPATHI_ID, 1);
            battle::get_mut(ahead_fixture.id).current.location = Point { x: 2000, y: 1000 };
            battle::get_mut(ahead_fixture.id).next.location =
                battle::get(ahead_fixture.id).current.location;
            battle::get_mut(other.id).current.location = Point { x: 2000, y: 3000 };
            battle::get_mut(other.id).next.location = battle::get(other.id).current.location;
            let ahead = ahead_fixture.id;
            fleet.ship_mut().ship_facing = 0;

            assert!(intelligence_sets_special(&mut fleet, ship, Some(behind), 8));
            assert!(!intelligence_sets_special(&mut fleet, ship, Some(ahead), 8));
            assert!(!intelligence_sets_special(&mut fleet, ship, None, 8));

            // The enemy lies ahead of the ship's tail only after it turns around.
            fleet.ship_mut().ship_facing = 8;
            assert!(intelligence_sets_special(&mut fleet, ship, Some(ahead), 8));
            assert!(!intelligence_sets_special(
                &mut fleet,
                ship,
                Some(behind),
                8
            ));
        }
    })
}

#[test]
fn intelligence_respects_counter_and_arrival_time() {
    battle_context::run(|_| {
        let _battle = Battle::new(16);
        let mut fleet = Ship::new(&_battle, SPECIES_ID_SPATHI_ID, 0);
        fleet.ship_mut().player_nr = 0;
        battle::get_mut(fleet.id).player_nr = 0;
        battle::get_mut(fleet.id).current.location = Point { x: 2000, y: 2000 };
        battle::get_mut(fleet.id).next.location = Point { x: 2000, y: 2000 };
        let ship = fleet.id;
        let mut other = Ship::new(&_battle, SPECIES_ID_SPATHI_ID, 1);
        other.ship_mut().player_nr = 1;
        battle::get_mut(other.id).player_nr = 1;
        battle::get_mut(other.id).current.location = Point { x: 2000, y: 3000 };
        battle::get_mut(other.id).next.location = Point { x: 2000, y: 3000 };
        let behind = other.id;
        fleet.ship_mut().ship_facing = 0;

        assert!(intelligence_sets_special(
            &mut fleet,
            ship,
            Some(behind),
            24
        ));
        assert!(!intelligence_sets_special(
            &mut fleet,
            ship,
            Some(behind),
            25
        ));
        fleet.ship_mut().special_counter = 1;
        assert!(!intelligence_sets_special(
            &mut fleet,
            ship,
            Some(behind),
            8
        ));
    })
}

#[test]
fn intelligence_beyond_eight_turns_needs_the_enemy_to_be_closing() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            let mut fleet = Ship::new(&_battle, SPECIES_ID_SPATHI_ID, 0);
            fleet.ship_mut().player_nr = 0;
            battle::get_mut(fleet.id).player_nr = 0;
            battle::get_mut(fleet.id).current.location = Point { x: 2000, y: 2000 };
            battle::get_mut(fleet.id).next.location = Point { x: 2000, y: 2000 };
            let ship = fleet.id;
            let mut other = Ship::new(&_battle, SPECIES_ID_SPATHI_ID, 1);
            other.ship_mut().player_nr = 1;
            battle::get_mut(other.id).player_nr = 1;
            battle::get_mut(other.id).current.location = Point { x: 2000, y: 3000 };
            battle::get_mut(other.id).next.location = Point { x: 2000, y: 3000 };
            let behind = other.id;
            fleet.ship_mut().ship_facing = 0;

            // Enemy drifting up the screen (angle 0): facing 8 + 8 - 0 + 4 wraps to 4.
            battle::get_mut(behind).velocity.travel_angle = 0;
            assert!(intelligence_sets_special(
                &mut fleet,
                ship,
                Some(behind),
                20
            ));
            // Enemy drifting down the screen (angle 32): 8 + 8 - 8 + 4 is 12, past 8.
            battle::get_mut(behind).velocity.travel_angle = 32;
            assert!(!intelligence_sets_special(
                &mut fleet,
                ship,
                Some(behind),
                20
            ));
            // At eight turns or fewer the enemy's drift no longer matters.
            assert!(intelligence_sets_special(&mut fleet, ship, Some(behind), 8));
        }
    })
}

#[test]
fn intelligence_while_fast_or_in_gravity_needs_the_tail_along_travel() {
    battle_context::run(|_| {
        {
            let _battle = Battle::new(16);
            let mut fleet = Ship::new(&_battle, SPECIES_ID_SPATHI_ID, 0);
            fleet.ship_mut().player_nr = 0;
            battle::get_mut(fleet.id).player_nr = 0;
            battle::get_mut(fleet.id).current.location = Point { x: 2000, y: 2000 };
            battle::get_mut(fleet.id).next.location = Point { x: 2000, y: 2000 };
            let ship = fleet.id;
            let mut other = Ship::new(&_battle, SPECIES_ID_SPATHI_ID, 1);
            other.ship_mut().player_nr = 1;
            battle::get_mut(other.id).player_nr = 1;
            battle::get_mut(other.id).current.location = Point { x: 2000, y: 3000 };
            battle::get_mut(other.id).next.location = Point { x: 2000, y: 3000 };
            let behind = other.id;
            fleet.ship_mut().ship_facing = 0;

            for flag in [SHIP_BEYOND_MAX_SPEED, SHIP_IN_GRAVITY_WELL] {
                fleet.ship_mut().cur_status_flags = flag as u16;
                // Travelling up the screen (angle 0) puts the target astern.
                battle::get_mut(ship).velocity.travel_angle = 0;
                assert!(intelligence_sets_special(&mut fleet, ship, Some(behind), 8));
                // Travelling down the screen (angle 32) puts it ahead of the path.
                battle::get_mut(ship).velocity.travel_angle = 32;
                assert!(!intelligence_sets_special(
                    &mut fleet,
                    ship,
                    Some(behind),
                    8
                ));
            }
            // Slow ships ignore their travel direction.
            fleet.ship_mut().cur_status_flags = 0;
            battle::get_mut(ship).velocity.travel_angle = 32;
            assert!(intelligence_sets_special(&mut fleet, ship, Some(behind), 8));
        }
    })
}

#[path = "support/hooks.rs"]
mod hooks;
