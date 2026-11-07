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

use super::super::clock::days_in_month;
use super::super::world::state_file::{StateFileManager, DEFGRPINFO_FILE, RANDGRPINFO_FILE};
use super::statefiles::StateSectionError;
pub use uqm_save::ip_groups::{Date, SaveIpInput, SaveIpPlan};

/// How long the groups of a system stay good, in days (`PutGroupInfo`).
const GROUP_LIFETIME_DAYS: u32 = 7;

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

pub fn plan_save_ip_groups(
    random_groups: &[u8],
    defined_groups: &[u8],
    input: &SaveIpInput<'_>,
) -> Result<SaveIpPlan, StateSectionError> {
    uqm_save::ip_groups::plan_save_ip_groups(
        random_groups,
        defined_groups,
        input,
        expiry_date(input.now)?,
    )
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
