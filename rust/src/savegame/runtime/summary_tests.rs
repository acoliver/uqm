//! Tests for the live "Summ" and "Star" capture.
//!
//! The game structures are the ones the other runtime tests use: a real
//! `GAME_STATE` with `InitQueue` pools, a `SIS_STATE` laid out by the `repr(C)`
//! mirror, and the Rust game-state singleton replaced the way a load does.
//! Expected values are written out here from the bytes of a save the C game
//! wrote (`C_WRITTEN_SAVE`, read with offsets worked out from `save.c`), from
//! `globdata.h` and `planets/devices.c`, and from the text of `plandata.c`.
//! None comes from the encoder or from the code under test.
//!
//! Most of these tests answer for the solar-system state with a stand-in that
//! records how often it was asked; what they check is when `PrepareSummary`'s
//! branches consult it and what they do with the answer. The last group of
//! solar tests runs the real Rust computation (`solar`) over a wired
//! `SOLARSYS_STATE` and checks what ends up in the summary.

use std::cell::Cell;

use serial_test::serial;

use super::queue_tests::{LiveGame, C_WRITTEN_SAVE};
use super::solar::{SolarError, SolarSystem};
use super::solar_tests::{owner, text, System, Table, FLAGSHIP_NAME};
use super::star::*;
use super::summary::*;
use crate::comm::dispatch::{IN_INTERPLANETARY, IN_LAST_BATTLE};
use crate::comm::locdata::CSisState;
use crate::savegame::types::{Point, StarDesc};
use crate::state::ffi::replace_game_state;
use crate::state::game_state::NUM_GAME_STATE_BYTES;

// Chunk positions inside C_WRITTEN_SAVE, worked out from `save.c`.
const FIXTURE_SUMM: std::ops::Range<usize> = 12..172;
const FIXTURE_GLST_ACTIVITY: usize = 211;
const FIXTURE_GMST: std::ops::Range<usize> = 263..418;

const START_ENCOUNTER: u16 = 0x0400;

fn le16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn le32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

// ---------------------------------------------------------------------------
// Game-state bits, set with a loop that does not share the code under test
// ---------------------------------------------------------------------------

type StateBytes = [u8; NUM_GAME_STATE_BYTES];

fn set_bits(bytes: &mut StateBytes, start: usize, width: usize, value: u8) {
    for bit in 0..width {
        let at = start + bit;
        if value >> bit & 1 == 1 {
            bytes[at / 8] |= 1 << (at % 8);
        } else {
            bytes[at / 8] &= !(1 << (at % 8));
        }
    }
}

/// Start bit and width of a game-state key, summed up from the
/// `ADD_GAME_STATE (name, bits)` lines of `globdata.h` the way the C enum does.
fn key(name: &str) -> (usize, usize) {
    let header = include_str!("../../../../sc2/src/uqm/globdata.h");
    let list = &header[header.find("\nSTART_GAME_STATE\n").unwrap()..];
    let list = &list[..list.find("\nEND_GAME_STATE").unwrap()];
    let mut next = 0usize;
    for line in list.lines() {
        let Some(args) = line.trim().strip_prefix("ADD_GAME_STATE (") else {
            continue;
        };
        let (key_name, rest) = args.split_once(',').unwrap();
        let width: usize = rest.split(')').next().unwrap().trim().parse().unwrap();
        if key_name.trim() == name {
            return (next, width);
        }
        next += width;
    }
    panic!("{name} is not in globdata.h");
}

fn set_key(bytes: &mut StateBytes, name: &str, value: u8) {
    let (start, width) = key(name);
    set_bits(bytes, start, width, value);
}

fn install(bytes: &StateBytes) {
    replace_game_state(bytes).expect("the singleton is not poisoned");
}

/// The one-bit "on ship" flags, by device id, as `InventoryDevices` reads them
/// (planets/devices.c).
const DEVICE_FLAGS: [(u8, &str); 20] = [
    (0, "PORTAL_SPAWNER_ON_SHIP"),
    (1, "TALKING_PET_ON_SHIP"),
    (2, "UTWIG_BOMB_ON_SHIP"),
    (3, "SUN_DEVICE_ON_SHIP"),
    (4, "ROSY_SPHERE_ON_SHIP"),
    (5, "AQUA_HELIX_ON_SHIP"),
    (6, "CLEAR_SPINDLE_ON_SHIP"),
    (11, "MAIDENS_ON_SHIP"),
    (12, "UMGAH_BROADCASTERS_ON_SHIP"),
    (13, "BURV_BROADCASTERS_ON_SHIP"),
    (17, "TAALO_PROTECTOR_ON_SHIP"),
    (18, "EGG_CASE0_ON_SHIP"),
    (19, "EGG_CASE1_ON_SHIP"),
    (20, "EGG_CASE2_ON_SHIP"),
    (21, "SYREEN_SHUTTLE_ON_SHIP"),
    (22, "VUX_BEAST_ON_SHIP"),
    (24, "PORTAL_KEY_ON_SHIP"),
    (25, "ARTIFACT_2_ON_SHIP"),
    (26, "ARTIFACT_3_ON_SHIP"),
    (27, "MOONBASE_ON_SHIP"),
];

// ---------------------------------------------------------------------------
// A stand-in for the solar-system owner
// ---------------------------------------------------------------------------

struct Solar {
    planet: [u8; 16],
    orbit: bool,
    sa_matra: [u8; 16],
    name_asks: Cell<u32>,
    orbit_asks: Cell<u32>,
    last_battle_asks: Cell<u32>,
}

impl Solar {
    fn new() -> Self {
        Self {
            planet: *b"Moon-B\0\0\0\0\0\0\0\0\0\0",
            orbit: false,
            sa_matra: *b"Sa-Matra\0\0\0\0\0\0\0\0",
            name_asks: Cell::new(0),
            orbit_asks: Cell::new(0),
            last_battle_asks: Cell::new(0),
        }
    }
}

impl SolarSystemOwner for Solar {
    fn planet_or_moon_name(&self) -> Result<[u8; 16], SolarError> {
        self.name_asks.set(self.name_asks.get() + 1);
        Ok(self.planet)
    }
    fn player_in_planet_orbit(&self) -> bool {
        self.orbit_asks.set(self.orbit_asks.get() + 1);
        self.orbit
    }
    fn last_battle_planet_name(&self) -> [u8; 16] {
        self.last_battle_asks.set(self.last_battle_asks.get() + 1);
        self.sa_matra
    }
}

fn sis() -> CSisState {
    CSisState {
        log_x: -123_456,
        log_y: 654_321,
        res_units: 0xA1B2_C3D4,
        fuel_on_board: 0x0102_0304,
        crew_enlisted: 0x1112,
        total_element_mass: 0x2122,
        total_bio_mass: 0x3132,
        module_slots: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
        drive_slots: [21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31],
        jet_slots: [41, 42, 43, 44, 45, 46, 47, 48],
        num_landers: 3,
        element_amounts: [
            0x0101, 0x0202, 0x0303, 0x0404, 0x0505, 0x0606, 0x0707, 0x0808,
        ],
        ship_name: *b"Vind\xc3\xa9cator\0\0\0\0\0",
        commander_name: *b"Zelnick\0\0\0\0\0\0\0\0\0",
        planet_name: *b"Alpha Centauri\0\0",
    }
}

fn blank_state() -> StateBytes {
    [0u8; NUM_GAME_STATE_BYTES]
}

// ---------------------------------------------------------------------------
// Summ against the save the C game wrote
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn summary_of_the_game_the_c_save_came_from_matches_what_c_wrote() {
    assert_eq!(&C_WRITTEN_SAVE[4..8], b"Summ");
    assert_eq!(le32(&C_WRITTEN_SAVE[8..12]), 160);
    let raw = &C_WRITTEN_SAVE[FIXTURE_SUMM];

    // The live owners, set up from the other chunks of that save.
    let mut state = blank_state();
    let gmst = &C_WRITTEN_SAVE[FIXTURE_GMST];
    state[..gmst.len()].copy_from_slice(gmst);
    install(&state);

    let mut game = LiveGame::bare();
    game.current_activity = le16(&C_WRITTEN_SAVE[FIXTURE_GLST_ACTIVITY..]);
    game.game_clock.day_index = raw[124];
    game.game_clock.month_index = raw[125];
    game.game_clock.year_index = le16(&raw[126..]);
    game.push_fragment(false, raw[132], 0);

    let flagship = CSisState {
        log_x: le32(&raw[0..]) as i32,
        log_y: le32(&raw[4..]) as i32,
        res_units: le32(&raw[8..]),
        fuel_on_board: le32(&raw[12..]),
        crew_enlisted: le16(&raw[16..]),
        total_element_mass: le16(&raw[18..]),
        total_bio_mass: le16(&raw[20..]),
        module_slots: raw[22..38].try_into().unwrap(),
        drive_slots: raw[38..49].try_into().unwrap(),
        jet_slots: raw[49..57].try_into().unwrap(),
        num_landers: raw[57],
        element_amounts: std::array::from_fn(|i| le16(&raw[58 + 2 * i..])),
        ship_name: raw[74..90].try_into().unwrap(),
        commander_name: raw[90..106].try_into().unwrap(),
        planet_name: raw[106..122].try_into().unwrap(),
    };

    // The C game named the planet with nothing, so the owner answers with
    // nothing; the activity is the C one's.
    let mut solar = Solar::new();
    solar.planet = raw[106..122].try_into().unwrap();
    let summary = capture_summary(&flagship, &game, &solar, b"").unwrap();

    assert_eq!(game.current_activity & 0xff, 4);
    assert_eq!(summary.activity, raw[122]);
    assert_eq!(summary.flags, raw[123]);
    assert_eq!(summary.day_index, 17);
    assert_eq!(summary.month_index, 2);
    assert_eq!(summary.year_index, 2155);
    assert_eq!(summary.mcredit_lo, raw[128]);
    assert_eq!(summary.mcredit_hi, raw[129]);
    assert_eq!(summary.num_ships, raw[130]);
    assert_eq!(summary.num_devices, raw[131]);
    assert_eq!(&summary.ship_list[..], &raw[132..144]);
    assert_eq!(&summary.device_list[..], &raw[144..160]);
    assert_eq!(summary.sis.log_x, 27_987);
    assert_eq!(summary.sis.fuel_on_board, 1000);
    assert_eq!(&summary.sis.module_slots[..], &raw[22..38]);
    assert_eq!(&summary.sis.ship_name[..], &raw[74..90]);
    assert_eq!(&summary.sis.ship_name[..10], b"Vindicator");
    assert_eq!(&summary.sis.commander_name[..7], b"Zelnick");
    assert_eq!(&summary.sis.planet_name[..], &raw[106..122]);
    assert!(summary.save_name.as_bytes().is_empty());
}

#[test]
#[serial]
fn summary_copies_the_flagship_state_field_by_field() {
    install(&blank_state());
    let mut game = LiveGame::bare();
    game.current_activity = 3;
    let flagship = sis();
    let summary = capture_summary(&flagship, &game, &Solar::new(), b"x").unwrap();
    assert_eq!(summary.sis.log_x, -123_456);
    assert_eq!(summary.sis.log_y, 654_321);
    assert_eq!(summary.sis.res_units, 0xA1B2_C3D4);
    assert_eq!(summary.sis.fuel_on_board, 0x0102_0304);
    assert_eq!(summary.sis.crew_enlisted, 0x1112);
    assert_eq!(summary.sis.total_element_mass, 0x2122);
    assert_eq!(summary.sis.total_bio_mass, 0x3132);
    assert_eq!(summary.sis.module_slots, flagship.module_slots);
    assert_eq!(summary.sis.drive_slots, flagship.drive_slots);
    assert_eq!(summary.sis.jet_slots, flagship.jet_slots);
    assert_eq!(summary.sis.num_landers, 3);
    assert_eq!(summary.sis.element_amounts, flagship.element_amounts);
    assert_eq!(&summary.sis.ship_name[..], b"Vind\xc3\xa9cator\0\0\0\0\0");
    assert_eq!(&summary.sis.commander_name[..7], b"Zelnick");
    // Hyperspace keeps the planet name the flagship holds.
    assert_eq!(&summary.sis.planet_name[..], b"Alpha Centauri\0\0");
}

// ---------------------------------------------------------------------------
// The activity of a summary
// ---------------------------------------------------------------------------

/// `PrepareSummary`'s switch, written out from `save.c`.
fn expected_activity(lobyte: u8, arilou_side: u8, starbase: bool, orbit: bool) -> u8 {
    match lobyte {
        3 if arilou_side > 1 => 6,
        4 if starbase => 8,
        4 if orbit => 7,
        other => other,
    }
}

#[test]
#[serial]
fn the_activity_of_a_summary_follows_prepare_summary_for_every_state() {
    for arilou in 0u8..4 {
        for starbase in [false, true] {
            let mut state = blank_state();
            set_key(&mut state, "ARILOU_SPACE_SIDE", arilou);
            if starbase {
                set_key(&mut state, "GLOBAL_FLAGS_AND_DATA", 0xFF);
            }
            install(&state);
            let mut game = LiveGame::bare();
            for orbit in [false, true] {
                let mut solar = Solar::new();
                solar.orbit = orbit;
                for lobyte in 0u16..=8 {
                    for high in 0u16..128 {
                        game.current_activity = lobyte | (high << 8);
                        let summary = capture_summary(&sis(), &game, &solar, b"s").unwrap();
                        assert_eq!(
                            summary.activity,
                            expected_activity(lobyte as u8, arilou, starbase, orbit),
                            "activity {lobyte:#x} high {high:#x} arilou {arilou} \
                             starbase {starbase} orbit {orbit}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
#[serial]
fn the_solar_system_owner_is_asked_only_where_prepare_summary_asks_it() {
    install(&blank_state());
    for activity in [0u16, 1, 2, 3, 4, 5, 6, 7, 8] {
        let mut game = LiveGame::bare();
        game.current_activity = activity | START_ENCOUNTER;
        let solar = Solar::new();
        let summary = capture_summary(&sis(), &game, &solar, b"").unwrap();
        match activity {
            4 => {
                assert_eq!(solar.name_asks.get(), 1);
                assert_eq!(solar.orbit_asks.get(), 1);
                assert_eq!(&summary.sis.planet_name[..6], b"Moon-B");
                assert_eq!(summary.sis.planet_name[6], 0);
                assert_eq!(solar.last_battle_asks.get(), 0);
            }
            1 => {
                assert_eq!(solar.last_battle_asks.get(), 1);
                assert_eq!(solar.name_asks.get() + solar.orbit_asks.get(), 0);
                assert_eq!(&summary.sis.planet_name[..8], b"Sa-Matra");
            }
            _ => {
                assert_eq!(
                    solar.name_asks.get() + solar.orbit_asks.get() + solar.last_battle_asks.get(),
                    0
                );
                assert_eq!(&summary.sis.planet_name[..], b"Alpha Centauri\0\0");
            }
        }
    }
}

#[test]
#[serial]
fn a_starbase_visit_does_not_ask_whether_the_flagship_is_in_orbit() {
    let mut state = blank_state();
    set_key(&mut state, "GLOBAL_FLAGS_AND_DATA", 0xFF);
    install(&state);
    let mut game = LiveGame::bare();
    game.current_activity = 4;
    let solar = Solar::new();
    let summary = capture_summary(&sis(), &game, &solar, b"").unwrap();
    assert_eq!(summary.activity, IN_STARBASE);
    assert_eq!(solar.orbit_asks.get(), 0);
    assert_eq!(solar.name_asks.get(), 1);
}

// ---------------------------------------------------------------------------
// Fields read from the game-state singleton
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn credit_and_lander_flags_come_from_the_singleton_bits() {
    let mut state = blank_state();
    set_key(&mut state, "MELNORME_CREDIT0", 0xA5);
    set_key(&mut state, "MELNORME_CREDIT1", 0x5A);
    set_key(&mut state, "LANDER_SHIELDS", 0b1010);
    set_key(&mut state, "IMPROVED_LANDER_CARGO", 1);
    install(&state);
    let mut game = LiveGame::bare();
    game.current_activity = 3;
    let summary = capture_summary(&sis(), &game, &Solar::new(), b"").unwrap();
    assert_eq!(summary.mcredit_lo, 0xA5);
    assert_eq!(summary.mcredit_hi, 0x5A);
    assert_eq!(summary.flags, 0b0010_1010);

    for (speed, cargo, shot, bomb, flags) in [
        (1, 0, 0, 0, 0b0001_0000u8),
        (0, 1, 0, 0, 0b0010_0000),
        (0, 0, 1, 0, 0b0100_0000),
        (0, 0, 0, 1, 0b0000_0000),
        (0, 0, 0, 2, 0b1000_0000),
        (0, 0, 0, 3, 0b1000_0000),
        (1, 1, 1, 3, 0b1111_0000),
    ] {
        let mut state = blank_state();
        set_key(&mut state, "IMPROVED_LANDER_SPEED", speed);
        set_key(&mut state, "IMPROVED_LANDER_CARGO", cargo);
        set_key(&mut state, "IMPROVED_LANDER_SHOT", shot);
        set_key(&mut state, "CHMMR_BOMB_STATE", bomb);
        install(&state);
        let summary = capture_summary(&sis(), &game, &Solar::new(), b"").unwrap();
        assert_eq!(summary.flags, flags, "{speed} {cargo} {shot} {bomb}");
    }
}

#[test]
fn the_state_bits_used_here_are_where_globdata_h_puts_them() {
    // Bit 267 is byte 33, mask 0x08.
    assert_eq!(key("ROSY_SPHERE_ON_SHIP"), (267, 1));
    assert_eq!(key("MELNORME_CREDIT0"), (59, 8));
    assert_eq!(key("ULTRON_CONDITION"), (228, 3));
    let mut state = blank_state();
    set_key(&mut state, "ROSY_SPHERE_ON_SHIP", 1);
    assert_eq!(state[33], 0x08);
    assert_eq!(state.iter().filter(|&&byte| byte != 0).count(), 1);
}

#[test]
#[serial]
fn each_device_is_listed_by_its_id_when_only_its_condition_holds() {
    let game = {
        let mut game = LiveGame::bare();
        game.current_activity = 3;
        game
    };
    for (id, name) in DEVICE_FLAGS {
        let mut state = blank_state();
        set_key(&mut state, name, 1);
        install(&state);
        let summary = capture_summary(&sis(), &game, &Solar::new(), b"").unwrap();
        assert_eq!(summary.num_devices, 1, "device {id}");
        assert_eq!(summary.device_list[0], id);
        assert!(summary.device_list[1..].iter().all(|&slot| slot == 0));
    }

    // ULTRON_CONDITION 1..=4 are devices 7..=10; 0 and the values above 4 are none.
    for (condition, expected) in [
        (0u8, None),
        (1, Some(7)),
        (2, Some(8)),
        (3, Some(9)),
        (4, Some(10)),
        (5, None),
        (7, None),
    ] {
        let mut state = blank_state();
        set_key(&mut state, "ULTRON_CONDITION", condition);
        install(&state);
        let summary = capture_summary(&sis(), &game, &Solar::new(), b"").unwrap();
        match expected {
            Some(id) => {
                assert_eq!(summary.num_devices, 1, "condition {condition}");
                assert_eq!(summary.device_list[0], id);
            }
            None => assert_eq!(summary.num_devices, 0, "condition {condition}"),
        }
    }
}

#[test]
#[serial]
fn devices_are_listed_in_id_order_and_stop_at_sixteen() {
    let mut state = blank_state();
    for (_, name) in DEVICE_FLAGS {
        set_key(&mut state, name, 1);
    }
    set_key(&mut state, "ULTRON_CONDITION", 2);
    install(&state);
    let mut game = LiveGame::bare();
    game.current_activity = 3;
    let summary = capture_summary(&sis(), &game, &Solar::new(), b"").unwrap();
    assert_eq!(summary.num_devices, 16);
    assert_eq!(
        summary.device_list,
        [0, 1, 2, 3, 4, 5, 6, 8, 11, 12, 13, 17, 18, 19, 20, 21]
    );
}

#[test]
fn the_device_ids_are_the_enum_of_globdata_h() {
    let header = include_str!("../../../../sc2/src/uqm/globdata.h");
    let start = header
        .find("PORTAL_SPAWNER_DEVICE = 0")
        .expect("device enum");
    let names: Vec<&str> = header[start..]
        .lines()
        .map(|line| line.trim().trim_end_matches(','))
        .take_while(|line| *line != "NUM_DEVICES")
        .filter(|line| !line.is_empty())
        .map(|line| line.split(" = ").next().unwrap())
        .collect();
    assert_eq!(names.len(), NUM_DEVICES);
    let id = |name: &str| names.iter().position(|n| *n == name).unwrap() as u8;
    let expected = [
        ("PORTAL_SPAWNER_DEVICE", 0),
        ("TALKING_PET_DEVICE", 1),
        ("ROSY_SPHERE_DEVICE", 4),
        ("ULTRON_0_DEVICE", 7),
        ("MAIDENS_DEVICE", 11),
        ("DATA_PLATE_1_DEVICE", 14),
        ("TAALO_PROTECTOR_DEVICE", 17),
        ("DESTRUCT_CODE_DEVICE", 23),
        ("URQUAN_WARP_DEVICE", 24),
        ("LUNAR_BASE_DEVICE", 27),
    ];
    for (name, value) in expected {
        assert_eq!(id(name), value, "{name}");
    }
}

// ---------------------------------------------------------------------------
// Escorts, date, name
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn escorts_and_date_follow_the_live_queue_and_clock() {
    install(&blank_state());
    let mut game = LiveGame::bare();
    game.current_activity = 3;
    let before = capture_summary(&sis(), &game, &Solar::new(), b"").unwrap();
    assert_eq!(before.num_ships, 0);
    assert_eq!(before.ship_list, [0; 12]);

    for race in [2u8, 7, 7, 11] {
        game.push_fragment(false, race, 0);
    }
    // The npc queue is not part of the summary.
    game.push_fragment(true, 9, 0);
    game.game_clock.day_index = 28;
    game.game_clock.month_index = 12;
    game.game_clock.year_index = 2160;

    let after = capture_summary(&sis(), &game, &Solar::new(), b"").unwrap();
    assert_eq!(after.num_ships, 4);
    assert_eq!(after.ship_list, [2, 7, 7, 11, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(
        (after.day_index, after.month_index, after.year_index),
        (28, 12, 2160)
    );
}

#[test]
#[serial]
fn a_full_hangar_fills_the_ship_list() {
    install(&blank_state());
    let mut game = LiveGame::bare();
    game.current_activity = 3;
    for race in 1..=12u8 {
        game.push_fragment(false, race, 0);
    }
    let summary = capture_summary(&sis(), &game, &Solar::new(), b"").unwrap();
    assert_eq!(summary.num_ships, 12);
    assert_eq!(summary.ship_list, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
}

#[test]
#[serial]
fn a_corrupt_built_ship_queue_is_an_error_not_a_summary() {
    install(&blank_state());
    let mut game = LiveGame::bare();
    game.current_activity = 3;
    game.push_fragment(false, 2, 0);
    let head = game.built_ship_q.head;
    game.built_ship_q.head = (head as usize + 1) as *mut std::ffi::c_void;
    let error = capture_summary(&sis(), &game, &Solar::new(), b"").unwrap_err();
    assert!(matches!(error, SummaryError::Queue(_)), "{error:?}");
    game.built_ship_q.head = head;
}

#[test]
#[serial]
fn the_save_name_is_cut_where_strncpy_cuts_it() {
    install(&blank_state());
    let mut game = LiveGame::bare();
    game.current_activity = 3;
    let name = |bytes: &[u8]| {
        capture_summary(&sis(), &game, &Solar::new(), bytes)
            .unwrap()
            .save_name
            .as_bytes()
            .to_vec()
    };
    assert_eq!(name(b""), b"");
    assert_eq!(name(b"Game 1"), b"Game 1");
    assert_eq!(name(b"ab\0cd"), b"ab");
    assert_eq!(name(&[b'a'; 63]), vec![b'a'; 63]);
    assert_eq!(name(&[b'a'; 64]), vec![b'a'; 63]);
    assert_eq!(name(&[b'z'; 200]), vec![b'z'; 63]);
    // The bytes are not checked as UTF-8: the game cuts in the middle of a
    // character the same way.
    let mut long = vec![b'a'; 62];
    long.extend_from_slice("é".as_bytes());
    assert_eq!(name(&long), long[..63].to_vec());
}

#[test]
fn utf8_string_copy_terminates_inside_the_buffer() {
    assert_eq!(
        &utf8_string_copy(b"Sa-Matra")[..],
        b"Sa-Matra\0\0\0\0\0\0\0\0"
    );
    assert_eq!(
        &utf8_string_copy(b"abc\0def")[..],
        b"abc\0\0\0\0\0\0\0\0\0\0\0\0\0"
    );
    let long = utf8_string_copy(&[b'q'; 40]);
    assert_eq!(&long[..15], &[b'q'; 15]);
    assert_eq!(long[15], 0);
}

#[test]
#[serial]
fn a_changed_owner_changes_the_summary() {
    install(&blank_state());
    let mut game = LiveGame::bare();
    game.current_activity = 3;
    let mut flagship = sis();
    let first = capture_summary(&flagship, &game, &Solar::new(), b"a").unwrap();

    flagship.fuel_on_board += 50;
    flagship.num_landers = 2;
    game.game_clock.year_index += 1;
    let mut state = blank_state();
    set_key(&mut state, "MELNORME_CREDIT0", 9);
    install(&state);
    let second = capture_summary(&flagship, &game, &Solar::new(), b"a").unwrap();

    assert_eq!(second.sis.fuel_on_board, first.sis.fuel_on_board + 50);
    assert_eq!(second.sis.num_landers, 2);
    assert_eq!(second.year_index, first.year_index + 1);
    assert_eq!((first.mcredit_lo, second.mcredit_lo), (0, 9));
}

// ---------------------------------------------------------------------------
// Summ over the Rust solar-system reads
// ---------------------------------------------------------------------------

/// `GAME_STRING (PLANET_NUMBER_BASE + n)` is `s<375 + n>` in the test table.
const SOL_STAR: u8 = 1;
const LUNA: &str = "s384";
const EARTH: &str = "s377";
const SA_MATRA_NAME: &str = "s407";

fn interplanetary_game() -> LiveGame {
    let mut game = LiveGame::bare();
    game.current_activity = IN_INTERPLANETARY | START_ENCOUNTER;
    game
}

fn summary_over(
    system: &System,
    game: &LiveGame,
    star: Option<u8>,
) -> Result<crate::savegame::types::Summary, SummaryError> {
    capture_summary(&sis(), game, &owner(system, star, sis().planet_name), b"")
}

#[test]
#[serial]
fn the_summary_names_the_moon_the_flagship_orbits() {
    install(&blank_state());
    let game = interplanetary_game();
    let summary = summary_over(&System::orbiting_moon(2, 1), &game, Some(SOL_STAR)).unwrap();
    assert_eq!(text(summary.sis.planet_name), LUNA);
    assert_eq!(summary.activity, IN_PLANET_ORBIT);
    // Everything else of the flagship is the live `SIS_STATE`.
    assert_eq!(summary.sis.fuel_on_board, sis().fuel_on_board);
    assert_eq!(summary.sis.ship_name, sis().ship_name);
}

#[test]
#[serial]
fn a_changed_solar_system_changes_the_summary() {
    install(&blank_state());
    let game = interplanetary_game();

    let mut system = System::orbiting_planet(2);
    let at_earth = summary_over(&system, &game, Some(SOL_STAR)).unwrap();
    assert_eq!(text(at_earth.sis.planet_name), EARTH);
    assert_eq!(at_earth.activity, IN_PLANET_ORBIT);

    // Leaving orbit keeps the name of the inner system's planet but not the
    // orbit activity.
    system.InOrbit = 0;
    let flying = summary_over(&system, &game, Some(SOL_STAR)).unwrap();
    assert_eq!(text(flying.sis.planet_name), EARTH);
    assert_eq!(flying.activity, IN_INTERPLANETARY as u8);

    // Back out to the outer system: the flagship's own planet name.
    let outer = System::outer();
    let outside = summary_over(&outer, &game, Some(SOL_STAR)).unwrap();
    assert_eq!(&outside.sis.planet_name, &sis().planet_name);
    assert_eq!(outside.activity, IN_INTERPLANETARY as u8);

    // Orbiting an unnamed moon of an unnamed star.
    let moon = System::orbiting_moon(3, 2);
    let at_moon = summary_over(&moon, &game, Some(6)).unwrap();
    assert_eq!(text(at_moon.sis.planet_name), "Alpha Centauri");
    let mut short = sis();
    short.planet_name = FLAGSHIP_NAME;
    let at_moon =
        capture_summary(&short, &game, &owner(&moon, Some(6), FLAGSHIP_NAME), b"").unwrap();
    assert_eq!(text(at_moon.sis.planet_name), "Beta Hydri-C");
}

#[test]
#[serial]
fn a_starbase_visit_is_a_starbase_even_in_orbit_and_is_still_named() {
    let mut state = blank_state();
    set_key(&mut state, "GLOBAL_FLAGS_AND_DATA", 0xFF);
    install(&state);
    let game = interplanetary_game();
    let summary = summary_over(&System::orbiting_moon(2, 0), &game, Some(SOL_STAR)).unwrap();
    assert_eq!(summary.activity, IN_STARBASE);
    assert_eq!(text(summary.sis.planet_name), "s422");
}

#[test]
#[serial]
fn the_final_battle_summary_names_sa_matra_whatever_the_system_holds() {
    install(&blank_state());
    let mut game = LiveGame::bare();
    game.current_activity = IN_LAST_BATTLE | START_ENCOUNTER;
    let summary = summary_over(&System::orbiting_planet(2), &game, Some(SOL_STAR)).unwrap();
    assert_eq!(text(summary.sis.planet_name), SA_MATRA_NAME);
    assert_eq!(summary.activity, IN_LAST_BATTLE as u8);
}

#[test]
#[serial]
fn the_other_activities_do_not_read_the_solar_system() {
    install(&blank_state());
    // A state `planet_or_moon_name` would reject.
    let mut broken = System::inner(2);
    broken.pOrbitalDesc = std::ptr::null_mut();
    for activity in [0u16, 2, 3, 5, 6, 7, 8] {
        let mut game = LiveGame::bare();
        game.current_activity = activity | START_ENCOUNTER;
        let summary = summary_over(&broken, &game, Some(SOL_STAR)).unwrap();
        assert_eq!(
            &summary.sis.planet_name,
            &sis().planet_name,
            "activity {activity}"
        );
    }
}

#[test]
#[serial]
fn a_broken_solar_system_is_an_error_not_a_guessed_name() {
    install(&blank_state());
    let game = interplanetary_game();
    let mut broken = System::inner(2);
    broken.pOrbitalDesc = std::ptr::null_mut();
    assert_eq!(
        summary_over(&broken, &game, Some(SOL_STAR)).unwrap_err(),
        SummaryError::Solar(SolarError::OrbitalWorldMissing)
    );
}

#[test]
#[serial]
fn no_solar_system_summarises_as_plain_interplanetary_flight() {
    install(&blank_state());
    let game = interplanetary_game();
    let solar = SolarSystem::new(None, None, sis().planet_name, &Table);
    let summary = capture_summary(&sis(), &game, &solar, b"").unwrap();
    assert_eq!(summary.activity, IN_INTERPLANETARY as u8);
    assert_eq!(&summary.sis.planet_name, &sis().planet_name);
}

// ---------------------------------------------------------------------------
// The star catalog
// ---------------------------------------------------------------------------

/// A copy of the real `starmap_array`, so tests can alter entries.
fn catalog() -> Vec<CStarDesc> {
    // SAFETY: the table is only written by the game thread, which these tests
    // do not start.
    let table = unsafe { (&raw const crate::planets::tables::STARMAP_ARRAY).read() };
    table.to_vec()
}

fn wire(star: &CStarDesc) -> StarDesc {
    StarDesc {
        star_pt: Point {
            x: star.star_pt.x,
            y: star.star_pt.y,
        },
        star_type: star.star_type,
        index: star.index,
        prefix: star.prefix,
        postfix: star.postfix,
    }
}

const ZERO_STAR: StarDesc = StarDesc {
    star_pt: Point { x: 0, y: 0 },
    star_type: 0,
    index: 0,
    prefix: 0,
    postfix: 0,
};

#[test]
fn the_catalog_has_the_shape_find_star_expects() {
    let table = catalog();
    // Both sections are sorted by y and end in a sentinel.
    assert!(table[..NUM_SOLAR_SYSTEMS]
        .windows(2)
        .all(|pair| pair[0].star_pt.y <= pair[1].star_pt.y));
    assert_eq!(table[NUM_SOLAR_SYSTEMS].star_pt.y, 19_998);
    assert_eq!(table[CATALOG_LEN - 1].star_pt.y, 19_998);
    // Sol is where the C game's save puts the flagship's star.
    let sol = &C_WRITTEN_SAVE[C_WRITTEN_SAVE.len() - 8..];
    let (x, y) = (le16(sol) as i16, le16(&sol[2..]) as i16);
    assert!(table[..NUM_SOLAR_SYSTEMS]
        .iter()
        .any(|star| star.star_pt.x == x && star.star_pt.y == y));
}

#[test]
fn find_star_finds_the_first_entry_at_every_real_point() {
    let table = catalog();
    for index in 0..NUM_SOLAR_SYSTEMS {
        let point = Point {
            x: table[index].star_pt.x,
            y: table[index].star_pt.y,
        };
        let first = table[..NUM_SOLAR_SYSTEMS]
            .iter()
            .position(|star| star.star_pt.x == point.x && star.star_pt.y == point.y)
            .unwrap();
        assert_eq!(find_star(&table, 0, point).unwrap(), Some(first), "{index}");
        assert_eq!(find_star(&table, 1, point).unwrap(), Some(first));
    }
    // Quasispace looks only at the portals.
    for offset in 0..=NUM_HYPER_VORTICES {
        let index = NUM_SOLAR_SYSTEMS + 1 + offset;
        let point = Point {
            x: table[index].star_pt.x,
            y: table[index].star_pt.y,
        };
        assert_eq!(find_star(&table, 2, point).unwrap(), Some(index));
        assert_eq!(find_star(&table, 3, point).unwrap(), Some(index));
        assert_eq!(find_star(&table, 0, point).unwrap(), None);
    }
    let sol = table[0].star_pt;
    assert_eq!(
        find_star(&table, 2, Point { x: sol.x, y: sol.y }).unwrap(),
        None
    );
}

#[test]
fn find_star_misses_points_between_stars_and_off_the_map() {
    let table = catalog();
    let first = table[0].star_pt;
    for point in [
        Point {
            x: first.x + 1,
            y: first.y,
        },
        Point {
            x: first.x,
            y: first.y + 1,
        },
        Point { x: 0, y: 0 },
        Point { x: -1, y: -1 },
        Point { x: 9999, y: 9999 },
        Point {
            x: 19_998,
            y: 19_998,
        },
    ] {
        assert_eq!(find_star(&table, 0, point).unwrap(), None, "{point:?}");
    }
}

#[test]
fn the_star_chunk_of_the_c_save_is_what_the_catalog_entry_gives() {
    let raw = &C_WRITTEN_SAVE[C_WRITTEN_SAVE.len() - 16..];
    assert_eq!(&raw[..4], b"Star");
    assert_eq!(le32(&raw[4..]), 8);
    let saved = StarDesc {
        star_pt: Point {
            x: le16(&raw[8..]) as i16,
            y: le16(&raw[10..]) as i16,
        },
        star_type: raw[12],
        index: raw[13],
        prefix: raw[14],
        postfix: raw[15],
    };

    // The catalog entry at the saved point is the star the C game saved.
    let table = catalog();
    let row = table[..NUM_SOLAR_SYSTEMS]
        .iter()
        .position(|star| (star.star_pt.x, star.star_pt.y) == (saved.star_pt.x, saved.star_pt.y))
        .expect("the saved star is a system");
    let captured = capture_star(&table, &table[row]).unwrap();
    assert_eq!(captured, saved);

    assert_eq!(
        resolve_star(&table, 0, &saved).unwrap(),
        LoadedStar::Catalog(row)
    );
}

#[test]
fn capture_star_reads_the_entry_current_points_at() {
    let table = catalog();
    for index in [0usize, 1, 100, 501, 503, 518] {
        let captured = capture_star(&table, &table[index]).unwrap();
        assert_eq!(captured, wire(&table[index]), "{index}");
    }
    // NULL is all zeroes, as `memset (&SD, 0, sizeof (SD))`.
    assert_eq!(capture_star(&table, std::ptr::null()).unwrap(), ZERO_STAR);
}

#[test]
fn capture_star_refuses_a_pointer_the_catalog_does_not_own() {
    let table = catalog();
    let outside = table[5];
    assert_eq!(
        capture_star(&table, &outside).unwrap_err(),
        StarError::CurrentOutsideCatalog
    );
    let inside_the_table_but_between_entries =
        (table.as_ptr() as usize + 3 * 8 + 2) as *const CStarDesc;
    assert_eq!(
        capture_star(&table, inside_the_table_but_between_entries).unwrap_err(),
        StarError::CurrentOutsideCatalog
    );
    let past_the_end = (table.as_ptr() as usize + CATALOG_LEN * 8) as *const CStarDesc;
    assert_eq!(
        capture_star(&table, past_the_end).unwrap_err(),
        StarError::CurrentOutsideCatalog
    );
    assert_eq!(
        capture_star(&table[..10], &table[1]).unwrap_err(),
        StarError::WrongCatalogLength { found: 10 }
    );
}

#[test]
fn resolve_star_accepts_a_catalog_entry_and_the_null_descriptor() {
    let table = catalog();
    for index in [0usize, 250, 501, 503, 510, 518] {
        let side = if index > NUM_SOLAR_SYSTEMS { 2 } else { 0 };
        let saved = wire(&table[index]);
        let resolved = resolve_star(&table, side, &saved).unwrap();
        // Duplicated points resolve to the first of them.
        match resolved {
            LoadedStar::Catalog(found) => assert_eq!(wire(&table[found]), saved),
            LoadedStar::None => panic!("{index} resolved to NULL"),
        }
    }
    assert_eq!(
        resolve_star(&table, 0, &ZERO_STAR).unwrap(),
        LoadedStar::None
    );
    assert_eq!(
        resolve_star(&table, 2, &ZERO_STAR).unwrap(),
        LoadedStar::None
    );
}

#[test]
fn resolve_star_rejects_what_the_catalog_does_not_hold() {
    let table = catalog();
    let real = wire(&table[40]);

    let mut moved = real;
    moved.star_pt.x += 1;
    assert_eq!(
        resolve_star(&table, 0, &moved).unwrap_err(),
        StarError::NotInCatalog {
            star_pt: moved.star_pt
        }
    );

    // A zero point with other fields is not the NULL descriptor.
    let mut stray = ZERO_STAR;
    stray.postfix = 1;
    assert!(matches!(
        resolve_star(&table, 0, &stray).unwrap_err(),
        StarError::NotInCatalog { .. }
    ));

    // The point of a system, found on the wrong side of the Arilou gate.
    assert!(matches!(
        resolve_star(&table, 2, &real).unwrap_err(),
        StarError::NotInCatalog { .. }
    ));

    for change in 0..4 {
        let mut saved = real;
        match change {
            0 => saved.star_type ^= 1,
            1 => saved.index ^= 1,
            2 => saved.prefix ^= 1,
            _ => saved.postfix ^= 1,
        }
        let first = find_star(&table, 0, saved.star_pt).unwrap().unwrap();
        assert_eq!(
            resolve_star(&table, 0, &saved).unwrap_err(),
            StarError::FieldsDisagree {
                catalog_index: first
            },
            "field {change}"
        );
    }
}
