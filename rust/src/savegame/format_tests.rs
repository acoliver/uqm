//! Codec tests against byte vectors written out by hand from the field order
//! in `sc2/src/uqm/save.c` (and read back by `load.c`), plus one save file
//! that the C game itself wrote. None of the expected bytes come from this
//! crate's encoder.

use std::io::{self, Read};

use super::format::*;
use super::*;

fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

fn chunk(tag: &[u8; 4], size: [u8; 4], body: &[u8]) -> Vec<u8> {
    cat(&[tag, &size, body])
}

// ---------------------------------------------------------------------------
// Tags and sizes
// ---------------------------------------------------------------------------

#[test]
fn tag_values_are_the_ascii_names_little_endian() {
    assert_eq!(SAVEFILE_TAG, u32::from_le_bytes(*b"UMS\x01"));
    assert_eq!(SUMMARY_TAG, u32::from_le_bytes(*b"Summ"));
    assert_eq!(GLOBAL_STATE_TAG, u32::from_le_bytes(*b"GlSt"));
    assert_eq!(GAME_STATE_TAG, u32::from_le_bytes(*b"GmSt"));
    assert_eq!(EVENTS_TAG, u32::from_le_bytes(*b"Evts"));
    assert_eq!(ENCOUNTERS_TAG, u32::from_le_bytes(*b"Enct"));
    assert_eq!(RACE_Q_TAG, u32::from_le_bytes(*b"RacQ"));
    assert_eq!(IP_GRP_Q_TAG, u32::from_le_bytes(*b"IGpQ"));
    assert_eq!(NPC_SHIP_Q_TAG, u32::from_le_bytes(*b"NpcQ"));
    assert_eq!(SHIP_Q_TAG, u32::from_le_bytes(*b"ShpQ"));
    assert_eq!(STAR_TAG, u32::from_le_bytes(*b"Star"));
    assert_eq!(SCAN_TAG, u32::from_le_bytes(*b"Scan"));
    assert_eq!(BATTLE_GROUP_TAG, u32::from_le_bytes(*b"BtGp"));
    assert_eq!(GROUP_LIST_TAG, u32::from_le_bytes(*b"Grps"));
    assert_eq!(tag_text(RACE_Q_TAG), "RacQ");
}

#[test]
fn chunk_sizes_are_the_sums_of_the_written_fields() {
    // save.c SaveSisState: 4 words + 3 counts + slots + lander + elements +
    // three names.
    let sis = 4 * 4 + 3 * 2 + 16 + 11 + 8 + 1 + 8 * 2 + 3 * 16;
    // SaveSummary: eight single bytes and a year word, then the two lists.
    assert_eq!(SUMMARY_FIXED_BYTES, sis + 6 + 2 + 2 + 12 + 16);
    // SaveGameState GlSt: three bytes, costs, worth, activity, clock, three
    // points, facing, two bytes, velocity.
    assert_eq!(
        GLOBAL_STATE_BYTES,
        3 + 20 + 8 + 2 + (1 + 1 + 2 + 2 + 2) + 3 * 4 + 2 + 1 + 1 + (2 + 4 * 4)
    );
    assert_eq!(
        RACE_ENTRY_BYTES,
        2 + 2 + 1 + 1 + 2 + 2 + 1 + 1 + 4 + 2 + 2 + 4 + 1 + 1 + 4
    );
    assert_eq!(IP_GROUP_ENTRY_BYTES, 2 + 7 + 4);
    assert_eq!(SHIP_FRAGMENT_ENTRY_BYTES, 2 + 1 + 1 + 1 + 2 + 2 + 1 + 1);
    assert_eq!(EVENT_ENTRY_BYTES, 1 + 1 + 2 + 1);
    assert_eq!(
        ENCOUNTER_ENTRY_BYTES,
        2 + 4 + 2 + 4 + 3 + 7 * (1 + 2 + 2 + 1) + 8
    );
    assert_eq!(STAR_BYTES, 4 + 4);
    assert_eq!(GROUP_LIST_ENTRY_BYTES, 1 + 2 + 7 + 4);
    assert_eq!(BATTLE_GROUP_HEADER_BYTES, 4 + 1 + 2 + 1 + 1 + 2 + 1);
    assert_eq!(BATTLE_SHIP_BYTES, 1 + 1 + 1 + 1 + 2 + 2 + 1 + 1);
    assert_eq!(GAME_STATE_BYTES, 155);
}

// ---------------------------------------------------------------------------
// Hand-written chunk bodies
// ---------------------------------------------------------------------------

const SUMM_FIXED: [u8; 160] = {
    let mut b = [0u8; 160];
    // log_x 0x01020304, log_y -2
    let head: [u8; 22] = [
        0x04, 0x03, 0x02, 0x01, 0xFE, 0xFF, 0xFF, 0xFF, // log_x, log_y
        0x44, 0x33, 0x22, 0x11, // ResUnits 0x11223344
        0x88, 0x77, 0x66, 0x55, // FuelOnBoard 0x55667788
        0x02, 0x01, // CrewEnlisted 0x0102
        0x04, 0x03, // TotalElementMass 0x0304
        0x06, 0x05, // TotalBioMass 0x0506
    ];
    let mut i = 0;
    while i < 22 {
        b[i] = head[i];
        i += 1;
    }
    // ModuleSlots 0x10..=0x1F
    let mut i = 0;
    while i < 16 {
        b[22 + i] = 0x10 + i as u8;
        i += 1;
    }
    // DriveSlots 0x20..=0x2A
    let mut i = 0;
    while i < 11 {
        b[38 + i] = 0x20 + i as u8;
        i += 1;
    }
    // JetSlots 0x30..=0x37
    let mut i = 0;
    while i < 8 {
        b[49 + i] = 0x30 + i as u8;
        i += 1;
    }
    b[57] = 0x04; // NumLanders
                  // ElementAmounts 0x1001..=0x1008, little-endian
    let mut i = 0;
    while i < 8 {
        b[58 + 2 * i] = 1 + i as u8;
        b[59 + 2 * i] = 0x10;
        i += 1;
    }
    // ShipName "Flagship", CommanderName "Zelnick", PlanetName "Sol"
    let ship = *b"Flagship";
    let mut i = 0;
    while i < 8 {
        b[74 + i] = ship[i];
        i += 1;
    }
    let cmd = *b"Zelnick";
    let mut i = 0;
    while i < 7 {
        b[90 + i] = cmd[i];
        i += 1;
    }
    let planet = *b"Sol";
    let mut i = 0;
    while i < 3 {
        b[106 + i] = planet[i];
        i += 1;
    }
    b[122] = 0x07; // Activity
    b[123] = 0x5A; // Flags
    b[124] = 0x0F; // day
    b[125] = 0x0C; // month
    b[126] = 0xE5; // year 0x07E5
    b[127] = 0x07;
    b[128] = 0x34; // MCreditLo
    b[129] = 0x12; // MCreditHi
    b[130] = 0x03; // NumShips
    b[131] = 0x02; // NumDevices
    let mut i = 0;
    while i < 12 {
        b[132 + i] = 0xA0 + i as u8; // ShipList
        i += 1;
    }
    let mut i = 0;
    while i < 16 {
        b[144 + i] = 0xB0 + i as u8; // DeviceList
        i += 1;
    }
    b
};

fn summary_chunk(name: &[u8]) -> Vec<u8> {
    let size = (160 + name.len() as u32).to_le_bytes();
    chunk(b"Summ", size, &cat(&[&SUMM_FIXED, name]))
}

fn padded<const N: usize>(text: &[u8]) -> [u8; N] {
    let mut a = [0u8; N];
    a[..text.len()].copy_from_slice(text);
    a
}

fn expected_summary(name: &[u8]) -> Summary {
    Summary {
        sis: SisState {
            log_x: 0x0102_0304,
            log_y: -2,
            res_units: 0x1122_3344,
            fuel_on_board: 0x5566_7788,
            crew_enlisted: 0x0102,
            total_element_mass: 0x0304,
            total_bio_mass: 0x0506,
            module_slots: std::array::from_fn(|i| 0x10 + i as u8),
            drive_slots: std::array::from_fn(|i| 0x20 + i as u8),
            jet_slots: std::array::from_fn(|i| 0x30 + i as u8),
            num_landers: 4,
            element_amounts: std::array::from_fn(|i| 0x1001 + i as u16),
            ship_name: padded(b"Flagship"),
            commander_name: padded(b"Zelnick"),
            planet_name: padded(b"Sol"),
        },
        activity: 7,
        flags: 0x5A,
        day_index: 15,
        month_index: 12,
        year_index: 0x07E5,
        mcredit_lo: 0x34,
        mcredit_hi: 0x12,
        num_ships: 3,
        num_devices: 2,
        ship_list: std::array::from_fn(|i| 0xA0 + i as u8),
        device_list: std::array::from_fn(|i| 0xB0 + i as u8),
        save_name: SaveName::new(name).unwrap(),
    }
}

const GLST_BODY: [u8; 75] = {
    let mut b = [0u8; 75];
    b[0] = 0x01; // glob_flags
    b[1] = 0x02; // CrewCost
    b[2] = 0x03; // FuelCost
    let mut i = 0;
    while i < 20 {
        b[3 + i] = 0x40 + i as u8; // ModuleCost
        i += 1;
    }
    let mut i = 0;
    while i < 8 {
        b[23 + i] = 0x60 + i as u8; // ElementWorth
        i += 1;
    }
    let rest: [u8; 44] = [
        0x34, 0x12, // CurrentActivity 0x1234
        0x05, 0x06, 0xE6, 0x07, // clock day 5, month 6, year 0x07E6
        0xFF, 0xFF, // tick_count -1
        0x2C, 0x01, // day_in_ticks 300
        0x10, 0x00, 0xF0, 0xFF, // autopilot (16, -16)
        0xE8, 0x03, 0x18, 0xFC, // ip_location (1000, -1000)
        0x0B, 0x0A, 0x0D, 0x0C, // ShipStamp origin (0x0A0B, 0x0C0D)
        0x0F, 0x00, // ShipFacing 15
        0x09, 0x08, // ip_planet 9, in_orbit 8
        0x21, 0x43, // TravelAngle 0x4321
        0x01, 0x00, 0xFF, 0xFF, // vector (1, -1)
        0x02, 0x00, 0xFE, 0xFF, // fract (2, -2)
        0x03, 0x00, 0xFD, 0xFF, // error (3, -3)
        0x04, 0x00, 0xFC, 0xFF, // incr (4, -4)
    ];
    let mut i = 0;
    while i < 44 {
        b[31 + i] = rest[i];
        i += 1;
    }
    b
};

fn expected_global() -> GlobalState {
    GlobalState {
        glob_flags: 1,
        crew_cost: 2,
        fuel_cost: 3,
        module_cost: std::array::from_fn(|i| 0x40 + i as u8),
        element_worth: std::array::from_fn(|i| 0x60 + i as u8),
        current_activity: 0x1234,
        clock: ClockState {
            day_index: 5,
            month_index: 6,
            year_index: 0x07E6,
            tick_count: -1,
            day_in_ticks: 300,
        },
        autopilot: Point { x: 16, y: -16 },
        ip_location: Point { x: 1000, y: -1000 },
        ship_stamp_origin: Point {
            x: 0x0A0B,
            y: 0x0C0D,
        },
        ship_facing: 15,
        ip_planet: 9,
        in_orbit: 8,
        velocity: Velocity {
            travel_angle: 0x4321,
            vector: Extent {
                width: 1,
                height: -1,
            },
            fract: Extent {
                width: 2,
                height: -2,
            },
            error: Extent {
                width: 3,
                height: -3,
            },
            incr: Extent {
                width: 4,
                height: -4,
            },
        },
    }
}

fn game_state_pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i * 7 + 3) as u8).collect()
}

fn expected_game_state() -> [u8; GAME_STATE_BYTES] {
    std::array::from_fn(|i| (i * 7 + 3) as u8)
}

fn gmst_chunk(len: usize) -> Vec<u8> {
    chunk(
        b"GmSt",
        (len as u32).to_le_bytes(),
        &game_state_pattern(len),
    )
}

fn glst_chunk() -> Vec<u8> {
    chunk(b"GlSt", [75, 0, 0, 0], &GLST_BODY)
}

const RACE_ENTRY_1: [u8; 30] = [
    0x03, 0x00, // queue index 3
    0x02, 0x01, // allied_state 0x0102
    0x05, 0x06, // days_left, growth_fract
    0x07, 0x08, // crew_level 0x0807
    0x09, 0x0A, // max_crew 0x0A09
    0x0B, 0x0C, // growth, max_energy
    0x0D, 0x0E, 0xFF, 0xFF, // loc (0x0E0D, -1)
    0x10, 0x11, // actual_strength 0x1110
    0x12, 0x13, // known_strength 0x1312
    0x14, 0x15, 0x00, 0x80, // known_loc (0x1514, -32768)
    0x16, 0x17, // growth_err_term, func_index
    0xFE, 0xFF, 0x18, 0x19, // dest_loc (-2, 0x1918)
];

fn race_1() -> RaceEntry {
    RaceEntry {
        index: 3,
        allied_state: 0x0102,
        days_left: 5,
        growth_fract: 6,
        crew_level: 0x0807,
        max_crew: 0x0A09,
        growth: 0x0B,
        max_energy: 0x0C,
        loc: Point { x: 0x0E0D, y: -1 },
        actual_strength: 0x1110,
        known_strength: 0x1312,
        known_loc: Point {
            x: 0x1514,
            y: -32768,
        },
        growth_err_term: 0x16,
        func_index: 0x17,
        dest_loc: Point { x: -2, y: 0x1918 },
    }
}

fn race_2_bytes() -> [u8; 30] {
    let mut b = RACE_ENTRY_1;
    b[0] = 0x04;
    b
}

fn race_2() -> RaceEntry {
    RaceEntry {
        index: 4,
        ..race_1()
    }
}

const IP_GROUP_BYTES: [u8; 13] = [
    0x34, 0x12, // group_counter 0x1234
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
    0x07, // race, sys_loc, task, in_system, dest, orbit, id
    0x08, 0x09, 0xFF, 0x7F, // loc (0x0908, 32767)
];

fn ip_group_1() -> IpGroup {
    IpGroup {
        group_counter: 0x1234,
        race_id: 1,
        sys_loc: 2,
        task: 3,
        in_system: 4,
        dest_loc: 5,
        orbit_pos: 6,
        group_id: 7,
        loc: Point {
            x: 0x0908,
            y: 32767,
        },
    }
}

const SHIP_BYTES: [u8; 11] = [
    0x0A, 0x00, // race index 10
    0x01, 0x02, 0x03, // captain, race_id, index
    0x04, 0x05, // crew_level 0x0504
    0x06, 0x07, // max_crew 0x0706
    0x08, 0x09, // energy, max_energy
];

fn ship_1(race_index: u16) -> ShipFragmentEntry {
    ShipFragmentEntry {
        race_index,
        captains_name_index: 1,
        race_id: 2,
        index: 3,
        crew_level: 0x0504,
        max_crew: 0x0706,
        energy_level: 8,
        max_energy: 9,
    }
}

const EVENT_BYTES: [u8; 5] = [0x1F, 0x0B, 0xE7, 0x07, 0x02];

fn event_1() -> EventEntry {
    EventEntry {
        day_index: 31,
        month_index: 11,
        year_index: 0x07E7,
        func_index: 2,
    }
}

const ENCOUNTER_BYTES: [u8; 65] = {
    let mut b = [0u8; 65];
    let head: [u8; 15] = [
        0xFE, 0xFF, // transition_state -2
        0x01, 0x02, 0x03, 0x04, // origin (0x0201, 0x0403)
        0x05, 0x06, // radius 0x0605
        0x07, 0x08, 0xFF, 0xFF, // loc_pt (0x0807, -1)
        0x09, 0x03, 0x80, // race_id, num_ships, flags
    ];
    let mut i = 0;
    while i < 15 {
        b[i] = head[i];
        i += 1;
    }
    // Seven BRIEF_SHIP_INFO: race, crew word, max_crew word, max_energy.
    let mut s = 0;
    while s < 7 {
        let at = 15 + 6 * s;
        b[at] = 1 + s as u8;
        b[at + 1] = 0x10 + s as u8;
        b[at + 2] = 0x00;
        b[at + 3] = 0x20 + s as u8;
        b[at + 4] = 0x00;
        b[at + 5] = 0x30 + s as u8;
        s += 1;
    }
    // log_x 0x80010000 (negative), log_y 0x12345678
    b[57] = 0x00;
    b[58] = 0x00;
    b[59] = 0x01;
    b[60] = 0x80;
    b[61] = 0x78;
    b[62] = 0x56;
    b[63] = 0x34;
    b[64] = 0x12;
    b
};

fn encounter_1() -> EncounterEntry {
    EncounterEntry {
        transition_state: -2,
        origin: Point {
            x: 0x0201,
            y: 0x0403,
        },
        radius: 0x0605,
        loc_pt: Point { x: 0x0807, y: -1 },
        race_id: 9,
        num_ships: 3,
        flags: 0x80,
        ship_list: std::array::from_fn(|s| BriefShipInfo {
            race_id: 1 + s as u8,
            crew_level: 0x10 + s as u16,
            max_crew: 0x20 + s as u16,
            max_energy: 0x30 + s as u8,
        }),
        log_x: i32::from_le_bytes([0x00, 0x00, 0x01, 0x80]),
        log_y: 0x1234_5678,
    }
}

const SCAN_BYTES: [u8; 8] = [0x01, 0x00, 0x00, 0x00, 0x78, 0x56, 0x34, 0x12];

const GROUP_LIST_BODY: [u8; 29] = [
    0x02, // last encounter
    0x21, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x00, 0x80, 0x22, 0x11,
    0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x7F, 0xFF,
];

fn group_list() -> GroupList {
    GroupList {
        last_encounter: 2,
        entries: vec![
            GroupListEntry {
                race_outer: 0x21,
                group: IpGroup {
                    group_counter: 0x0201,
                    race_id: 3,
                    sys_loc: 4,
                    task: 5,
                    in_system: 6,
                    dest_loc: 7,
                    orbit_pos: 8,
                    group_id: 9,
                    loc: Point {
                        x: 0x0B0A,
                        y: -32768,
                    },
                },
            },
            GroupListEntry {
                race_outer: 0x22,
                group: IpGroup {
                    group_counter: 0x1211,
                    race_id: 0x13,
                    sys_loc: 0x14,
                    task: 0x15,
                    in_system: 0x16,
                    dest_loc: 0x17,
                    orbit_pos: 0x18,
                    group_id: 0x19,
                    loc: Point { x: 0x1B1A, y: -129 },
                },
            },
        ],
    }
}

/// "BtGp" body for encounter id `id` (little-endian bytes) and `current`.
fn battle_body(id: [u8; 4], current: u8) -> Vec<u8> {
    cat(&[
        &id,
        &[current],
        &[0x34, 0x12], // star_index 0x1234
        &[3, 4],       // day, month
        &[0xE8, 0x07], // year 0x07E8
        &[2],          // two fleets
        &[0x0A, 2],    // fleet icon 0x0A, two ships
        &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A],
        &[0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A],
        &[0x0B, 0], // fleet icon 0x0B, no ships
    ])
}

fn battle(encounter_id: u32, current: bool) -> BattleGroup {
    BattleGroup {
        encounter_id,
        current,
        star_index: 0x1234,
        day_index: 3,
        month_index: 4,
        year_index: 0x07E8,
        fleets: vec![
            BattleFleet {
                icon: 0x0A,
                ships: vec![
                    BattleShip {
                        race_outer: 1,
                        captains_name_index: 2,
                        race_id: 3,
                        index: 4,
                        crew_level: 0x0605,
                        max_crew: 0x0807,
                        energy_level: 9,
                        max_energy: 10,
                    },
                    BattleShip {
                        race_outer: 0x11,
                        captains_name_index: 0x12,
                        race_id: 0x13,
                        index: 0x14,
                        crew_level: 0x1615,
                        max_crew: 0x1817,
                        energy_level: 0x19,
                        max_energy: 0x1A,
                    },
                ],
            },
            BattleFleet {
                icon: 0x0B,
                ships: vec![],
            },
        ],
    }
}

const STAR_BYTES_FIXTURE: [u8; 8] = [0x88, 0x13, 0xA0, 0x0F, 0x11, 0x22, 0x33, 0x44];

fn star() -> StarDesc {
    StarDesc {
        star_pt: Point { x: 5000, y: 4000 },
        star_type: 0x11,
        index: 0x22,
        prefix: 0x33,
        postfix: 0x44,
    }
}

fn star_chunk() -> Vec<u8> {
    chunk(b"Star", [8, 0, 0, 0], &STAR_BYTES_FIXTURE)
}

/// Magic, Summ, GlSt, GmSt with the canonical 155 bytes.
fn prefix() -> Vec<u8> {
    cat(&[
        b"UMS\x01",
        &summary_chunk(b"Slot name"),
        &glst_chunk(),
        &gmst_chunk(155),
    ])
}

fn minimal_file(middle: &[u8]) -> Vec<u8> {
    cat(&[&prefix(), middle, &star_chunk()])
}

fn minimal_expected() -> SaveFile {
    SaveFile {
        summary: expected_summary(b"Slot name"),
        global: expected_global(),
        game_state: expected_game_state(),
        race_queue: vec![],
        ip_group_queue: vec![],
        npc_ship_queue: vec![],
        ship_queue: vec![],
        events: vec![],
        encounters: vec![],
        scan: None,
        canned_nodes: Vec::new(),
        group_list: None,
        battle_groups: vec![],
        star: star(),
    }
}

// ---------------------------------------------------------------------------
// One chunk at a time: decode matches the hand-written fields, and the
// encoder reproduces the hand-written bytes.
// ---------------------------------------------------------------------------

fn assert_chunk(middle: Vec<u8>, expected: SaveFile) {
    let bytes = minimal_file(&middle);
    assert_eq!(decode_save(&bytes).unwrap(), expected);
    assert_eq!(encode_save(&expected).unwrap(), bytes);
}

#[test]
fn minimal_file_has_no_optional_chunks() {
    assert_chunk(vec![], minimal_expected());
}

#[test]
fn race_queue_chunk() {
    let mut e = minimal_expected();
    e.race_queue = vec![race_1(), race_2()];
    // 2 entries * 30 bytes = 60 = 0x3C
    let body = cat(&[&RACE_ENTRY_1, &race_2_bytes()]);
    assert_chunk(chunk(b"RacQ", [0x3C, 0, 0, 0], &body), e);
}

#[test]
fn ip_group_queue_chunk() {
    let mut e = minimal_expected();
    e.ip_group_queue = vec![ip_group_1()];
    assert_chunk(chunk(b"IGpQ", [13, 0, 0, 0], &IP_GROUP_BYTES), e);
}

#[test]
fn npc_ship_queue_chunk() {
    let mut e = minimal_expected();
    e.npc_ship_queue = vec![ship_1(10)];
    assert_chunk(chunk(b"NpcQ", [11, 0, 0, 0], &SHIP_BYTES), e);
}

#[test]
fn ship_queue_chunk() {
    let mut e = minimal_expected();
    e.ship_queue = vec![ship_1(10)];
    assert_chunk(chunk(b"ShpQ", [11, 0, 0, 0], &SHIP_BYTES), e);
}

#[test]
fn events_chunk() {
    let mut e = minimal_expected();
    e.events = vec![event_1()];
    assert_chunk(chunk(b"Evts", [5, 0, 0, 0], &EVENT_BYTES), e);
}

#[test]
fn encounters_chunk() {
    let mut e = minimal_expected();
    e.encounters = vec![encounter_1()];
    assert_chunk(chunk(b"Enct", [65, 0, 0, 0], &ENCOUNTER_BYTES), e);
}

#[test]
fn scan_chunk() {
    let mut e = minimal_expected();
    e.scan = Some(vec![1, 0x1234_5678]);
    assert_chunk(chunk(b"Scan", [8, 0, 0, 0], &SCAN_BYTES), e);
}

#[test]
fn empty_scan_chunk_is_present_but_empty() {
    let mut e = minimal_expected();
    e.scan = Some(vec![]);
    assert_chunk(chunk(b"Scan", [0, 0, 0, 0], &[]), e);
}

#[test]
fn group_list_chunk() {
    let mut e = minimal_expected();
    e.group_list = Some(group_list());
    // 1 + 2 * 14 = 29 = 0x1D
    assert_chunk(chunk(b"Grps", [0x1D, 0, 0, 0], &GROUP_LIST_BODY), e);
}

#[test]
fn group_list_with_no_entries_is_one_byte() {
    let mut e = minimal_expected();
    e.group_list = Some(GroupList {
        last_encounter: 7,
        entries: vec![],
    });
    assert_chunk(chunk(b"Grps", [1, 0, 0, 0], &[7]), e);
}

#[test]
fn battle_group_chunks() {
    let mut e = minimal_expected();
    e.battle_groups = vec![battle(0, false), battle(2, true)];
    // 12 + (2 + 2*10) + 2 = 36 = 0x24
    let middle = cat(&[
        &chunk(b"BtGp", [0x24, 0, 0, 0], &battle_body([0, 0, 0, 0], 0)),
        &chunk(b"BtGp", [0x24, 0, 0, 0], &battle_body([2, 0, 0, 0], 1)),
    ]);
    assert_chunk(middle, e);
}

#[test]
fn battle_group_encounter_id_is_four_little_endian_bytes() {
    let mut e = minimal_expected();
    e.battle_groups = vec![battle(0x0403_0201, false)];
    let middle = chunk(
        b"BtGp",
        [0x24, 0, 0, 0],
        &battle_body([0x01, 0x02, 0x03, 0x04], 0),
    );
    assert_chunk(middle, e);
}

#[test]
fn star_chunk_is_required_and_last_one_wins() {
    let mut other = STAR_BYTES_FIXTURE;
    other[4] = 0x99;
    let bytes = cat(&[
        &prefix(),
        &star_chunk(),
        &chunk(b"Star", [8, 0, 0, 0], &other),
    ]);
    assert_eq!(decode_save(&bytes).unwrap().star.star_type, 0x99);

    assert_eq!(
        decode_save(&prefix()),
        Err(SaveFormatError::MissingChunk { tag: STAR_TAG })
    );
}

#[test]
fn every_chunk_together_in_writer_order() {
    let middle = cat(&[
        &chunk(
            b"RacQ",
            [0x3C, 0, 0, 0],
            &cat(&[&RACE_ENTRY_1, &race_2_bytes()]),
        ),
        &chunk(b"NpcQ", [11, 0, 0, 0], &SHIP_BYTES),
        &chunk(b"IGpQ", [13, 0, 0, 0], &IP_GROUP_BYTES),
        &chunk(b"ShpQ", [11, 0, 0, 0], &SHIP_BYTES),
        &chunk(b"Evts", [5, 0, 0, 0], &EVENT_BYTES),
        &chunk(b"Enct", [65, 0, 0, 0], &ENCOUNTER_BYTES),
        &chunk(b"Scan", [8, 0, 0, 0], &SCAN_BYTES),
        &chunk(b"Grps", [0x1D, 0, 0, 0], &GROUP_LIST_BODY),
        &chunk(b"BtGp", [0x24, 0, 0, 0], &battle_body([0, 0, 0, 0], 0)),
        &chunk(b"BtGp", [0x24, 0, 0, 0], &battle_body([2, 0, 0, 0], 1)),
    ]);
    let expected = SaveFile {
        race_queue: vec![race_1(), race_2()],
        ip_group_queue: vec![ip_group_1()],
        npc_ship_queue: vec![ship_1(10)],
        ship_queue: vec![ship_1(10)],
        events: vec![event_1()],
        encounters: vec![encounter_1()],
        scan: Some(vec![1, 0x1234_5678]),
        group_list: Some(group_list()),
        battle_groups: vec![battle(0, false), battle(2, true)],
        ..minimal_expected()
    };
    assert_chunk(middle, expected);
}

#[test]
fn repeated_queue_chunks_append_as_the_loader_does() {
    let middle = cat(&[
        &chunk(b"Evts", [5, 0, 0, 0], &EVENT_BYTES),
        &chunk(b"Evts", [5, 0, 0, 0], &[1, 2, 0x03, 0x00, 4]),
    ]);
    let save = decode_save(&minimal_file(&middle)).unwrap();
    assert_eq!(save.events.len(), 2);
    assert_eq!(
        save.events[1],
        EventEntry {
            day_index: 1,
            month_index: 2,
            year_index: 3,
            func_index: 4
        }
    );
}

// ---------------------------------------------------------------------------
// Summary: names, summary-only reads
// ---------------------------------------------------------------------------

fn summary_of(bytes: &[u8]) -> Result<Summary, SaveFormatError> {
    decode_summary_from(bytes, bytes.len() as u64)
}

#[test]
fn summary_only_read_ignores_everything_after_summ() {
    let bytes = cat(&[b"UMS\x01", &summary_chunk(b"Slot name")]);
    assert_eq!(summary_of(&bytes).unwrap(), expected_summary(b"Slot name"));
    // Still fine when the rest of the file is garbage.
    let junk = cat(&[&bytes, &[0xDE, 0xAD]]);
    assert_eq!(summary_of(&junk).unwrap(), expected_summary(b"Slot name"));
}

#[test]
fn summary_name_lengths() {
    // Empty name: Summ is exactly 160 bytes.
    let empty = cat(&[b"UMS\x01", &summary_chunk(b"")]);
    assert_eq!(&empty[8..12], &[160, 0, 0, 0]);
    assert!(summary_of(&empty).unwrap().save_name.as_bytes().is_empty());

    // 63 bytes is the longest name SaveGame writes.
    let longest = [b'n'; 63];
    let bytes = cat(&[b"UMS\x01", &summary_chunk(&longest)]);
    assert_eq!(&bytes[8..12], &[223, 0, 0, 0]);
    assert_eq!(
        summary_of(&bytes).unwrap().save_name.as_bytes(),
        &longest[..]
    );

    // Longer names are cut to 63 bytes and the rest skipped, as load.c does.
    let mut long = vec![b'a'; 63];
    long.extend_from_slice(b"BEYOND");
    let bytes = cat(&[b"UMS\x01", &summary_chunk(&long), &glst_chunk()]);
    assert_eq!(
        summary_of(&bytes).unwrap().save_name.as_bytes(),
        &long[..63]
    );
}

#[test]
fn oversized_name_is_skipped_so_following_chunks_still_parse() {
    let name = vec![b'z'; 5000];
    let bytes = cat(&[
        b"UMS\x01",
        &summary_chunk(&name),
        &glst_chunk(),
        &gmst_chunk(155),
        &star_chunk(),
    ]);
    let save = decode_save(&bytes).unwrap();
    assert_eq!(save.summary.save_name.as_bytes(), &name[..63]);
    assert_eq!(save.star, star());
}

#[test]
fn summary_name_bytes_are_raw_not_validated_utf8() {
    let utf8 = "Caf\u{e9} \u{1F680}".as_bytes();
    let bytes = cat(&[b"UMS\x01", &summary_chunk(utf8)]);
    assert_eq!(summary_of(&bytes).unwrap().save_name.as_bytes(), utf8);

    let invalid = [0xFF, 0xFE, b'a', 0xC3];
    let bytes = cat(&[b"UMS\x01", &summary_chunk(&invalid)]);
    assert_eq!(summary_of(&bytes).unwrap().save_name.as_bytes(), &invalid);
}

#[test]
fn summary_name_ends_at_the_first_nul_like_a_c_string() {
    let bytes = cat(&[b"UMS\x01", &summary_chunk(b"ab\0cd")]);
    assert_eq!(summary_of(&bytes).unwrap().save_name.as_bytes(), b"ab");
}

#[test]
fn save_names_the_game_cannot_write_are_refused() {
    assert_eq!(
        SaveName::new(vec![b'x'; 63]).map(|n| n.as_bytes().len()),
        Ok(63)
    );
    assert_eq!(
        SaveName::new(vec![b'x'; 64]),
        Err(SaveNameError::TooLong { len: 64 })
    );
    assert_eq!(
        SaveName::new(b"a\0b".to_vec()),
        Err(SaveNameError::ContainsNul)
    );
}

#[test]
fn encoded_summary_size_tracks_the_name() {
    let mut save = minimal_expected();
    save.summary.save_name = SaveName::new(b"twelve bytes".to_vec()).unwrap();
    let bytes = encode_save(&save).unwrap();
    assert_eq!(&bytes[4..8], b"Summ");
    assert_eq!(&bytes[8..12], &[172, 0, 0, 0]);
}

// ---------------------------------------------------------------------------
// GmSt and unknown chunks
// ---------------------------------------------------------------------------

fn with_game_state(len: usize) -> Vec<u8> {
    cat(&[
        b"UMS\x01",
        &summary_chunk(b"Slot name"),
        &glst_chunk(),
        &gmst_chunk(len),
        &star_chunk(),
    ])
}

#[test]
fn shorter_game_state_is_zero_padded() {
    let save = decode_save(&with_game_state(100)).unwrap();
    let pattern = game_state_pattern(100);
    assert_eq!(&save.game_state[..100], &pattern[..]);
    assert!(save.game_state[100..].iter().all(|&b| b == 0));
}

#[test]
fn longer_game_state_is_cut_and_the_excess_skipped() {
    let save = decode_save(&with_game_state(300)).unwrap();
    assert_eq!(save.game_state, std::array::from_fn(|i| (i * 7 + 3) as u8));
    assert_eq!(save.star, star());
}

#[test]
fn empty_game_state_is_all_zero() {
    let save = decode_save(&with_game_state(0)).unwrap();
    assert_eq!(save.game_state, [0u8; GAME_STATE_BYTES]);
}

#[test]
fn unknown_tags_are_skipped_by_declared_size() {
    let middle = cat(&[
        &chunk(b"Xtra", [3, 0, 0, 0], &[1, 2, 3]),
        &chunk(b"Evts", [5, 0, 0, 0], &EVENT_BYTES),
        &chunk(b"Zero", [0, 0, 0, 0], &[]),
        // The loader's switch does not know Summ/GlSt/GmSt/UMS after their
        // place either, so repeating them is skipped, not an error.
        &summary_chunk(b"again"),
        &glst_chunk(),
        &gmst_chunk(155),
        &chunk(b"UMS\x01", [2, 0, 0, 0], &[9, 9]),
    ]);
    let save = decode_save(&minimal_file(&middle)).unwrap();
    assert_eq!(save.events, vec![event_1()]);
    assert_eq!(save.summary, expected_summary(b"Slot name"));
}

#[test]
fn large_unknown_chunk_is_skipped_without_being_held() {
    // Reader that refuses any single read larger than a skip buffer.
    struct SmallReads<'a>(&'a [u8]);
    impl Read for SmallReads<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            assert!(buf.len() <= 4096, "read of {}", buf.len());
            self.0.read(buf)
        }
    }
    let big = vec![0u8; 1 << 20];
    let middle = chunk(b"Blob", (big.len() as u32).to_le_bytes(), &big);
    let bytes = minimal_file(&middle);
    let save = decode_save_from(SmallReads(&bytes), bytes.len() as u64).unwrap();
    assert_eq!(save.star, star());
}

#[test]
fn unknown_chunk_larger_than_the_input_is_truncated_not_allocated() {
    let middle = chunk(b"Xtra", [0xFF, 0xFF, 0xFF, 0xFF], &[1, 2, 3]);
    assert_eq!(
        decode_save(&minimal_file(&middle)),
        Err(SaveFormatError::Truncated {
            at: Section::Chunk(u32::from_le_bytes(*b"Xtra"))
        })
    );
}

// ---------------------------------------------------------------------------
// Malformed input
// ---------------------------------------------------------------------------

#[test]
fn input_shorter_than_the_magic_is_truncated() {
    for len in 0..4 {
        assert_eq!(
            decode_save(&b"UMS\x01"[..len]),
            Err(SaveFormatError::Truncated {
                at: Section::FileMagic
            }),
            "len {len}"
        );
    }
}

#[test]
fn other_magic_is_refused_not_decoded_as_legacy() {
    assert_eq!(
        decode_save(&[0x01, 0x02, 0x03, 0x04, 0, 0, 0, 0]),
        Err(SaveFormatError::BadMagic { found: 0x0403_0201 })
    );
    // "UMS\x02" is a different version.
    assert_eq!(
        decode_save(b"UMS\x02Summ"),
        Err(SaveFormatError::BadMagic {
            found: u32::from_le_bytes(*b"UMS\x02")
        })
    );
}

#[test]
fn first_three_chunks_must_be_summ_glst_gmst() {
    let swapped = cat(&[
        b"UMS\x01",
        &glst_chunk(),
        &summary_chunk(b""),
        &gmst_chunk(155),
    ]);
    assert_eq!(
        decode_save(&swapped),
        Err(SaveFormatError::UnexpectedTag {
            expected: SUMMARY_TAG,
            found: GLOBAL_STATE_TAG
        })
    );
    let no_game_state = cat(&[
        b"UMS\x01",
        &summary_chunk(b""),
        &glst_chunk(),
        &star_chunk(),
    ]);
    assert_eq!(
        decode_save(&no_game_state),
        Err(SaveFormatError::UnexpectedTag {
            expected: GAME_STATE_TAG,
            found: STAR_TAG
        })
    );
    let no_glst = cat(&[b"UMS\x01", &summary_chunk(b""), &gmst_chunk(155)]);
    assert_eq!(
        decode_save(&no_glst),
        Err(SaveFormatError::UnexpectedTag {
            expected: GLOBAL_STATE_TAG,
            found: GAME_STATE_TAG
        })
    );
}

#[test]
fn summ_smaller_than_its_fixed_part_is_refused() {
    let bytes = cat(&[b"UMS\x01", b"Summ", &[159, 0, 0, 0], &SUMM_FIXED[..159]]);
    assert_eq!(
        summary_of(&bytes),
        Err(SaveFormatError::ChunkSize {
            tag: SUMMARY_TAG,
            size: 159
        })
    );
}

#[test]
fn summ_cut_short_is_truncated() {
    let full = cat(&[b"UMS\x01", &summary_chunk(b"Slot name")]);
    for len in [12, 100, 171, 175] {
        assert_eq!(
            summary_of(&full[..len]),
            Err(SaveFormatError::Truncated {
                at: Section::Chunk(SUMMARY_TAG)
            }),
            "len {len}"
        );
    }
    // Inside the tag or size word.
    for len in [4, 6, 8, 10] {
        assert_eq!(
            summary_of(&full[..len]),
            Err(SaveFormatError::Truncated {
                at: Section::ChunkHeader
            }),
            "len {len}"
        );
    }
}

#[test]
fn glst_must_be_exactly_75_bytes() {
    for size in [0u32, 74, 76, 1000] {
        let bytes = cat(&[
            b"UMS\x01",
            &summary_chunk(b""),
            b"GlSt",
            &size.to_le_bytes(),
            &GLST_BODY,
        ]);
        assert_eq!(
            decode_save(&bytes),
            Err(SaveFormatError::ChunkSize {
                tag: GLOBAL_STATE_TAG,
                size
            }),
            "size {size}"
        );
    }
}

#[test]
fn glst_and_gmst_cut_short_are_truncated() {
    let full = prefix();
    let glst_at = 4 + 8 + 160 + 9;
    let gmst_at = glst_at + 8 + 75;
    for (len, tag) in [
        (glst_at + 8 + 10, GLOBAL_STATE_TAG),
        (gmst_at + 8 + 10, GAME_STATE_TAG),
    ] {
        assert_eq!(
            decode_save(&full[..len]),
            Err(SaveFormatError::Truncated {
                at: Section::Chunk(tag)
            }),
            "len {len}"
        );
    }
}

#[test]
fn a_dangling_chunk_header_is_truncated() {
    // load.c stops quietly on 1-3 stray bytes; a writer never produces them,
    // so the decoder reports them.
    for extra in 1..8 {
        let mut bytes = minimal_file(&[]);
        bytes.extend_from_slice(&b"Evts\x05\0\0\0"[..extra]);
        assert_eq!(
            decode_save(&bytes),
            Err(SaveFormatError::Truncated {
                at: Section::ChunkHeader
            }),
            "extra {extra}"
        );
    }
}

#[test]
fn chunk_body_shorter_than_declared_is_truncated() {
    let bytes = cat(&[&prefix(), b"Evts", &[10, 0, 0, 0], &EVENT_BYTES]);
    assert_eq!(
        decode_save(&bytes),
        Err(SaveFormatError::Truncated {
            at: Section::Chunk(EVENTS_TAG)
        })
    );
}

#[test]
fn queue_chunks_must_hold_whole_entries() {
    for (tag, size) in [
        (b"RacQ", 31u32),
        (b"IGpQ", 14),
        (b"NpcQ", 12),
        (b"ShpQ", 10),
        (b"Evts", 6),
        (b"Enct", 66),
    ] {
        let body = vec![0u8; size as usize];
        assert_eq!(
            decode_save(&minimal_file(&chunk(tag, size.to_le_bytes(), &body))),
            Err(SaveFormatError::ChunkSize {
                tag: u32::from_le_bytes(*tag),
                size
            }),
            "{}",
            tag_text(u32::from_le_bytes(*tag))
        );
    }
}

#[test]
fn queue_counts_beyond_a_16_bit_count_are_refused_before_any_allocation() {
    // 65536 events declared; no body supplied at all.
    let size = 65_536u32 * 5;
    let middle = cat(&[b"Evts", &size.to_le_bytes()]);
    assert_eq!(
        decode_save(&minimal_file(&middle)),
        Err(SaveFormatError::CountOverflow {
            tag: EVENTS_TAG,
            count: 65_536
        })
    );
}

#[test]
fn queue_with_the_largest_count_round_trips() {
    let mut events = Vec::new();
    for i in 0..65_535u32 {
        events.push([(i & 0xFF) as u8, ((i >> 8) & 0xFF) as u8, 0x01, 0x00, 0x02]);
    }
    let body: Vec<u8> = events.concat();
    let size = (body.len() as u32).to_le_bytes();
    let bytes = minimal_file(&chunk(b"Evts", size, &body));
    let save = decode_save(&bytes).unwrap();
    assert_eq!(save.events.len(), 65_535);
    assert_eq!(save.events[300].day_index, 44);
    assert_eq!(save.events[300].month_index, 1);
    assert_eq!(encode_save(&save).unwrap(), bytes);

    let mut too_many = save;
    too_many.events.push(event_1());
    assert_eq!(
        encode_save(&too_many),
        Err(EncodeError::CountOverflow {
            tag: EVENTS_TAG,
            count: 65_536
        })
    );
}

#[test]
fn group_list_sizes_and_counts() {
    for size in [0u32, 2, 15 + 1] {
        let body = vec![0u8; size as usize];
        assert_eq!(
            decode_save(&minimal_file(&chunk(b"Grps", size.to_le_bytes(), &body))),
            Err(SaveFormatError::ChunkSize {
                tag: GROUP_LIST_TAG,
                size
            }),
            "size {size}"
        );
    }
    // 256 entries cannot be counted in the byte the format uses.
    let size = 1 + 14 * 256u32;
    assert_eq!(
        decode_save(&minimal_file(&cat(&[b"Grps", &size.to_le_bytes()]))),
        Err(SaveFormatError::CountOverflow {
            tag: GROUP_LIST_TAG,
            count: 256
        })
    );
    // 255 entries is the most there can be.
    let size = 1 + 14 * 255u32;
    let save = decode_save(&minimal_file(&chunk(
        b"Grps",
        size.to_le_bytes(),
        &vec![0u8; size as usize],
    )))
    .unwrap();
    assert_eq!(save.group_list.unwrap().entries.len(), 255);
}

#[test]
fn battle_group_sizes_must_match_their_contents() {
    // Smaller than the fixed header.
    assert_eq!(
        decode_save(&minimal_file(&chunk(b"BtGp", [11, 0, 0, 0], &[0u8; 11]))),
        Err(SaveFormatError::ChunkSize {
            tag: BATTLE_GROUP_TAG,
            size: 11
        })
    );
    // Declares two fleets but the chunk ends after the header.
    let mut header_only = battle_body([0; 4], 0);
    header_only.truncate(12);
    assert_eq!(
        decode_save(&minimal_file(&chunk(b"BtGp", [12, 0, 0, 0], &header_only))),
        Err(SaveFormatError::ChunkSize {
            tag: BATTLE_GROUP_TAG,
            size: 12
        })
    );
    // A fleet claims more ships than the chunk holds.
    let mut short = battle_body([0; 4], 0);
    short.truncate(12 + 2 + 10);
    assert_eq!(
        decode_save(&minimal_file(&chunk(b"BtGp", [24, 0, 0, 0], &short))),
        Err(SaveFormatError::ChunkSize {
            tag: BATTLE_GROUP_TAG,
            size: 24
        })
    );
    // Bytes left over after the last fleet.
    let mut long = battle_body([0; 4], 0);
    long.push(0);
    assert_eq!(
        decode_save(&minimal_file(&chunk(b"BtGp", [37, 0, 0, 0], &long))),
        Err(SaveFormatError::ChunkSize {
            tag: BATTLE_GROUP_TAG,
            size: 37
        })
    );
    // Larger than 255 fleets of 255 ships could be, refused before reading.
    let size = 12 + 255 * (2 + 255 * 10) + 1u32;
    assert_eq!(
        decode_save(&minimal_file(&cat(&[b"BtGp", &size.to_le_bytes()]))),
        Err(SaveFormatError::ChunkSize {
            tag: BATTLE_GROUP_TAG,
            size
        })
    );
}

#[test]
fn battle_group_at_the_format_maximum_round_trips() {
    let ship = BattleShip {
        race_outer: 1,
        captains_name_index: 2,
        race_id: 3,
        index: 4,
        crew_level: 5,
        max_crew: 6,
        energy_level: 7,
        max_energy: 8,
    };
    let mut save = minimal_expected();
    save.battle_groups = vec![BattleGroup {
        encounter_id: 1,
        current: false,
        star_index: 0,
        day_index: 0,
        month_index: 0,
        year_index: 0,
        fleets: (0..255)
            .map(|icon| BattleFleet {
                icon,
                ships: vec![ship.clone(); 255],
            })
            .collect(),
    }];
    let bytes = encode_save(&save).unwrap();
    assert_eq!(decode_save(&bytes).unwrap(), save);

    save.battle_groups[0].fleets.push(BattleFleet {
        icon: 0,
        ships: vec![],
    });
    assert_eq!(
        encode_save(&save),
        Err(EncodeError::CountOverflow {
            tag: BATTLE_GROUP_TAG,
            count: 256
        })
    );
    save.battle_groups[0].fleets.pop();
    save.battle_groups[0].fleets[0].ships.push(ship);
    assert_eq!(
        encode_save(&save),
        Err(EncodeError::CountOverflow {
            tag: BATTLE_GROUP_TAG,
            count: 256
        })
    );
}

#[test]
fn group_list_over_255_entries_is_not_encodable() {
    let mut save = minimal_expected();
    let entry = group_list().entries[0].clone();
    save.group_list = Some(GroupList {
        last_encounter: 0,
        entries: vec![entry; 256],
    });
    assert_eq!(
        encode_save(&save),
        Err(EncodeError::CountOverflow {
            tag: GROUP_LIST_TAG,
            count: 256
        })
    );
}

#[test]
fn scan_and_star_sizes_are_checked() {
    for size in [1u32, 3, 6] {
        let body = vec![0u8; size as usize];
        assert_eq!(
            decode_save(&minimal_file(&chunk(b"Scan", size.to_le_bytes(), &body))),
            Err(SaveFormatError::ChunkSize {
                tag: SCAN_TAG,
                size
            })
        );
    }
    for size in [0u32, 7, 9] {
        let body = vec![0u8; size as usize];
        assert_eq!(
            decode_save(&cat(&[
                &prefix(),
                &chunk(b"Star", size.to_le_bytes(), &body)
            ])),
            Err(SaveFormatError::ChunkSize {
                tag: STAR_TAG,
                size
            })
        );
    }
    // A scan chunk that claims more than the input has.
    let middle = cat(&[b"Scan", &0x4000_0000u32.to_le_bytes(), &[0u8; 8]]);
    assert_eq!(
        decode_save(&minimal_file(&middle)),
        Err(SaveFormatError::Truncated {
            at: Section::Chunk(SCAN_TAG)
        })
    );
}

#[test]
fn reader_failures_other_than_end_of_input_are_reported() {
    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "no"))
        }
    }
    assert_eq!(
        decode_save_from(Broken, 100),
        Err(SaveFormatError::Io {
            kind: io::ErrorKind::PermissionDenied
        })
    );
}

#[test]
fn input_shorter_than_its_declared_length_is_truncated() {
    let bytes = minimal_file(&[]);
    assert_eq!(
        decode_save_from(&bytes[..50], bytes.len() as u64),
        Err(SaveFormatError::Truncated {
            at: Section::Chunk(SUMMARY_TAG)
        })
    );
}

// ---------------------------------------------------------------------------
// A save the C game wrote
// ---------------------------------------------------------------------------

/// Tracked evidence from a real run of the C game: epic 20 load/save proof.
const C_WRITTEN_SAVE: &[u8] = include_bytes!("../../tests/fixtures/savegame/legacy-save.00");

#[test]
fn c_written_save_decodes_to_the_values_in_its_bytes() {
    // Expected values were read from the file with a separate script.
    let save = decode_save(C_WRITTEN_SAVE).unwrap();
    let s = &save.summary;
    assert_eq!((s.sis.log_x, s.sis.log_y), (27987, 164141));
    assert_eq!(
        (s.sis.res_units, s.sis.fuel_on_board, s.sis.crew_enlisted),
        (0, 1000, 50)
    );
    assert_eq!(
        s.sis.module_slots,
        [
            0x16, 0x05, 0x03, 0x16, 0x16, 0x16, 0x16, 0x16, 0x04, 0x16, 0x16, 0x16, 0x16, 0x16,
            0x16, 0x09
        ]
    );
    assert_eq!(
        s.sis.drive_slots,
        [0x14, 0x14, 0x14, 0x14, 0x14, 0x01, 0x01, 0x14, 0x14, 0x14, 0x14]
    );
    assert_eq!(
        s.sis.jet_slots,
        [0x02, 0x15, 0x15, 0x15, 0x15, 0x15, 0x02, 0x15]
    );
    assert_eq!(s.sis.num_landers, 1);
    assert_eq!(s.sis.ship_name, padded::<16>(b"Vindicator"));
    assert_eq!(s.sis.commander_name, padded::<16>(b"Zelnick"));
    assert_eq!(
        (
            s.activity,
            s.flags,
            s.day_index,
            s.month_index,
            s.year_index
        ),
        (4, 0, 17, 2, 2155)
    );
    assert_eq!((s.num_ships, s.num_devices), (1, 0));
    assert_eq!(s.ship_list[0], 2);
    assert!(s.save_name.as_bytes().is_empty());

    let g = &save.global;
    assert_eq!((g.glob_flags, g.crew_cost, g.fuel_cost), (0, 3, 20));
    assert_eq!(g.module_cost[..10], [10, 10, 10, 40, 15, 10, 0, 40, 0, 40]);
    assert_eq!(g.element_worth, [1, 2, 3, 4, 5, 6, 8, 25]);
    assert_eq!(g.current_activity, 4);
    assert_eq!(g.clock.day_index, 17);
    assert_eq!(g.clock.month_index, 2);
    assert_eq!(g.clock.year_index, 2155);
    assert_eq!((g.clock.tick_count, g.clock.day_in_ticks), (674, 720));
    assert_eq!(g.autopilot, Point { x: -1, y: -1 });
    assert_eq!(g.ip_location, Point { x: 0, y: 16384 });
    assert_eq!(g.ship_stamp_origin, Point { x: 121, y: 225 });
    assert_eq!((g.ship_facing, g.ip_planet, g.in_orbit), (1, 0, 0));
    assert_eq!(save.game_state[93], 4);
    assert_eq!(save.game_state[142], 64);

    assert_eq!(save.race_queue.len(), 24);
    assert_eq!(save.race_queue[0].index, 0);
    assert_eq!(save.race_queue[0].allied_state, 2);
    assert_eq!(save.race_queue[0].loc, Point { x: 438, y: 6372 });
    assert_eq!(save.ip_group_queue.len(), 1);
    assert_eq!(save.ip_group_queue[0].loc, Point { x: -2185, y: -969 });
    assert_eq!(save.ship_queue.len(), 1);
    assert_eq!(save.ship_queue[0].race_index, 2);
    assert_eq!(save.events.len(), 4);
    assert_eq!(
        save.events[0],
        EventEntry {
            day_index: 18,
            month_index: 2,
            year_index: 2155,
            func_index: 2
        }
    );
    assert_eq!(save.scan.as_ref().map(Vec::len), Some(502));
    let list = save.group_list.as_ref().unwrap();
    assert_eq!(list.last_encounter, 0);
    assert_eq!(list.entries.len(), 1);
    assert_eq!(list.entries[0].race_outer, 0x17);
    assert_eq!(save.battle_groups.len(), 2);
    assert_eq!(save.battle_groups[0].encounter_id, 0);
    assert_eq!(save.battle_groups[1].encounter_id, 12);
    assert_eq!(save.star.star_pt, Point { x: 1752, y: 1450 });
    assert_eq!(
        (
            save.star.star_type,
            save.star.index,
            save.star.prefix,
            save.star.postfix
        ),
        (167, 1, 0, 129)
    );
    assert!(save.npc_ship_queue.is_empty());
    assert!(save.encounters.is_empty());
}

#[test]
fn c_written_save_re_encodes_byte_for_byte() {
    let save = decode_save(C_WRITTEN_SAVE).unwrap();
    assert_eq!(encode_save(&save).unwrap(), C_WRITTEN_SAVE);
}

#[test]
fn c_written_save_summary_alone_matches_the_full_decode() {
    let full = decode_save(C_WRITTEN_SAVE).unwrap();
    let only = decode_summary_from(&C_WRITTEN_SAVE[..172], 172).unwrap();
    assert_eq!(only, full.summary);
}

#[test]
fn uncollected_canisters_round_trip_in_optional_save_chunk() {
    let mut bytes = minimal_file(&[]);
    let body = [1, 0, 2, 1, 31, 3, 0xc7, 3, 0x2b, 1];
    bytes.extend(chunk(b"Cans", [10, 0, 0, 0], &body));
    let decoded = decode_save(&bytes).unwrap();
    let encoded = encode_save(&decoded).unwrap();
    assert!(encoded.windows(4).any(|tag| tag == b"Cans"));
    assert_eq!(decode_save(&encoded).unwrap(), decoded);
}

#[test]
fn malformed_canister_records_are_rejected_and_old_saves_remain_unchanged() {
    let old = minimal_file(&[]);
    let decoded = decode_save(&old).unwrap();
    assert!(decoded.canned_nodes.is_empty());
    assert_eq!(encode_save(&decoded).unwrap(), old);
    for body in [vec![0; 9], vec![0; 10], vec![1, 0, 2, 1, 32, 3, 0, 0, 0, 0]] {
        let mut bytes = old.clone();
        bytes.extend(chunk(b"Cans", (body.len() as u32).to_le_bytes(), &body));
        assert!(decode_save(&bytes).is_err());
    }
    let body = [1, 0, 2, 1, 31, 3, 0, 0, 0, 0];
    let mut bytes = old;
    bytes.extend(chunk(b"Cans", [20, 0, 0, 0], &[body, body].concat()));
    assert!(decode_save(&bytes).is_err());
}
