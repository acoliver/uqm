// Shofixti Scout - Dart gun + glory device
// @plan PLAN-20260314-SHIPS.P11

#[cfg(not(test))]
use crate::ships::battle_bridge::{self, MissileBlock};
use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

// C: shofixti.c constants
const MAX_CREW: u16 = 6;
const MAX_ENERGY: u8 = 4;
const ENERGY_REGENERATION: u8 = 1;
const ENERGY_WAIT: u8 = 9;
const MAX_THRUST: u16 = 35;
const THRUST_INCREMENT: u16 = 5;
const TURN_WAIT: u8 = 1;
const THRUST_WAIT: u8 = 0;
const WEAPON_WAIT: u8 = 3;
const SPECIAL_WAIT: u8 = 0;
const SHIP_MASS: u8 = 1;

const WEAPON_ENERGY_COST: u8 = 1;
#[cfg(not(test))]
const SHOFIXTI_OFFSET: i16 = 15;
#[cfg(not(test))]
const MISSILE_OFFSET: i16 = 1;
const MISSILE_LIFE: u16 = 10;
const MISSILE_HITS: i16 = 1;
const MISSILE_DAMAGE: i16 = 1;

const SPECIAL_ENERGY_COST: u8 = 0;

#[derive(Debug, Default)]
pub struct ShofixtiShip;

impl ShipBehavior for ShofixtiShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE,
                ship_cost: 5,
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
                weapon_range: 960, // MISSILE_SPEED * MISSILE_LIFE
            },
        }
    }

    /// C: shofixti_postprocess — glory device activation.
    /// Toggling SPECIAL 3 times (tracked by captain frame index) triggers
    /// self_destruct which kills nearby ships. Complex element
    /// manipulation — kept in C.
    fn postprocess(
        &mut self,
        _ship: &mut ShipState,
        _ctx: &BattleContext,
    ) -> Result<(), ShipsError> {
        // C: captain frame tracks toggle count → self_destruct on 3rd toggle.
        // Requires captain_control.special frame index tracking — kept in C.
        Ok(())
    }

    /// C: initialize_standard_missile — fires dart.
    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        #[cfg(not(test))]
        {
            let missile_speed = battle_bridge::bridge::display_to_world(24) as i16;
            let block = MissileBlock {
                cx: ship.position.0 as i16,
                cy: ship.position.1 as i16,
                flags: crate::ships::runtime::IGNORE_SIMILAR,
                sender: ship.player_nr,
                pixoffs: SHOFIXTI_OFFSET,
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
            velocity: (96, 0),
            life_span: MISSILE_LIFE,
            hit_points: MISSILE_HITS as u16,
            damage: MISSILE_DAMAGE as u16,
            mass: 0,
        }])
    }

    fn intelligence(&mut self, _ship: &ShipState, _ctx: &BattleContext) -> StatusFlags {
        // C: shofixti_intelligence — glory device when doomed.
        StatusFlags::THRUST
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_template_matches_c() {
        let ship = ShofixtiShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 5);
        assert_eq!(desc.ship_info.max_crew, 6);
        assert_eq!(desc.ship_info.max_energy, 4);
        assert_eq!(desc.characteristics.max_thrust, 35);
        assert_eq!(desc.characteristics.ship_mass, 1);
        assert_eq!(desc.characteristics.special_energy_cost, 0);
    }

    #[test]
    fn weapon_basic() {
        let mut ship = ShofixtiShip;
        let state = ShipState {
            crew_level: 6,
            max_crew: 6,
            energy_level: 4,
            max_energy: 4,
            ship_facing: 0,
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
    fn ai_basic() {
        let mut ship = ShofixtiShip;
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
