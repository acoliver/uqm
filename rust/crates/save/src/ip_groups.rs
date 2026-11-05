//! Group state-file image encoding and patching.
use super::statefiles::{
    group_list_bytes, GroupHeader, Reader, StateSectionError, DEFGRPINFO_FILE,
    NUM_SAVED_BATTLE_GROUPS, RANDGRPINFO_FILE,
};
use crate::types::{ClockState, GroupList, GroupListEntry, IpGroup};

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
    expiry: Date,
) -> Result<SaveIpPlan, StateSectionError> {
    let mut random = random_groups.to_vec();
    let mut defined = defined_groups.to_vec();
    let mut defined_changed = false;
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
