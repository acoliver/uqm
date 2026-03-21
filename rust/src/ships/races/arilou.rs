// Arilou Skiff - Auto-aiming laser + quasi-space teleport
// @plan PLAN-20260314-SHIPS.P11

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct ArilouShip;

impl ShipBehavior for ArilouShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::IMMEDIATE_WEAPON,
                ship_cost: 16,
                crew_level: 6,
                max_crew: 6,
                energy_level: 20,
                max_energy: 20,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 44,
                known_loc: (438, 6372),
            },
            characteristics: Characteristics {
                max_thrust: 40,
                thrust_increment: 40,
                energy_regeneration: 1,
                weapon_energy_cost: 2,
                special_energy_cost: 3,
                energy_wait: 6,
                turn_wait: 0,
                thrust_wait: 0,
                weapon_wait: 1,
                special_wait: 2,
                ship_mass: 1,
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 218, // LASER_RANGE >> 1
            },
        }
    }

    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        // Auto-aiming tracking laser
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (0, 0), // Instant beam
            life_span: 1,     // Single-frame
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
        let ship = ArilouShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 16);
        assert_eq!(desc.ship_info.max_crew, 6);
        assert_eq!(desc.ship_info.max_energy, 20);
        assert_eq!(desc.characteristics.max_thrust, 40);
        assert_eq!(desc.fleet.strength, 44);
        assert_eq!(desc.intel.weapon_range, 218);
    }

    #[test]
    fn weapon_basic() {
        let mut ship = ArilouShip::default();
        let state = ShipState {
            crew_level: 6,
            max_crew: 6,
            energy_level: 20,
            max_energy: 20,
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
        assert_eq!(weapons[0].damage, 1);
    }

    #[test]
    fn ai_basic() {
        let mut ship = ArilouShip::default();
        let state = ShipState {
            crew_level: 6,
            max_crew: 6,
            energy_level: 20,
            max_energy: 20,
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
