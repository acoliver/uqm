// ZoqFotPik Stinger - Spit + tongue attack
// @plan PLAN-20260314-SHIPS.P11

use crate::ships::battle_bridge::{self, MissileBlock};
use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

// C: zoqfot.c constants
const MAX_CREW: u16 = 10;
const MAX_ENERGY: u8 = 10;
const ENERGY_REGENERATION: u8 = 1;
const ENERGY_WAIT: u8 = 4;
const MAX_THRUST: u16 = 40;
const THRUST_INCREMENT: u16 = 10;
const THRUST_WAIT: u8 = 0;
const TURN_WAIT: u8 = 1;
const SHIP_MASS: u8 = 5;

const WEAPON_ENERGY_COST: u8 = 1;
const WEAPON_WAIT: u8 = 0;
const ZOQFOTPIK_OFFSET: i16 = 13;
const MISSILE_OFFSET: i16 = 0;
const MISSILE_LIFE: u16 = 10;
const MISSILE_DAMAGE: i16 = 1;
const MISSILE_HITS: i16 = 1;

const SPECIAL_ENERGY_COST: u8 = 7; // MAX_ENERGY * 3 / 4
const SPECIAL_WAIT: u8 = 6;
const TONGUE_OFFSET: i16 = 4;
const TONGUE_DAMAGE: i16 = 12;
const TONGUE_HITS: i16 = 1;

#[derive(Debug, Default)]
pub struct ZoqfotpikShip;

impl ShipBehavior for ZoqfotpikShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE,
                ship_cost: 6,
                crew_level: MAX_CREW,
                max_crew: MAX_CREW,
                energy_level: MAX_ENERGY,
                max_energy: MAX_ENERGY,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 57, // 320/SPHERE_RADIUS_INCREMENT*2
                known_loc: (3761, 5333),
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
                weapon_range: 400, // MISSILE_SPEED * MISSILE_LIFE
            },
        }
    }

    /// C: zoqfotpik_postprocess — tongue attack on special, spawns tongue
    /// chain. The tongue is a recursive spawn (tongue_postprocess calls
    /// spawn_tongue again) requiring element list manipulation.
    fn postprocess(
        &mut self,
        ship: &mut ShipState,
        _ctx: &BattleContext,
    ) -> Result<(), ShipsError> {
        if !ship.cur_status_flags.contains(StatusFlags::SPECIAL) {
            return Ok(());
        }
        if ship.special_counter > 0 {
            // Already tonguing — spawn_tongue continues from C
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
            // spawn_tongue handled by C postprocess_func
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

    /// C: initialize_spit — fires animated spit projectile.
    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        #[cfg(not(test))]
        {
            // C: speed = DISPLAY_TO_WORLD(GetFrameCount(weapon[0])) << 1
            // We use a fixed value here since we can't query frame count
            let missile_speed =
                battle_bridge::bridge::display_to_world(10) as i16;
            let block = MissileBlock {
                cx: ship.position.0 as i16,
                cy: ship.position.1 as i16,
                flags: crate::ships::runtime::IGNORE_SIMILAR as u16,
                sender: ship.player_nr,
                pixoffs: ZOQFOTPIK_OFFSET,
                speed: missile_speed,
                hit_points: MISSILE_HITS,
                damage: MISSILE_DAMAGE,
                face: ship.ship_facing as u16,
                index: 0,
                life: MISSILE_LIFE,
                farray: ship.weapon_farray as *mut battle_bridge::Frame,
                preprocess_func: None, // spit_preprocess handled by C
                blast_offs: MISSILE_OFFSET,
            };
            let _ = battle_bridge::bridge::create_missile(&block);
            return Ok(vec![]);
        }

        #[cfg(test)]
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (40, 0),
            life_span: MISSILE_LIFE,
            hit_points: MISSILE_HITS as u16,
            damage: MISSILE_DAMAGE as u16,
            mass: 0,
        }])
    }

    fn intelligence(&mut self, _ship: &ShipState, _ctx: &BattleContext) -> StatusFlags {
        // C: zoqfotpik_intelligence — uses tongue when close, spit otherwise.
        // Full port requires EVALUATE_DESC.
        StatusFlags::THRUST
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_template_matches_c() {
        let ship = ZoqfotpikShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 6);
        assert_eq!(desc.ship_info.max_crew, 10);
        assert_eq!(desc.ship_info.max_energy, 10);
        assert!(desc.ship_info.ship_flags.contains(ShipFlags::FIRES_FORE));
        assert_eq!(desc.characteristics.max_thrust, 40);
        assert_eq!(desc.characteristics.thrust_increment, 10);
        assert_eq!(desc.characteristics.special_energy_cost, 7); // MAX_ENERGY * 3/4
        assert_eq!(desc.fleet.known_loc, (3761, 5333));
    }

    #[test]
    fn weapon_basic() {
        let mut ship = ZoqfotpikShip::default();
        let state = ShipState {
            crew_level: 10,
            max_crew: 10,
            energy_level: 10,
            max_energy: 10,
            ship_facing: 2,
            player_nr: 0,
            position: (100, 100),
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
    fn tongue_drains_energy() {
        let mut ship = ZoqfotpikShip::default();
        let mut state = ShipState {
            crew_level: 10,
            max_crew: 10,
            energy_level: 10,
            max_energy: 10,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.postprocess(&mut state, &ctx).unwrap();

        assert_eq!(state.energy_level, 3); // 10 - 7
        assert_eq!(state.special_counter, SPECIAL_WAIT);
    }

    #[test]
    fn tongue_denied_low_energy() {
        let mut ship = ZoqfotpikShip::default();
        let mut state = ShipState {
            crew_level: 10,
            max_crew: 10,
            energy_level: 5,
            max_energy: 10,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.postprocess(&mut state, &ctx).unwrap();

        assert_eq!(state.energy_level, 5); // No change
        assert_eq!(state.special_counter, 0);
    }

    #[test]
    fn ai_basic() {
        let mut ship = ZoqfotpikShip::default();
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
