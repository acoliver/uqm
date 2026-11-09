use crate::battle::gravity::{calculate_gravity, time_space_matter_conflict};
use crate::engine::battle::{Battle, Cx, ElementId, World};
use std::cell::Cell;

fn with_elements(call: impl FnOnce(&mut Cx<'_>, [ElementId; 3])) {
    // SAFETY: the fixture owns the local queues, slots and projections until cleanup.
    {
        let mut battle = Battle::new();
        let mut kernel = crate::engine::battle::record::Kernel::new();
        let elements = kernel.fixture_elements();
        elements.reset_capacity(3);
        let ids = std::array::from_fn(|_| {
            let id = elements.alloc().unwrap_or_else(|| panic!("fixture slot"));

            elements.put(id);
            id
        });
        let mut world = World::new();
        let mut cx = Cx::new(&mut battle, &mut world, &mut kernel, || 0);
        cx.element_mut(ids[2]).flags = crate::ships::native::abi::NONSOLID as u16;
        call(&mut cx, ids);
    }
}

#[test]
fn gravity_selects_next_positions_without_changing_traversal_order() {
    use crate::ships::native::abi::{NONSOLID, PRE_PROCESS};
    with_elements(|cx, [ship, planet, _]| {
        cx.element_mut(planet).mass = 200;
        cx.element_mut(ship).current.location.x = 2000;
        cx.element_mut(ship).next.location.x = 400;
        cx.element_mut(planet).next.location.x = 400;
        let head = cx.elements().head();
        assert!(!calculate_gravity(cx, ship, (320, 240)));
        cx.element_mut(ship).flags = PRE_PROCESS as u16;
        assert!(calculate_gravity(cx, ship, (320, 240)));
        assert_eq!(cx.elements().head(), head);
        assert_eq!(cx.elements().succ(ship), Some(planet));
        cx.element_mut(planet).flags = NONSOLID as u16;
        assert!(!calculate_gravity(cx, ship, (320, 240)));
    });
}

#[test]
fn gravity_accelerates_a_non_gravitating_element_toward_the_source() {
    with_elements(|cx, [planet, ship, _]| {
        cx.element_mut(planet).mass = 200;
        cx.element_mut(ship).current.location.x = 320;
        assert!(!calculate_gravity(cx, planet, (320, 240)));
        let (x, y) = cx.element(ship).velocity.get_current_components();
        assert!(x < 0);
        assert_eq!(y, 0);
    });
}

#[test]
fn conflict_checks_player_ships_even_when_nonsolid() {
    use crate::ships::native::abi::{NONSOLID, PLAYER_SHIP};
    with_elements(|cx, [subject, other, _]| {
        cx.element_mut(subject).current.location.x = 100;
        cx.element_mut(other).flags = NONSOLID as u16;
        let control = crate::battle::gravity::control(
            cx,
            subject,
            crate::engine::battle::views::FrameSelection::ABSENT,
        );
        assert_eq!(control.EndPoint.x, 25);
        assert_eq!(control.IntersectStamp.origin.x, 25);
        let calls = Cell::new(0);
        let intersect = |id| {
            calls.set(calls.get() + 1);
            assert_eq!(id, other);
            true
        };
        assert!(!time_space_matter_conflict(cx, subject, intersect));
        assert_eq!(calls.get(), 0);
        cx.element_mut(other).flags |= PLAYER_SHIP as u16;
        assert!(time_space_matter_conflict(cx, subject, intersect));
        assert_eq!(calls.get(), 1);
    });
}
