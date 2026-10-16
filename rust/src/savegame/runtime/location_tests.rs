//! Tests for the solar-location step of a save (`SaveSolarSysLocation`).
//!
//! The expected values are worked out here from solarsys.c and state.c, not
//! produced by the code under test:
//!
//! * `ShipFacing` is the frame's `Index` plus one, the frame being a
//!   `FRAME_DESC` in the field order of drawable.h;
//! * `STARINFO` is a table of one 32-bit offset per star followed, for each
//!   star that was written, by 12-byte scan records: a planet's record, then
//!   one per moon of that planet, planet after planet;
//! * `PLANETARY_CHANGE` is bit 17 of the game state (globdata.h counts
//!   3+2+3+2+1+1+1+1+1+1+1 bits before it), numbered from the low bit of each
//!   byte.
//!
//! The solar-system state is the real `SOLARSYS_STATE` layout wired as
//! `ExploreSolarSys` wires it (see `solar_tests`).

use std::ffi::c_void;

use serial_test::serial;

use super::location::*;
use super::solar::SolarError;
use super::solar_tests::System;
use crate::c_bindings::global_state::{EXTENT, FRAME_DESC, PLANET_DESC, POINT};
use crate::comm::locdata::CGameState;
use crate::state::ffi::{
    replace_game_state, rust_init_game_state, snapshot_game_state, with_state_files,
};
use crate::state::state_file::{StateFileError, STARINFO_FILE};

/// A frame as C lays it out: the generated `FRAME_DESC`, so the tests read and
/// write the same `Index` the code under test does.
fn frame(index: u16) -> Box<FRAME_DESC> {
    Box::new(FRAME_DESC {
        Type: 3,
        Index: index,
        HotSpot: POINT { x: 0, y: 0 },
        Bounds: EXTENT {
            width: 64,
            height: 64,
        },
        image: std::ptr::null_mut(),
        parent: std::ptr::null_mut(),
    })
}

/// A flagship with a recognisable value in every field the step may touch or
/// must leave alone.
fn game(ship_frame: Option<&mut FRAME_DESC>) -> Box<CGameState> {
    // SAFETY: `CGameState` is integers and raw pointers; zero is valid.
    let mut game: Box<CGameState> = Box::new(unsafe { std::mem::zeroed() });
    game.ship_stamp.frame = ship_frame.map_or(std::ptr::null_mut(), |f| {
        std::ptr::from_mut(f).cast::<c_void>()
    });
    game.ship_stamp.origin.x = 301;
    game.ship_stamp.origin.y = -302;
    game.ip_location.x = 4000;
    game.ip_location.y = -5000;
    game.autopilot.x = 77;
    game.velocity.travel_angle = 123;
    game.ship_facing = 0x5555;
    game.ip_planet = 0xAB;
    game.in_orbit = 0x7C;
    game.glob_flags = 9;
    game.battle_group_ref = 0xDEAD_BEEF;
    game
}

/// Same as [`game`] with `ip_planet` set as stage one leaves it.
fn game_in_system(ship_frame: Option<&mut FRAME_DESC>, ip_planet: u8) -> Box<CGameState> {
    let mut game = game(ship_frame);
    game.ip_planet = ip_planet;
    game
}

fn unchanged_apart_from(game: &CGameState, before: &CGameState, touched: &[&str]) {
    let same = |name: &str, equal: bool| {
        assert!(
            touched.contains(&name) || equal,
            "{name} was written and should not be"
        );
    };
    same("ship_facing", game.ship_facing == before.ship_facing);
    same("ip_planet", game.ip_planet == before.ip_planet);
    same("in_orbit", game.in_orbit == before.in_orbit);
    same(
        "ip_location",
        game.ip_location.x == before.ip_location.x && game.ip_location.y == before.ip_location.y,
    );
    assert_eq!(game.ship_stamp.frame, before.ship_stamp.frame);
    assert_eq!(game.ship_stamp.origin.x, before.ship_stamp.origin.x);
    assert_eq!(game.ship_stamp.origin.y, before.ship_stamp.origin.y);
    assert_eq!(game.autopilot.x, before.autopilot.x);
    assert_eq!(game.velocity.travel_angle, before.velocity.travel_angle);
    assert_eq!(game.glob_flags, before.glob_flags);
    assert_eq!(game.battle_group_ref, before.battle_group_ref);
}

fn copy_of(game: &CGameState) -> Box<CGameState> {
    let mut copy = self::game(None);
    copy.ship_stamp.frame = game.ship_stamp.frame;
    copy.ship_stamp.origin = game.ship_stamp.origin;
    copy.ip_location = game.ip_location;
    copy.autopilot = game.autopilot;
    copy.velocity.travel_angle = game.velocity.travel_angle;
    copy.ship_facing = game.ship_facing;
    copy.ip_planet = game.ip_planet;
    copy.in_orbit = game.in_orbit;
    copy.glob_flags = game.glob_flags;
    copy.battle_group_ref = game.battle_group_ref;
    copy
}

const PLANETARY_CHANGE_BYTE: usize = 17 / 8;
const PLANETARY_CHANGE_MASK: u8 = 1 << (17 % 8);

fn planetary_change() -> bool {
    snapshot_game_state().unwrap()[PLANETARY_CHANGE_BYTE] & PLANETARY_CHANGE_MASK != 0
}

fn set_planetary_change() {
    let mut bytes = snapshot_game_state().unwrap();
    bytes[PLANETARY_CHANGE_BYTE] |= PLANETARY_CHANGE_MASK;
    replace_game_state(&bytes).unwrap();
}

/// A fresh game state and a `STARINFO` of `stars` stars nobody has scanned
/// (`InitPlanetInfo`: one zero offset each).
fn fresh_starinfo(stars: usize) {
    // SAFETY: no requirements.
    unsafe { rust_init_game_state() };
    with_state_files(|files| {
        files
            .replace_contents(STARINFO_FILE, &vec![0u8; 4 * stars])
            .unwrap()
    })
    .unwrap();
}

fn starinfo() -> Vec<u8> {
    with_state_files(|files| files.get_file(STARINFO_FILE).unwrap().contents().to_vec()).unwrap()
}

fn words(values: [u32; 3]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// A system of three planets with 1, 2 and 0 moons, in the inner system of
/// `planet`, as `ExploreSolarSys` leaves it after `FillOrbits`.
fn three_planet_system(planet: usize) -> System {
    let mut system = System::inner(planet);
    system.SunDesc[0].NumPlanets = 3;
    for (world, moons) in [1u8, 2, 0].into_iter().enumerate() {
        system.PlanetDesc[world].NumPlanets = moons;
    }
    system
}

fn with_masks(mut system: System, masks: [u32; 3]) -> System {
    system.SysInfo.PlanetInfo.ScanRetrieveMask = masks;
    system
}

// ---------------------------------------------------------------------------
// saveNonOrbitalLocation
// ---------------------------------------------------------------------------

#[test]
fn the_outer_system_records_the_frame_index_plus_one_and_no_planet() {
    let mut ship = frame(5);
    let mut game = game(Some(&mut ship));
    let before = copy_of(&game);
    let system = System::outer();

    // SAFETY: the frame is a live `FRAME_DESC`.
    unsafe { save_solar_location(&mut game, &system, 0) }.unwrap();

    assert_eq!(game.ship_facing, 6);
    assert_eq!(game.in_orbit, 0);
    assert_eq!(game.ip_planet, 0);
    // The outer system keeps the location it had.
    assert_eq!((game.ip_location.x, game.ip_location.y), (4000, -5000));
    unchanged_apart_from(&game, &before, &["ship_facing", "in_orbit", "ip_planet"]);
}

#[test]
fn a_ship_without_a_frame_faces_one() {
    let mut game = game(None);
    let system = System::outer();
    // SAFETY: the frame is NULL.
    unsafe { save_solar_location(&mut game, &system, 0) }.unwrap();
    assert_eq!(game.ship_facing, 1);
}

#[test]
fn the_largest_frame_index_that_fits_is_kept() {
    let mut ship = frame(65534);
    let mut game = game(Some(&mut ship));
    // SAFETY: the frame is a live `FRAME_DESC`.
    unsafe { save_solar_location(&mut game, &System::outer(), 0) }.unwrap();
    assert_eq!(game.ship_facing, 65535);
}

#[test]
fn a_frame_index_that_would_wrap_the_facing_to_zero_is_refused() {
    let mut ship = frame(65535);
    let mut game = game(Some(&mut ship));
    let before = copy_of(&game);
    // SAFETY: the frame is a live `FRAME_DESC`.
    let result = unsafe { save_solar_location(&mut game, &System::outer(), 0) };
    assert_eq!(result, Err(LocationError::FrameIndexOverflow));
    unchanged_apart_from(&game, &before, &[]);
}

#[test]
fn an_inner_system_records_one_based_planet_and_the_suns_location() {
    for planet in 0..16usize {
        let mut ship = frame(12);
        let mut game = game(Some(&mut ship));
        let before = copy_of(&game);
        let mut system = System::inner(planet);
        system.SunDesc[0].location = POINT { x: 11, y: -22 };

        // SAFETY: the frame is a live `FRAME_DESC`.
        unsafe { save_solar_location(&mut game, &system, 0) }.unwrap();

        assert_eq!(game.ip_planet as usize, planet + 1, "planet {planet}");
        assert_eq!((game.ip_location.x, game.ip_location.y), (11, -22));
        assert_eq!(game.ship_facing, 13);
        assert_eq!(game.in_orbit, 0);
        unchanged_apart_from(
            &game,
            &before,
            &["ship_facing", "in_orbit", "ip_planet", "ip_location"],
        );
    }
}

#[test]
fn the_inner_system_of_a_moons_planet_names_that_planet() {
    // Not in orbit but heading for a moon: `planetIndex` of a moon is the
    // planet it orbits.
    let mut system = System::inner(6);
    system.pOrbitalDesc = &mut system.MoonDesc[2];
    let mut game = game(None);
    // SAFETY: the frame is NULL.
    unsafe { save_solar_location(&mut game, &system, 0) }.unwrap();
    assert_eq!(game.ip_planet, 7);
}

#[test]
fn an_inner_system_without_an_orbital_world_is_refused_and_writes_nothing() {
    let mut system = System::inner(3);
    system.pOrbitalDesc = std::ptr::null_mut();
    let mut game = game(None);
    let before = copy_of(&game);
    // SAFETY: the frame is NULL.
    let result = unsafe { save_solar_location(&mut game, &system, 0) };
    assert_eq!(
        result,
        Err(LocationError::Solar(SolarError::OrbitalWorldMissing))
    );
    unchanged_apart_from(&game, &before, &[]);
}

#[test]
fn a_foreign_or_misaligned_orbital_world_is_refused_and_writes_nothing() {
    // SAFETY: all-zero is a valid `PLANET_DESC`.
    let mut foreign: PLANET_DESC = unsafe { std::mem::zeroed() };
    let mut cases: Vec<System> = Vec::new();
    let mut outside = System::inner(1);
    outside.pOrbitalDesc = &mut foreign;
    cases.push(outside);
    let mut misaligned = System::inner(1);
    misaligned.pOrbitalDesc = (misaligned.PlanetDesc.as_ptr() as usize + 1) as *mut PLANET_DESC;
    cases.push(misaligned);

    for system in &cases {
        let mut game = game(None);
        let before = copy_of(&game);
        // SAFETY: the frame is NULL.
        let result = unsafe { save_solar_location(&mut game, system, 0) };
        assert_eq!(
            result,
            Err(LocationError::Solar(SolarError::WorldNotInSystem))
        );
        unchanged_apart_from(&game, &before, &[]);
    }
}

// ---------------------------------------------------------------------------
// SaveSolarSysLocation in orbit
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn orbiting_a_planet_stores_one_and_nothing_else() {
    fresh_starinfo(4);
    let mut ship = frame(9);
    let mut game = game_in_system(Some(&mut ship), 3);
    let before = copy_of(&game);
    let mut system = System::orbiting_planet(2);
    system.SunDesc[0].location = POINT { x: 5, y: 5 };

    // SAFETY: the frame is a live `FRAME_DESC`.
    unsafe { save_solar_location(&mut game, &system, 1) }.unwrap();

    assert_eq!(game.in_orbit, 1);
    // Stage one's work is left as it was, including the facing and the
    // location, which are not the frame's or the sun's now.
    assert_eq!(game.ship_facing, 0x5555);
    assert_eq!(game.ip_planet, 3);
    unchanged_apart_from(&game, &before, &["in_orbit"]);
    assert_eq!(starinfo(), vec![0u8; 16]);
}

#[test]
#[serial]
fn orbiting_a_moon_stores_two_plus_its_index() {
    for moon in 0..4usize {
        fresh_starinfo(4);
        let mut game = game_in_system(None, 4);
        let before = copy_of(&game);
        let system = System::orbiting_moon(3, moon);

        // SAFETY: the frame is NULL.
        unsafe { save_solar_location(&mut game, &system, 0) }.unwrap();

        assert_eq!(game.in_orbit as usize, 2 + moon, "moon {moon}");
        unchanged_apart_from(&game, &before, &["in_orbit"]);
    }
}

#[test]
#[serial]
fn orbit_before_stage_one_is_refused_and_persists_nothing() {
    fresh_starinfo(4);
    set_planetary_change();
    let mut game = game_in_system(None, 0);
    let before = copy_of(&game);
    let mut system = with_masks(three_planet_system(1), [1, 2, 3]);
    system.InOrbit = 1;

    // SAFETY: the frame is NULL.
    let result = unsafe { save_solar_location(&mut game, &system, 1) };

    assert_eq!(result, Err(LocationError::IpPlanetNotSet));
    unchanged_apart_from(&game, &before, &[]);
    assert!(planetary_change());
    assert_eq!(starinfo(), vec![0u8; 16]);
}

#[test]
#[serial]
fn a_planetary_change_writes_the_worlds_scan_record_and_then_clears_the_flag() {
    fresh_starinfo(4);
    set_planetary_change();
    let mut game = game_in_system(None, 2);
    // Planet 1 has 2 moons; orbiting its moon 1 is scan record 2 after its own.
    let mut system = with_masks(three_planet_system(1), [0x11, 0x2222, 0x333333]);
    system.pOrbitalDesc = &mut system.MoonDesc[1];
    system.InOrbit = 1;

    // SAFETY: the frame is NULL.
    unsafe { save_solar_location(&mut game, &system, 3) }.unwrap();

    assert_eq!(game.in_orbit, 3);
    assert!(!planetary_change());

    // Four offsets, then 1+1 + 1+2 + 1+0 = 6 records of 12 bytes. Star 3's
    // offset is where the records start: the end of the old file (16).
    let mut expected = vec![0u8; 12];
    expected.extend_from_slice(&16u32.to_le_bytes());
    for record in 0..6usize {
        // planet 0 owns records 0-1, planet 1 owns 2-4, planet 2 owns 5;
        // planet 1's moon index 1 is the stored moon index 2: record 4.
        if record == 4 {
            expected.extend_from_slice(&words([0x11, 0x2222, 0x333333]));
        } else {
            expected.extend_from_slice(&[0u8; 12]);
        }
    }
    assert_eq!(starinfo(), expected);
}

#[test]
#[serial]
fn the_planet_itself_is_the_first_record_of_its_planet() {
    fresh_starinfo(2);
    set_planetary_change();
    let mut game = game_in_system(None, 3);
    let mut system = with_masks(three_planet_system(2), [7, 8, 9]);
    system.InOrbit = 1;

    // SAFETY: the frame is NULL.
    unsafe { save_solar_location(&mut game, &system, 1) }.unwrap();

    assert_eq!(game.in_orbit, 1);
    let image = starinfo();
    assert_eq!(&image[4..8], &8u32.to_le_bytes());
    // Planet 2 comes after records 0-1 and 2-4: record 5.
    let record = 8 + 5 * 12;
    assert_eq!(&image[record..record + 12], &words([7, 8, 9])[..]);
    assert_eq!(image.len(), 8 + 6 * 12);
    assert!(image[8..record].iter().all(|&b| b == 0));
}

#[test]
#[serial]
fn a_second_world_of_the_same_star_reuses_the_star_record() {
    fresh_starinfo(2);
    let mut game = game_in_system(None, 1);

    set_planetary_change();
    let mut first = with_masks(three_planet_system(0), [1, 2, 3]);
    first.InOrbit = 1;
    // SAFETY: the frame is NULL.
    unsafe { save_solar_location(&mut game, &first, 0) }.unwrap();
    let after_first = starinfo();

    set_planetary_change();
    let mut second = with_masks(three_planet_system(1), [4, 5, 6]);
    second.InOrbit = 1;
    // SAFETY: the frame is NULL.
    unsafe { save_solar_location(&mut game, &second, 0) }.unwrap();
    let after_second = starinfo();

    assert_eq!(after_second.len(), after_first.len());
    assert_eq!(&after_second[..4], &8u32.to_le_bytes());
    assert_eq!(&after_second[8..20], &words([1, 2, 3])[..]);
    // Planet 1 starts after planet 0's two records.
    assert_eq!(&after_second[8 + 24..8 + 36], &words([4, 5, 6])[..]);
}

#[test]
#[serial]
fn without_a_planetary_change_the_state_files_are_left_alone() {
    fresh_starinfo(4);
    let mut game = game_in_system(None, 2);
    let mut system = with_masks(three_planet_system(1), [9, 9, 9]);
    system.InOrbit = 1;

    // SAFETY: the frame is NULL.
    unsafe { save_solar_location(&mut game, &system, 2) }.unwrap();

    assert_eq!(game.in_orbit, 1);
    assert!(!planetary_change());
    assert_eq!(starinfo(), vec![0u8; 16]);
}

#[test]
#[serial]
fn a_refused_scan_record_leaves_the_flag_the_record_and_the_location_alone() {
    fresh_starinfo(4);
    set_planetary_change();
    let mut game = game_in_system(None, 4);
    let before = copy_of(&game);
    // Planet 2 has no moons, so a moon of it has no record.
    let mut system = with_masks(three_planet_system(2), [1, 1, 1]);
    system.pOrbitalDesc = &mut system.MoonDesc[0];
    system.InOrbit = 1;

    // SAFETY: the frame is NULL.
    let result = unsafe { save_solar_location(&mut game, &system, 2) };

    assert_eq!(
        result,
        Err(LocationError::PlanetInfo(StateFileError::ReadOutOfBounds))
    );
    assert!(planetary_change());
    assert_eq!(starinfo(), vec![0u8; 16]);
    unchanged_apart_from(&game, &before, &[]);
}

#[test]
#[serial]
fn a_system_with_more_planets_than_the_table_holds_is_refused() {
    fresh_starinfo(4);
    set_planetary_change();
    let mut game = game_in_system(None, 2);
    let mut system = three_planet_system(1);
    system.SunDesc[0].NumPlanets = 17;
    system.InOrbit = 1;

    // SAFETY: the frame is NULL.
    let result = unsafe { save_solar_location(&mut game, &system, 0) };

    assert_eq!(
        result,
        Err(LocationError::PlanetCountOutOfRange { found: 17 })
    );
    assert!(planetary_change());
    assert_eq!(game.in_orbit, 0x7C);
}

#[test]
#[serial]
fn an_orbit_whose_tables_disagree_about_the_planet_is_refused() {
    fresh_starinfo(4);
    set_planetary_change();
    let mut game = game_in_system(None, 2);
    let before = copy_of(&game);
    let mut system = three_planet_system(1);
    // The inner system belongs to planet 1 but the orbited world is planet 2.
    system.pOrbitalDesc = &mut system.PlanetDesc[2];
    system.InOrbit = 1;

    // SAFETY: the frame is NULL.
    let result = unsafe { save_solar_location(&mut game, &system, 0) };

    assert_eq!(
        result,
        Err(LocationError::Solar(SolarError::WorldDisagreesWithItsTable))
    );
    assert!(planetary_change());
    unchanged_apart_from(&game, &before, &[]);
}

#[test]
#[serial]
fn an_orbit_outside_an_inner_system_is_refused() {
    fresh_starinfo(4);
    set_planetary_change();
    let mut game = game_in_system(None, 2);
    let before = copy_of(&game);
    let mut system = System::outer();
    system.InOrbit = 1;

    // SAFETY: the frame is NULL.
    let result = unsafe { save_solar_location(&mut game, &system, 0) };

    assert_eq!(result, Err(LocationError::OrbitOutsideInnerSystem));
    assert!(planetary_change());
    unchanged_apart_from(&game, &before, &[]);
}
