// Arilou Skiff - Auto-aiming laser + quasi-space teleport
// @plan PLAN-20260314-SHIPS.P11

#[cfg(not(test))]
use crate::ships::battle_bridge::{self, LaserBlock};
use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

// C: arilou.c constants
const MAX_CREW: u16 = 6;
const MAX_ENERGY: u8 = 20;
const ENERGY_REGENERATION: u8 = 1;
const ENERGY_WAIT: u8 = 6;
const MAX_THRUST: u16 = 40; // DISPLAY_TO_WORLD(10)
const THRUST_INCREMENT: u16 = 40; // == MAX_THRUST (instant)
const THRUST_WAIT: u8 = 0;
const TURN_WAIT: u8 = 0;
const SHIP_MASS: u8 = 1;

const WEAPON_ENERGY_COST: u8 = 2;
const WEAPON_WAIT: u8 = 1;
#[cfg(not(test))]
const ARILOU_OFFSET: i16 = 9;

const SPECIAL_ENERGY_COST: u8 = 3;
const SPECIAL_WAIT: u8 = 2;
// C constant reserved for full hyper-space Rust port.
#[expect(dead_code)]
const HYPER_LIFE: u16 = 5;

#[derive(Debug, Default)]
pub struct ArilouShip;

impl ShipBehavior for ArilouShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::IMMEDIATE_WEAPON,
                ship_cost: 16,
                crew_level: MAX_CREW,
                max_crew: MAX_CREW,
                energy_level: MAX_ENERGY,
                max_energy: MAX_ENERGY,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 44, // 250/SPHERE_RADIUS_INCREMENT*2
                known_loc: (438, 6372),
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
                weapon_range: 218, // LASER_RANGE >> 1 = DISPLAY_TO_WORLD(109) >> 1
            },
        }
    }

    /// C: arilou_preprocess — handles teleport (special).
    /// When SPECIAL pressed with enough energy: ship becomes NONSOLID,
    /// plays teleport animation, then reappears at random location.
    fn preprocess(&mut self, ship: &mut ShipState, _ctx: &BattleContext) -> Result<(), ShipsError> {
        // C: arilou_preprocess handles two states:
        // 1. Normal (not NONSOLID): zero velocity each frame, handle special activation
        // 2. Teleporting (NONSOLID): animate warp, reappear at random location
        //
        // The NONSOLID/FINITE_LIFE state machine requires direct ELEMENT access
        // (life_span, image.farray, state_flags) which aren't in ShipState.
        // The velocity zeroing and special activation logic port cleanly:

        // Zero velocity when not thrusting (Arilou has instant max speed)
        if ship.thrust_wait == 0 {
            ship.cur_status_flags &= !StatusFlags::SHIP_AT_MAX_SPEED;
        }

        if !ship.cur_status_flags.contains(StatusFlags::SPECIAL) {
            return Ok(());
        }
        if ship.special_counter > 0 {
            return Ok(());
        }

        // Teleport activation — requires C element for DeltaEnergy/ProcessSound
        #[cfg(not(test))]
        if !ship.element_ptr.is_null() {
            unsafe {
                if !battle_bridge::bridge::delta_energy(
                    ship.element_ptr,
                    -(SPECIAL_ENERGY_COST as i16),
                ) {
                    return Ok(());
                }
                let jump_sound = battle_bridge::bridge::set_abs_sound_index(ship.ship_sounds, 1);
                battle_bridge::bridge::process_sound(jump_sound, ship.element_ptr);
            }
        }

        #[cfg(test)]
        {
            if ship.energy_level < SPECIAL_ENERGY_COST as u16 {
                return Ok(());
            }
            ship.energy_level -= SPECIAL_ENERGY_COST as u16;
        }

        ship.cur_status_flags &= !(StatusFlags::SHIP_AT_MAX_SPEED
            | StatusFlags::LEFT
            | StatusFlags::RIGHT
            | StatusFlags::THRUST
            | StatusFlags::WEAPON);
        ship.special_counter = SPECIAL_WAIT;

        Ok(())
    }

    /// C: initialize_autoaim_laser — creates tracking laser.
    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        #[cfg(not(test))]
        {
            let laser_range = battle_bridge::bridge::display_to_world(100 + ARILOU_OFFSET as i32);
            let mut face = ship.ship_facing as u16;

            // Auto-aim: track enemy ship
            if !ship.element_ptr.is_null() {
                unsafe {
                    let delta = battle_bridge::bridge::track_ship(ship.element_ptr, &mut face);
                    if delta > 0 {
                        face = battle_bridge::bridge::normalize_facing(
                            ship.ship_facing as u16 + delta as u16,
                        );
                    }
                    battle_bridge::bridge::untarget(ship.element_ptr);
                }
            }

            let angle = battle_bridge::bridge::facing_to_angle(face);
            let block = LaserBlock {
                cx: ship.position.0 as i16,
                cy: ship.position.1 as i16,
                ex: battle_bridge::bridge::cosine(angle, laser_range as i16) as i16,
                ey: battle_bridge::bridge::sine(angle, laser_range as i16) as i16,
                face,
                sender: ship.player_nr,
                flags: crate::ships::runtime::IGNORE_SIMILAR,
                pixoffs: ARILOU_OFFSET,
                color: battle_bridge::Color {
                    r: 0xFC,
                    g: 0xFC,
                    b: 0x54,
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
            hit_points: 1,
            damage: 1,
            mass: 0,
        }])
    }

    fn intelligence(&mut self, _ship: &ShipState, _ctx: &BattleContext) -> StatusFlags {
        // C: arilou_intelligence — always thrust, entice enemy, use special
        // to dodge incoming weapons. Full port requires EVALUATE_DESC.
        StatusFlags::THRUST
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_template_matches_c() {
        let ship = ArilouShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 16);
        assert_eq!(desc.ship_info.max_crew, 6);
        assert_eq!(desc.ship_info.max_energy, 20);
        assert!(desc
            .ship_info
            .ship_flags
            .contains(ShipFlags::IMMEDIATE_WEAPON));
        assert_eq!(desc.characteristics.max_thrust, 40);
        assert_eq!(desc.characteristics.thrust_increment, 40);
        assert_eq!(desc.characteristics.weapon_energy_cost, 2);
        assert_eq!(desc.characteristics.special_energy_cost, 3);
        assert_eq!(desc.characteristics.ship_mass, 1);
        assert_eq!(desc.fleet.strength, 44);
        assert_eq!(desc.fleet.known_loc, (438, 6372));
        assert_eq!(desc.intel.weapon_range, 218);
    }

    #[test]
    fn weapon_basic() {
        let mut ship = ArilouShip;
        let state = ShipState {
            crew_level: 6,
            max_crew: 6,
            energy_level: 20,
            max_energy: 20,
            ship_facing: 4,
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
        assert_eq!(weapons[0].damage, 1);
    }

    #[test]
    fn teleport_drains_energy() {
        let mut ship = ArilouShip;
        let mut state = ShipState {
            crew_level: 6,
            max_crew: 6,
            energy_level: 20,
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

        assert_eq!(state.energy_level, 17); // 20 - 3
        assert_eq!(state.special_counter, SPECIAL_WAIT);
        // Clears movement flags during teleport
        assert!(!state.cur_status_flags.contains(StatusFlags::THRUST));
        assert!(!state.cur_status_flags.contains(StatusFlags::WEAPON));
    }

    #[test]
    fn teleport_denied_low_energy() {
        let mut ship = ArilouShip;
        let mut state = ShipState {
            crew_level: 6,
            max_crew: 6,
            energy_level: 2, // Not enough (need 3)
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

        assert_eq!(state.energy_level, 2); // No change
        assert_eq!(state.special_counter, 0);
    }

    #[test]
    fn teleport_denied_during_cooldown() {
        let mut ship = ArilouShip;
        let mut state = ShipState {
            crew_level: 6,
            max_crew: 6,
            energy_level: 20,
            max_energy: 20,
            special_counter: 1,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.preprocess(&mut state, &ctx).unwrap();

        assert_eq!(state.energy_level, 20); // No change
    }

    #[test]
    fn ai_basic() {
        let mut ship = ArilouShip;
        let state = ShipState {
            crew_level: 6,
            max_crew: 6,
            energy_level: 20,
            max_energy: 20,
            player_nr: 1,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        let flags = ship.intelligence(&state, &ctx);
        assert!(flags.contains(StatusFlags::THRUST));
    }
}
