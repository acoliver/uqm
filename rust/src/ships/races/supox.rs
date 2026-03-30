// Supox Blade - Gob launcher + lateral/reverse thrust
// @plan PLAN-20260314-SHIPS.P11

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct SupoxShip;

impl ShipBehavior for SupoxShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE,
                ship_cost: 16,
                crew_level: 12,
                max_crew: 12,
                energy_level: 16,
                max_energy: 16,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 60,
                known_loc: (7468, 9246),
            },
            characteristics: Characteristics {
                max_thrust: 40,
                thrust_increment: 8,
                energy_regeneration: 1,
                weapon_energy_cost: 1,
                special_energy_cost: 1,
                energy_wait: 4,
                turn_wait: 1,
                thrust_wait: 0,
                weapon_wait: 2,
                special_wait: 0,
                ship_mass: 4,
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 600,
            },
        }
    }

    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        // Gob launcher
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (120, 0), // speed=120
            life_span: 10,
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
        let ship = SupoxShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 16);
        assert_eq!(desc.ship_info.max_crew, 12);
        assert_eq!(desc.ship_info.max_energy, 16);
        assert_eq!(desc.characteristics.max_thrust, 40);
        assert_eq!(desc.fleet.strength, 60);
        assert_eq!(desc.intel.weapon_range, 600);
    }

    #[test]
    fn weapon_basic() {
        let mut ship = SupoxShip::default();
        let state = ShipState {
            crew_level: 12,
            max_crew: 12,
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
        assert_eq!(weapons[0].life_span, 10);
    }

    #[test]
    fn ai_basic() {
        let mut ship = SupoxShip::default();
        let state = ShipState {
            crew_level: 12,
            max_crew: 12,
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
