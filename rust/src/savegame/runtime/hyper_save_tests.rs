//! Tests for the hyperspace step of a save (`getSisElement` and
//! `SaveSisHyperState`).
//!
//! The queues are real `QUEUE_TABLE` pools made by `InitQueue`, sized as
//! `InitKernel` and `InitSpace` size them (14 `STARSHIP`s per side, 150
//! `ELEMENT`s), and every record is the generated `STARSHIP` or `ELEMENT`
//! filled in and linked with the Rust queue owner. The expected values are
//! worked out here from hyper.c:
//!
//! * the facing C stores is the owner's `ShipFacing` plus one;
//! * the owner is the element's `pParent`, which these tests make differ from
//!   the head of `race_q` to show that the head is not what is read;
//! * `PLAYER_SHIP` is bit 2 of the element flags (element.h).

use std::ffi::c_void;
use std::mem::size_of;

use super::hyper_save::*;
use super::queues::{QueueError, QueueKind};
use crate::c_bindings::global_state::{ELEMENT, PLAYER_SHIP, RPG_PLAYER_NUM, STARSHIP};
use crate::collections::queue::{AllocLink, InitQueue, PutQueue, Queue, UninitQueue};
use crate::comm::locdata::{CGameState, CQueue};

/// `PLAYER_SHIP` (element.h), written out so the generated value is checked
/// against the header's `1 << 2`.
const PLAYER_SHIP_BIT: u16 = 1 << 2;
/// Another element flag that a flagship may carry: `APPEARING` (element.h).
const APPEARING_BIT: u16 = 1 << 0;
/// `MAX_SHIPS_PER_SIDE` (races.h) and `MAX_DISPLAY_ELEMENTS` (element.h).
const SHIP_SLOTS: u16 = 14;
const ELEMENT_SLOTS: u16 = 150;
const SENTINEL_FACING: u16 = 0x5555;

fn zeroed_queue() -> Box<CQueue> {
    Box::new(CQueue::default())
}

fn init(queue: &mut CQueue, slots: u16, record_size: usize) {
    assert!(unsafe {
        // SAFETY: The queue storage is exclusive and its element layout matches this pool.
        InitQueue(
            std::ptr::from_mut(queue).cast::<Queue>(),
            slots,
            record_size as u16,
        )
    });
}

/// A free slot of `queue`, holding `record`, not linked into it.
fn allocate<T>(queue: &mut CQueue, record: T) -> *mut T {
    let link = unsafe {
        // SAFETY: The initialized queue is exclusively borrowed and its free-list belongs to its pool.
        AllocLink(std::ptr::from_mut(queue).cast::<Queue>())
    };
    assert!(!link.is_null(), "the pool has no free slot");
    // SAFETY: `link` is a free, aligned slot of a pool of `T`.
    unsafe { link.cast::<T>().write(record) };
    link.cast::<T>()
}

/// A slot holding `record`, appended to `queue` with `PutQueue`.
fn push<T>(queue: &mut CQueue, record: T) -> *mut T {
    let slot = allocate(queue, record);
    unsafe {
        // SAFETY: The queue is exclusively borrowed and the element was allocated from this pool.
        PutQueue(std::ptr::from_mut(queue).cast::<Queue>(), slot.cast())
    };
    slot
}

fn starship(facing: u16, element: *mut c_void) -> STARSHIP {
    // SAFETY: `STARSHIP` is integers, pointers and optional function pointers.
    let mut ship: STARSHIP = unsafe { std::mem::zeroed() };
    ship.ShipFacing = facing;
    ship.hShip = element;
    ship.captains_name_index = 1;
    ship.crew_level = 42;
    ship
}

fn element(flags: u16, owner: *mut STARSHIP) -> ELEMENT {
    // SAFETY: `ELEMENT` is integers, pointers and optional function pointers.
    let mut element: ELEMENT = unsafe { std::mem::zeroed() };
    element.state_flags = flags;
    element.pParent = owner.cast::<c_void>();
    element.playerNr = 0;
    element
}

/// The two queues of a hyperspace flight with nothing queued yet.
struct Queues {
    ships: Box<CQueue>,
    display: Box<CQueue>,
}

impl Queues {
    fn new() -> Self {
        let mut ships = zeroed_queue();
        let mut display = zeroed_queue();
        init(&mut ships, SHIP_SLOTS, size_of::<STARSHIP>());
        init(&mut display, ELEMENT_SLOTS, size_of::<ELEMENT>());
        Self { ships, display }
    }

    /// The flagship of a normal flight: one `STARSHIP` heading `race_q`, one
    /// `ELEMENT` of the display list that it owns and that points back at it.
    /// An unrelated element sits before and after it in the display list.
    fn flight(facing: u16) -> Self {
        let mut queues = Self::new();
        push(
            &mut queues.display,
            element(APPEARING_BIT, std::ptr::null_mut()),
        );
        let ship = push(&mut queues.ships, starship(facing, std::ptr::null_mut()));
        let flagship = push(
            &mut queues.display,
            element(PLAYER_SHIP_BIT | APPEARING_BIT, ship),
        );
        push(&mut queues.display, element(0, std::ptr::null_mut()));
        // SAFETY: `ship` is the slot just pushed.
        unsafe { (*ship).hShip = flagship.cast::<c_void>() };
        queues
    }

    fn head_ship(&self) -> *mut STARSHIP {
        self.ships.head.cast::<STARSHIP>()
    }

    fn flagship_element(&self) -> *mut ELEMENT {
        // SAFETY: the head is a slot of the ship pool.
        unsafe { (*self.head_ship()).hShip.cast::<ELEMENT>() }
    }

    fn save(&self, game: &mut CGameState) -> Result<HyperFlagship, HyperSaveError> {
        // SAFETY: both queues are pools made by `InitQueue` for these records
        // and nothing else touches them.
        unsafe { save_hyper_facing(game, &self.ships, &self.display) }
    }
}

impl Drop for Queues {
    fn drop(&mut self) {
        for queue in [&mut self.ships, &mut self.display] {
            unsafe {
                // SAFETY: This scope owns the initialized pool and no element is used after teardown.
                UninitQueue(std::ptr::from_mut::<CQueue>(queue).cast::<Queue>())
            };
        }
    }
}

/// A `GAME_STATE` with a recognisable value in every field the step might touch.
fn game() -> Box<CGameState> {
    // SAFETY: `CGameState` is integers and raw pointers; zero is valid.
    let mut game: Box<CGameState> = Box::new(unsafe { std::mem::zeroed() });
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
    game.current_activity = 0x0123;
    game.battle_group_ref = 0xDEAD_BEEF;

    game
}

fn bytes_of<T>(value: &T) -> Vec<u8> {
    // SAFETY: a read of `size_of::<T>()` bytes of a live value; the values here
    // start zeroed, so padding is initialised.
    unsafe { std::slice::from_raw_parts(std::ptr::from_ref(value).cast::<u8>(), size_of::<T>()) }
        .to_vec()
}

fn pool_bytes(queue: &CQueue) -> Vec<u8> {
    let len = usize::from(queue.object_size) * usize::from(queue.num_objects);
    // SAFETY: the pool is `len` bytes, allocated by `InitQueue`.
    unsafe { std::slice::from_raw_parts(queue.pq_tab, len) }.to_vec()
}

/// Everything about both queues: the queue records, their pools and so the
/// ship and element records they hold.
fn queues_snapshot(queues: &Queues) -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
    (
        bytes_of(&*queues.ships),
        pool_bytes(&queues.ships),
        bytes_of(&*queues.display),
        pool_bytes(&queues.display),
    )
}

/// The bytes of the game state apart from `ShipFacing`, which is zeroed.
fn without_facing(state: &CGameState) -> Vec<u8> {
    let mut bytes = bytes_of(state);
    let at = std::mem::offset_of!(CGameState, ship_facing);
    bytes[at..at + size_of::<u16>()].fill(0);
    bytes
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
    let second = push(&mut queues.ships, starship(11, std::ptr::null_mut()));
    // SAFETY: the flagship element is a slot of the display pool.
    unsafe { (*queues.flagship_element()).pParent = second.cast::<c_void>() };
    // SAFETY: the head is a slot of the ship pool.
    assert_eq!(unsafe { (*queues.head_ship()).ShipFacing }, 3);

    let mut game = game();
    assert_eq!(queues.save(&mut game), Ok(HyperFlagship::Facing(12)));
    assert_eq!(game.ship_facing, 12);
}

#[test]
fn only_the_head_ship_names_the_element_and_later_ships_are_not_consulted() {
    let mut queues = Queues::flight(6);
    // A second ship with its own element: C looks at the head only.
    let other_owner = push(&mut queues.ships, starship(14, std::ptr::null_mut()));
    let other_element = push(&mut queues.display, element(PLAYER_SHIP_BIT, other_owner));
    // SAFETY: `other_owner` is a slot of the ship pool.
    unsafe { (*other_owner).hShip = other_element.cast::<c_void>() };

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
    let queues = Queues::flight(5);
    // SAFETY: the head is a slot of the ship pool.
    unsafe { (*queues.head_ship()).hShip = std::ptr::null_mut() };
    // A display list that would be rejected if it were read.
    let display = queues.display.head.cast::<ELEMENT>();
    // SAFETY: `display` is a slot of the element pool.
    unsafe { (*display).succ = display.cast::<c_void>() };

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
    let owner = queues.head_ship();
    let loose = allocate(&mut queues.display, element(PLAYER_SHIP_BIT, owner));
    // SAFETY: the head is a slot of the ship pool.
    unsafe { (*queues.head_ship()).hShip = loose.cast::<c_void>() };

    let mut game = game();
    assert_eq!(
        queues.save(&mut game),
        Err(HyperSaveError::ElementNotInDisplayList)
    );
    assert_eq!(game.ship_facing, SENTINEL_FACING);
}

#[test]
fn a_handle_inside_a_slot_or_outside_the_pool_is_refused() {
    let queues = Queues::flight(5);
    let flagship = queues.flagship_element();
    for handle in [
        // SAFETY: stays inside the same slot, which is what is being tested.
        unsafe { flagship.cast::<u8>().add(2) }.cast::<c_void>(),
        std::ptr::NonNull::<ELEMENT>::dangling()
            .as_ptr()
            .cast::<c_void>(),
    ] {
        // SAFETY: the head is a slot of the ship pool.
        unsafe { (*queues.head_ship()).hShip = handle };
        let mut game = game();
        assert_eq!(
            queues.save(&mut game),
            Err(HyperSaveError::ElementNotInDisplayList)
        );
    }
}

#[test]
fn an_element_without_player_ship_is_refused() {
    let queues = Queues::flight(5);
    // SAFETY: the flagship element is a slot of the display pool.
    unsafe { (*queues.flagship_element()).state_flags = APPEARING_BIT };
    let mut game = game();
    assert_eq!(
        queues.save(&mut game),
        Err(HyperSaveError::ElementIsNotAPlayerShip)
    );
    assert_eq!(game.ship_facing, SENTINEL_FACING);
}

#[test]
fn an_element_with_no_parent_is_refused() {
    let queues = Queues::flight(5);
    // SAFETY: the flagship element is a slot of the display pool.
    unsafe { (*queues.flagship_element()).pParent = std::ptr::null_mut() };
    let mut game = game();
    assert_eq!(queues.save(&mut game), Err(HyperSaveError::NoOwner));
    assert_eq!(game.ship_facing, SENTINEL_FACING);
}

#[test]
fn an_owner_that_is_not_a_link_of_the_player_queue_is_refused() {
    let mut queues = Queues::flight(5);
    // A free slot of the same pool, a ship of the other side's pool, and an
    // address inside a ship slot but not at its start.
    let free_slot = allocate(&mut queues.ships, starship(1, std::ptr::null_mut()));
    let mut other_side = zeroed_queue();
    init(&mut other_side, SHIP_SLOTS, size_of::<STARSHIP>());
    let npc = push(&mut other_side, starship(2, std::ptr::null_mut()));
    // SAFETY: stays inside the head's slot, which is what is being tested.
    let inside = unsafe { queues.head_ship().cast::<u8>().add(8) }.cast::<STARSHIP>();

    for owner in [free_slot, npc, inside] {
        // SAFETY: the flagship element is a slot of the display pool.
        unsafe { (*queues.flagship_element()).pParent = owner.cast::<c_void>() };
        let mut game = game();
        assert_eq!(
            queues.save(&mut game),
            Err(HyperSaveError::OwnerNotInPlayerQueue)
        );
        assert_eq!(game.ship_facing, SENTINEL_FACING);
    }
    unsafe {
        // SAFETY: This scope owns the initialized pool and no element is used after teardown.
        UninitQueue(std::ptr::from_mut::<CQueue>(&mut other_side).cast::<Queue>())
    };
}

#[test]
fn a_ship_queue_that_is_not_a_pool_of_starships_is_refused() {
    let mut queues = Queues::new();
    unsafe {
        // SAFETY: This scope owns the initialized pool and no element is used after teardown.
        UninitQueue(std::ptr::from_mut::<CQueue>(&mut queues.ships).cast::<Queue>())
    };
    init(&mut queues.ships, SHIP_SLOTS, size_of::<ELEMENT>());
    let mut game = game();
    assert_eq!(
        queues.save(&mut game),
        Err(HyperSaveError::Queue(QueueError::WrongObjectSize {
            queue: QueueKind::PlayerShips,
            found: size_of::<ELEMENT>() as u16,
            expected: size_of::<STARSHIP>(),
        }))
    );
}

#[test]
fn a_queue_that_was_never_initialised_is_refused() {
    let queues = Queues::new();
    let mut game = game();
    let uninitialised = CQueue::default();
    // SAFETY: an empty queue record with no pool is walked and rejected.
    let result = unsafe { save_hyper_facing(&mut game, &uninitialised, &queues.display) };
    assert_eq!(
        result,
        Err(HyperSaveError::Queue(QueueError::PoolMissing {
            queue: QueueKind::PlayerShips
        }))
    );
}

#[test]
fn a_display_list_that_loops_is_refused() {
    let queues = Queues::flight(5);
    let tail = queues.display.tail.cast::<ELEMENT>();
    // SAFETY: `tail` is a slot of the element pool.
    unsafe { (*tail).succ = queues.display.head };
    let mut game = game();
    assert_eq!(
        queues.save(&mut game),
        Err(HyperSaveError::Queue(QueueError::CorruptQueue {
            queue: QueueKind::DisplayList,
            position: 3,
        }))
    );
    assert_eq!(game.ship_facing, SENTINEL_FACING);
}

#[test]
fn a_display_list_that_is_not_a_pool_of_elements_is_refused() {
    let mut queues = Queues::flight(5);
    let mut wrong = zeroed_queue();
    init(&mut wrong, ELEMENT_SLOTS, size_of::<STARSHIP>());
    std::mem::swap(&mut queues.display, &mut wrong);
    let mut game = game();
    // The ship queue still names an element that this queue does not hold.
    let result = queues.save(&mut game);
    std::mem::swap(&mut queues.display, &mut wrong);
    unsafe {
        // SAFETY: This scope owns the initialized pool and no element is used after teardown.
        UninitQueue(std::ptr::from_mut::<CQueue>(&mut wrong).cast::<Queue>())
    };
    assert_eq!(
        result,
        Err(HyperSaveError::Queue(QueueError::WrongObjectSize {
            queue: QueueKind::DisplayList,
            found: size_of::<STARSHIP>() as u16,
            expected: size_of::<ELEMENT>(),
        }))
    );
}

/// The numbers below are what clang computes for these headers on the 64-bit
/// targets this project builds for (bindgen's own layout tests compare the
/// same types field by field at test time); they are a check of this build, not
/// a claim about every ABI.
#[test]
fn generated_records_have_the_layout_the_headers_give_them() {
    use std::mem::{align_of, offset_of};
    assert_eq!(size_of::<STARSHIP>(), STARSHIP_SIZE);
    assert_eq!(align_of::<STARSHIP>(), 8);
    assert_eq!(offset_of!(STARSHIP, hShip), STARSHIP_HSHIP);
    assert_eq!(offset_of!(STARSHIP, ShipFacing), STARSHIP_SHIP_FACING);
    assert_eq!(size_of::<ELEMENT>(), ELEMENT_SIZE);
    assert_eq!(align_of::<ELEMENT>(), 8);
    assert_eq!(offset_of!(ELEMENT, state_flags), ELEMENT_STATE_FLAGS);
    assert_eq!(offset_of!(ELEMENT, pParent), ELEMENT_PARENT);
    assert_eq!(offset_of!(STARSHIP, pred), 0);
    assert_eq!(offset_of!(STARSHIP, succ), 8);
    assert_eq!(offset_of!(ELEMENT, pred), 0);
    assert_eq!(offset_of!(ELEMENT, succ), 8);
}

const STARSHIP_SIZE: usize = 80;
const STARSHIP_HSHIP: usize = 64;
const STARSHIP_SHIP_FACING: usize = 72;
const ELEMENT_SIZE: usize = 176;
const ELEMENT_STATE_FLAGS: usize = 50;
const ELEMENT_PARENT: usize = 160;

#[test]
fn reading_the_flagship_facing_returns_the_owner_facing_and_writes_nothing() {
    let queues = Queues::flight(9);
    // SAFETY: both queues are pools made by `InitQueue` for these records.
    let facing = unsafe { read_flagship_facing(&queues.ships, &queues.display) };
    assert_eq!(
        facing,
        Ok(Some(9)),
        "the raw facing, not the saved plus one"
    );
    // SAFETY: as above.
    let again = unsafe { read_flagship_facing(&queues.ships, &queues.display) };
    assert_eq!(again, Ok(Some(9)));
}

#[test]
fn reading_the_facing_of_a_flight_without_a_flagship_is_none() {
    let queues = Queues::new();
    // SAFETY: both queues are pools made by `InitQueue` for these records.
    let facing = unsafe { read_flagship_facing(&queues.ships, &queues.display) };
    assert_eq!(facing, Ok(None));
}
