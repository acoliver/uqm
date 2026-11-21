//! Mycon native entry point and callbacks linked against the engine archive.
#![forbid(unsafe_code)]

#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
use battle::*;

use uqm_engine::battle::ai_constants::{FIRES_FORE, SEEKING_WEAPON, SPECIAL};
use uqm_engine::battle::ElementId;
use uqm_engine::graphics::geometry::Point;
use uqm_engine::species::species_id::SPECIES_ID_MYCON_ID;

#[test]
fn descriptor_and_plasmoid_callbacks_match_mycon_c_data() {
    battle_context::run(|_| {
        let d = descriptor(SPECIES_ID_MYCON_ID);
        assert_eq!(d.ship_info.ship_flags, (FIRES_FORE | SEEKING_WEAPON) as u16);
        assert_eq!(d.ship_info.ship_cost, 21);
        assert_eq!((d.ship_info.crew_level, d.ship_info.max_crew), (20, 20));
        assert_eq!((d.ship_info.energy_level, d.ship_info.max_energy), (40, 40));
        assert_eq!(d.ship_info.race_strings_rsc.unwrap(), c"ship.mycon.text");
        assert_eq!(d.ship_info.icons_rsc.unwrap(), c"ship.mycon.icons");
        assert_eq!(
            d.ship_info.melee_icon_rsc.unwrap(),
            c"ship.mycon.meleeicons"
        );
        assert_eq!(d.fleet.strength, 194);
        assert_eq!((d.fleet.known_loc.x, d.fleet.known_loc.y), (6392, 2200));
        assert_eq!(
            (
                d.characteristics.max_thrust,
                d.characteristics.thrust_increment
            ),
            (27, 9)
        );
        assert_eq!(
            (
                d.characteristics.weapon_energy_cost,
                d.characteristics.special_energy_cost
            ),
            (20, 40)
        );
        assert_eq!(
            (
                d.characteristics.energy_wait,
                d.characteristics.turn_wait,
                d.characteristics.thrust_wait,
                d.characteristics.weapon_wait,
                d.characteristics.special_wait,
                d.characteristics.ship_mass
            ),
            (4, 6, 6, 5, 0, 7)
        );
        assert_eq!(
            d.ship_data.ship_rsc[0].unwrap(),
            c"ship.mycon.graphics.podship.large"
        );
        assert_eq!(
            d.ship_data.weapon_rsc[2].unwrap(),
            c"ship.mycon.graphics.plasma.small"
        );
        assert!(d.ship_data.special_rsc.iter().all(|r| r.is_none()));
        assert_eq!(d.cyborg_control.weapon_range, 3200);
        assert!(d.callbacks()[4] != 0);
        assert!(d.callbacks()[2] != 0);
        assert!(d.callbacks()[3] != 0);
        assert!(d.callbacks()[1] == 0);
    })
}

#[test]
fn plasma_launch_and_postprocess_boundaries_match_c() {
    battle_context::run(|_| {
        let _battle = Battle::new(8);
        let mut fixture = Ship::bare(&_battle, SPECIES_ID_MYCON_ID, 0);
        let ship = fixture.id;
        battle::get_mut(ship).next.location = Point { x: 100, y: 200 };
        let slots = fixture.weapon(1);
        let handles: [ElementId; 1] = [slots[0].expect("allocated weapon")];
        let plasma = handles[0];
        assert_eq!(battle::get(plasma).life, 143);
        assert_eq!(battle::get(plasma).hits, 10);
        assert_eq!(battle::get(plasma).mass, 10);
        assert_eq!(battle::get(plasma).turn, 3);
        assert_eq!(battle::get(plasma).thrust, 0);
        battle::get_mut(plasma).next = battle::get(plasma).current.clone();
        battle::get_mut(plasma).life = 100;
        hooks::invoke_element(battle::get(plasma).preprocess_func.unwrap(), plasma);
        assert_eq!(battle::get(plasma).hits, 7);
        assert_eq!(battle::get(plasma).mass, 7);
        assert_eq!(battle::get(plasma).turn, 2);
        battle::get_mut(plasma).hits = 3;
        hooks::invoke_element(battle::get(plasma).preprocess_func.unwrap(), plasma);
        assert_eq!(battle::get(plasma).life, 39);
        assert_eq!(battle::get(plasma).mass, 3);
        assert_eq!(battle::get(plasma).turn, 1);
        battle::get_mut(ship).hits = 17;
        fixture.ship_mut().cur_status_flags = SPECIAL as u16;
        fixture.postprocess();
        assert_eq!(battle::get(ship).hits, 20);
        assert_eq!(fixture.descriptor_mut().ship_info.energy_level, 0);
        assert_eq!(fixture.ship_mut().special_counter, 0);

        battle::get_mut(ship).hits = 10;
        fixture.descriptor_mut().ship_info.energy_level = 39;
        fixture.postprocess();
        assert_eq!(battle::get(ship).hits, 10);
        assert_eq!(fixture.descriptor_mut().ship_info.energy_level, 39);
        battle_context::dispose(handles[0]);
    })
}

#[path = "support/hooks.rs"]
mod hooks;
