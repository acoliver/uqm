// Mycon Podship - Homing plasmoid + Crew regeneration
// @plan PLAN-20260314-SHIPS.P13

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct MyconShip;

impl ShipBehavior for MyconShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::SEEKING_WEAPON,
                ship_cost: 21,
                crew_level: 20,
                max_crew: 20,
                energy_level: 40,
                max_energy: 40,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 194,
                known_loc: (6392, 2200),
            },
            characteristics: Characteristics {
                max_thrust: 27,
                thrust_increment: 9,
                energy_regeneration: 1,
                weapon_energy_cost: 20,
                special_energy_cost: 40, // MAX_ENERGY
                energy_wait: 4,
                turn_wait: 6,
                thrust_wait: 6,
                weapon_wait: 5,
                special_wait: 0,
                ship_mass: 7,
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 3200, // DISPLAY_TO_WORLD(800)
            },
        }
    }

    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        // Homing plasmoid - DISPLAY_TO_WORLD(8) = 32, life = 11*13 = 143
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (32, 0), // speed=32
            life_span: 143,
            hit_points: 1,
            damage: 10,
            mass: 4,
        }])
    }

    fn intelligence(&mut self, _ship: &ShipState, _ctx: &BattleContext) -> StatusFlags {
        // Race-specific AI deferred: depends on ship_intelligence() from cyborg.c (battle engine scope)
        StatusFlags::THRUST
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mycon_descriptor() {
        let ship = MyconShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 21);
        assert_eq!(desc.ship_info.crew_level, 20);
        assert_eq!(desc.ship_info.max_crew, 20);
        assert_eq!(desc.ship_info.energy_level, 40);
        assert_eq!(desc.ship_info.max_energy, 40);
        assert_eq!(desc.characteristics.max_thrust, 27);
        assert_eq!(desc.fleet.strength, 194);
        assert_eq!(desc.intel.weapon_range, 3200);
    }

    #[test]
    fn test_mycon_weapon() {
        let mut ship = MyconShip;
        let state = ShipState {
            crew_level: 0,
            max_crew: 0,
            energy_level: 0,
            max_energy: 0,
            ship_facing: 0,
            cur_status_flags: StatusFlags::empty(),
            old_status_flags: StatusFlags::empty(),
            player_nr: 0,
            position: (0, 0),
            velocity: (0, 0),
        };
        let context = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };
        let weapons = ship.init_weapon(&state, &context).unwrap();

        assert_eq!(weapons.len(), 1);
        assert_eq!(weapons[0].damage, 10);
    }
}
