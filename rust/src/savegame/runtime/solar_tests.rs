//! Tests for the Rust port of the solar-system reads of `PrepareSummary`.
//!
//! Fixtures retain the production owner and select bodies through its typed transitions.
//! A supplied game-string table makes the selected string index observable.
//!
//! Expected string indexes are worked out here from the `*_COUNT` defines of
//! `gamestr.h`, and the star indexes from the `gendef.h` enum, not from the
//! constants the code under test uses. The expected answers are those of
//! `GetNamedPlanetaryBody`/`GetPlanetOrMoonName` read from solarsys.c.

use super::solar::*;
use crate::c_bindings::global_state;
use crate::engine::world::generation::Body;
use crate::planets::solar_system::SolarSystem;
use crate::savegame::types::SIS_NAME_SIZE;
use std::ops::{Deref, DerefMut};

/// STAR 133 + DEVICE 29 + CARGO 10 + ELEMENTS 133 + SCAN 56 + STAR_NUMBER 14.
const PLANET_BASE: u16 = 133 + 29 + 10 + 133 + 56 + 14;
/// PLANET_BASE + PLANET_NUMBER 33 + MONTHS 12 + FEEDBACK 2.
const STARBASE_BASE: u16 = PLANET_BASE + 33 + 12 + 2;

/// `gendef.h`: SOL, SHOFIXTI, MAIDENS, START_COLONY, SPATHI, ZOQFOT, nine
/// MELNORMEs, then TALKING_PET .. SAMATRA.
const SOL: u8 = 1;
const SPATHI: u8 = 5;
const ZOQFOT: u8 = 6;
const SAMATRA: u8 = 29;

/// The names in the game string table, `s<index>`.
pub(super) struct Table;

impl GameStrings for Table {
    fn game_string(&self, index: u16) -> Vec<u8> {
        format!("s{index}").into_bytes()
    }
}

/// Fixtures retain the production owner and use its real topology transitions.
pub(super) struct System(pub SolarSystem);
impl Deref for System {
    type Target = SolarSystem;
    fn deref(&self) -> &SolarSystem {
        &self.0
    }
}
impl DerefMut for System {
    fn deref_mut(&mut self) -> &mut SolarSystem {
        &mut self.0
    }
}
impl System {
    pub(super) fn outer() -> Self {
        let mut world = crate::engine::World::new();
        world.set_catalog(std::borrow::Cow::Borrowed(
            &crate::planets::tables::STARMAP_ARRAY,
        ));
        let mut solar = SolarSystem::new(&world, crate::engine::StarId(0));
        solar.sys.sun.num_planets = 16;
        for planet in &mut solar.sys.planets {
            planet.num_planets = 4;
        }
        Self(solar)
    }
    pub(super) fn inner(planet: usize) -> Self {
        let mut system = Self::outer();
        system.enter_inner(planet as u8);
        system
    }
    pub(super) fn orbiting_planet(planet: usize) -> Self {
        let mut system = Self::inner(planet);
        system.enter_orbit(Body::Planet(planet as u8));
        system
    }
    pub(super) fn orbiting_moon(planet: usize, moon: usize) -> Self {
        let mut system = Self::inner(planet);
        system.enter_orbit(Body::Moon {
            planet: planet as u8,
            moon: moon as u8,
        });
        system
    }
}

pub(super) const FLAGSHIP_NAME: [u8; SIS_NAME_SIZE] = *b"Beta Hydri\0\0\0\0\0\0";

pub(super) fn owner(
    state: &System,
    star: Option<u8>,
    sis_planet_name: [u8; SIS_NAME_SIZE],
) -> SolarSummary<'_> {
    let mut snapshot = state.snapshot((320, 240));
    snapshot.generator_index = star.unwrap_or(0);
    SolarSummary::new(Some(snapshot), star, sis_planet_name, &Table)
}

pub(super) fn text(name: [u8; SIS_NAME_SIZE]) -> String {
    let end = name.iter().position(|&b| b == 0).unwrap_or(SIS_NAME_SIZE);
    String::from_utf8(name[..end].to_vec()).expect("test names are ASCII")
}

fn named(system: &System, star: Option<u8>) -> String {
    text(
        owner(system, star, FLAGSHIP_NAME)
            .planet_or_moon_name()
            .expect("a well-formed system"),
    )
}

// ---------------------------------------------------------------------------
// The numbers the port takes from the headers
// ---------------------------------------------------------------------------

#[test]
fn the_header_numbers_are_the_ones_worked_out_from_gamestr_h_and_gendef_h() {
    assert_eq!(global_state::PLANET_NUMBER_BASE as u16, PLANET_BASE);
    assert_eq!(global_state::STARBASE_STRING_BASE as u16, STARBASE_BASE);
    assert_eq!(global_state::SOL_DEFINED as u8, SOL);
    assert_eq!(global_state::SPATHI_DEFINED as u8, SPATHI);
    assert_eq!(global_state::SAMATRA_DEFINED as u8, SAMATRA);
    assert_eq!(PLANET_BASE + 32, super::summary::SA_MATRA_STRING);
}

#[test]
fn moving_the_owner_preserves_body_and_persistence_identity_without_rewiring() {
    let system = System::orbiting_moon(2, 1);
    let before = system.snapshot((320, 240));
    let moved = Box::new(system);
    let after = moved.snapshot((320, 240));
    assert_eq!(before, after);
    assert_eq!(
        moved
            .body_location(Body::Moon { planet: 2, moon: 1 })
            .node_key(31),
        (0, 2, 2, 31)
    );
}

// ---------------------------------------------------------------------------
// playerInPlanetOrbit
// ---------------------------------------------------------------------------

#[test]
fn without_a_solar_system_there_is_no_orbit_and_no_moon_name() {
    let solar = SolarSummary::new(None, Some(SOL), FLAGSHIP_NAME, &Table);
    assert!(!solar.player_in_planet_orbit());
    assert_eq!(solar.planet_or_moon_name(), Ok(FLAGSHIP_NAME));
}

#[test]
fn the_orbit_test_is_the_in_orbit_flag_of_the_live_state() {
    let mut system = System::inner(2);
    assert!(!owner(&system, Some(SOL), FLAGSHIP_NAME).player_in_planet_orbit());
    system.enter_orbit(Body::Planet(2));
    assert!(owner(&system, Some(SOL), FLAGSHIP_NAME).player_in_planet_orbit());
    system.finish_orbit();
    assert!(!owner(&system, Some(SOL), FLAGSHIP_NAME).player_in_planet_orbit());
}

#[test]
fn an_encounter_is_not_inferred_to_be_an_orbit_from_flight_being_stopped() {
    let mut system = System::orbiting_moon(2, 1);
    system.in_ip_flight = false;
    assert!(owner(&system, Some(SOL), FLAGSHIP_NAME).player_in_planet_orbit());
    system.finish_orbit();
    assert!(!owner(&system, Some(SOL), FLAGSHIP_NAME).player_in_planet_orbit());
}

// ---------------------------------------------------------------------------
// GetNamedPlanetaryBody / GetPlanetOrMoonName
// ---------------------------------------------------------------------------

#[test]
fn the_outer_system_keeps_the_flagships_planet_name() {
    let system = System::outer();
    assert_eq!(named(&system, Some(SOL)), "Beta Hydri");
}

#[test]
fn every_planet_of_sol_is_named_by_its_planet_number() {
    for planet in 0..global_state::MAX_PLANETS as usize {
        let system = System::orbiting_planet(planet);
        assert_eq!(
            named(&system, Some(SOL)),
            format!("s{}", PLANET_BASE + planet as u16),
            "planet {planet}"
        );
    }
}

#[test]
fn the_named_moons_of_sol_read_the_strings_solarsys_c_names() {
    let expected = [
        (2, 0, STARBASE_BASE),
        (2, 1, PLANET_BASE + 9),
        (4, 0, PLANET_BASE + 10),
        (4, 1, PLANET_BASE + 11),
        (4, 2, PLANET_BASE + 12),
        (4, 3, PLANET_BASE + 13),
        (5, 0, PLANET_BASE + 14),
        (7, 0, PLANET_BASE + 15),
    ];
    for (planet, moon, index) in expected {
        let system = System::orbiting_moon(planet, moon);
        assert_eq!(
            named(&system, Some(SOL)),
            format!("s{index}"),
            "planet {planet} moon {moon}"
        );
    }
}

#[test]
fn the_other_moons_of_sol_take_the_flagship_name_and_a_letter() {
    let named_moons = [
        (2, 0),
        (2, 1),
        (4, 0),
        (4, 1),
        (4, 2),
        (4, 3),
        (5, 0),
        (7, 0),
    ];
    for planet in 0..global_state::MAX_PLANETS as usize {
        for moon in 0..global_state::MAX_MOONS as usize {
            if named_moons.contains(&(planet, moon)) {
                continue;
            }
            let system = System::orbiting_moon(planet, moon);
            let letter = (b'A' + moon as u8) as char;
            assert_eq!(
                named(&system, Some(SOL)),
                format!("Beta Hydri-{letter}"),
                "planet {planet} moon {moon}"
            );
        }
    }
}

#[test]
fn only_sol_and_sa_matra_have_named_bodies() {
    // Earth's moon and Jupiter's moon, in a system that is not Sol.
    for (planet, moon) in [(2, 1), (4, 0)] {
        let system = System::orbiting_moon(planet, moon);
        let letter = (b'A' + moon as u8) as char;
        assert_eq!(named(&system, Some(ZOQFOT)), format!("Beta Hydri-{letter}"));
    }
    let planet = System::orbiting_planet(2);
    assert_eq!(named(&planet, Some(ZOQFOT)), "Beta Hydri");
}

#[test]
fn spathis_first_planet_is_not_named() {
    let system = System::orbiting_planet(0);
    assert_eq!(named(&system, Some(SPATHI)), "Beta Hydri");
}

#[test]
fn sa_matra_is_the_first_moon_of_the_fifth_planet_of_its_system() {
    let sa_matra = System::orbiting_moon(4, 0);
    assert_eq!(
        named(&sa_matra, Some(SAMATRA)),
        format!("s{}", PLANET_BASE + 32)
    );
    for (planet, moon) in [(4, 1), (3, 0), (5, 0), (0, 0)] {
        let system = System::orbiting_moon(planet, moon);
        let letter = (b'A' + moon as u8) as char;
        assert_eq!(
            named(&system, Some(SAMATRA)),
            format!("Beta Hydri-{letter}"),
            "planet {planet} moon {moon}"
        );
    }
    // The fifth planet itself is not named, and Sol's Jupiter moon is not
    // Sa-Matra's.
    assert_eq!(
        named(&System::orbiting_planet(4), Some(SAMATRA)),
        "Beta Hydri"
    );
    assert_eq!(
        named(&sa_matra, Some(SOL)),
        format!("s{}", PLANET_BASE + 10)
    );
}

#[test]
fn bodies_are_not_named_while_the_star_is_unknown() {
    let system = System::orbiting_planet(2);
    assert_eq!(named(&system, None), "Beta Hydri");
}

#[test]
fn a_moon_letter_needs_room_for_three_bytes() {
    let system = System::orbiting_moon(0, 1);
    let with = |name: &[u8]| {
        let mut buf = [0u8; SIS_NAME_SIZE];
        buf[..name.len()].copy_from_slice(name);
        text(
            owner(&system, Some(ZOQFOT), buf)
                .planet_or_moon_name()
                .unwrap(),
        )
    };
    assert_eq!(with(b""), "-B");
    assert_eq!(with(b"Twelve chars"), "Twelve chars-B");
    // 13 bytes leave exactly "-B\0".
    assert_eq!(with(b"Thirteen char"), "Thirteen char-B");
    // 14 and 15 leave less than three bytes: the name stays as it was.
    assert_eq!(with(b"Fourteen chars"), "Fourteen chars");
    assert_eq!(with(b"Fifteen chars!!"), "Fifteen chars!!");
}

#[test]
fn a_name_that_fills_the_buffer_is_cut_where_utf8_string_copy_cuts_it() {
    let system = System::orbiting_planet(0);
    let full = *b"0123456789abcdef";
    let name = owner(&system, Some(ZOQFOT), full)
        .planet_or_moon_name()
        .unwrap();
    assert_eq!(&name[..15], b"0123456789abcde");
    assert_eq!(name[15], 0);
}

#[test]
fn the_final_battle_names_sa_matra_from_the_string_table() {
    let system = System::outer();
    let solar = owner(&system, None, FLAGSHIP_NAME);
    let name = solar.last_battle_planet_name();
    assert_eq!(text(name), format!("s{}", PLANET_BASE + 32));
    let none = SolarSummary::new(None, None, FLAGSHIP_NAME, &Table);
    assert_eq!(none.last_battle_planet_name(), name);
}

#[test]
fn a_copy_with_a_missing_selected_body_propagates_the_summary_error() {
    let system = System::inner(2);
    let mut snapshot = system.snapshot((320, 240));
    snapshot.selected = None;
    assert_eq!(
        SolarSummary::new(Some(snapshot), Some(SOL), FLAGSHIP_NAME, &Table).planet_or_moon_name(),
        Err(SolarError::OrbitalWorldMissing)
    );
}

#[test]
fn invalid_body_transitions_do_not_mutate_the_owners_selection() {
    let mut system = System::inner(2);
    for body in [Body::Planet(3), Body::Moon { planet: 2, moon: 4 }] {
        let before = system.snapshot((320, 240));
        assert!(std::panic::catch_unwind(
            std::panic::AssertUnwindSafe(|| system.enter_orbit(body))
        )
        .is_err());
        assert_eq!(system.snapshot((320, 240)), before);
    }
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| system.enter_inner(16))).is_err()
    );
    assert_eq!(system.selected_body(), Some(Body::Planet(2)));
}
