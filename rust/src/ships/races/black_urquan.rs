// Kohr-Ah Marauder - Spinning blade boomerang + F.R.I.E.D. ring
// @plan PLAN-20260314-SHIPS.P13

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct BlackUrquanShip;

impl ShipBehavior for BlackUrquanShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE,
                ship_cost: 30,
                crew_level: 42,
                max_crew: 42,
                energy_level: 42,
                max_energy: 42,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 484,
                known_loc: (6000, 6250),
            },
            characteristics: Characteristics {
                max_thrust: 30,
                thrust_increment: 6,
                energy_regeneration: 1,
                weapon_energy_cost: 6,
                special_energy_cost: 21, // MAX_ENERGY_SIZE/2
                energy_wait: 4,
                turn_wait: 4,
                thrust_wait: 6,
                weapon_wait: 6,
                special_wait: 9,
                ship_mass: 10,
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 200, // CLOSE_RANGE_WEAPON
            },
        }
    }

    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        // Spinning blade - raw speed (NOT DISPLAY_TO_WORLD)
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (64, 0), // speed=64
            life_span: 64,
            hit_points: 10,
            damage: 4,
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
    fn test_black_urquan_descriptor() {
        let ship = BlackUrquanShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 30);
        assert_eq!(desc.ship_info.crew_level, 42);
        assert_eq!(desc.ship_info.max_crew, 42);
        assert_eq!(desc.ship_info.energy_level, 42);
        assert_eq!(desc.ship_info.max_energy, 42);
        assert_eq!(desc.characteristics.max_thrust, 30);
        assert_eq!(desc.fleet.strength, 484);
        assert_eq!(desc.intel.weapon_range, 200);
    }

    #[test]
    fn test_black_urquan_weapon() {
        let mut ship = BlackUrquanShip;
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
        assert_eq!(weapons[0].damage, 4);
        assert_eq!(weapons[0].hit_points, 10);
    }
}
