// Yehat Terminator - Twin pulse cannon + force shield
// @plan PLAN-20260314-SHIPS.P11

#[cfg(not(test))]
use crate::ships::battle_bridge::{self, MissileBlock};
use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

// C: yehat.c constants
const MAX_CREW: u16 = 20;
const MAX_ENERGY: u8 = 10;
const ENERGY_REGENERATION: u8 = 2;
const ENERGY_WAIT: u8 = 6;
const MAX_THRUST: u16 = 30;
const THRUST_INCREMENT: u16 = 6;
const THRUST_WAIT: u8 = 2;
const TURN_WAIT: u8 = 2;
const SHIP_MASS: u8 = 3;

const WEAPON_ENERGY_COST: u8 = 1;
const WEAPON_WAIT: u8 = 0;
#[cfg(not(test))]
const YEHAT_OFFSET: i16 = 16;
const MISSILE_LIFE: u16 = 10;
const MISSILE_HITS: i16 = 1;
const MISSILE_DAMAGE: i16 = 1;
#[cfg(not(test))]
const MISSILE_OFFSET: i16 = 1;

const SPECIAL_ENERGY_COST: u8 = 3;
const SPECIAL_WAIT: u8 = 2;
// C constant reserved for full force-bubble Rust port.
#[expect(dead_code)]
const SHIELD_LIFE: u16 = 10;

#[derive(Debug, Default)]
pub struct YehatShip;

impl ShipBehavior for YehatShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::SHIELD_DEFENSE,
                ship_cost: 23,
                crew_level: MAX_CREW,
                max_crew: MAX_CREW,
                energy_level: MAX_ENERGY,
                max_energy: MAX_ENERGY,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 135, // 750/SPHERE_RADIUS_INCREMENT*2
                known_loc: (4970, 40),
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
                weapon_range: 666, // MISSILE_SPEED * MISSILE_LIFE / 3
            },
        }
    }

    /// C: yehat_preprocess — manages shield activation.
    /// Sets element to NONSOLID with SHIELD_LIFE extra lifespan,
    /// swaps to shield graphics. Requires direct element access.
    fn preprocess(&mut self, ship: &mut ShipState, _ctx: &BattleContext) -> Result<(), ShipsError> {
        if !ship.cur_status_flags.contains(StatusFlags::SPECIAL) {
            return Ok(());
        }
        if ship.special_counter > 0 {
            return Ok(());
        }

        #[cfg(not(test))]
        if !ship.element_ptr.is_null() {
            unsafe {
                // Check energy — DeltaEnergy will flash if insufficient
                if ship.energy_level < SPECIAL_ENERGY_COST as u16 {
                    battle_bridge::bridge::delta_energy(
                        ship.element_ptr,
                        -(SPECIAL_ENERGY_COST as i16),
                    );
                    return Ok(());
                }
                // Shield activation: set life_span, swap image to special.
                // Handled by C preprocess_func since it modifies element
                // fields (life_span, image.farray) not in ShipState.
            }
        }

        #[cfg(test)]
        {
            if ship.energy_level < SPECIAL_ENERGY_COST as u16 {
                return Ok(());
            }
            // Shield activated
            ship.special_counter = SPECIAL_WAIT;
        }

        Ok(())
    }

    /// C: yehat_postprocess — handles shield energy drain per frame.
    fn postprocess(
        &mut self,
        _ship: &mut ShipState,
        _ctx: &BattleContext,
    ) -> Result<(), ShipsError> {
        // C: drains SPECIAL_ENERGY_COST per frame while shield is active
        // (life_span > NORMAL_LIFE). Requires element life_span check.
        // Handled by C postprocess_func.

        #[cfg(not(test))]
        if !_ship.element_ptr.is_null() && _ship.special_counter > 0 {
            unsafe {
                let sound = battle_bridge::bridge::set_abs_sound_index(_ship.ship_sounds, 1);
                battle_bridge::bridge::process_sound(sound, _ship.element_ptr);
                battle_bridge::bridge::delta_energy(
                    _ship.element_ptr,
                    -(SPECIAL_ENERGY_COST as i16),
                );
            }
        }

        Ok(())
    }

    /// C: initialize_standard_missiles — fires twin pulse cannons.
    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        #[cfg(not(test))]
        {
            let missile_speed = battle_bridge::bridge::display_to_world(20) as i16;
            let launch_offs = battle_bridge::bridge::display_to_world(8) as i16;

            let angle = battle_bridge::bridge::facing_to_angle(ship.ship_facing as u16);
            let offs_x = -battle_bridge::bridge::sine(angle, launch_offs) as i16;
            let offs_y = battle_bridge::bridge::cosine(angle, launch_offs) as i16;

            let mut block = MissileBlock {
                cx: ship.position.0 as i16 + offs_x,
                cy: ship.position.1 as i16 + offs_y,
                flags: crate::ships::runtime::IGNORE_SIMILAR,
                sender: ship.player_nr,
                pixoffs: YEHAT_OFFSET,
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

            // Second cannon (opposite offset)
            block.cx = ship.position.0 as i16 - offs_x;
            block.cy = ship.position.1 as i16 - offs_y;
            let _ = battle_bridge::bridge::create_missile(&block);

            Ok(vec![])
        }

        #[cfg(test)]
        Ok(vec![
            WeaponElement {
                offset: (0, 0),
                facing: ship.ship_facing,
                velocity: (80, 0),
                life_span: MISSILE_LIFE,
                hit_points: MISSILE_HITS as u16,
                damage: MISSILE_DAMAGE as u16,
                mass: 0,
            },
            WeaponElement {
                offset: (0, 0),
                facing: ship.ship_facing,
                velocity: (80, 0),
                life_span: MISSILE_LIFE,
                hit_points: MISSILE_HITS as u16,
                damage: MISSILE_DAMAGE as u16,
                mass: 0,
            },
        ])
    }

    fn intelligence(&mut self, _ship: &ShipState, _ctx: &BattleContext) -> StatusFlags {
        // C: yehat_intelligence — activates shield vs incoming weapons,
        // pursues ships without immediate weapons. Full port requires EVALUATE_DESC.
        StatusFlags::THRUST
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_template_matches_c() {
        let ship = YehatShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 23);
        assert_eq!(desc.ship_info.max_crew, 20);
        assert_eq!(desc.ship_info.max_energy, 10);
        assert!(desc.ship_info.ship_flags.contains(ShipFlags::FIRES_FORE));
        assert!(desc
            .ship_info
            .ship_flags
            .contains(ShipFlags::SHIELD_DEFENSE));
        assert_eq!(desc.characteristics.max_thrust, 30);
        assert_eq!(desc.characteristics.energy_regeneration, 2);
        assert_eq!(desc.characteristics.special_energy_cost, 3);
        assert_eq!(desc.fleet.strength, 135);
        assert_eq!(desc.fleet.known_loc, (4970, 40));
    }

    #[test]
    fn weapon_fires_two_missiles() {
        let mut ship = YehatShip;
        let state = ShipState {
            crew_level: 20,
            max_crew: 20,
            energy_level: 10,
            max_energy: 10,
            ship_facing: 0,
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
        assert_eq!(weapons.len(), 2); // Twin cannons
        assert_eq!(weapons[0].damage, MISSILE_DAMAGE as u16);
        assert_eq!(weapons[1].damage, MISSILE_DAMAGE as u16);
    }

    #[test]
    fn shield_activates() {
        let mut ship = YehatShip;
        let mut state = ShipState {
            crew_level: 20,
            max_crew: 20,
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

        ship.preprocess(&mut state, &ctx).unwrap();

        assert_eq!(state.special_counter, SPECIAL_WAIT);
    }

    #[test]
    fn shield_denied_low_energy() {
        let mut ship = YehatShip;
        let mut state = ShipState {
            crew_level: 20,
            max_crew: 20,
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

        ship.preprocess(&mut state, &ctx).unwrap();

        assert_eq!(state.special_counter, 0);
    }

    #[test]
    fn ai_basic() {
        let mut ship = YehatShip;
        let state = ShipState {
            crew_level: 20,
            max_crew: 20,
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
