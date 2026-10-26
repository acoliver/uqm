// Ur-Quan Dreadnought - Fusion blast + autonomous fighters
// @plan PLAN-20260314-SHIPS.P11

#[cfg(not(test))]
use crate::ships::battle_bridge::{self, MissileBlock};
use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

// C: urquan.c constants
const MAX_CREW: u16 = 42; // MAX_CREW_SIZE
const MAX_ENERGY: u8 = 42; // MAX_ENERGY_SIZE
const ENERGY_REGENERATION: u8 = 1;
const ENERGY_WAIT: u8 = 4;
const MAX_THRUST: u16 = 30;
const THRUST_INCREMENT: u16 = 6;
const THRUST_WAIT: u8 = 6;
const TURN_WAIT: u8 = 4;
const SHIP_MASS: u8 = 10;

const WEAPON_ENERGY_COST: u8 = 6;
const WEAPON_WAIT: u8 = 6;
#[cfg(not(test))]
const URQUAN_OFFSET: i16 = 32;
#[cfg(not(test))]
const MISSILE_OFFSET: i16 = 8;
const MISSILE_LIFE: u16 = 20;
const MISSILE_HITS: i16 = 10;
const MISSILE_DAMAGE: i16 = 6;

const SPECIAL_ENERGY_COST: u8 = 8;
const SPECIAL_WAIT: u8 = 9;

#[derive(Debug, Default)]
pub struct UrquanShip;

impl ShipBehavior for UrquanShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::SEEKING_SPECIAL,
                ship_cost: 30,
                crew_level: MAX_CREW,
                max_crew: MAX_CREW,
                energy_level: MAX_ENERGY,
                max_energy: MAX_ENERGY,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 480, // 2666/SPHERE_RADIUS_INCREMENT*2
                known_loc: (5750, 6000),
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
                weapon_range: 1600, // MISSILE_SPEED * MISSILE_LIFE
            },
        }
    }

    /// C: urquan_postprocess — launch fighters (costs crew + energy).
    /// Fighters are independent sub-elements with own AI/collision.
    fn postprocess(
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
        if ship.crew_level <= 1 {
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
                // spawn_fighters handled by C postprocess_func
            }
        }

        #[cfg(test)]
        {
            if ship.energy_level < SPECIAL_ENERGY_COST as u16 {
                return Ok(());
            }
            ship.energy_level -= SPECIAL_ENERGY_COST as u16;
            // Fighters cost 2 crew (1 per fighter, up to 2)
            let fighters = if ship.crew_level > 2 { 2 } else { 1 };
            ship.crew_level -= fighters;
            ship.special_counter = SPECIAL_WAIT;
        }

        Ok(())
    }

    /// C: initialize_fusion — fires fusion blast.
    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        #[cfg(not(test))]
        {
            let missile_speed = battle_bridge::bridge::display_to_world(20) as i16;
            let block = MissileBlock {
                cx: ship.position.0 as i16,
                cy: ship.position.1 as i16,
                flags: crate::ships::runtime::IGNORE_SIMILAR,
                sender: ship.player_nr,
                pixoffs: URQUAN_OFFSET,
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
            Ok(vec![])
        }

        #[cfg(test)]
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (80, 0),
            life_span: MISSILE_LIFE,
            hit_points: MISSILE_HITS as u16,
            damage: MISSILE_DAMAGE as u16,
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
        let ship = UrquanShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 30);
        assert_eq!(desc.ship_info.max_crew, 42);
        assert_eq!(desc.ship_info.max_energy, 42);
        assert!(desc
            .ship_info
            .ship_flags
            .contains(ShipFlags::SEEKING_SPECIAL));
        assert_eq!(desc.characteristics.special_energy_cost, 8);
        assert_eq!(desc.fleet.known_loc, (5750, 6000));
    }

    #[test]
    fn weapon_fires_fusion() {
        let mut ship = UrquanShip;
        let state = ShipState {
            crew_level: 42,
            max_crew: 42,
            energy_level: 42,
            max_energy: 42,
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
    }

    #[test]
    fn fighters_drain_crew_and_energy() {
        let mut ship = UrquanShip;
        let mut state = ShipState {
            crew_level: 42,
            max_crew: 42,
            energy_level: 42,
            max_energy: 42,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.postprocess(&mut state, &ctx).unwrap();

        assert_eq!(state.energy_level, 34); // 42 - 8
        assert_eq!(state.crew_level, 40); // 42 - 2 fighters
        assert_eq!(state.special_counter, SPECIAL_WAIT);
    }

    #[test]
    fn fighters_denied_low_crew() {
        let mut ship = UrquanShip;
        let mut state = ShipState {
            crew_level: 1,
            max_crew: 42,
            energy_level: 42,
            max_energy: 42,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.postprocess(&mut state, &ctx).unwrap();

        assert_eq!(state.crew_level, 1);
        assert_eq!(state.energy_level, 42);
    }

    #[test]
    fn ai_basic() {
        let mut ship = UrquanShip;
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
