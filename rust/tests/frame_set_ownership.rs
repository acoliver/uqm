use uqm_rust::engine::battle::{
    frames::{FrameArray, FrameOwner, FrameSetRef, Frames, Role, SharedFrames},
    Battle,
};
use uqm_rust::graphics::{
    drawable_abi::{retain_frame_allocations, AllocDrawable, DestroyDrawable},
    pixmap_abi::CaptureDrawable,
};

#[test]
#[serial_test::serial]
fn parentless_image_retains_frames_after_descriptor_and_owner_are_gone() {
    // SAFETY: the fixture owns the registered drawable and serializes its graphics calls.
    unsafe {
        let drawable = AllocDrawable(1);
        let frame = CaptureDrawable(drawable);
        let mut owner = Battle::new(0);
        owner.reset_ship_capacity(0, 1);
        let mut ships = owner.ships(0);
        let link = ships.alloc().unwrap();
        let ship = ships.ship_id(link);
        let mut descriptor = uqm_rust::engine::battle::record::RaceDescriptor::EMPTY;
        descriptor.ship_data.weapon = FrameArray::from_native([frame; 3]);
        let image = FrameSetRef::ship(ship, Role::Weapon, descriptor.ship_data.weapon.clone());
        drop(descriptor);
        drop(owner);
        assert_ne!(DestroyDrawable(drawable), 0);
        assert_eq!((*image.view(0).raw()).parent, drawable);
        let copy = image.clone();
        drop(image);
        assert_eq!((*copy.view(2).raw()).parent, drawable);
        drop(copy);
        assert!(std::panic::catch_unwind(|| retain_frame_allocations(&[frame])).is_err());
    }
}

#[test]
fn ownership_distinguishes_role_ship_and_reused_slot_generation() {
    let mut owner = Battle::new(0);
    owner.reset_ship_capacity(0, 1);
    let mut ships = owner.ships(0);
    let link = ships.alloc().unwrap();
    let ship = ships.ship_id(link);
    ships.free(link);
    let next = ships.alloc().unwrap();
    let other = ships.ship_id(next);
    assert_eq!(ship.slot(), other.slot());
    assert_ne!(ship.generation(), other.generation());
    let set = FrameSetRef::ship(ship, Role::Special, FrameArray::EMPTY);
    assert_eq!(set.owner, FrameOwner::Ship(ship, Role::Special));
    assert_ne!(set.owner, FrameOwner::Ship(ship, Role::Weapon));
    assert_ne!(set.owner, FrameOwner::Ship(other, Role::Special));
}

#[test]
#[serial_test::serial]
fn copy_view_then_release_owner_keeps_each_drawable_alive() {
    // SAFETY: every native address comes from one of the serialized registered allocations.
    unsafe {
        let drawables = std::array::from_fn::<_, 3, _>(|_| AllocDrawable(1));
        let views = drawables.map(|d| CaptureDrawable(d));
        let owner = FrameArray::from_native(views);
        for drawable in drawables {
            assert_ne!(DestroyDrawable(drawable), 0);
        }
        // Logical release has already removed the original resource owners. The copied views
        // must acquire the real allocations through the still-live leases, without an Any owner.
        let copied = FrameArray::retain(owner.views());
        let image = Frames::shared(SharedFrames::Fixture(0), copied);
        drop(owner);
        for (i, drawable) in drawables.into_iter().enumerate() {
            assert_eq!((*image.view(i).raw()).parent, drawable);
        }
        let last = image.clone();
        drop(image);
        assert_eq!((*last.view(2).raw()).parent, drawables[2]);
        drop(last);
        for view in views {
            assert!(std::panic::catch_unwind(|| retain_frame_allocations(&[view])).is_err());
        }
    }
}
