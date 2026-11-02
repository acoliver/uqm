use std::{cell::Cell, rc::Rc};
use uqm_rust::engine::battle::{
    frames::{FrameArray, FrameOwner, FrameSetRef, Role},
    views::Frame,
    ShipId,
};

struct Lease {
    drops: Rc<Cell<usize>>,
}
impl Drop for Lease {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}

#[test]
fn parentless_image_retains_frames_after_descriptor_and_owner_are_gone() {
    let releases = Rc::new(Cell::new(0));
    let lease = Rc::new(Lease {
        drops: releases.clone(),
    });
    let mut descriptor = uqm_rust::engine::battle::record::RACE_DESC::EMPTY;
    descriptor.ship_data.weapon = FrameArray::retain([Frame::ABSENT; 3], lease.clone());
    let ship = ShipId {
        side: 0,
        slot: 2,
        gen: 3,
    };
    let image = FrameSetRef::ship(ship, Role::Weapon, descriptor.ship_data.weapon.clone());
    drop(descriptor);
    drop(lease);
    assert_eq!(releases.get(), 0);
    for index in 0..3 {
        assert!(image.view(index).is_absent());
    }
    let copy = image.clone();
    drop(image);
    assert_eq!(releases.get(), 0);
    drop(copy);
    assert_eq!(releases.get(), 1);
}

#[test]
fn ownership_distinguishes_role_ship_and_reused_slot_generation() {
    let ship = ShipId {
        side: 0,
        slot: 2,
        gen: 3,
    };
    let other = ShipId { gen: 4, ..ship };
    let set = FrameSetRef::ship(ship, Role::Special, FrameArray::EMPTY);
    assert_eq!(set.owner, FrameOwner::Ship(ship, Role::Special));
    assert_ne!(set.owner, FrameOwner::Ship(ship, Role::Weapon));
    assert_ne!(set.owner, FrameOwner::Ship(other, Role::Special));
}
