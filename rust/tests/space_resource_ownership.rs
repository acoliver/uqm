#![forbid(unsafe_code)]
use std::panic::AssertUnwindSafe;
use uqm_engine::battle::frames::free_image;
use uqm_engine::graphics::drawable::{retain_frames, FrameId, LoadedDrawable};

#[test]
#[serial_test::serial]
fn releasing_animation_rejects_unowned_addresses_before_mutation() {
    let foreign = FrameId::from_bits(0x7777 << 16);
    let mut frames = [FrameId::NULL, foreign, FrameId::NULL];
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
    let frame = LoadedDrawable::create(0, 1, 1, 1).unwrap().into_frame();
    let mut frames = [frame; 3];
    assert_eq!(free_image(&mut frames, "ownership-test"), 1);
    assert_eq!(frames, [FrameId::NULL; 3]);
    assert!(std::panic::catch_unwind(|| retain_frames(&[frame])).is_err());
}
