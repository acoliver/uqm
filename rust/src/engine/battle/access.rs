//! Stage B resource and record projections. Borrows end before legacy handoffs.

use super::{legacy, record::*, Cx, ElementId, Parent, ShipId};
use crate::graphics::native_abi::PRIMITIVE;

impl Cx<'_> {
    pub(crate) fn ship_id(&self, address: usize) -> ShipId {
        for side in 0..2 {
            // SAFETY: queue authorities are owned by this game-thread context.
            let queue = unsafe { &legacy::race_q[side] };
            let stride = usize::from(queue.object_size);
            if let Some(offset) = address.checked_sub(queue.pq_tab as usize) {
                if stride != 0
                    && offset.is_multiple_of(stride)
                    && offset / stride < usize::from(queue.num_objects)
                {
                    return ShipId {
                        side: side as u8,
                        slot: (offset / stride) as u8,
                    };
                }
            }
        }
        panic!("ship address {address:#x} is outside the ship pools")
    }

    pub fn owner(&self, id: ElementId) -> ShipId {
        match self.parent(id) {
            Parent::Ship(ship) => ship,
            other => panic!("element {id:?} has no ship owner: {other:?}"),
        }
    }

    pub fn parent(&self, id: ElementId) -> Parent {
        // SAFETY: Cx owns the game-thread authorities for this synchronous call.
        unsafe {
            self.b.elements.parent(
                id,
                [&raw const legacy::race_q[0], &raw const legacy::race_q[1]],
                self.world.groups(),
            )
        }
    }

    pub(crate) fn ship_pointer(&self, id: ShipId) -> *mut STARSHIP {
        assert!(id.side < 2, "invalid ship side {id:?}");
        // SAFETY: queue storage is address-stable for the battle's lifetime.
        unsafe {
            let queue = &legacy::race_q[usize::from(id.side)];
            assert!(
                usize::from(id.slot) < usize::from(queue.num_objects),
                "invalid ship {id:?}"
            );
            queue
                .pq_tab
                .add(usize::from(id.slot) * usize::from(queue.object_size))
                .cast()
        }
    }

    pub fn ship(&self, id: ShipId) -> &STARSHIP {
        // SAFETY: the validated slot belongs to this context's ship authority.
        unsafe { &*self.ship_pointer(id) }
    }

    pub fn ship_mut(&mut self, id: ShipId) -> &mut STARSHIP {
        // SAFETY: this exclusive context borrow scopes the one-record projection.
        unsafe { &mut *self.ship_pointer(id) }
    }

    pub fn desc(&self, id: ShipId) -> &RACE_DESC {
        let pointer = self.ship(id).RaceDescPtr;
        assert!(!pointer.is_null(), "ship {id:?} has no descriptor");
        // SAFETY: each live ship owns its descriptor until teardown.
        unsafe { &*pointer }
    }

    pub fn desc_mut(&mut self, id: ShipId) -> &mut RACE_DESC {
        let pointer = self.ship(id).RaceDescPtr;
        assert!(!pointer.is_null(), "ship {id:?} has no descriptor");
        // SAFETY: the exclusive context borrow scopes the descriptor projection.
        unsafe { &mut *pointer }
    }

    pub fn ship_element(&self, ship: ShipId) -> ElementId {
        self.b
            .elements
            .id(self.ship(ship).hShip)
            .expect("ship has no element")
    }

    pub fn set_parent(&mut self, element: ElementId, parent: Option<ShipId>) {
        self.b.elements[element].pParent =
            parent.map_or(std::ptr::null_mut(), |ship| self.ship_pointer(ship).cast());
    }

    pub fn target(&self, element: ElementId) -> Option<ElementId> {
        self.b.elements.id(self.b.elements[element].hTarget)
    }

    pub fn set_target(&mut self, element: ElementId, target: Option<ElementId>) {
        self.b.elements[element].hTarget =
            target.map_or(std::ptr::null_mut(), |id| self.b.elements.raw(id));
    }

    pub fn put(&mut self, id: ElementId) {
        let handle = self.b.elements.raw(id);
        // SAFETY: id is allocated and unlinked; the caller preserves queue order.
        unsafe { crate::collections::queue::PutQueue(&raw mut legacy::disp_q, handle) }
    }

    pub fn primitive(&mut self, id: ElementId) -> &mut PRIMITIVE {
        let index = usize::from(self.b.elements[id].PrimIndex);
        assert!(index < 330, "invalid primitive index {index}");
        // SAFETY: the display authority remains live for the context's lifetime.
        unsafe { &mut legacy::DisplayArray[index] }
    }
}
