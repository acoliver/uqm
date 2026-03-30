// Chenjesu Broodhome - Photon crystal shard + D.O.G.I. (de-energizer)
// @plan PLAN-20260314-SHIPS.P11

use crate::ships::battle_bridge::{self, MissileBlock};
use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

// C: chenjesu.c constants
const MAX_CREW: u16 = 36;
const MAX_ENERGY: u8 = 30;
const ENERGY_REGENERATION: u8 = 1;
const ENERGY_WAIT: u8 = 4;
const MAX_THRUST: u16 = 27; // DISPLAY_TO_WORLD(7) ~ 27
const THRUST_INCREMENT: u16 = 3; // DISPLAY_TO_WORLD(2) ~ 3
const THRUST_WAIT: u8 = 4;
const TURN_WAIT: u8 = 6;
const SHIP_MASS: u8 = 10;

// Photon Shard
const WEAPON_ENERGY_COST: u8 = 5;
const WEAPON_WAIT: u8 = 0;
const CHENJESU_OFFSET: i16 = 16;
const MISSILE_OFFSET: i16 = 0;
const MISSILE_LIFE: u16 = 90;
const MISSILE_HITS: i16 = 10;
const MISSILE_DAMAGE: i16 = 6;

// D.O.G.I.
const SPECIAL_ENERGY_COST: u8 = 30; // MAX_ENERGY
const SPECIAL_WAIT: u8 = 0;
const MAX_DOGGIES: u8 = 4;

#[derive(Debug, Default)]
pub struct ChenjesuShip;

impl ShipBehavior for ChenjesuShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE
                    | ShipFlags::SEEKING_SPECIAL
                    | ShipFlags::SEEKING_WEAPON,
                ship_cost: 28,
                crew_level: MAX_CREW,
                max_crew: MAX_CREW,
                energy_level: MAX_ENERGY,
                max_energy: MAX_ENERGY,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 0,
                known_loc: (0, 0),
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
                weapon_range: 0, // LONG_RANGE_WEAPON
            },
        }
    }

    /// C: chenjesu_preprocess — hold weapon key to keep crystal alive;
    /// manages DOGI count via special_counter.
    fn preprocess(
        &mut self,
        ship: &mut ShipState,
        _ctx: &BattleContext,
    ) -> Result<(), ShipsError> {
        // Hold-to-extend crystal: if weapon held across frames, increment weapon_counter
        if ship
            .cur_status_flags
            .contains(StatusFlags::WEAPON)
            && ship
                .old_status_flags
                .contains(StatusFlags::WEAPON)
        {
            ship.weapon_counter += 1;
        }

        Ok(())
    }

    /// C: chenjesu_postprocess — spawn D.O.G.I. (up to MAX_DOGGIES).
    fn postprocess(
        &mut self,
        ship: &mut ShipState,
        _ctx: &BattleContext,
    ) -> Result<(), ShipsError> {
        if ship.cur_status_flags.contains(StatusFlags::SPECIAL)
            && ship.special_counter < MAX_DOGGIES
        {
            #[cfg(not(test))]
            if !ship.element_ptr.is_null() {
                unsafe {
                    if !battle_bridge::bridge::delta_energy(
                        ship.element_ptr,
                        -(SPECIAL_ENERGY_COST as i16),
                    ) {
                        // Still set counter to 1 per C logic
                        ship.special_counter = 1;
                        return Ok(());
                    }
                    // spawn_doggy handled by C postprocess_func
                }
            }

            #[cfg(test)]
            {
                if ship.energy_level < SPECIAL_ENERGY_COST as u16 {
                    ship.special_counter = 1;
                    return Ok(());
                }
                ship.energy_level -= SPECIAL_ENERGY_COST as u16;
            }
        }

        // C: StarShipPtr->special_counter = 1;
        // Always set to 1 because ship_postprocess will decrement
        ship.special_counter = 1;

        Ok(())
    }

    /// C: initialize_crystal — fires photon crystal shard.
    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        #[cfg(not(test))]
        {
            let missile_speed =
                battle_bridge::bridge::display_to_world(16) as i16;
            let block = MissileBlock {
                cx: ship.position.0 as i16,
                cy: ship.position.1 as i16,
                flags: crate::ships::runtime::IGNORE_SIMILAR as u16,
                sender: ship.player_nr,
                pixoffs: CHENJESU_OFFSET,
                speed: missile_speed,
                hit_points: MISSILE_HITS,
                damage: MISSILE_DAMAGE,
                face: ship.ship_facing as u16,
                index: 0,
                life: MISSILE_LIFE,
                farray: ship.weapon_farray as *mut battle_bridge::Frame,
                preprocess_func: None, // crystal_preprocess set by C
                blast_offs: MISSILE_OFFSET,
            };
            let _ = battle_bridge::bridge::create_missile(&block);
            return Ok(vec![]);
        }

        #[cfg(test)]
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (64, 0), // DISPLAY_TO_WORLD(16) ~ 64
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
        let ship = ChenjesuShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 28);
        assert_eq!(desc.ship_info.max_crew, 36);
        assert_eq!(desc.ship_info.max_energy, 30);
        assert!(desc
            .ship_info
            .ship_flags
            .contains(ShipFlags::SEEKING_WEAPON));
        assert_eq!(desc.characteristics.weapon_energy_cost, 5);
        assert_eq!(desc.characteristics.special_energy_cost, 30);
        assert_eq!(desc.characteristics.turn_wait, 6);
    }

    #[test]
    fn weapon_fires_crystal() {
        let mut ship = ChenjesuShip::default();
        let state = ShipState {
            energy_level: 30,
            max_energy: 30,
            ..ShipState::default()
        };
        let ctx = BattleContext { hyperspace: false, frame_count: 0, gravity_center: None };

        let weapons = ship.init_weapon(&state, &ctx).unwrap();
        assert_eq!(weapons.len(), 1);
        assert_eq!(weapons[0].damage, MISSILE_DAMAGE as u16);
        assert_eq!(weapons[0].life_span, 90);
    }

    #[test]
    fn dogi_drains_all_energy() {
        let mut ship = ChenjesuShip::default();
        let mut state = ShipState {
            crew_level: 36,
            energy_level: 30,
            max_energy: 30,
            cur_status_flags: StatusFlags::SPECIAL,
            special_counter: 0,
            ..ShipState::default()
        };
        let ctx = BattleContext { hyperspace: false, frame_count: 0, gravity_center: None };

        ship.postprocess(&mut state, &ctx).unwrap();
        assert_eq!(state.energy_level, 0);
        assert_eq!(state.special_counter, 1);
    }

    #[test]
    fn dogi_denied_low_energy() {
        let mut ship = ChenjesuShip::default();
        let mut state = ShipState {
            crew_level: 36,
            energy_level: 10,
            max_energy: 30,
            cur_status_flags: StatusFlags::SPECIAL,
            special_counter: 0,
            ..ShipState::default()
        };
        let ctx = BattleContext { hyperspace: false, frame_count: 0, gravity_center: None };

        ship.postprocess(&mut state, &ctx).unwrap();
        assert_eq!(state.energy_level, 10);
        assert_eq!(state.special_counter, 1);
    }

    #[test]
    fn dogi_denied_at_max() {
        let mut ship = ChenjesuShip::default();
        let mut state = ShipState {
            crew_level: 36,
            energy_level: 30,
            max_energy: 30,
            cur_status_flags: StatusFlags::SPECIAL,
            special_counter: 4, // MAX_DOGGIES
            ..ShipState::default()
        };
        let ctx = BattleContext { hyperspace: false, frame_count: 0, gravity_center: None };

        ship.postprocess(&mut state, &ctx).unwrap();
        assert_eq!(state.energy_level, 30); // not drained
        assert_eq!(state.special_counter, 1);
    }

    #[test]
    fn preprocess_holds_weapon() {
        let mut ship = ChenjesuShip::default();
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
        let mut ship = ChenjesuShip::default();
        let state = ShipState::default();
        let ctx = BattleContext { hyperspace: false, frame_count: 0, gravity_center: None };

        let flags = ship.intelligence(&state, &ctx);
        assert!(flags.contains(StatusFlags::THRUST));
    }
}
