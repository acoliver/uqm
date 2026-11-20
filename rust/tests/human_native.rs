//! Exercise the Cruiser descriptor and Native callbacks against the linked engine.
#![forbid(unsafe_code)]

#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;

use battle::*;
use uqm_rust::engine::battle::ai_constants::{FIRES_FORE, POINT_DEFENSE, SEEKING_WEAPON, SPECIAL};
use uqm_rust::engine::battle::flags::{APPEARING, FINITE_LIFE, NONSOLID};
use uqm_rust::engine::species::species_id::SPECIES_ID_EARTHLING_ID;
use uqm_rust::graphics::geometry::Point;
use uqm_rust::math::velocity::VelocityDesc;

#[test]
fn cruiser_descriptor_and_registered_weapon_and_defense_callbacks() {
    battle_context::run(|_| {
        let battle = Battle::new(8);
        {
            let mut ship = Ship::new(&battle, SPECIES_ID_EARTHLING_ID, 0);
            let desc = descriptor(SPECIES_ID_EARTHLING_ID);
            assert_eq!(
                desc.ship_info.ship_flags,
                (FIRES_FORE | SEEKING_WEAPON | POINT_DEFENSE) as u16
            );
            assert_eq!(desc.ship_info.ship_cost, 11);
            assert_eq!(desc.ship_info.crew_level, 18);
            assert_eq!(desc.ship_info.max_crew, 18);
            assert_eq!(desc.ship_info.energy_level, 18);
            assert_eq!(desc.ship_info.max_energy, 18);
            assert_eq!(
                desc.ship_info.race_strings_rsc.unwrap(),
                c"ship.earthling.text"
            );
            assert_eq!(
                desc.ship_data.weapon_rsc[0].unwrap(),
                c"ship.earthling.graphics.saturn.large"
            );
            assert_eq!(desc.characteristics.special_energy_cost, 4);
            assert_eq!(desc.characteristics.special_wait, 9);
            assert_eq!(desc.cyborg_control.weapon_range, 4000);
            assert!(desc.race.is_some());
            assert_eq!(desc.callbacks()[0], 0);
            assert_eq!(desc.callbacks()[1], 0);
            assert_ne!(desc.callbacks()[2], 0);
            assert_ne!(desc.callbacks()[3], 0);
            assert_ne!(desc.callbacks()[4], 0);

            ship.element_mut().next.location = Point { x: 1000, y: 2000 };
            let weapons = ship.weapon(1);
            let nuke = weapons[0].expect("allocated weapon");
            assert_eq!(battle::get(nuke).life, 60);
            assert_eq!(battle::get(nuke).hits, 1);
            assert_eq!(battle::get(nuke).mass, 4);
            assert_eq!(battle::get(nuke).turn, 3);
            assert_eq!(battle::get(nuke).thrust, 8);
            for expected_wait in [2, 1, 0, 3] {
                call_element("ships::races::human_native::nuke_preprocess", nuke);
                assert_eq!(battle::get(nuke).turn, expected_wait);
            }
            battle::get_mut(nuke).life = 50;
            call_element("ships::races::human_native::nuke_preprocess", nuke);
            let mut expected: VelocityDesc = VelocityDesc::new();
            uqm_rust::battle::velocity::set_velocity_vector(&mut expected, 80, 0);
            assert_eq!(
                battle::get(nuke).velocity.vector.width,
                expected.vector.width
            );
            assert_eq!(
                battle::get(nuke).velocity.vector.height,
                expected.vector.height
            );
            battle_context::dispose(nuke);

            // Special creates a deferred scan only when pressed and off cooldown.
            let before = queued();
            ship.postprocess();
            assert_eq!(queued(), before);
            ship.ship_mut().cur_status_flags = SPECIAL as u16;
            ship.ship_mut().special_counter = 1;
            ship.postprocess();
            assert_eq!(queued(), before);
            ship.ship_mut().special_counter = 0;
            ship.postprocess();
            let defense = battle::tail().unwrap();
            assert_ne!(defense, ship.id);
            assert_eq!(
                battle::get(defense).state_flags,
                (APPEARING | NONSOLID | FINITE_LIFE) as u16
            );
            assert_eq!(battle::get(defense).parent, battle::get(ship.id).parent);
            assert!(battle::get(defense).death_func.is_some());
            call_element("ships::races::human_native::spawn_point_defense", defense);
            assert_eq!(
                ship.descriptor_mut().ship_info.energy_level,
                18,
                "no targets, no energy payment"
            );
            assert_eq!(ship.ship_mut().special_counter, 0);
        }
    })
}
