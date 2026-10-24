//! Tests for the live "Scan", "Grps" and "BtGp" capture and restore.
//!
//! Expected images are assembled here byte by byte from the layouts in
//! `grpinfo.c` (`WriteGroupHeader`, `WriteShipFragment`, `WriteIpGroup`) and
//! `load.c`, not produced by the code under test. The seed save is a file the
//! C game wrote; its chunk values below were decoded with a separate script.

use serial_test::serial;

use super::statefiles::*;
use crate::comm::locdata::CGameState;
use crate::savegame::decode::decode_save;
use crate::savegame::types::{
    BattleFleet, BattleGroup, BattleShip, GroupListEntry, IpGroup, Point, GAME_STATE_BYTES,
};
use crate::state::ffi::{
    replace_game_state, rust_init_game_state, snapshot_game_state, with_state_files,
};
use crate::state::game_state::{get_state_32_raw, set_state_32_raw};
use crate::state::state_file::{DEFGRPINFO_FILE, RANDGRPINFO_FILE, STARINFO_FILE};

const C_WRITTEN_SAVE: &[u8] = include_bytes!("../../../tests/fixtures/savegame/legacy-save.00");

/// `SHOFIXTI_GRPOFFS0` and the 32-bit stride, from `globdata.h`.
const FIRST_OFFSET_BIT: usize = 790;
/// The Ur-Quan probe is defined group 12 (slot 11), SAMATRA is group 14.
const PROBE_OFFSET_BIT: usize = 790 + 11 * 32;
const SAMATRA_OFFSET_BIT: usize = 790 + 13 * 32;

fn game() -> Box<CGameState> {
    // SAFETY: `CGameState` is integers and raw pointers; zero is valid.
    Box::new(unsafe { std::mem::zeroed() })
}

/// A fresh game state and empty group files, as `InitGlobData` leaves them.
fn fresh_game() -> Box<CGameState> {
    // SAFETY: no requirements.
    unsafe { rust_init_game_state() };
    init_group_info(true).unwrap();
    with_state_files(|files| files.replace_contents(STARINFO_FILE, &[]).unwrap()).unwrap();
    game()
}

fn file(index: usize) -> Vec<u8> {
    with_state_files(|files| files.get_file(index).unwrap().contents().to_vec()).unwrap()
}

fn header(num: u8, day: u8, month: u8, star: u16, year: u16, offsets: &[(usize, u32)]) -> Vec<u8> {
    let mut out = vec![num, day, month, 0];
    out.extend_from_slice(&star.to_le_bytes());
    out.extend_from_slice(&year.to_le_bytes());
    let mut table = [0u32; 65];
    for &(slot, offset) in offsets {
        table[slot] = offset;
    }
    for offset in table {
        out.extend_from_slice(&offset.to_le_bytes());
    }
    assert_eq!(out.len(), 268);
    out
}

/// outer race, 6 unused bytes around the name index, race, index, crew bytes,
/// energy bytes, 4 unused bytes.
fn ship(outer: u8, fields: [u8; 7]) -> Vec<u8> {
    let [captain, race, index, crew, max_crew, en, max_en] = fields;
    vec![
        outer, 0, 0, captain, 0, 0, 0, race, index, crew, max_crew, en, max_en, 0, 0, 0, 0,
    ]
}

fn typed_ship(outer: u8, fields: [u8; 7]) -> BattleShip {
    let [captain, race, index, crew, max_crew, en, max_en] = fields;
    BattleShip {
        race_outer: outer,
        captains_name_index: captain,
        race_id: race,
        index,
        crew_level: u16::from(crew),
        max_crew: u16::from(max_crew),
        energy_level: en,
        max_energy: max_en,
    }
}

fn seed() -> crate::savegame::types::SaveFile {
    decode_save(C_WRITTEN_SAVE).unwrap()
}

fn seed_sections() -> StateFileSections {
    let save = seed();
    StateFileSections {
        scan: save.scan,
        canned_nodes: save.canned_nodes,
        group_list: save.group_list,
        battle_groups: save.battle_groups,
    }
}

/// `RANDGRPINFO` of the seed: header at 0, group list at 268.
fn seed_random_image() -> Vec<u8> {
    let mut image = header(0, 24, 2, 82, 2155, &[(0, 268)]);
    image.extend_from_slice(&[0x00, 0x01]); // last encounter, count
    image.extend_from_slice(&[
        0x17, 0, 0, 0, 0, 0, 0, 0x17, 0x03, 0x01, 0, 0xd3, 0x01, 0x77, 0xf7, 0x37, 0xfc,
    ]);
    image
}

/// `DEFGRPINFO` of the seed: the reserved byte, then group 12 at offset 1.
fn seed_defined_image() -> Vec<u8> {
    let mut image = vec![0];
    image.extend_from_slice(&header(1, 24, 2, 82, 2155, &[(1, 269)]));
    image.extend_from_slice(&[0x17, 0x01]);
    image.extend_from_slice(&ship(0x17, [0, 0x17, 0, 1, 1, 0, 1]));
    image
}

#[test]
#[serial]
fn seed_restores_to_the_images_the_c_loader_builds() {
    let mut game = fresh_game();
    replace_game_state(&seed().game_state).unwrap();
    restore_state_files(&mut game, &seed_sections()).unwrap();

    assert_eq!(file(STARINFO_FILE), vec![0u8; 2008]);
    assert_eq!(file(RANDGRPINFO_FILE), seed_random_image());
    assert_eq!(file(DEFGRPINFO_FILE), seed_defined_image());
    assert_eq!(game.battle_group_ref, 1);
    let state = snapshot_game_state().unwrap();
    assert_eq!(get_state_32_raw(&state, PROBE_OFFSET_BIT), 1);
    assert_eq!(game.game_state_bytes, state);
}

#[test]
#[serial]
fn capture_after_restore_gives_back_the_seed_sections() {
    let mut game = fresh_game();
    replace_game_state(&seed().game_state).unwrap();
    restore_state_files(&mut game, &seed_sections()).unwrap();

    let captured = capture_state_files(&game).unwrap();
    assert_eq!(captured, seed_sections());

    let list = captured.group_list.as_ref().unwrap();
    assert_eq!(list.last_encounter, 0);
    assert_eq!(
        list.entries,
        vec![GroupListEntry {
            race_outer: 0x17,
            group: IpGroup {
                group_counter: 0,
                race_id: 0x17,
                sys_loc: 3,
                task: 0,
                in_system: 1,
                dest_loc: 3,
                orbit_pos: 13,
                group_id: 1,
                loc: Point { x: -2185, y: -969 },
            },
        }]
    );
    let random = &captured.battle_groups[0];
    assert_eq!((random.encounter_id, random.current), (0, false));
    assert_eq!(
        (
            random.star_index,
            random.day_index,
            random.month_index,
            random.year_index
        ),
        (82, 24, 2, 2155)
    );
    assert!(random.fleets.is_empty());
    let probe = &captured.battle_groups[1];
    assert_eq!((probe.encounter_id, probe.current), (12, true));
    assert_eq!(
        probe.fleets,
        vec![BattleFleet {
            icon: 0x17,
            ships: vec![typed_ship(0x17, [0, 0x17, 0, 1, 1, 0, 1])],
        }]
    );
}

#[test]
fn capture_reads_independently_assembled_images() {
    // Random file: two group slots, the second wiped (offset 0), no list.
    let mut random = header(2, 3, 4, 9, 2160, &[(1, 268)]);
    random.extend_from_slice(&[5, 2]);
    random.extend_from_slice(&ship(7, [1, 7, 2, 200, 255, 3, 4]));
    random.extend_from_slice(&ship(8, [9, 8, 10, 0, 12, 13, 14]));

    // Defined file: group 1 (Shofixti) at 1, group 14 (Samatra) after it.
    let mut defined = vec![0];
    defined.extend_from_slice(&header(1, 1, 2, 3, 2150, &[(1, 269)]));
    defined.extend_from_slice(&[0x20, 1]);
    defined.extend_from_slice(&ship(0x20, [0, 0x20, 1, 5, 6, 7, 8]));
    let second = defined.len() as u32;
    defined.extend_from_slice(&header(
        2,
        5,
        6,
        7,
        2151,
        &[(1, second + 268), (2, second + 268 + 2)],
    ));
    defined.extend_from_slice(&[0x30, 0, 0x31, 0]);

    let mut state = [0u8; GAME_STATE_BYTES];
    set_state_32_raw(&mut state, FIRST_OFFSET_BIT, 1);
    set_state_32_raw(&mut state, SAMATRA_OFFSET_BIT, second);

    let star = [1u8, 0, 0, 0, 0xff, 0xff, 0xff, 0xff];
    let sections = capture_from_images(&star, &random, &defined, &state, second).unwrap();

    assert_eq!(sections.scan, Some(vec![1, u32::MAX]));
    assert!(sections.group_list.is_none());
    assert_eq!(sections.battle_groups.len(), 3);
    let r = &sections.battle_groups[0];
    assert_eq!(r.fleets.len(), 2);
    assert_eq!(r.fleets[0].icon, 5);
    assert_eq!(
        r.fleets[0].ships,
        vec![
            typed_ship(7, [1, 7, 2, 200, 255, 3, 4]),
            typed_ship(8, [9, 8, 10, 0, 12, 13, 14])
        ]
    );
    assert!(r.fleets[1].ships.is_empty());
    let (a, b) = (&sections.battle_groups[1], &sections.battle_groups[2]);
    assert_eq!((a.encounter_id, a.current), (1, false));
    assert_eq!((b.encounter_id, b.current), (14, true));
    assert_eq!(
        (b.star_index, b.day_index, b.month_index, b.year_index),
        (7, 5, 6, 2151)
    );
    assert_eq!(
        b.fleets
            .iter()
            .map(|f| (f.icon, f.ships.len()))
            .collect::<Vec<_>>(),
        vec![(0x30, 0), (0x31, 0)]
    );
}

#[test]
fn empty_storage_captures_an_empty_scan_and_no_groups() {
    let sections = capture_from_images(&[], &[], &[], &[0; GAME_STATE_BYTES], 0).unwrap();
    assert_eq!(sections.scan, Some(Vec::new()));
    assert!(sections.group_list.is_none());
    assert!(sections.battle_groups.is_empty());
}

#[test]
fn malformed_images_are_typed_errors() {
    let zero = [0u8; GAME_STATE_BYTES];
    assert_eq!(
        capture_from_images(&[1, 2, 3], &[], &[], &zero, 0),
        Err(StateSectionError::StarInfoNotWordAligned { len: 3 })
    );
    // A header cut short.
    assert!(matches!(
        capture_from_images(&[], &[0; 100], &[], &zero, 0),
        Err(StateSectionError::OutOfBounds {
            file: RANDGRPINFO_FILE,
            len: 100,
            ..
        })
    ));
    // 65 groups: one more than the offset table holds.
    assert_eq!(
        capture_from_images(&[], &header(65, 0, 0, 0, 0, &[]), &[], &zero, 0),
        Err(StateSectionError::TooManyGroups { count: 65 })
    );
    // A group whose offset lies outside the file.
    assert!(matches!(
        capture_from_images(&[], &header(1, 0, 0, 0, 0, &[(1, 5000)]), &[], &zero, 0),
        Err(StateSectionError::OutOfBounds {
            file: RANDGRPINFO_FILE,
            offset: 5000,
            ..
        })
    ));
    // A ship count that runs off the end.
    let mut random = header(1, 0, 0, 0, 0, &[(1, 268)]);
    random.extend_from_slice(&[1, 2]);
    random.extend_from_slice(&ship(1, [1, 1, 1, 1, 1, 1, 1]));
    assert!(matches!(
        capture_from_images(&[], &random, &[], &zero, 0),
        Err(StateSectionError::OutOfBounds {
            file: RANDGRPINFO_FILE,
            ..
        })
    ));
    // A defined group offset in the game state beyond the defined file.
    let mut state = zero;
    set_state_32_raw(&mut state, FIRST_OFFSET_BIT, 900);
    assert!(matches!(
        capture_from_images(&[], &[], &[0], &state, 0),
        Err(StateSectionError::OutOfBounds {
            file: DEFGRPINFO_FILE,
            offset: 900,
            ..
        })
    ));
}

fn probe_group(current: bool) -> BattleGroup {
    BattleGroup {
        encounter_id: 12,
        current,
        star_index: 1,
        day_index: 2,
        month_index: 3,
        year_index: 2155,
        fleets: vec![BattleFleet {
            icon: 4,
            ships: vec![typed_ship(4, [0, 4, 0, 10, 20, 30, 40])],
        }],
    }
}

fn random_group(fleets: usize) -> BattleGroup {
    BattleGroup {
        encounter_id: 0,
        current: false,
        star_index: 33,
        day_index: 7,
        month_index: 8,
        year_index: 2160,
        fleets: (0..fleets)
            .map(|n| BattleFleet {
                icon: n as u8 + 1,
                ships: vec![typed_ship(1, [0, n as u8, 0, 1, 2, 3, 4]); n + 1],
            })
            .collect(),
    }
}

#[test]
#[serial]
fn rejected_restore_leaves_files_and_state_alone() {
    let mut game = fresh_game();
    replace_game_state(&seed().game_state).unwrap();
    restore_state_files(&mut game, &seed_sections()).unwrap();
    let before = (
        file(STARINFO_FILE),
        file(RANDGRPINFO_FILE),
        file(DEFGRPINFO_FILE),
        snapshot_game_state().unwrap(),
        game.game_state_bytes,
        game.battle_group_ref,
    );

    let mut too_much_crew = probe_group(false);
    too_much_crew.fleets[0].ships[0].crew_level = 256;
    let mut duplicate = vec![probe_group(false), probe_group(false)];
    duplicate.insert(0, random_group(2));
    let mut two_current = vec![probe_group(true)];
    let mut other = probe_group(true);
    other.encounter_id = 3;
    two_current.push(other);
    let mut bad_nibble_list = seed_sections().group_list.unwrap();
    bad_nibble_list.entries[0].group.task = 16;

    let bad: Vec<(StateFileSections, StateSectionError)> = vec![
        (
            StateFileSections {
                scan: Some(vec![9; 4]),
                battle_groups: vec![too_much_crew],
                ..Default::default()
            },
            StateSectionError::ValueTooWide {
                field: "crew_level",
                value: 256,
            },
        ),
        (
            StateFileSections {
                scan: Some(vec![9; 4]),
                battle_groups: duplicate,
                ..Default::default()
            },
            StateSectionError::DuplicateEncounter { encounter_id: 12 },
        ),
        (
            StateFileSections {
                scan: Some(vec![9; 4]),
                battle_groups: two_current,
                ..Default::default()
            },
            StateSectionError::MultipleCurrentGroups,
        ),
        (
            StateFileSections {
                scan: Some(vec![9; 4]),
                battle_groups: vec![BattleGroup {
                    encounter_id: 15,
                    ..probe_group(false)
                }],
                ..Default::default()
            },
            StateSectionError::EncounterOutOfRange {
                encounter_id: 15,
                slots: 14,
            },
        ),
        (
            StateFileSections {
                scan: Some(vec![9; 4]),
                group_list: Some(bad_nibble_list),
                ..Default::default()
            },
            StateSectionError::ValueTooWide {
                field: "task",
                value: 16,
            },
        ),
        (
            StateFileSections {
                scan: Some(vec![9; 4]),
                battle_groups: vec![random_group(65)],
                ..Default::default()
            },
            StateSectionError::TooManyGroups { count: 65 },
        ),
        // The game state still names group 12, but the save holds none: the
        // offset would point at nothing in the new defined file.
        (
            StateFileSections {
                scan: Some(vec![9; 4]),
                battle_groups: vec![random_group(1)],
                ..Default::default()
            },
            StateSectionError::StaleDefinedGroupOffset {
                encounter_id: 12,
                offset: 1,
            },
        ),
    ];
    for (sections, expected) in bad {
        assert_eq!(restore_state_files(&mut game, &sections), Err(expected));
        let after = (
            file(STARINFO_FILE),
            file(RANDGRPINFO_FILE),
            file(DEFGRPINFO_FILE),
            snapshot_game_state().unwrap(),
            game.game_state_bytes,
            game.battle_group_ref,
        );
        assert_eq!(after, before, "state changed after {expected:?}");
    }
}

#[test]
#[serial]
fn repeated_loads_start_from_the_initial_group_files() {
    let mut game = fresh_game();
    replace_game_state(&seed().game_state).unwrap();
    restore_state_files(&mut game, &seed_sections()).unwrap();

    // A second save: three random fleets, no defined groups, game state clean.
    replace_game_state(&[0; GAME_STATE_BYTES]).unwrap();
    let mut random = random_group(3);
    random.current = true; // LoadBattleGroup forces this off for the random group
    let second = StateFileSections {
        canned_nodes: Vec::new(),
        scan: Some(vec![0xdead_beef]),
        group_list: None,
        battle_groups: vec![random],
    };
    restore_state_files(&mut game, &second).unwrap();

    assert_eq!(file(STARINFO_FILE), vec![0xef, 0xbe, 0xad, 0xde]);
    assert_eq!(
        file(DEFGRPINFO_FILE),
        vec![0],
        "defined groups restart from InitGroupInfo(TRUE)"
    );
    assert_eq!(game.battle_group_ref, 0);
    let random_file = file(RANDGRPINFO_FILE);
    // Fleet n holds n+1 ships of 17 bytes after a 2 byte header: 268, then
    // 2+17, 2+34, 2+51 further on.
    let expected_offsets = [(1, 268u32), (2, 287), (3, 323)];
    let mut expected = header(3, 7, 8, 33, 2160, &expected_offsets);
    for n in 0..3u8 {
        expected.extend_from_slice(&[n + 1, n + 1]);
        for _ in 0..=n {
            expected.extend_from_slice(&ship(1, [0, n, 0, 1, 2, 3, 4]));
        }
    }
    assert_eq!(random_file, expected);
    assert_eq!(
        capture_state_files(&game).unwrap().battle_groups,
        vec![{
            let mut g = random_group(3);
            g.current = false;
            g
        }]
    );

    // A save with only a Scan chunk touches nothing else.
    restore_state_files(
        &mut game,
        &StateFileSections {
            scan: Some(vec![]),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(file(STARINFO_FILE).is_empty());
    assert_eq!(file(RANDGRPINFO_FILE), expected);
}

#[test]
#[serial]
fn several_defined_groups_get_their_own_offsets() {
    let mut game = fresh_game();
    replace_game_state(&[0; GAME_STATE_BYTES]).unwrap();
    let mut first = probe_group(false);
    first.encounter_id = 1;
    let mut last = probe_group(true);
    last.encounter_id = 14;
    let sections = StateFileSections {
        scan: None,
        canned_nodes: Vec::new(),
        group_list: None,
        battle_groups: vec![random_group(0), last.clone(), first.clone()],
    };
    restore_state_files(&mut game, &sections).unwrap();

    // Group 14 came first in the save, so it sits at 1; group 1 follows the
    // 268 byte header and the 2 + 17 byte fleet.
    let second_offset = 1 + 268 + 19;
    let state = snapshot_game_state().unwrap();
    assert_eq!(get_state_32_raw(&state, SAMATRA_OFFSET_BIT), 1);
    assert_eq!(get_state_32_raw(&state, FIRST_OFFSET_BIT), second_offset);
    assert_eq!(game.battle_group_ref, 1);

    let captured = capture_state_files(&game).unwrap();
    assert_eq!(captured.scan, Some(Vec::new()));
    assert_eq!(captured.battle_groups[1], first);
    assert_eq!(captured.battle_groups[2], last);
}

#[test]
#[serial]
fn init_group_info_matches_the_c_initialiser() {
    let mut game = fresh_game();
    replace_game_state(&seed().game_state).unwrap();
    restore_state_files(&mut game, &seed_sections()).unwrap();

    init_group_info(false).unwrap();
    let mut empty_random = vec![0, 0, 0, 0, 0xff, 0xff, 0, 0];
    empty_random.extend_from_slice(&[0; 260]);
    assert_eq!(file(RANDGRPINFO_FILE), empty_random);
    assert_eq!(
        file(DEFGRPINFO_FILE),
        seed_defined_image(),
        "FirstTime FALSE keeps defined groups"
    );

    init_group_info(true).unwrap();
    assert_eq!(file(DEFGRPINFO_FILE), vec![0]);

    // An initialised but empty random file still saves its header as a group.
    replace_game_state(&[0; GAME_STATE_BYTES]).unwrap();
    let captured = capture_state_files(&game).unwrap();
    assert!(captured.group_list.is_none());
    assert_eq!(captured.battle_groups.len(), 1);
    assert_eq!(captured.battle_groups[0].star_index, u16::MAX);
}

#[test]
#[serial]
fn deleted_files_capture_as_empty_storage() {
    let game = fresh_game();
    with_state_files(|files| {
        for index in [STARINFO_FILE, RANDGRPINFO_FILE, DEFGRPINFO_FILE] {
            files.delete(index).unwrap();
        }
    })
    .unwrap();
    let captured = capture_state_files(&game).unwrap();
    assert_eq!(captured.scan, Some(Vec::new()));
    assert!(captured.group_list.is_none() && captured.battle_groups.is_empty());
}

#[test]
#[serial]
fn canned_nodes_capture_restore_and_old_save_clear_are_atomic() {
    let mut game = fresh_game();
    let node = crate::state::planet_info::CannedNode {
        star: 1,
        planet: 2,
        moon: 1,
        node: 31,
        value: 15,
        x: 967,
        y: 299,
    };
    with_state_files(|files| files.can_node(node)).unwrap();
    let captured = capture_state_files(&game).unwrap();
    assert_eq!(captured.canned_nodes, [node]);
    with_state_files(|files| files.remove_canned_node(node.key())).unwrap();
    restore_state_files(&mut game, &captured).unwrap();
    assert_eq!(
        with_state_files(|files| files.canned_node(node.key())).unwrap(),
        Some(node)
    );
    let mut invalid_group = probe_group(false);
    invalid_group.fleets[0].ships[0].crew_level = 256;
    assert!(restore_state_files(
        &mut game,
        &StateFileSections {
            battle_groups: vec![invalid_group],
            ..Default::default()
        }
    )
    .is_err());
    assert_eq!(
        with_state_files(|files| files.canned_node(node.key())).unwrap(),
        Some(node)
    );
    restore_state_files(&mut game, &StateFileSections::default()).unwrap();
    assert!(with_state_files(|files| files.canned_node(node.key()))
        .unwrap()
        .is_none());
    with_state_files(|files| files.can_node(node)).unwrap();
    unsafe { rust_init_game_state() };
    assert!(with_state_files(|files| files.canned_node(node.key()))
        .unwrap()
        .is_none());
}
