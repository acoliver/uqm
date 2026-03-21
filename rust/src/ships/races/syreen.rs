// Syreen Penetrator - Particle beam + Siren Song crew steal
// @plan PLAN-20260314-SHIPS.P12

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

#[derive(Debug, Default)]
pub struct SyreenShip;

impl ShipBehavior for SyreenShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE,
                ship_cost: 13,
                crew_level: 12,
                max_crew: 42, // SYREEN_MAX_CREW_SIZE
                energy_level: 16,
                max_energy: 16,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 0,
                known_loc: (0, 0),
            },
            characteristics: Characteristics {
                max_thrust: 36,
                thrust_increment: 9,
                energy_regeneration: 1,
                weapon_energy_cost: 1,
                special_energy_cost: 5,
                energy_wait: 6,
                turn_wait: 1,
                thrust_wait: 1,
                weapon_wait: 8,
                special_wait: 20,
                ship_mass: 2,
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 800, // (MISSILE_SPEED * MISSILE_LIFE * 2 / 3)
            },
        }
    }

    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        // Particle beam
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (0, 0), // Speed calculated from facing
            life_span: 10,
            hit_points: 1,
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
        let ship = SyreenShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 13);
        assert_eq!(desc.ship_info.crew_level, 12);
        assert_eq!(desc.ship_info.max_crew, 42);
        assert_eq!(desc.ship_info.max_energy, 16);
        assert_eq!(desc.characteristics.max_thrust, 36);
        assert_eq!(desc.fleet.strength, 0);
        assert_eq!(desc.intel.weapon_range, 800);
    }

    #[test]
    fn weapon_basic() {
        let mut ship = SyreenShip::default();
        let state = ShipState {
            crew_level: 12,
            max_crew: 42,
            energy_level: 16,
            max_energy: 16,
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
    }

    #[test]
    fn ai_basic() {
        let mut ship = SyreenShip::default();
        let state = ShipState {
            crew_level: 12,
            max_crew: 42,
            energy_level: 16,
            max_energy: 16,
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
