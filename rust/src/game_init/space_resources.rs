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
fn space_animations(
    kernel: &mut crate::engine::Kernel,
) -> impl Iterator<Item = (*mut [*mut frame_desc; 3], [&'static std::ffi::CStr; 3])> {
    [
        &raw mut kernel.explosion,
        &raw mut kernel.blast,
        &raw mut kernel.asteroid,
    ]
    .into_iter()
    .zip(SPACE_ANIMATION_KEYS)
}

#[cfg(not(test))]
use crate::graphics::{
    drawable_abi::DestroyDrawable,
    pixmap_abi::{CaptureDrawable, ReleaseDrawable},
    resgfx::LoadGraphicInstance,
};
#[cfg(not(test))]
use std::ffi::c_char;

#[cfg(not(test))]
/// # Safety
/// Frames points to three writable slots. Resource keys are NUL-terminated;
/// medium and small may be null to reuse the previous drawable.
pub unsafe fn load_animation(
    frames: *mut *mut frame_desc,
    big: *const c_char,
    medium: *const c_char,
    small: *const c_char,
    role: &str,
) -> u32 {
    let mut drawable =
        LoadGraphicInstance(big).cast::<crate::graphics::native_abi::drawable_desc>();
    if drawable.is_null() {
        return 0;
    }
    *frames = CaptureDrawable(drawable);
    for (i, key) in [medium, small].into_iter().enumerate() {
        if !key.is_null() {
            drawable = LoadGraphicInstance(key).cast();
            if drawable.is_null() {
                return 0;
            }
        }
        *frames.add(i + 1) = CaptureDrawable(drawable);
    }
    observe_views(std::slice::from_raw_parts(frames, 3), role, |frame| {
        *frame as usize
    });
    1
}

#[cfg(not(test))]
/// # Safety
/// Frames points to three exclusively borrowed live or null frame handles.
pub unsafe fn free_image(frames: *mut [*mut frame_desc; 3], role: &str) -> u32 {
    let frames = &mut *frames;
    let freed = release_images(frames, |frame| DestroyDrawable(ReleaseDrawable(frame)) != 0);
    observe_views(frames, role, |frame| *frame as usize);
    freed as u32
}

#[cfg(not(test))]
pub(crate) mod live {
    use super::*;

    /// # Safety
    /// Game thread only, after graphics and resource initialization.
    pub unsafe fn init_space(kernel: &mut crate::engine::Kernel, activity: u16) -> u32 {
        {
            let old = kernel.space_init_count;
            kernel.space_init_count = kernel.space_init_count.wrapping_add(1);
            if old == 0 && activity as u8 <= crate::ships::native::abi::IN_ENCOUNTER as u8 {
                let stars = &mut *ptr::addr_of_mut!(kernel.stars_in_space);
                *stars = CaptureDrawable(LoadGraphicInstance(c"graphics.stars".as_ptr()).cast());
                if stars.is_null() {
                    return 0;
                }
                observe_views(std::slice::from_ref(stars), "engine-global:stars", |f| {
                    *f as usize
                });
                if space_animations(&mut *kernel)
                    .zip([
                        "engine-global:explosion",
                        "engine-global:blast",
                        "engine-global:asteroid",
                    ])
                    .any(|((frames, [big, medium, small]), role)| {
                        load_animation(
                            frames.cast(),
                            big.as_ptr(),
                            medium.as_ptr(),
                            small.as_ptr(),
                            role,
                        ) == 0
                    })
                {
                    return 0;
                }
            }
            crate::collections::observation::record(
                crate::collections::observation::Event::SpaceActive(true),
            );
            1
        }
    }

    /// # Safety
    /// Game thread only; pairs with InitSpace while graphics is live.
    pub unsafe fn uninit_space(kernel: &mut crate::engine::Kernel) {
        {
            if kernel.space_init_count != 0 {
                kernel.space_init_count -= 1;
                if kernel.space_init_count == 0 {
                    crate::collections::observation::record(
                        crate::collections::observation::Event::SpaceActive(false),
                    );
                    free_image(ptr::addr_of_mut!(kernel.blast), "engine-global:blast");
                    free_image(
                        ptr::addr_of_mut!(kernel.explosion),
                        "engine-global:explosion",
                    );
                    free_image(ptr::addr_of_mut!(kernel.asteroid), "engine-global:asteroid");
                    for plane in kernel.star_planes.iter_mut() {
                        plane
                            .frames
                            .fill(crate::engine::battle::frames::FrameSelection::ABSENT);
                    }
                    let stars = &mut *ptr::addr_of_mut!(kernel.stars_in_space);
                    DestroyDrawable(ReleaseDrawable(*stars));
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
