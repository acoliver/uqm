//! Space animation ownership, ported from init.c (GPL-2.0-or-later).

#[cfg(not(test))]
use crate::collections::observation::observe_views;

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

#[cfg(not(test))]
use crate::engine::battle::frames::{free_image, load_animation, load_graphic};

#[cfg(not(test))]
pub(crate) mod live {
    use super::*;

    pub fn init_space(kernel: &mut crate::engine::Kernel, activity: u16) -> u32 {
        {
            let old = kernel.space_init_count;
            kernel.space_init_count = kernel.space_init_count.wrapping_add(1);
            if old == 0 && activity as u8 <= crate::ships::native::abi::IN_ENCOUNTER as u8 {
                let stars = &mut kernel.stars_in_space;
                *stars = load_graphic(c"graphics.stars");
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
                    for plane in kernel.star_planes.iter_mut() {
                        plane
                            .frames
                            .fill(crate::engine::battle::frames::FrameSelection::ABSENT);
                    }
                    free_image(std::array::from_mut(stars), "engine-global:stars");
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
    fn animation_keys_keep_explosion_blast_asteroid_order_and_three_sizes() {
        assert_eq!(SPACE_ANIMATION_KEYS[0][0], c"graphics.boom.large");
        assert_eq!(SPACE_ANIMATION_KEYS[1][1], c"graphics.blast.medium");
        assert_eq!(SPACE_ANIMATION_KEYS[2][2], c"graphics.asteroid.small");
    }
}

#[cfg(not(test))]
pub use live::*;
