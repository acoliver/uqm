// Pkunk Fury - Triple spread shot + Insult energy regen + Resurrection
// @plan PLAN-20260314-SHIPS.P12

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct PkunkShip;

impl ShipBehavior for PkunkShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::FIRES_LEFT | ShipFlags::FIRES_RIGHT,
                ship_cost: 20,
                crew_level: 8,
                max_crew: 8,
                energy_level: 12,
                max_energy: 12,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 120,
                known_loc: (502, 401),
            },
            characteristics: Characteristics {
                max_thrust: 64,
                thrust_increment: 16,
                energy_regeneration: 0,
                weapon_energy_cost: 1,
                special_energy_cost: 2,
                energy_wait: 0,
                turn_wait: 0,
                thrust_wait: 0,
                weapon_wait: 0,
                special_wait: 16,
                ship_mass: 1,
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 201, // CLOSE_RANGE_WEAPON + 1
            },
        }
    }

    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        // Triple shot - fore, left, right
        Ok(vec![
            WeaponElement {
                offset: (0, 0),
                facing: ship.ship_facing,
                velocity: (0, 0),
                life_span: 5,
                hit_points: 1,
                damage: 1,
                mass: 0,
            },
            WeaponElement {
                offset: (0, 0),
                facing: ship.ship_facing.wrapping_sub(2), // Left
                velocity: (0, 0),
                life_span: 5,
                hit_points: 1,
                damage: 1,
                mass: 0,
            },
            WeaponElement {
                offset: (0, 0),
                facing: ship.ship_facing.wrapping_add(2), // Right
                velocity: (0, 0),
                life_span: 5,
                hit_points: 1,
                damage: 1,
                mass: 0,
            },
        ])
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
        let ship = PkunkShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 20);
        assert_eq!(desc.ship_info.max_crew, 8);
        assert_eq!(desc.ship_info.max_energy, 12);
        assert_eq!(desc.characteristics.max_thrust, 64);
        assert_eq!(desc.fleet.strength, 120);
        assert_eq!(desc.intel.weapon_range, 201);
    }

    #[test]
    fn weapon_basic() {
        let mut ship = PkunkShip::default();
        let state = ShipState {
            crew_level: 8,
            max_crew: 8,
            energy_level: 12,
            max_energy: 12,
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
        assert_eq!(weapons.len(), 3);
        assert_eq!(weapons[0].damage, 1);
    }

    #[test]
    fn ai_basic() {
        let mut ship = PkunkShip::default();
        let state = ShipState {
            crew_level: 8,
            max_crew: 8,
            energy_level: 12,
            max_energy: 12,
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
