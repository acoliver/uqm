// Chenjesu Broodhome - Crystal shard (fragments on impact) + DOGI mine
// @plan PLAN-20260314-SHIPS.P13

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct ChenjesuShip;

impl ShipBehavior for ChenjesuShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE
                    | ShipFlags::SEEKING_SPECIAL
                    | ShipFlags::SEEKING_WEAPON,
                ship_cost: 28,
                crew_level: 36,
                max_crew: 36,
                energy_level: 30,
                max_energy: 30,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 0,
                known_loc: (0, 0),
            },
            characteristics: Characteristics {
                max_thrust: 27,
                thrust_increment: 3,
                energy_regeneration: 1,
                weapon_energy_cost: 5,
                special_energy_cost: 30, // MAX_ENERGY
                energy_wait: 4,
                turn_wait: 6,
                thrust_wait: 4,
                weapon_wait: 0,
                special_wait: 0,
                ship_mass: 10,
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 4000, // LONG_RANGE_WEAPON
            },
        }
    }

    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        // Crystal shard - DISPLAY_TO_WORLD(16) = 64
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (64, 0), // speed=64
            life_span: 90,
            hit_points: 10,
            damage: 6,
            mass: 4,
        }])
    }

    fn intelligence(&mut self, _ship: &ShipState, _ctx: &BattleContext) -> StatusFlags {
        // Race-specific AI deferred: depends on ship_intelligence() from cyborg.c (battle engine scope)
        StatusFlags::THRUST
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chenjesu_descriptor() {
        let ship = ChenjesuShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 28);
        assert_eq!(desc.ship_info.crew_level, 36);
        assert_eq!(desc.ship_info.max_crew, 36);
        assert_eq!(desc.ship_info.energy_level, 30);
        assert_eq!(desc.ship_info.max_energy, 30);
        assert_eq!(desc.characteristics.max_thrust, 27);
        assert_eq!(desc.fleet.strength, 0);
        assert_eq!(desc.intel.weapon_range, 4000);
    }

    #[test]
    fn test_chenjesu_flags() {
        let ship = ChenjesuShip;
        let desc = ship.descriptor_template();

        assert!(desc.ship_info.ship_flags.contains(ShipFlags::FIRES_FORE));
        assert!(desc
            .ship_info
            .ship_flags
            .contains(ShipFlags::SEEKING_SPECIAL));
        assert!(desc
            .ship_info
            .ship_flags
            .contains(ShipFlags::SEEKING_WEAPON));
    }

    #[test]
    fn test_chenjesu_weapon() {
        let mut ship = ChenjesuShip;
        let state = ShipState {
            crew_level: 0,
            max_crew: 0,
            energy_level: 0,
            max_energy: 0,
            ship_facing: 0,
            cur_status_flags: StatusFlags::empty(),
            old_status_flags: StatusFlags::empty(),
            player_nr: 0,
            position: (0, 0),
            velocity: (0, 0), ..ShipState::default()
        };
        let context = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };
        let weapons = ship.init_weapon(&state, &context).unwrap();

        assert_eq!(weapons.len(), 1);
        assert_eq!(weapons[0].damage, 6);
        assert_eq!(weapons[0].hit_points, 10);
    }
}
