// Androsynth Guardian - Acid bubbles + Blazer mode transform
// @plan PLAN-20260314-SHIPS.P12

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct AndrosynthShip;

impl ShipBehavior for AndrosynthShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::SEEKING_WEAPON,
                ship_cost: 15,
                crew_level: 20,
                max_crew: 20,
                energy_level: 24,
                max_energy: 24,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 0xFFFF,
                known_loc: (4999, 4999),
            },
            characteristics: Characteristics {
                max_thrust: 24,
                thrust_increment: 3,
                energy_regeneration: 1,
                weapon_energy_cost: 3,
                special_energy_cost: 2,
                energy_wait: 8,
                turn_wait: 4,
                thrust_wait: 0,
                weapon_wait: 0,
                special_wait: 0,
                ship_mass: 6,
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 1000, // LONG_RANGE_WEAPON >> 2
            },
        }
    }

    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        // Acid bubble
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (0, 0), // Speed calculated from facing
            life_span: 200,
            hit_points: 3,
            damage: 2,
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
        let ship = AndrosynthShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 15);
        assert_eq!(desc.ship_info.crew_level, 20);
        assert_eq!(desc.ship_info.max_crew, 20);
        assert_eq!(desc.ship_info.max_energy, 24);
        assert_eq!(desc.characteristics.max_thrust, 24);
        assert_eq!(desc.fleet.strength, 0xFFFF);
        assert_eq!(desc.intel.weapon_range, 1000);
    }

    #[test]
    fn weapon_basic() {
        let mut ship = AndrosynthShip::default();
        let state = ShipState {
            crew_level: 20,
            max_crew: 20,
            energy_level: 24,
            max_energy: 24,
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
        assert_eq!(weapons[0].damage, 2);
        assert_eq!(weapons[0].hit_points, 3);
    }

    #[test]
    fn ai_basic() {
        let mut ship = AndrosynthShip::default();
        let state = ShipState {
            crew_level: 20,
            max_crew: 20,
            energy_level: 24,
            max_energy: 24,
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
