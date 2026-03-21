// Earthling Cruiser - Nuclear missile + point-defense laser
// @plan PLAN-20260314-SHIPS.P11

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct HumanShip;

impl ShipBehavior for HumanShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE
                    | ShipFlags::SEEKING_WEAPON
                    | ShipFlags::POINT_DEFENSE,
                ship_cost: 11,
                crew_level: 18,
                max_crew: 18,
                energy_level: 18,
                max_energy: 18,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 0,
                known_loc: (1752, 1450),
            },
            characteristics: Characteristics {
                max_thrust: 24,
                thrust_increment: 3,
                energy_regeneration: 1,
                weapon_energy_cost: 9,
                special_energy_cost: 4,
                energy_wait: 8,
                turn_wait: 1,
                thrust_wait: 4,
                weapon_wait: 10,
                special_wait: 9,
                ship_mass: 6,
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 4000,
            },
        }
    }

    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        // Nuclear missile
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (40, 0), // speed=40
            life_span: 60,
            hit_points: 1,
            damage: 4,
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
        let ship = HumanShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 11);
        assert_eq!(desc.ship_info.max_crew, 18);
        assert_eq!(desc.ship_info.max_energy, 18);
        assert_eq!(desc.characteristics.max_thrust, 24);
        assert_eq!(desc.fleet.strength, 0);
        assert_eq!(desc.intel.weapon_range, 4000);
    }

    #[test]
    fn weapon_basic() {
        let mut ship = HumanShip::default();
        let state = ShipState {
            crew_level: 18,
            max_crew: 18,
            energy_level: 18,
            max_energy: 18,
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
        assert_eq!(weapons[0].damage, 4);
        assert_eq!(weapons[0].life_span, 60);
    }

    #[test]
    fn ai_basic() {
        let mut ship = HumanShip::default();
        let state = ShipState {
            crew_level: 18,
            max_crew: 18,
            energy_level: 18,
            max_energy: 18,
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
