// ZoqFotPik Stinger - Anti-matter spray + Tongue grab attack
// @plan PLAN-20260314-SHIPS.P13

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct ZoqfotpikShip;

impl ShipBehavior for ZoqfotpikShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE,
                ship_cost: 6,
                crew_level: 10,
                max_crew: 10,
                energy_level: 10,
                max_energy: 10,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 58,
                known_loc: (3761, 5333),
            },
            characteristics: Characteristics {
                max_thrust: 40,
                thrust_increment: 10,
                energy_regeneration: 1,
                weapon_energy_cost: 1,
                special_energy_cost: 7, // MAX_ENERGY*3/4 = 10*3/4 integer = 7
                energy_wait: 4,
                turn_wait: 1,
                thrust_wait: 0,
                weapon_wait: 0,
                special_wait: 6,
                ship_mass: 5,
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 400, // MISSILE_SPEED * MISSILE_LIFE = 40 * 10
            },
        }
    }

    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        // Anti-matter spray - DISPLAY_TO_WORLD(10) = 40
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (40, 0), // speed=40
            life_span: 10,
            hit_points: 1,
            damage: 1,
            mass: 1,
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
    fn test_zoqfotpik_descriptor() {
        let ship = ZoqfotpikShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 6);
        assert_eq!(desc.ship_info.crew_level, 10);
        assert_eq!(desc.ship_info.max_crew, 10);
        assert_eq!(desc.ship_info.energy_level, 10);
        assert_eq!(desc.ship_info.max_energy, 10);
        assert_eq!(desc.characteristics.max_thrust, 40);
        assert_eq!(desc.fleet.strength, 58);
        assert_eq!(desc.intel.weapon_range, 400);
    }

    #[test]
    fn test_zoqfotpik_weapon() {
        let mut ship = ZoqfotpikShip;
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
        assert_eq!(weapons[0].damage, 1);
    }
}
