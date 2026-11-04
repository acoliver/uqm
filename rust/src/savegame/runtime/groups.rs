//! Capture and restore of the "IGpQ" and "Enct" sections against the game's
//! live queues: `GLOBAL (ip_group_q)` of `IP_GROUP`s and `GLOBAL (encounter_q)`
//! of `ENCOUNTER`s.
//!
//! World owns the fixed-capacity group and encounter pools. Capture traverses
//! typed links; restore validates before replacing records in file order.
//!
//! Capture follows `SaveGroupQueue` and `SaveEncounters`:
//!
//! * "IGpQ" is written only when the save is made in `IN_INTERPLANETARY`
//!   without `START_INTERPLANETARY` or `START_ENCOUNTER`
//!   (`saves_ip_group_queue`); in every other activity the section is absent
//!   and the capture is empty. The npc ship queue has the mirror-image
//!   condition (`START_ENCOUNTER`) in `SaveGame`, which is
//!   `queues::saves_npc_ship_queue`.
//! * "Enct" is written whenever the encounter queue holds an element.
//!
//! Restore follows `LoadGame`, which empties both queues before it reads the
//! first chunk (a save without the section therefore leaves the queue empty),
//! and `LoadGroupQueue` and the "Enct" case of `LoadGame`, which allocate each
//! element from the queue's own pool and `PutQueue` it in file order.
//!
//! Where this differs from the C code on purpose:
//!
//! * `LoadGroupQueue` builds every group with `BuildGroup (queue, 0)` and then
//!   overwrites `race_id`, so every loaded group keeps the `melee_icon` of
//!   fleet 0, whatever its race. The icon is not saved; it is the handle of
//!   the race's fleet template, and the group is drawn with it. Here the group
//!   takes the icon of the template of the race the save names, which is what
//!   `BuildGroup` does for a group the game creates. A race without a template
//!   is `UnknownRace`; it is never coerced to another one.
//! * `LoadEncounter` does not check what it reads. `num_ships` above
//!   `MAX_HYPER_SHIPS` and races without a fleet are rejected.
//!
//! The not-saved `hElement` of an encounter is zero after a restore, as in
//! `LoadEncounter`. Fleet templates are only read, never written.
//!
//! Everything that can be rejected is checked before the first write, so a
//! rejected save leaves both queues as they were.
//!
//! Not ported here: `InitGameStructures` (the pools and the fleet templates),
//! `BuildGroups` and the C group exports (`GetGroupInfo`, `PutGroupInfo`,
//! `InitGroupInfo`), `LastEncGroup`, the `RANDGRPINFO` and `DEFGRPINFO` files'
//! relation to this queue, and the `SaveGame`/`LoadGame` orchestration that
//! decides, from the activity, which sections a file carries.

use std::ffi::c_void;
use std::fmt;

use super::queues::{c_point, entries, point, QueueElement, QueueError, QueueKind};
use crate::c_bindings::{assert_c_layout, global_state};
use crate::comm::dispatch::{
    CEncounter, CFrameHandle, CPoint, IN_INTERPLANETARY, MAX_HYPER_SHIPS, START_ENCOUNTER,
    START_INTERPLANETARY,
};
use crate::engine::world::queues::{
    BriefShipInfo as ShipInfo, EncounterRecord, IpGroup as GroupRecord,
};
use crate::engine::World;
use crate::savegame::types::{self, BriefShipInfo, EncounterEntry, IpGroup};

const _: () = assert!(MAX_HYPER_SHIPS == types::MAX_HYPER_SHIPS);

/// C: `IP_GROUP` (grpinfo.h) — an element of `ip_group_q`.
#[repr(C)]
pub struct CIpGroup {
    pub pred: *mut c_void,
    pub succ: *mut c_void,
    pub group_counter: u16,
    pub race_id: u8,
    pub sys_loc: u8,
    pub task: u8,
    pub in_system: u8,
    pub dest_loc: u8,
    pub orbit_pos: u8,
    pub group_id: u8,
    pub loc: CPoint,
    pub melee_icon: CFrameHandle,
}

assert_c_layout!(
    CIpGroup,
    global_state::IP_GROUP {
        pred: pred,
        succ: succ,
        group_counter: group_counter,
        race_id: race_id,
        sys_loc: sys_loc,
        task: task,
        in_system: in_system,
        dest_loc: dest_loc,
        orbit_pos: orbit_pos,
        group_id: group_id,
        loc: loc,
        melee_icon: melee_icon,
    }
);

/// `MAX_BATTLE_GROUPS` (globdata.h): slots of `ip_group_q`.
pub const MAX_BATTLE_GROUPS: u8 = global_state::MAX_BATTLE_GROUPS as u8;
/// `MAX_ENCOUNTERS` (globdata.h): slots of `encounter_q`.
pub const MAX_ENCOUNTERS: u8 = global_state::MAX_ENCOUNTERS as u8;

impl QueueElement for CIpGroup {
    fn pred(&self) -> *mut c_void {
        self.pred
    }
    fn succ(&self) -> *mut c_void {
        self.succ
    }
}

impl QueueElement for CEncounter {
    fn pred(&self) -> *mut c_void {
        self.pred
    }
    fn succ(&self) -> *mut c_void {
        self.succ
    }
}

/// Why the group and encounter sections could not be captured or restored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupError {
    /// A pool or its links are not what the game's queues are.
    Queue(QueueError),
    /// The entry names a race the fleet queue has no template for.
    UnknownRace {
        queue: QueueKind,
        position: usize,
        race: u8,
    },
    /// More elements than the queue has slots.
    TooMany {
        queue: QueueKind,
        count: usize,
        capacity: usize,
    },
    /// An encounter lists more ships than `MAX_HYPER_SHIPS`.
    TooManyEncounterShips { position: usize, num_ships: u8 },
}

impl fmt::Display for GroupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for GroupError {}

impl From<QueueError> for GroupError {
    fn from(error: QueueError) -> Self {
        Self::Queue(error)
    }
}

/// The group and encounter queues as a save holds them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupSections {
    pub ip_group_queue: Vec<IpGroup>,
    pub encounters: Vec<EncounterEntry>,
}

/// Whether `SaveGame` writes "IGpQ" in a save made in `activity`.
pub fn saves_ip_group_queue(activity: u16) -> bool {
    activity & START_INTERPLANETARY == 0
        && activity & START_ENCOUNTER == 0
        && activity & 0x00ff == IN_INTERPLANETARY
}

fn ip_group_entry(group: &GroupRecord) -> IpGroup {
    IpGroup {
        group_counter: group.group_counter,
        race_id: group.race_id,
        sys_loc: group.sys_loc,
        task: group.task,
        in_system: group.in_system,
        dest_loc: group.dest_loc,
        orbit_pos: group.orbit_pos,
        group_id: group.group_id,
        loc: point(group.loc),
    }
}

fn encounter_entry(encounter: &EncounterRecord) -> EncounterEntry {
    EncounterEntry {
        transition_state: encounter.transition_state,
        origin: point(encounter.origin),
        radius: encounter.radius,
        loc_pt: point(encounter.loc_pt),
        race_id: encounter.race_id,
        num_ships: encounter.num_ships,
        flags: encounter.flags,
        ship_list: std::array::from_fn(|i| {
            let ship = &encounter.ShipList[i];
            BriefShipInfo {
                race_id: ship.race_id,
                crew_level: ship.crew_level,
                max_crew: ship.max_crew,
                max_energy: ship.max_energy,
            }
        }),
        log_x: encounter.log_x,
        log_y: encounter.log_y,
    }
}

fn validate_groups(entries: &[IpGroup], race_count: usize) -> Result<(), GroupError> {
    for (position, entry) in entries.iter().enumerate() {
        if usize::from(entry.race_id) >= race_count {
            return Err(GroupError::UnknownRace {
                queue: QueueKind::IpGroups,
                position,
                race: entry.race_id,
            });
        }
    }
    Ok(())
}

/// The races of an encounter and of the ships it lists, which the game
/// indexes into the fleet queue.
fn validate_encounters(entries: &[EncounterEntry], race_count: usize) -> Result<(), GroupError> {
    for (position, entry) in entries.iter().enumerate() {
        if usize::from(entry.num_ships) > MAX_HYPER_SHIPS {
            return Err(GroupError::TooManyEncounterShips {
                position,
                num_ships: entry.num_ships,
            });
        }
        let listed = &entry.ship_list[..usize::from(entry.num_ships)];
        for race in std::iter::once(entry.race_id).chain(listed.iter().map(|ship| ship.race_id)) {
            if usize::from(race) >= race_count {
                return Err(GroupError::UnknownRace {
                    queue: QueueKind::Encounters,
                    position,
                    race,
                });
            }
        }
    }
    Ok(())
}

/// The group and encounter queues of `game`, head to tail, as a save writes
/// them. The group queue is empty when `saves_ip_group_queue` is false for the
/// game's activity.
pub fn capture_groups(world: &World) -> Result<GroupSections, GroupError> {
    let races = world.avail_races_ref().count();
    let ip_group_queue = if saves_ip_group_queue(world.activity()) {
        entries(world.ip_groups_ref(), |_, group| ip_group_entry(group))
    } else {
        Vec::new()
    };
    validate_groups(&ip_group_queue, races)?;
    let encounters = entries(world.encounters_ref(), |_, encounter| {
        encounter_entry(encounter)
    });
    validate_encounters(&encounters, races)?;
    Ok(GroupSections {
        ip_group_queue,
        encounters,
    })
}

fn check_capacity(queue: QueueKind, count: usize, capacity: usize) -> Result<(), GroupError> {
    if count > capacity {
        return Err(GroupError::TooMany {
            queue,
            count,
            capacity,
        });
    }
    Ok(())
}

/// Make the live queues hold the given sections.
///
/// Both queues are emptied first, as `LoadGame` does, then filled in order from
/// their own pools. On error the queues are untouched.
pub fn restore_groups(game: &mut World, sections: &GroupSections) -> Result<(), GroupError> {
    restore_groups_with_icons(game, sections, false)
}

/// LoadGame calls BuildGroup(queue, 0) before overwriting the saved race byte.
/// Keep its fleet-zero icon at this ABI boundary.
pub(crate) fn restore_loaded_groups(
    game: &mut World,
    sections: &GroupSections,
) -> Result<(), GroupError> {
    restore_groups_with_icons(game, sections, true)
}

fn restore_groups_with_icons(
    game: &mut World,
    sections: &GroupSections,
    first_fleet_icon: bool,
) -> Result<(), GroupError> {
    let templates = entries(game.avail_races_ref(), |_, fleet| *fleet);
    check_capacity(
        QueueKind::IpGroups,
        sections.ip_group_queue.len(),
        MAX_BATTLE_GROUPS as usize,
    )?;
    check_capacity(
        QueueKind::Encounters,
        sections.encounters.len(),
        MAX_ENCOUNTERS as usize,
    )?;
    validate_groups(&sections.ip_group_queue, templates.len())?;
    validate_encounters(&sections.encounters, templates.len())?;
    let mut groups = game.ip_groups();
    groups.reinit();
    for entry in &sections.ip_group_queue {
        let icon = if first_fleet_icon {
            0
        } else {
            usize::from(entry.race_id)
        };
        let id = groups
            .alloc()
            .expect("group capacity validated before installation");
        *groups.get_mut(id) = GroupRecord {
            group_counter: entry.group_counter,
            race_id: entry.race_id,
            sys_loc: entry.sys_loc,
            task: entry.task,
            in_system: entry.in_system,
            dest_loc: entry.dest_loc,
            orbit_pos: entry.orbit_pos,
            group_id: entry.group_id,
            loc: c_point(entry.loc),
            melee_icon: templates[icon].melee_icon,
            ..Default::default()
        };
        groups.put(id);
    }
    let mut encounters = game.encounters();
    encounters.reinit();
    for entry in &sections.encounters {
        let id = encounters
            .alloc()
            .expect("encounter capacity validated before installation");
        *encounters.get_mut(id) = EncounterRecord {
            transition_state: entry.transition_state,
            origin: c_point(entry.origin),
            radius: entry.radius,
            race_id: entry.race_id,
            num_ships: entry.num_ships,
            flags: entry.flags,
            loc_pt: c_point(entry.loc_pt),
            ShipList: std::array::from_fn(|i| {
                let ship = &entry.ship_list[i];
                ShipInfo {
                    race_id: ship.race_id,
                    crew_level: ship.crew_level,
                    max_crew: ship.max_crew,
                    max_energy: ship.max_energy,
                }
            }),
            log_x: entry.log_x,
            log_y: entry.log_y,
            ..Default::default()
        };
        encounters.put(id);
    }
    Ok(())
}
