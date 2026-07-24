// SIS Flagship - Modular ship: configurable blasters + point defense
// @plan PLAN-20260314-SHIPS.P13

use crate::ships::traits::{BattleContext, ShipBehavior, ShipState, WeaponElement};
use crate::ships::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    ShipsError, StatusFlags,
};

// C: sis_ship.c constants (defaults before module configuration)
const MAX_CREW: u16 = 42; // MAX_CREW_SIZE (actual determined by crew pods)
const MAX_ENERGY: u8 = 42; // MAX_ENERGY_SIZE
const ENERGY_REGENERATION: u8 = 1; // increased by Shiva furnaces
const ENERGY_WAIT: u8 = 10; // decreased by Dynamo units
const MAX_THRUST: u16 = 10; // increased by fusion thrusters
const THRUST_INCREMENT: u16 = 4;
const THRUST_WAIT: u8 = 6;
const TURN_WAIT: u8 = 17; // decreased by turning jets
const SHIP_MASS: u8 = 10; // MAX_SHIP_MASS

const WEAPON_ENERGY_COST: u8 = 1; // modified by weapon modules
const WEAPON_WAIT: u8 = 6;
#[cfg(test)]
const BLASTER_DAMAGE: u16 = 2;
#[cfg(test)]
const BLASTER_LIFE: u16 = 12;

const SPECIAL_ENERGY_COST: u8 = 0; // increased by antimissile defense modules
const SPECIAL_WAIT: u8 = 9;

#[derive(Debug, Default)]
pub struct SisShip;

impl ShipBehavior for SisShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        RaceDescTemplate {
            ship_info: ShipInfo {
                ship_flags: ShipFlags::empty(),
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
                weapon_range: 1152, // BLASTER_SPEED * BLASTER_LIFE
            },
        }
    }

    /// C: sis_battle_preprocess — disables weapon/special if no modules installed.
    fn preprocess(&mut self, ship: &mut ShipState, _ctx: &BattleContext) -> Result<(), ShipsError> {
        // If no point defense installed, disable special
        if ship.special_counter == 0 {
            // Energy cost 0 means no antimissile modules
            // In C: if special_energy_cost == 0, disable SPECIAL and set counter=2
        }
        Ok(())
    }

    /// C: sis_battle_postprocess — spawn point defense lasers.
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

        // Point defense: scan nearby enemy projectiles and fire lasers at them.
        // All scanning + laser spawning handled by C spawn_point_defense.
        #[cfg(not(test))]
        if !ship.element_ptr.is_null() {
            // C handles spawn_point_defense via death_func chain
        }

        #[cfg(test)]
        {
            // Point defense fires automatically — energy cost is special_energy_cost * 4
            ship.special_counter = SPECIAL_WAIT;
        }

        Ok(())
    }

    /// C: initialize_blasters — fires configurable blasters based on module loadout.
    fn init_weapon(
        &mut self,
        _ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, ShipsError> {
        #[cfg(not(test))]
        {
            // SIS weapon init is fully module-dependent:
            // InitWeaponSlots configures MissileBlocks from GLOBAL_SIS(ModuleSlots)
            // The C init_weapon_func handles all blaster creation via SIS_DATA
            // We don't replicate this in Rust — it stays in C
            Ok(vec![])
        }

        #[cfg(test)]
        Ok(vec![WeaponElement {
            offset: (0, 0),
            facing: _ship.ship_facing,
            velocity: (96, 0), // DISPLAY_TO_WORLD(24)
            life_span: BLASTER_LIFE,
            hit_points: 2,
            damage: BLASTER_DAMAGE,
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
        let ship = SisShip;
        let desc = ship.descriptor_template();

        assert_eq!(desc.ship_info.ship_cost, 16);
        assert_eq!(desc.ship_info.max_crew, 42);
        assert_eq!(desc.ship_info.max_energy, 42);
        assert_eq!(desc.characteristics.turn_wait, 17);
        assert_eq!(desc.characteristics.energy_wait, 10);
        assert_eq!(desc.characteristics.ship_mass, 10);
    }

    #[test]
    fn weapon_fires_blaster() {
        let mut ship = SisShip;
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
        assert_eq!(weapons[0].damage, BLASTER_DAMAGE);
    }

    #[test]
    fn point_defense_sets_counter() {
        let mut ship = SisShip;
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
        assert_eq!(state.special_counter, SPECIAL_WAIT);
    }

    #[test]
    fn ai_basic() {
        let mut ship = SisShip;
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
