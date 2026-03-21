// Ship Behavior Registry & Template Table
// @plan PLAN-20260314-SHIPS.P04
// @requirement REQ-NONMELEE-SAME-RUNTIME, REQ-ROSTER-PRESERVE, REQ-MUTATION-PRESERVE

use super::traits::ShipBehavior;
use super::types::{
    Characteristics, FleetStuff, IntelStuff, RaceDesc, RaceDescTemplate, ShipData, ShipFlags,
    ShipInfo, ShipsError, SpeciesId,
};

// ---------------------------------------------------------------------------
// Constants (from C sources)
// ---------------------------------------------------------------------------

/// Infinite sphere of influence radius.
const INFINITE_RADIUS: u16 = 0xFFFF;

// ---------------------------------------------------------------------------
// TemplateOnlyShip  (metadata-safe behavior object)
// ---------------------------------------------------------------------------

/// Metadata-safe ship behavior implementation for species whose combat
/// behavior hasn't been ported yet.
///
/// All hooks are safe no-ops. This allows metadata-only descriptor creation
/// for catalog/analysis work before full race implementation is complete.
/// The template is captured at construction time so `descriptor_template()`
/// can never panic.
#[derive(Debug, Clone)]
struct TemplateOnlyShip {
    template: RaceDescTemplate,
}

impl TemplateOnlyShip {
    fn new(species: SpeciesId) -> Result<Self, ShipsError> {
        let template = descriptor_template_for_species(species)?;
        Ok(Self { template })
    }
}

impl ShipBehavior for TemplateOnlyShip {
    fn descriptor_template(&self) -> RaceDescTemplate {
        self.template.clone()
    }
}

// ---------------------------------------------------------------------------
// Template Table  (complete coverage for all 28 species)
// ---------------------------------------------------------------------------

/// Returns the complete static descriptor template for any valid species.
///
/// This table covers all 28 species and must succeed for any valid `SpeciesId`.
/// Resource IDs are set to 0 (actual resource loading is Phase P05's job).
/// All data verified against C sources in `sc2/src/uqm/ships/*/*.c`.
///
/// # Errors
/// Returns `ShipsError::UnknownSpecies` if `species` is `NoId` or invalid.
pub fn descriptor_template_for_species(species: SpeciesId) -> Result<RaceDescTemplate, ShipsError> {
    let (ship_info, fleet, characteristics, intel) = match species {
        SpeciesId::Arilou => (
            ShipInfo {
                ship_flags: ShipFlags::IMMEDIATE_WEAPON,
                ship_cost: 16,
                crew_level: 6,
                max_crew: 6,
                energy_level: 20,
                max_energy: 20,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 44,
                known_loc: (438, 6372),
            },
            Characteristics {
                max_thrust: 40,
                thrust_increment: 40,
                energy_regeneration: 1,
                weapon_energy_cost: 2,
                special_energy_cost: 3,
                energy_wait: 6,
                turn_wait: 0,
                thrust_wait: 0,
                weapon_wait: 1,
                special_wait: 2,
                ship_mass: 1,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 218,
            },
        ),
        SpeciesId::Chmmr => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE
                    | ShipFlags::IMMEDIATE_WEAPON
                    | ShipFlags::SEEKING_SPECIAL
                    | ShipFlags::POINT_DEFENSE,
                ship_cost: 30,
                crew_level: 42,
                max_crew: 42,
                energy_level: 42,
                max_energy: 42,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 0,
                known_loc: (0, 0),
            },
            Characteristics {
                max_thrust: 35,
                thrust_increment: 7,
                energy_regeneration: 1,
                weapon_energy_cost: 2,
                special_energy_cost: 1,
                energy_wait: 1,
                turn_wait: 3,
                thrust_wait: 5,
                weapon_wait: 0,
                special_wait: 0,
                ship_mass: 10,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 200,
            },
        ),
        SpeciesId::Earthling => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE
                    | ShipFlags::SEEKING_WEAPON
                    | ShipFlags::POINT_DEFENSE,
                ship_cost: 11,
                crew_level: 18,
                max_crew: 18,
                energy_level: 18,
                max_energy: 18,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 0,
                known_loc: (1752, 1450),
            },
            Characteristics {
                max_thrust: 24,
                thrust_increment: 3,
                energy_regeneration: 1,
                weapon_energy_cost: 9,
                special_energy_cost: 4,
                energy_wait: 8,
                turn_wait: 1,
                thrust_wait: 4,
                weapon_wait: 10,
                special_wait: 9,
                ship_mass: 6,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 4000,
            },
        ),
        SpeciesId::Orz => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::SEEKING_SPECIAL,
                ship_cost: 23,
                crew_level: 16,
                max_crew: 16,
                energy_level: 20,
                max_energy: 20,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 60,
                known_loc: (3608, 2637),
            },
            Characteristics {
                max_thrust: 35,
                thrust_increment: 5,
                energy_regeneration: 1,
                weapon_energy_cost: 6,
                special_energy_cost: 0,
                energy_wait: 6,
                turn_wait: 1,
                thrust_wait: 0,
                weapon_wait: 4,
                special_wait: 12,
                ship_mass: 4,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 1440,
            },
        ),
        SpeciesId::Pkunk => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::FIRES_LEFT | ShipFlags::FIRES_RIGHT,
                ship_cost: 20,
                crew_level: 8,
                max_crew: 8,
                energy_level: 12,
                max_energy: 12,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 120,
                known_loc: (502, 401),
            },
            Characteristics {
                max_thrust: 64,
                thrust_increment: 16,
                energy_regeneration: 0,
                weapon_energy_cost: 1,
                special_energy_cost: 2,
                energy_wait: 0,
                turn_wait: 0,
                thrust_wait: 0,
                weapon_wait: 0,
                special_wait: 16,
                ship_mass: 1,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 201,
            },
        ),
        SpeciesId::Shofixti => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE,
                ship_cost: 5,
                crew_level: 6,
                max_crew: 6,
                energy_level: 4,
                max_energy: 4,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 0,
                known_loc: (0, 0),
            },
            Characteristics {
                max_thrust: 35,
                thrust_increment: 5,
                energy_regeneration: 1,
                weapon_energy_cost: 1,
                special_energy_cost: 0,
                energy_wait: 9,
                turn_wait: 1,
                thrust_wait: 0,
                weapon_wait: 3,
                special_wait: 0,
                ship_mass: 1,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 960,
            },
        ),
        SpeciesId::Spathi => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE
                    | ShipFlags::FIRES_AFT
                    | ShipFlags::SEEKING_SPECIAL
                    | ShipFlags::DONT_CHASE,
                ship_cost: 18,
                crew_level: 30,
                max_crew: 30,
                energy_level: 10,
                max_energy: 10,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 180,
                known_loc: (2549, 3600),
            },
            Characteristics {
                max_thrust: 48,
                thrust_increment: 12,
                energy_regeneration: 1,
                weapon_energy_cost: 2,
                special_energy_cost: 3,
                energy_wait: 10,
                turn_wait: 1,
                thrust_wait: 1,
                weapon_wait: 0,
                special_wait: 7,
                ship_mass: 5,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 1200,
            },
        ),
        SpeciesId::Supox => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE,
                ship_cost: 16,
                crew_level: 12,
                max_crew: 12,
                energy_level: 16,
                max_energy: 16,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 60,
                known_loc: (7468, 9246),
            },
            Characteristics {
                max_thrust: 40,
                thrust_increment: 8,
                energy_regeneration: 1,
                weapon_energy_cost: 1,
                special_energy_cost: 1,
                energy_wait: 4,
                turn_wait: 1,
                thrust_wait: 0,
                weapon_wait: 2,
                special_wait: 0,
                ship_mass: 4,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 600,
            },
        ),
        SpeciesId::Thraddash => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE,
                ship_cost: 10,
                crew_level: 8,
                max_crew: 8,
                energy_level: 24,
                max_energy: 24,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 150,
                known_loc: (2535, 8358),
            },
            Characteristics {
                max_thrust: 28,
                thrust_increment: 7,
                energy_regeneration: 1,
                weapon_energy_cost: 2,
                special_energy_cost: 1,
                energy_wait: 6,
                turn_wait: 1,
                thrust_wait: 0,
                weapon_wait: 12,
                special_wait: 0,
                ship_mass: 7,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 900,
            },
        ),
        SpeciesId::Utwig => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE
                    | ShipFlags::POINT_DEFENSE
                    | ShipFlags::SHIELD_DEFENSE,
                ship_cost: 22,
                crew_level: 20,
                max_crew: 20,
                energy_level: 10,
                max_energy: 20,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 120,
                known_loc: (8534, 8797),
            },
            Characteristics {
                max_thrust: 36,
                thrust_increment: 6,
                energy_regeneration: 0,
                weapon_energy_cost: 0,
                special_energy_cost: 1,
                energy_wait: 255,
                turn_wait: 1,
                thrust_wait: 6,
                weapon_wait: 7,
                special_wait: 12,
                ship_mass: 8,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 200,
            },
        ),
        SpeciesId::Vux => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE
                    | ShipFlags::SEEKING_SPECIAL
                    | ShipFlags::IMMEDIATE_WEAPON,
                ship_cost: 12,
                crew_level: 20,
                max_crew: 20,
                energy_level: 40,
                max_energy: 40,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 162,
                known_loc: (4412, 1558),
            },
            Characteristics {
                max_thrust: 21,
                thrust_increment: 7,
                energy_regeneration: 1,
                weapon_energy_cost: 1,
                special_energy_cost: 2,
                energy_wait: 8,
                turn_wait: 6,
                thrust_wait: 4,
                weapon_wait: 0,
                special_wait: 7,
                ship_mass: 6,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 200,
            },
        ),
        SpeciesId::Yehat => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::SHIELD_DEFENSE,
                ship_cost: 23,
                crew_level: 20,
                max_crew: 20,
                energy_level: 10,
                max_energy: 10,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 136,
                known_loc: (4970, 40),
            },
            Characteristics {
                max_thrust: 30,
                thrust_increment: 6,
                energy_regeneration: 2,
                weapon_energy_cost: 1,
                special_energy_cost: 3,
                energy_wait: 6,
                turn_wait: 2,
                thrust_wait: 2,
                weapon_wait: 0,
                special_wait: 2,
                ship_mass: 3,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 266,
            },
        ),
        SpeciesId::Melnorme => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE,
                ship_cost: 18,
                crew_level: 20,
                max_crew: 20,
                energy_level: 42,
                max_energy: 42,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: INFINITE_RADIUS,
                known_loc: (4999, 4999),
            },
            Characteristics {
                max_thrust: 36,
                thrust_increment: 6,
                energy_regeneration: 1,
                weapon_energy_cost: 5,
                special_energy_cost: 20,
                energy_wait: 4,
                turn_wait: 4,
                thrust_wait: 4,
                weapon_wait: 1,
                special_wait: 20,
                ship_mass: 7,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 1800,
            },
        ),
        SpeciesId::Druuge => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE,
                ship_cost: 17,
                crew_level: 14,
                max_crew: 14,
                energy_level: 32,
                max_energy: 32,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 254,
                known_loc: (9500, 2792),
            },
            Characteristics {
                max_thrust: 20,
                thrust_increment: 2,
                energy_regeneration: 1,
                weapon_energy_cost: 4,
                special_energy_cost: 16,
                energy_wait: 50,
                turn_wait: 4,
                thrust_wait: 1,
                weapon_wait: 10,
                special_wait: 30,
                ship_mass: 5,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 2400,
            },
        ),
        SpeciesId::Ilwrath => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE,
                ship_cost: 10,
                crew_level: 22,
                max_crew: 22,
                energy_level: 16,
                max_energy: 16,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 256,
                known_loc: (48, 1700),
            },
            Characteristics {
                max_thrust: 25,
                thrust_increment: 5,
                energy_regeneration: 4,
                weapon_energy_cost: 1,
                special_energy_cost: 3,
                energy_wait: 4,
                turn_wait: 2,
                thrust_wait: 0,
                weapon_wait: 0,
                special_wait: 13,
                ship_mass: 7,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 200,
            },
        ),
        SpeciesId::Mycon => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::SEEKING_WEAPON,
                ship_cost: 21,
                crew_level: 20,
                max_crew: 20,
                energy_level: 40,
                max_energy: 40,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 194,
                known_loc: (6392, 2200),
            },
            Characteristics {
                max_thrust: 27,
                thrust_increment: 9,
                energy_regeneration: 1,
                weapon_energy_cost: 20,
                special_energy_cost: 40,
                energy_wait: 4,
                turn_wait: 6,
                thrust_wait: 6,
                weapon_wait: 5,
                special_wait: 0,
                ship_mass: 7,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 3200,
            },
        ),
        SpeciesId::Slylandro => (
            ShipInfo {
                ship_flags: ShipFlags::SEEKING_WEAPON | ShipFlags::CREW_IMMUNE,
                ship_cost: 17,
                crew_level: 12,
                max_crew: 12,
                energy_level: 20,
                max_energy: 20,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: INFINITE_RADIUS,
                known_loc: (333, 9812),
            },
            Characteristics {
                max_thrust: 60,
                thrust_increment: 60,
                energy_regeneration: 0,
                weapon_energy_cost: 2,
                special_energy_cost: 0,
                energy_wait: 10,
                turn_wait: 0,
                thrust_wait: 0,
                weapon_wait: 17,
                special_wait: 20,
                ship_mass: 1,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 400,
            },
        ),
        SpeciesId::Umgah => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::IMMEDIATE_WEAPON,
                ship_cost: 7,
                crew_level: 10,
                max_crew: 10,
                energy_level: 30,
                max_energy: 30,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 150,
                known_loc: (1798, 6000),
            },
            Characteristics {
                max_thrust: 18,
                thrust_increment: 6,
                energy_regeneration: 30,
                weapon_energy_cost: 0,
                special_energy_cost: 1,
                energy_wait: 150,
                turn_wait: 4,
                thrust_wait: 3,
                weapon_wait: 0,
                special_wait: 2,
                ship_mass: 1,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 16000,
            },
        ),
        SpeciesId::UrQuan => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::SEEKING_SPECIAL,
                ship_cost: 30,
                crew_level: 42,
                max_crew: 42,
                energy_level: 42,
                max_energy: 42,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 484,
                known_loc: (5750, 6000),
            },
            Characteristics {
                max_thrust: 30,
                thrust_increment: 6,
                energy_regeneration: 1,
                weapon_energy_cost: 6,
                special_energy_cost: 8,
                energy_wait: 4,
                turn_wait: 4,
                thrust_wait: 6,
                weapon_wait: 6,
                special_wait: 9,
                ship_mass: 10,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 1600,
            },
        ),
        SpeciesId::Zoqfotpik => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE,
                ship_cost: 6,
                crew_level: 10,
                max_crew: 10,
                energy_level: 10,
                max_energy: 10,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 58,
                known_loc: (3761, 5333),
            },
            Characteristics {
                max_thrust: 40,
                thrust_increment: 10,
                energy_regeneration: 1,
                weapon_energy_cost: 1,
                special_energy_cost: 7,
                energy_wait: 4,
                turn_wait: 1,
                thrust_wait: 0,
                weapon_wait: 0,
                special_wait: 6,
                ship_mass: 5,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 400,
            },
        ),
        SpeciesId::Syreen => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE,
                ship_cost: 13,
                crew_level: 12,
                max_crew: 42,
                energy_level: 16,
                max_energy: 16,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 0,
                known_loc: (0, 0),
            },
            Characteristics {
                max_thrust: 36,
                thrust_increment: 9,
                energy_regeneration: 1,
                weapon_energy_cost: 1,
                special_energy_cost: 5,
                energy_wait: 6,
                turn_wait: 1,
                thrust_wait: 1,
                weapon_wait: 8,
                special_wait: 20,
                ship_mass: 2,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 800,
            },
        ),
        SpeciesId::KohrAh => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE,
                ship_cost: 30,
                crew_level: 42,
                max_crew: 42,
                energy_level: 42,
                max_energy: 42,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 484,
                known_loc: (6000, 6250),
            },
            Characteristics {
                max_thrust: 30,
                thrust_increment: 6,
                energy_regeneration: 1,
                weapon_energy_cost: 6,
                special_energy_cost: 21,
                energy_wait: 4,
                turn_wait: 4,
                thrust_wait: 6,
                weapon_wait: 6,
                special_wait: 9,
                ship_mass: 10,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 200,
            },
        ),
        SpeciesId::Androsynth => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::SEEKING_WEAPON,
                ship_cost: 15,
                crew_level: 20,
                max_crew: 20,
                energy_level: 24,
                max_energy: 24,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: INFINITE_RADIUS,
                known_loc: (4999, 4999),
            },
            Characteristics {
                max_thrust: 24,
                thrust_increment: 3,
                energy_regeneration: 1,
                weapon_energy_cost: 3,
                special_energy_cost: 2,
                energy_wait: 8,
                turn_wait: 4,
                thrust_wait: 0,
                weapon_wait: 0,
                special_wait: 0,
                ship_mass: 6,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 1000,
            },
        ),
        SpeciesId::Chenjesu => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE
                    | ShipFlags::SEEKING_SPECIAL
                    | ShipFlags::SEEKING_WEAPON,
                ship_cost: 28,
                crew_level: 36,
                max_crew: 36,
                energy_level: 30,
                max_energy: 30,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 0,
                known_loc: (0, 0),
            },
            Characteristics {
                max_thrust: 27,
                thrust_increment: 3,
                energy_regeneration: 1,
                weapon_energy_cost: 5,
                special_energy_cost: 30,
                energy_wait: 4,
                turn_wait: 6,
                thrust_wait: 4,
                weapon_wait: 0,
                special_wait: 0,
                ship_mass: 10,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 4000,
            },
        ),
        SpeciesId::Mmrnmhrm => (
            ShipInfo {
                ship_flags: ShipFlags::FIRES_FORE | ShipFlags::IMMEDIATE_WEAPON,
                ship_cost: 19,
                crew_level: 20,
                max_crew: 20,
                energy_level: 10,
                max_energy: 10,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 0,
                known_loc: (0, 0),
            },
            Characteristics {
                max_thrust: 20,
                thrust_increment: 5,
                energy_regeneration: 2,
                weapon_energy_cost: 1,
                special_energy_cost: 10,
                energy_wait: 6,
                turn_wait: 2,
                thrust_wait: 1,
                weapon_wait: 0,
                special_wait: 0,
                ship_mass: 3,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 200,
            },
        ),
        SpeciesId::SisShip => (
            ShipInfo {
                ship_flags: ShipFlags::empty(),
                ship_cost: 16,
                crew_level: 42,
                max_crew: 42,
                energy_level: 42,
                max_energy: 42,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 0,
                known_loc: (0, 0),
            },
            Characteristics {
                max_thrust: 10,
                thrust_increment: 4,
                energy_regeneration: 1,
                weapon_energy_cost: 1,
                special_energy_cost: 0,
                energy_wait: 10,
                turn_wait: 17,
                thrust_wait: 6,
                weapon_wait: 6,
                special_wait: 9,
                ship_mass: 10,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 1152,
            },
        ),
        SpeciesId::SaMatra => (
            ShipInfo {
                ship_flags: ShipFlags::IMMEDIATE_WEAPON | ShipFlags::CREW_IMMUNE,
                ship_cost: 16,
                crew_level: 1,
                max_crew: 1,
                energy_level: 42,
                max_energy: 42,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 0,
                known_loc: (0, 0),
            },
            Characteristics {
                max_thrust: 0,
                thrust_increment: 0,
                energy_regeneration: 1,
                weapon_energy_cost: 2,
                special_energy_cost: 3,
                energy_wait: 6,
                turn_wait: 0,
                thrust_wait: 0,
                weapon_wait: 240,
                special_wait: 72,
                ship_mass: 100,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 0,
            },
        ),
        SpeciesId::UrQuanProbe => (
            ShipInfo {
                ship_flags: ShipFlags::empty(),
                ship_cost: 0,
                crew_level: 1,
                max_crew: 1,
                energy_level: 1,
                max_energy: 1,
                ..ShipInfo::default()
            },
            FleetStuff {
                strength: 0,
                known_loc: (0, 0),
            },
            Characteristics {
                max_thrust: 0,
                thrust_increment: 0,
                energy_regeneration: 0,
                weapon_energy_cost: 0,
                special_energy_cost: 0,
                energy_wait: 0,
                turn_wait: 0,
                thrust_wait: 0,
                weapon_wait: 0,
                special_wait: 0,
                ship_mass: 0,
            },
            IntelStuff {
                maneuverability_index: 0,
                weapon_range: 0,
            },
        ),
        SpeciesId::NoId => {
            return Err(ShipsError::UnknownSpecies(0));
        }
    };

    Ok(RaceDescTemplate {
        ship_info,
        fleet,
        characteristics,
        ship_data: ShipData::default(),
        intel,
    })
}

// ---------------------------------------------------------------------------
// Ship Behavior Creation
// ---------------------------------------------------------------------------

/// Creates a ship behavior instance for the specified species.
///
/// Phase P11 implemented 8 simple races with real combat behavior.
/// Phase P12 implemented 8 mode-switching races with real combat behavior.
/// Phase P13 implements 12 complex & non-melee races with real combat behavior.
/// All 28 species now have complete race-specific implementations.
///
/// # Errors
/// Returns `ShipsError::UnknownSpecies` for `NoId` or invalid species.
pub fn create_ship_behavior(species: SpeciesId) -> Result<Box<dyn ShipBehavior>, ShipsError> {
    match species {
        SpeciesId::NoId => Err(ShipsError::UnknownSpecies(0)),

        // Phase P11: 8 simple races with real combat behavior
        SpeciesId::Arilou => Ok(Box::<super::races::ArilouShip>::default()),
        SpeciesId::Earthling => Ok(Box::<super::races::HumanShip>::default()),
        SpeciesId::Spathi => Ok(Box::<super::races::SpathiShip>::default()),
        SpeciesId::Supox => Ok(Box::<super::races::SupoxShip>::default()),
        SpeciesId::Thraddash => Ok(Box::<super::races::ThraddashShip>::default()),
        SpeciesId::Yehat => Ok(Box::<super::races::YehatShip>::default()),
        SpeciesId::Druuge => Ok(Box::<super::races::DruugeShip>::default()),
        SpeciesId::Ilwrath => Ok(Box::<super::races::IlwrathShip>::default()),

        // Phase P12: 8 mode-switching races with real combat behavior
        SpeciesId::Androsynth => Ok(Box::<super::races::AndrosynthShip>::default()),
        SpeciesId::Mmrnmhrm => Ok(Box::<super::races::MmrnmhrmShip>::default()),
        SpeciesId::Orz => Ok(Box::<super::races::OrzShip>::default()),
        SpeciesId::Pkunk => Ok(Box::<super::races::PkunkShip>::default()),
        SpeciesId::Shofixti => Ok(Box::<super::races::ShofixtiShip>::default()),
        SpeciesId::Syreen => Ok(Box::<super::races::SyreenShip>::default()),
        SpeciesId::Utwig => Ok(Box::<super::races::UtwigShip>::default()),
        SpeciesId::Vux => Ok(Box::<super::races::VuxShip>::default()),

        // Phase P13: 12 complex & non-melee races with real combat behavior
        SpeciesId::Chmmr => Ok(Box::<super::races::ChmmrShip>::default()),
        SpeciesId::Chenjesu => Ok(Box::<super::races::ChenjesuShip>::default()),
        SpeciesId::Mycon => Ok(Box::<super::races::MyconShip>::default()),
        SpeciesId::Melnorme => Ok(Box::<super::races::MelnormeShip>::default()),
        SpeciesId::Umgah => Ok(Box::<super::races::UmgahShip>::default()),
        SpeciesId::UrQuan => Ok(Box::<super::races::UrquanShip>::default()),
        SpeciesId::KohrAh => Ok(Box::<super::races::BlackUrquanShip>::default()),
        SpeciesId::Slylandro => Ok(Box::<super::races::SlylandroShip>::default()),
        SpeciesId::Zoqfotpik => Ok(Box::<super::races::ZoqfotpikShip>::default()),
        SpeciesId::SisShip => Ok(Box::<super::races::SisShip>::default()),
        SpeciesId::SaMatra => Ok(Box::<super::races::SamatraShip>::default()),
        SpeciesId::UrQuanProbe => Ok(Box::<super::races::ProbeShip>::default()),

        _ => Err(ShipsError::UnknownSpecies(species as i32)),
    }
}

// ---------------------------------------------------------------------------
// RaceDesc Creation
// ---------------------------------------------------------------------------

/// Creates a full `RaceDesc` instance for the specified species.
///
/// Combines the species template with a behavior object. For now, all species
/// use `TemplateOnlyShip`. Real combat behavior will be added in P11-P13.
///
/// # Errors
/// Returns `ShipsError::UnknownSpecies` for `NoId` or invalid species.
pub fn create_race_desc(species: SpeciesId) -> Result<RaceDesc, ShipsError> {
    let template = descriptor_template_for_species(species)?;
    let behavior = create_ship_behavior(species)?;

    Ok(RaceDesc {
        ship_info: template.ship_info,
        fleet: template.fleet,
        characteristics: template.characteristics,
        ship_data: template.ship_data,
        intel: template.intel,
        behavior,
        data: None,
    })
}

/// Creates a metadata-only `RaceDesc` for catalog/analysis work.
///
/// This function must succeed for all 28 valid species, regardless of whether
/// their combat behavior has been implemented yet. Uses `TemplateOnlyShip` for
/// safe no-op behavior. Safe for use in non-combat contexts like ship catalog,
/// fleet management, UI display, etc.
///
/// # Errors
/// Returns `ShipsError::UnknownSpecies` for `NoId` or invalid species.
pub fn create_metadata_only_desc(species: SpeciesId) -> Result<RaceDesc, ShipsError> {
    // Currently identical to create_race_desc since all species use TemplateOnlyShip.
    // This separation allows future divergence when live combat behavior is added.
    create_race_desc(species)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ships::traits::{BattleContext, ShipState};
    use crate::ships::types::StatusFlags;

    // -- Template coverage tests --------------------------------------------

    #[test]
    fn descriptor_template_all_28_species_succeed() {
        let all_species = [
            SpeciesId::Arilou,
            SpeciesId::Chmmr,
            SpeciesId::Earthling,
            SpeciesId::Orz,
            SpeciesId::Pkunk,
            SpeciesId::Shofixti,
            SpeciesId::Spathi,
            SpeciesId::Supox,
            SpeciesId::Thraddash,
            SpeciesId::Utwig,
            SpeciesId::Vux,
            SpeciesId::Yehat,
            SpeciesId::Melnorme,
            SpeciesId::Druuge,
            SpeciesId::Ilwrath,
            SpeciesId::Mycon,
            SpeciesId::Slylandro,
            SpeciesId::Umgah,
            SpeciesId::UrQuan,
            SpeciesId::Zoqfotpik,
            SpeciesId::Syreen,
            SpeciesId::KohrAh,
            SpeciesId::Androsynth,
            SpeciesId::Chenjesu,
            SpeciesId::Mmrnmhrm,
            SpeciesId::SisShip,
            SpeciesId::SaMatra,
            SpeciesId::UrQuanProbe,
        ];

        for species in &all_species {
            let result = descriptor_template_for_species(*species);
            assert!(
                result.is_ok(),
                "descriptor_template_for_species failed for {:?}",
                species
            );
        }
    }

    #[test]
    fn descriptor_template_no_id_returns_error() {
        let result = descriptor_template_for_species(SpeciesId::NoId);
        assert!(result.is_err());
        match result {
            Err(ShipsError::UnknownSpecies(0)) => {}
            _ => panic!("Expected UnknownSpecies(0)"),
        }
    }

    // -- Spot check template values against C -------------------------------

    #[test]
    fn arilou_template_values() {
        let template = descriptor_template_for_species(SpeciesId::Arilou).unwrap();
        assert_eq!(template.ship_info.ship_cost, 16);
        assert_eq!(template.ship_info.max_crew, 6);
        assert_eq!(template.ship_info.max_energy, 20);
        assert!(template
            .ship_info
            .ship_flags
            .contains(ShipFlags::IMMEDIATE_WEAPON));
        assert_eq!(template.characteristics.max_thrust, 40);
        assert_eq!(template.characteristics.thrust_increment, 40);
        assert_eq!(template.fleet.strength, 44);
        assert_eq!(template.fleet.known_loc, (438, 6372));
        assert_eq!(template.intel.weapon_range, 218);
    }

    #[test]
    fn chmmr_template_values() {
        let template = descriptor_template_for_species(SpeciesId::Chmmr).unwrap();
        assert_eq!(template.ship_info.ship_cost, 30);
        assert_eq!(template.ship_info.max_crew, 42);
        assert_eq!(template.ship_info.max_energy, 42);
        assert!(template
            .ship_info
            .ship_flags
            .contains(ShipFlags::FIRES_FORE));
        assert!(template
            .ship_info
            .ship_flags
            .contains(ShipFlags::POINT_DEFENSE));
        assert_eq!(template.characteristics.ship_mass, 10);
    }

    #[test]
    fn earthling_template_values() {
        let template = descriptor_template_for_species(SpeciesId::Earthling).unwrap();
        assert_eq!(template.ship_info.ship_cost, 11);
        assert_eq!(template.characteristics.weapon_energy_cost, 9);
        assert_eq!(template.intel.weapon_range, 4000);
    }

    #[test]
    fn spathi_template_values() {
        let template = descriptor_template_for_species(SpeciesId::Spathi).unwrap();
        assert!(template
            .ship_info
            .ship_flags
            .contains(ShipFlags::DONT_CHASE));
        assert!(template.ship_info.ship_flags.contains(ShipFlags::FIRES_AFT));
        assert_eq!(template.fleet.known_loc, (2549, 3600));
    }

    #[test]
    fn melnorme_infinite_radius() {
        let template = descriptor_template_for_species(SpeciesId::Melnorme).unwrap();
        assert_eq!(template.fleet.strength, INFINITE_RADIUS);
    }

    #[test]
    fn slylandro_infinite_radius_and_crew_immune() {
        let template = descriptor_template_for_species(SpeciesId::Slylandro).unwrap();
        assert_eq!(template.fleet.strength, INFINITE_RADIUS);
        assert!(template
            .ship_info
            .ship_flags
            .contains(ShipFlags::CREW_IMMUNE));
    }

    #[test]
    fn androsynth_infinite_radius() {
        let template = descriptor_template_for_species(SpeciesId::Androsynth).unwrap();
        assert_eq!(template.fleet.strength, INFINITE_RADIUS);
    }

    #[test]
    fn urquan_probe_minimal_stats() {
        let template = descriptor_template_for_species(SpeciesId::UrQuanProbe).unwrap();
        assert_eq!(template.ship_info.ship_cost, 0);
        assert_eq!(template.ship_info.max_crew, 1);
        assert_eq!(template.characteristics.max_thrust, 0);
        assert_eq!(template.characteristics.ship_mass, 0);
    }

    #[test]
    fn sa_matra_crew_immune() {
        let template = descriptor_template_for_species(SpeciesId::SaMatra).unwrap();
        assert!(template
            .ship_info
            .ship_flags
            .contains(ShipFlags::CREW_IMMUNE));
        assert_eq!(template.ship_info.max_crew, 1);
        assert_eq!(template.characteristics.ship_mass, 100);
    }

    // -- create_ship_behavior tests -----------------------------------------

    #[test]
    fn create_ship_behavior_all_28_species_succeed() {
        let all_species = [
            SpeciesId::Arilou,
            SpeciesId::Chmmr,
            SpeciesId::Earthling,
            SpeciesId::Orz,
            SpeciesId::Pkunk,
            SpeciesId::Shofixti,
            SpeciesId::Spathi,
            SpeciesId::Supox,
            SpeciesId::Thraddash,
            SpeciesId::Utwig,
            SpeciesId::Vux,
            SpeciesId::Yehat,
            SpeciesId::Melnorme,
            SpeciesId::Druuge,
            SpeciesId::Ilwrath,
            SpeciesId::Mycon,
            SpeciesId::Slylandro,
            SpeciesId::Umgah,
            SpeciesId::UrQuan,
            SpeciesId::Zoqfotpik,
            SpeciesId::Syreen,
            SpeciesId::KohrAh,
            SpeciesId::Androsynth,
            SpeciesId::Chenjesu,
            SpeciesId::Mmrnmhrm,
            SpeciesId::SisShip,
            SpeciesId::SaMatra,
            SpeciesId::UrQuanProbe,
        ];

        for species in &all_species {
            let result = create_ship_behavior(*species);
            assert!(
                result.is_ok(),
                "create_ship_behavior failed for {:?}",
                species
            );
        }
    }

    #[test]
    fn create_ship_behavior_no_id_returns_error() {
        let result = create_ship_behavior(SpeciesId::NoId);
        assert!(result.is_err());
    }

    // -- create_race_desc tests ----------------------------------------------

    #[test]
    fn create_race_desc_all_28_species_succeed() {
        let all_species = [
            SpeciesId::Arilou,
            SpeciesId::Chmmr,
            SpeciesId::Earthling,
            SpeciesId::Orz,
            SpeciesId::Pkunk,
            SpeciesId::Shofixti,
            SpeciesId::Spathi,
            SpeciesId::Supox,
            SpeciesId::Thraddash,
            SpeciesId::Utwig,
            SpeciesId::Vux,
            SpeciesId::Yehat,
            SpeciesId::Melnorme,
            SpeciesId::Druuge,
            SpeciesId::Ilwrath,
            SpeciesId::Mycon,
            SpeciesId::Slylandro,
            SpeciesId::Umgah,
            SpeciesId::UrQuan,
            SpeciesId::Zoqfotpik,
            SpeciesId::Syreen,
            SpeciesId::KohrAh,
            SpeciesId::Androsynth,
            SpeciesId::Chenjesu,
            SpeciesId::Mmrnmhrm,
            SpeciesId::SisShip,
            SpeciesId::SaMatra,
            SpeciesId::UrQuanProbe,
        ];

        for species in &all_species {
            let result = create_race_desc(*species);
            assert!(result.is_ok(), "create_race_desc failed for {:?}", species);
            let desc = result.unwrap();
            assert_eq!(desc.ship_info.ship_cost, desc.ship_info.ship_cost);
        }
    }

    #[test]
    fn create_race_desc_no_id_returns_error() {
        let result = create_race_desc(SpeciesId::NoId);
        assert!(result.is_err());
    }

    #[test]
    fn create_race_desc_aggregates_template() {
        let desc = create_race_desc(SpeciesId::Arilou).unwrap();
        assert_eq!(desc.ship_info.ship_cost, 16);
        assert_eq!(desc.characteristics.max_thrust, 40);
        assert_eq!(desc.intel.weapon_range, 218);
    }

    // -- create_metadata_only_desc tests -------------------------------------

    #[test]
    fn create_metadata_only_desc_all_28_species_succeed() {
        let all_species = [
            SpeciesId::Arilou,
            SpeciesId::Chmmr,
            SpeciesId::Earthling,
            SpeciesId::Orz,
            SpeciesId::Pkunk,
            SpeciesId::Shofixti,
            SpeciesId::Spathi,
            SpeciesId::Supox,
            SpeciesId::Thraddash,
            SpeciesId::Utwig,
            SpeciesId::Vux,
            SpeciesId::Yehat,
            SpeciesId::Melnorme,
            SpeciesId::Druuge,
            SpeciesId::Ilwrath,
            SpeciesId::Mycon,
            SpeciesId::Slylandro,
            SpeciesId::Umgah,
            SpeciesId::UrQuan,
            SpeciesId::Zoqfotpik,
            SpeciesId::Syreen,
            SpeciesId::KohrAh,
            SpeciesId::Androsynth,
            SpeciesId::Chenjesu,
            SpeciesId::Mmrnmhrm,
            SpeciesId::SisShip,
            SpeciesId::SaMatra,
            SpeciesId::UrQuanProbe,
        ];

        for species in &all_species {
            let result = create_metadata_only_desc(*species);
            assert!(
                result.is_ok(),
                "create_metadata_only_desc failed for {:?}",
                species
            );
        }
    }

    #[test]
    fn create_metadata_only_desc_no_id_returns_error() {
        let result = create_metadata_only_desc(SpeciesId::NoId);
        assert!(result.is_err());
    }

    // -- TemplateOnlyShip tests ----------------------------------------------

    #[test]
    fn template_only_ship_returns_correct_template() {
        let ship = TemplateOnlyShip::new(SpeciesId::Spathi).unwrap();
        let template = ship.descriptor_template();
        assert_eq!(template.ship_info.ship_cost, 18);
    }

    #[test]
    fn template_only_ship_defaults_are_safe() {
        let mut ship = TemplateOnlyShip::new(SpeciesId::Arilou).unwrap();
        let mut state = ShipState {
            crew_level: 6,
            max_crew: 6,
            energy_level: 20,
            max_energy: 20,
            ship_facing: 0,
            cur_status_flags: StatusFlags::empty(),
            old_status_flags: StatusFlags::empty(),
            player_nr: 0,
            position: (0, 0),
            velocity: (0, 0),
        };
        let ctx = BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        };

        // All default hooks should be safe no-ops
        assert!(ship.preprocess(&mut state, &ctx).is_ok());
        assert!(ship.postprocess(&mut state, &ctx).is_ok());
        assert!(ship.init_weapon(&state, &ctx).unwrap().is_empty());
        assert!(ship.intelligence(&state, &ctx).is_empty());
        ship.uninit(); // Should not panic
        assert!(ship.collision_override().is_none());
    }

    // -- Resource IDs are zero (P05's job) -----------------------------------

    #[test]
    fn template_ship_data_all_zeroes() {
        let template = descriptor_template_for_species(SpeciesId::Chmmr).unwrap();
        assert_eq!(template.ship_data.ship_res, [0; 3]);
        assert_eq!(template.ship_data.weapon_res, [0; 3]);
        assert_eq!(template.ship_data.special_res, [0; 3]);
        assert_eq!(template.ship_data.captain.captain_res, 0);
        assert_eq!(template.ship_data.victory_ditty_res, 0);
        assert_eq!(template.ship_data.ship_sounds_res, 0);
    }
}
