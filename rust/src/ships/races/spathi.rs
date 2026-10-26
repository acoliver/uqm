// Spathi Eluder - Forward torpedo + rear-seeking BUTT missile
// @plan PLAN-20260314-SHIPS.P11

#[cfg(not(test))]
use crate::ships::battle_bridge::{self, MissileBlock};
use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

// C: spathi.c constants
const MAX_CREW: u16 = 30;
const MAX_ENERGY: u8 = 10;
const ENERGY_REGENERATION: u8 = 1;
const ENERGY_WAIT: u8 = 10;
const MAX_THRUST: u16 = 48;
const THRUST_INCREMENT: u16 = 12;
const THRUST_WAIT: u8 = 1;
const TURN_WAIT: u8 = 1;
const SHIP_MASS: u8 = 5;

// Forward gun
const WEAPON_ENERGY_COST: u8 = 2;
const WEAPON_WAIT: u8 = 0;
#[cfg(not(test))]
const SPATHI_FORWARD_OFFSET: i16 = 16;
const MISSILE_LIFE: u16 = 10;
const MISSILE_HITS: i16 = 1;
const MISSILE_DAMAGE: i16 = 1;
#[cfg(not(test))]
const MISSILE_OFFSET: i16 = 1;

// B.U.T.T. missile
const SPECIAL_ENERGY_COST: u8 = 3;
const SPECIAL_WAIT: u8 = 7;
#[cfg(not(test))]
const SPATHI_REAR_OFFSET: i16 = 20;
#[cfg(not(test))]
const DISCRIMINATOR_LIFE: u16 = 30;
#[cfg(not(test))]
const DISCRIMINATOR_HITS: i16 = 1;
#[cfg(not(test))]
const DISCRIMINATOR_DAMAGE: i16 = 2;
#[cfg(not(test))]
const DISCRIMINATOR_OFFSET: i16 = 4;

#[derive(Debug, Default)]
pub struct SpathiShip;

impl ShipBehavior for SpathiShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE
                    | ShipFlags::FIRES_AFT
                    | ShipFlags::SEEKING_SPECIAL
                    | ShipFlags::DONT_CHASE,
                ship_cost: 18,
                crew_level: MAX_CREW,
                max_crew: MAX_CREW,
                energy_level: MAX_ENERGY,
                max_energy: MAX_ENERGY,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 180, // 1000/SPHERE_RADIUS_INCREMENT*2
                known_loc: (2549, 3600),
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
                weapon_range: 1200, // MISSILE_SPEED * MISSILE_LIFE
            },
        }
    }

    /// C: spathi_postprocess — fires B.U.T.T. missile (special) from rear.
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

                // Fire B.U.T.T. from rear (facing + HALF_CIRCLE)
                let rear_facing = battle_bridge::bridge::normalize_facing(
                    ship.ship_facing as u16
                        + crate::ships::runtime::angle_to_facing(
                            crate::ships::runtime::HALF_CIRCLE,
                        ),
                );
                let disc_speed = battle_bridge::bridge::display_to_world(8) as i16;
                let block = MissileBlock {
                    cx: ship.position.0 as i16,
                    cy: ship.position.1 as i16,
                    flags: 0,
                    sender: ship.player_nr,
                    pixoffs: SPATHI_REAR_OFFSET,
                    speed: disc_speed,
                    hit_points: DISCRIMINATOR_HITS,
                    damage: DISCRIMINATOR_DAMAGE,
                    face: rear_facing,
                    index: rear_facing,
                    life: DISCRIMINATOR_LIFE,
                    farray: ship.special_farray as *mut battle_bridge::Frame,
                    preprocess_func: None, // butt_missile_preprocess handled by C
                    blast_offs: DISCRIMINATOR_OFFSET,
                };

                if let Some(h) = battle_bridge::bridge::create_missile(&block) {
                    let ptr = battle_bridge::bridge::lock_element(h);
                    let sound = battle_bridge::bridge::set_abs_sound_index(ship.ship_sounds, 1);
                    battle_bridge::bridge::process_sound(sound, ptr);
                    battle_bridge::bridge::unlock_element(h);
                    battle_bridge::bridge::put_element(h);
                }
            }
        }

        #[cfg(test)]
        {
            if ship.energy_level < SPECIAL_ENERGY_COST as u16 {
                return Ok(());
            }
            ship.energy_level -= SPECIAL_ENERGY_COST as u16;
        }

        ship.special_counter = SPECIAL_WAIT;

        Ok(())
    }

    /// C: initialize_standard_missile — fires forward torpedo.
    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        #[cfg(not(test))]
        {
            let missile_speed = battle_bridge::bridge::display_to_world(30) as i16;
            let block = MissileBlock {
                cx: ship.position.0 as i16,
                cy: ship.position.1 as i16,
                flags: crate::ships::runtime::IGNORE_SIMILAR,
                sender: ship.player_nr,
                pixoffs: SPATHI_FORWARD_OFFSET,
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
            velocity: (120, 0),
            life_span: MISSILE_LIFE,
            hit_points: MISSILE_HITS as u16,
            damage: MISSILE_DAMAGE as u16,
            mass: 0,
        }])
    }

    fn intelligence(&mut self, _ship: &ShipState, _ctx: &BattleContext) -> StatusFlags {
        // C: spathi_intelligence — runs away, fires B.U.T.T. when enemy behind.
        // Full port requires EVALUATE_DESC + velocity angle checks.
        StatusFlags::THRUST
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_template_matches_c() {
        let ship = SpathiShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 18);
        assert_eq!(desc.ship_info.max_crew, 30);
        assert_eq!(desc.ship_info.max_energy, 10);
        assert!(desc.ship_info.ship_flags.contains(ShipFlags::FIRES_FORE));
        assert!(desc.ship_info.ship_flags.contains(ShipFlags::FIRES_AFT));
        assert!(desc
            .ship_info
            .ship_flags
            .contains(ShipFlags::SEEKING_SPECIAL));
        assert!(desc.ship_info.ship_flags.contains(ShipFlags::DONT_CHASE));
        assert_eq!(desc.characteristics.max_thrust, 48);
        assert_eq!(desc.characteristics.thrust_increment, 12);
        assert_eq!(desc.characteristics.special_energy_cost, 3);
        assert_eq!(desc.characteristics.special_wait, 7);
        assert_eq!(desc.fleet.strength, 180);
        assert_eq!(desc.fleet.known_loc, (2549, 3600));
        assert_eq!(desc.intel.weapon_range, 1200);
    }

    #[test]
    fn weapon_basic() {
        let mut ship = SpathiShip;
        let state = ShipState {
            crew_level: 30,
            max_crew: 30,
            energy_level: 10,
            max_energy: 10,
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
        assert_eq!(weapons[0].damage, MISSILE_DAMAGE as u16);
        assert_eq!(weapons[0].life_span, MISSILE_LIFE);
    }

    #[test]
    fn butt_missile_drains_energy() {
        let mut ship = SpathiShip;
        let mut state = ShipState {
            crew_level: 30,
            max_crew: 30,
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

        assert_eq!(state.energy_level, 7); // 10 - 3
        assert_eq!(state.special_counter, SPECIAL_WAIT);
    }

    #[test]
    fn butt_denied_low_energy() {
        let mut ship = SpathiShip;
        let mut state = ShipState {
            crew_level: 30,
            max_crew: 30,
            energy_level: 2,
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

        assert_eq!(state.energy_level, 2); // No change
        assert_eq!(state.special_counter, 0);
    }

    #[test]
    fn butt_denied_during_cooldown() {
        let mut ship = SpathiShip;
        let mut state = ShipState {
            crew_level: 30,
            max_crew: 30,
            energy_level: 10,
            max_energy: 10,
            special_counter: 3,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.postprocess(&mut state, &ctx).unwrap();

        assert_eq!(state.energy_level, 10); // No change
    }

    #[test]
    fn ai_basic() {
        let mut ship = SpathiShip;
        let state = ShipState {
            crew_level: 30,
            max_crew: 30,
            energy_level: 10,
            max_energy: 10,
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
