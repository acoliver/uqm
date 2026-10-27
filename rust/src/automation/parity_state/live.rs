//! Synchronous game-thread graph borrowing and journal orchestration.
use super::Fields;
#[cfg(not(test))]
use super::{custom, elements, ships::Ship, value};
#[cfg(not(test))]
use crate::{
    automation::battle_fight::ShipState, battle::netplay::CrcState, comm::locdata::CGameState,
    ships::native::abi::GlobData,
};
use crate::{automation::parity::Journal, battle::element::Element, comm::locdata::CQueue};

#[cfg(not(test))]
pub fn checkpoint(journal: &mut Journal, boundary: &str) -> std::io::Result<()> {
    let mut hooks = crate::ships::hook_registry::hooks();
    hooks.extend(crate::automation::parity_hooks::hooks());
    let mut fields = Fields::new();
    value(&mut fields, "mode", "boundary", boundary);
    // SAFETY: the checkpoint owns an immutable borrow of the initialized game,
    // queue pools and their linked records. No callback runs before it returns.
    let (game, queues, elements) = unsafe {
        (
            &*(&raw const GlobData.Game_state).cast::<CGameState>(),
            std::array::from_fn::<_, 2, _>(|side| {
                Ship::queue(&*crate::c_bindings::live_queues::player_ships(side))
            }),
            display_elements(&*crate::c_bindings::live_queues::display_list()),
        )
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
    for (index, element) in elements.enumerate() {
        let prefix = format!("elements.{index}");
        elements::element(&mut fields, &prefix, element, journal, &hooks, &owners);
        visuals(&mut fields, &prefix, element, journal);
        crc.process_element(element);
    }
    journal.checkpoint(game.current_activity, crc.finish(), &fields)
}

#[cfg(not(test))]
pub(in crate::automation) fn fighters() -> ([Option<ShipState>; 2], [u8; 2]) {
    let mut fighters = [None, None];
    // SAFETY: BattleSnapshot::live runs synchronously on the game thread.
    let (elements, counters) = unsafe {
        (
            display_elements(&*crate::c_bindings::live_queues::display_list()),
            crate::battle::ffi::live_counters::battle_counters(),
        )
    };
    for element in elements.filter(|element| element.is_player_ship()) {
        let side = match element.player_nr {
            0 => 0,
            1 => 1,
            _ => continue,
        };
        if !element.p_parent.is_null() {
            // SAFETY: a non-null PLAYER_SHIP parent is a live ship record; the
            // synchronous snapshot keeps it immutable until its fields are read.
            let state = unsafe { &*element.p_parent.cast() };
            let owner = Ship { state };
            fighters[side] = Some(ShipState {
                x: i32::from(element.current.location.x),
                y: i32::from(element.current.location.y),
                facing: (owner.facing() % 16) as u8,
                crew: element.crew_or_hp,
            });
        }
    }
    (fighters, counters)
}

/// # Safety
/// The queue authority and its pool must remain live and immutable for the borrow.
unsafe fn display_elements(queue: &CQueue) -> impl Iterator<Item = &Element> {
    use crate::savegame::runtime::queues::{walk, QueueKind};
    let links = if queue.pq_tab.is_null() && queue.head.is_null() {
        assert!(
            queue.tail.is_null(),
            "uninitialized display queue has a tail"
        );
        Vec::new()
    } else {
        walk::<Element>(queue, QueueKind::DisplayList)
            .unwrap_or_else(|error| panic!("live display queue: {error}"))
    };
    // SAFETY: pool, slot alignment, record size, links and tail were checked by walk.
    links.into_iter().map(|pointer| unsafe { &*pointer })
}

fn visuals(fields: &mut Fields, prefix: &str, element: &Element, journal: &mut Journal) {
    let identity = journal.identity(std::ptr::from_ref(element) as usize);
    for (name, pointer) in [
        ("current.frame", element.current.frame),
        ("next.frame", element.next.frame),
        (
            "intersect.frame",
            element.intersect_control.intersect_stamp.frame,
        ),
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
        ("current.farray", element.current.farray),
        ("next.farray", element.next.farray),
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
        // SAFETY: frame_desc has only integer, pointer and optional callback fields.
        let mut frame: frame_desc = unsafe { std::mem::zeroed() };
        frame.Index = 7;
        let mut views = [std::ptr::from_mut(&mut frame).cast()];
        let mut element = Element::new();
        element.current.frame = views[0];
        element.current.farray = views.as_mut_ptr();
        journal.register_frame(views[0] as usize, 0, frame.Index);
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
        // SAFETY: the local frame has only scalar and pointer fields and stays borrowed.
        let mut frame: frame_desc = unsafe { std::mem::zeroed() };
        frame.Index = 7;
        let pointer = std::ptr::from_mut(&mut frame).cast();
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
    fn checked_display_pool_preserves_link_order() {
        let mut elements = [Element::new(); 3];
        let queue = pool(&mut elements);
        // SAFETY: the local pool remains live and unchanged for this traversal.
        let addresses: Vec<_> = unsafe { display_elements(&queue) }
            .map(std::ptr::from_ref)
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
        let _ = unsafe { display_elements(&queue) }.count();
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
        let _ = unsafe { display_elements(&queue) }.count();
    }
}
