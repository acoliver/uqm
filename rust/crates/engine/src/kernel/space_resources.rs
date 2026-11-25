//! Battle-space graphics (stars, explosions, blasts and asteroids), counted so
//! nested users share one load, ported from `InitSpace`/`UninitSpace` (init.c).

#[cfg(not(test))]
use super::Kernel;
use crate::world::activity::ActivityKind;

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

/// Space graphics load only for melee, last battle and encounters.
fn loads_space_graphics(activity: u16) -> bool {
    activity as u8 <= ActivityKind::InEncounter as u8
}

#[cfg(not(test))]
impl Kernel {
    /// `InitSpace`: the first user loads the space graphics; later users only
    /// count. Returns `false` when a graphic fails to load.
    pub fn init_space(&mut self, activity: u16) -> bool {
        use crate::battle::frames::{load_animation, load_graphic};
        use crate::collections::observation::{observe_views, record, Event};
        let old = self.space_init_count;
        self.space_init_count = self.space_init_count.wrapping_add(1);
        if old == 0 && loads_space_graphics(activity) {
            let content = self.content();
            let stars = &mut self.stars_in_space;
            *stars = match load_graphic(&content, c"graphics.stars") {
                Ok(frame) => frame,
                Err(error) => {
                    log::error!("InitSpace: {error}");
                    return false;
                }
            };
            observe_views(std::slice::from_ref(stars), "engine-global:stars", |f| {
                f.bits()
            });
            if [&mut self.explosion, &mut self.blast, &mut self.asteroid]
                .into_iter()
                .zip(SPACE_ANIMATION_KEYS)
                .zip([
                    "engine-global:explosion",
                    "engine-global:blast",
                    "engine-global:asteroid",
                ])
                .any(|((frames, [big, medium, small]), role)| {
                    load_animation(
                        &content,
                        frames,
                        [Some(big), Some(medium), Some(small)],
                        role,
                    )
                    .map(|loaded| !loaded)
                    .unwrap_or_else(|error| {
                        log::error!("InitSpace: {error}");
                        true
                    })
                })
            {
                return false;
            }
        }
        record(Event::SpaceActive(true));
        true
    }

    /// `UninitSpace`: the last user releases the space graphics.
    pub fn uninit_space(&mut self) {
        use crate::battle::frames::{free_image, FrameSelection};
        use crate::collections::observation::{record, Event};
        if self.space_init_count == 0 {
            return;
        }
        self.space_init_count -= 1;
        if self.space_init_count != 0 {
            return;
        }
        record(Event::SpaceActive(false));
        free_image(&mut self.blast, "engine-global:blast");
        free_image(&mut self.explosion, "engine-global:explosion");
        free_image(&mut self.asteroid, "engine-global:asteroid");
        for plane in self.star_planes.iter_mut() {
            plane.frames.fill(FrameSelection::ABSENT);
        }
        free_image(
            std::array::from_mut(&mut self.stars_in_space),
            "engine-global:stars",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn space_graphics_load_for_melee_last_battle_and_encounters_only() {
        for activity in [0, 1, 2, 0x1002] {
            assert!(loads_space_graphics(activity), "{activity:#x}");
        }
        for activity in [3, 4, 7, 8, 0x1003] {
            assert!(!loads_space_graphics(activity), "{activity:#x}");
        }
    }

    #[test]
    fn animation_keys_keep_explosion_blast_asteroid_order_and_three_sizes() {
        assert_eq!(SPACE_ANIMATION_KEYS[0][0], c"graphics.boom.large");
        assert_eq!(SPACE_ANIMATION_KEYS[1][1], c"graphics.blast.medium");
        assert_eq!(SPACE_ANIMATION_KEYS[2][2], c"graphics.asteroid.small");
    }
}
