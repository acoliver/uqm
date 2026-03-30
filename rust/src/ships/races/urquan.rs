// Ur-Quan Dreadnought - Fusion blast + Autonomous fighter launch
// @plan PLAN-20260314-SHIPS.P13

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct UrquanShip;

impl ShipBehavior for UrquanShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::SEEKING_SPECIAL,
                ship_cost: 30,
                crew_level: 42,
                max_crew: 42,
                energy_level: 42,
                max_energy: 42,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 484,
                known_loc: (5750, 6000),
            },
            characteristics: Characteristics {
                max_thrust: 30,
                thrust_increment: 6,
                energy_regeneration: 1,
                weapon_energy_cost: 6,
                special_energy_cost: 8,
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
                weapon_range: 1600, // MISSILE_SPEED * MISSILE_LIFE = 80 * 20
            },
        }
    }

    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        // Fusion blast - DISPLAY_TO_WORLD(20) = 80
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (80, 0), // speed=80
            life_span: 20,
            hit_points: 10,
            damage: 6,
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
    fn test_urquan_descriptor() {
        let ship = UrquanShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 30);
        assert_eq!(desc.ship_info.crew_level, 42);
        assert_eq!(desc.ship_info.max_crew, 42);
        assert_eq!(desc.ship_info.energy_level, 42);
        assert_eq!(desc.ship_info.max_energy, 42);
        assert_eq!(desc.characteristics.max_thrust, 30);
        assert_eq!(desc.fleet.strength, 484);
        assert_eq!(desc.intel.weapon_range, 1600);
    }

    #[test]
    fn test_urquan_weapon() {
        let mut ship = UrquanShip;
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
            velocity: (0, 0), ..ShipState::default()
        };
        let context = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };
        let weapons = ship.init_weapon(&state, &context).unwrap();

        assert_eq!(weapons.len(), 1);
        assert_eq!(weapons[0].damage, 6);
        assert_eq!(weapons[0].hit_points, 10);
    }
}
