//! Tests for the Rust port of the solar-system reads of `PrepareSummary`.
//!
//! The state under test is a real `SOLARSYS_STATE` in bindgen's layout, wired
//! the way `FillOrbits`/`ExploreSolarSys` wire it: every `PlanetDesc` orbits
//! `SunDesc[0]`, every `MoonDesc` orbits the planet whose inner system is
//! entered, `pBaseDesc` is `PlanetDesc` in the outer system and `MoonDesc`
//! in an inner one, `pOrbitalDesc` points into one of the tables. The game
//! string table is a stand-in whose text for index `i` is `s<i>`, so a test
//! sees which `GAME_STRING` index was read.
//!
//! Expected string indexes are worked out here from the `*_COUNT` defines of
//! `gamestr.h`, and the star indexes from the `gendef.h` enum, not from the
//! constants the code under test uses. The expected answers are those of
//! `GetNamedPlanetaryBody`/`GetPlanetOrMoonName` read from solarsys.c.

use std::mem::{offset_of, size_of};
use std::ops::{Deref, DerefMut};

use super::solar::*;
use super::summary::SolarSystemOwner;
use crate::c_bindings::global_state::{self, PLANET_DESC, SOLARSYS_STATE};
use crate::savegame::types::SIS_NAME_SIZE;

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

/// A solar system wired the way the C game wires it.
pub(super) struct System(Box<SOLARSYS_STATE>);

impl Deref for System {
    type Target = SOLARSYS_STATE;
    fn deref(&self) -> &SOLARSYS_STATE {
        &self.0
    }
}

impl DerefMut for System {
    fn deref_mut(&mut self) -> &mut SOLARSYS_STATE {
        &mut self.0
    }
}

impl System {
    /// The outer system: `pBaseDesc` is `PlanetDesc`, nothing is orbited.
    pub(super) fn outer() -> Self {
        // SAFETY: all-zero is a valid `SOLARSYS_STATE` (NULL pointers, zero
        // numbers, `None` for the input function).
        let mut state: Box<SOLARSYS_STATE> = Box::new(unsafe { std::mem::zeroed() });
        let sun: *mut PLANET_DESC = state.SunDesc.as_mut_ptr();
        for planet in state.PlanetDesc.iter_mut() {
            planet.pPrevDesc = sun;
        }
        state.pBaseDesc = state.PlanetDesc.as_mut_ptr();
        Self(state)
    }

    /// Fly into the inner system of `planet`.
    pub(super) fn inner(planet: usize) -> Self {
        let mut system = Self::outer();
        let host: *mut PLANET_DESC = &mut system.0.PlanetDesc[planet];
        for moon in system.0.MoonDesc.iter_mut() {
            moon.pPrevDesc = host;
        }
        system.0.pBaseDesc = system.0.MoonDesc.as_mut_ptr();
        system.0.pOrbitalDesc = host;
        system
    }

    /// In orbit around `planet` itself.
    pub(super) fn orbiting_planet(planet: usize) -> Self {
        let mut system = Self::inner(planet);
        system.0.InOrbit = 1;
        system
    }

    /// In orbit around moon `moon` of `planet`.
    pub(super) fn orbiting_moon(planet: usize, moon: usize) -> Self {
        let mut system = Self::inner(planet);
        system.0.pOrbitalDesc = &mut system.0.MoonDesc[moon];
        system.0.InOrbit = 1;
        system
    }
}

pub(super) const FLAGSHIP_NAME: [u8; SIS_NAME_SIZE] = *b"Beta Hydri\0\0\0\0\0\0";

pub(super) fn owner(
    state: &System,
    star: Option<u8>,
    sis_planet_name: [u8; SIS_NAME_SIZE],
) -> SolarSystem<'_> {
    SolarSystem::new(Some(state), star, sis_planet_name, &Table)
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
fn the_bindgen_layout_is_the_one_the_hand_written_solar_views_assume() {
    // `planet_side::orbit_scan` reads `pOrbitalDesc` 1032 bytes in
    // (8 + 4 + 2 + 2 pad, 21 worlds of 48 bytes, then `pBaseDesc`).
    assert_eq!(size_of::<PLANET_DESC>(), 48);
    assert_eq!(global_state::MAX_SUNS, 1);
    assert_eq!(global_state::MAX_PLANETS, 16);
    assert_eq!(global_state::MAX_MOONS, 4);
    assert_eq!(offset_of!(SOLARSYS_STATE, SunDesc), 16);
    assert_eq!(offset_of!(SOLARSYS_STATE, pBaseDesc), 1024);
    assert_eq!(offset_of!(SOLARSYS_STATE, pOrbitalDesc), 1032);
}

// ---------------------------------------------------------------------------
// playerInPlanetOrbit
// ---------------------------------------------------------------------------

#[test]
fn without_a_solar_system_there_is_no_orbit_and_no_moon_name() {
    let solar = SolarSystem::new(None, Some(SOL), FLAGSHIP_NAME, &Table);
    assert!(!solar.player_in_planet_orbit());
    assert_eq!(solar.planet_or_moon_name(), Ok(FLAGSHIP_NAME));
}

#[test]
fn the_orbit_test_is_the_in_orbit_flag_of_the_live_state() {
    let mut system = System::inner(2);
    assert!(!owner(&system, Some(SOL), FLAGSHIP_NAME).player_in_planet_orbit());
    system.InOrbit = 1;
    assert!(owner(&system, Some(SOL), FLAGSHIP_NAME).player_in_planet_orbit());
    system.InOrbit = 0;
    assert!(!owner(&system, Some(SOL), FLAGSHIP_NAME).player_in_planet_orbit());
}

#[test]
fn the_orbit_test_does_not_look_at_the_tables() {
    // `playerInPlanetOrbit` is `playerInSolarSystem () && InOrbit`: in the
    // outer system a stale orbit flag still answers true, as in C.
    let mut system = System::outer();
    system.InOrbit = 1;
    assert!(owner(&system, None, FLAGSHIP_NAME).player_in_planet_orbit());
}

// ---------------------------------------------------------------------------
// GetNamedPlanetaryBody / GetPlanetOrMoonName
// ---------------------------------------------------------------------------

#[test]
fn the_outer_system_keeps_the_flagships_planet_name() {
    let mut system = System::outer();
    // Not read in the outer system, even when it is garbage.
    system.pOrbitalDesc = 0x10 as *mut PLANET_DESC;
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
    let none = SolarSystem::new(None, None, FLAGSHIP_NAME, &Table);
    assert_eq!(none.last_battle_planet_name(), name);
}

// ---------------------------------------------------------------------------
// States the C code would index through garbage
// ---------------------------------------------------------------------------

#[test]
fn an_inner_system_without_an_orbital_world_is_an_error() {
    let mut system = System::inner(2);
    system.pOrbitalDesc = std::ptr::null_mut();
    assert_eq!(
        owner(&system, Some(SOL), FLAGSHIP_NAME).planet_or_moon_name(),
        Err(SolarError::OrbitalWorldMissing)
    );
}

#[test]
fn a_world_outside_both_tables_is_an_error() {
    let mut foreign: Box<PLANET_DESC> = Box::new(unsafe { std::mem::zeroed() });
    let mut system = System::inner(2);
    system.pOrbitalDesc = &mut *foreign;
    assert_eq!(
        owner(&system, Some(SOL), FLAGSHIP_NAME).planet_or_moon_name(),
        Err(SolarError::WorldNotInSystem)
    );
    // One past the last moon is outside as well.
    system.pOrbitalDesc = system.MoonDesc.as_mut_ptr_range().end;
    assert_eq!(
        owner(&system, Some(SOL), FLAGSHIP_NAME).planet_or_moon_name(),
        Err(SolarError::WorldNotInSystem)
    );
}

#[test]
fn a_world_between_two_table_entries_is_an_error() {
    let mut system = System::inner(2);
    let misaligned = system.PlanetDesc.as_mut_ptr().cast::<u8>().wrapping_add(1);
    system.pOrbitalDesc = misaligned.cast::<PLANET_DESC>();
    assert_eq!(
        owner(&system, Some(SOL), FLAGSHIP_NAME).planet_or_moon_name(),
        Err(SolarError::WorldNotInSystem)
    );
    let misaligned = system
        .MoonDesc
        .as_mut_ptr()
        .cast::<u8>()
        .wrapping_add(size_of::<PLANET_DESC>() - 1);
    system.pOrbitalDesc = misaligned.cast::<PLANET_DESC>();
    assert_eq!(
        owner(&system, Some(SOL), FLAGSHIP_NAME).planet_or_moon_name(),
        Err(SolarError::WorldNotInSystem)
    );
}

#[test]
fn a_world_that_disagrees_with_its_table_is_an_error() {
    // A planet that does not orbit the sun.
    let mut planet = System::inner(2);
    let other: *mut PLANET_DESC = &mut planet.PlanetDesc[3];
    planet.PlanetDesc[2].pPrevDesc = other;
    assert_eq!(
        owner(&planet, Some(SOL), FLAGSHIP_NAME).planet_or_moon_name(),
        Err(SolarError::WorldDisagreesWithItsTable)
    );

    // A moon that orbits the sun.
    let mut moon = System::orbiting_moon(2, 1);
    let sun: *mut PLANET_DESC = moon.SunDesc.as_mut_ptr();
    moon.MoonDesc[1].pPrevDesc = sun;
    assert_eq!(
        owner(&moon, Some(SOL), FLAGSHIP_NAME).planet_or_moon_name(),
        Err(SolarError::WorldDisagreesWithItsTable)
    );

    // A moon whose planet is not in `PlanetDesc`.
    let mut moon = System::orbiting_moon(2, 1);
    let mut foreign: Box<PLANET_DESC> = Box::new(unsafe { std::mem::zeroed() });
    moon.MoonDesc[1].pPrevDesc = &mut *foreign;
    assert_eq!(
        owner(&moon, Some(SOL), FLAGSHIP_NAME).planet_or_moon_name(),
        Err(SolarError::WorldDisagreesWithItsTable)
    );
}

#[test]
#[should_panic(expected = "neither PlanetDesc nor MoonDesc")]
fn a_base_table_that_is_neither_planets_nor_moons_is_a_broken_invariant() {
    let mut system = System::inner(2);
    system.pBaseDesc = system.SunDesc.as_mut_ptr();
    let _ = owner(&system, Some(SOL), FLAGSHIP_NAME).planet_or_moon_name();
}
