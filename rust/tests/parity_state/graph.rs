//! Real queue pools, element fields and the no-journal fighter path.
use super::*;
use uqm_rust::collections::observation;
use uqm_rust::collections::observation::Event;
use uqm_rust::engine::battle::record::RaceDescriptor;
use uqm_rust::engine::species::species_id::SPECIES_ID_EARTHLING_ID;
use uqm_rust::engine::world::activity::activity_flags::IN_BATTLE;

pub(super) fn clear_queues() {
    // records before releasing their stack storage; there are no active readers.
    {
        crate::battle_context::with(|cx| {
            cx.kernel_mut().reset_element_capacity(150);
            for side in 0..2 {
                cx.kernel_mut().reset_ship_capacity(side, 32);
            }
        });
        crate::battle_context::with(|cx| {
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
    {
        attach_elements(&mut pool);
    }
    let [first, second, neutral] = &mut pool;
    let mut owner = battle::PoolShip::new(0);
    owner.record().species_id = SPECIES_ID_EARTHLING_ID;
    owner.record().ship_facing = 31;
    first.record().player_nr = 0;
    first.record().state_flags = ElementFlags::PLAYER_SHIP.bits();
    first.record().parent = uqm_rust::engine::battle::Parent::Ship(owner.ship_id);
    first.record().current.location.x = 17;
    first.record().hits = 11;

    second.record().player_nr = 0;
    second.record().state_flags =
        (ElementFlags::PLAYER_SHIP | ElementFlags::BACKGROUND_OBJECT).bits();
    second.record().parent = first.get().parent;
    second.record().current.location.x = -91;
    second.record().current.location.y = 43;
    second.record().hits = 7;
    second.record().preprocess_func = Some(uqm_rust::engine::battle::ElementHook::Native(
        unregistered_hook,
    ));

    neutral.record().player_nr = -1;
    crate::battle_context::with(|cx| cx.set_target(neutral.id, Some(first.id)));
    let campaign = {
        install_ship(&mut owner);
        crate::battle_context::with(|cx| {
            let world = &mut cx.world;
            world.set_activity(IN_BATTLE);
            uqm_rust::engine::observe::CampaignView::from_world(world)
        })
    };
    uqm_rust::math::seed_random(654321);
    let fighters = crate::battle_context::with(|cx| BattleSnapshot::live(cx, &campaign));
    assert_eq!(uqm_rust::math::seed_random(654321), 654321);
    let player = fighters
        .player
        .unwrap_or_else(|| panic!("missing live player ship"));
    assert_eq!(player.x, -91, "last matching ship wins");
    assert_eq!(player.facing, 15);
    assert_eq!(player.crew, 7);
    assert!(fighters.enemy.is_none());
    second.record().preprocess_func = None;
    for element in [&*first, &*second, &*neutral] {
        observation::record(Event::Allocate(element.address()));
    }
    let (before, line) = snapshot(observer, directory);
    let mut expected = CrcState::new();
    expected.process_u32(123456);
    for element in [&*first, &*second, &*neutral] {
        expected.process_element(&Element::from(&*element.get()));
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
    {
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
    first.record().life = 19;
    first.record().mass = 3;
    first.record().turn = 4;
    first.record().thrust = 5;
    first.record().color_cycle_index = 6;
    first.record().prim_index = 7;
    first.record().next.location.x = -13;
    first.record().next.location.y = 29;
    first.record().intersect_control.last_time_val = 17;
    first.record().intersect_control.intersect_stamp.origin.x = -23;
    first.record().intersect_control.intersect_stamp.origin.y = 31;
    first.record().intersect_control.end_point.x = -37;
    first.record().intersect_control.end_point.y = 41;
    first.record().velocity.travel_angle = 43;
    first.record().velocity.vector.width = -47;
    first.record().velocity.vector.height = 53;
    first.record().velocity.fract.width = -59;
    first.record().velocity.fract.height = 61;
    first.record().velocity.error.width = -67;
    first.record().velocity.error.height = 71;
    first.record().velocity.incr.width = -73;
    first.record().velocity.incr.height = 79;
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
    let pointer = native.frame(7);
    let drawable = pointer.drawable().map_or(0, |drawable| drawable.bits());
    let view = uqm_rust::engine::battle::views::FrameSelection::retain(pointer);
    let array = uqm_rust::engine::battle::frames::FrameArray::retain([view.view(); 3]);
    let set = uqm_rust::engine::battle::frames::Frames::shared(
        uqm_rust::engine::battle::frames::SharedFrames::Fixture(0),
        array.clone(),
    );
    observation::record(Event::Drawable(drawable));
    observation::record(Event::Frame {
        address: pointer.bits(),
        owner: drawable,
        index: pointer.index(),
    });
    observation::record(Event::FrameSet {
        address: array.as_ptr() as usize,
        frames: vec![pointer.bits(); 3],
        role: "test-views".into(),
    });
    observation::assign_frame(&mut first.record().current.image.frame, view.clone());
    observation::assign_set(&mut first.record().current.image.frames, set.clone(), None);
    observation::assign_frame(&mut first.record().next.image.frame, view.clone());
    observation::assign_set(&mut first.record().next.image.frames, set.clone(), None);
    observation::assign_frame(
        &mut first.record().intersect_control.intersect_stamp.frame,
        pointer,
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
        &mut first.record().current.image.frame,
        uqm_rust::engine::battle::frames::FrameSelection::ABSENT,
    );
    observation::assign_set(
        &mut first.record().current.image.frames,
        uqm_rust::engine::battle::frames::Frames::ABSENT,
        None,
    );
    observation::assign_frame(
        &mut first.record().next.image.frame,
        uqm_rust::engine::battle::frames::FrameSelection::ABSENT,
    );
    observation::assign_set(
        &mut first.record().next.image.frames,
        uqm_rust::engine::battle::frames::Frames::ABSENT,
        None,
    );
    observation::assign_frame(
        &mut first.record().intersect_control.intersect_stamp.frame,
        uqm_rust::graphics::drawable::FrameId::NULL,
    );
    assert_eq!(battle::succ(second.id), Some(neutral.id));
}

pub(super) fn fighters_without_journal(directory: &Path) {
    let mut element = ElementRecord::new();
    let mut ship = battle::PoolShip::new(0);
    ship.record().descriptor = Some(Box::new(RaceDescriptor::EMPTY));
    ship.record().ship_facing = 31;
    element.record().state_flags = ElementFlags::PLAYER_SHIP.bits();
    element.record().player_nr = 0;
    element.record().hits = 19;
    element.record().current.location.x = -9;
    element.record().parent = uqm_rust::engine::battle::Parent::Ship(ship.ship_id);
    element.record().preprocess_func = Some(uqm_rust::engine::battle::ElementHook::Native(
        unregistered_hook,
    ));
    let campaign = {
        let campaign = crate::battle_context::with(|cx| {
            let world = &mut cx.world;
            world.set_activity(IN_BATTLE);
            uqm_rust::engine::observe::CampaignView::from_world(world)
        });
        install_ship(&mut ship);
        attach_elements(std::slice::from_mut(&mut element));
        campaign
    };
    uqm_rust::math::seed_random(56789);
    let snapshot = crate::battle_context::with(|cx| {
        uqm_rust::automation::battle_fight::BattleSnapshot::live(cx, &campaign)
    });
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
        element.record().player_nr = side;
        element.record().parent = uqm_rust::engine::battle::Parent::Ship(ship.ship_id);
        let snapshot = crate::battle_context::with(|cx| BattleSnapshot::live(cx, &campaign));
        assert!(snapshot.player.is_none());
        assert!(snapshot.enemy.is_none());
    }
    {
        clear_queues();
    }
}

pub(super) fn attach_elements(elements: &mut [ElementRecord]) {
    for element in elements {
        {
            crate::battle_context::with(|cx| cx.put(element.id));
        }
    }
}

/// An element slot this test resets to an empty record and links by hand.
pub(super) struct ElementRecord {
    pub(super) id: uqm_rust::engine::battle::ElementId,
    primitive: u16,
}
impl ElementRecord {
    pub(super) fn new() -> Self {
        let id = crate::battle_context::with(|cx| cx.alloc_element())
            .unwrap_or_else(|| panic!("fixture capacity"));
        let mut record = battle::get_mut(id);
        let primitive = record.prim_index;
        *record = uqm_rust::engine::battle::record::ElementRecord::default();
        record.player_nr = -1;
        record.prim_index = primitive;
        Self { id, primitive }
    }
    /// Lend the live record until the guard drops.
    pub(super) fn record(
        &mut self,
    ) -> std::cell::RefMut<'static, uqm_rust::engine::battle::record::ElementRecord> {
        battle::get_mut(self.id)
    }
    pub(super) fn get(&self) -> battle::Snapshot<uqm_rust::engine::battle::record::ElementRecord> {
        battle::get(self.id)
    }
    /// The pool address the parity observer identifies this element by.
    pub(super) fn address(&self) -> usize {
        std::ptr::from_ref(&*battle::get_mut(self.id)) as usize
    }
}
impl Drop for ElementRecord {
    fn drop(&mut self) {
        let live = crate::battle_context::with(|cx| cx.elements().contains(self.id));
        if live {
            battle::get_mut(self.id).prim_index = self.primitive;
            crate::battle_context::with(|cx| {
                if cx.elements().linked(self.id) {
                    uqm_rust::battle::sounds::engine::unlink(cx, self.id);
                }
                cx.dispose(self.id);
            });
        }
    }
}

pub(super) fn event_queue_fields(observer: &AutomationObserver, directory: &Path) {
    use uqm_rust::savegame::{runtime::events::restore_events, types::EventEntry};

    crate::battle_context::with(|cx| {
        cx.world.clock.init();
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
        restore_events(&mut cx.world.clock, &[first, second])
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
    crate::battle_context::with(|cx| cx.world.clock.uninit());
}

pub(super) fn non_ship_parent_fields(observer: &AutomationObserver, directory: &Path) {
    let mut element = ElementRecord::new();
    {
        let (group_id, handle) = crate::battle_context::with(|cx| {
            let mut queue = cx.world.ip_groups();
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
        element.record().parent = uqm_rust::engine::battle::Parent::IpGroup(group_id);
        attach_elements(std::slice::from_mut(&mut element));
        observation::record(Event::Allocate(element.address()));
        let callback = uqm_rust::battle::hook_registry::hooks()
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
        crate::battle_context::with(|cx| cx.world.ip_groups().get_mut(group_id).group_id = 32);
        let (fields, _) = snapshot(observer, directory);
        assert_eq!(fields["elements.0.parent"], "group:32");
        assert_eq!(fields["mode.groups.32.counter"], "123");
        clear_queues();
        crate::battle_context::with(|cx| cx.world.ip_groups().reinit());
    }
}
