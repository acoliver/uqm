// Ur-Quan Probe - Minimal autonomous probe with no combat capability
// @plan PLAN-20260314-SHIPS.P13

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct ProbeShip;

impl ShipBehavior for ProbeShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::empty(), // no flags
                ship_cost: 0,
                crew_level: 1,
                max_crew: 1,
                energy_level: 1,
                max_energy: 1,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 0,
                known_loc: (0, 0),
            },
            characteristics: Characteristics {
                max_thrust: 0,
                thrust_increment: 0,
                energy_regeneration: 0,
                weapon_energy_cost: 0,
                special_energy_cost: 0,
                energy_wait: 0,
                turn_wait: 0,
                thrust_wait: 0,
                weapon_wait: 0,
                special_wait: 0,
                ship_mass: 0,
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 0,
            },
        }
    }

    fn init_weapon(
        &mut self,
        _ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        // No weapon - returns empty vec
        Ok(vec![])
    }

    fn intelligence(&mut self, _ship: &ShipState, _ctx: &BattleContext) -> StatusFlags {
        // No AI logic needed for probe
        StatusFlags::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_descriptor() {
        let ship = ProbeShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 0);
        assert_eq!(desc.ship_info.crew_level, 1);
        assert_eq!(desc.ship_info.max_crew, 1);
        assert_eq!(desc.ship_info.energy_level, 1);
        assert_eq!(desc.ship_info.max_energy, 1);
        assert_eq!(desc.characteristics.max_thrust, 0);
        assert_eq!(desc.characteristics.ship_mass, 0);
        assert_eq!(desc.intel.weapon_range, 0);
    }

    #[test]
    fn test_probe_weapon() {
        let mut ship = ProbeShip;
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

        assert_eq!(weapons.len(), 0);
    }
}
