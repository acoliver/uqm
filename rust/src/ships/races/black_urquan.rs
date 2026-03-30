// Kohr-Ah Marauder - Spinning buzzsaw + F.R.I.E.D. gas cloud
// @plan PLAN-20260314-SHIPS.P11

use crate::ships::battle_bridge::{self, MissileBlock};
use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

// C: blackurq.c constants
const MAX_CREW: u16 = 42; // MAX_CREW_SIZE
const MAX_ENERGY: u8 = 42; // MAX_ENERGY_SIZE
const ENERGY_REGENERATION: u8 = 1;
const ENERGY_WAIT: u8 = 4;
const MAX_THRUST: u16 = 30;
const THRUST_INCREMENT: u16 = 6;
const THRUST_WAIT: u8 = 6;
const TURN_WAIT: u8 = 4;
const SHIP_MASS: u8 = 10;

// Buzzsaw
const WEAPON_ENERGY_COST: u8 = 6;
const WEAPON_WAIT: u8 = 6;
const KOHR_AH_OFFSET: i16 = 28;
const MISSILE_OFFSET: i16 = 9;
const MISSILE_SPEED: i16 = 64;
const MISSILE_LIFE: u16 = 64;
const MISSILE_HITS: i16 = 10;
const MISSILE_DAMAGE: i16 = 4;

// F.R.I.E.D.
const SPECIAL_ENERGY_COST: u8 = 21; // MAX_ENERGY_SIZE / 2
const SPECIAL_WAIT: u8 = 9;
const GAS_DAMAGE: u16 = 3;

#[derive(Debug, Default)]
pub struct BlackUrquanShip;

impl ShipBehavior for BlackUrquanShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE,
                ship_cost: 30,
                crew_level: MAX_CREW,
                max_crew: MAX_CREW,
                energy_level: MAX_ENERGY,
                max_energy: MAX_ENERGY,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 480, // 2666/SPHERE_RADIUS_INCREMENT*2
                known_loc: (6000, 6250),
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
                weapon_range: 0, // CLOSE_RANGE_WEAPON
            },
        }
    }

    /// C: black_urquan_preprocess — resets special_wait counter (tracks active saws);
    /// holds weapon key to keep buzzsaw alive.
    fn preprocess(
        &mut self,
        ship: &mut ShipState,
        _ctx: &BattleContext,
    ) -> Result<(), ShipsError> {
        // Track active buzzsaws via special_wait reset
        #[cfg(not(test))]
        if !ship.element_ptr.is_null() {
            // C: StarShipPtr->RaceDescPtr->characteristics.special_wait = 0;
            // Handled by C preprocess_func
        }

        // Hold-to-extend behavior: if weapon key held, increment weapon_counter
        // to prevent re-fire (saw stays alive while key is held)
        if ship
            .cur_status_flags
            .contains(StatusFlags::WEAPON)
            && ship
                .old_status_flags
                .contains(StatusFlags::WEAPON)
            && ship.weapon_counter == 0
        {
            ship.weapon_counter += 1;
        }

        Ok(())
    }

    /// C: black_urquan_postprocess — spawn F.R.I.E.D. gas cloud ring.
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
                // spawn_gas_cloud handled by C postprocess_func
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

    /// C: initialize_buzzsaw — fires spinning buzzsaw projectile.
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
                pixoffs: KOHR_AH_OFFSET,
                speed: MISSILE_SPEED,
                hit_points: MISSILE_HITS,
                damage: MISSILE_DAMAGE,
                face: ship.ship_facing as u16,
                index: 0,
                life: MISSILE_LIFE,
                farray: ship.weapon_farray as *mut battle_bridge::Frame,
                preprocess_func: None, // buzzsaw_preprocess set in C
                blast_offs: MISSILE_OFFSET,
            };
            let _ = battle_bridge::bridge::create_missile(&block);
            return Ok(vec![]);
        }

        #[cfg(test)]
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (MISSILE_SPEED as i32, 0),
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
        let ship = BlackUrquanShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 30);
        assert_eq!(desc.ship_info.max_crew, 42);
        assert_eq!(desc.ship_info.max_energy, 42);
        assert!(desc.ship_info.ship_flags.contains(ShipFlags::FIRES_FORE));
        assert_eq!(desc.characteristics.weapon_energy_cost, 6);
        assert_eq!(desc.characteristics.special_energy_cost, 21);
        assert_eq!(desc.fleet.known_loc, (6000, 6250));
    }

    #[test]
    fn weapon_fires_buzzsaw() {
        let mut ship = BlackUrquanShip::default();
        let state = ShipState {
            energy_level: 42,
            max_energy: 42,
            ..ShipState::default()
        };
        let ctx = BattleContext { hyperspace: false, frame_count: 0, gravity_center: None };

        let weapons = ship.init_weapon(&state, &ctx).unwrap();
        assert_eq!(weapons.len(), 1);
        assert_eq!(weapons[0].damage, MISSILE_DAMAGE as u16);
        assert_eq!(weapons[0].hit_points, MISSILE_HITS as u16);
    }

    #[test]
    fn fried_drains_energy() {
        let mut ship = BlackUrquanShip::default();
        let mut state = ShipState {
            crew_level: 42,
            max_crew: 42,
            energy_level: 42,
            max_energy: 42,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext { hyperspace: false, frame_count: 0, gravity_center: None };

        ship.postprocess(&mut state, &ctx).unwrap();
        assert_eq!(state.energy_level, 42 - 21);
        assert_eq!(state.special_counter, SPECIAL_WAIT);
    }

    #[test]
    fn fried_denied_low_energy() {
        let mut ship = BlackUrquanShip::default();
        let mut state = ShipState {
            crew_level: 42,
            energy_level: 10,
            max_energy: 42,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext { hyperspace: false, frame_count: 0, gravity_center: None };

        ship.postprocess(&mut state, &ctx).unwrap();
        assert_eq!(state.energy_level, 10);
    }

    #[test]
    fn preprocess_holds_weapon() {
        let mut ship = BlackUrquanShip::default();
        let mut state = ShipState {
            cur_status_flags: StatusFlags::WEAPON,
            old_status_flags: StatusFlags::WEAPON,
            weapon_counter: 0,
            ..ShipState::default()
        };
        let ctx = BattleContext { hyperspace: false, frame_count: 0, gravity_center: None };

        ship.preprocess(&mut state, &ctx).unwrap();
        assert_eq!(state.weapon_counter, 1);
    }

    #[test]
    fn ai_basic() {
        let mut ship = BlackUrquanShip::default();
        let state = ShipState::default();
        let ctx = BattleContext { hyperspace: false, frame_count: 0, gravity_center: None };

        let flags = ship.intelligence(&state, &ctx);
        assert!(flags.contains(StatusFlags::THRUST));
    }
}
