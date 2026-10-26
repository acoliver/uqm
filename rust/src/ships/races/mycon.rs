// Mycon Podship - Tracking plasmoid + regeneration
// @plan PLAN-20260314-SHIPS.P11

#[cfg(not(test))]
use crate::ships::battle_bridge::{self, MissileBlock};
use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

// C: mycon.c constants
const MAX_CREW: u16 = 20;
const MAX_ENERGY: u8 = 40;
const ENERGY_REGENERATION: u8 = 1;
const ENERGY_WAIT: u8 = 4;
const MAX_THRUST: u16 = 27;
const THRUST_INCREMENT: u16 = 9;
const THRUST_WAIT: u8 = 6;
const TURN_WAIT: u8 = 6;
const SHIP_MASS: u8 = 7;

const WEAPON_ENERGY_COST: u8 = 20;
const WEAPON_WAIT: u8 = 5;
#[cfg(not(test))]
const MYCON_OFFSET: i16 = 24;
#[cfg(not(test))]
const MISSILE_OFFSET: i16 = 0;
const NUM_PLASMAS: u16 = 11;
const PLASMA_DURATION: u16 = 13;
const MISSILE_LIFE: u16 = NUM_PLASMAS * PLASMA_DURATION;
const MISSILE_DAMAGE: i16 = 10;
// C constant reserved for full homing-plasma Rust port.
#[expect(dead_code)]
const TRACK_WAIT: u8 = 1;

const SPECIAL_ENERGY_COST: u8 = 40; // MAX_ENERGY
const SPECIAL_WAIT: u8 = 0;
const REGENERATION_AMOUNT: u16 = 4;

#[derive(Debug, Default)]
pub struct MyconShip;

impl ShipBehavior for MyconShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::SEEKING_WEAPON,
                ship_cost: 21,
                crew_level: MAX_CREW,
                max_crew: MAX_CREW,
                energy_level: MAX_ENERGY,
                max_energy: MAX_ENERGY,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 192, // 1070/SPHERE_RADIUS_INCREMENT*2
                known_loc: (6392, 2200),
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
                weapon_range: 3200, // DISPLAY_TO_WORLD(800)
            },
        }
    }

    /// C: mycon_postprocess — regenerates crew on special.
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
        if ship.crew_level >= ship.max_crew {
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

                let mut add_crew = REGENERATION_AMOUNT as i16;
                let headroom = (ship.max_crew - ship.crew_level) as i16;
                if add_crew > headroom {
                    add_crew = headroom;
                }
                battle_bridge::bridge::delta_crew(ship.element_ptr, add_crew);
            }
            return Ok(());
        }

        #[cfg(test)]
        {
            if ship.energy_level < SPECIAL_ENERGY_COST as u16 {
                return Ok(());
            }
            ship.energy_level -= SPECIAL_ENERGY_COST as u16;

            let mut add_crew = REGENERATION_AMOUNT;
            let headroom = ship.max_crew - ship.crew_level;
            if add_crew > headroom {
                add_crew = headroom;
            }
            ship.crew_level += add_crew;

            ship.special_counter = SPECIAL_WAIT;
        }

        Ok(())
    }

    /// C: initialize_plasma — fires tracking plasmoid.
    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        #[cfg(not(test))]
        {
            let missile_speed = battle_bridge::bridge::display_to_world(8) as i16;
            let block = MissileBlock {
                cx: ship.position.0 as i16,
                cy: ship.position.1 as i16,
                flags: 0,
                sender: ship.player_nr,
                pixoffs: MYCON_OFFSET,
                speed: missile_speed,
                hit_points: MISSILE_DAMAGE, // hp = damage for plasmoid
                damage: MISSILE_DAMAGE,
                face: ship.ship_facing as u16,
                index: 0, // Always index 0
                life: MISSILE_LIFE,
                farray: ship.weapon_farray as *mut battle_bridge::Frame,
                preprocess_func: None, // plasma_preprocess handled by C
                blast_offs: MISSILE_OFFSET,
            };
            let _ = battle_bridge::bridge::create_missile(&block);
            Ok(vec![])
        }

        #[cfg(test)]
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (32, 0), // DISPLAY_TO_WORLD(8)
            life_span: MISSILE_LIFE,
            hit_points: MISSILE_DAMAGE as u16,
            damage: MISSILE_DAMAGE as u16,
            mass: 0,
        }])
    }

    fn intelligence(&mut self, _ship: &ShipState, _ctx: &BattleContext) -> StatusFlags {
        // C: mycon_intelligence — complex targeting, regen-when-damaged logic.
        // Full port requires EVALUATE_DESC.
        StatusFlags::THRUST
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_template_matches_c() {
        let ship = MyconShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 21);
        assert_eq!(desc.ship_info.max_crew, 20);
        assert_eq!(desc.ship_info.max_energy, 40);
        assert!(desc.ship_info.ship_flags.contains(ShipFlags::FIRES_FORE));
        assert!(desc
            .ship_info
            .ship_flags
            .contains(ShipFlags::SEEKING_WEAPON));
        assert_eq!(desc.characteristics.weapon_energy_cost, 20);
        assert_eq!(desc.characteristics.special_energy_cost, 40);
        assert_eq!(desc.fleet.known_loc, (6392, 2200));
    }

    #[test]
    fn weapon_basic() {
        let mut ship = MyconShip;
        let state = ShipState {
            crew_level: 20,
            max_crew: 20,
            energy_level: 40,
            max_energy: 40,
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
    fn regeneration_adds_crew() {
        let mut ship = MyconShip;
        let mut state = ShipState {
            crew_level: 16,
            max_crew: 20,
            energy_level: 40,
            max_energy: 40,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.postprocess(&mut state, &ctx).unwrap();

        assert_eq!(state.crew_level, 20); // 16 + 4
        assert_eq!(state.energy_level, 0); // 40 - 40
    }

    #[test]
    fn regeneration_caps_at_max() {
        let mut ship = MyconShip;
        let mut state = ShipState {
            crew_level: 18,
            max_crew: 20,
            energy_level: 40,
            max_energy: 40,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.postprocess(&mut state, &ctx).unwrap();

        assert_eq!(state.crew_level, 20); // Capped at max, not 22
    }

    #[test]
    fn regeneration_denied_at_max_crew() {
        let mut ship = MyconShip;
        let mut state = ShipState {
            crew_level: 20,
            max_crew: 20,
            energy_level: 40,
            max_energy: 40,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.postprocess(&mut state, &ctx).unwrap();

        assert_eq!(state.crew_level, 20);
        assert_eq!(state.energy_level, 40); // No energy spent
    }

    #[test]
    fn regeneration_denied_low_energy() {
        let mut ship = MyconShip;
        let mut state = ShipState {
            crew_level: 10,
            max_crew: 20,
            energy_level: 30,
            max_energy: 40,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.postprocess(&mut state, &ctx).unwrap();

        assert_eq!(state.crew_level, 10); // No change
        assert_eq!(state.energy_level, 30);
    }

    #[test]
    fn ai_basic() {
        let mut ship = MyconShip;
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
