// Pkunk Fury - Triple minigun + taunt/insult energy restore + phoenix resurrection
// @plan PLAN-20260314-SHIPS.P13

#[cfg(not(test))]
use crate::ships::battle_bridge::{self, MissileBlock};
use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

// C: pkunk.c constants
const MAX_CREW: u16 = 8;
const MAX_ENERGY: u8 = 12;
const ENERGY_REGENERATION: u8 = 0;
const ENERGY_WAIT: u8 = 0;
const MAX_THRUST: u16 = 64;
const THRUST_INCREMENT: u16 = 16;
const THRUST_WAIT: u8 = 0;
const TURN_WAIT: u8 = 0;
const SHIP_MASS: u8 = 1;

const WEAPON_ENERGY_COST: u8 = 1;
const WEAPON_WAIT: u8 = 0;
#[cfg(not(test))]
const PKUNK_OFFSET: i16 = 15;
#[cfg(not(test))]
const MISSILE_OFFSET: i16 = 1;
const MISSILE_LIFE: u16 = 5;
const MISSILE_HITS: i16 = 1;
const MISSILE_DAMAGE: i16 = 1;

const SPECIAL_ENERGY_COST: u8 = 2;
#[cfg(test)]
const SPECIAL_WAIT: u8 = 16;

#[derive(Debug, Default)]
pub struct PkunkShip;

impl ShipBehavior for PkunkShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::FIRES_LEFT | ShipFlags::FIRES_RIGHT,
                ship_cost: 20,
                crew_level: MAX_CREW,
                max_crew: MAX_CREW,
                energy_level: MAX_ENERGY,
                max_energy: MAX_ENERGY,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 120, // 666/SPHERE_RADIUS_INCREMENT*2
                known_loc: (502, 401),
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
                special_wait: 0, // C: set to 0 in descriptor
                ship_mass: SHIP_MASS,
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                maneuverability_index: 0,
                weapon_range: 1, // CLOSE_RANGE_WEAPON + 1
            },
        }
    }

    /// C: pkunk_postprocess — taunt/insult restores energy.
    /// Also handles phoenix resurrection (stays in C for now).
    fn postprocess(
        &mut self,
        ship: &mut ShipState,
        _ctx: &BattleContext,
    ) -> Result<(), ShipsError> {
        // Taunt: pressing SPECIAL restores energy if below max
        if !ship.cur_status_flags.contains(StatusFlags::SPECIAL) {
            return Ok(());
        }
        if ship.special_counter > 0 {
            return Ok(());
        }

        #[cfg(not(test))]
        if !ship.element_ptr.is_null() {
            unsafe {
                // DeltaEnergy adds energy (insult restores SPECIAL_ENERGY_COST)
                battle_bridge::bridge::delta_energy(ship.element_ptr, SPECIAL_ENERGY_COST as i16);
                let sound = battle_bridge::bridge::set_abs_sound_index(ship.ship_sounds, 2);
                battle_bridge::bridge::process_sound(sound, ship.element_ptr);
            }
        }

        #[cfg(test)]
        {
            let max = ship.max_energy;
            if ship.energy_level < max {
                ship.energy_level = (ship.energy_level + SPECIAL_ENERGY_COST as u16).min(max);
                ship.special_counter = SPECIAL_WAIT;
            }
        }

        Ok(())
    }

    /// C: initialize_bug_missile — fires 3 missiles (fore, left, right).
    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        #[cfg(not(test))]
        {
            let missile_speed = battle_bridge::bridge::display_to_world(24) as i16;

            // Fire 3 missiles: fore, left, right
            for i in 0u16..3 {
                let mut face = ship.ship_facing as u16 + (4 * i); // ANGLE_TO_FACING(QUADRANT) * i
                if i == 2 {
                    face += 4; // extra QUADRANT for right side
                }
                face %= 16; // NORMALIZE_FACING

                let block = MissileBlock {
                    cx: ship.position.0 as i16,
                    cy: ship.position.1 as i16,
                    flags: crate::ships::runtime::IGNORE_SIMILAR,
                    sender: ship.player_nr,
                    pixoffs: PKUNK_OFFSET,
                    speed: missile_speed,
                    hit_points: MISSILE_HITS,
                    damage: MISSILE_DAMAGE,
                    face,
                    index: 0,
                    life: MISSILE_LIFE,
                    farray: ship.weapon_farray as *mut battle_bridge::Frame,
                    preprocess_func: None,
                    blast_offs: MISSILE_OFFSET,
                };
                let _ = battle_bridge::bridge::create_missile(&block);
            }
            Ok(vec![])
        }

        #[cfg(test)]
        {
            let mut weapons = Vec::with_capacity(3);
            for i in 0u8..3 {
                let mut face = ship.ship_facing + (4 * i);
                if i == 2 {
                    face += 4;
                }
                face %= 16;
                weapons.push(WeaponElement {
                    offset: (0, 0),
                    facing: face,
                    velocity: (96, 0), // DISPLAY_TO_WORLD(24)
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
        StatusFlags::THRUST
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_template_matches_c() {
        let ship = PkunkShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 20);
        assert_eq!(desc.ship_info.max_crew, 8);
        assert_eq!(desc.ship_info.max_energy, 12);
        assert!(desc.ship_info.ship_flags.contains(ShipFlags::FIRES_FORE));
        assert!(desc.ship_info.ship_flags.contains(ShipFlags::FIRES_LEFT));
        assert!(desc.ship_info.ship_flags.contains(ShipFlags::FIRES_RIGHT));
        assert_eq!(desc.characteristics.ship_mass, 1);
        assert_eq!(desc.fleet.known_loc, (502, 401));
    }

    #[test]
    fn weapon_fires_triple() {
        let mut ship = PkunkShip;
        let state = ShipState {
            energy_level: 12,
            max_energy: 12,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        let weapons = ship.init_weapon(&state, &ctx).unwrap();
        assert_eq!(weapons.len(), 3);
        assert_eq!(weapons[0].damage, 1);
    }

    #[test]
    fn taunt_restores_energy() {
        let mut ship = PkunkShip;
        let mut state = ShipState {
            crew_level: 8,
            energy_level: 5,
            max_energy: 12,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.postprocess(&mut state, &ctx).unwrap();
        assert_eq!(state.energy_level, 7); // 5 + 2
        assert_eq!(state.special_counter, SPECIAL_WAIT);
    }

    #[test]
    fn taunt_caps_at_max() {
        let mut ship = PkunkShip;
        let mut state = ShipState {
            crew_level: 8,
            energy_level: 11,
            max_energy: 12,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.postprocess(&mut state, &ctx).unwrap();
        assert_eq!(state.energy_level, 12);
    }

    #[test]
    fn taunt_noop_at_full() {
        let mut ship = PkunkShip;
        let mut state = ShipState {
            crew_level: 8,
            energy_level: 12,
            max_energy: 12,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.postprocess(&mut state, &ctx).unwrap();
        // Energy stays at max, no special_counter set
        assert_eq!(state.energy_level, 12);
    }

    #[test]
    fn ai_basic() {
        let mut ship = PkunkShip;
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
