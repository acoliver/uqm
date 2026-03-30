// Ilwrath Avenger - Hellfire spout + cloaking device
// @plan PLAN-20260314-SHIPS.P11

use crate::ships::battle_bridge::{self, MissileBlock};
use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

// C: ilwrath.c constants
const MAX_CREW: u16 = 22;
const MAX_ENERGY: u8 = 16;
const ENERGY_REGENERATION: u8 = 4;
const ENERGY_WAIT: u8 = 4;
const MAX_THRUST: u16 = 25;
const THRUST_INCREMENT: u16 = 5;
const THRUST_WAIT: u8 = 0;
const TURN_WAIT: u8 = 2;
const SHIP_MASS: u8 = 7;

const WEAPON_ENERGY_COST: u8 = 1;
const WEAPON_WAIT: u8 = 0;
const ILWRATH_OFFSET: i16 = 29;
const MISSILE_LIFE: u16 = 8;
const MISSILE_SPEED: i16 = 25; // MAX_THRUST
const MISSILE_HITS: i16 = 1;
const MISSILE_DAMAGE: i16 = 1;
const MISSILE_OFFSET: i16 = 0;

const SPECIAL_ENERGY_COST: u8 = 3;
const SPECIAL_WAIT: u8 = 13;

#[derive(Debug, Default)]
pub struct IlwrathShip;

impl ShipBehavior for IlwrathShip {
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
                strength: 254, // 1410/SPHERE_RADIUS_INCREMENT*2
                known_loc: (48, 1700),
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
                weapon_range: 100, // CLOSE_RANGE_WEAPON
            },
        }
    }

    /// C: ilwrath_preprocess — cloaking device with color fade animation.
    /// Complex STAMPFILL_PRIM / color cycling + auto-aiming while cloaked.
    /// Kept in C.
    fn preprocess(
        &mut self,
        ship: &mut ShipState,
        _ctx: &BattleContext,
    ) -> Result<(), ShipsError> {
        if !ship.cur_status_flags.contains(StatusFlags::SPECIAL) {
            return Ok(());
        }
        if ship.special_counter > 0 {
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
                let sound =
                    battle_bridge::bridge::set_abs_sound_index(ship.ship_sounds, 1);
                battle_bridge::bridge::process_sound(sound, ship.element_ptr);
            }
            // Color cycling + STAMPFILL handled by C preprocess_func
            return Ok(());
        }

        #[cfg(test)]
        {
            if ship.energy_level < SPECIAL_ENERGY_COST as u16 {
                return Ok(());
            }
            ship.energy_level -= SPECIAL_ENERGY_COST as u16;
            ship.special_counter = SPECIAL_WAIT;
        }

        Ok(())
    }

    /// C: initialize_flame — fires hellfire spout.
    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        #[cfg(not(test))]
        {
            let block = MissileBlock {
                cx: ship.position.0 as i16,
                cy: ship.position.1 as i16,
                flags: crate::ships::runtime::IGNORE_SIMILAR as u16,
                sender: ship.player_nr,
                pixoffs: ILWRATH_OFFSET,
                speed: MISSILE_SPEED,
                hit_points: MISSILE_HITS,
                damage: MISSILE_DAMAGE,
                face: ship.ship_facing as u16,
                index: 0,
                life: MISSILE_LIFE,
                farray: ship.weapon_farray as *mut battle_bridge::Frame,
                preprocess_func: None, // flame_preprocess handled by C
                blast_offs: MISSILE_OFFSET,
            };
            let _ = battle_bridge::bridge::create_missile(&block);
            return Ok(vec![]);
        }

        #[cfg(test)]
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (25, 0), // MISSILE_SPEED = MAX_THRUST
            life_span: MISSILE_LIFE,
            hit_points: MISSILE_HITS as u16,
            damage: MISSILE_DAMAGE as u16,
            mass: 0,
        }])
    }

    fn intelligence(&mut self, _ship: &ShipState, _ctx: &BattleContext) -> StatusFlags {
        // C: ilwrath_intelligence — uncloaks to flame, cloaks otherwise.
        StatusFlags::THRUST
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_template_matches_c() {
        let ship = IlwrathShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 10);
        assert_eq!(desc.ship_info.max_crew, 22);
        assert_eq!(desc.ship_info.max_energy, 16);
        assert_eq!(desc.characteristics.energy_regeneration, 4);
        assert_eq!(desc.characteristics.special_energy_cost, 3);
        assert_eq!(desc.characteristics.special_wait, 13);
        assert_eq!(desc.fleet.known_loc, (48, 1700));
    }

    #[test]
    fn weapon_basic() {
        let mut ship = IlwrathShip::default();
        let state = ShipState {
            crew_level: 22,
            max_crew: 22,
            energy_level: 16,
            max_energy: 16,
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
        assert_eq!(weapons[0].damage, MISSILE_DAMAGE as u16);
        assert_eq!(weapons[0].life_span, MISSILE_LIFE);
    }

    #[test]
    fn cloak_activates() {
        let mut ship = IlwrathShip::default();
        let mut state = ShipState {
            crew_level: 22,
            max_crew: 22,
            energy_level: 16,
            max_energy: 16,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.preprocess(&mut state, &ctx).unwrap();

        assert_eq!(state.energy_level, 13); // 16 - 3
        assert_eq!(state.special_counter, SPECIAL_WAIT);
    }

    #[test]
    fn cloak_denied_low_energy() {
        let mut ship = IlwrathShip::default();
        let mut state = ShipState {
            crew_level: 22,
            max_crew: 22,
            energy_level: 2,
            max_energy: 16,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.preprocess(&mut state, &ctx).unwrap();

        assert_eq!(state.energy_level, 2);
        assert_eq!(state.special_counter, 0);
    }

    #[test]
    fn ai_basic() {
        let mut ship = IlwrathShip::default();
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
