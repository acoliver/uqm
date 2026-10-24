//! Capture and restore of the "Scan", "Grps" and "BtGp" sections against the
//! live state files (`STARINFO`, `RANDGRPINFO`, `DEFGRPINFO`).
//!
//! The files are the ones the running game reads and writes through
//! `rust_*_state_file`. Their working layout is not the wire layout:
//!
//! * A group header is 268 bytes: `NumGroups`, day, month, one pad byte, a
//!   16-bit star index, a 16-bit year, then 65 absolute 32-bit offsets
//!   (`WriteGroupHeader`, `NUM_SAVED_BATTLE_GROUPS + 1`).
//! * A ship inside a group is 1 byte of outer race plus a 16-byte fragment
//!   (`WriteShipFragment`); the wire carries 10 bytes of the 17.
//! * An entry of the group list is 1 byte of outer race plus a 16-byte
//!   `IP_GROUP` (`WriteIpGroup`); the wire carries 14 bytes of it.
//!
//! Capture follows `SaveStarInfo`, `SaveGroups` and `SaveBattleGroup`. Restore
//! follows `LoadScanInfo`, `LoadGroupList` and `LoadBattleGroup`, including the
//! reset `InitGroupInfo(TRUE)` makes when the first group chunk arrives.
//!
//! Restore builds the complete new file images and the new game-state bytes
//! first. Nothing live changes unless every check passed, and the commit runs
//! with both owners locked and cannot fail, so a rejected save leaves the old
//! files and state as they were.
//!
//! Where this differs from the C code on purpose:
//!
//! * A `STARINFO` whose length is not a multiple of 4 makes capture fail; C
//!   logged a warning and left the chunk out of the save.
//! * A group slot whose offset is 0 (a group that left the system) is captured
//!   as a fleet without ships; C read the file header as if it were a group.
//!   `GetGroupInfo` treats a group without ships as dead, as it does offset 0.
//! * Crew values above 255 are rejected on restore; `swrite_8` cut them.
//! * A random group list is captured as absent when `RANDGRPINFO` has no list
//!   (offset 0); C read the header bytes as list entries.
//!
//! Not checked here: race ids against the available-race queue, which belongs
//! to the unported "RacQ" owner.

use std::fmt;

use super::RuntimeError;
use crate::comm::locdata::CGameState;
use crate::savegame::types::{
    BattleFleet, BattleGroup, BattleShip, GroupList, GroupListEntry, IpGroup, Point,
    GAME_STATE_BYTES,
};
use crate::state::ffi::{with_state_files, with_state_owners, GameStateOwnerError};
use crate::state::game_state::{
    get_state_32_raw, set_state_32_raw, GameState, NUM_GAME_STATE_BITS,
};
use crate::state::game_state_keys::bit_range;
use crate::state::state_file::{
    StateFileManager, DEFGRPINFO_FILE, RANDGRPINFO_FILE, STARINFO_FILE,
};

/// `NUM_SAVED_BATTLE_GROUPS` (grpinfo.h).
pub const NUM_SAVED_BATTLE_GROUPS: usize = 64;
/// Size of `GROUP_HEADER` as `WriteGroupHeader` writes it.
pub const GROUP_HEADER_BYTES: usize = GROUP_OFFSETS_AT + 4 * (NUM_SAVED_BATTLE_GROUPS + 1);
/// Where the offset array starts inside a header; `GroupOffset[0]` is first.
pub(super) const GROUP_OFFSETS_AT: usize = 8;
/// The 16 bytes `WriteIpGroup` writes.
const IP_GROUP_BYTES: usize = 16;
/// Outer race byte plus the `IP_GROUP`.
const GROUP_LIST_RECORD_BYTES: usize = 1 + IP_GROUP_BYTES;
/// `star_index` of a header nobody has written yet (`(COUNT)~0`).
const UNINITIALISED_STAR: u16 = u16::MAX;
/// Game-state bits per stored group offset (`GET_GAME_STATE_32` stride).
const OFFSET_STRIDE_BITS: usize = 32;

/// Defined (encounter) groups: from `SHOFIXTI_GRPOFFS0` to the end of the
/// game-state array, one 32-bit offset each.
fn defined_group_slots() -> (usize, usize) {
    let (first, _) = bit_range("SHOFIXTI_GRPOFFS0").expect("SHOFIXTI_GRPOFFS0 is a game-state key");
    let first = first as usize;
    (first, (NUM_GAME_STATE_BITS - first) / OFFSET_STRIDE_BITS)
}

/// Why the state-file sections could not be captured or restored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateSectionError {
    Owner(GameStateOwnerError),
    /// `STARINFO` is not a whole number of 32-bit words.
    StarInfoNotWordAligned {
        len: usize,
    },
    /// A read of `needed` bytes at `offset` runs past the `len`-byte file.
    OutOfBounds {
        file: usize,
        offset: usize,
        needed: usize,
        len: usize,
    },
    /// A group header names more groups than it has offsets for.
    TooManyGroups {
        count: usize,
    },
    /// A group list or fleet holds more than 255 entries.
    TooManyEntries {
        count: usize,
    },
    /// A defined group's id is not one of the encounter slots.
    EncounterOutOfRange {
        encounter_id: u32,
        slots: usize,
    },
    /// Two chunks describe the same defined group.
    DuplicateEncounter {
        encounter_id: u32,
    },
    /// Two defined groups claim to be the current one.
    MultipleCurrentGroups,
    /// A defined group offset in the game state points at no restored group.
    StaleDefinedGroupOffset {
        encounter_id: u32,
        offset: u32,
    },
    /// A group id that is not one of the `NUM_SAVED_BATTLE_GROUPS` slots.
    GroupIdOutOfRange {
        group_id: u8,
    },
    /// A group that left the system has no slot in its header to flush into.
    GroupSlotUnallocated {
        group_id: u8,
    },
    /// A value does not fit the byte or nibble the state file stores.
    ValueTooWide {
        field: &'static str,
        value: u32,
    },
}

impl fmt::Display for StateSectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Owner(error) => write!(f, "state owners: {error}"),
            Self::StarInfoNotWordAligned { len } => {
                write!(f, "STARINFO is {len} bytes, not a whole number of words")
            }
            Self::OutOfBounds {
                file,
                offset,
                needed,
                len,
            } => write!(
                f,
                "state file {file}: {needed} bytes at offset {offset} exceed its {len} bytes"
            ),
            Self::TooManyGroups { count } => write!(
                f,
                "{count} battle groups, a header holds {NUM_SAVED_BATTLE_GROUPS}"
            ),
            Self::TooManyEntries { count } => write!(f, "{count} entries do not fit one byte"),
            Self::EncounterOutOfRange {
                encounter_id,
                slots,
            } => write!(f, "defined group {encounter_id}, there are {slots} slots"),
            Self::DuplicateEncounter { encounter_id } => {
                write!(f, "defined group {encounter_id} appears twice")
            }
            Self::MultipleCurrentGroups => f.write_str("more than one group is current"),
            Self::StaleDefinedGroupOffset {
                encounter_id,
                offset,
            } => write!(
                f,
                "game state keeps offset {offset} for defined group {encounter_id}, which the save does not hold"
            ),
            Self::GroupIdOutOfRange { group_id } => {
                write!(f, "group id {group_id} is not one of the {NUM_SAVED_BATTLE_GROUPS} slots")
            }
            Self::GroupSlotUnallocated { group_id } => {
                write!(f, "group {group_id} has no slot to flush into")
            }
            Self::ValueTooWide { field, value } => {
                write!(f, "{field} {value} does not fit the state file")
            }
        }
    }
}

impl std::error::Error for StateSectionError {}

impl From<GameStateOwnerError> for StateSectionError {
    fn from(error: GameStateOwnerError) -> Self {
        Self::Owner(error)
    }
}

impl From<StateSectionError> for RuntimeError {
    fn from(error: StateSectionError) -> Self {
        Self::StateFiles(error)
    }
}

/// The three sections as the save model carries them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StateFileSections {
    pub scan: Option<Vec<u32>>,
    pub canned_nodes: Vec<crate::state::planet_info::CannedNode>,
    pub group_list: Option<GroupList>,
    pub battle_groups: Vec<BattleGroup>,
}

// ---------------------------------------------------------------------------
// Reading the working layout
// ---------------------------------------------------------------------------

pub(super) struct Reader<'a> {
    file: usize,
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    pub(super) fn at(file: usize, bytes: &'a [u8], offset: usize) -> Self {
        Self {
            file,
            bytes,
            at: offset,
        }
    }

    fn take(&mut self, needed: usize) -> Result<&'a [u8], StateSectionError> {
        let out_of_bounds = StateSectionError::OutOfBounds {
            file: self.file,
            offset: self.at,
            needed,
            len: self.bytes.len(),
        };
        let end = self.at.checked_add(needed).ok_or(out_of_bounds)?;
        let slice = self.bytes.get(self.at..end).ok_or(out_of_bounds)?;
        self.at = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, StateSectionError> {
        Ok(self.take(1)?[0])
    }

    fn skip(&mut self, count: usize) -> Result<(), StateSectionError> {
        self.take(count).map(|_| ())
    }

    fn u16(&mut self) -> Result<u16, StateSectionError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn i16(&mut self) -> Result<i16, StateSectionError> {
        Ok(self.u16()? as i16)
    }

    fn u32(&mut self) -> Result<u32, StateSectionError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
}

/// `GROUP_HEADER`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GroupHeader {
    pub(super) num_groups: u8,
    pub(super) day_index: u8,
    pub(super) month_index: u8,
    pub(super) star_index: u16,
    pub(super) year_index: u16,
    pub(super) group_offset: [u32; NUM_SAVED_BATTLE_GROUPS + 1],
}

impl GroupHeader {
    /// `memset (&GH, 0, ...)` followed by `GH.star_index = (COUNT)~0`.
    fn uninitialised() -> Self {
        Self {
            num_groups: 0,
            day_index: 0,
            month_index: 0,
            star_index: UNINITIALISED_STAR,
            year_index: 0,
            group_offset: [0; NUM_SAVED_BATTLE_GROUPS + 1],
        }
    }

    /// `ReadGroupHeader`.
    pub(super) fn read(r: &mut Reader<'_>) -> Result<Self, StateSectionError> {
        let num_groups = r.u8()?;
        let day_index = r.u8()?;
        let month_index = r.u8()?;
        r.skip(1)?;
        let star_index = r.u16()?;
        let year_index = r.u16()?;
        let mut group_offset = [0; NUM_SAVED_BATTLE_GROUPS + 1];
        for offset in &mut group_offset {
            *offset = r.u32()?;
        }
        Ok(Self {
            num_groups,
            day_index,
            month_index,
            star_index,
            year_index,
            group_offset,
        })
    }

    /// `WriteGroupHeader` into `image` at `at`, which must hold a header.
    pub(super) fn write_over(&self, image: &mut [u8], at: usize) {
        let out = &mut image[at..at + GROUP_HEADER_BYTES];
        out[0] = self.num_groups;
        out[1] = self.day_index;
        out[2] = self.month_index;
        out[3] = 0;
        out[4..6].copy_from_slice(&self.star_index.to_le_bytes());
        out[6..8].copy_from_slice(&self.year_index.to_le_bytes());
        for (slot, offset) in out[GROUP_OFFSETS_AT..]
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(self.group_offset)
        {
            *slot = offset.to_le_bytes();
        }
    }
}

// ---------------------------------------------------------------------------
// Capture
// ---------------------------------------------------------------------------

/// `SaveStarInfo`: the file as 32-bit words.
fn capture_scan(star_info: &[u8]) -> Result<Vec<u32>, StateSectionError> {
    if !star_info.len().is_multiple_of(4) {
        return Err(StateSectionError::StarInfoNotWordAligned {
            len: star_info.len(),
        });
    }
    Ok(star_info
        .as_chunks::<4>()
        .0
        .iter()
        .map(|w| u32::from_le_bytes(*w))
        .collect())
}

/// The list written at `GroupOffset[0]`: `SaveGroups` without the header.
fn capture_group_list(
    file: usize,
    image: &[u8],
    offset: usize,
) -> Result<GroupList, StateSectionError> {
    let mut r = Reader::at(file, image, offset);
    let last_encounter = r.u8()?;
    let count = r.u8()?;
    let mut entries = Vec::with_capacity(usize::from(count));
    for _ in 0..count {
        let race_outer = r.u8()?;
        r.skip(4)?; // unused which_side, captains_name_index, padding
        let group_counter = r.u16()?;
        let race_id = r.u8()?;
        let place = r.u8()?;
        let in_system = r.u8()?;
        r.skip(1)?; // unused max_crew
        let heading = r.u8()?;
        let group_id = r.u8()?;
        let x = r.i16()?;
        let y = r.i16()?;
        entries.push(GroupListEntry {
            race_outer,
            group: IpGroup {
                group_counter,
                race_id,
                sys_loc: place & 0x0F,
                task: place >> 4,
                in_system,
                dest_loc: heading & 0x0F,
                orbit_pos: heading >> 4,
                group_id,
                loc: Point { x, y },
            },
        });
    }
    Ok(GroupList {
        last_encounter,
        entries,
    })
}

/// `SaveBattleGroup`, reading the header at `header_at`.
fn capture_battle_group(
    file: usize,
    image: &[u8],
    encounter_id: u32,
    header_at: usize,
    current: bool,
) -> Result<BattleGroup, StateSectionError> {
    let header = GroupHeader::read(&mut Reader::at(file, image, header_at))?;
    let count = usize::from(header.num_groups);
    if count > NUM_SAVED_BATTLE_GROUPS {
        return Err(StateSectionError::TooManyGroups { count });
    }
    let mut fleets = Vec::with_capacity(count);
    for &offset in &header.group_offset[1..=count] {
        if offset == 0 {
            fleets.push(BattleFleet {
                icon: 0,
                ships: Vec::new(),
            });
            continue;
        }
        let mut r = Reader::at(file, image, offset as usize);
        let icon = r.u8()?;
        let num_ships = r.u8()?;
        let mut ships = Vec::with_capacity(usize::from(num_ships));
        for _ in 0..num_ships {
            let race_outer = r.u8()?;
            r.skip(2)?; // unused which_side
            let captains_name_index = r.u8()?;
            r.skip(3)?; // padding, unused ship_flags
            let race_id = r.u8()?;
            let index = r.u8()?;
            let crew_level = u16::from(r.u8()?);
            let max_crew = u16::from(r.u8()?);
            let energy_level = r.u8()?;
            let max_energy = r.u8()?;
            r.skip(4)?; // unused loc
            ships.push(BattleShip {
                race_outer,
                captains_name_index,
                race_id,
                index,
                crew_level,
                max_crew,
                energy_level,
                max_energy,
            });
        }
        fleets.push(BattleFleet { icon, ships });
    }
    Ok(BattleGroup {
        encounter_id,
        current,
        star_index: header.star_index,
        day_index: header.day_index,
        month_index: header.month_index,
        year_index: header.year_index,
        fleets,
    })
}

/// The sections for the given file images and game state.
pub fn capture_from_images(
    star_info: &[u8],
    random_groups: &[u8],
    defined_groups: &[u8],
    game_state: &[u8; GAME_STATE_BYTES],
    battle_group_ref: u32,
) -> Result<StateFileSections, StateSectionError> {
    let mut sections = StateFileSections {
        scan: Some(capture_scan(star_info)?),
        ..StateFileSections::default()
    };

    if !random_groups.is_empty() {
        let header = GroupHeader::read(&mut Reader::at(RANDGRPINFO_FILE, random_groups, 0))?;
        if header.group_offset[0] != 0 {
            sections.group_list = Some(capture_group_list(
                RANDGRPINFO_FILE,
                random_groups,
                header.group_offset[0] as usize,
            )?);
        }
        sections.battle_groups.push(capture_battle_group(
            RANDGRPINFO_FILE,
            random_groups,
            0,
            0,
            false,
        )?);
    }

    if !defined_groups.is_empty() {
        let (first_bit, slots) = defined_group_slots();
        for slot in 0..slots {
            let offset = get_state_32_raw(game_state, first_bit + slot * OFFSET_STRIDE_BITS);
            if offset != 0 {
                sections.battle_groups.push(capture_battle_group(
                    DEFGRPINFO_FILE,
                    defined_groups,
                    slot as u32 + 1,
                    offset as usize,
                    battle_group_ref == offset,
                )?);
            }
        }
    }
    Ok(sections)
}

/// Read "Scan", "Grps" and "BtGp" from the live state files, with the
/// defined-group offsets taken from the live game-state singleton and the
/// current group from `game`.
pub fn capture_state_files(game: &CGameState) -> Result<StateFileSections, StateSectionError> {
    with_state_owners(|state, files| {
        let contents = |index: usize| {
            files
                .get_file(index)
                .expect("the three state files always exist")
                .contents()
        };
        let mut sections = capture_from_images(
            contents(STARINFO_FILE),
            contents(RANDGRPINFO_FILE),
            contents(DEFGRPINFO_FILE),
            state.as_bytes(),
            game.battle_group_ref,
        )?;
        sections.canned_nodes = files.canned_nodes.clone();
        Ok(sections)
    })?
}

// ---------------------------------------------------------------------------
// Restore
// ---------------------------------------------------------------------------

/// `InitGroupInfo`: the `RANDGRPINFO` image of an empty system.
pub fn initial_random_image() -> Vec<u8> {
    let mut image = vec![0; GROUP_HEADER_BYTES];
    GroupHeader::uninitialised().write_over(&mut image, 0);
    image
}

/// `InitGroupInfo (TRUE)`: the `DEFGRPINFO` image, one byte so that no group
/// header starts at offset 0, the offset that means "random".
pub fn initial_defined_image() -> Vec<u8> {
    vec![0]
}

fn byte(field: &'static str, value: u16) -> Result<u8, StateSectionError> {
    u8::try_from(value).map_err(|_| StateSectionError::ValueTooWide {
        field,
        value: u32::from(value),
    })
}

fn nibble(field: &'static str, value: u8) -> Result<u8, StateSectionError> {
    if value > 0x0F {
        return Err(StateSectionError::ValueTooWide {
            field,
            value: u32::from(value),
        });
    }
    Ok(value)
}

fn count_byte(count: usize) -> Result<u8, StateSectionError> {
    u8::try_from(count).map_err(|_| StateSectionError::TooManyEntries { count })
}

/// The bytes of a group list as `SaveGroupList` and `FlushGroupInfo` lay them
/// out in the file: `LastEncGroup`, the count, then one outer race byte plus
/// `WriteIpGroup` per entry.
pub(super) fn group_list_bytes(list: &GroupList) -> Result<Vec<u8>, StateSectionError> {
    let count = count_byte(list.entries.len())?;
    let mut records = Vec::with_capacity(2 + list.entries.len() * GROUP_LIST_RECORD_BYTES);
    records.push(list.last_encounter);
    records.push(count);
    for entry in &list.entries {
        let g = &entry.group;
        let place = nibble("sys_loc", g.sys_loc)? | nibble("task", g.task)? << 4;
        let heading = nibble("dest_loc", g.dest_loc)? | nibble("orbit_pos", g.orbit_pos)? << 4;
        records.push(entry.race_outer);
        records.extend_from_slice(&[0; 4]);
        records.extend_from_slice(&g.group_counter.to_le_bytes());
        records.extend_from_slice(&[g.race_id, place, g.in_system, 0, heading, g.group_id]);
        records.extend_from_slice(&g.loc.x.to_le_bytes());
        records.extend_from_slice(&g.loc.y.to_le_bytes());
    }
    Ok(records)
}

/// `LoadGroupList`: append the list to `image` and point `GroupOffset[0]` at it.
fn append_group_list(image: &mut Vec<u8>, list: &GroupList) -> Result<(), StateSectionError> {
    let records = group_list_bytes(list)?;
    let offset = image.len() as u32;
    image.extend_from_slice(&records);
    image[GROUP_OFFSETS_AT..GROUP_OFFSETS_AT + 4].copy_from_slice(&offset.to_le_bytes());
    Ok(())
}

/// `LoadBattleGroup`: append the fleets to `image` and write the header at
/// `header_at`, which already holds the header to update.
fn append_battle_group(
    image: &mut Vec<u8>,
    header_at: usize,
    mut header: GroupHeader,
    group: &BattleGroup,
) -> Result<(), StateSectionError> {
    if group.fleets.len() > NUM_SAVED_BATTLE_GROUPS {
        return Err(StateSectionError::TooManyGroups {
            count: group.fleets.len(),
        });
    }
    header.star_index = group.star_index;
    header.day_index = group.day_index;
    header.month_index = group.month_index;
    header.year_index = group.year_index;
    header.num_groups = group.fleets.len() as u8;
    for (slot, fleet) in group.fleets.iter().enumerate() {
        header.group_offset[slot + 1] = image.len() as u32;
        image.push(fleet.icon);
        image.push(count_byte(fleet.ships.len())?);
        for ship in &fleet.ships {
            image.push(ship.race_outer);
            image.extend_from_slice(&[0, 0, ship.captains_name_index, 0, 0, 0]);
            image.push(ship.race_id);
            image.push(ship.index);
            image.push(byte("crew_level", ship.crew_level)?);
            image.push(byte("max_crew", ship.max_crew)?);
            image.push(ship.energy_level);
            image.push(ship.max_energy);
            image.extend_from_slice(&[0; 4]);
        }
    }
    header.write_over(image, header_at);
    Ok(())
}

/// Everything a restore will do, fully checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestorePlan {
    /// New `STARINFO`, when the save has a "Scan" chunk.
    pub star_info: Option<Vec<u8>>,
    /// New `RANDGRPINFO` and `DEFGRPINFO`, when the save has group chunks.
    pub groups: Option<(Vec<u8>, Vec<u8>)>,
    /// The game state with the defined-group offsets of the new files.
    pub game_state: [u8; GAME_STATE_BYTES],
    /// `BattleGroupRef` after the restore, when group chunks reset it.
    pub battle_group_ref: Option<u32>,
}

/// Check `sections` and build the images and game state a restore installs.
/// `game_state` is the state the save's "GmSt" chunk produced.
pub fn plan_restore(
    sections: &StateFileSections,
    game_state: &[u8; GAME_STATE_BYTES],
) -> Result<RestorePlan, StateSectionError> {
    let mut plan = RestorePlan {
        star_info: sections.scan.as_ref().map(|words| {
            words
                .iter()
                .flat_map(|word| word.to_le_bytes())
                .collect::<Vec<u8>>()
        }),
        groups: None,
        game_state: *game_state,
        battle_group_ref: None,
    };
    if sections.group_list.is_none() && sections.battle_groups.is_empty() {
        return Ok(plan);
    }

    let (first_bit, slots) = defined_group_slots();
    let mut random = initial_random_image();
    let mut defined = initial_defined_image();
    let mut current = 0;
    let mut covered = vec![false; slots];

    if let Some(list) = &sections.group_list {
        append_group_list(&mut random, list)?;
    }
    for group in &sections.battle_groups {
        if group.encounter_id == 0 {
            let header = GroupHeader::read(&mut Reader::at(RANDGRPINFO_FILE, &random, 0))?;
            append_battle_group(&mut random, 0, header, group)?;
            continue;
        }
        let slot = (group.encounter_id - 1) as usize;
        if slot >= slots {
            return Err(StateSectionError::EncounterOutOfRange {
                encounter_id: group.encounter_id,
                slots,
            });
        }
        if std::mem::replace(&mut covered[slot], true) {
            return Err(StateSectionError::DuplicateEncounter {
                encounter_id: group.encounter_id,
            });
        }
        let offset = defined.len();
        defined.resize(offset + GROUP_HEADER_BYTES, 0);
        append_battle_group(&mut defined, offset, GroupHeader::uninitialised(), group)?;
        set_state_32_raw(
            &mut plan.game_state,
            first_bit + slot * OFFSET_STRIDE_BITS,
            offset as u32,
        );
        if group.current {
            if current != 0 {
                return Err(StateSectionError::MultipleCurrentGroups);
            }
            current = offset as u32;
        }
    }
    for (slot, covered) in covered.iter().enumerate() {
        let stale = get_state_32_raw(&plan.game_state, first_bit + slot * OFFSET_STRIDE_BITS);
        if !covered && stale != 0 {
            return Err(StateSectionError::StaleDefinedGroupOffset {
                encounter_id: slot as u32 + 1,
                offset: stale,
            });
        }
    }
    plan.groups = Some((random, defined));
    plan.battle_group_ref = Some(current);
    Ok(plan)
}

/// Install `plan`. Both owners are held by the caller; nothing here fails.
fn commit(plan: &RestorePlan, state: &mut GameState, files: &mut StateFileManager) {
    let mut install = |index: usize, image: &[u8]| {
        files
            .replace_contents(index, image)
            .expect("the three state files always exist");
    };
    if let Some(star_info) = &plan.star_info {
        install(STARINFO_FILE, star_info);
    }
    if let Some((random, defined)) = &plan.groups {
        install(RANDGRPINFO_FILE, random);
        install(DEFGRPINFO_FILE, defined);
    }
    *state = GameState::from_bytes(&plan.game_state);
}

/// Make the live state files, game-state singleton and `game` hold `sections`.
///
/// Call after the game state of the save is installed (`restore_runtime`): the
/// defined-group offsets of the new files are written over those bits. The C
/// shadow of the game state in `game` and `BattleGroupRef` follow.
pub fn restore_state_files(
    game: &mut CGameState,
    sections: &StateFileSections,
) -> Result<(), StateSectionError> {
    let plan = with_state_owners(|state, files| {
        let plan = plan_restore(sections, state.as_bytes())?;
        commit(&plan, state, files);
        files.canned_nodes.clone_from(&sections.canned_nodes);
        Ok::<_, StateSectionError>(plan)
    })??;
    game.game_state_bytes = plan.game_state;
    if let Some(reference) = plan.battle_group_ref {
        game.battle_group_ref = reference;
    }
    Ok(())
}

/// `InitGroupInfo` on the live files: empty the random groups and, when
/// `first_time`, restart the defined groups.
pub fn init_group_info(first_time: bool) -> Result<(), StateSectionError> {
    with_state_files(|files| {
        files
            .replace_contents(RANDGRPINFO_FILE, &initial_random_image())
            .expect("the three state files always exist");
        if first_time {
            files
                .replace_contents(DEFGRPINFO_FILE, &initial_defined_image())
                .expect("the three state files always exist");
        }
    })
    .map_err(StateSectionError::from)
}
