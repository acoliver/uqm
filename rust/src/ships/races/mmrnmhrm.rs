// Mmrnmhrm X-Form/Y-Form - Laser wing (X) + Twin missiles (Y)
// @plan PLAN-20260314-SHIPS.P12

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct MmrnmhrmShip;

impl ShipBehavior for MmrnmhrmShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::IMMEDIATE_WEAPON,
                ship_cost: 19,
                crew_level: 20,
                max_crew: 20,
                energy_level: 10,
                max_energy: 10,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 0,
                known_loc: (0, 0),
            },
            characteristics: Characteristics {
                max_thrust: 20,
                thrust_increment: 5,
                energy_regeneration: 2,
                weapon_energy_cost: 1,
                special_energy_cost: 10, // MAX_ENERGY
                energy_wait: 6,
                turn_wait: 2,
                thrust_wait: 1,
                weapon_wait: 0,
                special_wait: 0,
                ship_mass: 3,
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
        // X-form laser (immediate)
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
        let ship = MmrnmhrmShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 19);
        assert_eq!(desc.ship_info.max_crew, 20);
        assert_eq!(desc.ship_info.max_energy, 10);
        assert_eq!(desc.characteristics.max_thrust, 20);
        assert_eq!(desc.fleet.strength, 0);
        assert_eq!(desc.intel.weapon_range, 200);
    }

    #[test]
    fn weapon_basic() {
        let mut ship = MmrnmhrmShip::default();
        let state = ShipState {
            crew_level: 20,
            max_crew: 20,
            energy_level: 10,
            max_energy: 10,
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
    }

    #[test]
    fn ai_basic() {
        let mut ship = MmrnmhrmShip::default();
        let state = ShipState {
            crew_level: 20,
            max_crew: 20,
            energy_level: 10,
            max_energy: 10,
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
