//! Tests for the live "RacQ", "NpcQ" and "ShpQ" capture and restore.
//!
//! World owns fixed-capacity pools sized as `InitGameStructures` sizes them
//! (24 fleets, 14 NPC fragments and 12 escort fragments),
//! and the fleet templates carry resource handles that are never dereferenced
//! but must come through a restore unchanged. The expected values are
//! written out here from the bytes of a save the C game wrote
//! (`C_WRITTEN_SAVE`, read with a separate script) and from `races.h`, not
//! from the encoder.

use std::ops::{Deref, DerefMut};

use super::queues::*;
use crate::engine::clock::NUM_EVENTS;

use crate::comm::dispatch::{CPoint, INFINITE_FLEET};
use crate::engine::world::queues::{FleetInfo as CFleetInfo, ShipFragment as CShipFragment};
use crate::savegame::decode::decode_save;
use crate::savegame::types::{Point, RaceEntry, ShipFragmentEntry};

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
// An owned World with initialized queues
// ---------------------------------------------------------------------------

/// An owned campaign with a template for every race.
pub(super) struct LiveGame(crate::engine::World);

pub(super) fn handle(race: usize, which: usize) -> *mut std::ffi::c_void {
    (0x1_0000 + race * 0x100 + which * 0x10) as *mut std::ffi::c_void
}
impl LiveGame {
    pub(super) fn new() -> Self {
        let mut game = Self::bare();
        for race in 0..RACES {
            let mut queue = game.avail_races();
            let id = queue.alloc().unwrap();
            *queue.get_mut(id) = crate::engine::world::queues::FleetInfo {
                species_id: race as u32 + 1,
                allied_state: 2,
                days_left: 0xa1,
                growth_fract: 0xa2,
                crew_level: 100 + race as u16,
                max_crew: 200 + race as u16,
                growth: 0xa3,
                max_energy: 30 + race as u8,
                loc: CPoint { x: -1, y: -2 },
                race_strings: handle(race, 1).cast(),
                icons: handle(race, 2).cast(),
                melee_icon: handle(race, 3).cast(),
                actual_strength: 0xa4a4,
                known_strength: 0xa5a5,
                known_loc: CPoint { x: -3, y: -4 },
                growth_err_term: 0xa6,
                func_index: 3,
                dest_loc: CPoint { x: -5, y: -6 },
            };
            queue.put(id);
        }
        game
    }
    pub(super) fn bare() -> Self {
        Self(crate::engine::World::new())
    }
    pub(super) fn template(&self, race: usize) -> CFleetInfo {
        let view = self.avail_races_ref();
        *view.get(view.at(race).unwrap())
    }
    fn built_fragments(&self) -> Vec<CShipFragment> {
        records(&self.built_ships_ref())
    }
    fn npc_fragments(&self) -> Vec<CShipFragment> {
        records(&self.npc_built_ships_ref())
    }
    pub(super) fn push_fragment(&mut self, npc: bool, race: u8, captain: u8) {
        if npc {
            push_fragment(self.npc_built_ships(), race, captain);
        } else {
            push_fragment(self.built_ships(), race, captain);
        }
    }
}
fn push_fragment<const N: usize>(
    mut queue: crate::engine::battle::link_queue::LinkQueue<
        '_,
        crate::engine::world::queues::Fragment,
        N,
    >,
    race: u8,
    captain: u8,
) {
    let id = queue.alloc().unwrap();
    *queue.get_mut(id) = crate::engine::world::queues::ShipFragment {
        race_id: race,
        captains_name_index: captain,
        crew_level: 1,
        max_crew: 1,
        ..Default::default()
    };
    queue.put(id);
}
impl Deref for LiveGame {
    type Target = crate::engine::World;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for LiveGame {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub(super) trait FixturePool {
    fn remaining(&self) -> usize;
    fn identity(&self) -> String;
}
impl<T: crate::engine::battle::link_queue::QueueRecord + std::fmt::Debug, const N: usize>
    FixturePool for crate::engine::battle::link_queue::LinkQueue<'_, T, N>
{
    fn remaining(&self) -> usize {
        N - self.count()
    }
    fn identity(&self) -> String {
        let mut ids = Vec::new();
        let mut next = self.head();
        while let Some(id) = next {
            ids.push(id);
            next = self.succ(id);
        }
        format!("{ids:?}")
    }
}
pub(super) fn records<T: crate::engine::battle::link_queue::QueueRecord, const N: usize>(
    queue: &crate::engine::battle::link_queue::LinkQueue<'_, T, N>,
) -> Vec<T::Record>
where
    T::Record: Copy,
{
    let mut values = Vec::new();
    let mut next = queue.head();
    while let Some(id) = next {
        values.push(*queue.record(id));
        next = queue.succ(id);
    }
    values
}
pub(super) fn free_slots(queue: &impl FixturePool) -> usize {
    queue.remaining()
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
    let fleet0 = game.avail_races_ref().head().unwrap();
    {
        let mut fleets = game.avail_races();
        let f = fleets.get_mut(fleet0);
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
        let escort = game.built_ships_ref().head().unwrap();
        // ShpQ of the seed: 02 00 | 0f | 02 | 00 | 12 00 | 12 00 | 00 | 12
        let mut ships = game.built_ships();
        let f = ships.get_mut(escort);
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
fn fragment_ids_are_pool_specific_and_reset_invalidates_them() {
    let mut world = crate::engine::World::new();
    let id = world.built_ships().alloc().unwrap();
    world.built_ships().put(id);
    assert!(!world.npc_built_ships_ref().contains(id));
    world.built_ships().reinit();
    let replacement = world.built_ships().alloc().unwrap();
    assert_ne!(id, replacement);
    assert!(!world.built_ships_ref().contains(id));
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
        assert_eq!(t.species_id, race as u32 + 1, "race {race}");
        assert_eq!(t.race_strings, handle(race, 1).cast(), "race {race}");
        assert_eq!(t.icons, handle(race, 2).cast(), "race {race}");
        assert_eq!(t.melee_icon, handle(race, 3).cast(), "race {race}");
    }
    assert_eq!(game.avail_races_ref().count(), RACES);
    assert_eq!(free_slots(&game.avail_races_ref()), 0);

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
    assert_eq!(ship.race_strings, handle(2, 1).cast());
    assert_eq!(ship.icons, handle(2, 2).cast());
    assert_eq!(ship.melee_icon, handle(2, 3).cast());
    // One captain draw for the clone, whatever name the save then put on top.
    assert_eq!(draws.get(), 1);
    assert!(game.npc_fragments().is_empty());
    assert_eq!(
        free_slots(&game.built_ships_ref()),
        BUILT_SLOTS as usize - 1
    );
    assert_eq!(free_slots(&game.npc_built_ships_ref()), NPC_SLOTS as usize);
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
    assert_eq!(
        free_slots(&game.built_ships_ref()),
        BUILT_SLOTS as usize - 3
    );
    assert_eq!(
        free_slots(&game.npc_built_ships_ref()),
        NPC_SLOTS as usize - 1
    );
    // The templates of races 2 and 5 were the source of each clone.
    assert_eq!(game.built_fragments()[0].species_id, 6);
    assert_eq!(game.built_fragments()[1].race_strings, handle(2, 1).cast());
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
    assert_eq!(free_slots(&game.built_ships_ref()), BUILT_SLOTS as usize);
    assert_eq!(free_slots(&game.npc_built_ships_ref()), NPC_SLOTS as usize);
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
        assert_eq!(
            free_slots(&game.built_ships_ref()),
            BUILT_SLOTS as usize - 1
        );
        assert_eq!(free_slots(&game.avail_races_ref()), 0);
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
        game.avail_races_ref().identity(),
        game.npc_built_ships_ref().identity(),
        game.built_ships_ref().identity(),
    );
    let (draws, mut pick) = names();
    let error = restore_queues_with(game, sections, &mut pick).unwrap_err();
    assert_eq!(draws.get(), 0, "no captain drawn for a rejected save");
    assert_eq!(capture_queues(game).unwrap(), before);
    assert_eq!(
        heads,
        (
            game.avail_races_ref().identity(),
            game.npc_built_ships_ref().identity(),
            game.built_ships_ref().identity()
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
        game.set_activity(activity);
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
    let id = game.npc_built_ships_ref().head().unwrap();
    game.npc_built_ships().get_mut(id).race_id = 255;
    game.set_activity(4);
    assert!(capture_queues_for_activity(&game)
        .unwrap()
        .npc_ship_queue
        .is_empty());
    assert!(matches!(
        capture_queues(&game),
        Err(QueueError::UnknownRace {
            queue: QueueKind::NpcShips,
            ..
        })
    ));
    game.set_activity(START_ENCOUNTER_BIT | 2);
    assert!(matches!(
        capture_queues_for_activity(&game),
        Err(QueueError::UnknownRace {
            queue: QueueKind::NpcShips,
            ..
        })
    ));
}
