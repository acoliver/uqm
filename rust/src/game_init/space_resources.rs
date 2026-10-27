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
) -> impl Iterator<Item = (*mut [*mut frame_desc; 3], [&'static std::ffi::CStr; 3])> {
    [
        &raw mut live::explosion,
        &raw mut live::blast,
        &raw mut live::asteroid,
    ]
    .into_iter()
    .zip(SPACE_ANIMATION_KEYS)
}

#[cfg(not(test))]
fn animation_role(address: usize) -> &'static str {
    for (slot, role) in [
        (&raw mut live::explosion as usize, "engine-global:explosion"),
        (&raw mut live::blast as usize, "engine-global:blast"),
        (&raw mut live::asteroid as usize, "engine-global:asteroid"),
    ] {
        if address == slot {
            return role;
        }
    }
    "animation"
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
#[no_mangle]
pub unsafe extern "C" fn load_animation(
    frames: *mut *mut frame_desc,
    big: *const c_char,
    medium: *const c_char,
    small: *const c_char,
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
    observe_views(
        std::slice::from_raw_parts(frames, 3),
        animation_role(frames as usize),
        |frame| *frame as usize,
    );
    1
}

#[cfg(not(test))]
/// # Safety
/// Frames points to three exclusively borrowed live or null frame handles.
#[no_mangle]
pub unsafe extern "C" fn free_image(frames: *mut [*mut frame_desc; 3]) -> u32 {
    let frames = &mut *frames;
    let freed = release_images(frames, |frame| DestroyDrawable(ReleaseDrawable(frame)) != 0);
    observe_views(frames, animation_role(frames.as_ptr() as usize), |frame| {
        *frame as usize
    });
    freed as u32
}

#[cfg(not(test))]
pub(crate) mod live {
    #![allow(non_snake_case)]
    use super::*;

    #[no_mangle]
    pub static mut stars_in_space: *mut frame_desc = ptr::null_mut();
    #[no_mangle]
    pub static mut asteroid: [*mut frame_desc; 3] = [ptr::null_mut(); 3];
    #[no_mangle]
    pub static mut blast: [*mut frame_desc; 3] = [ptr::null_mut(); 3];
    #[no_mangle]
    pub static mut explosion: [*mut frame_desc; 3] = [ptr::null_mut(); 3];
    static mut SPACE_INIT_COUNT: u8 = 0;

    /// # Safety
    /// Game thread only, after graphics and resource initialization.
    #[no_mangle]
    pub unsafe extern "C" fn InitSpace() -> u32 {
        let old = SPACE_INIT_COUNT;
        SPACE_INIT_COUNT = SPACE_INIT_COUNT.wrapping_add(1);
        if old == 0
            && crate::game_init::macro_abi::get_current_activity() as u8
                <= crate::ships::native::abi::IN_ENCOUNTER as u8
        {
            let stars = &mut *ptr::addr_of_mut!(stars_in_space);
            *stars = CaptureDrawable(LoadGraphicInstance(c"graphics.stars".as_ptr()).cast());
            if stars.is_null() {
                return 0;
            }
            observe_views(std::slice::from_ref(stars), "engine-global:stars", |f| {
                *f as usize
            });
            if space_animations().any(|(frames, [big, medium, small])| {
                load_animation(frames.cast(), big.as_ptr(), medium.as_ptr(), small.as_ptr()) == 0
            }) {
                return 0;
            }
        }
        crate::collections::observation::record(
            crate::collections::observation::Event::SpaceActive(true),
        );
        1
    }

    /// # Safety
    /// Game thread only; pairs with InitSpace while graphics is live.
    #[no_mangle]
    pub unsafe extern "C" fn UninitSpace() {
        if SPACE_INIT_COUNT != 0 {
            SPACE_INIT_COUNT -= 1;
            if SPACE_INIT_COUNT == 0 {
                crate::collections::observation::record(
                    crate::collections::observation::Event::SpaceActive(false),
                );
                free_image(ptr::addr_of_mut!(blast));
                free_image(ptr::addr_of_mut!(explosion));
                free_image(ptr::addr_of_mut!(asteroid));
                let stars = &mut *ptr::addr_of_mut!(stars_in_space);
                DestroyDrawable(ReleaseDrawable(*stars));
                *stars = ptr::null_mut();
                observe_views(std::slice::from_ref(stars), "engine-global:stars", |f| {
                    *f as usize
                });
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
