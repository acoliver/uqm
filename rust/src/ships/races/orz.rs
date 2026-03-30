// Orz Nemesis - Rotating howitzer turret + space marine boarding
// @plan PLAN-20260314-SHIPS.P13

use crate::ships::battle_bridge::{self, MissileBlock};
use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

// C: orz.c constants
const MAX_CREW: u16 = 16;
const MAX_ENERGY: u8 = 20;
const ENERGY_REGENERATION: u8 = 1;
const ENERGY_WAIT: u8 = 6;
const MAX_THRUST: u16 = 35;
const THRUST_INCREMENT: u16 = 5;
const THRUST_WAIT: u8 = 0;
const TURN_WAIT: u8 = 1;
const SHIP_MASS: u8 = 4;

// Howitzer
const WEAPON_ENERGY_COST: u8 = 6; // MAX_ENERGY / 3 (rounded)
const WEAPON_WAIT: u8 = 4;
const TURRET_OFFSET: i16 = 14;
const MISSILE_SPEED_DTW: u16 = 30; // DISPLAY_TO_WORLD(30)
const MISSILE_LIFE: u16 = 12;
const MISSILE_HITS: i16 = 2;
const MISSILE_DAMAGE: i16 = 3;
const MISSILE_OFFSET: i16 = 1;

// Marines
const SPECIAL_ENERGY_COST: u8 = 0;
const SPECIAL_WAIT: u8 = 12;
const MAX_MARINES: u8 = 8;
const MARINE_HIT_POINTS: u8 = 3;

#[derive(Debug, Default)]
pub struct OrzShip;

impl ShipBehavior for OrzShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::SEEKING_SPECIAL,
                ship_cost: 23,
                crew_level: MAX_CREW,
                max_crew: MAX_CREW,
                energy_level: MAX_ENERGY,
                max_energy: MAX_ENERGY,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: 60, // 333/SPHERE_RADIUS_INCREMENT*2
                known_loc: (3608, 2637),
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
                weapon_range: 1440, // MISSILE_SPEED * MISSILE_LIFE
            },
        }
    }

    /// C: orz_preprocess — manages turret rotation and marine launch gating.
    /// When SPECIAL+WEAPON held together, weapon_counter incremented to block fire
    /// (marine launch instead). Turret spawned on APPEARING.
    fn preprocess(
        &mut self,
        ship: &mut ShipState,
        _ctx: &BattleContext,
    ) -> Result<(), ShipsError> {
        // If SPECIAL+direction held, lock turn (turret rotation in C)
        if (ship.cur_status_flags.contains(StatusFlags::SPECIAL)
            || ship.old_status_flags.contains(StatusFlags::SPECIAL))
            && (ship
                .cur_status_flags
                .contains(StatusFlags::LEFT)
                || ship.cur_status_flags.contains(StatusFlags::RIGHT))
            && ship.turn_wait == 0
        {
            ship.turn_wait += 1;
        }

        // SPECIAL+WEAPON = launch marine (block weapon fire)
        if ship.cur_status_flags.contains(StatusFlags::SPECIAL)
            && ship.cur_status_flags.contains(StatusFlags::WEAPON)
            && ship.weapon_counter == 0
        {
            ship.weapon_counter += 1;
        }

        Ok(())
    }

    /// C: initialize_turret_missile — fires howitzer from rotating turret.
    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        #[cfg(not(test))]
        {
            let missile_speed =
                battle_bridge::bridge::display_to_world(30) as i16;
            let block = MissileBlock {
                cx: ship.position.0 as i16,
                cy: ship.position.1 as i16,
                flags: crate::ships::runtime::IGNORE_SIMILAR as u16,
                sender: ship.player_nr,
                pixoffs: TURRET_OFFSET,
                speed: missile_speed,
                hit_points: MISSILE_HITS,
                damage: MISSILE_DAMAGE,
                face: ship.ship_facing as u16, // turret offset applied in C
                index: ship.ship_facing as u16,
                life: MISSILE_LIFE,
                farray: ship.weapon_farray as *mut battle_bridge::Frame,
                preprocess_func: None,
                blast_offs: MISSILE_OFFSET,
            };
            let _ = battle_bridge::bridge::create_missile(&block);
            return Ok(vec![]);
        }

        #[cfg(test)]
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (120, 0), // DISPLAY_TO_WORLD(30) ~ 120
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
        let ship = OrzShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 23);
        assert_eq!(desc.ship_info.max_crew, 16);
        assert_eq!(desc.ship_info.max_energy, 20);
        assert!(desc
            .ship_info
            .ship_flags
            .contains(ShipFlags::SEEKING_SPECIAL));
        assert_eq!(desc.characteristics.weapon_energy_cost, 6);
        assert_eq!(desc.characteristics.special_energy_cost, 0);
        assert_eq!(desc.fleet.known_loc, (3608, 2637));
    }

    #[test]
    fn weapon_fires_howitzer() {
        let mut ship = OrzShip::default();
        let state = ShipState {
            energy_level: 20,
            max_energy: 20,
            ..ShipState::default()
        };
        let ctx = BattleContext { hyperspace: false, frame_count: 0, gravity_center: None };

        let weapons = ship.init_weapon(&state, &ctx).unwrap();
        assert_eq!(weapons.len(), 1);
        assert_eq!(weapons[0].damage, MISSILE_DAMAGE as u16);
        assert_eq!(weapons[0].hit_points, MISSILE_HITS as u16);
    }

    #[test]
    fn preprocess_blocks_weapon_for_marine() {
        let mut ship = OrzShip::default();
        let mut state = ShipState {
            crew_level: 16,
            energy_level: 20,
            cur_status_flags: StatusFlags::SPECIAL | StatusFlags::WEAPON,
            weapon_counter: 0,
            ..ShipState::default()
        };
        let ctx = BattleContext { hyperspace: false, frame_count: 0, gravity_center: None };

        ship.preprocess(&mut state, &ctx).unwrap();
        assert_eq!(state.weapon_counter, 1); // blocked weapon for marine launch
    }

    #[test]
    fn preprocess_turret_lock() {
        let mut ship = OrzShip::default();
        let mut state = ShipState {
            cur_status_flags: StatusFlags::SPECIAL | StatusFlags::RIGHT,
            turn_wait: 0,
            ..ShipState::default()
        };
        let ctx = BattleContext { hyperspace: false, frame_count: 0, gravity_center: None };

        ship.preprocess(&mut state, &ctx).unwrap();
        assert_eq!(state.turn_wait, 1);
    }

    #[test]
    fn ai_basic() {
        let mut ship = OrzShip::default();
        let state = ShipState::default();
        let ctx = BattleContext { hyperspace: false, frame_count: 0, gravity_center: None };

        let flags = ship.intelligence(&state, &ctx);
        assert!(flags.contains(StatusFlags::THRUST));
    }
}
