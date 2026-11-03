//! Real queue pools, element fields and the no-journal fighter path.
use super::*;

pub(super) unsafe fn clear_queues() {
    // SAFETY: the caller owns the idle global queues and detaches all test
    // records before releasing their stack storage; there are no active readers.
    unsafe {
        uqm_rust::app::bridge::battle(|battle| {
            battle.reset_element_capacity(150);
            for side in 0..2 {
                battle.reset_ship_capacity(side, 32);
            }
        });
        uqm_rust::battle::process_native::InitDisplayList();
    }
}

pub(super) fn unregistered_hook(
    _: &mut uqm_rust::engine::battle::Cx<'_>,
    _: uqm_rust::engine::battle::ElementId,
) {
}

pub(super) fn elements_and_fighters(directory: &Path) {
    let mut pool = std::array::from_fn::<_, 3, _>(|_| ElementRecord::new());
    // SAFETY: the local contiguous pool stays live until the queues are detached.
    unsafe {
        attach_elements(&mut pool);
    }
    let [first, second, neutral] = &mut pool;
    // SAFETY: STARSHIP contains integers, pointers and optional callbacks.
    let mut owner = unsafe { battle::PoolShip::new(0) };
    owner.SpeciesID = SPECIES_ID_EARTHLING_ID;
    owner.ShipFacing = 31;
    first.player_nr = 0;
    first.state_flags = ElementFlags::PLAYER_SHIP;
    first.parent = uqm_rust::engine::battle::Parent::Ship(owner.ship_id);
    first.current.location.x = 17;
    first.crew_or_hp = 11;

    second.player_nr = 0;
    second.state_flags = ElementFlags::PLAYER_SHIP | ElementFlags::BACKGROUND_OBJECT;
    second.parent = first.parent;
    second.current.location.x = -91;
    second.current.location.y = 43;
    second.crew_or_hp = 7;
    second.preprocess_func = Some(uqm_rust::engine::battle::ElementHook::Native(
        unregistered_hook,
    ));

    neutral.player_nr = -1;
    // SAFETY: attachment publishes these allocated stack slots for this synchronous fixture.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            let source = cx
                .elements()
                .id(neutral.pointer().cast())
                .unwrap_or_else(|| panic!("fixture source"));
            let target = cx
                .elements()
                .id(first.pointer().cast())
                .unwrap_or_else(|| panic!("fixture target"));
            cx.set_target(source, Some(target));
        });
    }
    // SAFETY: stack graph remains live; no renderer or pool allocator is running.
    unsafe {
        install_ship(&mut owner);
        (*uqm_rust::ships::native::abi::GLOBDATA::current())
            .Game_state
            .CurrentActivity = IN_BATTLE as u16;
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
        observation::record(Event::Allocate(element.pointer() as usize));
    }
    let (before, line) = snapshot(directory);
    let mut expected = CrcState::new();
    expected.process_u32(123456);
    for element in [&**first, &**second, &**neutral] {
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
    first: &mut ElementRecord,
    second: &mut ElementRecord,
    neutral: &mut ElementRecord,
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
    first: &mut ElementRecord,
    second: &mut ElementRecord,
    neutral: &mut ElementRecord,
) {
    let native = battle::NativeFrameSet::new(8);
    // SAFETY: the fixture retains this registered drawable through every image observation.
    let (drawable, frame) = unsafe {
        let frame = &mut *native.frame(7).cast::<frame_desc>();
        (&mut *frame.parent, frame)
    };
    let pointer: *mut std::ffi::c_void = ptr::from_mut(frame).cast();
    // SAFETY: native retains this registered drawable through every image observation.
    let view = uqm_rust::engine::battle::views::FrameSelection::retain(pointer.cast());
    let array = uqm_rust::engine::battle::frames::FrameArray::retain([view.view(); 3]);
    let set = uqm_rust::engine::battle::frames::Frames::shared(
        uqm_rust::engine::battle::frames::SharedFrames::Fixture(0),
        array.clone(),
    );
    observation::record(Event::Drawable(ptr::from_mut(drawable) as usize));
    observation::record(Event::Frame {
        address: ptr::from_mut(frame) as usize,
        owner: ptr::from_mut(drawable) as usize,
        index: frame.Index,
    });
    observation::record(Event::FrameSet {
        address: array.as_ptr() as usize,
        frames: vec![pointer as usize; 3],
        role: "test-views".into(),
    });
    observation::assign_frame(&mut first.current.frame, view.clone());
    observation::assign_set(&mut first.current.frames, set.clone(), None);
    observation::assign_frame(&mut first.next.frame, view.clone());
    observation::assign_set(&mut first.next.frames, set.clone(), None);
    observation::assign_frame(&mut first.intersect_control.intersect_stamp.frame, pointer);
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
    observation::assign_frame(
        &mut first.current.frame,
        uqm_rust::engine::battle::frames::FrameSelection::ABSENT,
    );
    observation::assign_set(
        &mut first.current.frames,
        uqm_rust::engine::battle::frames::Frames::ABSENT,
        None,
    );
    observation::assign_frame(
        &mut first.next.frame,
        uqm_rust::engine::battle::frames::FrameSelection::ABSENT,
    );
    observation::assign_set(
        &mut first.next.frames,
        uqm_rust::engine::battle::frames::Frames::ABSENT,
        None,
    );
    observation::assign_frame(
        &mut first.intersect_control.intersect_stamp.frame,
        ptr::null_mut(),
    );
    // SAFETY: both fixture owners retain their slots throughout this read.
    let successor = unsafe { battle::succ(second.pointer()) };
    assert_eq!(successor, neutral.pointer().cast());
}

pub(super) fn fighters_without_journal(directory: &Path) {
    let mut element = ElementRecord::new();
    // SAFETY: zero is valid for all native ship scalar/optional pointer fields.
    let mut ship = unsafe { battle::PoolShip::new(0) };
    ship.descriptor = Some(Box::new(RaceDescriptor::EMPTY));
    ship.ShipFacing = 31;
    element.state_flags = ElementFlags::PLAYER_SHIP;
    element.player_nr = 0;
    element.crew_or_hp = 19;
    element.current.location.x = -9;
    element.parent = uqm_rust::engine::battle::Parent::Ship(ship.ship_id);
    element.preprocess_func = Some(uqm_rust::engine::battle::ElementHook::Native(
        unregistered_hook,
    ));
    // SAFETY: this child has not initialized Coordinator, and it alone owns the graph.
    unsafe {
        (*uqm_rust::ships::native::abi::GLOBDATA::current())
            .Game_state
            .CurrentActivity = IN_BATTLE as u16;
        install_ship(&mut ship);
        attach_elements(std::slice::from_mut(&mut element));
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
        element.parent = uqm_rust::engine::battle::Parent::Ship(ship.ship_id);
        let snapshot = BattleSnapshot::live();
        assert!(snapshot.player.is_none());
        assert!(snapshot.enemy.is_none());
    }
    // SAFETY: detach the stack graph before subsequent tests create a coordinator.
    unsafe {
        clear_queues();
    }
}

pub(super) unsafe fn attach_elements(elements: &mut [ElementRecord]) {
    for element in elements {
        // SAFETY: each owner retains its allocated fixture slot on the game thread.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| cx.put(element.id));
        }
    }
}

pub(super) struct ElementRecord {
    id: uqm_rust::engine::battle::ElementId,
    pointer: *mut Element,
    primitive: u16,
}
impl ElementRecord {
    pub(super) fn new() -> Self {
        // SAFETY: the fixture owns the published context and every allocated operand.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                let id = cx
                    .alloc_element()
                    .unwrap_or_else(|| panic!("fixture capacity"));
                let pointer = cx.elements().raw(id).cast::<Element>();
                let primitive = (*pointer).prim_index;
                pointer.write(Element::new());
                (*pointer).prim_index = primitive;
                Self {
                    id,
                    pointer,
                    primitive,
                }
            })
        }
    }
    pub(super) fn pointer(&self) -> *mut Element {
        self.pointer
    }
}
impl std::ops::Deref for ElementRecord {
    type Target = Element;
    fn deref(&self) -> &Element {
        // SAFETY: this fixture owner retains its slot, and the projection ends before a callback.
        unsafe { &*self.pointer }
    }
}
impl std::ops::DerefMut for ElementRecord {
    fn deref_mut(&mut self) -> &mut Element {
        // SAFETY: fixture edits and context callbacks do not overlap.
        unsafe { &mut *self.pointer }
    }
}
impl Drop for ElementRecord {
    fn drop(&mut self) {
        // SAFETY: the owning serialized fixture outlives its allocated operands.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                if cx.elements().contains(self.id) {
                    (*self.pointer).prim_index = self.primitive;
                    if cx.elements().linked(self.id) {
                        cx.unlink(self.id);
                    }
                    cx.dispose(self.id);
                }
            });
        }
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
        let queue = &raw mut (*uqm_rust::ships::native::abi::GLOBDATA::current())
            .Game_state
            .GameClock
            .event_q;
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
    let mut element = ElementRecord::new();
    // SAFETY: no game loop is running; this test owns both queues and the group pool.
    unsafe {
        let queue = &raw mut (*uqm_rust::ships::native::abi::GLOBDATA::current())
            .Game_state
            .ip_group_q;
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
        element.parent = uqm_rust::engine::battle::Parent::IpGroup(0);
        attach_elements(std::slice::from_mut(&mut element));
        observation::record(Event::Allocate(element.pointer() as usize));
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
