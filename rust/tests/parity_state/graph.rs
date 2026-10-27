//! Real queue pools, element fields and the no-journal fighter path.
use super::*;

pub(super) unsafe fn clear_queues() {
    // SAFETY: the caller owns the idle global queues and detaches all test
    // records before releasing their stack storage; there are no active readers.
    unsafe {
        race_q[0].head = ptr::null_mut();
        race_q[0].tail = ptr::null_mut();
        race_q[1].head = ptr::null_mut();
        race_q[1].tail = ptr::null_mut();
        disp_q.head = ptr::null_mut();
        disp_q.tail = ptr::null_mut();
        disp_q.pq_tab = ptr::null_mut();
        race_q[0].pq_tab = ptr::null_mut();
        race_q[1].pq_tab = ptr::null_mut();
    }
}

pub(super) unsafe extern "C" fn unregistered_hook(_element: *mut Element) {}

pub(super) fn elements_and_fighters(directory: &Path) {
    let mut pool = [Element::new(); 3];
    // SAFETY: the local contiguous pool stays live until the queues are detached.
    unsafe {
        attach_elements(&mut pool);
    }
    let [first, second, neutral] = &mut pool;
    // SAFETY: STARSHIP contains integers, pointers and optional callbacks.
    let mut owner: STARSHIP = unsafe { std::mem::zeroed() };
    owner.SpeciesID = SPECIES_ID_EARTHLING_ID;
    owner.ShipFacing = 31;
    first.player_nr = 0;
    first.state_flags = ElementFlags::PLAYER_SHIP;
    first.p_parent = ptr::from_mut(&mut owner).cast();
    first.current.location.x = 17;
    first.crew_or_hp = 11;
    first.succ = &mut *second;
    second.pred = &mut *first;
    second.succ = &mut *neutral;
    second.player_nr = 0;
    second.state_flags = ElementFlags::PLAYER_SHIP | ElementFlags::BACKGROUND_OBJECT;
    second.p_parent = first.p_parent;
    second.current.location.x = -91;
    second.current.location.y = 43;
    second.crew_or_hp = 7;
    second.preprocess_func = Some(unregistered_hook);
    neutral.pred = &mut *second;
    neutral.player_nr = -1;
    neutral.h_target = &mut *first;
    // SAFETY: stack graph remains live; no renderer or pool allocator is running.
    unsafe {
        install_ship(&mut owner);
        disp_q.head = ptr::from_mut(&mut *first).cast();
        disp_q.tail = ptr::from_mut(&mut *neutral).cast();
        GlobData.Game_state.CurrentActivity = IN_BATTLE as u16;
    }
    uqm_rust::math::seed_random(654321);
    let fighters = BattleSnapshot::live();
    assert_eq!(uqm_rust::math::seed_random(654321), 654321);
    let player = fighters
        .player
        .unwrap_or_else(|| panic!("missing live player ship"));
    assert_eq!(player.x, -91, "last matching ship wins");
    assert_eq!(player.facing, 15);
    assert_eq!(player.crew, 7);
    assert!(fighters.enemy.is_none());
    second.preprocess_func = None;
    for element in [&mut *first, &mut *second, &mut *neutral] {
        observation::record(Event::Allocate(ptr::from_mut(element) as usize));
    }
    let (before, line) = snapshot(directory);
    let mut expected = CrcState::new();
    expected.process_u32(123456);
    for element in [&*first, &*second, &*neutral] {
        expected.process_element(element);
    }
    assert_eq!(
        line.split_whitespace()
            .nth(3)
            .unwrap_or_else(|| panic!("checkpoint trace has no CRC")),
        format!("{:08x}", expected.finish())
    );
    assert_eq!(before["elements.0.current.x"], "17");
    assert_eq!(before["elements.1.current.x"], "-91");
    assert_eq!(before["elements.2.side"], "-1");
    assert_eq!(before["elements.0.succ"], before["elements.1.identity"]);
    assert_eq!(before["elements.2.target"], before["elements.0.identity"]);
    element_mutations(directory, &mut *first, &mut *second, &mut *neutral, &before);
    // SAFETY: detach all stack graph nodes before returning.
    unsafe {
        clear_queues();
    }
}

pub(super) fn element_mutations(
    directory: &Path,
    first: &mut Element,
    second: &mut Element,
    neutral: &mut Element,
    before: &Fields,
) {
    first.life_span = 19;
    first.mass_points = 3;
    first.turn_wait = 4;
    first.thrust_or_blast = 5;
    first.color_cycle_index = 6;
    first.prim_index = 7;
    first.next.location.x = -13;
    first.next.location.y = 29;
    first.intersect_control.last_time_val = 17;
    first.intersect_control.intersect_stamp.origin.x = -23;
    first.intersect_control.intersect_stamp.origin.y = 31;
    first.intersect_control.end_point.x = -37;
    first.intersect_control.end_point.y = 41;
    first.velocity.travel_angle = 43;
    first.velocity.vector.width = -47;
    first.velocity.vector.height = 53;
    first.velocity.fract.width = -59;
    first.velocity.fract.height = 61;
    first.velocity.error.width = -67;
    first.velocity.error.height = 71;
    first.velocity.incr.width = -73;
    first.velocity.incr.height = 79;
    let (after, _) = snapshot(directory);
    assert_eq!(changed(before, &after).len(), 22);
    assert_eq!(after["elements.0.velocity.incr.width"], "-73");
    assert_eq!(after["elements.0.intersect.origin.x"], "-23");
    frame_fields(directory, first, second, neutral);
}

pub(super) fn frame_fields(
    directory: &Path,
    first: &mut Element,
    second: &mut Element,
    neutral: &mut Element,
) {
    // SAFETY: drawable_desc contains integers, pointers and optional callbacks.
    let mut drawable: drawable_desc = unsafe { std::mem::zeroed() };
    // SAFETY: frame_desc contains integers, pointers and optional callbacks.
    let mut frame: frame_desc = unsafe { std::mem::zeroed() };
    frame.parent = &mut drawable;
    frame.Index = 7;
    let pointer = ptr::from_mut(&mut frame).cast();
    let mut array = [pointer; 3];
    first.current.frame = pointer;
    first.current.farray = array.as_mut_ptr();
    first.next.frame = pointer;
    first.next.farray = array.as_mut_ptr();
    first.intersect_control.intersect_stamp.frame = pointer;
    observation::record(Event::Drawable(ptr::from_mut(&mut drawable) as usize));
    observation::record(Event::Frame {
        address: ptr::from_mut(&mut frame) as usize,
        owner: ptr::from_mut(&mut drawable) as usize,
        index: frame.Index,
    });
    let (fields, _) = snapshot(directory);
    for name in [
        "current.frame",
        "next.frame",
        "intersect.frame",
        "current.farray",
        "next.farray",
    ] {
        assert_eq!(
            fields[&format!("elements.0.{name}.owner")],
            fields["elements.0.current.frame.owner"]
        );
        assert_eq!(fields[&format!("elements.0.{name}.index")], "7");
    }
    assert_eq!(fields["elements.1.current.frame.owner"], "none");
    assert_eq!(fields["elements.2.current.farray.index"], "0");
    first.current.frame = ptr::null_mut();
    first.current.farray = ptr::null_mut();
    first.next.frame = ptr::null_mut();
    first.next.farray = ptr::null_mut();
    first.intersect_control.intersect_stamp.frame = ptr::null_mut();
    // Keep node borrows explicit until the last graph read.
    assert_eq!(second.succ, ptr::from_mut(neutral));
}

pub(super) fn fighters_without_journal(directory: &Path) {
    let mut element = Element::new();
    // SAFETY: zero is valid for all native ship scalar/optional pointer fields.
    let mut ship: STARSHIP = unsafe { std::mem::zeroed() };
    ship.RaceDescPtr = ptr::dangling_mut();
    ship.ShipFacing = 31;
    element.state_flags = ElementFlags::PLAYER_SHIP;
    element.player_nr = 0;
    element.crew_or_hp = 19;
    element.current.location.x = -9;
    element.p_parent = ptr::from_mut(&mut ship).cast();
    element.preprocess_func = Some(unregistered_hook);
    // SAFETY: this child has not initialized Coordinator, and it alone owns the graph.
    unsafe {
        GlobData.Game_state.CurrentActivity = IN_BATTLE as u16;
        attach_elements(std::slice::from_mut(&mut element));
        disp_q.head = ptr::from_mut(&mut element).cast();
        disp_q.tail = disp_q.head;
    }
    uqm_rust::math::seed_random(56789);
    let snapshot = uqm_rust::automation::battle_fight::BattleSnapshot::live();
    let player = snapshot
        .player
        .unwrap_or_else(|| panic!("missing no-journal player ship"));
    assert_eq!(player.facing, 15);
    assert_eq!(player.crew, 19);
    assert_eq!(player.x, -9);
    assert!(snapshot.enemy.is_none());
    assert_eq!(uqm_rust::math::seed_random(56789), 56789);
    assert!(!directory.join("parity.fields").exists());
    let element = &mut element;
    for side in [-1, 2, i16::MAX] {
        element.player_nr = side;
        element.p_parent = ptr::dangling_mut();
        let snapshot = BattleSnapshot::live();
        assert!(snapshot.player.is_none());
        assert!(snapshot.enemy.is_none());
    }
    // SAFETY: detach the stack graph before subsequent tests create a coordinator.
    unsafe {
        clear_queues();
    }
}

pub(super) unsafe fn attach_elements(elements: &mut [Element]) {
    let base = elements.as_mut_ptr();
    for (index, element) in elements.iter_mut().enumerate() {
        element.pred = if index == 0 {
            ptr::null_mut()
        } else {
            // SAFETY: index is nonzero and strictly inside the borrowed slice.
            unsafe { base.add(index - 1) }
        };
        // SAFETY: this is an in-bounds or one-past pointer; the final successor
        // is reset to null before any queue traversal can read it.
        element.succ = unsafe { base.add(index + 1) };
    }
    elements
        .last_mut()
        .unwrap_or_else(|| panic!("test element pool is empty"))
        .succ = ptr::null_mut();
    // SAFETY: the caller owns the idle queue and keeps the pool live at this
    // address until clear_queues detaches it after the synchronous readers finish.
    unsafe {
        disp_q.pq_tab = base.cast();
        disp_q.object_size = std::mem::size_of::<Element>() as u16;
        disp_q.num_objects = elements.len() as u8;
        disp_q.head = base.cast();
        disp_q.tail = ptr::from_mut(
            elements
                .last_mut()
                .unwrap_or_else(|| panic!("test element pool is empty")),
        )
        .cast();
    }
}

pub(super) fn event_queue_fields(directory: &Path) {
    use uqm_rust::{
        collections::queue::{InitQueue, UninitQueue},
        savegame::{runtime::events::restore_events, types::EventEntry},
        time::clock_bridge::{CEvent, NUM_EVENTS},
    };
    // SAFETY: this test owns the idle game clock queue until it is uninitialized.
    unsafe {
        let queue = &raw mut GlobData.Game_state.GameClock.event_q;
        assert!(InitQueue(
            queue,
            NUM_EVENTS as u16,
            std::mem::size_of::<CEvent>() as u16
        ));
        let first = EventEntry {
            day_index: 8,
            month_index: 9,
            year_index: 2161,
            func_index: 1,
        };
        let second = EventEntry {
            day_index: 10,
            month_index: 9,
            year_index: 2161,
            func_index: 2,
        };
        restore_events(&mut *queue, &[first, second])
            .unwrap_or_else(|error| panic!("restore test events: {error}"));
        let (fields, _) = snapshot(directory);
        assert_eq!(fields["mode.events.initialized"], "true");
        assert_eq!(fields["mode.events.count"], "2");
        for (index, day, function) in [(0, 8, 1), (1, 10, 2)] {
            assert_eq!(fields[&format!("mode.events.{index}.day")], day.to_string());
            assert_eq!(fields[&format!("mode.events.{index}.month")], "9");
            assert_eq!(fields[&format!("mode.events.{index}.year")], "2161");
            assert_eq!(
                fields[&format!("mode.events.{index}.function")],
                function.to_string()
            );
        }
        assert!(UninitQueue(queue));
    }
}

pub(super) fn non_ship_parent_fields(directory: &Path) {
    use uqm_rust::{
        collections::queue::{AllocLink, InitQueue, PutQueue, UninitQueue},
        savegame::runtime::groups::CIpGroup,
    };
    let mut element = Element::new();
    // SAFETY: no game loop is running; this test owns both queues and the group pool.
    unsafe {
        let queue = &raw mut GlobData.Game_state.ip_group_q;
        assert!(InitQueue(queue, 4, std::mem::size_of::<CIpGroup>() as u16));
        let handle = AllocLink(queue);
        assert!(!handle.is_null());
        handle.cast::<CIpGroup>().write(std::mem::zeroed());
        let group = &mut *handle.cast::<CIpGroup>();
        group.group_id = 31;
        group.race_id = 4;
        group.group_counter = 123;
        group.loc.x = -71;
        group.task = 2;
        PutQueue(queue, handle);
        element.p_parent = handle;
        attach_elements(std::slice::from_mut(&mut element));
        disp_q.head = ptr::from_mut(&mut element).cast();
        disp_q.tail = disp_q.head;
        observation::record(Event::Allocate(ptr::from_mut(&mut element) as usize));
        let callback = uqm_rust::ships::hook_registry::hooks()
            .into_iter()
            .find(|(_, name)| *name == "battle::ship_native::collision")
            .unwrap_or_else(|| panic!("collision callback is absent from the registry"))
            .0;
        observation::record(Event::Hook {
            parent: handle as usize,
            callback,
        });
        let (fields, line) = snapshot(directory);
        assert_eq!(fields["elements.0.parent"], "group:31");
        assert_eq!(fields["mode.groups.31.counter"], "123");
        assert_eq!(fields["mode.groups.31.x"], "-71");
        assert!(line.contains("[hook 5 battle::ship_native::collision]"));
        (*handle.cast::<CIpGroup>()).group_id = 32;
        let (fields, _) = snapshot(directory);
        assert_eq!(fields["elements.0.parent"], "group:32");
        assert_eq!(fields["mode.groups.32.counter"], "123");
        clear_queues();
        assert!(UninitQueue(queue));
    }
}
