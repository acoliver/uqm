//! `PutGroupInfo (GROUPS_RANDOM, GROUP_SAVE_IP)` (grpinfo.c) in Rust: the step
//! of `SaveGame` that writes the interplanetary groups of the current system
//! into `RANDGRPINFO`, so that the "Grps" and "BtGp" chunks that follow carry
//! the groups as they are now.
//!
//! The work is `PutGroupInfo` (stamp the header with the date the groups stay
//! good until and with the system's star) followed by `FlushGroupInfo` for
//! `GROUP_LIST`:
//!
//! * `GroupOffset[0]` is the end of the file when the list was never written.
//! * A group of the queue that is no longer in the system is written out as an
//!   empty group in its own slot, in `DEFGRPINFO` when `BattleGroupRef` is set
//!   and in `RANDGRPINFO` when it is not, its slot in the random header is
//!   cleared and the group leaves the queue.
//! * The random header is written, then the list at `GroupOffset[0]`: the
//!   `LastEncGroup` byte (0 for a save), the count of the groups that stayed
//!   and their records. Like the C seek-and-write, this overwrites whatever
//!   was at that offset and extends the file when the list is longer.
//!
//! The functions work on the state-file images and on the owner borrow the
//! caller already holds. [`plan_save_ip_groups`] builds the new images without
//! changing anything and [`save_ip_groups`] installs them, so a rejected
//! header or slot leaves both files as they were.
//!
//! Where this differs from the C code on purpose:
//!
//! * A group that left the system and whose slot has no offset although its
//!   id is within `NumGroups` makes C write the empty group over the file
//!   header at offset 0. That is an error here, as is a group id of 0 or
//!   above `NUM_SAVED_BATTLE_GROUPS`, which C indexes out of the header.
//! * A file shorter than a header is an error; C reads whatever follows.
//!
//! Not ported, so a save is not complete without them:
//!
//! * `LastEncGroup` is a `static BYTE` of grpinfo.c that `GetGroupInfo` sets
//!   and this step resets to 0. It has no Rust owner, so the caller must still
//!   clear it.
//! * Removing the departed groups from the live `ip_group_q` (`RemoveQueue`
//!   and `FreeIpGroup`); this module reports their positions.

use super::statefiles::{
    group_list_bytes, GroupHeader, Reader, StateSectionError, NUM_SAVED_BATTLE_GROUPS,
};
use crate::savegame::types::{ClockState, GroupList, GroupListEntry, IpGroup};
use crate::state::state_file::{StateFileManager, DEFGRPINFO_FILE, RANDGRPINFO_FILE};
use crate::time::clock_bridge::days_in_month;

/// How long the groups of a system stay good, in days (`PutGroupInfo`).
const GROUP_LIFETIME_DAYS: u32 = 7;

/// A calendar date as the group headers store it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    pub day_index: u8,
    pub month_index: u8,
    pub year_index: u16,
}

impl From<&ClockState> for Date {
    fn from(clock: &ClockState) -> Self {
        Self {
            day_index: clock.day_index,
            month_index: clock.month_index,
            year_index: clock.year_index,
        }
    }
}

/// `ValidateEvent (RELATIVE_EVENT, 0, 7, 0)` against `now`: the date the
/// groups are good until.
pub fn expiry_date(now: Date) -> Result<Date, StateSectionError> {
    if !(1..=12).contains(&now.month_index) {
        return Err(StateSectionError::ValueTooWide {
            field: "month_index",
            value: u32::from(now.month_index),
        });
    }
    // month 0 is added to (month - 1), so the month stays the same
    let mut month = u32::from(now.month_index);
    let mut year = u32::from(now.year_index);
    let mut day = u32::from(now.day_index) + GROUP_LIFETIME_DAYS;
    // The month number is at most 12 and a year fits 16 bits (checked below);
    // `days_in_month` takes a byte and a 16-bit year.
    loop {
        let in_month = u32::from(days_in_month(
            month as u8,
            year.min(u32::from(u16::MAX)) as u16,
        ));
        if day <= in_month {
            break;
        }
        day -= in_month;
        month += 1;
        if month > 12 {
            month = 1;
            year += 1;
        }
    }
    let year = u16::try_from(year).map_err(|_| StateSectionError::ValueTooWide {
        field: "year_index",
        value: year,
    })?;
    Ok(Date {
        day_index: day as u8,
        month_index: month as u8,
        year_index: year,
    })
}

/// What `SaveGame` knows when it persists the groups.
#[derive(Debug, Clone, Copy)]
pub struct SaveIpInput<'a> {
    /// `CurStarDescPtr - star_array`.
    pub star_index: u16,
    /// The game clock.
    pub now: Date,
    /// `GLOBAL (BattleGroupRef)`: the `DEFGRPINFO` offset of the current
    /// encounter's group header, or 0.
    pub battle_group_ref: u32,
    /// `GLOBAL (ip_group_q)`, head to tail.
    pub groups: &'a [IpGroup],
}

/// The images a save installs, and the groups that leave the queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveIpPlan {
    pub random_groups: Vec<u8>,
    /// Only present when a departed group was recorded in `DEFGRPINFO`.
    pub defined_groups: Option<Vec<u8>>,
    /// Positions in the input queue of the groups that left the system and
    /// must be removed from it, in queue order.
    pub departed: Vec<usize>,
}

fn stamp(header: &mut GroupHeader, expiry: Date, star_index: u16) {
    header.day_index = expiry.day_index;
    header.month_index = expiry.month_index;
    header.year_index = expiry.year_index;
    header.star_index = star_index;
}

/// `seek (at)` and write: overwrite, extending the file when the write ends
/// beyond it. Seeking past the end is an error.
fn write_at(
    file: usize,
    image: &mut Vec<u8>,
    at: usize,
    bytes: &[u8],
) -> Result<(), StateSectionError> {
    if at > image.len() {
        return Err(StateSectionError::OutOfBounds {
            file,
            offset: at,
            needed: bytes.len(),
            len: image.len(),
        });
    }
    let end = at + bytes.len();
    if end > image.len() {
        image.resize(end, 0);
    }
    image[at..end].copy_from_slice(bytes);
    Ok(())
}

/// `FlushGroupInfo` for a group that left: make sure the header has a slot for
/// `group_id`, write the header at `header_at` and an empty group (no race, no
/// ships) at the slot.
fn flush_empty_group(
    file: usize,
    image: &mut Vec<u8>,
    header_at: usize,
    header: &mut GroupHeader,
    group_id: u8,
) -> Result<(), StateSectionError> {
    let slot = usize::from(group_id);
    if slot == 0 || slot > NUM_SAVED_BATTLE_GROUPS {
        return Err(StateSectionError::GroupIdOutOfRange { group_id });
    }
    if group_id > header.num_groups {
        header.num_groups = group_id;
        header.group_offset[slot] =
            u32::try_from(image.len()).map_err(|_| StateSectionError::ValueTooWide {
                field: "state file length",
                value: u32::MAX,
            })?;
    } else if header.group_offset[slot] == 0 {
        return Err(StateSectionError::GroupSlotUnallocated { group_id });
    }
    header.write_over(image, header_at);
    write_at(file, image, header.group_offset[slot] as usize, &[0, 0])
}

/// `PutGroupInfo (BattleGroupRef, group_id)` for a group that left: stamp the
/// header of the current encounter's group and flush the empty group into it.
fn put_defined_empty_group(
    image: &mut Vec<u8>,
    offset: usize,
    group_id: u8,
    expiry: Date,
    star_index: u16,
) -> Result<(), StateSectionError> {
    let mut header = GroupHeader::read(&mut Reader::at(DEFGRPINFO_FILE, image, offset))?;
    stamp(&mut header, expiry, star_index);
    flush_empty_group(DEFGRPINFO_FILE, image, offset, &mut header, group_id)
}

/// Build the images `PutGroupInfo (GROUPS_RANDOM, GROUP_SAVE_IP)` leaves.
///
/// `random_groups` and `defined_groups` are the current `RANDGRPINFO` and
/// `DEFGRPINFO`. Nothing is changed.
pub fn plan_save_ip_groups(
    random_groups: &[u8],
    defined_groups: &[u8],
    input: &SaveIpInput<'_>,
) -> Result<SaveIpPlan, StateSectionError> {
    let mut random = random_groups.to_vec();
    let mut defined = defined_groups.to_vec();
    let mut defined_changed = false;
    let expiry = expiry_date(input.now)?;
    let mut header = GroupHeader::read(&mut Reader::at(RANDGRPINFO_FILE, &random, 0))?;
    stamp(&mut header, expiry, input.star_index);
    if header.group_offset[0] == 0 {
        header.group_offset[0] =
            u32::try_from(random.len()).map_err(|_| StateSectionError::ValueTooWide {
                field: "state file length",
                value: u32::MAX,
            })?;
    }

    let mut staying = Vec::new();
    let mut departed = Vec::new();
    for (position, group) in input.groups.iter().enumerate() {
        if group.in_system != 0 {
            staying.push(GroupListEntry {
                race_outer: group.race_id,
                group: group.clone(),
            });
            continue;
        }
        if input.battle_group_ref != 0 {
            put_defined_empty_group(
                &mut defined,
                input.battle_group_ref as usize,
                group.group_id,
                expiry,
                input.star_index,
            )?;
            defined_changed = true;
        } else {
            flush_empty_group(
                RANDGRPINFO_FILE,
                &mut random,
                0,
                &mut header,
                group.group_id,
            )?;
        }
        header.group_offset[usize::from(group.group_id)] = 0;
        departed.push(position);
    }

    header.write_over(&mut random, 0);
    let list = group_list_bytes(&GroupList {
        last_encounter: 0,
        entries: staying,
    })?;
    write_at(
        RANDGRPINFO_FILE,
        &mut random,
        header.group_offset[0] as usize,
        &list,
    )?;

    Ok(SaveIpPlan {
        random_groups: random,
        defined_groups: defined_changed.then_some(defined),
        departed,
    })
}

/// `PutGroupInfo (GROUPS_RANDOM, GROUP_SAVE_IP)` on the live state files, which
/// the caller holds. Returns the positions of the groups that left the system
/// and must be removed from `ip_group_q`.
///
/// Nothing is written when the plan is rejected.
pub fn save_ip_groups(
    files: &mut StateFileManager,
    input: &SaveIpInput<'_>,
) -> Result<Vec<usize>, StateSectionError> {
    let contents = |files: &StateFileManager, index: usize| -> Vec<u8> {
        files
            .get_file(index)
            .expect("the three state files always exist")
            .contents()
            .to_vec()
    };
    let plan = plan_save_ip_groups(
        &contents(files, RANDGRPINFO_FILE),
        &contents(files, DEFGRPINFO_FILE),
        input,
    )?;
    files
        .replace_contents(RANDGRPINFO_FILE, &plan.random_groups)
        .expect("the three state files always exist");
    if let Some(defined) = &plan.defined_groups {
        files
            .replace_contents(DEFGRPINFO_FILE, defined)
            .expect("the three state files always exist");
    }
    Ok(plan.departed)
}
