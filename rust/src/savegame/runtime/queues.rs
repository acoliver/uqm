//! Capture and restore of the "RacQ", "NpcQ" and "ShpQ" sections against the
//! game's live queues: `GLOBAL (avail_race_q)`, `npc_built_ship_q` and
//! `built_ship_q`.
//!
//! World owns three fixed-capacity pools. The fleet templates and their loaded
//! resources survive restore; fragment pools are replaced in save-file order.
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
//! templates. The "IGpQ" and "Enct" queues are in `groups`, which uses the same
//! typed traversal order.

use std::fmt;

use crate::comm::dispatch::{
    production_captain_name, CPoint, INFINITE_FLEET, START_ENCOUNTER, START_INTERPLANETARY,
};
use crate::engine::battle::link_queue::{LinkQueue, QueueRecord};
use crate::engine::clock::NUM_EVENTS;
use crate::engine::world::queues::{FleetInfo, Fragment, ShipFragment};
use crate::engine::World;
use crate::savegame::types::{Point, RaceEntry, ShipFragmentEntry};

/// `MAX_BUILT_SHIPS` (sis.h): hangar slots, so the bound of an escort's index.
pub const MAX_BUILT_SHIPS: u8 = 12;
/// Values of `FLEET_INFO.allied_state` (races.h): `DEAD_GUY`, `GOOD_GUY`,
/// `BAD_GUY`.
const ALLIED_STATES: u16 = 3;
/// `(BYTE) ~0`: a fleet with no function to call on arrival.
const NO_FLEET_FUNCTION: u8 = u8::MAX;

/// The queues the runtime walks: the three of this module, the two of
/// `groups`, and the ship and display queues of `hyper_save`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueKind {
    AvailableRaces,
    NpcShips,
    BuiltShips,
    IpGroups,
    Encounters,
    /// `race_q[RPG_PLAYER_NUM]`: the `STARSHIP`s of the human player's side.
    PlayerShips,
    /// `disp_q`: every live `ELEMENT`.
    DisplayList,
}

/// Why the queue sections could not be captured or restored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueError {
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

pub(super) fn point(p: CPoint) -> Point {
    Point { x: p.x, y: p.y }
}

pub(super) fn c_point(p: Point) -> CPoint {
    CPoint { x: p.x, y: p.y }
}

fn race_entry(index: usize, fleet: &FleetInfo) -> RaceEntry {
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

fn ship_entry(fragment: &ShipFragment) -> ShipFragmentEntry {
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

pub(super) fn entries<T: QueueRecord, const N: usize, R>(
    view: LinkQueue<'_, T, N>,
    mut map: impl FnMut(usize, &T::Record) -> R,
) -> Vec<R> {
    let mut values = Vec::with_capacity(view.count());
    let mut next = view.head();
    while let Some(id) = next {
        values.push(map(values.len(), view.record(id)));
        next = view.succ(id);
    }
    values
}

fn capture_ships<const N: usize>(
    queue: LinkQueue<'_, Fragment, N>,
    kind: QueueKind,
    race_count: usize,
) -> Result<Vec<ShipFragmentEntry>, QueueError> {
    let entries = entries(queue, |_, ship| ship_entry(ship));
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

fn capture_queues_with(world: &World, with_npc: bool) -> Result<QueueSections, QueueError> {
    let race_queue = entries(world.avail_races_ref(), race_entry);
    let races = race_queue.len();
    validate_races(&race_queue, races)?;
    Ok(QueueSections {
        race_queue,
        npc_ship_queue: if with_npc {
            capture_ships(world.npc_built_ships_ref(), QueueKind::NpcShips, races)?
        } else {
            Vec::new()
        },
        ship_queue: capture_ships(world.built_ships_ref(), QueueKind::BuiltShips, races)?,
    })
}

/// The three queues of `game`, head to tail, whatever the game's activity.
pub fn capture_queues(game: &World) -> Result<QueueSections, QueueError> {
    capture_queues_with(game, true)
}

/// The queues a save made in the game's current activity writes: "RacQ" and
/// "ShpQ" always, "NpcQ" only when `saves_npc_ship_queue` holds. An npc queue
/// that is not written is not read, so it is empty here and is not validated.
///
/// This reads each queue on its own; it is not a snapshot of the whole game
/// taken under one lock, and nothing here makes a save atomic.
pub fn capture_queues_for_activity(game: &World) -> Result<QueueSections, QueueError> {
    capture_queues_with(game, saves_npc_ship_queue(game.activity()))
}

/// Make the live queues hold the given sections, drawing captain names from
/// the game's random stream.
pub fn restore_queues(game: &mut World, sections: &QueueSections) -> Result<(), QueueError> {
    restore_queues_with(game, sections, &mut production_captain_name)
}

/// `restore_queues` with the captain-name draw supplied by the caller.
///
/// On error the queues are untouched.
pub fn restore_queues_with(
    game: &mut World,
    sections: &QueueSections,
    pick_name: &mut dyn FnMut() -> u8,
) -> Result<(), QueueError> {
    let races = game.avail_races_ref().count();
    validate_races(&sections.race_queue, races)?;
    for (capacity, kind, entries) in [
        (14, QueueKind::NpcShips, &sections.npc_ship_queue),
        (12, QueueKind::BuiltShips, &sections.ship_queue),
    ] {
        if entries.len() > capacity {
            return Err(QueueError::TooManyShips {
                queue: kind,
                count: entries.len(),
                capacity,
            });
        }
        validate_ships(kind, entries, races)?;
    }

    for entry in &sections.race_queue {
        let id = game
            .avail_races_ref()
            .at(usize::from(entry.index))
            .unwrap_or_else(|| panic!("validated fleet index"));
        let mut fleets = game.avail_races();
        let fleet = fleets.get_mut(id);
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

    let templates = entries(game.avail_races_ref(), |_, fleet| *fleet);
    restore_ships(
        game.npc_built_ships(),
        &sections.npc_ship_queue,
        &templates,
        pick_name,
    );
    restore_ships(
        game.built_ships(),
        &sections.ship_queue,
        &templates,
        pick_name,
    );
    Ok(())
}

fn restore_ships<const N: usize>(
    mut queue: LinkQueue<'_, Fragment, N>,
    entries: &[ShipFragmentEntry],
    templates: &[FleetInfo],
    pick_name: &mut dyn FnMut() -> u8,
) {
    queue.reinit();
    for entry in entries {
        let template = &templates[usize::from(entry.race_index)];
        let id = queue
            .clone_fragment(template, entry.race_index as u8, 0, &mut *pick_name)
            .unwrap_or_else(|| panic!("ship capacity validated before installation"));
        let fragment = queue.get_mut(id);
        fragment.captains_name_index = entry.captains_name_index;
        fragment.race_id = entry.race_id;
        fragment.index = entry.index;
        fragment.crew_level = entry.crew_level;
        fragment.max_crew = entry.max_crew;
        fragment.energy_level = entry.energy_level;
        fragment.max_energy = entry.max_energy;
    }
}
