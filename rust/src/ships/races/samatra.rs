// Sa-Matra - Final battle boss with multiple weapon systems
// @plan PLAN-20260314-SHIPS.P13

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct SamatraShip;

impl ShipBehavior for SamatraShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::IMMEDIATE_WEAPON | ShipFlags::CREW_IMMUNE,
                ship_cost: 16,
                crew_level: 1,
                max_crew: 1,
                energy_level: 42,
                max_energy: 42,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 0,
                known_loc: (0, 0),
            },
            characteristics: Characteristics {
                max_thrust: 0,
                thrust_increment: 0,
                energy_regeneration: 1,
                weapon_energy_cost: 2,
                special_energy_cost: 3,
                energy_wait: 6,
                turn_wait: 0,
                thrust_wait: 0,
                weapon_wait: 240, // (ONE_SECOND/BATTLE_FRAME_RATE)*10 = 24*10
                special_wait: 72, // (ONE_SECOND/BATTLE_FRAME_RATE)*3 = 24*3
                ship_mass: 100,   // MAX_SHIP_MASS*10
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 0,
            },
        }
    }

    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        // Comet - DISPLAY_TO_WORLD(12) = 48
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (48, 0), // speed=48
            life_span: 2,
            hit_points: 12,
            damage: 2,
            mass: 6,
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
    fn test_samatra_descriptor() {
        let ship = SamatraShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 16);
        assert_eq!(desc.ship_info.crew_level, 1);
        assert_eq!(desc.ship_info.max_crew, 1);
        assert_eq!(desc.ship_info.energy_level, 42);
        assert_eq!(desc.ship_info.max_energy, 42);
        assert_eq!(desc.characteristics.max_thrust, 0);
        assert_eq!(desc.characteristics.ship_mass, 100);
        assert_eq!(desc.characteristics.weapon_wait, 240);
        assert_eq!(desc.characteristics.special_wait, 72);
        assert_eq!(desc.intel.weapon_range, 0);
    }

    #[test]
    fn test_samatra_flags() {
        let ship = SamatraShip;
        let desc = ship.descriptor_template();

        assert!(desc
            .ship_info
            .ship_flags
            .contains(ShipFlags::IMMEDIATE_WEAPON));
        assert!(desc.ship_info.ship_flags.contains(ShipFlags::CREW_IMMUNE));
    }

    #[test]
    fn test_samatra_weapon() {
        let mut ship = SamatraShip;
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
            velocity: (0, 0),
        };
        let context = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };
        let weapons = ship.init_weapon(&state, &context).unwrap();

        assert_eq!(weapons.len(), 1);
    }
}
