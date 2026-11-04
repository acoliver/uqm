use std::{panic::AssertUnwindSafe, ptr};
use uqm_rust::{
    engine::battle::frames::free_image,
    graphics::{drawable_abi::AllocDrawable, pixmap_abi::CaptureDrawable},
};

#[test]
#[serial_test::serial]
fn releasing_animation_rejects_unowned_addresses_before_mutation() {
    let foreign = ptr::dangling_mut();
    let mut frames = [ptr::null_mut(), foreign, ptr::null_mut()];
    let before = frames;
    assert!(std::panic::catch_unwind(AssertUnwindSafe(|| {
        free_image(&mut frames, "ownership-test")
    }))
    .is_err());
    assert_eq!(frames, before);
}

#[test]
#[serial_test::serial]
fn releasing_an_aliased_animation_retires_its_allocation_once() {
    // SAFETY: this integration test exclusively owns the native drawable and its three aliases.
    let frame = unsafe { CaptureDrawable(AllocDrawable(1)) };
    assert!(!frame.is_null());
    let mut frames = [frame; 3];
    assert_eq!(free_image(&mut frames, "ownership-test"), 1);
    assert_eq!(frames, [ptr::null_mut(); 3]);
    assert!(std::panic::catch_unwind(|| {
        uqm_rust::graphics::drawable_abi::retain_frame_allocations(&[frame])
    })
    .is_err());
}
