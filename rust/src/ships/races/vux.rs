// VUX Intruder - Laser + Limpet speed reducer + Warp-in advantage
// @plan PLAN-20260314-SHIPS.P12

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct VuxShip;

impl ShipBehavior for VuxShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE
                    | ShipFlags::SEEKING_SPECIAL
                    | ShipFlags::IMMEDIATE_WEAPON,
                ship_cost: 12,
                crew_level: 20,
                max_crew: 20,
                energy_level: 40,
                max_energy: 40,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 162,
                known_loc: (4412, 1558),
            },
            characteristics: Characteristics {
                max_thrust: 21,
                thrust_increment: 7,
                energy_regeneration: 1,
                weapon_energy_cost: 1,
                special_energy_cost: 2,
                energy_wait: 8,
                turn_wait: 6,
                thrust_wait: 4,
                weapon_wait: 0,
                special_wait: 7,
                ship_mass: 6,
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
        // Laser (immediate)
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
        let ship = VuxShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 12);
        assert_eq!(desc.ship_info.max_crew, 20);
        assert_eq!(desc.ship_info.max_energy, 40);
        assert_eq!(desc.characteristics.max_thrust, 21);
        assert_eq!(desc.fleet.strength, 162);
        assert_eq!(desc.intel.weapon_range, 200);
    }

    #[test]
    fn weapon_basic() {
        let mut ship = VuxShip::default();
        let state = ShipState {
            crew_level: 20,
            max_crew: 20,
            energy_level: 40,
            max_energy: 40,
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
        let mut ship = VuxShip::default();
        let state = ShipState {
            crew_level: 20,
            max_crew: 20,
            energy_level: 40,
            max_energy: 40,
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
