// Yehat Terminator - Twin pulse cannon + energy shield
// @plan PLAN-20260314-SHIPS.P11

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct YehatShip;

impl ShipBehavior for YehatShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::SHIELD_DEFENSE,
                ship_cost: 23,
                crew_level: 20,
                max_crew: 20,
                energy_level: 10,
                max_energy: 10,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 136,
                known_loc: (4970, 40),
            },
            characteristics: Characteristics {
                max_thrust: 30,
                thrust_increment: 6,
                energy_regeneration: 2,
                weapon_energy_cost: 1,
                special_energy_cost: 3,
                energy_wait: 6,
                turn_wait: 2,
                thrust_wait: 2,
                weapon_wait: 0,
                special_wait: 2,
                ship_mass: 3,
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 266,
            },
        }
    }

    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        // Twin pulse cannon - TWO projectiles offset laterally
        Ok(vec![
            WeaponElement {
                offset: (0, -16), // Left pulse, offset perpendicular to facing
                facing: ship.ship_facing,
                velocity: (80, 0), // speed=80
                life_span: 10,
                hit_points: 1,
                damage: 1,
                mass: 0,
            },
            WeaponElement {
                offset: (0, 16), // Right pulse, offset perpendicular to facing
                facing: ship.ship_facing,
                velocity: (80, 0), // speed=80
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
        let ship = YehatShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 23);
        assert_eq!(desc.ship_info.max_crew, 20);
        assert_eq!(desc.ship_info.max_energy, 10);
        assert_eq!(desc.characteristics.max_thrust, 30);
        assert_eq!(desc.fleet.strength, 136);
        assert_eq!(desc.intel.weapon_range, 266);
    }

    #[test]
    fn weapon_basic() {
        let mut ship = YehatShip::default();
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
        assert_eq!(weapons.len(), 2); // Twin pulse!
        assert_eq!(weapons[0].damage, 1);
        assert_eq!(weapons[1].damage, 1);
        assert_eq!(weapons[0].life_span, 10);
    }

    #[test]
    fn ai_basic() {
        let mut ship = YehatShip::default();
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
