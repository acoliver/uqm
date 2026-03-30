// Orz Nemesis - Howitzer turret + Space marine boarding
// @plan PLAN-20260314-SHIPS.P12

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct OrzShip;

impl ShipBehavior for OrzShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::SEEKING_SPECIAL,
                ship_cost: 23,
                crew_level: 16,
                max_crew: 16,
                energy_level: 20,
                max_energy: 20,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 60,
                known_loc: (3608, 2637),
            },
            characteristics: Characteristics {
                max_thrust: 35,
                thrust_increment: 5,
                energy_regeneration: 1,
                weapon_energy_cost: 6, // MAX_ENERGY/3
                special_energy_cost: 0,
                energy_wait: 6,
                turn_wait: 1,
                thrust_wait: 0,
                weapon_wait: 4,
                special_wait: 12,
                ship_mass: 4,
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 1440, // MISSILE_SPEED * MISSILE_LIFE
            },
        }
    }

    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        // Howitzer
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (0, 0), // Speed calculated from facing
            life_span: 12,
            hit_points: 2,
            damage: 3,
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
        let ship = OrzShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 23);
        assert_eq!(desc.ship_info.max_crew, 16);
        assert_eq!(desc.ship_info.max_energy, 20);
        assert_eq!(desc.characteristics.max_thrust, 35);
        assert_eq!(desc.fleet.strength, 60);
        assert_eq!(desc.intel.weapon_range, 1440);
    }

    #[test]
    fn weapon_basic() {
        let mut ship = OrzShip::default();
        let state = ShipState {
            crew_level: 16,
            max_crew: 16,
            energy_level: 20,
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
        assert_eq!(weapons.len(), 1);
        assert_eq!(weapons[0].damage, 3);
        assert_eq!(weapons[0].hit_points, 2);
    }

    #[test]
    fn ai_basic() {
        let mut ship = OrzShip::default();
        let state = ShipState {
            crew_level: 16,
            max_crew: 16,
            energy_level: 20,
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
