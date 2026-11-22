//! Tests for the live "GlSt", "GmSt" and "Evts" capture and restore.
//!
//! The field checks place and read bytes at explicit offsets in the retained
//! scalar record. Live activity is supplied separately. Serialized chunks are
//! compared with the bytes of a save written by the C runtime; event checks
//! use the owned fixed-capacity game-clock pool.

use super::*;
use crate::savegame::decode::decode_save;
use crate::savegame::types::{
    ClockState, EventEntry, Extent, GlobalState, Point, Velocity, GAME_STATE_BYTES,
};
use serial_test::serial;
use std::mem::offset_of;
use std::ops::{Deref, DerefMut};
use uqm_engine::battle::link_queue::LinkId;
use uqm_engine::battle::schema::GameRecord;
use uqm_engine::clock::NUM_EVENTS;
use uqm_engine::clock::{GameClock, ScheduledEvent};
use uqm_engine::state::ffi::{snapshot_game_state, GameStateOwnerError};

/// Tracked evidence from a real run of the C game: epic 20 load/save proof.
const C_WRITTEN_SAVE: &[u8] = include_bytes!("../../../tests/fixtures/savegame/legacy-save.00");

// Chunk positions inside C_WRITTEN_SAVE, read with a separate script.
const FIXTURE_GLST: std::ops::Range<usize> = 180..255;
const FIXTURE_GMST: std::ops::Range<usize> = 263..418;
const FIXTURE_EVTS: std::ops::Range<usize> = 1194..1214;

// ---------------------------------------------------------------------------
// A live GAME_STATE with a real event pool
// ---------------------------------------------------------------------------

struct TestState {
    game: CGameState,
    current_activity: u16,
}
impl Deref for TestState {
    type Target = CGameState;
    fn deref(&self) -> &Self::Target {
        &self.game
    }
}
impl DerefMut for TestState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.game
    }
}
struct LiveGame(Box<TestState>, GameClock);

impl LiveGame {
    fn new() -> Self {
        let game = Box::new(TestState {
            game: CGameState::default(),
            current_activity: 0,
        });
        Self(game, GameClock::new())
    }
}

impl Deref for LiveGame {
    type Target = TestState;
    fn deref(&self) -> &TestState {
        &self.0
    }
}

impl DerefMut for LiveGame {
    fn deref_mut(&mut self) -> &mut TestState {
        &mut self.0
    }
}

/// One field of the record: its offset as the C compiler lays it out and
/// its little-endian bytes.
struct Field {
    offset: usize,
    bytes: Vec<u8>,
}

fn pair_bytes(a: i16, b: i16) -> Vec<u8> {
    [a.to_le_bytes(), b.to_le_bytes()].concat()
}

/// The record's `GlSt` fields and battle group reference at their offsets.
fn fields(game: &CGameState) -> Vec<Field> {
    use c_offset::*;
    let clock = &game.game_clock;
    let velocity = &game.velocity;
    [
        (GLOB_FLAGS, vec![game.glob_flags]),
        (CREW_COST, vec![game.crew_cost]),
        (FUEL_COST, vec![game.fuel_cost]),
        (MODULE_COST, game.module_cost.to_vec()),
        (ELEMENT_WORTH, game.element_worth.to_vec()),
        (CLOCK_DAY, vec![clock.day_index]),
        (CLOCK_MONTH, vec![clock.month_index]),
        (CLOCK_YEAR, clock.year_index.to_le_bytes().to_vec()),
        (CLOCK_TICKS, clock.tick_count.to_le_bytes().to_vec()),
        (CLOCK_DAY_TICKS, clock.day_in_ticks.to_le_bytes().to_vec()),
        (AUTOPILOT, pair_bytes(game.autopilot.x, game.autopilot.y)),
        (
            IP_LOCATION,
            pair_bytes(game.ip_location.x, game.ip_location.y),
        ),
        (
            STAMP_ORIGIN,
            pair_bytes(game.ship_stamp.origin.x, game.ship_stamp.origin.y),
        ),
        (SHIP_FACING, game.ship_facing.to_le_bytes().to_vec()),
        (IP_PLANET, vec![game.ip_planet]),
        (IN_ORBIT, vec![game.in_orbit]),
        (VELOCITY_ANGLE, velocity.travel_angle.to_le_bytes().to_vec()),
        (
            VELOCITY_VECTOR,
            pair_bytes(velocity.vector.width, velocity.vector.height),
        ),
        (
            VELOCITY_FRACT,
            pair_bytes(velocity.fract.width, velocity.fract.height),
        ),
        (
            VELOCITY_ERROR,
            pair_bytes(velocity.error.width, velocity.error.height),
        ),
        (
            VELOCITY_INCR,
            pair_bytes(velocity.incr.width, velocity.incr.height),
        ),
        (
            BATTLE_GROUP_REF,
            game.battle_group_ref.to_le_bytes().to_vec(),
        ),
    ]
    .into_iter()
    .map(|(offset, bytes)| Field { offset, bytes })
    .collect()
}

/// Write the field at byte `offset` of the record from its little-endian `bytes`.
fn put(game: &mut CGameState, offset: usize, bytes: &[u8]) {
    use c_offset::*;
    let byte = || bytes[0];
    let half = |at: usize| [bytes[at], bytes[at + 1]];
    let word = |at: usize| i16::from_le_bytes(half(at));
    let target = &mut *game;
    match offset {
        GLOB_FLAGS => target.glob_flags = byte(),
        CREW_COST => target.crew_cost = byte(),
        FUEL_COST => target.fuel_cost = byte(),
        MODULE_COST => target.module_cost.copy_from_slice(bytes),
        ELEMENT_WORTH => target.element_worth.copy_from_slice(bytes),
        CLOCK_DAY => target.game_clock.day_index = byte(),
        CLOCK_MONTH => target.game_clock.month_index = byte(),
        CLOCK_YEAR => target.game_clock.year_index = u16::from_le_bytes(half(0)),
        CLOCK_TICKS => target.game_clock.tick_count = word(0),
        CLOCK_DAY_TICKS => target.game_clock.day_in_ticks = word(0),
        AUTOPILOT => (target.autopilot.x, target.autopilot.y) = (word(0), word(2)),
        IP_LOCATION => (target.ip_location.x, target.ip_location.y) = (word(0), word(2)),
        STAMP_ORIGIN => {
            let origin = &mut target.ship_stamp.origin;
            (origin.x, origin.y) = (word(0), word(2));
        }
        SHIP_FACING => target.ship_facing = u16::from_le_bytes(half(0)),
        IP_PLANET => target.ip_planet = byte(),
        IN_ORBIT => target.in_orbit = byte(),
        VELOCITY_ANGLE => target.velocity.travel_angle = u16::from_le_bytes(half(0)),
        VELOCITY_VECTOR => {
            let v = &mut target.velocity.vector;
            (v.width, v.height) = (word(0), word(2));
        }
        VELOCITY_FRACT => {
            let v = &mut target.velocity.fract;
            (v.width, v.height) = (word(0), word(2));
        }
        VELOCITY_ERROR => {
            let v = &mut target.velocity.error;
            (v.width, v.height) = (word(0), word(2));
        }
        VELOCITY_INCR => {
            let v = &mut target.velocity.incr;
            (v.width, v.height) = (word(0), word(2));
        }
        _ => panic!("no field at offset {offset}"),
    }
}

/// Read the `len` bytes of the field at byte `offset` of the record.
fn get(game: &CGameState, offset: usize, len: usize) -> Vec<u8> {
    let field = fields(game)
        .into_iter()
        .find(|field| field.offset == offset)
        .unwrap_or_else(|| panic!("no field at offset {offset}"));
    assert_eq!(field.bytes.len(), len, "field length at offset {offset}");
    field.bytes
}

/// Distinct keys for the `GlSt` fields of the record: their offsets in it.
mod c_offset {
    use super::{offset_of, GameRecord};
    pub const GLOB_FLAGS: usize = offset_of!(GameRecord, glob_flags);
    pub const CREW_COST: usize = offset_of!(GameRecord, crew_cost);
    pub const FUEL_COST: usize = offset_of!(GameRecord, fuel_cost);
    pub const MODULE_COST: usize = offset_of!(GameRecord, module_cost);
    pub const ELEMENT_WORTH: usize = offset_of!(GameRecord, element_worth);
    pub const CLOCK_DAY: usize = offset_of!(GameRecord, game_clock.day_index);
    pub const CLOCK_MONTH: usize = offset_of!(GameRecord, game_clock.month_index);
    pub const CLOCK_YEAR: usize = offset_of!(GameRecord, game_clock.year_index);
    pub const CLOCK_TICKS: usize = offset_of!(GameRecord, game_clock.tick_count);
    pub const CLOCK_DAY_TICKS: usize = offset_of!(GameRecord, game_clock.day_in_ticks);
    pub const AUTOPILOT: usize = offset_of!(GameRecord, autopilot);
    pub const IP_LOCATION: usize = offset_of!(GameRecord, ip_location);
    pub const STAMP_ORIGIN: usize = offset_of!(GameRecord, ship_stamp.origin);
    pub const SHIP_FACING: usize = offset_of!(GameRecord, ship_facing);
    pub const IP_PLANET: usize = offset_of!(GameRecord, ip_planet);
    pub const IN_ORBIT: usize = offset_of!(GameRecord, in_orbit);
    pub const VELOCITY_ANGLE: usize = offset_of!(GameRecord, velocity.travel_angle);
    pub const VELOCITY_VECTOR: usize = offset_of!(GameRecord, velocity.vector);
    pub const VELOCITY_FRACT: usize = offset_of!(GameRecord, velocity.fract);
    pub const VELOCITY_ERROR: usize = offset_of!(GameRecord, velocity.error);
    pub const VELOCITY_INCR: usize = offset_of!(GameRecord, velocity.incr);
    pub const BATTLE_GROUP_REF: usize = offset_of!(GameRecord, battle_group_ref);
}

/// `SaveGameState` writes the 75 "GlSt" bytes in this order: (position in the
/// chunk, length, offset in `GAME_STATE`). Every multi-byte field is stored
/// little endian, as it is in memory on the targets this builds for.
fn glst_layout() -> Vec<(usize, usize, usize)> {
    use c_offset::*;
    vec![
        (0, 1, GLOB_FLAGS),
        (1, 1, CREW_COST),
        (2, 1, FUEL_COST),
        (3, 20, MODULE_COST),
        (23, 8, ELEMENT_WORTH),
        (33, 1, CLOCK_DAY),
        (34, 1, CLOCK_MONTH),
        (35, 2, CLOCK_YEAR),
        (37, 2, CLOCK_TICKS),
        (39, 2, CLOCK_DAY_TICKS),
        (41, 4, AUTOPILOT),
        (45, 4, IP_LOCATION),
        (49, 4, STAMP_ORIGIN),
        (53, 2, SHIP_FACING),
        (55, 1, IP_PLANET),
        (56, 1, IN_ORBIT),
        (57, 2, VELOCITY_ANGLE),
        (59, 4, VELOCITY_VECTOR),
        (63, 4, VELOCITY_FRACT),
        (67, 4, VELOCITY_ERROR),
        (71, 4, VELOCITY_INCR),
    ]
}

/// Values for every `GlSt` field that differ from one another and from zero.
fn sentinel_global() -> GlobalState {
    let mut module_cost = [0u8; 20];
    for (i, byte) in module_cost.iter_mut().enumerate() {
        *byte = 0x20 + i as u8;
    }
    let mut element_worth = [0u8; 8];
    for (i, byte) in element_worth.iter_mut().enumerate() {
        *byte = 0x60 + i as u8;
    }
    GlobalState {
        glob_flags: 0x11,
        crew_cost: 0x12,
        fuel_cost: 0x13,
        module_cost,
        element_worth,
        current_activity: 0x4a05,
        clock: ClockState {
            day_index: 21,
            month_index: 9,
            year_index: 2161,
            tick_count: -37,
            day_in_ticks: 720,
        },
        autopilot: Point { x: -1234, y: 4321 },
        ip_location: Point { x: 7777, y: -8888 },
        ship_stamp_origin: Point { x: 321, y: -654 },
        ship_facing: 0xa1b2,
        ip_planet: 5,
        in_orbit: 6,
        velocity: Velocity {
            travel_angle: 0xc3d4,
            vector: Extent {
                width: -11,
                height: 12,
            },
            fract: Extent {
                width: 13,
                height: -14,
            },
            error: Extent {
                width: -15,
                height: 16,
            },
            incr: Extent {
                width: 17,
                height: -18,
            },
        },
    }
}

/// The bytes of `sentinel_global()` as the C record holds them, written out
/// field by field here and not derived from the Rust types.
fn sentinel_fields() -> Vec<(usize, Vec<u8>)> {
    use c_offset::*;
    let g = sentinel_global();
    let pair = |a: i16, b: i16| [a.to_le_bytes(), b.to_le_bytes()].concat();
    vec![
        (GLOB_FLAGS, vec![0x11]),
        (CREW_COST, vec![0x12]),
        (FUEL_COST, vec![0x13]),
        (MODULE_COST, g.module_cost.to_vec()),
        (ELEMENT_WORTH, g.element_worth.to_vec()),
        (CLOCK_DAY, vec![21]),
        (CLOCK_MONTH, vec![9]),
        (CLOCK_YEAR, 2161u16.to_le_bytes().to_vec()),
        (CLOCK_TICKS, (-37i16).to_le_bytes().to_vec()),
        (CLOCK_DAY_TICKS, 720i16.to_le_bytes().to_vec()),
        (AUTOPILOT, pair(-1234, 4321)),
        (IP_LOCATION, pair(7777, -8888)),
        (STAMP_ORIGIN, pair(321, -654)),
        (SHIP_FACING, 0xa1b2u16.to_le_bytes().to_vec()),
        (IP_PLANET, vec![5]),
        (IN_ORBIT, vec![6]),
        (VELOCITY_ANGLE, 0xc3d4u16.to_le_bytes().to_vec()),
        (VELOCITY_VECTOR, pair(-11, 12)),
        (VELOCITY_FRACT, pair(13, -14)),
        (VELOCITY_ERROR, pair(-15, 16)),
        (VELOCITY_INCR, pair(17, -18)),
    ]
}

// ---------------------------------------------------------------------------
// The event pool, read by following its raw links
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawEvent {
    slot: LinkId<ScheduledEvent>,
    pred_slot: Option<LinkId<ScheduledEvent>>,
    succ_slot: Option<LinkId<ScheduledEvent>>,
    date_and_function: [u8; 5],
}

fn walk_events(clock: &GameClock) -> Vec<RawEvent> {
    let queue = clock.events_ref();
    let mut walked = Vec::new();
    let mut next = queue.head();
    while let Some(id) = next {
        let event = queue.get(id);
        let year = event.year_index.to_le_bytes();
        walked.push(RawEvent {
            slot: id,
            pred_slot: queue.pred(id),
            succ_slot: queue.succ(id),
            date_and_function: [
                event.day_index,
                event.month_index,
                year[0],
                year[1],
                event.func_index,
            ],
        });
        next = queue.succ(id);
        assert!(walked.len() <= NUM_EVENTS, "event list does not end");
    }
    walked
}

fn queue_state(
    clock: &GameClock,
) -> (
    Option<LinkId<ScheduledEvent>>,
    Option<LinkId<ScheduledEvent>>,
    Vec<RawEvent>,
) {
    let queue = clock.events_ref();
    (queue.head(), queue.tail(), walk_events(clock))
}

fn event(day: u8, month: u8, year: u16, func: u8) -> EventEntry {
    EventEntry {
        day_index: day,
        month_index: month,
        year_index: year,
        func_index: func,
    }
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

#[test]
fn event_pool_preserves_all_handler_records_at_capacity() {
    assert_eq!(NUM_EVENTS, 18, "gameev.h lists 18 event kinds");
    let mut clock = GameClock::new();
    let mut ids = Vec::new();
    for selector in 0..NUM_EVENTS {
        let mut queue = clock.events();
        let Some(id) = queue.alloc() else {
            panic!("event pool exhausted before handler {selector}");
        };
        let record = queue.get_mut(id);
        record.day_index = 1;
        record.month_index = 1;
        record.year_index = 2155;
        record.func_index = selector as u8;
        ids.push(id);
    }
    assert!(clock.events().alloc().is_none());
    assert_eq!(events::capture_events(&clock), Ok(Vec::new()));
    for id in ids {
        clock.events().put(id);
    }
    let expected = (0..NUM_EVENTS)
        .map(|selector| event(1, 1, 2155, selector as u8))
        .collect();
    assert_eq!(events::capture_events(&clock), Ok(expected));
}

// ---------------------------------------------------------------------------
// "GlSt" fields, one offset at a time
// ---------------------------------------------------------------------------

#[test]
fn capture_reads_every_glst_field_from_its_c_offset() {
    let mut game = LiveGame::new();
    for (offset, bytes) in sentinel_fields() {
        put(&mut game, offset, &bytes);
    }
    game.current_activity = 0x4a05;
    assert_eq!(
        global::capture_global_state(&game, game.current_activity),
        sentinel_global()
    );
}

#[test]
fn restore_writes_every_glst_field_at_its_c_offset() {
    let mut game = LiveGame::new();
    global::apply_global_state(&mut game, &sentinel_global());
    for (offset, bytes) in sentinel_fields() {
        assert_eq!(get(&game, offset, bytes.len()), bytes, "offset {offset}");
    }
}

#[test]
fn restore_leaves_what_glst_does_not_carry() {
    let mut game = LiveGame::new();
    let frame = uqm_engine::graphics::drawable::FrameId::from_bits(0x1_0001);
    game.ship_stamp.frame = frame;
    game.battle_group_ref = 0xdead_beef;

    game.current_activity = 0x0102;
    let events = [event(18, 2, 2155, 2), event(7, 3, 2156, 9)];
    events::restore_events(&mut game.1, &events).unwrap();
    let before = queue_state(&game.1);

    let next = global::apply_global_state(&mut game, &sentinel_global());

    assert_eq!(next, 0x4a05);
    assert_eq!(game.current_activity, 0x0102);
    assert_eq!(game.ship_stamp.frame, frame);
    assert_eq!(game.battle_group_ref, 0xdead_beef);

    assert_eq!(queue_state(&game.1), before);
    assert_eq!(
        get(&game, c_offset::BATTLE_GROUP_REF, 4),
        0xdead_beefu32.to_le_bytes()
    );
}

#[test]
fn every_activity_code_is_carried_through() {
    for activity in [
        0u16, 1, 2, 3, 4, 0x0100, 0x0200, 0x0408, 0x4000, 0x7fff, 0xffff,
    ] {
        let mut game = LiveGame::new();
        game.current_activity = 0x0100;
        let mut saved = sentinel_global();
        saved.current_activity = activity;
        assert_eq!(global::apply_global_state(&mut game, &saved), activity);
        assert_eq!(game.current_activity, 0x0100);
        game.current_activity = activity;
        assert_eq!(
            global::capture_global_state(&game, game.current_activity).current_activity,
            activity
        );
    }
}

// ---------------------------------------------------------------------------
// "Evts"
// ---------------------------------------------------------------------------

#[test]
fn restore_rebuilds_the_real_queue_in_order() {
    let mut game = LiveGame::new();
    // Deliberately not date order: the loader keeps the file's order.
    let events = [
        event(28, 2, 2156, 17),
        event(1, 1, 2155, 0),
        event(29, 2, 2156, 8),
        event(31, 12, 65535, 12),
    ];
    events::restore_events(&mut game.1, &events).unwrap();

    let queue = &game.1;
    let walked = walk_events(queue);
    assert_eq!(walked.len(), 4);
    for (index, (raw, wanted)) in walked.iter().zip(&events).enumerate() {
        let year = wanted.year_index.to_le_bytes();
        assert_eq!(
            raw.date_and_function,
            [
                wanted.day_index,
                wanted.month_index,
                year[0],
                year[1],
                wanted.func_index
            ]
        );
        assert_eq!(raw.pred_slot, index.checked_sub(1).map(|i| walked[i].slot));
        assert_eq!(raw.succ_slot, walked.get(index + 1).map(|next| next.slot));
    }
    assert_eq!(queue.events_ref().tail(), Some(walked[3].slot));
    assert_eq!((NUM_EVENTS - queue.events_ref().count()), NUM_EVENTS - 4);

    assert_eq!(events::capture_events(queue).unwrap(), events);
}

#[test]
fn restore_replaces_events_that_were_already_queued() {
    let mut game = LiveGame::new();
    let stale: Vec<_> = (0..10).map(|n| event(1 + n, 3, 2155, n)).collect();
    events::restore_events(&mut game.1, &stale).unwrap();

    let fresh = [event(5, 5, 2160, 4)];
    events::restore_events(&mut game.1, &fresh).unwrap();

    assert_eq!(events::capture_events(&game.1).unwrap(), fresh);
    assert_eq!((NUM_EVENTS - game.1.events_ref().count()), NUM_EVENTS - 1);

    events::restore_events(&mut game.1, &[]).unwrap();
    assert!(events::capture_events(&game.1).unwrap().is_empty());
    assert!(game.1.events_ref().head().is_none());
    assert!(game.1.events_ref().tail().is_none());
    assert_eq!((NUM_EVENTS - game.1.events_ref().count()), NUM_EVENTS);
}

#[test]
fn a_full_pool_of_events_round_trips_and_leaves_no_free_slot() {
    let mut game = LiveGame::new();
    let events: Vec<_> = (0..NUM_EVENTS as u8)
        .map(|n| event(1 + n, 1 + n % 12, 2155 + u16::from(n), n))
        .collect();
    events::restore_events(&mut game.1, &events).unwrap();
    assert!(game.1.events().alloc().is_none());
    assert_eq!(events::capture_events(&game.1).unwrap(), events);
}

#[test]
fn leap_day_is_valid_only_in_a_leap_year() {
    let mut game = LiveGame::new();
    let queue = &mut game.1;
    assert!(events::restore_events(queue, &[event(29, 2, 2156, 0)]).is_ok());
    assert!(events::restore_events(queue, &[event(29, 2, 2400, 0)]).is_ok());
    for year in [2155u16, 2100] {
        assert_eq!(
            events::restore_events(queue, &[event(29, 2, year, 0)]),
            Err(EventError::InvalidEventDate {
                position: 0,
                day_index: 29,
                month_index: 2,
                year_index: year
            })
        );
    }
}

#[test]
fn rejected_events_leave_the_queue_exactly_as_it_was() {
    let mut game = LiveGame::new();
    let before_events = [
        event(18, 2, 2155, 2),
        event(2, 4, 2155, 11),
        event(9, 9, 2157, 3),
    ];
    events::restore_events(&mut game.1, &before_events).unwrap();
    let before = queue_state(&game.1);

    let too_many: Vec<_> = (0..=NUM_EVENTS as u8)
        .map(|n| event(1, 1, 2155, n % 18))
        .collect();
    let cases: Vec<(Vec<EventEntry>, EventError)> = vec![
        (
            too_many,
            EventError::TooManyEvents {
                count: NUM_EVENTS + 1,
                capacity: NUM_EVENTS,
            },
        ),
        (
            vec![event(1, 1, 2155, 0), event(1, 1, 2155, NUM_EVENTS as u8)],
            EventError::UnknownEventFunction {
                position: 1,
                func_index: NUM_EVENTS as u8,
            },
        ),
        (
            vec![event(1, 1, 2155, 255)],
            EventError::UnknownEventFunction {
                position: 0,
                func_index: 255,
            },
        ),
        (
            vec![event(1, 13, 2155, 0)],
            EventError::InvalidEventDate {
                position: 0,
                day_index: 1,
                month_index: 13,
                year_index: 2155,
            },
        ),
        (
            vec![event(1, 0, 2155, 0)],
            EventError::InvalidEventDate {
                position: 0,
                day_index: 1,
                month_index: 0,
                year_index: 2155,
            },
        ),
        (
            vec![event(0, 5, 2155, 0)],
            EventError::InvalidEventDate {
                position: 0,
                day_index: 0,
                month_index: 5,
                year_index: 2155,
            },
        ),
        (
            vec![event(3, 3, 2155, 0), event(31, 4, 2155, 0)],
            EventError::InvalidEventDate {
                position: 1,
                day_index: 31,
                month_index: 4,
                year_index: 2155,
            },
        ),
    ];
    for (events, expected) in cases {
        assert_eq!(events::restore_events(&mut game.1, &events), Err(expected));
        assert_eq!(queue_state(&game.1), before, "{expected}");
    }
    assert_eq!(events::capture_events(&game.1).unwrap(), before_events);
}

#[test]
fn event_ids_cannot_be_used_with_another_clock() {
    let mut a = GameClock::new();
    let mut b = GameClock::new();
    let id = a.events().alloc().unwrap();
    b.events().alloc().unwrap();
    assert!(!b.events_ref().contains(id));
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b
        .events_ref()
        .get(id)
        .func_index))
    .is_err());
}

#[test]
fn fixed_event_capacity_bounds_restore() {
    let mut clock = GameClock::new();
    let entries: Vec<_> = (0..=NUM_EVENTS).map(|_| event(1, 1, 2155, 0)).collect();
    assert_eq!(
        events::restore_events(&mut clock, &entries),
        Err(EventError::TooManyEvents {
            count: NUM_EVENTS + 1,
            capacity: NUM_EVENTS
        })
    );
    events::restore_events(&mut clock, &entries[..NUM_EVENTS]).unwrap();
    assert!(clock.events().alloc().is_none());
}

#[test]
fn capture_refuses_invalid_live_event_fields() {
    let mut clock = GameClock::new();
    let entries = [
        event(1, 1, 2155, 0),
        event(2, 1, 2155, 1),
        event(3, 1, 2155, 2),
    ];
    events::restore_events(&mut clock, &entries).unwrap();
    let second = clock.events_ref().at(1).unwrap();
    clock.events().get_mut(second).func_index = 99;
    assert_eq!(
        events::capture_events(&clock),
        Err(EventError::UnknownEventFunction {
            position: 1,
            func_index: 99
        })
    );
    clock.events().get_mut(second).func_index = 1;
    let first = clock.events_ref().head().unwrap();
    clock.events().get_mut(first).month_index = 14;
    assert!(matches!(
        events::capture_events(&clock),
        Err(EventError::InvalidEventDate { position: 0, .. })
    ));
}

// ---------------------------------------------------------------------------
// GlSt + GmSt + Evts together
// ---------------------------------------------------------------------------

fn game_state_bits(seed: u8) -> [u8; GAME_STATE_BYTES] {
    let mut bits = [0u8; GAME_STATE_BYTES];
    for (i, byte) in bits.iter_mut().enumerate() {
        *byte = seed.wrapping_add((i * 3) as u8);
    }
    bits
}

#[test]
#[serial]
fn restore_uses_the_already_borrowed_story_owner() {
    let mut owner = uqm_engine::Game::new();
    {
        let mut game = LiveGame::new();
        {
            super::restore_runtime(
                &mut game.0,
                &mut owner.world.story,
                &sentinel_global(),
                &game_state_bits(7),
                &[],
                &mut game.1,
            )
            .unwrap();
        };
    }
    assert_eq!(*owner.world.story.unwrap().as_bytes(), game_state_bits(7));
}

#[test]
#[serial]
fn restore_then_capture_returns_the_sections_that_were_restored() {
    let mut game_owner = uqm_engine::Game::new();
    {
        let mut game = LiveGame::new();
        game.current_activity = 0x0100;
        let global = sentinel_global();
        let bits = game_state_bits(9);
        let events = [event(18, 2, 2155, 2), event(1, 6, 2156, 5)];

        let restored = {
            restore_runtime(
                &mut game.0,
                &mut game_owner.world.story,
                &global,
                &bits,
                &events,
                &mut game.1,
            )
        }
        .unwrap();

        assert_eq!(restored.next_activity, global.current_activity);
        assert_eq!(game.current_activity, 0x0100);
        // The Rust singleton owns the bits; the C shadow array follows it.
        assert_eq!(snapshot_game_state(&game_owner.world).unwrap(), bits);
        assert_eq!(snapshot_game_state(&game_owner.world).unwrap(), bits);

        game.current_activity = global.current_activity;
        let captured = capture_runtime(
            &game,
            game.current_activity,
            &game_owner.world.story,
            &game.1,
        )
        .unwrap();
        assert_eq!(captured.global, global);
        assert_eq!(captured.game_state, bits);
        assert_eq!(captured.events, events);
    }
}

#[test]
#[serial]
fn capture_reads_game_state_from_the_singleton() {
    let mut game_owner = uqm_engine::Game::new();
    {
        let mut game = LiveGame::new();
        let singleton = game_state_bits(40);
        {
            restore_runtime(
                &mut game.0,
                &mut game_owner.world.story,
                &sentinel_global(),
                &singleton,
                &[],
                &mut game.1,
            )
        }
        .unwrap();

        assert_eq!(
            capture_runtime(
                &game,
                game.current_activity,
                &game_owner.world.story,
                &game.1
            )
            .unwrap()
            .game_state,
            singleton
        );
    }
}

#[test]
#[serial]
fn a_rejected_restore_changes_nothing() {
    let mut game_owner = uqm_engine::Game::new();
    {
        let mut game = LiveGame::new();
        let original_bits = game_state_bits(1);
        let original_events = [event(4, 4, 2155, 4)];
        let mut original_global = sentinel_global();
        original_global.current_activity = 0x0100;
        {
            restore_runtime(
                &mut game.0,
                &mut game_owner.world.story,
                &original_global,
                &original_bits,
                &original_events,
                &mut game.1,
            )
        }
        .unwrap();
        game.current_activity = 0x0100;
        let before_sections = capture_runtime(
            &game,
            game.current_activity,
            &game_owner.world.story,
            &game.1,
        )
        .unwrap();
        let before_queue = queue_state(&game.1);

        let mut other_global = sentinel_global();
        other_global.glob_flags = 0xff;
        other_global.clock.year_index = 3000;
        let bad = [event(1, 1, 2155, 0), event(30, 2, 2155, 1)];
        let result = {
            restore_runtime(
                &mut game.0,
                &mut game_owner.world.story,
                &other_global,
                &game_state_bits(200),
                &bad,
                &mut game.1,
            )
        };

        assert_eq!(
            result,
            Err(RuntimeError::Events(EventError::InvalidEventDate {
                position: 1,
                day_index: 30,
                month_index: 2,
                year_index: 2155
            }))
        );
        assert_eq!(
            capture_runtime(
                &game,
                game.current_activity,
                &game_owner.world.story,
                &game.1
            )
            .unwrap(),
            before_sections
        );
        assert_eq!(queue_state(&game.1), before_queue);

        assert_eq!(
            snapshot_game_state(&game_owner.world).unwrap(),
            original_bits
        );
    }
}

// ---------------------------------------------------------------------------
// A save the C game wrote
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn restoring_a_c_written_save_puts_its_bytes_at_the_c_offsets() {
    let mut game_owner = uqm_engine::Game::new();
    {
        assert_eq!(&C_WRITTEN_SAVE[FIXTURE_GLST.start - 8..][..4], b"GlSt");
        assert_eq!(&C_WRITTEN_SAVE[FIXTURE_GMST.start - 8..][..4], b"GmSt");
        assert_eq!(&C_WRITTEN_SAVE[FIXTURE_EVTS.start - 8..][..4], b"Evts");
        let glst = &C_WRITTEN_SAVE[FIXTURE_GLST];
        let gmst = &C_WRITTEN_SAVE[FIXTURE_GMST];
        let evts = &C_WRITTEN_SAVE[FIXTURE_EVTS];
        assert_eq!((glst.len(), gmst.len(), evts.len()), (75, 155, 20));

        let save = decode_save(C_WRITTEN_SAVE).unwrap();
        let mut game = LiveGame::new();
        game.current_activity = 0x0100;
        let restored = {
            restore_runtime(
                &mut game.0,
                &mut game_owner.world.story,
                &save.global,
                &save.game_state,
                &save.events,
                &mut game.1,
            )
        }
        .unwrap();

        // GlSt: every chunk byte lands where the C compiler puts that field.
        let layout = glst_layout();
        assert_eq!(layout.iter().map(|&(_, len, _)| len).sum::<usize>() + 2, 75);
        assert_eq!(game.current_activity, 0x0100);
        assert_eq!(restored.next_activity.to_le_bytes(), glst[31..33]);
        for (chunk_at, len, c_at) in layout {
            assert_eq!(
                get(&game, c_at, len),
                glst[chunk_at..chunk_at + len],
                "chunk byte {chunk_at} -> offset {c_at}"
            );
        }

        // GmSt: the singleton holds the chunk.
        assert_eq!(
            snapshot_game_state(&game_owner.world).unwrap().as_slice(),
            gmst
        );

        // Evts: four events, in the file's order, in real pool slots.
        let walked = walk_events(&game.1);
        assert_eq!(walked.len(), 4);
        for (index, raw) in walked.iter().enumerate() {
            assert_eq!(raw.date_and_function, evts[index * 5..index * 5 + 5]);
        }
        assert_eq!((NUM_EVENTS - game.1.events_ref().count()), NUM_EVENTS - 4);

        // Capturing the restored game gives back what the file held.
        game.current_activity = restored.next_activity;
        let captured = capture_runtime(
            &game,
            game.current_activity,
            &game_owner.world.story,
            &game.1,
        )
        .unwrap();
        assert_eq!(captured.global, save.global);
        assert_eq!(captured.game_state, save.game_state);
        assert_eq!(captured.events, save.events);
    }
}

#[test]
fn runtime_errors_say_which_part_failed() {
    let events: RuntimeError = EventError::TooManyEvents {
        count: 19,
        capacity: 18,
    }
    .into();
    assert_eq!(
        events.to_string(),
        "event queue: 19 events do not fit the 18-slot event pool"
    );
    let state: RuntimeError = GameStateOwnerError::NotInitialized.into();
    assert_eq!(
        state.to_string(),
        "game state: the game-state singleton is not initialised"
    );
}
