// Druuge Mauler - Mass driver cannon + crew furnace
// @plan PLAN-20260314-SHIPS.P11

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct DruugeShip;

impl ShipBehavior for DruugeShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE,
                ship_cost: 17,
                crew_level: 14,
                max_crew: 14,
                energy_level: 32,
                max_energy: 32,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 254,
                known_loc: (9500, 2792),
            },
            characteristics: Characteristics {
                max_thrust: 20,
                thrust_increment: 2,
                energy_regeneration: 1,
                weapon_energy_cost: 4,
                special_energy_cost: 16,
                energy_wait: 50,
                turn_wait: 4,
                thrust_wait: 1,
                weapon_wait: 10,
                special_wait: 30,
                ship_mass: 5,
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 2400,
            },
        }
    }

    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        // Mass driver cannon
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (120, 0), // speed=120
            life_span: 20,
            hit_points: 4,
            damage: 6,
            mass: 0,
        }])
    }

    fn intelligence(&mut self, _ship: &ShipState, _ctx: &BattleContext) -> StatusFlags {
        StatusFlags::THRUST
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_template_matches_c() {
        let ship = DruugeShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 17);
        assert_eq!(desc.ship_info.max_crew, 14);
        assert_eq!(desc.ship_info.max_energy, 32);
        assert_eq!(desc.characteristics.max_thrust, 20);
        assert_eq!(desc.fleet.strength, 254);
        assert_eq!(desc.intel.weapon_range, 2400);
    }

    #[test]
    fn weapon_basic() {
        let mut ship = DruugeShip::default();
        let state = ShipState {
            crew_level: 14,
            max_crew: 14,
            energy_level: 32,
            max_energy: 32,
            ship_facing: 4,
            cur_status_flags: StatusFlags::empty(),
            old_status_flags: StatusFlags::empty(),
            player_nr: 0,
            position: (100, 100),
            velocity: (0, 0),
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        let weapons = ship.init_weapon(&state, &ctx).unwrap();
        assert_eq!(weapons.len(), 1);
        assert_eq!(weapons[0].damage, 6);
        assert_eq!(weapons[0].hit_points, 4);
        assert_eq!(weapons[0].life_span, 20);
    }

    #[test]
    fn ai_basic() {
        let mut ship = DruugeShip::default();
        let state = ShipState {
            crew_level: 14,
            max_crew: 14,
            energy_level: 32,
            max_energy: 32,
            ship_facing: 0,
            cur_status_flags: StatusFlags::empty(),
            old_status_flags: StatusFlags::empty(),
            player_nr: 1,
            position: (0, 0),
            velocity: (0, 0),
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        let flags = ship.intelligence(&state, &ctx);
        assert!(flags.contains(StatusFlags::THRUST));
    }
}
