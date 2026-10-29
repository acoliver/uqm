//! Synchronous game-thread graph borrowing and journal orchestration.
use super::Fields;
#[cfg(not(test))]
use super::{custom, elements, ships::Ship, value};
use crate::engine::battle::access::observe::ElementObservation;
#[cfg(not(test))]
use crate::{
    automation::battle_fight::ShipState, battle::netplay::CrcState, comm::locdata::CGameState,
    ships::native::abi::GlobData,
};
use crate::{automation::parity::Journal, comm::locdata::CQueue};

#[cfg(not(test))]
pub fn checkpoint(journal: &mut Journal, boundary: &str) -> std::io::Result<()> {
    let mut hooks = crate::ships::hook_registry::hooks();
    hooks.extend(crate::automation::parity_hooks::hooks());
    let mut fields = Fields::new();
    value(&mut fields, "mode", "boundary", boundary);
    // SAFETY: the checkpoint owns an immutable borrow of the initialized game,
    // queue pools and their linked records. No callback runs before it returns.
    let (game, queues, elements) = unsafe {
        crate::app::bridge::enter(|cx| {
            (
                &*(&raw const GlobData.Game_state).cast::<CGameState>(),
                std::array::from_fn::<_, 2, _>(|side| {
                    Ship::queue(
                        &*crate::c_bindings::live_queues::player_ships(side),
                        cx.elements(),
                    )
                }),
                display_elements(
                    &*crate::c_bindings::live_queues::display_list(),
                    cx.elements(),
                ),
            )
        })
    };
    crate::automation::parity_mode::capture(&mut fields, game, journal);
    let ships: [_; 2] = std::array::from_fn(|side| {
        queues[side]
            .iter()
            .enumerate()
            .map(|(slot, ship)| (format!("ships.{side}.{slot}"), ship))
            .collect::<Vec<_>>()
    });
    for side in &ships {
        for (prefix, ship) in side {
            journal.register_parent(ship.address(), prefix.clone());
            journal.owner(ship.address(), ship.side(), ship.species());
        }
    }
    journal.drain_events(&hooks);
    let space_active = journal.space_active;
    value(&mut fields, "mode", "space_active", space_active);
    for side in &ships {
        for (prefix, ship) in side {
            ship.write(&mut fields, prefix, journal);
            // SAFETY: the checkpoint holds the ship's entire immutable live graph.
            let resources = unsafe { ship.resources() };
            resources.register(journal, &journal.parent(ship.address()));
            resources.write(&mut fields, prefix, journal, &hooks);
        }
    }
    let owners = ships
        .iter()
        .flatten()
        .map(|(_, ship)| ship.address())
        .collect();
    custom::pkunk_sound(&mut fields);
    let seed = crate::math::seed_random(0);
    crate::math::seed_random(seed);
    value(&mut fields, "mode", "rng", seed);
    let mut crc = CrcState::new();
    crc.process_u32(seed);
    for (index, element) in elements.iter().enumerate() {
        let prefix = format!("elements.{index}");
        elements::element(&mut fields, &prefix, element, journal, &hooks, &owners);
        visuals(&mut fields, &prefix, element, journal);
        crate::supermelee::netplay::checksum::crc_process_element(&mut crc, &element.state);
    }
    journal.checkpoint(game.current_activity, crc.finish(), &fields)
}

#[cfg(not(test))]
pub(in crate::automation) fn fighters() -> ([Option<ShipState>; 2], [u8; 2]) {
    // SAFETY: the snapshot runs synchronously at a published game-thread boundary.
    unsafe {
        crate::app::bridge::enter(|cx| {
            let mut fighters = [None, None];
            let mut next = cx.elements().head();
            while let Some(id) = next {
                next = cx.elements().succ(id);
                let e = cx.element(id);
                if e.flags & crate::ships::native::abi::PLAYER_SHIP as u16 == 0 {
                    continue;
                }
                let side = match e.player {
                    0 => 0,
                    1 => 1,
                    _ => continue,
                };
                if let crate::engine::battle::Parent::Ship(owner) = cx.parent(id) {
                    fighters[side] = Some(ShipState {
                        x: i32::from(e.current.location.x),
                        y: i32::from(e.current.location.y),
                        facing: (cx.ship_state(owner).facing % 16) as u8,
                        crew: e.hits,
                    });
                }
            }
            (fighters, cx.b.counters)
        })
    }
}

/// # Safety
/// The pool remains live and immutable throughout observation.
unsafe fn display_elements(
    queue: &CQueue,
    #[cfg(not(test))] elements: &crate::engine::battle::ElementTable,
) -> Vec<ElementObservation> {
    // SAFETY: the caller retains the immutable display pool throughout this observation.
    unsafe {
        crate::engine::battle::access::observe::elements(
            queue,
            #[cfg(not(test))]
            elements,
        )
    }
}

fn visuals(fields: &mut Fields, prefix: &str, element: &ElementObservation, journal: &mut Journal) {
    let identity = journal.identity(element.address);
    for (name, pointer) in [
        ("current.frame", element.current.image.frame.raw()),
        ("next.frame", element.next.image.frame.raw()),
        ("intersect.frame", element.intersect.IntersectStamp.frame),
    ] {
        journal.write_element_frame(
            &identity,
            name,
            pointer as usize,
            fields,
            &format!("{prefix}.{name}"),
        );
    }
    for (name, array) in [
        ("current.farray", element.current.image.frames.raw()),
        ("next.farray", element.next.image.frames.raw()),
    ] {
        let address = array as usize;
        if address != 0 && journal.frame_set(address).is_none() {
            // SAFETY: descriptor arrays are registered while owned by their ship.
            // Remaining first-seen arrays are the live engine's fixed frame globals.
            let first = unsafe { *array };
            let mut frame = Fields::new();
            journal.write_frame(first as usize, &mut frame, "frame");
            journal.register_frame_set(
                address,
                vec![first as usize],
                format!("frame-single:{}", frame["frame.owner"]),
            );
        }
        journal.write_element_frame_set(
            &identity,
            name,
            address,
            fields,
            &format!("{prefix}.{name}"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::element::Element;
    fn visuals(fields: &mut Fields, prefix: &str, element: &Element, journal: &mut Journal) {
        // SAFETY: the test owns a layout-compatible source record.
        let observation =
            ElementObservation::decode(unsafe { &*std::ptr::from_ref(element).cast() });
        super::visuals(fields, prefix, &observation, journal);
    }

    use crate::graphics::native_abi::frame_desc;
    fn pool(elements: &mut [Element]) -> CQueue {
        let base = elements.as_mut_ptr();
        for index in 0..elements.len() {
            elements[index].pred = if index == 0 {
                std::ptr::null_mut()
            } else {
                &mut elements[index - 1]
            };
            elements[index].succ = if index + 1 == elements.len() {
                std::ptr::null_mut()
            } else {
                &mut elements[index + 1]
            };
        }
        CQueue {
            head: base.cast(),
            tail: std::ptr::from_mut(
                elements
                    .last_mut()
                    .unwrap_or_else(|| panic!("test display pool is empty")),
            )
            .cast(),
            pq_tab: base.cast(),
            object_size: std::mem::size_of::<Element>() as u16,
            num_objects: elements.len() as u8,
            ..CQueue::default()
        }
    }
    #[test]
    fn visual_fields_include_the_borrowed_frame_array() {
        let directory = super::super::test_directory();
        let mut journal = Journal::create(directory.path())
            .unwrap_or_else(|error| panic!("create visual test journal: {error}"));
        let mut storage = 0u8;
        let mut views = [std::ptr::from_mut(&mut storage).cast()];
        let mut element = Element::new();
        element.current.frame = views[0];
        element.current.farray = views.as_mut_ptr();
        journal.register_frame(views[0] as usize, 0, 7);
        journal.allocate(std::ptr::from_ref(&element) as usize);
        let mut fields = Fields::new();
        visuals(&mut fields, "element", &element, &mut journal);
        assert_eq!(fields["element.current.frame.index"], "7");
        assert_eq!(fields["element.current.farray.0.index"], "7");
        assert_eq!(fields["element.next.frame.owner"], "none");
    }

    #[test]
    fn linked_element_keeps_retired_frame_binding_when_allocator_reuses_address() {
        let directory = super::super::test_directory();
        let mut journal = Journal::create(directory.path()).expect("journal");
        let mut storage = 0u8;
        let pointer = std::ptr::from_mut(&mut storage).cast();
        let mut views = [pointer];
        let mut element = Element::new();
        element.current.frame = pointer;
        element.current.farray = views.as_mut_ptr();
        journal.allocate(std::ptr::from_ref(&element) as usize);
        let owner = journal.register_resource(100, "drawable");
        journal.register_frame(pointer as usize, 100, 7);
        journal.register_frame_set(
            views.as_ptr() as usize,
            vec![pointer as usize],
            "old-set".into(),
        );
        let mut fields = Fields::new();
        visuals(&mut fields, "element", &element, &mut journal);
        journal.retire_drawable(100);
        journal.register_resource(100, "drawable");
        journal.register_frame(pointer as usize, 100, 99);
        journal.register_frame_set(
            views.as_ptr() as usize,
            vec![pointer as usize],
            "new-set".into(),
        );
        visuals(&mut fields, "element", &element, &mut journal);
        assert_eq!(fields["element.current.frame.owner"], owner);
        assert_eq!(fields["element.current.frame.index"], "7");
        assert_eq!(fields["element.current.frame.retired"], "true");
        assert_eq!(fields["element.current.farray.0.owner"], owner);
        assert_eq!(fields["element.current.farray.role"], "old-set");
        journal.allocate(std::ptr::from_ref(&element) as usize);
        visuals(&mut fields, "element", &element, &mut journal);
        assert_eq!(fields["element.current.frame.index"], "99");
        assert_eq!(fields["element.current.frame.retired"], "false");
        assert_eq!(fields["element.current.farray.role"], "new-set");
    }

    #[test]
    #[serial_test::serial]
    fn space_arrays_reload_all_views_with_fresh_and_reused_storage() {
        use crate::collections::observation::observe_views;
        use crate::collections::observation::{self, Event};
        observation::enable();
        observation::drain();
        for reuse in [false, true] {
            let directory = super::super::test_directory();
            let mut journal = Journal::create(directory.path()).expect("journal");
            let mut views = [std::ptr::null_mut(); 3];
            let mut element = Element::new();
            journal.allocate(std::ptr::from_ref(&element) as usize);
            let generation = journal.identity(std::ptr::from_ref(&element) as usize);
            for cycle in 0..2 {
                let mut owners = Vec::new();
                for (view, frame) in views.iter_mut().enumerate() {
                    let owner = 100 + view + if reuse { 0 } else { cycle * 10 };
                    let address = 200 + view + if reuse { 0 } else { cycle * 10 };
                    observation::record(Event::Drawable(owner));
                    observation::record(Event::Frame {
                        address,
                        owner,
                        index: 0,
                    });
                    *frame = address as *mut frame_desc;
                    owners.push(owner);
                }
                observe_views(&views, "engine-global:test-animation", |frame| {
                    *frame as usize
                });
                observation::assign_frame(&mut element.current.frame, views[0].cast());
                if cycle == 0 {
                    observation::assign_set(
                        &mut element.current.farray,
                        views.as_mut_ptr().cast(),
                        None,
                    );
                }
                journal.drain_events(&[]);
                assert_eq!(
                    journal.identity(std::ptr::from_ref(&element) as usize),
                    generation
                );
                let mut fields = Fields::new();
                visuals(&mut fields, "element", &element, &mut journal);
                for (view, owner) in owners.into_iter().enumerate() {
                    assert_eq!(
                        fields[&format!("element.current.farray.{view}.owner")],
                        journal.resource(owner)
                    );
                    assert_eq!(
                        fields[&format!("element.current.farray.{view}.retired")],
                        "false"
                    );
                    observation::record(Event::DrawableFree(owner));
                }
                views.fill(std::ptr::null_mut());
                observe_views(&views, "engine-global:test-animation", |frame| {
                    *frame as usize
                });
                journal.drain_events(&[]);
                visuals(&mut fields, "element", &element, &mut journal);
                assert_eq!(fields["element.current.frame.retired"], "true");
                for view in 0..3 {
                    assert_eq!(
                        fields[&format!("element.current.farray.{view}.owner")],
                        "none"
                    );
                }
                assert_eq!(
                    journal.frame_set(views.as_ptr() as usize),
                    Some([0, 0, 0].as_slice())
                );
            }
        }
    }

    #[test]
    fn checked_display_pool_preserves_link_order() {
        let mut elements = [Element::new(); 3];
        let queue = pool(&mut elements);
        // SAFETY: the local pool remains live and unchanged for this traversal.
        let addresses: Vec<_> = unsafe { display_elements(&queue).into_iter() }
            .map(|e| e.address as *const Element)
            .collect();
        assert_eq!(
            addresses,
            elements.iter().map(std::ptr::from_ref).collect::<Vec<_>>()
        );
    }
    #[test]
    #[should_panic(expected = "live display queue")]
    fn cyclic_display_pool_fails_before_following_invalid_links() {
        let mut elements = [Element::new(); 2];
        let queue = pool(&mut elements);
        let elements = &mut elements;
        elements[1].succ = &mut elements[0];
        // SAFETY: the pool remains live; walk rejects the malformed link before
        // borrowing any slot through it.
        let _ = unsafe { display_elements(&queue).into_iter() }.count();
    }
    #[test]
    #[should_panic(expected = "live display queue")]
    fn pointer_outside_display_pool_fails_before_dereference() {
        let mut elements = [Element::new(); 2];
        let queue = pool(&mut elements);
        let elements = &mut elements;
        elements[0].succ = std::ptr::dangling_mut();
        // SAFETY: the pool remains live; walk rejects the malformed link before
        // borrowing any slot through it.
        let _ = unsafe { display_elements(&queue).into_iter() }.count();
    }
}
