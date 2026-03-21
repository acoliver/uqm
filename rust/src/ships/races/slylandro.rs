// Slylandro Probe - Lightning bolt + Harvest energy from debris
// @plan PLAN-20260314-SHIPS.P13

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct SlylandroShip;

impl ShipBehavior for SlylandroShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::SEEKING_WEAPON | ShipFlags::CREW_IMMUNE,
                ship_cost: 17,
                crew_level: 12,
                max_crew: 12,
                energy_level: 20,
                max_energy: 20,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 0xFFFF,
                known_loc: (333, 9812),
            },
            characteristics: Characteristics {
                max_thrust: 60,
                thrust_increment: 60, // MAX_THRUST
                energy_regeneration: 0,
                weapon_energy_cost: 2,
                special_energy_cost: 0,
                energy_wait: 10,
                turn_wait: 0,
                thrust_wait: 0,
                weapon_wait: 17,
                special_wait: 20,
                ship_mass: 1,
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 400, // CLOSE_RANGE_WEAPON << 1 = 200 * 2
            },
        }
    }

    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        // Lightning bolt - immediate-like weapon
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (0, 0), // immediate-like
            life_span: 1,
            hit_points: 1,
            damage: 1,
            mass: 0,
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
    fn test_slylandro_descriptor() {
        let ship = SlylandroShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 17);
        assert_eq!(desc.ship_info.crew_level, 12);
        assert_eq!(desc.ship_info.max_crew, 12);
        assert_eq!(desc.ship_info.energy_level, 20);
        assert_eq!(desc.ship_info.max_energy, 20);
        assert_eq!(desc.characteristics.max_thrust, 60);
        assert_eq!(desc.fleet.strength, 0xFFFF);
        assert_eq!(desc.intel.weapon_range, 400);
    }

    #[test]
    fn test_slylandro_flags() {
        let ship = SlylandroShip;
        let desc = ship.descriptor_template();

        assert!(desc
            .ship_info
            .ship_flags
            .contains(ShipFlags::SEEKING_WEAPON));
        assert!(desc.ship_info.ship_flags.contains(ShipFlags::CREW_IMMUNE));
    }

    #[test]
    fn test_slylandro_weapon() {
        let mut ship = SlylandroShip;
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
    }
}
