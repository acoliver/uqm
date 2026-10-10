use super::*;
use crate::collections::queue::{ForAllLinks, FreeLink, HLink, Link, RemoveQueue};
use std::ffi::c_void;
use std::mem::size_of;

const SLOT_SIZE: u16 = (size_of::<Link>() + 8) as u16;

/// The audited game queues: zeroed until C initialises them.
struct GameQueues {
    queues: [Box<Queue>; QUEUE_COUNT],
}

impl GameQueues {
    fn new() -> Self {
        Self {
            queues: std::array::from_fn(|_| Box::new(Queue::new())),
        }
    }

    fn queue(&mut self, which: SelectedQueue) -> &mut Queue {
        &mut self.queues[SelectedQueue::ALL.iter().position(|q| *q == which).unwrap()]
    }

    fn snapshot(&self) -> CollectionSnapshot {
        CollectionSnapshot::observe(|which| {
            &self.queues[SelectedQueue::ALL.iter().position(|q| *q == which).unwrap()]
        })
    }

    fn init(&mut self, which: SelectedQueue, slots: u16) {
        assert!(self.queue(which).init(slots, SLOT_SIZE));
    }

    fn put(&mut self, which: SelectedQueue) -> HLink {
        let queue = self.queue(which);
        let h = queue.alloc_link();
        assert!(!h.is_null(), "{} exhausted", which.name());
        // SAFETY: `h` was just allocated from this queue.
        unsafe { queue.put(h) };
        h
    }
}

impl Drop for GameQueues {
    fn drop(&mut self) {
        for queue in &mut self.queues {
            queue.uninit();
        }
    }
}

fn intact(capacity: u8, queued: u8, free: u8, detached: u8) -> QueueObservation {
    QueueObservation::Intact(QueueAudit {
        capacity,
        queued,
        free,
        detached,
    })
}

fn observation(snapshot: &CollectionSnapshot, which: SelectedQueue) -> QueueObservation {
    snapshot
        .observations
        .iter()
        .find(|(queue, _)| *queue == which)
        .unwrap()
        .1
}

const SOURCE_LABEL: &str = "collection_invariants:avail_race_q=absent:npc_built_ship_q=absent:ip_group_q=absent:encounter_q=absent:built_ship_q=absent:event_q=absent:disp_q=absent:race_q[0]=absent:race_q[1]=absent:master_q=absent";

#[test]
fn queue_names_are_the_declarations_in_the_c_sources() {
    let declarations = |source: &str| -> Vec<String> {
        source
            .lines()
            .filter_map(|line| line.trim().strip_prefix("QUEUE "))
            .filter_map(|rest| rest.strip_suffix(';'))
            .map(|name| name.split('[').next().unwrap().to_string())
            .collect()
    };
    let game_state = declarations(include_str!("../../../../sc2/src/uqm/globdata.h"));
    let clock = declarations(include_str!("../../../../sc2/src/uqm/clock.h"));
    let setup = declarations(include_str!("../../../../sc2/src/uqm/setup.c"));
    let master = declarations(include_str!("../../../../sc2/src/uqm/master.c"));
    let selected: Vec<&str> = SelectedQueue::ALL.iter().map(|q| q.name()).collect();
    assert_eq!(
        selected,
        [
            "avail_race_q",
            "npc_built_ship_q",
            "ip_group_q",
            "encounter_q",
            "built_ship_q",
            "event_q",
            "disp_q",
            "race_q[0]",
            "race_q[1]",
            "master_q",
        ]
    );
    for (name, declared_in) in [
        ("avail_race_q", &game_state),
        ("npc_built_ship_q", &game_state),
        ("ip_group_q", &game_state),
        ("encounter_q", &game_state),
        ("built_ship_q", &game_state),
        ("event_q", &clock),
        ("disp_q", &setup),
        ("race_q", &setup),
        ("master_q", &master),
    ] {
        assert!(
            declared_in.iter().any(|declared| declared == name),
            "{name} is not declared as a QUEUE in the C sources"
        );
    }
}

#[test]
fn the_battle_and_entity_queues_are_audited_and_named_when_broken() {
    let mut game = GameQueues::new();
    for which in [
        SelectedQueue::DisplayList,
        SelectedQueue::BattleRace0,
        SelectedQueue::EventQueue,
        SelectedQueue::MasterShips,
    ] {
        game.init(which, 4);
    }
    let first = game.put(SelectedQueue::DisplayList);
    let healthy = game.snapshot().label();
    assert!(healthy.contains(":disp_q=intact/4/1/3/0:"), "{healthy}");
    assert!(healthy.contains(":race_q[1]=absent:"), "{healthy}");
    assert_eq!(validate_label(&healthy), Ok(LabelVerdict::Intact));

    // SAFETY: `first` is a slot of the pool; this models C corrupting `pred`.
    unsafe { (*first.cast::<Link>()).pred = first };
    let label = game.snapshot().label();
    assert_eq!(
        label,
        "collection_invariant_failed:queue=disp_q:fault=broken_pred:slot=0"
    );
    assert_eq!(validate_label(&label), Ok(LabelVerdict::Broken));
    unsafe { (*first.cast::<Link>()).pred = std::ptr::null_mut() };

    let race = game.put(SelectedQueue::BattleRace0);
    // SAFETY: as above, for `race_q[0]`.
    unsafe { (*race.cast::<Link>()).succ = race };
    let label = game.snapshot().label();
    assert_eq!(
        label,
        "collection_invariant_failed:queue=race_q[0]:fault=cycle:slot=0"
    );
    assert_eq!(validate_label(&label), Ok(LabelVerdict::Broken));
}

#[test]
fn queues_before_game_initialisation_are_absent_and_healthy() {
    let game = GameQueues::new();
    let snapshot = game.snapshot();
    assert_eq!(snapshot.first_fault(), None);
    assert_eq!(snapshot.label(), SOURCE_LABEL);
    assert_eq!(validate_label(&snapshot.label()), Ok(LabelVerdict::Intact));
}

#[test]
fn queue_lifecycle_is_observed_from_init_through_uninit_and_reinit() {
    let mut game = GameQueues::new();
    let which = SelectedQueue::IpGroup;

    game.init(which, 4);
    assert_eq!(observation(&game.snapshot(), which), intact(4, 0, 4, 0));

    game.put(which);
    game.put(which);
    assert_eq!(observation(&game.snapshot(), which), intact(4, 2, 2, 0));

    let held = game.queue(which).alloc_link();
    assert!(!held.is_null());
    assert_eq!(observation(&game.snapshot(), which), intact(4, 2, 1, 1));

    game.queue(which).reinit();
    assert_eq!(observation(&game.snapshot(), which), intact(4, 0, 4, 0));

    game.queue(which).uninit();
    assert_eq!(
        observation(&game.snapshot(), which),
        QueueObservation::Absent
    );
    assert_eq!(game.snapshot().first_fault(), None);

    game.init(which, 2);
    assert_eq!(observation(&game.snapshot(), which), intact(2, 0, 2, 0));
}

#[test]
fn an_exhausted_pool_is_intact_and_a_failed_init_is_absent() {
    let mut game = GameQueues::new();
    let which = SelectedQueue::Encounter;
    game.init(which, 3);
    for _ in 0..3 {
        game.put(which);
    }
    assert!(game.queue(which).alloc_link().is_null());
    assert_eq!(observation(&game.snapshot(), which), intact(3, 3, 0, 0));

    let other = SelectedQueue::BuiltShip;
    assert!(!game.queue(other).init(0, SLOT_SIZE));
    assert_eq!(
        observation(&game.snapshot(), other),
        QueueObservation::Absent
    );
    assert_eq!(game.snapshot().first_fault(), None);
}

struct RemoveAll {
    queue: *mut Queue,
}

extern "C" fn remove_and_free(link: *mut Link, arg: *mut c_void) {
    // SAFETY: the test passes a live `RemoveAll` and the link is a queued slot.
    unsafe {
        let state = &*arg.cast::<RemoveAll>();
        RemoveQueue(state.queue, link.cast());
        FreeLink(state.queue, link.cast());
    }
}

#[test]
fn removing_links_from_a_for_all_links_callback_keeps_the_pool_intact() {
    let mut game = GameQueues::new();
    let which = SelectedQueue::NpcBuiltShip;
    game.init(which, 5);
    for _ in 0..4 {
        game.put(which);
    }
    let queue: *mut Queue = game.queue(which);
    let mut state = RemoveAll { queue };
    // SAFETY: `queue` is a live initialised queue and the callback only
    // removes and frees the link it is given.
    unsafe { ForAllLinks(queue, remove_and_free, (&raw mut state).cast()) };
    assert_eq!(observation(&game.snapshot(), which), intact(5, 0, 5, 0));
    assert_eq!(game.snapshot().first_fault(), None);
}

fn link_of(h: HLink) -> *mut Link {
    h.cast()
}

#[test]
fn a_cycle_in_a_chain_is_the_first_fault_with_its_slot() {
    let mut game = GameQueues::new();
    let which = SelectedQueue::IpGroup;
    game.init(which, 4);
    let a = game.put(which);
    let b = game.put(which);
    // SAFETY: both handles are slots of the pool; the write models C
    // corrupting a `succ` field.
    unsafe { (*link_of(b)).succ = a };
    let snapshot = game.snapshot();
    assert_eq!(snapshot.first_fault(), Some((which, QueueFault::Cycle(0))));
    assert_eq!(
        snapshot.label(),
        "collection_invariant_failed:queue=ip_group_q:fault=cycle:slot=0"
    );
    assert_eq!(validate_label(&snapshot.label()), Ok(LabelVerdict::Broken));
}

#[test]
fn a_broken_pred_chain_names_the_slot_whose_pred_is_wrong() {
    let mut game = GameQueues::new();
    let which = SelectedQueue::Encounter;
    game.init(which, 4);
    game.put(which);
    let b = game.put(which);
    // SAFETY: `b` is a pool slot.
    unsafe { (*link_of(b)).pred = std::ptr::null_mut() };
    assert_eq!(
        game.snapshot().first_fault(),
        Some((which, QueueFault::BrokenPred(1)))
    );
}

#[test]
fn a_chain_that_stops_short_of_the_tail_is_a_broken_tail() {
    let mut game = GameQueues::new();
    let which = SelectedQueue::AvailRace;
    game.init(which, 4);
    let a = game.put(which);
    game.put(which);
    // SAFETY: `a` is a pool slot.
    unsafe { (*link_of(a)).succ = std::ptr::null_mut() };
    assert_eq!(
        game.snapshot().first_fault(),
        Some((which, QueueFault::BrokenTail))
    );
}

#[test]
fn a_link_outside_the_pool_is_a_foreign_link_without_an_address_in_the_label() {
    let mut game = GameQueues::new();
    let which = SelectedQueue::BuiltShip;
    game.init(which, 3);
    let a = game.put(which);
    let mut stray = [Link {
        pred: a,
        succ: std::ptr::null_mut(),
    }];
    // SAFETY: `a` is a pool slot.
    unsafe { (*link_of(a)).succ = stray.as_mut_ptr().cast() };
    let snapshot = game.snapshot();
    assert!(matches!(
        snapshot.first_fault(),
        Some((q, QueueFault::ForeignLink(_))) if q == which
    ));
    assert_eq!(
        snapshot.label(),
        "collection_invariant_failed:queue=built_ship_q:fault=foreign_link"
    );
}

#[test]
fn a_queued_slot_that_is_also_free_is_reported() {
    let mut game = GameQueues::new();
    let which = SelectedQueue::NpcBuiltShip;
    game.init(which, 2);
    let queued = game.put(which);
    let free = game.queue(which).free_list();
    // SAFETY: `free` is a pool slot. Chaining the free list into a queued
    // slot is the double use this invariant exists to catch.
    unsafe { (*link_of(free)).succ = queued };
    assert_eq!(
        game.snapshot().first_fault(),
        Some((which, QueueFault::QueuedAndFree(0)))
    );
}

#[test]
fn with_several_broken_queues_the_first_in_label_order_is_reported() {
    let mut game = GameQueues::new();
    for which in [SelectedQueue::IpGroup, SelectedQueue::BuiltShip] {
        game.init(which, 3);
        let a = game.put(which);
        // SAFETY: `a` is a pool slot.
        unsafe { (*link_of(a)).succ = a };
    }
    let (queue, _) = game.snapshot().first_fault().unwrap();
    assert_eq!(queue, SelectedQueue::IpGroup);
    assert!(game.snapshot().label().contains("queue=ip_group_q"));
}

#[test]
fn healthy_labels_round_trip_through_the_validator() {
    let mut game = GameQueues::new();
    game.init(SelectedQueue::AvailRace, 16);
    game.put(SelectedQueue::AvailRace);
    game.init(SelectedQueue::Encounter, 3);
    let held = game.queue(SelectedQueue::Encounter).alloc_link();
    assert!(!held.is_null());
    let label = game.snapshot().label();
    assert_eq!(
        label,
        "collection_invariants:avail_race_q=intact/16/1/15/0:npc_built_ship_q=absent:ip_group_q=absent:encounter_q=intact/3/0/2/1:built_ship_q=absent:event_q=absent:disp_q=absent:race_q[0]=absent:race_q[1]=absent:master_q=absent"
    );
    assert_eq!(validate_label(&label), Ok(LabelVerdict::Intact));
    assert!(is_collection_label(&label));
}

#[test]
fn the_validator_rejects_labels_the_producer_cannot_write() {
    let healthy = "collection_invariants:avail_race_q=intact/4/1/3/0:npc_built_ship_q=absent:ip_group_q=absent:encounter_q=absent:built_ship_q=absent:event_q=absent:disp_q=absent:race_q[0]=absent:race_q[1]=absent:master_q=absent";
    assert_eq!(validate_label(healthy), Ok(LabelVerdict::Intact));
    for (bad, why) in [
        ("collection_invariants", "no queues"),
        (
            "collection_invariants:npc_built_ship_q=absent:avail_race_q=absent:ip_group_q=absent:encounter_q=absent:built_ship_q=absent:event_q=absent:disp_q=absent:race_q[0]=absent:race_q[1]=absent:master_q=absent",
            "order",
        ),
        (
            "collection_invariants:avail_race_q=intact/4/1/2/0:npc_built_ship_q=absent:ip_group_q=absent:encounter_q=absent:built_ship_q=absent:event_q=absent:disp_q=absent:race_q[0]=absent:race_q[1]=absent:master_q=absent",
            "slots do not add up",
        ),
        (
            "collection_invariants:avail_race_q=intact/0/0/0/0:npc_built_ship_q=absent:ip_group_q=absent:encounter_q=absent:built_ship_q=absent:event_q=absent:disp_q=absent:race_q[0]=absent:race_q[1]=absent:master_q=absent",
            "zero capacity is absent",
        ),
        (
            "collection_invariants:avail_race_q=intact/04/1/3/0:npc_built_ship_q=absent:ip_group_q=absent:encounter_q=absent:built_ship_q=absent:event_q=absent:disp_q=absent:race_q[0]=absent:race_q[1]=absent:master_q=absent",
            "leading zero",
        ),
        (
            "collection_invariants:avail_race_q=intact/256/1/3/0:npc_built_ship_q=absent:ip_group_q=absent:encounter_q=absent:built_ship_q=absent:event_q=absent:disp_q=absent:race_q[0]=absent:race_q[1]=absent:master_q=absent",
            "count above a byte",
        ),
        (
            "collection_invariants:avail_race_q=broken:npc_built_ship_q=absent:ip_group_q=absent:encounter_q=absent:built_ship_q=absent:event_q=absent:disp_q=absent:race_q[0]=absent:race_q[1]=absent:master_q=absent",
            "broken state inside a healthy label",
        ),
        (
            "collection_invariants:avail_race_q=absent:npc_built_ship_q=absent:ip_group_q=absent:encounter_q=absent:built_ship_q=absent:event_q=absent:disp_q=absent:race_q[0]=absent:race_q[1]=absent:master_q=absent:extra=1",
            "trailing field",
        ),
        ("collection_invariant_failed:queue=display_q:fault=cycle:slot=0", "unknown queue"),
        ("collection_invariant_failed:queue=ip_group_q:fault=cycle", "cycle without slot"),
        ("collection_invariant_failed:queue=ip_group_q:fault=broken_tail:slot=1", "slot on slotless fault"),
        ("collection_invariant_failed:queue=ip_group_q:fault=cycle:slot=007", "leading zero slot"),
        ("collection_invariant_failed:queue=ip_group_q:fault=melted", "unknown fault"),
    ] {
        assert!(validate_label(bad).is_err(), "{why}: {bad}");
    }
}

#[test]
fn only_collection_labels_are_claimed_by_the_vocabulary() {
    assert!(is_collection_label("collection_invariants:x"));
    assert!(is_collection_label("collection_invariant_failed:queue=x"));
    assert!(!is_collection_label("collection_invariants_extra:x"));
    assert!(!is_collection_label("battle_frames_verified:count=1"));
    assert!(validate_label("battle_frames_verified:count=1").is_err());
}
