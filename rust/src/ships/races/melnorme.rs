// Melnorme Trader - Charge-up pumpup shot + Confusion pulse
// @plan PLAN-20260314-SHIPS.P13

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct MelnormeShip;

impl ShipBehavior for MelnormeShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE,
                ship_cost: 18,
                crew_level: 20,
                max_crew: 20,
                energy_level: 42,
                max_energy: 42,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 0xFFFF,
                known_loc: (4999, 4999),
            },
            characteristics: Characteristics {
                max_thrust: 36,
                thrust_increment: 6,
                energy_regeneration: 1,
                weapon_energy_cost: 5,
                special_energy_cost: 20,
                energy_wait: 4,
                turn_wait: 4,
                thrust_wait: 4,
                weapon_wait: 1,
                special_wait: 20,
                ship_mass: 7,
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 1800, // PUMPUP_SPEED * PUMPUP_LIFE = 180 * 10
            },
        }
    }

    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        // Pumpup shot - DISPLAY_TO_WORLD(45) = 180
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (180, 0), // speed=180
            life_span: 10,
            hit_points: 1,
            damage: 2,
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
    fn test_melnorme_descriptor() {
        let ship = MelnormeShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 18);
        assert_eq!(desc.ship_info.crew_level, 20);
        assert_eq!(desc.ship_info.max_crew, 20);
        assert_eq!(desc.ship_info.energy_level, 42);
        assert_eq!(desc.ship_info.max_energy, 42);
        assert_eq!(desc.characteristics.max_thrust, 36);
        assert_eq!(desc.fleet.strength, 0xFFFF);
        assert_eq!(desc.intel.weapon_range, 1800);
    }

    #[test]
    fn test_melnorme_weapon() {
        let mut ship = MelnormeShip;
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
        assert_eq!(weapons[0].velocity.0, 180);
    }
}
