//! Tests of `ip_group_save`, with expected file bytes written out by hand from
//! `FlushGroupInfo`/`PutGroupInfo` and the group file layout.

use super::ip_group_save::{expiry_date, plan_save_ip_groups, save_ip_groups, Date, SaveIpInput};
use super::statefiles::{
    capture_from_images, initial_defined_image, initial_random_image, StateSectionError,
};
use crate::savegame::decode_save;
use crate::savegame::types::{IpGroup, Point, GAME_STATE_BYTES};
use crate::state::state_file::{StateFileManager, DEFGRPINFO_FILE, RANDGRPINFO_FILE};

const C_WRITTEN_SAVE: &[u8] = include_bytes!(
    "../../../../docs/evidence/epic-20/exact-production-79e68744e/load-save/seed-uqmsave.00"
);

const STAR: u16 = 0x0102;
const NOW: Date = Date {
    day_index: 20,
    month_index: 3,
    year_index: 2155,
};

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
    out
}

fn group(group_id: u8, race_id: u8, in_system: u8) -> IpGroup {
    IpGroup {
        group_counter: 0x0102,
        race_id,
        sys_loc: 3,
        task: 2,
        in_system,
        dest_loc: 4,
        orbit_pos: 6,
        group_id,
        loc: Point { x: -5, y: 300 },
    }
}

/// The record of `group(group_id, race_id, _)` in the file.
fn record(group_id: u8, race_id: u8, in_system: u8) -> Vec<u8> {
    vec![
        race_id, 0, 0, 0, 0, 0x02, 0x01, race_id, 0x23, in_system, 0, 0x64, group_id, 0xFB, 0xFF,
        0x2C, 0x01,
    ]
}

fn plan(
    random: &[u8],
    defined: &[u8],
    battle_group_ref: u32,
    groups: &[IpGroup],
) -> Result<super::ip_group_save::SaveIpPlan, StateSectionError> {
    plan_save_ip_groups(
        random,
        defined,
        &SaveIpInput {
            star_index: STAR,
            now: NOW,
            battle_group_ref,
            groups,
        },
    )
}

#[test]
fn expiry_is_seven_days_on_with_month_and_year_rollover() {
    let date = |day_index, month_index, year_index| Date {
        day_index,
        month_index,
        year_index,
    };
    assert_eq!(expiry_date(date(20, 3, 2155)).unwrap(), date(27, 3, 2155));
    assert_eq!(expiry_date(date(25, 1, 2155)).unwrap(), date(1, 2, 2155));
    assert_eq!(expiry_date(date(28, 2, 2155)).unwrap(), date(7, 3, 2155));
    assert_eq!(expiry_date(date(29, 2, 2156)).unwrap(), date(7, 3, 2156));
    assert_eq!(expiry_date(date(30, 12, 2155)).unwrap(), date(6, 1, 2156));
    assert_eq!(
        expiry_date(date(1, 0, 2155)),
        Err(StateSectionError::ValueTooWide {
            field: "month_index",
            value: 0
        })
    );
}

#[test]
fn a_system_with_groups_in_it_gets_a_stamped_header_and_the_list() {
    let groups = [group(7, 5, 1), group(8, 6, 1)];
    let planned = plan(
        &initial_random_image(),
        &initial_defined_image(),
        0,
        &groups,
    )
    .unwrap();

    let mut expected = header(0, 27, 3, STAR, 2155, &[(0, 268)]);
    expected.extend_from_slice(&[0, 2]);
    expected.extend(record(7, 5, 1));
    expected.extend(record(8, 6, 1));
    assert_eq!(planned.random_groups, expected);
    assert_eq!(planned.defined_groups, None);
    assert!(planned.departed.is_empty());
}

#[test]
fn an_existing_list_is_overwritten_in_place_and_extended() {
    let mut random = header(0, 1, 1, 9, 2150, &[(0, 268)]);
    random.extend_from_slice(&[7, 1]);
    random.extend(record(1, 4, 1));
    let planned = plan(
        &random,
        &initial_defined_image(),
        0,
        &[group(2, 5, 1), group(3, 6, 1)],
    )
    .unwrap();

    let mut expected = header(0, 27, 3, STAR, 2155, &[(0, 268)]);
    expected.extend_from_slice(&[0, 2]);
    expected.extend(record(2, 5, 1));
    expected.extend(record(3, 6, 1));
    assert_eq!(planned.random_groups, expected);
}

#[test]
fn a_group_that_left_is_flushed_empty_then_overwritten_by_the_list() {
    let groups = [group(3, 9, 0), group(4, 5, 1)];
    let planned = plan(
        &initial_random_image(),
        &initial_defined_image(),
        0,
        &groups,
    )
    .unwrap();

    // Slot 3 was appended at the end of the file, where the list was put too,
    // and the list is written last.
    let mut expected = header(3, 27, 3, STAR, 2155, &[(0, 268)]);
    expected.extend_from_slice(&[0, 1]);
    expected.extend(record(4, 5, 1));
    assert_eq!(planned.random_groups, expected);
    assert_eq!(planned.departed, vec![0]);
}

#[test]
fn a_group_that_left_during_an_encounter_goes_to_the_defined_file() {
    let mut defined = vec![0];
    defined.extend(header(0, 1, 1, 9, 2150, &[]));
    let planned = plan(&initial_random_image(), &defined, 1, &[group(2, 9, 0)]).unwrap();

    let mut expected_random = header(0, 27, 3, STAR, 2155, &[(0, 268)]);
    expected_random.extend_from_slice(&[0, 0]);
    assert_eq!(planned.random_groups, expected_random);

    let mut expected_defined = vec![0];
    expected_defined.extend(header(2, 27, 3, STAR, 2155, &[(2, 269)]));
    expected_defined.extend_from_slice(&[0, 0]);
    assert_eq!(planned.defined_groups, Some(expected_defined));
    assert_eq!(planned.departed, vec![0]);
}

#[test]
fn rejected_groups_and_files_are_typed_errors() {
    let fresh = initial_random_image();
    let defined = initial_defined_image();
    assert_eq!(
        plan(&fresh, &defined, 0, &[group(0, 5, 0)]),
        Err(StateSectionError::GroupIdOutOfRange { group_id: 0 })
    );
    assert_eq!(
        plan(&fresh, &defined, 0, &[group(65, 5, 0)]),
        Err(StateSectionError::GroupIdOutOfRange { group_id: 65 })
    );
    // group 2 is within NumGroups but its slot was never allocated
    let unallocated = header(5, 1, 1, 9, 2150, &[(0, 268)]);
    assert_eq!(
        plan(&unallocated, &defined, 0, &[group(2, 5, 0)]),
        Err(StateSectionError::GroupSlotUnallocated { group_id: 2 })
    );
    assert!(matches!(
        plan(&fresh[..100], &defined, 0, &[]),
        Err(StateSectionError::OutOfBounds {
            file: RANDGRPINFO_FILE,
            ..
        })
    ));
    assert!(matches!(
        plan(&fresh, &defined, 1, &[group(2, 5, 0)]),
        Err(StateSectionError::OutOfBounds {
            file: DEFGRPINFO_FILE,
            ..
        })
    ));
    let mut list = group(1, 5, 1);
    list.sys_loc = 16;
    assert_eq!(
        plan(&fresh, &defined, 0, &[list]),
        Err(StateSectionError::ValueTooWide {
            field: "sys_loc",
            value: 16
        })
    );
}

#[test]
fn a_rejected_save_leaves_both_files_as_they_were() {
    let mut files = StateFileManager::new();
    files
        .replace_contents(RANDGRPINFO_FILE, &initial_random_image())
        .unwrap();
    files
        .replace_contents(DEFGRPINFO_FILE, &initial_defined_image())
        .unwrap();
    let contents =
        |files: &StateFileManager, index: usize| files.get_file(index).unwrap().contents().to_vec();
    let (random, defined) = (
        contents(&files, RANDGRPINFO_FILE),
        contents(&files, DEFGRPINFO_FILE),
    );

    let rejected = [group(4, 5, 1), group(0, 6, 0)];
    let result = save_ip_groups(
        &mut files,
        &SaveIpInput {
            star_index: STAR,
            now: NOW,
            battle_group_ref: 0,
            groups: &rejected,
        },
    );
    assert_eq!(
        result,
        Err(StateSectionError::GroupIdOutOfRange { group_id: 0 })
    );
    assert_eq!(contents(&files, RANDGRPINFO_FILE), random);
    assert_eq!(contents(&files, DEFGRPINFO_FILE), defined);

    let accepted = [group(3, 9, 0), group(4, 5, 1)];
    let departed = save_ip_groups(
        &mut files,
        &SaveIpInput {
            star_index: STAR,
            now: NOW,
            battle_group_ref: 0,
            groups: &accepted,
        },
    )
    .unwrap();
    assert_eq!(departed, vec![0]);
    assert_eq!(
        contents(&files, RANDGRPINFO_FILE),
        plan(&random, &defined, 0, &accepted).unwrap().random_groups
    );
}

#[test]
fn the_c_written_group_list_survives_the_save_and_reads_back() {
    let seed = decode_save(C_WRITTEN_SAVE).unwrap();
    let list = seed
        .group_list
        .expect("the C-written save has a Grps chunk");
    let groups: Vec<IpGroup> = list
        .entries
        .iter()
        .map(|entry| entry.group.clone())
        .collect();
    let planned = plan(
        &initial_random_image(),
        &initial_defined_image(),
        0,
        &groups,
    )
    .unwrap();
    let sections = capture_from_images(
        &[],
        &planned.random_groups,
        &initial_defined_image(),
        &[0; GAME_STATE_BYTES],
        0,
    )
    .unwrap();
    let read = sections.group_list.unwrap();
    assert_eq!(read.last_encounter, 0);
    let read_groups: Vec<IpGroup> = read
        .entries
        .iter()
        .map(|entry| entry.group.clone())
        .collect();
    assert_eq!(read_groups, groups);
}
