// Umgah Drone - Antimatter cone + Retro zip backward thrust
// @plan PLAN-20260314-SHIPS.P13

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct UmgahShip;

impl ShipBehavior for UmgahShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::IMMEDIATE_WEAPON,
                ship_cost: 7,
                crew_level: 10,
                max_crew: 10,
                energy_level: 30,
                max_energy: 30,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 150,
                known_loc: (1798, 6000),
            },
            characteristics: Characteristics {
                max_thrust: 18,
                thrust_increment: 6,
                energy_regeneration: 30, // MAX_ENERGY - special regeneration mechanic
                weapon_energy_cost: 0,
                special_energy_cost: 1,
                energy_wait: 150,
                turn_wait: 4,
                thrust_wait: 3,
                weapon_wait: 0,
                special_wait: 2,
                ship_mass: 1,
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 16000, // LONG_RANGE_WEAPON << 2 = 4000 * 4
            },
        }
    }

    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        // Antimatter cone - immediate weapon
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (0, 0), // immediate
            life_span: 1,
            hit_points: 100,
            damage: 1,
            mass: 0,
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
    fn test_umgah_descriptor() {
        let ship = UmgahShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 7);
        assert_eq!(desc.ship_info.crew_level, 10);
        assert_eq!(desc.ship_info.max_crew, 10);
        assert_eq!(desc.ship_info.energy_level, 30);
        assert_eq!(desc.ship_info.max_energy, 30);
        assert_eq!(desc.characteristics.max_thrust, 18);
        assert_eq!(desc.characteristics.energy_regeneration, 30);
        assert_eq!(desc.fleet.strength, 150);
        assert_eq!(desc.intel.weapon_range, 16000);
    }

    #[test]
    fn test_umgah_weapon() {
        let mut ship = UmgahShip;
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
        assert_eq!(weapons[0].damage, 1);
        assert_eq!(weapons[0].hit_points, 100);
    }
}
