//! Capture and restore of the "RacQ", "NpcQ" and "ShpQ" sections against the
//! game's live queues: `GLOBAL (avail_race_q)`, `npc_built_ship_q` and
//! `built_ship_q`.
//!
//! All three are `QUEUE_TABLE` pools driven by the Rust queue owner in
//! `collections::queue`. `avail_race_q` holds one `FLEET_INFO` per race and is
//! built once by `InitGameStructures` (the template resources are loaded
//! there, which is still C). The two ship queues hold `SHIP_FRAGMENT`s.
//!
//! Capture follows `SaveRaceQueue` and `SaveShipQueue`. Restore follows
//! `LoadRaceQueue` and `LoadShipQueue`, plus the `ReinitQueue` calls `LoadGame`
//! makes first:
//!
//! * `avail_race_q` is not rebuilt. Each saved entry names a queue position
//!   and replaces only the fields `LoadRaceQueue` reads. The links, the
//!   species id and the string, icon and melee-icon handles stay as the
//!   initialised template left them.
//! * `npc_built_ship_q` and `built_ship_q` are emptied, then each saved entry
//!   is cloned from its race's template with the Rust `CloneShipFragment`
//!   (`comm::dispatch`), and the fields `LoadShipQueue` reads are overwritten.
//!   The clone draws a captain name first, as the C clone does, so the random
//!   stream advances the same way.
//!
//! Everything that can be rejected is checked before the first write: the
//! pools, the race of every entry, the capacity of the ship queues. Allocation
//! cannot fail afterwards, so a rejected save leaves all three queues as they
//! were.
//!
//! Where this differs from the C code on purpose:
//!
//! * `CloneShipFragment` returned 0 for a race the fleet queue does not have
//!   and `LoadShipQueue` then dereferenced it; here that is `UnknownRace`.
//! * The two copies of a ship's race in a "ShpQ" entry (the clone selector
//!   and `race_id`) must agree, as the game always writes them.
//! * A fleet position named twice, an allied state other than `DEAD_GUY`,
//!   `GOOD_GUY` or `BAD_GUY`, a fleet function that is not an event kind and
//!   an escort that has more crew than its ship holds are rejected.
//!
//! Not ported here: `InitGameStructures`, which makes `avail_race_q` and its
//! templates. The "IGpQ" and "Enct" queues are in `groups`, which reuses the
//! pool walker of this module.

use std::ffi::c_void;
use std::fmt;
use std::mem::{align_of, size_of};

use crate::collections::queue::{Queue, ReinitQueue};
use crate::comm::dispatch::{
    clone_ship_fragment_from, production_captain_name, CFleetInfo, CPoint, CShipFragment,
    INFINITE_FLEET, START_ENCOUNTER, START_INTERPLANETARY,
};
use crate::comm::locdata::{CGameState, CQueue};
use crate::savegame::types::{Point, RaceEntry, ShipFragmentEntry};
use crate::time::clock_bridge::NUM_EVENTS;

const _: () = {
    assert!(size_of::<CQueue>() == size_of::<Queue>());
    assert!(align_of::<CQueue>() == align_of::<Queue>());
};

/// `MAX_BUILT_SHIPS` (sis.h): hangar slots, so the bound of an escort's index.
pub const MAX_BUILT_SHIPS: u8 = 12;
/// Values of `FLEET_INFO.allied_state` (races.h): `DEAD_GUY`, `GOOD_GUY`,
/// `BAD_GUY`.
const ALLIED_STATES: u16 = 3;
/// `(BYTE) ~0`: a fleet with no function to call on arrival.
const NO_FLEET_FUNCTION: u8 = u8::MAX;

/// The three queues of this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueKind {
    AvailableRaces,
    NpcShips,
    BuiltShips,
    IpGroups,
    Encounters,
}

/// Why the queue sections could not be captured or restored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueError {
    /// The queue has no slot table, so it was never initialised.
    PoolMissing { queue: QueueKind },
    /// The queue's slots are not the size of its element.
    WrongObjectSize {
        queue: QueueKind,
        found: u16,
        expected: usize,
    },
    /// The slot table is not aligned for its element.
    MisalignedPool { queue: QueueKind },
    /// A link is outside the pool, off a slot boundary, repeated, or its
    /// back-link, or the queue's tail, disagrees with the forward links.
    CorruptQueue { queue: QueueKind, position: usize },
    /// More ships than the queue has slots.
    TooManyShips {
        queue: QueueKind,
        count: usize,
        capacity: usize,
    },
    /// The entry names a race the fleet queue has no template for.
    UnknownRace {
        queue: QueueKind,
        position: usize,
        race: u16,
    },
    /// The clone selector and `race_id` of a ship entry differ.
    RaceIdMismatch {
        queue: QueueKind,
        position: usize,
        race_index: u16,
        race_id: u8,
    },
    /// Two fleet entries name the same queue position.
    DuplicateFleet { position: usize, index: u16 },
    /// `allied_state` is none of `DEAD_GUY`, `GOOD_GUY` and `BAD_GUY`.
    UnknownAlliedState { position: usize, allied_state: u16 },
    /// `func_index` is neither `~0` nor an event kind.
    UnknownFleetFunction { position: usize, func_index: u8 },
    /// An entry holds more crew than its `max_crew`.
    CrewExceedsMax {
        queue: QueueKind,
        position: usize,
        crew_level: u16,
        max_crew: u16,
    },
    /// An escort's hangar slot is not one of `MAX_BUILT_SHIPS`.
    EscortSlotOutOfRange { position: usize, index: u8 },
}

impl fmt::Display for QueueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for QueueError {}

/// The race, npc-ship and built-ship queues as a save holds them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueSections {
    pub race_queue: Vec<RaceEntry>,
    pub npc_ship_queue: Vec<ShipFragmentEntry>,
    pub ship_queue: Vec<ShipFragmentEntry>,
}

/// An element that starts with the `LINK` fields.
pub(super) trait QueueElement {
    fn pred(&self) -> *mut c_void;
    fn succ(&self) -> *mut c_void;
}

impl QueueElement for CFleetInfo {
    fn pred(&self) -> *mut c_void {
        self.pred
    }
    fn succ(&self) -> *mut c_void {
        self.succ
    }
}

impl QueueElement for CShipFragment {
    fn pred(&self) -> *mut c_void {
        self.pred
    }
    fn succ(&self) -> *mut c_void {
        self.succ
    }
}

/// Slots of a queue that really is a pool of `T`.
pub(super) fn pool_slots<T: QueueElement>(
    queue: &CQueue,
    kind: QueueKind,
) -> Result<usize, QueueError> {
    if queue.pq_tab.is_null() {
        return Err(QueueError::PoolMissing { queue: kind });
    }
    if usize::from(queue.object_size) != size_of::<T>() {
        return Err(QueueError::WrongObjectSize {
            queue: kind,
            found: queue.object_size,
            expected: size_of::<T>(),
        });
    }
    if !(queue.pq_tab as usize).is_multiple_of(align_of::<T>()) {
        return Err(QueueError::MisalignedPool { queue: kind });
    }
    Ok(usize::from(queue.num_objects))
}

/// The links of `queue`, head to tail.
///
/// Every link is checked against the pool, so a queue the game corrupted is an
/// error and not a save of whatever the pointers reach.
pub(super) fn walk<T: QueueElement>(
    queue: &CQueue,
    kind: QueueKind,
) -> Result<Vec<*mut T>, QueueError> {
    let slots = pool_slots::<T>(queue, kind)?;
    let pool = queue.pq_tab as usize;
    let slot_size = usize::from(queue.object_size);

    let mut links: Vec<*mut T> = Vec::new();
    let mut previous: *mut c_void = std::ptr::null_mut();
    let mut link = queue.head;
    while !link.is_null() {
        let position = links.len();
        let offset = (link as usize).wrapping_sub(pool);
        // A queue holds each slot at most once, so more links than slots is a loop.
        if position >= slots || offset >= slots * slot_size || !offset.is_multiple_of(slot_size) {
            return Err(QueueError::CorruptQueue {
                queue: kind,
                position,
            });
        }
        // SAFETY: `link` is an aligned slot inside the pool, checked above,
        // and the pool holds `T`s (size and alignment checked by `pool_slots`).
        let element = unsafe { &*(link as *const T) };
        if element.pred() != previous {
            return Err(QueueError::CorruptQueue {
                queue: kind,
                position,
            });
        }
        links.push(link.cast::<T>());
        previous = link;
        link = element.succ();
    }
    if previous != queue.tail {
        return Err(QueueError::CorruptQueue {
            queue: kind,
            position: links.len(),
        });
    }
    Ok(links)
}

pub(super) fn point(p: CPoint) -> Point {
    Point { x: p.x, y: p.y }
}

pub(super) fn c_point(p: Point) -> CPoint {
    CPoint { x: p.x, y: p.y }
}

fn race_entry(index: usize, fleet: &CFleetInfo) -> RaceEntry {
    RaceEntry {
        // `Index` is the position in the queue and the queue holds at most
        // `u8::MAX` slots, so it fits.
        index: index as u16,
        allied_state: fleet.allied_state,
        days_left: fleet.days_left,
        growth_fract: fleet.growth_fract,
        crew_level: fleet.crew_level,
        max_crew: fleet.max_crew,
        growth: fleet.growth,
        max_energy: fleet.max_energy,
        loc: point(fleet.loc),
        actual_strength: fleet.actual_strength,
        known_strength: fleet.known_strength,
        known_loc: point(fleet.known_loc),
        growth_err_term: fleet.growth_err_term,
        func_index: fleet.func_index,
        dest_loc: point(fleet.dest_loc),
    }
}

fn ship_entry(fragment: &CShipFragment) -> ShipFragmentEntry {
    ShipFragmentEntry {
        race_index: u16::from(fragment.race_id),
        captains_name_index: fragment.captains_name_index,
        race_id: fragment.race_id,
        index: fragment.index,
        crew_level: fragment.crew_level,
        max_crew: fragment.max_crew,
        energy_level: fragment.energy_level,
        max_energy: fragment.max_energy,
    }
}

/// What a fleet entry must hold for the game to run on it.
fn validate_races(entries: &[RaceEntry], race_count: usize) -> Result<(), QueueError> {
    let mut seen = vec![false; race_count];
    for (position, entry) in entries.iter().enumerate() {
        let slot = usize::from(entry.index);
        if slot >= race_count {
            return Err(QueueError::UnknownRace {
                queue: QueueKind::AvailableRaces,
                position,
                race: entry.index,
            });
        }
        if std::mem::replace(&mut seen[slot], true) {
            return Err(QueueError::DuplicateFleet {
                position,
                index: entry.index,
            });
        }
        if entry.allied_state >= ALLIED_STATES {
            return Err(QueueError::UnknownAlliedState {
                position,
                allied_state: entry.allied_state,
            });
        }
        if entry.func_index != NO_FLEET_FUNCTION && usize::from(entry.func_index) >= NUM_EVENTS {
            return Err(QueueError::UnknownFleetFunction {
                position,
                func_index: entry.func_index,
            });
        }
    }
    Ok(())
}

/// What a ship entry must hold for the clone and the fields after it.
fn validate_ships(
    queue: QueueKind,
    entries: &[ShipFragmentEntry],
    race_count: usize,
) -> Result<(), QueueError> {
    for (position, entry) in entries.iter().enumerate() {
        if usize::from(entry.race_index) >= race_count {
            return Err(QueueError::UnknownRace {
                queue,
                position,
                race: entry.race_index,
            });
        }
        if entry.race_index != u16::from(entry.race_id) {
            return Err(QueueError::RaceIdMismatch {
                queue,
                position,
                race_index: entry.race_index,
                race_id: entry.race_id,
            });
        }
        // An npc fleet of unlimited size has `INFINITE_FLEET` crew.
        let unlimited = queue == QueueKind::NpcShips && entry.crew_level == INFINITE_FLEET;
        if entry.crew_level > entry.max_crew && !unlimited {
            return Err(QueueError::CrewExceedsMax {
                queue,
                position,
                crew_level: entry.crew_level,
                max_crew: entry.max_crew,
            });
        }
        if queue == QueueKind::BuiltShips && entry.index >= MAX_BUILT_SHIPS {
            return Err(QueueError::EscortSlotOutOfRange {
                position,
                index: entry.index,
            });
        }
    }
    Ok(())
}

fn capture_ships(
    queue: &CQueue,
    kind: QueueKind,
    race_count: usize,
) -> Result<Vec<ShipFragmentEntry>, QueueError> {
    let entries: Vec<ShipFragmentEntry> = walk::<CShipFragment>(queue, kind)?
        .into_iter()
        // SAFETY: `walk` returned slots of the pool, which holds `SHIP_FRAGMENT`s.
        .map(|link| ship_entry(unsafe { &*link }))
        .collect();
    validate_ships(kind, &entries, race_count)?;
    Ok(entries)
}

/// Whether `SaveGame` writes "NpcQ" in a save made in `activity`.
///
/// `SaveGame` skips both queues when `START_INTERPLANETARY` is set (the
/// homeworld encounter is built anew on load) and otherwise writes "NpcQ" when
/// `START_ENCOUNTER` is set. The "IGpQ" test in `groups::saves_ip_group_queue`
/// is the `else` of the same branch.
pub fn saves_npc_ship_queue(activity: u16) -> bool {
    activity & START_INTERPLANETARY == 0 && activity & START_ENCOUNTER != 0
}

fn capture_queues_with(game: &CGameState, with_npc: bool) -> Result<QueueSections, QueueError> {
    let fleets = walk::<CFleetInfo>(&game.avail_race_q, QueueKind::AvailableRaces)?;
    let race_queue = fleets
        .iter()
        .enumerate()
        // SAFETY: `walk` returned slots of the pool, which holds `FLEET_INFO`s.
        .map(|(index, &link)| race_entry(index, unsafe { &*link }))
        .collect::<Vec<_>>();
    validate_races(&race_queue, fleets.len())?;
    let npc_ship_queue = if with_npc {
        capture_ships(&game.npc_built_ship_q, QueueKind::NpcShips, fleets.len())?
    } else {
        Vec::new()
    };
    Ok(QueueSections {
        race_queue,
        npc_ship_queue,
        ship_queue: capture_ships(&game.built_ship_q, QueueKind::BuiltShips, fleets.len())?,
    })
}

/// The three queues of `game`, head to tail, whatever the game's activity.
pub fn capture_queues(game: &CGameState) -> Result<QueueSections, QueueError> {
    capture_queues_with(game, true)
}

/// The queues a save made in the game's current activity writes: "RacQ" and
/// "ShpQ" always, "NpcQ" only when `saves_npc_ship_queue` holds. An npc queue
/// that is not written is not read, so it is empty here and is not validated.
///
/// This reads each queue on its own; it is not a snapshot of the whole game
/// taken under one lock, and nothing here makes a save atomic.
pub fn capture_queues_for_activity(game: &CGameState) -> Result<QueueSections, QueueError> {
    capture_queues_with(game, saves_npc_ship_queue(game.current_activity))
}

/// Make the live queues hold the given sections, drawing captain names from
/// the game's random stream.
pub fn restore_queues(game: &mut CGameState, sections: &QueueSections) -> Result<(), QueueError> {
    restore_queues_with(game, sections, &mut production_captain_name)
}

/// `restore_queues` with the captain-name draw supplied by the caller.
///
/// On error the queues are untouched.
pub fn restore_queues_with(
    game: &mut CGameState,
    sections: &QueueSections,
    pick_name: &mut dyn FnMut() -> u8,
) -> Result<(), QueueError> {
    let fleets = walk::<CFleetInfo>(&game.avail_race_q, QueueKind::AvailableRaces)?;
    validate_races(&sections.race_queue, fleets.len())?;
    for (queue, kind, entries) in [
        (
            &game.npc_built_ship_q,
            QueueKind::NpcShips,
            &sections.npc_ship_queue,
        ),
        (
            &game.built_ship_q,
            QueueKind::BuiltShips,
            &sections.ship_queue,
        ),
    ] {
        let capacity = pool_slots::<CShipFragment>(queue, kind)?;
        if entries.len() > capacity {
            return Err(QueueError::TooManyShips {
                queue: kind,
                count: entries.len(),
                capacity,
            });
        }
        validate_ships(kind, entries, fleets.len())?;
    }

    for entry in &sections.race_queue {
        // SAFETY: `fleets` are the live slots of `avail_race_q`, and the
        // validation above bounded `index` by their number.
        let slot = fleets[usize::from(entry.index)];
        let fleet = unsafe { &mut *slot };
        fleet.allied_state = entry.allied_state;
        fleet.days_left = entry.days_left;
        fleet.growth_fract = entry.growth_fract;
        fleet.crew_level = entry.crew_level;
        fleet.max_crew = entry.max_crew;
        fleet.growth = entry.growth;
        fleet.max_energy = entry.max_energy;
        fleet.loc = c_point(entry.loc);
        fleet.actual_strength = entry.actual_strength;
        fleet.known_strength = entry.known_strength;
        fleet.known_loc = c_point(entry.known_loc);
        fleet.growth_err_term = entry.growth_err_term;
        fleet.func_index = entry.func_index;
        fleet.dest_loc = c_point(entry.dest_loc);
    }

    let avail = std::ptr::addr_of_mut!(game.avail_race_q).cast::<Queue>();
    let npc = std::ptr::addr_of_mut!(game.npc_built_ship_q).cast::<Queue>();
    let built = std::ptr::addr_of_mut!(game.built_ship_q).cast::<Queue>();
    for (queue, entries) in [
        (npc, &sections.npc_ship_queue),
        (built, &sections.ship_queue),
    ] {
        ReinitQueue(queue);
        for entry in entries {
            // `race_index` fits a byte: it is below the number of fleets,
            // which is at most `u8::MAX`.
            //
            // SAFETY: `avail` holds `FLEET_INFO` links and `queue` is an
            // emptied pool of `SHIP_FRAGMENT` slots, both checked above.
            let link = unsafe {
                clone_ship_fragment_from(avail, entry.race_index as u8, queue, 0, pick_name)
            };
            assert!(
                !link.is_null(),
                "ship queue ran out of slots or races after validation"
            );
            // SAFETY: the clone is a `SHIP_FRAGMENT` slot of `queue`.
            let fragment = unsafe { &mut *link.cast::<CShipFragment>() };
            fragment.captains_name_index = entry.captains_name_index;
            fragment.race_id = entry.race_id;
            fragment.index = entry.index;
            fragment.crew_level = entry.crew_level;
            fragment.max_crew = entry.max_crew;
            fragment.energy_level = entry.energy_level;
            fragment.max_energy = entry.max_energy;
        }
    }
    Ok(())
}
