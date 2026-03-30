// Ilwrath Avenger - Hellfire spout + cloaking device
// @plan PLAN-20260314-SHIPS.P11

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct IlwrathShip;

impl ShipBehavior for IlwrathShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE,
                ship_cost: 10,
                crew_level: 22,
                max_crew: 22,
                energy_level: 16,
                max_energy: 16,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 256,
                known_loc: (48, 1700),
            },
            characteristics: Characteristics {
                max_thrust: 25,
                thrust_increment: 5,
                energy_regeneration: 4,
                weapon_energy_cost: 1,
                special_energy_cost: 3,
                energy_wait: 4,
                turn_wait: 2,
                thrust_wait: 0,
                weapon_wait: 0,
                special_wait: 13,
                ship_mass: 7,
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 200,
            },
        }
    }

    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        // Hellfire spout
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (25, 0), // speed=25 (matches MAX_THRUST)
            life_span: 8,
            hit_points: 1,
            damage: 1,
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
        let ship = IlwrathShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 10);
        assert_eq!(desc.ship_info.max_crew, 22);
        assert_eq!(desc.ship_info.max_energy, 16);
        assert_eq!(desc.characteristics.max_thrust, 25);
        assert_eq!(desc.fleet.strength, 256);
        assert_eq!(desc.intel.weapon_range, 200);
    }

    #[test]
    fn weapon_basic() {
        let mut ship = IlwrathShip::default();
        let state = ShipState {
            crew_level: 22,
            max_crew: 22,
            energy_level: 16,
            max_energy: 16,
            ship_facing: 4,
            cur_status_flags: StatusFlags::empty(),
            old_status_flags: StatusFlags::empty(),
            player_nr: 0,
            position: (100, 100),
            velocity: (0, 0), ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        let weapons = ship.init_weapon(&state, &ctx).unwrap();
        assert_eq!(weapons.len(), 1);
        assert_eq!(weapons[0].damage, 1);
        assert_eq!(weapons[0].life_span, 8);
    }

    #[test]
    fn ai_basic() {
        let mut ship = IlwrathShip::default();
        let state = ShipState {
            crew_level: 22,
            max_crew: 22,
            energy_level: 16,
            max_energy: 16,
            ship_facing: 0,
            cur_status_flags: StatusFlags::empty(),
            old_status_flags: StatusFlags::empty(),
            player_nr: 1,
            position: (0, 0),
            velocity: (0, 0), ..ShipState::default()
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
