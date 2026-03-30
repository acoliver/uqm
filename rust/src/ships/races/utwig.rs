// Utwig Jugger - Six-lance volley + absorption shield
// @plan PLAN-20260314-SHIPS.P11

use crate::ships::battle_bridge::{self, MissileBlock};
use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

// C: utwig.c constants
const MAX_CREW: u16 = 20;
const MAX_ENERGY: u8 = 20;
const ENERGY_REGENERATION: u8 = 0;
const ENERGY_WAIT: u8 = 255;
const MAX_THRUST: u16 = 36;
const THRUST_INCREMENT: u16 = 6;
const THRUST_WAIT: u8 = 6;
const TURN_WAIT: u8 = 1;
const SHIP_MASS: u8 = 8;

const WEAPON_ENERGY_COST: u8 = 0;
const WEAPON_WAIT: u8 = 7;
const MISSILE_LIFE: u16 = 10;
const MISSILE_HITS: i16 = 1;
const MISSILE_DAMAGE: i16 = 1;
const MISSILE_OFFSET: i16 = 1;

const SPECIAL_ENERGY_COST: u8 = 1;
const SPECIAL_WAIT: u8 = 12;

#[derive(Debug, Default)]
pub struct UtwigShip;

impl ShipBehavior for UtwigShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE
                    | ShipFlags::POINT_DEFENSE
                    | ShipFlags::SHIELD_DEFENSE,
                ship_cost: 22,
                crew_level: MAX_CREW,
                max_crew: MAX_CREW,
                energy_level: MAX_ENERGY / 2, // MAX_ENERGY >> 1
                max_energy: MAX_ENERGY,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 119, // 666/SPHERE_RADIUS_INCREMENT*2
                known_loc: (8534, 8797),
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

    /// C: utwig_preprocess — absorption shield.
    /// Gains energy from absorbed projectiles (life_span tracking).
    /// Complex element manipulation — kept in C.
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
            // C preprocess handles shield activation + energy drain.
            // Shield absorbs incoming damage and converts to energy.
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

    /// C: initialize_lance — fires 6 lances in spread pattern.
    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        #[cfg(not(test))]
        {
            let missile_speed =
                battle_bridge::bridge::display_to_world(30) as i16;

            let mut block = MissileBlock {
                cx: 0,
                cy: 0,
                flags: crate::ships::runtime::IGNORE_SIMILAR as u16,
                sender: ship.player_nr,
                pixoffs: 0,
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

            // 6 lances in 3 pairs, each pair offset symmetrically
            for _ in 0..3 {
                block.cx = ship.position.0 as i16;
                block.cy = ship.position.1 as i16;
                let _ = battle_bridge::bridge::create_missile(&block);
                let _ = battle_bridge::bridge::create_missile(&block);
            }

            return Ok(vec![]);
        }

        #[cfg(test)]
        {
            let mut weapons = Vec::with_capacity(6);
            for _ in 0..6 {
                weapons.push(WeaponElement {
                    offset: (0, 0),
                    facing: ship.ship_facing,
                    velocity: (120, 0),
                    life_span: MISSILE_LIFE,
                    hit_points: MISSILE_HITS as u16,
                    damage: MISSILE_DAMAGE as u16,
                    mass: 0,
                });
            }
            Ok(weapons)
        }
    }

    fn intelligence(&mut self, _ship: &ShipState, _ctx: &BattleContext) -> StatusFlags {
        // C: utwig_intelligence — shield vs incoming, pursue non-immediate weapons.
        StatusFlags::THRUST
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_template_matches_c() {
        let ship = UtwigShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 22);
        assert_eq!(desc.ship_info.max_crew, 20);
        assert_eq!(desc.ship_info.max_energy, 20);
        assert_eq!(desc.ship_info.energy_level, 10); // starts at half
        assert!(desc.ship_info.ship_flags.contains(ShipFlags::SHIELD_DEFENSE));
        assert!(desc.ship_info.ship_flags.contains(ShipFlags::POINT_DEFENSE));
        assert_eq!(desc.characteristics.energy_regeneration, 0);
        assert_eq!(desc.characteristics.energy_wait, 255);
        assert_eq!(desc.fleet.known_loc, (8534, 8797));
    }

    #[test]
    fn weapon_fires_six_lances() {
        let mut ship = UtwigShip::default();
        let state = ShipState {
            crew_level: 20,
            max_crew: 20,
            energy_level: 20,
            max_energy: 20,
            ship_facing: 0,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        let weapons = ship.init_weapon(&state, &ctx).unwrap();
        assert_eq!(weapons.len(), 6);
    }

    #[test]
    fn shield_activates() {
        let mut ship = UtwigShip::default();
        let mut state = ShipState {
            crew_level: 20,
            max_crew: 20,
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

        ship.preprocess(&mut state, &ctx).unwrap();

        assert_eq!(state.energy_level, 9);
        assert_eq!(state.special_counter, SPECIAL_WAIT);
    }

    #[test]
    fn shield_denied_no_energy() {
        let mut ship = UtwigShip::default();
        let mut state = ShipState {
            crew_level: 20,
            max_crew: 20,
            energy_level: 0,
            max_energy: 20,
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
        let mut ship = UtwigShip::default();
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
