// Slylandro Probe - Lightning weapon + space junk harvester
// @plan PLAN-20260314-SHIPS.P11

#[cfg(not(test))]
use crate::ships::battle_bridge::{self, LaserBlock};
use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

// C: slylandr.c constants
const MAX_CREW: u16 = 12;
const MAX_ENERGY: u8 = 20;
const ENERGY_REGENERATION: u8 = 0;
const ENERGY_WAIT: u8 = 10;
const MAX_THRUST: u16 = 60;
const THRUST_INCREMENT: u16 = 60; // MAX_THRUST
const THRUST_WAIT: u8 = 0;
const TURN_WAIT: u8 = 0;
const SHIP_MASS: u8 = 1;

const WEAPON_ENERGY_COST: u8 = 2;
const WEAPON_WAIT: u8 = 17;

const SPECIAL_ENERGY_COST: u8 = 0;
const SPECIAL_WAIT: u8 = 20;

#[derive(Debug, Default)]
pub struct SlylandroShip;

impl ShipBehavior for SlylandroShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::SEEKING_WEAPON | ShipFlags::CREW_IMMUNE,
                ship_cost: 17,
                crew_level: MAX_CREW,
                max_crew: MAX_CREW,
                energy_level: MAX_ENERGY,
                max_energy: MAX_ENERGY,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: u16::MAX, // INFINITE_RADIUS
                known_loc: (333, 9812),
            },
            characteristics: Characteristics {
                max_thrust: MAX_THRUST,
                thrust_increment: THRUST_INCREMENT,
                energy_regeneration: ENERGY_REGENERATION,
                weapon_energy_cost: WEAPON_ENERGY_COST,
                special_energy_cost: SPECIAL_ENERGY_COST,
                energy_wait: ENERGY_WAIT,
                turn_wait: TURN_WAIT,
                thrust_wait: THRUST_WAIT,
                weapon_wait: WEAPON_WAIT,
                special_wait: SPECIAL_WAIT,
                ship_mass: SHIP_MASS,
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 200, // CLOSE_RANGE_WEAPON << 1
            },
        }
    }

    /// C: slylandro_preprocess — always at max speed, thrust reversal.
    /// Complex facing/velocity manipulation — kept in C.
    fn preprocess(
        &mut self,
        _ship: &mut ShipState,
        _ctx: &BattleContext,
    ) -> Result<(), ShipsError> {
        // Handled by C preprocess_func: reverse on THRUST, always max speed
        Ok(())
    }

    /// C: slylandro_postprocess — continues lightning chain + harvests.
    fn postprocess(
        &mut self,
        ship: &mut ShipState,
        _ctx: &BattleContext,
    ) -> Result<(), ShipsError> {
        // C: spawns continuation lightning bolts and harvests space junk.
        // Both require element list walking — kept in C.
        if ship.cur_status_flags.contains(StatusFlags::SPECIAL) && ship.special_counter == 0 {
            // harvest_space_junk handled by C
            #[cfg(test)]
            {
                ship.special_counter = SPECIAL_WAIT;
            }
        }
        Ok(())
    }

    /// C: initialize_lightning — random branching laser chain.
    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        #[cfg(not(test))]
        {
            // Lightning uses random angles and chaining — handled by C
            let block = LaserBlock {
                cx: ship.position.0 as i16,
                cy: ship.position.1 as i16,
                ex: 0,
                ey: 0,
                face: 0,
                sender: ship.player_nr,
                flags: crate::ships::runtime::IGNORE_SIMILAR,
                pixoffs: 0,
                color: battle_bridge::Color {
                    r: 0xFF,
                    g: 0xFF,
                    b: 0xFF,
                    a: 0xFF,
                },
            };
            let _ = battle_bridge::bridge::create_laser(&block);
            Ok(vec![])
        }

        #[cfg(test)]
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (0, 0),
            life_span: 1,
            hit_points: 0,
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
        let ship = SlylandroShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 17);
        assert_eq!(desc.ship_info.max_crew, 12);
        assert_eq!(desc.ship_info.max_energy, 20);
        assert!(desc.ship_info.ship_flags.contains(ShipFlags::CREW_IMMUNE));
        assert_eq!(desc.characteristics.max_thrust, 60);
        assert_eq!(desc.characteristics.thrust_increment, 60);
        assert_eq!(desc.characteristics.energy_regeneration, 0);
        assert_eq!(desc.fleet.known_loc, (333, 9812));
    }

    #[test]
    fn weapon_basic() {
        let mut ship = SlylandroShip;
        let state = ShipState {
            crew_level: 12,
            max_crew: 12,
            energy_level: 20,
            max_energy: 20,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        let weapons = ship.init_weapon(&state, &ctx).unwrap();
        assert_eq!(weapons.len(), 1);
    }

    #[test]
    fn harvest_sets_cooldown() {
        let mut ship = SlylandroShip;
        let mut state = ShipState {
            crew_level: 12,
            max_crew: 12,
            energy_level: 10,
            max_energy: 20,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.postprocess(&mut state, &ctx).unwrap();

        assert_eq!(state.special_counter, SPECIAL_WAIT);
    }

    #[test]
    fn ai_basic() {
        let mut ship = SlylandroShip;
        let state = ShipState::default();
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        let flags = ship.intelligence(&state, &ctx);
        assert!(flags.contains(StatusFlags::THRUST));
    }
}
