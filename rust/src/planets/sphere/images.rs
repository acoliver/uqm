use crate::engine::graphics::drawable_abi::{DrawableLease, LoadedDrawable};

pub(crate) fn image(flags: u8, width: usize, height: usize, count: u16) -> LoadedDrawable {
    let width =
        i16::try_from(width).unwrap_or_else(|_| panic!("orbit image width exceeds native bounds"));
    let height = i16::try_from(height)
        .unwrap_or_else(|_| panic!("orbit image height exceeds native bounds"));
    LoadedDrawable::create(flags, width, height, count)
        .unwrap_or_else(|| panic!("allocating orbit image"))
}

pub(crate) struct OrbitImages {
    // Field drop order preserves the image teardown sequence; construction keeps allocation order.
    pub throb: Option<LoadedDrawable>,
    pub topography: Option<LoadedDrawable>,
    pub zoom: LoadedDrawable,
    pub sphere: LoadedDrawable,
    pub tint: LoadedDrawable,
    pub shield: Option<LoadedDrawable>,
    pub tint_color: crate::comm::locdata::CColor,
}

impl OrbitImages {
    pub fn new(width: usize) -> Self {
        Self {
            sphere: image(2 | 8, 75, 75, 2),
            tint: image(2, width, 75, 1),
            zoom: image(2, width * 4, 75 * 4, 1),
            topography: None,
            shield: None,
            throb: None,
            tint_color: crate::comm::locdata::CColor {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
        }
    }

    pub fn topography(&self) -> DrawableLease {
        self.topography
            .as_ref()
            .unwrap_or_else(|| panic!("orbit topography is not generated"))
            .lease()
    }

    pub fn object(&self) -> Option<DrawableLease> {
        self.throb
            .as_ref()
            .or(self.shield.as_ref())
            .map(LoadedDrawable::lease)
    }
}

pub(crate) fn required_frame(
    drawable: &DrawableLease,
    index: usize,
) -> crate::engine::graphics::native_abi::FRAME {
    drawable
        .frame(index)
        .unwrap_or_else(|| panic!("orbit image frame {index} is not initialized"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::collections::observation::{self, Event};

    #[test]
    #[serial_test::serial]
    fn orbit_images_release_in_the_original_teardown_order() {
        let mut game = crate::engine::Game::new();
        let display = crate::campaign_display_test::CampaignDisplay::install(&mut game).unwrap();
        observation::enable();
        observation::drain();
        let mut images = OrbitImages::new(4);
        images.topography = Some(image(2, 4, 75, 1));
        images.shield = Some(image(2 | 8, 89, 89, 1));
        images.throb = Some(image(2 | 8, 89, 89, 2));
        let allocated: Vec<_> = observation::drain()
            .into_iter()
            .filter_map(|event| match event {
                Event::Drawable(address) => Some(address),
                _ => None,
            })
            .collect();
        assert_eq!(allocated.len(), 6);
        drop(images);
        let released: Vec<_> = observation::drain()
            .into_iter()
            .filter_map(|event| match event {
                Event::DrawableFree(address) => Some(address),
                _ => None,
            })
            .collect();
        assert_eq!(released, [5, 3, 2, 0, 1, 4].map(|index| allocated[index]));
        display.finish(game);
        observation::drain();
    }
}
