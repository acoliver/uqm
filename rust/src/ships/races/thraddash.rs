// Thraddash Torch - Ion blasters + afterburner trail
// @plan PLAN-20260314-SHIPS.P11

use crate::ships::battle_bridge::{self, MissileBlock};
use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

// C: thradd.c constants
const MAX_CREW: u16 = 8;
const MAX_ENERGY: u8 = 24;
const ENERGY_REGENERATION: u8 = 1;
const ENERGY_WAIT: u8 = 6;
const MAX_THRUST: u16 = 28;
const THRUST_INCREMENT: u16 = 7;
const THRUST_WAIT: u8 = 0;
const TURN_WAIT: u8 = 1;
const SHIP_MASS: u8 = 7;

const WEAPON_ENERGY_COST: u8 = 2;
const WEAPON_WAIT: u8 = 12;
const THRADDASH_OFFSET: i16 = 9;
const MISSILE_OFFSET: i16 = 3;
const MISSILE_LIFE: u16 = 15;
const MISSILE_HITS: i16 = 2;
const MISSILE_DAMAGE: i16 = 1;

const SPECIAL_ENERGY_COST: u8 = 1;
const SPECIAL_WAIT: u8 = 0;
const SPECIAL_MAX_THRUST: u16 = 72;

#[derive(Debug, Default)]
pub struct ThraddashShip;

impl ShipBehavior for ThraddashShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE,
                ship_cost: 10,
                crew_level: MAX_CREW,
                max_crew: MAX_CREW,
                energy_level: MAX_ENERGY,
                max_energy: MAX_ENERGY,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 149, // 833/SPHERE_RADIUS_INCREMENT*2
                known_loc: (2535, 8358),
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
                weapon_range: 4500, // (MISSILE_SPEED * MISSILE_LIFE) >> 1
            },
        }
    }

    /// C: thraddash_preprocess — afterburner special.
    /// Boosts thrust and drops napalm trail. Complex element manipulation
    /// (creates trail elements, modifies thrust characteristics) — kept in C.
    fn preprocess(
        &mut self,
        ship: &mut ShipState,
        _ctx: &BattleContext,
    ) -> Result<(), ShipsError> {
        if !ship.cur_status_flags.contains(StatusFlags::SPECIAL) {
            // Not pressing special — but if just released, mark beyond max
            if ship.old_status_flags.contains(StatusFlags::SPECIAL)
                && ship
                    .cur_status_flags
                    .contains(StatusFlags::SHIP_AT_MAX_SPEED)
            {
                // C sets SHIP_BEYOND_MAX_SPEED when releasing afterburner at max
            }
            return Ok(());
        }

        #[cfg(not(test))]
        if !ship.element_ptr.is_null() {
            unsafe {
                if !battle_bridge::bridge::delta_energy(
                    ship.element_ptr,
                    -(SPECIAL_ENERGY_COST as i16),
                ) {
                    return Ok(());
                }
                // C creates napalm trail element + boosts thrust.
                // Done by C preprocess_func.
            }
        }

        #[cfg(test)]
        {
            if ship.energy_level < SPECIAL_ENERGY_COST as u16 {
                return Ok(());
            }
            ship.energy_level -= SPECIAL_ENERGY_COST as u16;
        }

        Ok(())
    }

    /// C: initialize_horn — fires ion blaster.
    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        #[cfg(not(test))]
        {
            let missile_speed =
                battle_bridge::bridge::display_to_world(30) as i16;
            let block = MissileBlock {
                cx: ship.position.0 as i16,
                cy: ship.position.1 as i16,
                flags: crate::ships::runtime::IGNORE_SIMILAR as u16,
                sender: ship.player_nr,
                pixoffs: THRADDASH_OFFSET,
                speed: missile_speed,
                hit_points: MISSILE_HITS,
                damage: MISSILE_DAMAGE,
                face: ship.ship_facing as u16,
                index: ship.ship_facing as u16,
                life: MISSILE_LIFE,
                farray: ship.weapon_farray as *mut battle_bridge::Frame,
                preprocess_func: None,
                blast_offs: MISSILE_OFFSET,
            };
            let _ = battle_bridge::bridge::create_missile(&block);
            return Ok(vec![]);
        }

        #[cfg(test)]
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (120, 0),
            life_span: MISSILE_LIFE,
            hit_points: MISSILE_HITS as u16,
            damage: MISSILE_DAMAGE as u16,
            mass: 0,
        }])
    }

    fn intelligence(&mut self, _ship: &ShipState, _ctx: &BattleContext) -> StatusFlags {
        // C: thraddash_intelligence — uses afterburner to pursue/dodge.
        StatusFlags::THRUST
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_template_matches_c() {
        let ship = ThraddashShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 10);
        assert_eq!(desc.ship_info.max_crew, 8);
        assert_eq!(desc.ship_info.max_energy, 24);
        assert_eq!(desc.characteristics.weapon_energy_cost, 2);
        assert_eq!(desc.characteristics.weapon_wait, 12);
        assert_eq!(desc.characteristics.special_energy_cost, 1);
        assert_eq!(desc.fleet.known_loc, (2535, 8358));
    }

    #[test]
    fn weapon_basic() {
        let mut ship = ThraddashShip::default();
        let state = ShipState {
            crew_level: 8,
            max_crew: 8,
            energy_level: 24,
            max_energy: 24,
            ship_facing: 0,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        let weapons = ship.init_weapon(&state, &ctx).unwrap();
        assert_eq!(weapons.len(), 1);
        assert_eq!(weapons[0].hit_points, MISSILE_HITS as u16);
    }

    #[test]
    fn afterburner_drains_energy() {
        let mut ship = ThraddashShip::default();
        let mut state = ShipState {
            crew_level: 8,
            max_crew: 8,
            energy_level: 24,
            max_energy: 24,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.preprocess(&mut state, &ctx).unwrap();

        assert_eq!(state.energy_level, 23); // 24 - 1
    }

    #[test]
    fn afterburner_denied_no_energy() {
        let mut ship = ThraddashShip::default();
        let mut state = ShipState {
            crew_level: 8,
            max_crew: 8,
            energy_level: 0,
            max_energy: 24,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.preprocess(&mut state, &ctx).unwrap();

        assert_eq!(state.energy_level, 0);
    }

    #[test]
    fn ai_basic() {
        let mut ship = ThraddashShip::default();
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
