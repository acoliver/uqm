//! Tests for the hyperspace step of a save (`getSisElement` and
//! `SaveSisHyperState`).
//!
//! Elements and ships use the owned battle pools at their game capacities.
//! Tests preserve the owner-facing rule, the player-ship flag, queue ordering,
//! refusal of missing/retired records, and the guarantee that saving writes only
//! the flagship's stored facing.

use crate::engine::battle::{ElementId, ElementTable, Parent, ShipId};
use crate::engine::collections::pool::Id;

use super::hyper_save::*;
use crate::comm::locdata::CGameState;
use crate::engine::battle::flags::{PLAYER_SHIP, RPG_PLAYER_NUM};
use crate::engine::battle::link_queue::RacePool;
use crate::engine::battle::record::{ElementRecord, ShipRecord};

/// `PLAYER_SHIP` (element.h), written out so the generated value is checked
/// against the header's `1 << 2`.
const PLAYER_SHIP_BIT: u16 = 1 << 2;
/// Another element flag that a flagship may carry: `APPEARING` (element.h).
const APPEARING_BIT: u16 = 1 << 0;
/// `MAX_SHIPS_PER_SIDE` (races.h) and `MAX_DISPLAY_ELEMENTS` (element.h).
const SHIP_SLOTS: u16 = 14;
const ELEMENT_SLOTS: u16 = 150;
const SENTINEL_FACING: u16 = 0x5555;

type ShipKey = Id<ShipRecord>;

fn allocate(queue: &mut RacePool, record: ShipRecord) -> ShipKey {
    queue
        .fixture_pool()
        .alloc(record)
        .unwrap_or_else(|| panic!("fixture capacity"))
}

fn push(queue: &mut RacePool, record: ShipRecord) -> ShipKey {
    let id = allocate(queue, record);
    queue.fixture_pool().put(id);
    id
}

fn allocate_element(elements: &mut ElementTable, value: ElementRecord) -> ElementId {
    let id = elements
        .alloc()
        .unwrap_or_else(|| panic!("fixture capacity"));
    *elements.fixture_record_mut(id) = value;
    id
}

fn push_element(elements: &mut ElementTable, value: ElementRecord) -> ElementId {
    let id = allocate_element(elements, value);
    elements.put(id);
    id
}

fn starship(facing: u16, element: Option<ElementId>) -> ShipRecord {
    let mut ship = ShipRecord::default();
    ship.ship_facing = facing;
    ship.ship_element = element;
    ship.captains_name_index = 1;
    ship.crew_level = 42;
    ship
}

fn element(flags: u16, owner: Parent) -> ElementRecord {
    ElementRecord {
        state_flags: flags,
        parent: owner,
        player_nr: 0,
        ..ElementRecord::default()
    }
}

/// The two queues of a hyperspace flight with nothing queued yet.
struct Queues {
    ships: RacePool,
    display: ElementTable,
}

impl Queues {
    fn new() -> Self {
        let ships = RacePool::with_capacity(0, usize::from(SHIP_SLOTS));
        let display = ElementTable::with_capacity(usize::from(ELEMENT_SLOTS));
        Self { ships, display }
    }

    /// The flagship of a normal flight: one `STARSHIP` heading `race_q`, one
    /// `ELEMENT` of the display list that it owns and that points back at it.
    /// An unrelated element sits before and after it in the display list.
    fn flight(facing: u16) -> Self {
        let mut queues = Self::new();
        push_element(&mut queues.display, element(APPEARING_BIT, Parent::None));
        let ship = push(&mut queues.ships, starship(facing, None));
        let flagship = push_element(
            &mut queues.display,
            element(
                PLAYER_SHIP_BIT | APPEARING_BIT,
                Parent::Ship(queue_ship(&queues.ships, 0)),
            ),
        );
        push_element(&mut queues.display, element(0, Parent::None));
        queues.ships.fixture_pool().get_mut(ship).ship_element = Some(flagship);
        queues
    }

    fn head_key(&self) -> ShipKey {
        self.ships
            .fixture_records()
            .head()
            .unwrap_or_else(|| panic!("a queued flagship"))
    }

    fn head_ship(&mut self) -> &mut ShipRecord {
        let head = self.head_key();
        self.ships.fixture_pool().get_mut(head)
    }

    fn flagship(&self) -> ElementId {
        self.ships
            .fixture_records()
            .get(self.head_key())
            .ship_element
            .unwrap_or_else(|| panic!("the flagship has an element"))
    }

    fn flagship_element(&mut self) -> &mut ElementRecord {
        let flagship = self.flagship();
        self.display.fixture_record_mut(flagship)
    }

    fn save(&self, game: &mut CGameState) -> Result<HyperFlagship, HyperSaveError> {
        save_hyper_facing(game, &self.ships, &self.display)
    }
}

/// A `GAME_STATE` with a recognisable value in every field the step might touch.
fn game() -> Box<CGameState> {
    let mut game = Box::<CGameState>::default();
    game.ship_stamp.origin.x = 301;
    game.ship_stamp.origin.y = -302;
    game.ip_location.x = 4000;
    game.ip_location.y = -5000;
    game.autopilot.x = 77;
    game.velocity.travel_angle = 123;
    game.ship_facing = SENTINEL_FACING;
    game.ip_planet = 0xAB;
    game.in_orbit = 0x7C;
    game.glob_flags = 9;
    game.battle_group_ref = 0xDEAD_BEEF;

    game
}

fn bytes_of(value: &CGameState) -> String {
    format!("{value:?}")
}

fn queues_snapshot(queues: &Queues) -> (Vec<String>, Vec<String>) {
    (
        (0..32)
            .filter_map(|slot| queues.ships.fixture_records().id_at(slot))
            .map(|id| {
                format!(
                    "{id:?}:{:?}:{:?}:{:?}:{:?}",
                    queues.ships.fixture_records().get(id),
                    queues.ships.fixture_records().pred(id),
                    queues.ships.fixture_records().succ(id),
                    queues.ships.fixture_records().linked(id)
                )
            })
            .collect(),
        (0..usize::from(ELEMENT_SLOTS))
            .filter_map(|slot| queues.display.id_at(slot))
            .map(|id| {
                format!(
                    "{id:?}:{:?}:{:?}:{:?}:{:?}",
                    queues.display[id],
                    queues.display.pred(id),
                    queues.display.succ(id),
                    queues.display.linked(id)
                )
            })
            .collect(),
    )
}

/// The game state apart from `ShipFacing`, which is zeroed.
fn without_facing(state: &CGameState) -> String {
    bytes_of(&CGameState {
        ship_facing: 0,
        ..*state
    })
}

#[test]
fn generated_flags_and_player_match_the_headers() {
    assert_eq!(PLAYER_SHIP, u32::from(PLAYER_SHIP_BIT));
    assert_eq!(RPG_PLAYER_NUM, 0);
}

#[test]
fn the_saved_facing_is_the_owners_facing_plus_one_and_nothing_else_changes() {
    let queues = Queues::flight(9);
    let mut game = game();
    let before_game = without_facing(&game);
    let before_queues = queues_snapshot(&queues);

    assert_eq!(queues.save(&mut game), Ok(HyperFlagship::Facing(10)));

    assert_eq!(game.ship_facing, 10);
    assert_eq!(without_facing(&game), before_game);
    assert_eq!(queues_snapshot(&queues), before_queues);
}

#[test]
fn facing_zero_is_saved_as_one_because_reentry_needs_it_non_zero() {
    let queues = Queues::flight(0);
    let mut game = game();
    assert_eq!(queues.save(&mut game), Ok(HyperFlagship::Facing(1)));
    assert_eq!(game.ship_facing, 1);
}

#[test]
fn every_facing_of_the_sixteen_is_saved_one_higher() {
    for facing in 0..16u16 {
        let queues = Queues::flight(facing);
        let mut game = game();
        assert_eq!(
            queues.save(&mut game),
            Ok(HyperFlagship::Facing(facing + 1))
        );
        assert_eq!(game.ship_facing, facing + 1, "facing {facing}");
    }
}

#[test]
fn the_facing_keeps_the_full_sixteen_bit_width() {
    let queues = Queues::flight(65534);
    let mut game = game();
    assert_eq!(queues.save(&mut game), Ok(HyperFlagship::Facing(65535)));
    assert_eq!(game.ship_facing, 65535);
}

#[test]
fn a_facing_that_would_wrap_to_zero_is_refused_and_writes_nothing() {
    let queues = Queues::flight(65535);
    let mut game = game();
    let before = bytes_of(&*game);
    assert_eq!(queues.save(&mut game), Err(HyperSaveError::FacingOverflow));
    assert_eq!(bytes_of(&*game), before);
    assert_eq!(game.ship_facing, SENTINEL_FACING);
}

/// The element's `pParent` is the owner, not the head of `race_q`: with two
/// `STARSHIP`s the facing is the second one's.
#[test]
fn the_owner_is_the_elements_parent_even_when_it_is_not_the_queue_head() {
    let mut queues = Queues::flight(3);
    push(&mut queues.ships, starship(11, None));
    queues.flagship_element().parent = Parent::Ship(queue_ship(&queues.ships, 1));
    assert_eq!(queues.head_ship().ship_facing, 3);

    let mut game = game();
    assert_eq!(queues.save(&mut game), Ok(HyperFlagship::Facing(12)));
    assert_eq!(game.ship_facing, 12);
}

#[test]
fn only_the_head_ship_names_the_element_and_later_ships_are_not_consulted() {
    let mut queues = Queues::flight(6);
    // A second ship with its own element: C looks at the head only.
    let other_owner = push(&mut queues.ships, starship(14, None));
    let other_element = push_element(
        &mut queues.display,
        element(PLAYER_SHIP_BIT, Parent::Ship(queue_ship(&queues.ships, 1))),
    );
    queues
        .ships
        .fixture_pool()
        .get_mut(other_owner)
        .ship_element = Some(other_element);

    let mut game = game();
    assert_eq!(queues.save(&mut game), Ok(HyperFlagship::Facing(7)));
}

#[test]
fn an_empty_player_queue_is_a_no_op() {
    let queues = Queues::new();
    let mut game = game();
    let before = bytes_of(&*game);
    assert_eq!(queues.save(&mut game), Ok(HyperFlagship::Absent));
    assert_eq!(bytes_of(&*game), before);
    assert_eq!(game.ship_facing, SENTINEL_FACING);
}

/// "Happens when saving a game from Hyperspace encounter screen": the ship is
/// still queued but has no element.
#[test]
fn a_flagship_without_an_element_is_a_no_op_and_the_display_list_is_not_read() {
    let mut queues = Queues::flight(5);
    queues.head_ship().ship_element = None;

    let mut game = game();
    let before_game = bytes_of(&*game);
    let before_queues = queues_snapshot(&queues);
    assert_eq!(queues.save(&mut game), Ok(HyperFlagship::Absent));
    assert_eq!(bytes_of(&*game), before_game);
    assert_eq!(queues_snapshot(&queues), before_queues);
}

#[test]
fn a_handle_that_is_not_a_link_of_the_display_list_is_refused() {
    let mut queues = Queues::flight(5);
    // A slot of the pool that was allocated but never queued.
    let loose = allocate_element(
        &mut queues.display,
        element(PLAYER_SHIP_BIT, Parent::Ship(queue_ship(&queues.ships, 0))),
    );
    queues.head_ship().ship_element = Some(loose);

    let mut game = game();
    assert_eq!(
        queues.save(&mut game),
        Err(HyperSaveError::ElementNotInDisplayList)
    );
    assert_eq!(game.ship_facing, SENTINEL_FACING);
}

#[test]
fn a_stale_generation_or_slot_outside_the_pool_is_refused() {
    let mut queues = Queues::flight(5);
    let flagship = queues.flagship();
    for id in [
        ElementId::fixture(flagship.slot(), flagship.generation() + 1),
        ElementId::fixture(u8::MAX, 0),
    ] {
        queues.head_ship().ship_element = Some(id);
        let mut game = game();
        assert_eq!(
            queues.save(&mut game),
            Err(HyperSaveError::ElementNotInDisplayList)
        );
    }
}

#[test]
fn an_element_without_player_ship_is_refused() {
    let mut queues = Queues::flight(5);
    queues.flagship_element().state_flags = APPEARING_BIT;
    let mut game = game();
    assert_eq!(
        queues.save(&mut game),
        Err(HyperSaveError::ElementIsNotAPlayerShip)
    );
    assert_eq!(game.ship_facing, SENTINEL_FACING);
}

#[test]
fn an_element_with_no_parent_is_refused() {
    let mut queues = Queues::flight(5);
    queues.flagship_element().parent = Parent::None;
    let mut game = game();
    assert_eq!(queues.save(&mut game), Err(HyperSaveError::NoOwner));
    assert_eq!(game.ship_facing, SENTINEL_FACING);
}

#[test]
fn an_owner_that_is_not_a_link_of_the_player_queue_is_refused() {
    let mut queues = Queues::flight(5);
    let free_slot = allocate(&mut queues.ships, starship(1, None));
    assert_ne!(free_slot, queues.head_key());
    let mut world = crate::engine::World::new();
    let group = world.ip_groups().alloc().unwrap();
    for owner in [
        Parent::Ship(queue_ship(&queues.ships, 1)),
        Parent::Ship(crate::engine::battle::ShipId::fixture(1, 0, 0)),
        Parent::IpGroup(group),
    ] {
        queues.flagship_element().parent = owner;
        let mut game = game();
        assert_eq!(
            queues.save(&mut game),
            Err(HyperSaveError::OwnerNotInPlayerQueue)
        );
        assert_eq!(game.ship_facing, SENTINEL_FACING);
    }
}

#[test]
fn a_disabled_ship_pool_has_no_flagship_and_writes_nothing() {
    let queues = Queues::new();
    let mut game = game();
    let disabled = RacePool::with_capacity(0, 0);
    let saved = save_hyper_facing(&mut game, &disabled, &queues.display);
    assert_eq!(saved, Ok(HyperFlagship::Absent));
    assert_eq!(game.ship_facing, SENTINEL_FACING);
}

#[test]
fn a_retired_element_is_refused_after_its_slot_is_freed() {
    let mut queues = Queues::flight(5);
    let id = queues.flagship();
    queues.display.remove(id);
    queues.display.free(id);
    let mut game = game();
    assert_eq!(
        queues.save(&mut game),
        Err(HyperSaveError::ElementNotInDisplayList)
    );
    assert_eq!(game.ship_facing, SENTINEL_FACING);
}

#[test]
fn a_reset_element_pool_does_not_retain_the_flagship() {
    let mut queues = Queues::flight(5);
    queues.display.reinit();
    let mut game = game();
    assert_eq!(
        queues.save(&mut game),
        Err(HyperSaveError::ElementNotInDisplayList)
    );
    assert_eq!(game.ship_facing, SENTINEL_FACING);
}

#[test]
fn reading_the_flagship_facing_returns_the_owner_facing_and_writes_nothing() {
    let queues = Queues::flight(9);
    let facing = read_flagship_facing(&queues.ships, &queues.display);
    assert_eq!(
        facing,
        Ok(Some(9)),
        "the raw facing, not the saved plus one"
    );
    let again = read_flagship_facing(&queues.ships, &queues.display);
    assert_eq!(again, Ok(Some(9)));
}

#[test]
fn reading_the_facing_of_a_flight_without_a_flagship_is_none() {
    let queues = Queues::new();
    let facing = read_flagship_facing(&queues.ships, &queues.display);
    assert_eq!(facing, Ok(None));
}

fn queue_ship(pool: &RacePool, slot: usize) -> ShipId {
    use crate::engine::battle::link_queue::{LinkQueue, Starship};
    let queue = LinkQueue::<Starship>::borrow(pool);
    let key = pool.fixture_records().id_at(slot).unwrap();
    let address = std::ptr::from_ref(pool.fixture_records().get(key)) as usize;
    queue.ship_id(queue.id(address).unwrap())
}
