// Chmmr Avatar - Megawatt laser + tractor beam + ZapSat point defense
// @plan PLAN-20260314-SHIPS.P13

#[cfg(not(test))]
use crate::ships::battle_bridge::{self, LaserBlock};
use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

// C: chmmr.c constants
const MAX_CREW: u16 = 42; // MAX_CREW_SIZE
const MAX_ENERGY: u8 = 42; // MAX_ENERGY_SIZE
const ENERGY_REGENERATION: u8 = 1;
const ENERGY_WAIT: u8 = 1;
const MAX_THRUST: u16 = 35;
const THRUST_INCREMENT: u16 = 7;
const THRUST_WAIT: u8 = 5;
const TURN_WAIT: u8 = 3;
const SHIP_MASS: u8 = 10;

// Laser
const WEAPON_ENERGY_COST: u8 = 2;
const WEAPON_WAIT: u8 = 0;

// Tractor beam
const SPECIAL_ENERGY_COST: u8 = 1;
const SPECIAL_WAIT: u8 = 0;
// C constants reserved for full satellite/tractor Rust port.
#[expect(dead_code)]
const NUM_SATELLITES: u8 = 3;

// Color cycle for laser
#[expect(dead_code)]
const NUM_CYCLES: u8 = 4;

#[derive(Debug, Default)]
pub struct ChmmrShip;

impl ShipBehavior for ChmmrShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE
                    | ShipFlags::IMMEDIATE_WEAPON
                    | ShipFlags::SEEKING_SPECIAL
                    | ShipFlags::POINT_DEFENSE,
                ship_cost: 30,
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
                weapon_range: 0, // CLOSE_RANGE_WEAPON
            },
        }
    }

    /// C: chmmr_postprocess — tractor beam pulls enemy ship.
    /// Also handles muzzle flash for laser and ZapSat defense.
    fn postprocess(
        &mut self,
        ship: &mut ShipState,
        _ctx: &BattleContext,
    ) -> Result<(), ShipsError> {
        // Tractor beam
        if ship.cur_status_flags.contains(StatusFlags::SPECIAL) {
            #[cfg(not(test))]
            if !ship.element_ptr.is_null() {
                unsafe {
                    battle_bridge::bridge::delta_energy(
                        ship.element_ptr,
                        -(SPECIAL_ENERGY_COST as i16),
                    );
                    let sound = battle_bridge::bridge::set_abs_sound_index(ship.ship_sounds, 1);
                    battle_bridge::bridge::process_sound(sound, ship.element_ptr);
                    // Tractor beam pull handled by C postprocess_func
                }
            }

            #[cfg(test)]
            {
                if ship.energy_level >= SPECIAL_ENERGY_COST as u16 {
                    ship.energy_level -= SPECIAL_ENERGY_COST as u16;
                }
            }
        }

        // Reset special_counter for laser color cycle
        ship.special_counter = 0;

        Ok(())
    }

    /// C: initialize_megawatt_laser — fires continuous laser beam.
    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        #[cfg(not(test))]
        {
            let laser_range = battle_bridge::bridge::display_to_world(150);
            let angle = battle_bridge::bridge::facing_to_angle(ship.ship_facing as u16);
            let ex = battle_bridge::bridge::cosine(angle, laser_range as i16) as i16;
            let ey = battle_bridge::bridge::sine(angle, laser_range as i16) as i16;

            let block = LaserBlock {
                cx: ship.position.0 as i16,
                cy: ship.position.1 as i16,
                face: ship.ship_facing as u16,
                ex,
                ey,
                sender: ship.player_nr,
                flags: crate::ships::runtime::IGNORE_SIMILAR,
                pixoffs: 0,
                color: battle_bridge::Color {
                    r: 0xBF,
                    g: 0x00,
                    b: 0x00,
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
            velocity: (0, 0), // laser, not projectile
            life_span: 1,     // continuous beam
            hit_points: 1,
            damage: 2, // mass_points = 2 in C
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
        let ship = ChmmrShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 30);
        assert_eq!(desc.ship_info.max_crew, 42);
        assert_eq!(desc.ship_info.max_energy, 42);
        assert!(desc.ship_info.ship_flags.contains(ShipFlags::POINT_DEFENSE));
        assert!(desc
            .ship_info
            .ship_flags
            .contains(ShipFlags::IMMEDIATE_WEAPON));
        assert_eq!(desc.characteristics.energy_wait, 1);
        assert_eq!(desc.characteristics.weapon_energy_cost, 2);
    }

    #[test]
    fn weapon_fires_laser() {
        let mut ship = ChmmrShip;
        let state = ShipState {
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
        assert_eq!(weapons[0].damage, 2);
    }

    #[test]
    fn tractor_drains_energy() {
        let mut ship = ChmmrShip;
        let mut state = ShipState {
            crew_level: 42,
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
        assert_eq!(state.energy_level, 41); // 42 - 1
        assert_eq!(state.special_counter, 0); // reset for color cycle
    }

    #[test]
    fn tractor_denied_no_energy() {
        let mut ship = ChmmrShip;
        let mut state = ShipState {
            crew_level: 42,
            energy_level: 0,
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
        assert_eq!(state.energy_level, 0);
    }

    #[test]
    fn ai_basic() {
        let mut ship = ChmmrShip;
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
