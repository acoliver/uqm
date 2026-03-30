// Sa-Matra (Last Battle) - Boss ship: yellow comets + green sentinels + shield generators
// @plan PLAN-20260314-SHIPS.P13

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

// C: lastbat.c constants
const MAX_CREW: u16 = 1;
const MAX_ENERGY: u8 = 42; // MAX_ENERGY_SIZE
const ENERGY_REGENERATION: u8 = 1;
const ENERGY_WAIT: u8 = 6;
const MAX_THRUST: u16 = 0;
const THRUST_INCREMENT: u16 = 0;
const THRUST_WAIT: u8 = 0;
const TURN_WAIT: u8 = 0;
const SHIP_MASS: u8 = 100; // MAX_SHIP_MASS * 10 (capped at u8)

const WEAPON_ENERGY_COST: u8 = 2;
const WEAPON_WAIT: u8 = 240; // (ONE_SECOND / BATTLE_FRAME_RATE) * 10
const SPECIAL_ENERGY_COST: u8 = 3;
const SPECIAL_WAIT: u8 = 72; // (ONE_SECOND / BATTLE_FRAME_RATE) * 3

const MAX_GENERATORS: u8 = 8;

#[derive(Debug, Default)]
pub struct SamatraShip;

impl ShipBehavior for SamatraShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::IMMEDIATE_WEAPON | ShipFlags::CREW_IMMUNE,
                ship_cost: 16,
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
                weapon_range: 0,
            },
        }
    }

    /// C: samatra_postprocess — spawns yellow comets and green sentinels.
    /// All sub-element logic (comets, sentinels, generators, gates) stays in C.
    fn postprocess(
        &mut self,
        _ship: &mut ShipState,
        _ctx: &BattleContext,
    ) -> Result<(), ShipsError> {
        // Sa-Matra postprocess is entirely C-side sub-element management:
        // - Spawn comets when weapon_counter==0 and active_comets < MAX_COMETS
        // - Spawn sentinels when special_counter==0 and active < MAX_SENTINELS
        // - All gated by num_generators > 0
        Ok(())
    }

    /// Sa-Matra has no standard init_weapon — comets are spawned by postprocess.
    fn init_weapon(
        &mut self,
        _ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        Ok(vec![])
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
        let ship = SamatraShip::default();
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 16);
        assert_eq!(desc.ship_info.max_crew, 1);
        assert_eq!(desc.ship_info.max_energy, 42);
        assert!(desc.ship_info.ship_flags.contains(ShipFlags::CREW_IMMUNE));
        assert!(desc
            .ship_info
            .ship_flags
            .contains(ShipFlags::IMMEDIATE_WEAPON));
        assert_eq!(desc.characteristics.max_thrust, 0);
        assert_eq!(desc.characteristics.ship_mass, 100);
    }

    #[test]
    fn no_standard_weapon() {
        let mut ship = SamatraShip::default();
        let state = ShipState::default();
        let ctx = BattleContext { hyperspace: false, frame_count: 0, gravity_center: None };

        let weapons = ship.init_weapon(&state, &ctx).unwrap();
        assert!(weapons.is_empty());
    }

    #[test]
    fn ai_basic() {
        let mut ship = SamatraShip::default();
        let state = ShipState::default();
        let ctx = BattleContext { hyperspace: false, frame_count: 0, gravity_center: None };

        let flags = ship.intelligence(&state, &ctx);
        assert!(flags.contains(StatusFlags::THRUST));
    }
}
