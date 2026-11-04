//! Space animation ownership, ported from init.c (GPL-2.0-or-later).

#[cfg(not(test))]
use crate::collections::observation::observe_views;
use crate::graphics::native_abi::frame_desc;
use std::ptr;

const SPACE_ANIMATION_KEYS: [[&std::ffi::CStr; 3]; 3] = [
    [
        c"graphics.boom.large",
        c"graphics.boom.medium",
        c"graphics.boom.small",
    ],
    [
        c"graphics.blast.large",
        c"graphics.blast.medium",
        c"graphics.blast.small",
    ],
    [
        c"graphics.asteroid.large",
        c"graphics.asteroid.medium",
        c"graphics.asteroid.small",
    ],
];

fn release_images(
    frames: &mut [*mut frame_desc; 3],
    mut destroy: impl FnMut(*mut frame_desc) -> bool,
) -> bool {
    let mut freed = [ptr::null_mut(); 3];
    let mut result = true;
    for i in 0..3 {
        let frame = frames[i];
        if !frame.is_null() {
            if !freed[..i].contains(&frame) && !destroy(frame) {
                result = false;
            }
            freed[i] = frame;
            frames[i] = ptr::null_mut();
        }
    }
    result
}

#[cfg(not(test))]
use crate::graphics::{
    drawable_abi::DestroyDrawable,
    pixmap_abi::{CaptureDrawable, ReleaseDrawable},
    resgfx::LoadGraphicInstance,
};

#[cfg(not(test))]
pub fn load_animation(
    frames: &mut [*mut frame_desc; 3],
    keys: [Option<&std::ffi::CStr>; 3],
    role: &str,
) -> u32 {
    let Some(big) = keys[0] else { return 0 };
    // SAFETY: CStr keys remain borrowed through resource decoding; loaded drawables own their frames.
    unsafe {
        let mut drawable =
            LoadGraphicInstance(big.as_ptr()).cast::<crate::graphics::native_abi::drawable_desc>();
        if drawable.is_null() {
            return 0;
        }
        frames[0] = CaptureDrawable(drawable);
        for (index, key) in keys[1..].iter().enumerate() {
            if let Some(key) = key {
                drawable = LoadGraphicInstance(key.as_ptr()).cast();
                if drawable.is_null() {
                    return 0;
                }
            }
            frames[index + 1] = CaptureDrawable(drawable);
        }
    }
    observe_views(frames, role, |frame| *frame as usize);
    1
}

#[cfg(not(test))]
pub fn free_image(frames: &mut [*mut frame_desc; 3], role: &str) -> u32 {
    let _allocations = crate::graphics::drawable_abi::retain_frame_allocations(frames);
    let freed = release_images(frames, |frame| {
        // SAFETY: every non-null address was validated and its allocation remains retained through release.
        unsafe { DestroyDrawable(ReleaseDrawable(frame)) != 0 }
    });
    observe_views(frames, role, |frame| *frame as usize);
    freed as u32
}

#[cfg(not(test))]
pub(crate) mod live {
    use super::*;

    pub fn init_space(kernel: &mut crate::engine::Kernel, activity: u16) -> u32 {
        {
            let old = kernel.space_init_count;
            kernel.space_init_count = kernel.space_init_count.wrapping_add(1);
            if old == 0 && activity as u8 <= crate::ships::native::abi::IN_ENCOUNTER as u8 {
                let stars = &mut kernel.stars_in_space;
                // SAFETY: the resource key is static and the returned drawable supplies its captured frame.
                *stars = unsafe {
                    CaptureDrawable(LoadGraphicInstance(c"graphics.stars".as_ptr()).cast())
                };
                if stars.is_null() {
                    return 0;
                }
                observe_views(std::slice::from_ref(stars), "engine-global:stars", |f| {
                    *f as usize
                });
                if [
                    &mut kernel.explosion,
                    &mut kernel.blast,
                    &mut kernel.asteroid,
                ]
                .into_iter()
                .zip(SPACE_ANIMATION_KEYS)
                .zip([
                    "engine-global:explosion",
                    "engine-global:blast",
                    "engine-global:asteroid",
                ])
                .any(|((frames, [big, medium, small]), role)| {
                    load_animation(frames, [Some(big), Some(medium), Some(small)], role) == 0
                }) {
                    return 0;
                }
            }
            crate::collections::observation::record(
                crate::collections::observation::Event::SpaceActive(true),
            );
            1
        }
    }

    pub fn uninit_space(kernel: &mut crate::engine::Kernel) {
        {
            if kernel.space_init_count != 0 {
                kernel.space_init_count -= 1;
                if kernel.space_init_count == 0 {
                    crate::collections::observation::record(
                        crate::collections::observation::Event::SpaceActive(false),
                    );
                    free_image(&mut kernel.blast, "engine-global:blast");
                    free_image(&mut kernel.explosion, "engine-global:explosion");
                    free_image(&mut kernel.asteroid, "engine-global:asteroid");
                    let stars = &mut kernel.stars_in_space;
                    let _allocation = crate::graphics::drawable_abi::retain_frame_allocations(
                        std::slice::from_ref(stars),
                    );
                    for plane in kernel.star_planes.iter_mut() {
                        plane
                            .frames
                            .fill(crate::engine::battle::frames::FrameSelection::ABSENT);
                    }
                    // SAFETY: the star frame is null or belongs to the retained registered allocation.
                    unsafe {
                        DestroyDrawable(ReleaseDrawable(*stars));
                    }
                    *stars = ptr::null_mut();
                    observe_views(std::slice::from_ref(stars), "engine-global:stars", |f| {
                        *f as usize
                    });
                }
            }
        }
    }

    #[no_mangle]
    pub extern "C" fn InitShips() -> i16 {
        unsafe { crate::ships::ffi::rust_ships_init() as i16 }
    }
    #[no_mangle]
    pub extern "C" fn UninitShips() {
        unsafe { crate::ships::ffi::rust_ships_uninit() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aliases_are_destroyed_once_and_all_slots_are_cleared() {
        let a = ptr::dangling_mut::<frame_desc>();
        let b = a.wrapping_add(1);
        let mut frames = [a, b, a];
        let mut destroyed = vec![];
        assert!(!release_images(&mut frames, |frame| {
            destroyed.push(frame);
            frame != a
        }));
        assert_eq!(destroyed, [a, b]);
        assert_eq!(frames, [ptr::null_mut(); 3]);
    }
    #[test]
    fn animation_keys_keep_explosion_blast_asteroid_order_and_three_sizes() {
        assert_eq!(SPACE_ANIMATION_KEYS[0][0], c"graphics.boom.large");
        assert_eq!(SPACE_ANIMATION_KEYS[1][1], c"graphics.blast.medium");
        assert_eq!(SPACE_ANIMATION_KEYS[2][2], c"graphics.asteroid.small");
    }
    #[test]
    fn null_slots_do_not_own_resources() {
        let mut frames = [ptr::null_mut(); 3];
        assert!(release_images(&mut frames, |_| panic!(
            "null frame destroyed"
        )));
    }
}

#[cfg(not(test))]
pub use live::*;
