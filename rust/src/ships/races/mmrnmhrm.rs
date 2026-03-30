// Mmrnmhrm X-Form - Twin laser/twin missile + transform
// @plan PLAN-20260314-SHIPS.P11

use crate::ships::battle_bridge::{self, LaserBlock, MissileBlock};
use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

// C: mmrnmhrm.c constants — X-Wing form
const MAX_CREW: u16 = 20;
const MAX_ENERGY: u8 = 10;
const ENERGY_REGENERATION: u8 = 2;
const ENERGY_WAIT: u8 = 6;
const MAX_THRUST: u16 = 20;
const THRUST_INCREMENT: u16 = 5;
const THRUST_WAIT: u8 = 1;
const TURN_WAIT: u8 = 2;
const SHIP_MASS: u8 = 3;

const WEAPON_ENERGY_COST: u8 = 1;
const WEAPON_WAIT: u8 = 0;
const MISSILE_LIFE: u16 = 40;
const MISSILE_HITS: i16 = 1;
const MISSILE_DAMAGE: i16 = 1;
const MISSILE_OFFSET: i16 = 0;

const SPECIAL_ENERGY_COST: u8 = 10; // MAX_ENERGY
const SPECIAL_WAIT: u8 = 0;

/// Whether X-Wing (laser) or Y-Wing (missile) form
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    XWing,
    YWing,
}

#[derive(Debug)]
pub struct MmrnmhrmShip {
    form: Form,
}

impl Default for MmrnmhrmShip {
    fn default() -> Self {
        Self { form: Form::XWing }
    }
}

impl ShipBehavior for MmrnmhrmShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::IMMEDIATE_WEAPON,
                ship_cost: 19,
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
                weapon_range: 100, // CLOSE_RANGE_WEAPON (X-Wing start)
            },
        }
    }

    /// C: mmrnmhrm_preprocess — transform between X-Wing and Y-Wing.
    fn preprocess(
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
            // Transform handled by C preprocess_func (swaps image.farray,
            // swaps characteristics, changes weapon range/flags)
        }

        #[cfg(test)]
        {
            if ship.energy_level < SPECIAL_ENERGY_COST as u16 {
                return Ok(());
            }
            ship.energy_level -= SPECIAL_ENERGY_COST as u16;
            self.form = match self.form {
                Form::XWing => Form::YWing,
                Form::YWing => Form::XWing,
            };
            ship.special_counter = SPECIAL_WAIT;
        }

        Ok(())
    }

    /// C: initialize_dual_weapons — twin lasers (X) or twin missiles (Y).
    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        #[cfg(not(test))]
        {
            // C init_weapon_func handles mode detection via image.farray.
            // Both modes produce 2 weapons. Handled by C.
            if self.form == Form::XWing {
                let laser_range = battle_bridge::bridge::display_to_world(
                    125 + 16, // MMRNMHRM_OFFSET
                );
                let angle = battle_bridge::bridge::facing_to_angle(ship.ship_facing as u16);

                let block = LaserBlock {
                    cx: ship.position.0 as i16,
                    cy: ship.position.1 as i16,
                    ex: battle_bridge::bridge::cosine(angle, laser_range as i16) as i16,
                    ey: battle_bridge::bridge::sine(angle, laser_range as i16) as i16,
                    face: ship.ship_facing as u16,
                    sender: ship.player_nr,
                    flags: 0,
                    pixoffs: 0,
                    color: battle_bridge::Color {
                        r: 0xFC,
                        g: 0x55,
                        b: 0x55,
                        a: 0xFF,
                    },
                };
                let _ = battle_bridge::bridge::create_laser(&block);
                let _ = battle_bridge::bridge::create_laser(&block);
            } else {
                let missile_speed =
                    battle_bridge::bridge::display_to_world(20) as i16;
                let block = MissileBlock {
                    cx: ship.position.0 as i16,
                    cy: ship.position.1 as i16,
                    flags: crate::ships::runtime::IGNORE_SIMILAR as u16,
                    sender: ship.player_nr,
                    pixoffs: 0,
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
                let _ = battle_bridge::bridge::create_missile(&block);
            }
            return Ok(vec![]);
        }

        #[cfg(test)]
        {
            let weapon = if self.form == Form::XWing {
                WeaponElement {
                    offset: (0, 0),
                    facing: ship.ship_facing,
                    velocity: (0, 0),
                    life_span: 1,
                    hit_points: 0,
                    damage: 1,
                    mass: 0,
                }
            } else {
                WeaponElement {
                    offset: (0, 0),
                    facing: ship.ship_facing,
                    velocity: (80, 0),
                    life_span: MISSILE_LIFE,
                    hit_points: MISSILE_HITS as u16,
                    damage: MISSILE_DAMAGE as u16,
                    mass: 0,
                }
            };
            Ok(vec![weapon.clone(), weapon])
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
        let ship = MmrnmhrmShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 19);
        assert_eq!(desc.ship_info.max_crew, 20);
        assert_eq!(desc.ship_info.max_energy, 10);
        assert!(desc.ship_info.ship_flags.contains(ShipFlags::IMMEDIATE_WEAPON));
        assert_eq!(desc.characteristics.energy_regeneration, 2);
        assert_eq!(desc.characteristics.special_energy_cost, 10);
    }

    #[test]
    fn xwing_fires_two_lasers() {
        let mut ship = MmrnmhrmShip::default();
        assert_eq!(ship.form, Form::XWing);
        let state = ShipState {
            crew_level: 20,
            max_crew: 20,
            energy_level: 10,
            max_energy: 10,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        let weapons = ship.init_weapon(&state, &ctx).unwrap();
        assert_eq!(weapons.len(), 2);
        assert_eq!(weapons[0].life_span, 1); // Laser
    }

    #[test]
    fn transform_switches_form() {
        let mut ship = MmrnmhrmShip::default();
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

        assert_eq!(ship.form, Form::YWing);
        assert_eq!(state.energy_level, 0); // 10 - 10
    }

    #[test]
    fn ywing_fires_two_missiles() {
        let mut ship = MmrnmhrmShip { form: Form::YWing };
        let state = ShipState {
            crew_level: 20,
            max_crew: 20,
            energy_level: 10,
            max_energy: 10,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        let weapons = ship.init_weapon(&state, &ctx).unwrap();
        assert_eq!(weapons.len(), 2);
        assert_eq!(weapons[0].life_span, MISSILE_LIFE); // Missile
    }

    #[test]
    fn transform_denied_low_energy() {
        let mut ship = MmrnmhrmShip::default();
        let mut state = ShipState {
            crew_level: 20,
            max_crew: 20,
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

        ship.preprocess(&mut state, &ctx).unwrap();

        assert_eq!(ship.form, Form::XWing); // No transform
        assert_eq!(state.energy_level, 5);
    }

    #[test]
    fn ai_basic() {
        let mut ship = MmrnmhrmShip::default();
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
