//! Tests for the live "RacQ", "NpcQ" and "ShpQ" capture and restore.
//!
//! The queues are real `QUEUE_TABLE` pools made by `InitQueue`, sized as
//! `InitGameStructures` sizes them (24 `FLEET_INFO`, 14 and 12 `SHIP_FRAGMENT`),
//! and the fleet templates carry resource handles that are never dereferenced
//! but must come through a restore unchanged. The expected values are
//! written out here from the bytes of a save the C game wrote
//! (`C_WRITTEN_SAVE`, read with a separate script) and from `races.h`, not
//! from the encoder.

use std::mem::{offset_of, size_of};
use std::ops::{Deref, DerefMut};

use super::groups::CIpGroup;
use super::queues::*;
use crate::collections::queue::{AllocLink, HLink, InitQueue, PutQueue, Queue, UninitQueue};
use crate::comm::dispatch::{CEncounter, CFleetInfo, CPoint, CShipFragment, INFINITE_FLEET};
use crate::comm::locdata::{CGameState, CQueue};
use crate::savegame::decode::decode_save;
use crate::savegame::types::{Point, RaceEntry, ShipFragmentEntry};
use crate::time::clock_bridge::NUM_EVENTS;

/// Tracked evidence from a real run of the C game: epic 20 load/save proof.
pub(super) const C_WRITTEN_SAVE: &[u8] =
    include_bytes!("../../../tests/fixtures/savegame/legacy-save.00");

/// `NUM_AVAILABLE_RACES` (races.h): the fleets `InitGameStructures` queues.
pub(super) const RACES: usize = 24;
const NPC_SLOTS: u16 = 14;
const BUILT_SLOTS: u16 = 12;

/// `SAMATRA_SHIP` is `URQUAN_DRONE_SHIP`.
const URQUAN_DRONE_SHIP: u16 = 23;
const HUMAN_SHIP: u16 = 2;

// ---------------------------------------------------------------------------
// A live GAME_STATE with initialised queues
// ---------------------------------------------------------------------------

/// A `GAME_STATE` whose three queues were made by `InitQueue`, with a template
/// in `avail_race_q` for every race.
pub(super) struct LiveGame(Box<CGameState>);

/// Resource handles of the template of `race`; never dereferenced.
pub(super) fn handle(race: usize, which: usize) -> *mut std::ffi::c_void {
    (0x1_0000 + race * 0x100 + which * 0x10) as *mut std::ffi::c_void
}

impl LiveGame {
    pub(super) fn new() -> Self {
        let mut game = Self::bare();
        for race in 0..RACES {
            let queue = game.avail_ptr();
            let link = AllocLink(queue);
            assert!(!link.is_null());
            // SAFETY: `link` is a free slot of the `FLEET_INFO` pool.
            unsafe {
                link.cast::<CFleetInfo>().write(CFleetInfo {
                    pred: std::ptr::null_mut(),
                    succ: std::ptr::null_mut(),
                    species_id: race as i32 + 1,
                    // Values no save holds, so a restore that skips a field
                    // shows.
                    allied_state: 2,
                    days_left: 0xA1,
                    growth_fract: 0xA2,
                    crew_level: 100 + race as u16,
                    max_crew: 200 + race as u16,
                    growth: 0xA3,
                    max_energy: 30 + race as u8,
                    loc: CPoint { x: -1, y: -2 },
                    race_strings: handle(race, 1),
                    icons: handle(race, 2),
                    melee_icon: handle(race, 3),
                    actual_strength: 0xA4A4,
                    known_strength: 0xA5A5,
                    known_loc: CPoint { x: -3, y: -4 },
                    growth_err_term: 0xA6,
                    func_index: 3,
                    dest_loc: CPoint { x: -5, y: -6 },
                });
            }
            PutQueue(queue, link);
        }
        game
    }

    /// The three pools with nothing queued in `avail_race_q`.
    pub(super) fn bare() -> Self {
        // SAFETY: `CGameState` is integers and raw pointers; all zeroes is a
        // valid value, and it is what C's static storage starts as.
        let mut game: Box<CGameState> = Box::new(unsafe { std::mem::zeroed() });
        assert!(InitQueue(
            std::ptr::addr_of_mut!(game.avail_race_q).cast::<Queue>(),
            RACES as u16,
            size_of::<CFleetInfo>() as u16
        ));
        assert!(InitQueue(
            std::ptr::addr_of_mut!(game.npc_built_ship_q).cast::<Queue>(),
            NPC_SLOTS,
            size_of::<CShipFragment>() as u16
        ));
        assert!(InitQueue(
            std::ptr::addr_of_mut!(game.built_ship_q).cast::<Queue>(),
            BUILT_SLOTS,
            size_of::<CShipFragment>() as u16
        ));
        // `MAX_BATTLE_GROUPS` and `MAX_ENCOUNTERS`, as `InitGameStructures` sizes them.
        assert!(InitQueue(
            std::ptr::addr_of_mut!(game.ip_group_q).cast::<Queue>(),
            32,
            size_of::<CIpGroup>() as u16
        ));
        assert!(InitQueue(
            std::ptr::addr_of_mut!(game.encounter_q).cast::<Queue>(),
            16,
            size_of::<CEncounter>() as u16
        ));
        Self(game)
    }

    pub(super) fn avail_ptr(&mut self) -> *mut Queue {
        std::ptr::addr_of_mut!(self.0.avail_race_q).cast::<Queue>()
    }

    pub(super) fn template(&self, race: usize) -> &CFleetInfo {
        let fleets: Vec<*mut CFleetInfo> = links(&self.avail_race_q);
        // SAFETY: the links are slots of the `FLEET_INFO` pool.
        unsafe { &*fleets[race] }
    }

    fn built_fragments(&self) -> Vec<&CShipFragment> {
        fragments(&self.built_ship_q)
    }

    fn npc_fragments(&self) -> Vec<&CShipFragment> {
        fragments(&self.npc_built_ship_q)
    }

    /// Append a fragment straight to `queue`, the way a test of the game's own
    /// ship building would, without the code under test.
    pub(super) fn push_fragment(&mut self, npc: bool, race: u8, captain: u8) {
        let queue = if npc {
            std::ptr::addr_of_mut!(self.0.npc_built_ship_q).cast::<Queue>()
        } else {
            std::ptr::addr_of_mut!(self.0.built_ship_q).cast::<Queue>()
        };
        let link = AllocLink(queue);
        assert!(!link.is_null());
        // SAFETY: `link` is a free slot of a `SHIP_FRAGMENT` pool.
        unsafe {
            link.cast::<CShipFragment>().write(CShipFragment {
                race_id: race,
                captains_name_index: captain,
                crew_level: 1,
                max_crew: 1,
                ..CShipFragment::default()
            });
        }
        PutQueue(queue, link);
    }
}

impl Drop for LiveGame {
    fn drop(&mut self) {
        for queue in [
            std::ptr::addr_of_mut!(self.0.avail_race_q),
            std::ptr::addr_of_mut!(self.0.npc_built_ship_q),
            std::ptr::addr_of_mut!(self.0.built_ship_q),
            std::ptr::addr_of_mut!(self.0.ip_group_q),
            std::ptr::addr_of_mut!(self.0.encounter_q),
        ] {
            UninitQueue(queue.cast::<Queue>());
        }
    }
}

impl Deref for LiveGame {
    type Target = CGameState;
    fn deref(&self) -> &CGameState {
        &self.0
    }
}

impl DerefMut for LiveGame {
    fn deref_mut(&mut self) -> &mut CGameState {
        &mut self.0
    }
}

/// The links of a queue by following the raw `succ` pointers, independent of
/// the walker under test.
pub(super) fn links<T>(queue: &CQueue) -> Vec<*mut T> {
    let mut out = Vec::new();
    let mut link = queue.head;
    while !link.is_null() {
        out.push(link.cast::<T>());
        // SAFETY: every link starts with `pred` and `succ`; the queues in
        // these tests are well formed.
        link = unsafe { (*link.cast::<crate::collections::queue::Link>()).succ };
    }
    out
}

fn fragments(queue: &CQueue) -> Vec<&CShipFragment> {
    links::<CShipFragment>(queue)
        .into_iter()
        // SAFETY: the links are slots of a `SHIP_FRAGMENT` pool.
        .map(|link| unsafe { &*link })
        .collect()
}

/// Slots on the free list of `queue`.
pub(super) fn free_slots(queue: &CQueue) -> usize {
    let mut count = 0;
    let mut link: HLink = queue.free_list;
    while !link.is_null() {
        count += 1;
        // SAFETY: free slots are linked through `succ`.
        link = unsafe { (*link.cast::<crate::collections::queue::Link>()).succ };
    }
    count
}

/// A counter standing in for the random captain-name draw.
fn names() -> (std::rc::Rc<std::cell::Cell<u32>>, impl FnMut() -> u8) {
    let draws = std::rc::Rc::new(std::cell::Cell::new(0u32));
    let seen = draws.clone();
    (draws, move || {
        seen.set(seen.get() + 1);
        // Names 5..=20 are the ones the game draws.
        5 + (seen.get() % 16) as u8
    })
}

fn p(x: i16, y: i16) -> Point {
    Point { x, y }
}

fn seed_sections() -> QueueSections {
    let save = decode_save(C_WRITTEN_SAVE).unwrap();
    QueueSections {
        race_queue: save.race_queue,
        npc_ship_queue: save.npc_ship_queue,
        ship_queue: save.ship_queue,
    }
}

fn human_entry(captain: u8, crew: u16) -> ShipFragmentEntry {
    ShipFragmentEntry {
        race_index: HUMAN_SHIP,
        captains_name_index: captain,
        race_id: HUMAN_SHIP as u8,
        index: 0,
        crew_level: crew,
        max_crew: 18,
        energy_level: 0,
        max_energy: 18,
    }
}

fn empty_sections() -> QueueSections {
    QueueSections {
        race_queue: Vec::new(),
        npc_ship_queue: Vec::new(),
        ship_queue: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

/// Offsets worked out by hand from `races.h` on LP64: two 8-byte links, a
/// 4-byte `SPECIES_ID`, then the fields in declaration order.
#[test]
fn queue_elements_have_the_lp64_layout_races_h_gives_them() {
    assert_eq!(size_of::<CFleetInfo>(), 80);
    assert_eq!(offset_of!(CFleetInfo, species_id), 16);
    assert_eq!(offset_of!(CFleetInfo, allied_state), 20);
    assert_eq!(offset_of!(CFleetInfo, crew_level), 24);
    assert_eq!(offset_of!(CFleetInfo, loc), 30);
    assert_eq!(offset_of!(CFleetInfo, race_strings), 40);
    assert_eq!(offset_of!(CFleetInfo, actual_strength), 64);
    assert_eq!(offset_of!(CFleetInfo, func_index), 73);
    assert_eq!(offset_of!(CFleetInfo, dest_loc), 74);
    assert_eq!(size_of::<CShipFragment>(), 56);
    assert_eq!(offset_of!(CShipFragment, captains_name_index), 20);
    assert_eq!(offset_of!(CShipFragment, race_id), 21);
    assert_eq!(offset_of!(CShipFragment, index), 22);
    assert_eq!(offset_of!(CShipFragment, crew_level), 24);
    assert_eq!(offset_of!(CShipFragment, max_energy), 29);
    assert_eq!(offset_of!(CShipFragment, race_strings), 32);
}

// ---------------------------------------------------------------------------
// Capture
// ---------------------------------------------------------------------------

/// Write the seed's first fleet and its escort into a live game with plain
/// stores, then check what capture reads against the wire bytes the C game
/// wrote.
#[test]
fn capture_reads_the_values_the_c_game_wrote() {
    let mut game = LiveGame::new();
    // RacQ entry 0 of the seed: 00 00 | 02 00 | 00 | 00 | 06 00 | 06 00 | 00 |
    // 14 | b6 01 e4 18 | 2c 00 | 00 00 | b6 01 e4 18 | 7f | ff | d2 c2 00 00
    let fleet0 = links::<CFleetInfo>(&game.avail_race_q)[0];
    // SAFETY: the links are slots of the `FLEET_INFO` pool.
    unsafe {
        let f = &mut *fleet0;
        f.allied_state = 2;
        f.days_left = 0;
        f.growth_fract = 0;
        f.crew_level = 6;
        f.max_crew = 6;
        f.growth = 0;
        f.max_energy = 0x14;
        f.loc = CPoint { x: 438, y: 6372 };
        f.actual_strength = 0x2c;
        f.known_strength = 0;
        f.known_loc = CPoint { x: 438, y: 6372 };
        f.growth_err_term = 0x7f;
        f.func_index = 0xff;
        f.dest_loc = CPoint {
            x: 0xc2d2u16 as i16,
            y: 0,
        };
    }
    game.push_fragment(false, 2, 15);
    {
        let escort = links::<CShipFragment>(&game.built_ship_q)[0];
        // ShpQ of the seed: 02 00 | 0f | 02 | 00 | 12 00 | 12 00 | 00 | 12
        // SAFETY: slot of the `SHIP_FRAGMENT` pool.
        let f = unsafe { &mut *escort };
        f.crew_level = 0x12;
        f.max_crew = 0x12;
        f.max_energy = 0x12;
    }

    let captured = capture_queues(&game).unwrap();

    assert_eq!(captured.race_queue.len(), RACES);
    assert_eq!(
        captured.race_queue[0],
        RaceEntry {
            index: 0,
            allied_state: 2,
            days_left: 0,
            growth_fract: 0,
            crew_level: 6,
            max_crew: 6,
            growth: 0,
            max_energy: 20,
            loc: p(438, 6372),
            actual_strength: 44,
            known_strength: 0,
            known_loc: p(438, 6372),
            growth_err_term: 127,
            func_index: 255,
            dest_loc: p(-15662, 0),
        }
    );
    // Position, not species, is the index.
    assert_eq!(captured.race_queue[23].index, 23);
    assert!(captured.npc_ship_queue.is_empty());
    assert_eq!(
        captured.ship_queue,
        vec![ShipFragmentEntry {
            race_index: 2,
            captains_name_index: 15,
            race_id: 2,
            index: 0,
            crew_level: 18,
            max_crew: 18,
            energy_level: 0,
            max_energy: 18,
        }]
    );
    // The same ship as the codec reads it from the C-written file.
    assert_eq!(captured.ship_queue, seed_sections().ship_queue);
}

#[test]
fn capture_of_the_templates_as_initialised_has_the_queue_positions_as_indices() {
    let game = LiveGame::new();
    let captured = capture_queues(&game).unwrap();
    let indices: Vec<u16> = captured.race_queue.iter().map(|e| e.index).collect();
    assert_eq!(indices, (0..RACES as u16).collect::<Vec<_>>());
    assert_eq!(captured.race_queue[7].crew_level, 107);
    assert_eq!(captured.race_queue[7].max_crew, 207);
    assert!(captured.ship_queue.is_empty());
}

#[test]
fn capture_follows_queue_order_not_pool_order() {
    let mut game = LiveGame::new();
    game.push_fragment(true, 5, 6);
    game.push_fragment(true, 3, 7);
    game.push_fragment(true, 5, 8);
    let captured = capture_queues(&game).unwrap();
    let races: Vec<u8> = captured.npc_ship_queue.iter().map(|e| e.race_id).collect();
    let captains: Vec<u8> = captured
        .npc_ship_queue
        .iter()
        .map(|e| e.captains_name_index)
        .collect();
    assert_eq!(races, [5, 3, 5]);
    assert_eq!(captains, [6, 7, 8]);
}

#[test]
fn capture_rejects_a_queue_whose_links_disagree() {
    let mut game = LiveGame::new();
    game.push_fragment(false, 2, 6);
    game.push_fragment(false, 3, 7);
    let q = links::<CShipFragment>(&game.built_ship_q);
    let (first, second) = (q[0], q[1]);
    // A broken back-link.
    // SAFETY: slots of the `SHIP_FRAGMENT` pool.
    unsafe { (*second).pred = std::ptr::null_mut() };
    assert_eq!(
        capture_queues(&game).unwrap_err(),
        QueueError::CorruptQueue {
            queue: QueueKind::BuiltShips,
            position: 1
        }
    );
    // SAFETY: as above.
    unsafe { (*second).pred = first.cast() };

    // A loop: the tail points back at the head.
    // SAFETY: as above.
    unsafe { (*second).succ = first.cast() };
    assert!(matches!(
        capture_queues(&game).unwrap_err(),
        QueueError::CorruptQueue {
            queue: QueueKind::BuiltShips,
            ..
        }
    ));
    // SAFETY: as above.
    unsafe { (*second).succ = std::ptr::null_mut() };

    // A link outside the pool.
    let mut outside = CShipFragment::default();
    // SAFETY: as above.
    unsafe { (*second).succ = std::ptr::addr_of_mut!(outside).cast() };
    assert!(matches!(
        capture_queues(&game).unwrap_err(),
        QueueError::CorruptQueue {
            queue: QueueKind::BuiltShips,
            position: 2
        }
    ));
    // SAFETY: as above.
    unsafe { (*second).succ = std::ptr::null_mut() };

    // A tail that is not the last link.
    game.built_ship_q.tail = first.cast();
    assert!(matches!(
        capture_queues(&game).unwrap_err(),
        QueueError::CorruptQueue { .. }
    ));
}

#[test]
fn capture_refuses_uninitialised_or_foreign_pools() {
    // SAFETY: all zeroes is a valid `CGameState`.
    let game: Box<CGameState> = Box::new(unsafe { std::mem::zeroed() });
    assert_eq!(
        capture_queues(&game).unwrap_err(),
        QueueError::PoolMissing {
            queue: QueueKind::AvailableRaces
        }
    );

    let mut game = LiveGame::new();
    game.built_ship_q.object_size = size_of::<CFleetInfo>() as u16;
    assert_eq!(
        capture_queues(&game).unwrap_err(),
        QueueError::WrongObjectSize {
            queue: QueueKind::BuiltShips,
            found: size_of::<CFleetInfo>() as u16,
            expected: size_of::<CShipFragment>()
        }
    );
}

#[test]
fn capture_refuses_a_ship_of_a_race_without_a_template() {
    let mut game = LiveGame::new();
    game.push_fragment(false, RACES as u8, 6);
    assert_eq!(
        capture_queues(&game).unwrap_err(),
        QueueError::UnknownRace {
            queue: QueueKind::BuiltShips,
            position: 0,
            race: RACES as u16
        }
    );
}

// ---------------------------------------------------------------------------
// Restore
// ---------------------------------------------------------------------------

#[test]
fn restoring_the_seed_overwrites_only_the_persisted_fleet_fields() {
    let mut game = LiveGame::new();
    let (draws, mut pick) = names();
    restore_queues_with(&mut game, &seed_sections(), &mut pick).unwrap();

    // Fleet 0 from the C-written bytes (see the capture test).
    let f = game.template(0);
    assert_eq!(f.allied_state, 2);
    assert_eq!((f.days_left, f.growth_fract), (0, 0));
    assert_eq!((f.crew_level, f.max_crew), (6, 6));
    assert_eq!((f.growth, f.max_energy), (0, 0x14));
    assert_eq!((f.loc.x, f.loc.y), (438, 6372));
    assert_eq!((f.actual_strength, f.known_strength), (0x2c, 0));
    assert_eq!((f.known_loc.x, f.known_loc.y), (438, 6372));
    assert_eq!((f.growth_err_term, f.func_index), (0x7f, 0xff));
    assert_eq!((f.dest_loc.x, f.dest_loc.y), (-15662, 0));
    // Fleet 2 is allied in the seed (02 00 | 01 00 ...).
    assert_eq!(game.template(2).allied_state, 1);
    assert_eq!(
        (game.template(23).dest_loc.x, game.template(23).dest_loc.y),
        (-32126, -31870)
    );

    // What the save does not hold is the template's own.
    for race in 0..RACES {
        let t = game.template(race);
        assert_eq!(t.species_id, race as i32 + 1, "race {race}");
        assert_eq!(t.race_strings, handle(race, 1), "race {race}");
        assert_eq!(t.icons, handle(race, 2), "race {race}");
        assert_eq!(t.melee_icon, handle(race, 3), "race {race}");
    }
    assert_eq!(links::<CFleetInfo>(&game.avail_race_q).len(), RACES);
    assert_eq!(free_slots(&game.avail_race_q), 0);

    // The one escort of the seed: a clone of the Human template with the
    // saved fields on top.
    let built = game.built_fragments();
    assert_eq!(built.len(), 1);
    let ship = built[0];
    assert_eq!(ship.captains_name_index, 15);
    assert_eq!((ship.race_id, ship.index), (2, 0));
    assert_eq!((ship.crew_level, ship.max_crew), (18, 18));
    assert_eq!((ship.energy_level, ship.max_energy), (0, 18));
    assert_eq!(ship.species_id, 3);
    assert_eq!(ship.race_strings, handle(2, 1));
    assert_eq!(ship.icons, handle(2, 2));
    assert_eq!(ship.melee_icon, handle(2, 3));
    // One captain draw for the clone, whatever name the save then put on top.
    assert_eq!(draws.get(), 1);
    assert!(game.npc_fragments().is_empty());
    assert_eq!(free_slots(&game.built_ship_q), BUILT_SLOTS as usize - 1);
    assert_eq!(free_slots(&game.npc_built_ship_q), NPC_SLOTS as usize);
}

#[test]
fn restore_keeps_entry_order_and_replaces_what_was_queued() {
    let mut game = LiveGame::new();
    game.push_fragment(false, 9, 6);
    game.push_fragment(true, 9, 6);
    let sections = QueueSections {
        npc_ship_queue: vec![ShipFragmentEntry {
            crew_level: INFINITE_FLEET,
            ..human_entry(9, 1)
        }],
        ship_queue: vec![
            ShipFragmentEntry {
                race_index: 5,
                race_id: 5,
                index: 3,
                max_energy: 9,
                ..human_entry(20, 7)
            },
            human_entry(6, 18),
            ShipFragmentEntry {
                race_index: 5,
                race_id: 5,
                index: 11,
                ..human_entry(7, 2)
            },
        ],
        ..empty_sections()
    };
    let (_, mut pick) = names();
    restore_queues_with(&mut game, &sections, &mut pick).unwrap();

    let built: Vec<(u8, u8, u8, u16)> = game
        .built_fragments()
        .iter()
        .map(|f| (f.race_id, f.captains_name_index, f.index, f.crew_level))
        .collect();
    assert_eq!(built, [(5, 20, 3, 7), (2, 6, 0, 18), (5, 7, 11, 2)]);
    let npc = game.npc_fragments();
    assert_eq!(npc.len(), 1);
    assert_eq!(npc[0].crew_level, INFINITE_FLEET);
    assert_eq!(free_slots(&game.built_ship_q), BUILT_SLOTS as usize - 3);
    assert_eq!(free_slots(&game.npc_built_ship_q), NPC_SLOTS as usize - 1);
    // The templates of races 2 and 5 were the source of each clone.
    assert_eq!(game.built_fragments()[0].species_id, 6);
    assert_eq!(game.built_fragments()[1].race_strings, handle(2, 1));
    // The saved sections read back.
    assert_eq!(
        capture_queues(&game).unwrap().ship_queue,
        sections.ship_queue
    );
}

/// `LoadGame` empties both ship queues before any chunk; a save without
/// "ShpQ" or "NpcQ" leaves them empty.
#[test]
fn a_save_without_ship_chunks_empties_the_ship_queues() {
    let mut game = LiveGame::new();
    game.push_fragment(false, 2, 6);
    game.push_fragment(true, 2, 6);
    let (_, mut pick) = names();
    restore_queues_with(&mut game, &empty_sections(), &mut pick).unwrap();
    assert!(game.built_fragments().is_empty());
    assert!(game.npc_fragments().is_empty());
    assert_eq!(free_slots(&game.built_ship_q), BUILT_SLOTS as usize);
    assert_eq!(free_slots(&game.npc_built_ship_q), NPC_SLOTS as usize);
}

/// `CloneShipFragment` names no captain for the Sa-Matra (`SAMATRA_SHIP ==
/// URQUAN_DRONE_SHIP`) and so draws no random number for it.
#[test]
fn the_drone_clone_draws_no_captain_name_and_every_other_race_one() {
    let mut game = LiveGame::new();
    let drone = ShipFragmentEntry {
        race_index: URQUAN_DRONE_SHIP,
        race_id: URQUAN_DRONE_SHIP as u8,
        ..human_entry(0, 1)
    };
    let sections = QueueSections {
        npc_ship_queue: vec![drone.clone(), human_entry(9, 2), drone],
        ..empty_sections()
    };
    let (draws, mut pick) = names();
    restore_queues_with(&mut game, &sections, &mut pick).unwrap();
    assert_eq!(draws.get(), 1);
    assert_eq!(game.npc_fragments().len(), 3);
}

#[test]
fn loading_twice_gives_the_same_queues_and_leaks_no_slots() {
    let mut game = LiveGame::new();
    let sections = seed_sections();
    for _ in 0..3 {
        let (_, mut pick) = names();
        restore_queues_with(&mut game, &sections, &mut pick).unwrap();
        assert_eq!(game.built_fragments().len(), 1);
        assert_eq!(free_slots(&game.built_ship_q), BUILT_SLOTS as usize - 1);
        assert_eq!(free_slots(&game.avail_race_q), 0);
    }
    assert_eq!(
        capture_queues(&game).unwrap().race_queue,
        sections.race_queue
    );
}

#[test]
fn restore_of_a_partial_fleet_list_leaves_the_other_fleets() {
    let mut game = LiveGame::new();
    let mut sections = seed_sections();
    sections.race_queue.retain(|e| e.index == 4);
    sections.race_queue[0].crew_level = 77;
    sections.ship_queue.clear();
    let (_, mut pick) = names();
    restore_queues_with(&mut game, &sections, &mut pick).unwrap();
    assert_eq!(game.template(4).crew_level, 77);
    assert_eq!(game.template(5).crew_level, 105);
    assert_eq!(game.template(5).allied_state, 2);
}

fn assert_rejected_without_change(game: &mut LiveGame, sections: &QueueSections) -> QueueError {
    let before = capture_queues(game).unwrap();
    let heads = (
        game.avail_race_q.head,
        game.npc_built_ship_q.head,
        game.built_ship_q.head,
        game.npc_built_ship_q.free_list,
        game.built_ship_q.free_list,
    );
    let (draws, mut pick) = names();
    let error = restore_queues_with(game, sections, &mut pick).unwrap_err();
    assert_eq!(draws.get(), 0, "no captain drawn for a rejected save");
    assert_eq!(capture_queues(game).unwrap(), before);
    assert_eq!(
        heads,
        (
            game.avail_race_q.head,
            game.npc_built_ship_q.head,
            game.built_ship_q.head,
            game.npc_built_ship_q.free_list,
            game.built_ship_q.free_list,
        )
    );
    error
}

#[test]
fn a_rejected_save_leaves_all_three_queues_as_they_were() {
    let mut game = LiveGame::new();
    game.push_fragment(false, 2, 6);
    game.push_fragment(true, 4, 8);

    // A valid first fleet entry and ship must not be applied when a later
    // entry is bad.
    let good_race = seed_sections().race_queue[0].clone();
    let bad_ship = ShipFragmentEntry {
        race_index: RACES as u16,
        ..human_entry(6, 1)
    };
    let sections = QueueSections {
        race_queue: vec![RaceEntry {
            crew_level: 1,
            ..good_race.clone()
        }],
        ship_queue: vec![human_entry(6, 1), bad_ship],
        ..empty_sections()
    };
    assert_eq!(
        assert_rejected_without_change(&mut game, &sections),
        QueueError::UnknownRace {
            queue: QueueKind::BuiltShips,
            position: 1,
            race: RACES as u16
        }
    );

    let cases: Vec<(QueueSections, QueueError)> = vec![
        (
            QueueSections {
                race_queue: vec![RaceEntry {
                    index: RACES as u16,
                    ..good_race.clone()
                }],
                ..empty_sections()
            },
            QueueError::UnknownRace {
                queue: QueueKind::AvailableRaces,
                position: 0,
                race: RACES as u16,
            },
        ),
        (
            QueueSections {
                race_queue: vec![good_race.clone(), good_race.clone()],
                ..empty_sections()
            },
            QueueError::DuplicateFleet {
                position: 1,
                index: 0,
            },
        ),
        (
            QueueSections {
                race_queue: vec![RaceEntry {
                    allied_state: 3,
                    ..good_race.clone()
                }],
                ..empty_sections()
            },
            QueueError::UnknownAlliedState {
                position: 0,
                allied_state: 3,
            },
        ),
        (
            QueueSections {
                race_queue: vec![RaceEntry {
                    func_index: NUM_EVENTS as u8,
                    ..good_race.clone()
                }],
                ..empty_sections()
            },
            QueueError::UnknownFleetFunction {
                position: 0,
                func_index: NUM_EVENTS as u8,
            },
        ),
        (
            QueueSections {
                ship_queue: vec![ShipFragmentEntry {
                    race_id: 3,
                    ..human_entry(6, 1)
                }],
                ..empty_sections()
            },
            QueueError::RaceIdMismatch {
                queue: QueueKind::BuiltShips,
                position: 0,
                race_index: HUMAN_SHIP,
                race_id: 3,
            },
        ),
        (
            QueueSections {
                ship_queue: vec![human_entry(6, 19)],
                ..empty_sections()
            },
            QueueError::CrewExceedsMax {
                queue: QueueKind::BuiltShips,
                position: 0,
                crew_level: 19,
                max_crew: 18,
            },
        ),
        (
            // Unlimited crew is an npc fleet's alone.
            QueueSections {
                ship_queue: vec![human_entry(6, INFINITE_FLEET)],
                ..empty_sections()
            },
            QueueError::CrewExceedsMax {
                queue: QueueKind::BuiltShips,
                position: 0,
                crew_level: INFINITE_FLEET,
                max_crew: 18,
            },
        ),
        (
            QueueSections {
                ship_queue: vec![ShipFragmentEntry {
                    index: 12,
                    ..human_entry(6, 1)
                }],
                ..empty_sections()
            },
            QueueError::EscortSlotOutOfRange {
                position: 0,
                index: 12,
            },
        ),
        (
            QueueSections {
                ship_queue: vec![human_entry(6, 1); BUILT_SLOTS as usize + 1],
                ..empty_sections()
            },
            QueueError::TooManyShips {
                queue: QueueKind::BuiltShips,
                count: BUILT_SLOTS as usize + 1,
                capacity: BUILT_SLOTS as usize,
            },
        ),
        (
            QueueSections {
                npc_ship_queue: vec![human_entry(6, 1); NPC_SLOTS as usize + 1],
                ..empty_sections()
            },
            QueueError::TooManyShips {
                queue: QueueKind::NpcShips,
                count: NPC_SLOTS as usize + 1,
                capacity: NPC_SLOTS as usize,
            },
        ),
    ];
    for (sections, expected) in cases {
        assert_eq!(
            assert_rejected_without_change(&mut game, &sections),
            expected
        );
    }
}

#[test]
fn restore_refuses_ships_when_no_templates_were_initialised() {
    // The fleet pool exists but nothing was queued: every race is unknown.
    let mut game = LiveGame::bare();
    let sections = QueueSections {
        ship_queue: vec![human_entry(6, 1)],
        ..empty_sections()
    };
    let (_, mut pick) = names();
    assert_eq!(
        restore_queues_with(&mut game, &sections, &mut pick).unwrap_err(),
        QueueError::UnknownRace {
            queue: QueueKind::BuiltShips,
            position: 0,
            race: HUMAN_SHIP
        }
    );
    assert!(game.built_fragments().is_empty());

    // An empty save is not accepted over a game that never initialised its
    // fleet pool.
    // SAFETY: all zeroes is a valid `CGameState`.
    let mut game: Box<CGameState> = Box::new(unsafe { std::mem::zeroed() });
    assert_eq!(
        restore_queues_with(&mut game, &empty_sections(), &mut pick).unwrap_err(),
        QueueError::PoolMissing {
            queue: QueueKind::AvailableRaces
        }
    );
}

#[test]
fn restore_refuses_ship_queues_that_were_never_initialised() {
    let mut game = LiveGame::new();
    let pool = game.npc_built_ship_q.pq_tab;
    game.npc_built_ship_q.pq_tab = std::ptr::null_mut();
    let (_, mut pick) = names();
    assert_eq!(
        restore_queues_with(&mut game, &empty_sections(), &mut pick).unwrap_err(),
        QueueError::PoolMissing {
            queue: QueueKind::NpcShips
        }
    );
    game.npc_built_ship_q.pq_tab = pool;
}

// ---------------------------------------------------------------------------
// Which queues a save made in an activity writes
// ---------------------------------------------------------------------------

/// `START_ENCOUNTER` and `START_INTERPLANETARY` (globdata.h): bits 2 and 3 of
/// the high byte of the activity word.
const START_ENCOUNTER_BIT: u16 = 1 << 10;
const START_INTERPLANETARY_BIT: u16 = 1 << 11;

#[test]
fn npc_queue_is_saved_exactly_when_save_game_saves_it() {
    // Every 16-bit activity: `SaveGame` writes "NpcQ" when START_INTERPLANETARY
    // is clear and START_ENCOUNTER is set, whatever the low byte says.
    for activity in 0..=u16::MAX {
        let encounter = (activity >> 10) & 1 == 1;
        let interplanetary = (activity >> 11) & 1 == 1;
        assert_eq!(
            saves_npc_ship_queue(activity),
            encounter && !interplanetary,
            "{activity:#06x}"
        );
    }
    for (activity, saved) in [
        (START_ENCOUNTER_BIT | 2, true),
        (START_ENCOUNTER_BIT | 3, true),
        (START_ENCOUNTER_BIT | 4, true),
        (START_ENCOUNTER_BIT | START_INTERPLANETARY_BIT | 4, false),
        (START_INTERPLANETARY_BIT | 4, false),
        (4, false),
        (3, false),
        (0, false),
        (START_ENCOUNTER_BIT | 0x0001 | 0x0002 | 0x4000, true),
    ] {
        assert_eq!(saves_npc_ship_queue(activity), saved, "{activity:#06x}");
    }
}

#[test]
fn npc_and_group_queues_are_never_both_written_and_the_group_one_is_the_else() {
    use super::groups::saves_ip_group_queue;
    for activity in 0..=u16::MAX {
        let npc = saves_npc_ship_queue(activity);
        let groups = saves_ip_group_queue(activity);
        assert!(!(npc && groups), "{activity:#06x}");
        // With START_INTERPLANETARY clear, the low byte decides the other case.
        if (activity >> 11) & 1 == 0 && (activity >> 10) & 1 == 0 {
            assert_eq!(groups, activity & 0xff == 4, "{activity:#06x}");
        }
    }
}

#[test]
fn capture_for_activity_writes_the_npc_queue_only_for_an_encounter() {
    let mut game = LiveGame::new();
    game.push_fragment(true, HUMAN_SHIP as u8, 3);
    game.push_fragment(true, URQUAN_DRONE_SHIP as u8, 0);
    game.push_fragment(false, HUMAN_SHIP as u8, 5);
    let raw = capture_queues(&game).unwrap();
    assert_eq!(raw.npc_ship_queue.len(), 2);

    for (activity, with_npc) in [
        (START_ENCOUNTER_BIT | 2, true),
        (START_ENCOUNTER_BIT | 3, true),
        (START_ENCOUNTER_BIT | START_INTERPLANETARY_BIT | 4, false),
        (START_INTERPLANETARY_BIT | 4, false),
        (4, false),
        (3, false),
        (2, false),
        (0, false),
    ] {
        game.current_activity = activity;
        let selected = capture_queues_for_activity(&game).unwrap();
        assert_eq!(selected.race_queue, raw.race_queue, "{activity:#06x}");
        assert_eq!(selected.ship_queue, raw.ship_queue, "{activity:#06x}");
        assert_eq!(
            selected.npc_ship_queue,
            if with_npc {
                raw.npc_ship_queue.clone()
            } else {
                Vec::new()
            },
            "{activity:#06x}"
        );
    }
}

#[test]
fn an_npc_queue_that_is_not_saved_is_not_read() {
    let mut game = LiveGame::new();
    game.push_fragment(true, HUMAN_SHIP as u8, 3);
    let head = game.npc_built_ship_q.head;
    game.npc_built_ship_q.head = (head as usize + 1) as *mut std::ffi::c_void;

    game.current_activity = 4;
    let selected = capture_queues_for_activity(&game).unwrap();
    assert!(selected.npc_ship_queue.is_empty());
    assert!(matches!(
        capture_queues(&game).unwrap_err(),
        QueueError::CorruptQueue {
            queue: QueueKind::NpcShips,
            ..
        }
    ));

    game.current_activity = START_ENCOUNTER_BIT | 2;
    assert!(matches!(
        capture_queues_for_activity(&game).unwrap_err(),
        QueueError::CorruptQueue {
            queue: QueueKind::NpcShips,
            ..
        }
    ));
    game.npc_built_ship_q.head = head;
}
