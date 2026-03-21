// SIS Flagship - Configurable loadout (campaign modules determine weapons)
// @plan PLAN-20260314-SHIPS.P13

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct SisShip;

impl ShipBehavior for SisShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::empty(), // no flags
                ship_cost: 16,
                crew_level: 42,
                max_crew: 42,
                energy_level: 42,
                max_energy: 42,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 0,
                known_loc: (0, 0),
            },
            characteristics: Characteristics {
                max_thrust: 10,
                thrust_increment: 4,
                energy_regeneration: 1,
                weapon_energy_cost: 1,
                special_energy_cost: 0,
                energy_wait: 10,
                turn_wait: 17,
                thrust_wait: 6,
                weapon_wait: 6,
                special_wait: 9,
                ship_mass: 10,
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 1152, // BLASTER_SPEED * BLASTER_LIFE = 96 * 12
            },
        }
    }

    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        // Ion bolt blaster - DISPLAY_TO_WORLD(24) = 96
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (96, 0), // speed=96
            life_span: 12,
            hit_points: 2,
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
    fn test_sis_descriptor() {
        let ship = SisShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 16);
        assert_eq!(desc.ship_info.crew_level, 42);
        assert_eq!(desc.ship_info.max_crew, 42);
        assert_eq!(desc.ship_info.energy_level, 42);
        assert_eq!(desc.ship_info.max_energy, 42);
        assert_eq!(desc.characteristics.max_thrust, 10);
        assert_eq!(desc.fleet.strength, 0);
        assert_eq!(desc.intel.weapon_range, 1152);
    }

    #[test]
    fn test_sis_flags() {
        let ship = SisShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_flags, ShipFlags::empty());
    }

    #[test]
    fn test_sis_weapon() {
        let mut ship = SisShip;
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
        assert_eq!(weapons[0].damage, 2);
    }
}
