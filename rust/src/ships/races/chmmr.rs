// Chmmr Avatar - Photon crystal laser + tractor beam + ZapSat satellites
// @plan PLAN-20260314-SHIPS.P13

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct ChmmrShip;

impl ShipBehavior for ChmmrShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE
                    | ShipFlags::IMMEDIATE_WEAPON
                    | ShipFlags::SEEKING_SPECIAL
                    | ShipFlags::POINT_DEFENSE,
                ship_cost: 30,
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
                max_thrust: 35,
                thrust_increment: 7,
                energy_regeneration: 1,
                weapon_energy_cost: 2,
                special_energy_cost: 1,
                energy_wait: 1,
                turn_wait: 3,
                thrust_wait: 5,
                weapon_wait: 0,
                special_wait: 0,
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
        // Photon crystal laser - immediate weapon
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (0, 0), // immediate
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
    fn test_chmmr_descriptor() {
        let ship = ChmmrShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 30);
        assert_eq!(desc.ship_info.crew_level, 42);
        assert_eq!(desc.ship_info.max_crew, 42);
        assert_eq!(desc.ship_info.energy_level, 42);
        assert_eq!(desc.ship_info.max_energy, 42);
        assert_eq!(desc.characteristics.max_thrust, 35);
        assert_eq!(desc.fleet.strength, 0);
        assert_eq!(desc.intel.weapon_range, 200);
    }

    #[test]
    fn test_chmmr_flags() {
        let ship = ChmmrShip;
        let desc = ship.descriptor_template();

        assert!(desc.ship_info.ship_flags.contains(ShipFlags::FIRES_FORE));
        assert!(desc
            .ship_info
            .ship_flags
            .contains(ShipFlags::IMMEDIATE_WEAPON));
        assert!(desc
            .ship_info
            .ship_flags
            .contains(ShipFlags::SEEKING_SPECIAL));
        assert!(desc.ship_info.ship_flags.contains(ShipFlags::POINT_DEFENSE));
    }

    #[test]
    fn test_chmmr_weapon() {
        let mut ship = ChmmrShip;
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
