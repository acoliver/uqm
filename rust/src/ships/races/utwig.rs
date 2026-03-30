// Utwig Jugger - Six-shot energy bolt + Absorption shield
// @plan PLAN-20260314-SHIPS.P12

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct UtwigShip;

impl ShipBehavior for UtwigShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE
                    | ShipFlags::POINT_DEFENSE
                    | ShipFlags::SHIELD_DEFENSE,
                ship_cost: 22,
                crew_level: 20, // MAX_CREW
                max_crew: 20,
                energy_level: 10, // MAX_ENERGY >> 1
                max_energy: 20,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 120,
                known_loc: (8534, 8797),
            },
            characteristics: Characteristics {
                max_thrust: 36,
                thrust_increment: 6,
                energy_regeneration: 0,
                weapon_energy_cost: 0,
                special_energy_cost: 1,
                energy_wait: 255,
                turn_wait: 1,
                thrust_wait: 6,
                weapon_wait: 7,
                special_wait: 12,
                ship_mass: 8,
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
        // Six-shot spread pattern
        // Offsets from C: (20,-72), (52,-36), (68,-16), (-20,-72), (-52,-36), (-68,-16)
        Ok(vec![
            WeaponElement {
                offset: (20, -72),
                facing: ship.ship_facing,
                velocity: (0, 0),
                life_span: 10,
                hit_points: 1,
                damage: 1,
                mass: 0,
            },
            WeaponElement {
                offset: (52, -36),
                facing: ship.ship_facing,
                velocity: (0, 0),
                life_span: 10,
                hit_points: 1,
                damage: 1,
                mass: 0,
            },
            WeaponElement {
                offset: (68, -16),
                facing: ship.ship_facing,
                velocity: (0, 0),
                life_span: 10,
                hit_points: 1,
                damage: 1,
                mass: 0,
            },
            WeaponElement {
                offset: (-20, -72),
                facing: ship.ship_facing,
                velocity: (0, 0),
                life_span: 10,
                hit_points: 1,
                damage: 1,
                mass: 0,
            },
            WeaponElement {
                offset: (-52, -36),
                facing: ship.ship_facing,
                velocity: (0, 0),
                life_span: 10,
                hit_points: 1,
                damage: 1,
                mass: 0,
            },
            WeaponElement {
                offset: (-68, -16),
                facing: ship.ship_facing,
                velocity: (0, 0),
                life_span: 10,
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
        let ship = UtwigShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 22);
        assert_eq!(desc.ship_info.crew_level, 20);
        assert_eq!(desc.ship_info.max_crew, 20);
        assert_eq!(desc.ship_info.energy_level, 10);
        assert_eq!(desc.ship_info.max_energy, 20);
        assert_eq!(desc.characteristics.max_thrust, 36);
        assert_eq!(desc.fleet.strength, 120);
        assert_eq!(desc.intel.weapon_range, 200);
    }

    #[test]
    fn weapon_basic() {
        let mut ship = UtwigShip::default();
        let state = ShipState {
            crew_level: 20,
            max_crew: 20,
            energy_level: 10,
            max_energy: 20,
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
        assert_eq!(weapons.len(), 6);
        assert_eq!(weapons[0].damage, 1);
    }

    #[test]
    fn ai_basic() {
        let mut ship = UtwigShip::default();
        let state = ShipState {
            crew_level: 20,
            max_crew: 20,
            energy_level: 10,
            max_energy: 20,
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
