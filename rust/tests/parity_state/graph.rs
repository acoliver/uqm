//! Real queue pools, element fields and the no-journal fighter path.
use super::*;
use uqm_rust::battle::ContextOps;

pub(super) unsafe fn clear_queues() {
    // SAFETY: the caller owns the idle global queues and detaches all test
    // records before releasing their stack storage; there are no active readers.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            cx.kernel_mut().reset_element_capacity(150);
            for side in 0..2 {
                cx.kernel_mut().reset_ship_capacity(side, 32);
            }
        });
        uqm_rust::app::bridge::enter(|cx| {
            let scaled = cx.kernel_mut().configuration.preferences().melee_scale == 0;
            uqm_rust::battle::process_native::init_display_list(cx, scaled);
        });
    }
}

pub(super) fn unregistered_hook(
    _: &mut uqm_rust::engine::battle::Cx<'_>,
    _: uqm_rust::engine::battle::ElementId,
) {
}

pub(super) fn elements_and_fighters(observer: &AutomationObserver, directory: &Path) {
    let mut pool = std::array::from_fn::<_, 3, _>(|_| ElementRecord::new());
    // SAFETY: the local contiguous pool stays live until the queues are detached.
    unsafe {
        attach_elements(&mut pool);
    }
    let [first, second, neutral] = &mut pool;
    // SAFETY: STARSHIP contains integers, pointers and optional callbacks.
    let mut owner = unsafe { battle::PoolShip::new(0) };
    owner.SpeciesID = SPECIES_ID_EARTHLING_ID;
    owner.ship_facing = 31;
    first.playerNr = 0;
    first.state_flags = ElementFlags::PLAYER_SHIP.bits();
    first.parent = uqm_rust::engine::battle::Parent::Ship(owner.ship_id);
    first.current.location.x = 17;
    first.hits = 11;

    second.playerNr = 0;
    second.state_flags = (ElementFlags::PLAYER_SHIP | ElementFlags::BACKGROUND_OBJECT).bits();
    second.parent = first.parent;
    second.current.location.x = -91;
    second.current.location.y = 43;
    second.hits = 7;
    second.preprocess_func = Some(uqm_rust::engine::battle::ElementHook::Native(
        unregistered_hook,
    ));

    neutral.playerNr = -1;
    // SAFETY: attachment publishes these allocated stack slots for this synchronous fixture.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            let source = cx
                .elements()
                .id(std::ptr::NonNull::new(neutral.pointer().cast()))
                .unwrap_or_else(|| panic!("fixture source"));
            let target = cx
                .elements()
                .id(std::ptr::NonNull::new(first.pointer().cast()))
                .unwrap_or_else(|| panic!("fixture target"));
            cx.set_target(source, Some(target));
        });
    }
    // SAFETY: stack graph remains live; no renderer or pool allocator is running.
    let campaign = unsafe {
        install_ship(&mut owner);
        uqm_rust::engine::World::current().write(|world| {
            world.set_activity(IN_BATTLE as u16);
            uqm_rust::engine::observe::CampaignView::from_world(world)
        })
    };
    uqm_rust::math::seed_random(654321);
    let fighters = BattleSnapshot::live(&campaign);
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
    let (before, line) = snapshot(observer, directory);
    let mut expected = CrcState::new();
    expected.process_u32(123456);
    for element in [&**first, &**second, &**neutral] {
        expected.process_element(&Element::from(element));
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
    element_mutations(
        observer,
        directory,
        &mut *first,
        &mut *second,
        &mut *neutral,
        &before,
    );
    // SAFETY: detach all stack graph nodes before returning.
    unsafe {
        clear_queues();
    }
}

pub(super) fn element_mutations(
    observer: &AutomationObserver,
    directory: &Path,
    first: &mut ElementRecord,
    second: &mut ElementRecord,
    neutral: &mut ElementRecord,
    before: &Fields,
) {
    first.life = 19;
    first.mass = 3;
    first.turn = 4;
    first.thrust = 5;
    first.colorCycleIndex = 6;
    first.PrimIndex = 7;
    first.next.location.x = -13;
    first.next.location.y = 29;
    first.IntersectControl.last_time_val = 17;
    first.IntersectControl.IntersectStamp.origin.x = -23;
    first.IntersectControl.IntersectStamp.origin.y = 31;
    first.IntersectControl.EndPoint.x = -37;
    first.IntersectControl.EndPoint.y = 41;
    first.velocity.travel_angle = 43;
    first.velocity.vector.width = -47;
    first.velocity.vector.height = 53;
    first.velocity.fract.width = -59;
    first.velocity.fract.height = 61;
    first.velocity.error.width = -67;
    first.velocity.error.height = 71;
    first.velocity.incr.width = -73;
    first.velocity.incr.height = 79;
    let (after, _) = snapshot(observer, directory);
    assert_eq!(changed(before, &after).len(), 22);
    assert_eq!(after["elements.0.velocity.incr.width"], "-73");
    assert_eq!(after["elements.0.intersect.origin.x"], "-23");
    frame_fields(observer, directory, first, second, neutral);
}

pub(super) fn frame_fields(
    observer: &AutomationObserver,
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
    observation::assign_frame(&mut first.current.image.frame, view.clone());
    observation::assign_set(&mut first.current.image.frames, set.clone(), None);
    observation::assign_frame(&mut first.next.image.frame, view.clone());
    observation::assign_set(&mut first.next.image.frames, set.clone(), None);
    observation::assign_frame(
        &mut first.IntersectControl.IntersectStamp.frame,
        pointer.cast(),
    );
    let (fields, _) = snapshot(observer, directory);
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
        &mut first.current.image.frame,
        uqm_rust::engine::battle::frames::FrameSelection::ABSENT,
    );
    observation::assign_set(
        &mut first.current.image.frames,
        uqm_rust::engine::battle::frames::Frames::ABSENT,
        None,
    );
    observation::assign_frame(
        &mut first.next.image.frame,
        uqm_rust::engine::battle::frames::FrameSelection::ABSENT,
    );
    observation::assign_set(
        &mut first.next.image.frames,
        uqm_rust::engine::battle::frames::Frames::ABSENT,
        None,
    );
    observation::assign_frame(
        &mut first.IntersectControl.IntersectStamp.frame,
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
    ship.ship_facing = 31;
    element.state_flags = ElementFlags::PLAYER_SHIP.bits();
    element.playerNr = 0;
    element.hits = 19;
    element.current.location.x = -9;
    element.parent = uqm_rust::engine::battle::Parent::Ship(ship.ship_id);
    element.preprocess_func = Some(uqm_rust::engine::battle::ElementHook::Native(
        unregistered_hook,
    ));
    // SAFETY: this child has not initialized Coordinator, and it alone owns the graph.
    let campaign = unsafe {
        let campaign = uqm_rust::engine::World::current().write(|world| {
            world.set_activity(IN_BATTLE as u16);
            uqm_rust::engine::observe::CampaignView::from_world(world)
        });
        install_ship(&mut ship);
        attach_elements(std::slice::from_mut(&mut element));
        campaign
    };
    uqm_rust::math::seed_random(56789);
    let snapshot = uqm_rust::automation::battle_fight::BattleSnapshot::live(&campaign);
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
        element.playerNr = side;
        element.parent = uqm_rust::engine::battle::Parent::Ship(ship.ship_id);
        let snapshot = BattleSnapshot::live(&campaign);
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
    pointer: *mut uqm_rust::engine::battle::record::ElementRecord,
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
                let pointer = cx
                    .elements()
                    .raw(id)
                    .cast::<uqm_rust::engine::battle::record::ElementRecord>();
                let primitive = (*pointer).PrimIndex;
                pointer.write(uqm_rust::engine::battle::record::ElementRecord::default());
                (*pointer).playerNr = -1;
                (*pointer).PrimIndex = primitive;
                Self {
                    id,
                    pointer,
                    primitive,
                }
            })
        }
    }
    pub(super) fn pointer(&self) -> *mut uqm_rust::engine::battle::record::ElementRecord {
        self.pointer
    }
}
impl std::ops::Deref for ElementRecord {
    type Target = uqm_rust::engine::battle::record::ElementRecord;
    fn deref(&self) -> &Self::Target {
        // SAFETY: this fixture owner retains its slot, and the projection ends before a callback.
        unsafe { &*self.pointer }
    }
}
impl std::ops::DerefMut for ElementRecord {
    fn deref_mut(&mut self) -> &mut Self::Target {
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
                    (*self.pointer).PrimIndex = self.primitive;
                    if cx.elements().linked(self.id) {
                        cx.unlink(self.id);
                    }
                    cx.dispose(self.id);
                }
            });
        }
    }
}

pub(super) fn event_queue_fields(observer: &AutomationObserver, directory: &Path) {
    use uqm_rust::savegame::{runtime::events::restore_events, types::EventEntry};

    uqm_rust::engine::World::current().write(|world| {
        world.clock.init();
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
        restore_events(&mut world.clock, &[first, second])
            .unwrap_or_else(|error| panic!("restore test events: {error}"));
    });
    let (fields, _) = snapshot(observer, directory);
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
    uqm_rust::engine::World::current().write(|world| world.clock.uninit());
}

pub(super) fn non_ship_parent_fields(observer: &AutomationObserver, directory: &Path) {
    let mut element = ElementRecord::new();
    // SAFETY: no game loop is running; this test owns both queues and the group pool.
    unsafe {
        let (group_id, handle) = uqm_rust::engine::World::current().write(|world| {
            let mut queue = world.ip_groups();
            let id = queue
                .alloc()
                .unwrap_or_else(|| panic!("fixture pool capacity"));
            let group = queue.get_mut(id);
            group.group_id = 31;
            group.race_id = 4;
            group.group_counter = 123;
            group.loc.x = -71;
            group.task = 2;
            queue.put(id);
            (id, queue.raw(id))
        });
        element.parent = uqm_rust::engine::battle::Parent::IpGroup(group_id);
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
        let (fields, line) = snapshot(observer, directory);
        assert_eq!(fields["elements.0.parent"], "group:31");
        assert_eq!(fields["mode.groups.31.counter"], "123");
        assert_eq!(fields["mode.groups.31.x"], "-71");
        assert!(line.contains("[hook 5 battle::ship_native::collision]"));
        (*handle.cast::<uqm_rust::engine::world::queues::IpGroup>()).group_id = 32;
        let (fields, _) = snapshot(observer, directory);
        assert_eq!(fields["elements.0.parent"], "group:32");
        assert_eq!(fields["mode.groups.32.counter"], "123");
        clear_queues();
        uqm_rust::engine::World::current().write(|world| world.ip_groups().reinit());
    }
}
