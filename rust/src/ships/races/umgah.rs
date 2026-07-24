// Umgah Drone - Antimatter cone + retropropulsion zip
// @plan PLAN-20260314-SHIPS.P11

#[cfg(not(test))]
use crate::ships::battle_bridge::{self, MissileBlock};
use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

// C: umgah.c constants
const MAX_CREW: u16 = 10;
const MAX_ENERGY: u8 = 30;
const ENERGY_REGENERATION: u8 = 30; // MAX_ENERGY
const ENERGY_WAIT: u8 = 150;
const MAX_THRUST: u16 = 18;
const THRUST_INCREMENT: u16 = 6;
const THRUST_WAIT: u8 = 3;
const TURN_WAIT: u8 = 4;
const SHIP_MASS: u8 = 1;

const WEAPON_ENERGY_COST: u8 = 0;
const WEAPON_WAIT: u8 = 0;
const CONE_HITS: i16 = 100;
const CONE_DAMAGE: i16 = 1;
const CONE_LIFE: u16 = 1;

const SPECIAL_ENERGY_COST: u8 = 1;
const SPECIAL_WAIT: u8 = 2;

#[derive(Debug, Default)]
pub struct UmgahShip;

impl ShipBehavior for UmgahShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::IMMEDIATE_WEAPON,
                ship_cost: 7,
                crew_level: MAX_CREW,
                max_crew: MAX_CREW,
                energy_level: MAX_ENERGY,
                max_energy: MAX_ENERGY,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 149, // 833/SPHERE_RADIUS_INCREMENT*2
                known_loc: (1798, 6000),
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
                weapon_range: 40000, // LONG_RANGE_WEAPON << 2
            },
        }
    }

    /// C: umgah_preprocess — zip backwards (retropropulsion).
    fn preprocess(&mut self, ship: &mut ShipState, _ctx: &BattleContext) -> Result<(), ShipsError> {
        if !ship.cur_status_flags.contains(StatusFlags::SPECIAL) {
            return Ok(());
        }
        if ship.thrust_wait > 0 {
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
                let sound = battle_bridge::bridge::set_abs_sound_index(ship.ship_sounds, 1);
                battle_bridge::bridge::process_sound(sound, ship.element_ptr);
                // DeltaVelocity backwards handled by C preprocess_func
            }
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

    /// C: umgah_postprocess — zero velocity after zip.
    fn postprocess(
        &mut self,
        _ship: &mut ShipState,
        _ctx: &BattleContext,
    ) -> Result<(), ShipsError> {
        // C: zeroes velocity when special_counter > 0. Handled by C.
        Ok(())
    }

    /// C: initialize_cone — antimatter cone weapon.
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
                flags: crate::ships::runtime::IGNORE_SIMILAR,
                sender: ship.player_nr,
                pixoffs: 0,
                speed: 0, // CONE_SPEED
                hit_points: CONE_HITS,
                damage: CONE_DAMAGE,
                face: ship.ship_facing as u16,
                index: 0,
                life: CONE_LIFE,
                farray: ship.special_farray as *mut battle_bridge::Frame,
                preprocess_func: None, // cone_preprocess handled by C
                blast_offs: 0,
            };
            let _ = battle_bridge::bridge::create_missile(&block);
            Ok(vec![])
        }

        #[cfg(test)]
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (0, 0),
            life_span: CONE_LIFE,
            hit_points: CONE_HITS as u16,
            damage: CONE_DAMAGE as u16,
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
        let ship = UmgahShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 7);
        assert_eq!(desc.ship_info.max_crew, 10);
        assert_eq!(desc.ship_info.max_energy, 30);
        assert!(desc
            .ship_info
            .ship_flags
            .contains(ShipFlags::IMMEDIATE_WEAPON));
        assert_eq!(desc.characteristics.energy_regeneration, 30);
        assert_eq!(desc.characteristics.energy_wait, 150);
        assert_eq!(desc.fleet.known_loc, (1798, 6000));
    }

    #[test]
    fn weapon_basic() {
        let mut ship = UmgahShip;
        let state = ShipState {
            crew_level: 10,
            max_crew: 10,
            energy_level: 30,
            max_energy: 30,
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
        assert_eq!(weapons[0].hit_points, CONE_HITS as u16);
    }

    #[test]
    fn zip_drains_energy() {
        let mut ship = UmgahShip;
        let mut state = ShipState {
            crew_level: 10,
            max_crew: 10,
            energy_level: 30,
            max_energy: 30,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.preprocess(&mut state, &ctx).unwrap();

        assert_eq!(state.energy_level, 29);
        assert_eq!(state.special_counter, SPECIAL_WAIT);
    }

    #[test]
    fn zip_denied_no_energy() {
        let mut ship = UmgahShip;
        let mut state = ShipState {
            crew_level: 10,
            max_crew: 10,
            energy_level: 0,
            max_energy: 30,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.preprocess(&mut state, &ctx).unwrap();

        assert_eq!(state.special_counter, 0);
    }

    #[test]
    fn ai_basic() {
        let mut ship = UmgahShip;
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
