//! Tests for the live "IGpQ" and "Enct" capture and restore.
//!
//! World owns the same fixed capacities as `InitGameStructures` (32 groups
//! and 16 encounters), next to a full fleet pool whose resource handles are never
//! dereferenced. The expected values are written out here from the bytes of a
//! save the C game wrote (`C_WRITTEN_SAVE`, which holds an "IGpQ" chunk and no
//! "Enct" chunk, so the encounter bytes are assembled by hand from
//! `SaveEncounters`) and from `grpinfo.h` and `encount.h`, not from the codec.

use super::groups::*;
use super::queue_tests::{
    free_slots, handle, records, FixturePool, LiveGame, C_WRITTEN_SAVE, RACES,
};
use super::queues::QueueKind;

use crate::comm::dispatch::{
    CPoint, IN_ENCOUNTER, IN_HYPERSPACE, IN_INTERPLANETARY, START_ENCOUNTER, START_INTERPLANETARY,
};
use crate::engine::battle::link_queue::LinkQueue;
use crate::engine::world::queues::{
    Encounter, EncounterRecord as CEncounter, Group, IpGroup as CIpGroup,
};
use crate::savegame::decode::decode_save;
use crate::savegame::encode::encode_save;
use crate::savegame::types::{BriefShipInfo, EncounterEntry, IpGroup, Point};

/// The 13 bytes of the "IGpQ" chunk body of the seed save, read with a
/// separate script: `group_counter` 0, race 23, `sys_loc` 3, `task` 0,
/// `in_system` 1, `dest_loc` 3, `orbit_pos` 13, `group_id` 1, `loc` (-2185, -969).
const SEED_GROUP_WIRE: [u8; 13] = [0, 0, 23, 3, 0, 1, 3, 13, 1, 119, 247, 55, 252];

/// One "Enct" body as `SaveEncounters` writes it, field by field:
/// `transition_state` -2, `origin` (1234, -5), `radius` 1600,
/// `loc_pt` (5288, 4892), race 18, 2 ships, flags `ONE_SHOT_ENCOUNTER`, seven
/// `BRIEF_SHIP_INFO` of six bytes (two used), `log_x` 74565, `log_y` -70000.
#[rustfmt::skip]
const ENCOUNTER_WIRE: [u8; 65] = [
    0xfe, 0xff,
    0xd2, 0x04, 0xfb, 0xff,
    0x40, 0x06,
    0xa8, 0x14, 0x1c, 0x13,
    18, 2, 0x80,
    18, 30, 0, 42, 0, 40,
    18, 20, 0, 42, 0, 40,
    0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0,
    0x45, 0x23, 0x01, 0x00,
    0x90, 0xee, 0xfe, 0xff,
];

const MAX_GROUPS: usize = 32;
const MAX_ENCOUNTER_SLOTS: usize = 16;

fn p(x: i16, y: i16) -> Point {
    Point { x, y }
}

fn cp(x: i16, y: i16) -> CPoint {
    CPoint { x, y }
}

fn group(race: u8, tag: u8) -> IpGroup {
    IpGroup {
        group_counter: 0x100 + u16::from(tag),
        race_id: race,
        sys_loc: tag.wrapping_add(1),
        task: tag.wrapping_add(2) & 0x0f,
        in_system: 1,
        dest_loc: tag.wrapping_add(3),
        orbit_pos: tag.wrapping_add(4),
        group_id: tag.wrapping_add(5),
        loc: p(-100 - i16::from(tag), 200 + i16::from(tag)),
    }
}

fn encounter(race: u8, ships: u8, tag: u8) -> EncounterEntry {
    let mut ship_list = [BriefShipInfo::default(); 7];
    for (i, ship) in ship_list.iter_mut().enumerate().take(usize::from(ships)) {
        *ship = BriefShipInfo {
            race_id: race,
            crew_level: 10 + u16::from(tag) + i as u16,
            max_crew: 40 + i as u16,
            max_energy: 30 + i as u8,
        };
    }
    EncounterEntry {
        transition_state: -(i16::from(tag)) - 1,
        origin: p(1000 + i16::from(tag), -2000),
        radius: 0x0500 + u16::from(tag),
        loc_pt: p(5000 + i16::from(tag), 4000),
        race_id: race,
        num_ships: ships,
        flags: 0x40 | tag,
        ship_list,
        log_x: 70_000 + i32::from(tag),
        log_y: -80_000 - i32::from(tag),
    }
}

fn sections(groups: Vec<IpGroup>, encounters: Vec<EncounterEntry>) -> GroupSections {
    GroupSections {
        ip_group_queue: groups,
        encounters,
    }
}

fn push_group(game: &mut LiveGame, entry: &IpGroup, icon: usize) {
    let mut queue = game.ip_groups();
    let id = queue.alloc().unwrap();
    *queue.get_mut(id) = crate::engine::world::queues::IpGroup {
        group_counter: entry.group_counter,
        race_id: entry.race_id,
        sys_loc: entry.sys_loc,
        task: entry.task,
        in_system: entry.in_system,
        dest_loc: entry.dest_loc,
        orbit_pos: entry.orbit_pos,
        group_id: entry.group_id,
        loc: cp(entry.loc.x, entry.loc.y),
        melee_icon: icon as _,
        ..Default::default()
    };
    queue.put(id);
}
fn push_encounter(game: &mut LiveGame, entry: &EncounterEntry, element: usize) {
    let mut queue = game.encounters();
    let id = queue.alloc().unwrap();
    *queue.get_mut(id) = crate::engine::world::queues::EncounterRecord {
        h_element: element as _,
        transition_state: entry.transition_state,
        origin: cp(entry.origin.x, entry.origin.y),
        radius: entry.radius,
        race_id: entry.race_id,
        num_ships: entry.num_ships,
        flags: entry.flags,
        loc_pt: cp(entry.loc_pt.x, entry.loc_pt.y),
        ship_list: std::array::from_fn(|i| crate::engine::world::queues::BriefShipInfo {
            race_id: entry.ship_list[i].race_id,
            crew_level: entry.ship_list[i].crew_level,
            max_crew: entry.ship_list[i].max_crew,
            max_energy: entry.ship_list[i].max_energy,
        }),
        log_x: entry.log_x,
        log_y: entry.log_y,
        ..Default::default()
    };
    queue.put(id);
}
fn raw_groups(queue: &LinkQueue<'_, Group, 32>) -> Vec<CIpGroup> {
    records(queue)
}
fn raw_encounters(queue: &LinkQueue<'_, Encounter, 16>) -> Vec<CEncounter> {
    records(queue)
}

/// The 13 bytes `SaveGroupQueue` writes for a live group, by hand.
fn group_wire(g: &CIpGroup) -> Vec<u8> {
    let mut out = g.group_counter.to_le_bytes().to_vec();
    out.extend([
        g.race_id,
        g.sys_loc,
        g.task,
        g.in_system,
        g.dest_loc,
        g.orbit_pos,
        g.group_id,
    ]);
    out.extend(g.loc.x.to_le_bytes());
    out.extend(g.loc.y.to_le_bytes());
    out
}

/// Allocation identities and remaining capacity expose resets and replacements.
fn queue_words(queue: &impl FixturePool) -> (String, usize) {
    (queue.identity(), queue.remaining())
}
type Fingerprint = (GroupSections, (String, usize), (String, usize));

/// Everything an observer can read of the two queues.
fn fingerprint(game: &mut LiveGame) -> Fingerprint {
    let was = game.data.game.current_activity;
    game.data.game.current_activity = IN_INTERPLANETARY;
    let captured = capture_groups(game).unwrap();
    game.data.game.current_activity = was;
    (
        captured,
        queue_words(&game.ip_groups_ref()),
        queue_words(&game.encounters_ref()),
    )
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

#[test]
fn group_pool_capacities_match_the_save_limits() {
    let game = LiveGame::bare();
    assert_eq!(MAX_BATTLE_GROUPS as usize, MAX_GROUPS);
    assert_eq!(MAX_ENCOUNTERS as usize, MAX_ENCOUNTER_SLOTS);
    assert_eq!(game.ip_groups_ref().count(), 0);
    assert_eq!(game.encounters_ref().count(), 0);
}

// ---------------------------------------------------------------------------
// The C-written seed save
// ---------------------------------------------------------------------------

#[test]
fn seed_group_restores_into_the_live_queue_and_captures_back() {
    let seed = decode_save(C_WRITTEN_SAVE).unwrap();
    assert_eq!(seed.ip_group_queue.len(), 1);
    assert!(seed.encounters.is_empty(), "the seed has no Enct chunk");

    let mut game = LiveGame::new();
    game.data.game.current_activity = IN_INTERPLANETARY;
    restore_groups(
        &mut game,
        &sections(seed.ip_group_queue.clone(), Vec::new()),
    )
    .unwrap();

    let live = raw_groups(&game.ip_groups_ref());
    assert_eq!(live.len(), 1);
    let g = live[0];
    assert_eq!(
        (g.group_counter, g.race_id, g.sys_loc, g.task, g.in_system),
        (0, 23, 3, 0, 1)
    );
    assert_eq!((g.dest_loc, g.orbit_pos, g.group_id), (3, 13, 1));
    assert_eq!((g.loc.x, g.loc.y), (-2185, -969));
    // The icon is the template of the group's own race.
    assert_eq!(g.melee_icon, handle(23, 3).cast());
    assert_eq!(group_wire(&g), SEED_GROUP_WIRE);

    let captured = capture_groups(&game).unwrap();
    assert_eq!(captured.ip_group_queue, seed.ip_group_queue);
    assert!(captured.encounters.is_empty());
}

#[test]
fn hand_assembled_encounter_bytes_restore_and_encode_back_unchanged() {
    let mut wire = C_WRITTEN_SAVE.to_vec();
    wire.extend(b"Enct");
    wire.extend(65u32.to_le_bytes());
    wire.extend(ENCOUNTER_WIRE);
    let save = decode_save(&wire).unwrap();
    assert_eq!(save.encounters.len(), 1);

    let mut game = LiveGame::new();
    game.data.game.current_activity = IN_INTERPLANETARY;
    restore_groups(
        &mut game,
        &sections(save.ip_group_queue.clone(), save.encounters.clone()),
    )
    .unwrap();

    let live = raw_encounters(&game.encounters_ref());
    assert_eq!(live.len(), 1);
    let e = live[0];
    assert_eq!(e.transition_state, -2);
    assert_eq!((e.origin.x, e.origin.y), (1234, -5));
    assert_eq!(e.radius, 1600);
    assert_eq!((e.loc_pt.x, e.loc_pt.y), (5288, 4892));
    assert_eq!((e.race_id, e.num_ships, e.flags), (18, 2, 0x80));
    assert_eq!(
        (
            e.ship_list[0].race_id,
            e.ship_list[0].crew_level,
            e.ship_list[0].max_crew,
            e.ship_list[0].max_energy
        ),
        (18, 30, 42, 40)
    );
    assert_eq!(e.ship_list[1].crew_level, 20);
    assert_eq!(e.ship_list[2].race_id, 0);
    assert_eq!((e.log_x, e.log_y), (74565, -70000));
    assert!(e.h_element.is_null());

    let captured = capture_groups(&game).unwrap();
    let mut out = save.clone();
    out.ip_group_queue = captured.ip_group_queue;
    out.encounters = captured.encounters;
    let bytes = encode_save(&out).unwrap();
    let mut enct = b"Enct".to_vec();
    enct.extend(65u32.to_le_bytes());
    enct.extend(ENCOUNTER_WIRE);
    assert!(bytes.windows(enct.len()).any(|w| w == enct.as_slice()));
    let mut igpq = b"IGpQ".to_vec();
    igpq.extend(13u32.to_le_bytes());
    igpq.extend(SEED_GROUP_WIRE);
    assert!(bytes.windows(igpq.len()).any(|w| w == igpq.as_slice()));
}

// ---------------------------------------------------------------------------
// Restore
// ---------------------------------------------------------------------------

#[test]
fn groups_keep_their_order_and_take_the_icon_of_their_own_race() {
    let mut game = LiveGame::new();
    let groups = vec![group(5, 1), group(0, 2), group(17, 3), group(5, 4)];
    restore_groups(&mut game, &sections(groups.clone(), Vec::new())).unwrap();

    let live = raw_groups(&game.ip_groups_ref());
    let races: Vec<u8> = live.iter().map(|g| g.race_id).collect();
    assert_eq!(races, [5, 0, 17, 5]);
    let counters: Vec<u16> = live.iter().map(|g| g.group_counter).collect();
    assert_eq!(counters, [0x101, 0x102, 0x103, 0x104]);
    let icons: Vec<_> = live.iter().map(|g| g.melee_icon).collect();
    assert_eq!(
        icons,
        [
            handle(5, 3).cast(),
            handle(0, 3).cast(),
            handle(17, 3).cast(),
            handle(5, 3).cast()
        ]
    );
    assert_eq!(live[2].loc.x, -103);
    assert_eq!(live[2].loc.y, 203);
    assert_eq!(free_slots(&game.ip_groups_ref()), MAX_GROUPS - 4);

    game.data.game.current_activity = IN_INTERPLANETARY;
    assert_eq!(capture_groups(&game).unwrap().ip_group_queue, groups);
}

#[test]
fn encounters_keep_their_order_and_fields_and_clear_the_element_handle() {
    let mut game = LiveGame::new();
    // Stale encounters with live elements: restore replaces them.
    push_encounter(&mut game, &encounter(3, 1, 9), 0xdead_0000);
    push_encounter(&mut game, &encounter(4, 1, 9), 0xdead_0010);

    let encounters = vec![encounter(18, 3, 1), encounter(7, 7, 2), encounter(2, 0, 3)];
    restore_groups(&mut game, &sections(Vec::new(), encounters.clone())).unwrap();

    let live = raw_encounters(&game.encounters_ref());
    let races: Vec<u8> = live.iter().map(|e| e.race_id).collect();
    assert_eq!(races, [18, 7, 2]);
    assert!(live.iter().all(|e| e.h_element.is_null()));
    assert_eq!(live[1].num_ships, 7);
    assert_eq!(live[1].ship_list[6].crew_level, 10 + 2 + 6);
    assert_eq!(live[1].log_y, -80_002);
    assert_eq!(free_slots(&game.encounters_ref()), MAX_ENCOUNTER_SLOTS - 3);

    game.data.game.current_activity = IN_INTERPLANETARY;
    assert_eq!(capture_groups(&game).unwrap().encounters, encounters);
}

#[test]
fn restore_reads_but_never_changes_the_fleet_templates() {
    let mut game = LiveGame::new();
    let before: Vec<(u32, u16, usize, usize, usize)> = (0..RACES)
        .map(|race| {
            let t = game.template(race);
            (
                t.species_id,
                t.crew_level,
                t.race_strings as usize,
                t.icons as usize,
                t.melee_icon as usize,
            )
        })
        .collect();
    restore_groups(
        &mut game,
        &sections(vec![group(1, 1), group(22, 2)], vec![encounter(18, 2, 1)]),
    )
    .unwrap();
    let after: Vec<(u32, u16, usize, usize, usize)> = (0..RACES)
        .map(|race| {
            let t = game.template(race);
            (
                t.species_id,
                t.crew_level,
                t.race_strings as usize,
                t.icons as usize,
                t.melee_icon as usize,
            )
        })
        .collect();
    assert_eq!(before, after);
    assert_eq!(after[1].2, handle(1, 1) as usize);
}

#[test]
fn restoring_again_replaces_the_queues_and_recycles_the_owner_slots() {
    let mut game = LiveGame::new();
    restore_groups(
        &mut game,
        &sections(
            (0..5).map(|i| group(i, i)).collect(),
            vec![encounter(1, 1, 1), encounter(2, 1, 2)],
        ),
    )
    .unwrap();
    assert_eq!(free_slots(&game.ip_groups_ref()), MAX_GROUPS - 5);

    restore_groups(
        &mut game,
        &sections(vec![group(9, 7)], vec![encounter(3, 2, 5)]),
    )
    .unwrap();
    assert_eq!(raw_groups(&game.ip_groups_ref()).len(), 1);
    assert_eq!(raw_groups(&game.ip_groups_ref())[0].race_id, 9);
    assert_eq!(free_slots(&game.ip_groups_ref()), MAX_GROUPS - 1);
    assert_eq!(raw_encounters(&game.encounters_ref()).len(), 1);
    assert_eq!(free_slots(&game.encounters_ref()), MAX_ENCOUNTER_SLOTS - 1);

    // A full queue restores, and so does the same save twice.
    let full = sections(
        (0..MAX_GROUPS as u8).map(|i| group(i % 24, i)).collect(),
        (0..MAX_ENCOUNTER_SLOTS as u8)
            .map(|i| encounter(i % 24, 1, i))
            .collect(),
    );
    restore_groups(&mut game, &full).unwrap();
    restore_groups(&mut game, &full).unwrap();
    assert_eq!(free_slots(&game.ip_groups_ref()), 0);
    assert_eq!(free_slots(&game.encounters_ref()), 0);
    game.data.game.current_activity = IN_INTERPLANETARY;
    assert_eq!(capture_groups(&game).unwrap(), full);
}

#[test]
fn a_save_without_the_sections_empties_both_queues_as_load_game_does() {
    let mut game = LiveGame::new();
    push_group(&mut game, &group(4, 1), 1);
    push_encounter(&mut game, &encounter(4, 1, 1), 2);
    restore_groups(&mut game, &sections(Vec::new(), Vec::new())).unwrap();
    assert!(raw_groups(&game.ip_groups_ref()).is_empty());
    assert!(raw_encounters(&game.encounters_ref()).is_empty());
    assert_eq!(free_slots(&game.ip_groups_ref()), MAX_GROUPS);
    assert_eq!(free_slots(&game.encounters_ref()), MAX_ENCOUNTER_SLOTS);
}

#[test]
fn seven_ships_is_the_most_an_encounter_lists_and_unused_entries_are_kept() {
    let mut game = LiveGame::new();
    let mut e = encounter(5, 7, 1);
    restore_groups(&mut game, &sections(Vec::new(), vec![e.clone()])).unwrap();

    // Entries after `num_ships` are not read by the game; a stale race there
    // is carried through unchanged.
    e.num_ships = 1;
    e.ship_list[3].race_id = 200;
    restore_groups(&mut game, &sections(Vec::new(), vec![e.clone()])).unwrap();
    assert_eq!(
        raw_encounters(&game.encounters_ref())[0].ship_list[3].race_id,
        200
    );
}

// ---------------------------------------------------------------------------
// Rejected restores leave everything as it was
// ---------------------------------------------------------------------------

fn game_with_live_state() -> LiveGame {
    let mut game = LiveGame::new();
    push_group(&mut game, &group(2, 1), 0x11);
    push_group(&mut game, &group(8, 2), 0x12);
    push_encounter(&mut game, &encounter(18, 2, 1), 0x21);
    game
}

fn assert_rejected(game: &mut LiveGame, bad: GroupSections, expected: GroupError) {
    let before = fingerprint(game);
    let icons: Vec<usize> = raw_groups(&game.ip_groups_ref())
        .iter()
        .map(|g| g.melee_icon as usize)
        .collect();
    let elements: Vec<usize> = raw_encounters(&game.encounters_ref())
        .iter()
        .map(|e| e.h_element as usize)
        .collect();
    assert_eq!(restore_groups(game, &bad), Err(expected));
    assert_eq!(fingerprint(game), before);
    assert_eq!(
        raw_groups(&game.ip_groups_ref())
            .iter()
            .map(|g| g.melee_icon as usize)
            .collect::<Vec<_>>(),
        icons
    );
    assert_eq!(
        raw_encounters(&game.encounters_ref())
            .iter()
            .map(|e| e.h_element as usize)
            .collect::<Vec<_>>(),
        elements
    );
}

#[test]
fn a_group_of_a_race_without_a_fleet_is_rejected() {
    let mut game = game_with_live_state();
    assert_rejected(
        &mut game,
        sections(vec![group(3, 1), group(24, 2)], vec![encounter(1, 1, 1)]),
        GroupError::UnknownRace {
            queue: QueueKind::IpGroups,
            position: 1,
            race: 24,
        },
    );
}

#[test]
fn an_encounter_that_is_malformed_stops_the_whole_restore_even_after_good_groups() {
    let mut game = game_with_live_state();
    assert_rejected(
        &mut game,
        sections(
            vec![group(3, 1)],
            vec![encounter(1, 1, 1), encounter(255, 0, 2)],
        ),
        GroupError::UnknownRace {
            queue: QueueKind::Encounters,
            position: 1,
            race: 255,
        },
    );

    let mut listed = encounter(4, 2, 3);
    listed.ship_list[1].race_id = 99;
    assert_rejected(
        &mut game,
        sections(vec![group(3, 1)], vec![listed]),
        GroupError::UnknownRace {
            queue: QueueKind::Encounters,
            position: 0,
            race: 99,
        },
    );

    let mut crowded = encounter(4, 7, 3);
    crowded.num_ships = 8;
    assert_rejected(
        &mut game,
        sections(Vec::new(), vec![crowded]),
        GroupError::TooManyEncounterShips {
            position: 0,
            num_ships: 8,
        },
    );
}

#[test]
fn more_elements_than_the_pool_has_slots_is_rejected() {
    let mut game = game_with_live_state();
    assert_rejected(
        &mut game,
        sections((0..33).map(|i| group(i % 24, i)).collect(), Vec::new()),
        GroupError::TooMany {
            queue: QueueKind::IpGroups,
            count: 33,
            capacity: MAX_GROUPS,
        },
    );
    assert_rejected(
        &mut game,
        sections(
            vec![group(1, 1)],
            (0..17).map(|i| encounter(i % 24, 1, i)).collect(),
        ),
        GroupError::TooMany {
            queue: QueueKind::Encounters,
            count: 17,
            capacity: MAX_ENCOUNTER_SLOTS,
        },
    );
}

#[test]
fn group_ids_reject_foreign_storage_and_reinit() {
    let mut a = LiveGame::new();
    let b = LiveGame::new();
    let id = a.ip_groups().alloc().unwrap();
    assert!(!b.ip_groups_ref().contains(id));
    a.ip_groups().put(id);
    a.ip_groups().reinit();
    assert!(!a.ip_groups_ref().contains(id));
}

// ---------------------------------------------------------------------------
// Capture
// ---------------------------------------------------------------------------

#[test]
fn capture_reads_the_live_queues_in_order_without_the_codec() {
    let mut game = LiveGame::new();
    game.data.game.current_activity = IN_INTERPLANETARY;
    push_group(&mut game, &group(6, 1), 0);
    push_group(&mut game, &group(2, 2), 0);
    push_encounter(&mut game, &encounter(11, 4, 1), 0x77);
    push_encounter(&mut game, &encounter(12, 1, 2), 0);

    let captured = capture_groups(&game).unwrap();
    assert_eq!(captured.ip_group_queue, vec![group(6, 1), group(2, 2)]);
    assert_eq!(
        captured.encounters,
        vec![encounter(11, 4, 1), encounter(12, 1, 2)]
    );
}

#[test]
fn the_group_queue_is_saved_only_in_plain_interplanetary() {
    assert!(saves_ip_group_queue(IN_INTERPLANETARY));
    assert!(!saves_ip_group_queue(
        IN_INTERPLANETARY | START_INTERPLANETARY
    ));
    assert!(!saves_ip_group_queue(IN_INTERPLANETARY | START_ENCOUNTER));
    assert!(!saves_ip_group_queue(IN_HYPERSPACE));
    assert!(!saves_ip_group_queue(IN_ENCOUNTER));
    assert!(!saves_ip_group_queue(0));
    // Other flag bits (CHECK_LOAD, CHECK_ABORT) do not change the activity.
    assert!(saves_ip_group_queue(IN_INTERPLANETARY | 0x1000));

    let mut game = LiveGame::new();
    push_group(&mut game, &group(6, 1), 0);
    push_encounter(&mut game, &encounter(11, 1, 1), 0);
    for activity in [
        IN_HYPERSPACE,
        IN_INTERPLANETARY | START_ENCOUNTER,
        IN_INTERPLANETARY | START_INTERPLANETARY,
    ] {
        game.data.game.current_activity = activity;
        let captured = capture_groups(&game).unwrap();
        assert!(captured.ip_group_queue.is_empty(), "activity {activity:#x}");
        // "Enct" does not depend on the activity.
        assert_eq!(captured.encounters.len(), 1);
    }
}

#[test]
fn capture_refuses_what_restore_would_refuse() {
    let mut game = LiveGame::new();
    game.data.game.current_activity = IN_INTERPLANETARY;
    push_group(&mut game, &group(30, 1), 0);
    assert_eq!(
        capture_groups(&game),
        Err(GroupError::UnknownRace {
            queue: QueueKind::IpGroups,
            position: 0,
            race: 30
        })
    );

    let mut game = LiveGame::new();
    let mut bad = encounter(3, 1, 1);
    bad.num_ships = 9;
    push_encounter(&mut game, &bad, 0);
    assert_eq!(
        capture_groups(&game),
        Err(GroupError::TooManyEncounterShips {
            position: 0,
            num_ships: 9
        })
    );
}

#[test]
fn load_game_retains_the_build_group_zero_template_icon() {
    let mut game = LiveGame::new();
    restore_loaded_groups(&mut game, &sections(vec![group(7, 1)], Vec::new())).unwrap();
    let loaded = raw_groups(&game.ip_groups_ref());
    assert_eq!(loaded[0].race_id, 7);
    assert_eq!(loaded[0].melee_icon, handle(0, 3).cast());
}
