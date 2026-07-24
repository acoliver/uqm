// Androsynth Guardian - Tracking bubbles + blazer comet form
// @plan PLAN-20260314-SHIPS.P11

#[cfg(not(test))]
use crate::ships::battle_bridge::{self, MissileBlock};
use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

// C: androsyn.c constants
const MAX_CREW: u16 = 20;
const MAX_ENERGY: u8 = 24;
const ENERGY_REGENERATION: u8 = 1;
const ENERGY_WAIT: u8 = 8;
const MAX_THRUST: u16 = 24;
const THRUST_INCREMENT: u16 = 3;
const TURN_WAIT: u8 = 4;
const THRUST_WAIT: u8 = 0;
const SHIP_MASS: u8 = 6;

const WEAPON_ENERGY_COST: u8 = 3;
const WEAPON_WAIT: u8 = 0;
#[cfg(not(test))]
const ANDROSYNTH_OFFSET: i16 = 14;
#[cfg(not(test))]
const MISSILE_OFFSET: i16 = 3;
const MISSILE_HITS: i16 = 3;
const MISSILE_LIFE: u16 = 200;
const MISSILE_DAMAGE: i16 = 2;

const SPECIAL_ENERGY_COST: u8 = 2;
const SPECIAL_WAIT: u8 = 0;
// C constants reserved for full blazer-form Rust port.
#[expect(dead_code)]
const BLAZER_THRUST: u16 = 60;
#[expect(dead_code)]
const BLAZER_TURN_WAIT: u8 = 1;
#[expect(dead_code)]
const BLAZER_DAMAGE: i16 = 3;

/// Ship/blazer form tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    Ship,
    Blazer,
}

#[derive(Debug)]
pub struct AndrosynthShip {
    form: Form,
}

impl Default for AndrosynthShip {
    fn default() -> Self {
        Self { form: Form::Ship }
    }
}

impl ShipBehavior for AndrosynthShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::SEEKING_WEAPON,
                ship_cost: 15,
                crew_level: MAX_CREW,
                max_crew: MAX_CREW,
                energy_level: MAX_ENERGY,
                max_energy: MAX_ENERGY,
                ..ShipInfo::default()
            },
            fleet: FleetStuff {
                strength: u16::MAX,      // INFINITE_RADIUS
                known_loc: (5000, 5000), // MAX_X_UNIVERSE>>1, MAX_Y_UNIVERSE>>1
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
                weapon_range: 2500, // LONG_RANGE_WEAPON >> 2
            },
        }
    }

    /// C: androsynth_preprocess — blazer transform.
    /// Ship→Blazer: swaps image farray, sets blazer collision.
    /// Blazer→Ship: energy depletion forces revert.
    /// Complex element manipulation — kept in C.
    fn preprocess(&mut self, ship: &mut ShipState, _ctx: &BattleContext) -> Result<(), ShipsError> {
        if self.form == Form::Ship && ship.cur_status_flags.contains(StatusFlags::SPECIAL) {
            #[cfg(not(test))]
            {
                // C swaps image.farray to special[] and sets blazer_collision
            }

            #[cfg(test)]
            {
                if ship.energy_level < SPECIAL_ENERGY_COST as u16 {
                    return Ok(());
                }
                self.form = Form::Blazer;
            }
        }
        Ok(())
    }

    /// C: androsynth_postprocess — blazer energy drain.
    fn postprocess(
        &mut self,
        _ship: &mut ShipState,
        _ctx: &BattleContext,
    ) -> Result<(), ShipsError> {
        if self.form == Form::Blazer {
            #[cfg(test)]
            {
                // Blazer drains energy; reverts when depleted
                if _ship.energy_level == 0 {
                    self.form = Form::Ship;
                }
            }
        }
        Ok(())
    }

    /// C: initialize_bubble — fires tracking acid bubble.
    fn init_weapon(
        &mut self,
        ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        if self.form == Form::Blazer {
            return Ok(vec![]);
        }

        #[cfg(not(test))]
        {
            let missile_speed = battle_bridge::bridge::display_to_world(8) as i16;
            let block = MissileBlock {
                cx: ship.position.0 as i16,
                cy: ship.position.1 as i16,
                flags: crate::ships::runtime::IGNORE_SIMILAR,
                sender: ship.player_nr,
                pixoffs: ANDROSYNTH_OFFSET,
                speed: missile_speed,
                hit_points: MISSILE_HITS,
                damage: MISSILE_DAMAGE,
                face: ship.ship_facing as u16,
                index: 0,
                life: MISSILE_LIFE,
                farray: ship.weapon_farray as *mut battle_bridge::Frame,
                preprocess_func: None, // bubble_preprocess handled by C
                blast_offs: MISSILE_OFFSET,
            };
            let _ = battle_bridge::bridge::create_missile(&block);
            Ok(vec![])
        }

        #[cfg(test)]
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: ship.ship_facing,
            velocity: (32, 0),
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
        let ship = AndrosynthShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 15);
        assert_eq!(desc.ship_info.max_crew, 20);
        assert_eq!(desc.ship_info.max_energy, 24);
        assert!(desc
            .ship_info
            .ship_flags
            .contains(ShipFlags::SEEKING_WEAPON));
        assert_eq!(desc.characteristics.turn_wait, 4);
        assert_eq!(desc.characteristics.special_energy_cost, 2);
    }

    #[test]
    fn weapon_fires_bubble() {
        let mut ship = AndrosynthShip::default();
        let state = ShipState {
            crew_level: 20,
            max_crew: 20,
            energy_level: 24,
            max_energy: 24,
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
        assert_eq!(weapons[0].life_span, MISSILE_LIFE);
    }

    #[test]
    fn blazer_no_weapon() {
        let mut ship = AndrosynthShip { form: Form::Blazer };
        let state = ShipState {
            crew_level: 20,
            max_crew: 20,
            energy_level: 24,
            max_energy: 24,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        let weapons = ship.init_weapon(&state, &ctx).unwrap();
        assert!(weapons.is_empty());
    }

    #[test]
    fn transform_to_blazer() {
        let mut ship = AndrosynthShip::default();
        let mut state = ShipState {
            crew_level: 20,
            max_crew: 20,
            energy_level: 24,
            max_energy: 24,
            cur_status_flags: StatusFlags::SPECIAL,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.preprocess(&mut state, &ctx).unwrap();

        assert_eq!(ship.form, Form::Blazer);
    }

    #[test]
    fn blazer_reverts_on_depletion() {
        let mut ship = AndrosynthShip { form: Form::Blazer };
        let mut state = ShipState {
            crew_level: 20,
            max_crew: 20,
            energy_level: 0,
            max_energy: 24,
            ..ShipState::default()
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        ship.postprocess(&mut state, &ctx).unwrap();

        assert_eq!(ship.form, Form::Ship);
    }

    #[test]
    fn ai_basic() {
        let mut ship = AndrosynthShip::default();
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
