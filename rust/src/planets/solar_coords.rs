//! Integer coordinate transforms used by interplanetary flight.
use crate::engine::graphics::native_abi::POINT;

pub(crate) fn star_seed(point: POINT) -> u32 {
    (u32::from(point.y as u16) << 16) | u32::from(point.x as u16)
}

#[derive(Clone, Copy)]
pub(crate) struct SolarCoordinates {
    width: i32,
    height: i32,
}

impl SolarCoordinates {
    pub fn new(screen: (i32, i32)) -> Self {
        Self {
            width: screen.0 - 78,
            height: screen.1 - 13,
        }
    }

    pub fn dimensions(self) -> (i32, i32) {
        (self.width, self.height)
    }

    pub fn display_factor(self) -> i32 {
        (self.width >> 1) - 8
    }

    pub fn to_display(self, point: POINT, radius: i16) -> POINT {
        let (width, height) = self.dimensions();
        let scale = self.display_factor() >> 1;
        POINT {
            x: (i64::from(width >> 1) + i64::from(point.x) * i64::from(scale) / i64::from(radius))
                as i16,
            y: (i64::from(height >> 1) + i64::from(point.y) * i64::from(scale) / i64::from(radius))
                as i16,
        }
    }

    pub fn to_location(self, point: POINT, radius: i16) -> POINT {
        let (width, height) = self.dimensions();
        let scale = self.display_factor() >> 1;
        POINT {
            x: ((i64::from(point.x) - i64::from(width >> 1)) * i64::from(radius) / i64::from(scale))
                as i16,
            y: ((i64::from(point.y) - i64::from(height >> 1)) * i64::from(radius)
                / i64::from(scale)) as i16,
        }
    }
}

pub(crate) fn outer_location(
    solar: &crate::engine::world::generation::SolarSnapshot,
    planet: u16,
) -> crate::graphics::native_abi::POINT {
    SolarCoordinates::new(solar.screen).to_location(
        solar.planets[usize::from(planet)].display_origin,
        solar.zoom_radius,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supplied_screen_controls_the_viewport_without_ambient_geometry() {
        let small = SolarCoordinates::new((320, 200));
        let large = SolarCoordinates::new((640, 360));
        assert_eq!(small.dimensions(), (242, 187));
        assert_eq!(large.dimensions(), (562, 347));
        assert_eq!(small.display_factor(), 113);
        assert_eq!(large.display_factor(), 273);
        for (geometry, center) in [(small, (121, 93)), (large, (281, 173))] {
            let point = geometry.to_display(POINT { x: 0, y: 0 }, 8192);
            assert_eq!((point.x, point.y), center);
            let origin = geometry.to_location(point, 8192);
            assert_eq!((origin.x, origin.y), (0, 0));
        }
    }

    #[test]
    fn star_seed_preserves_signed_coordinate_words() {
        for (x, y, expected) in [
            (-2, 0x1234, 0x1234_fffe),
            (0x1234, -2, 0xfffe_1234),
            (i16::MIN, i16::MAX, 0x7fff_8000),
            (0, 0, 0),
        ] {
            assert_eq!(star_seed(POINT { x, y }), expected);
        }
    }

    #[test]
    fn copied_outer_geometry_keeps_its_screen_zoom_and_body() {
        let mut world = crate::engine::World::new();
        world.set_catalog(std::borrow::Cow::Borrowed(
            &crate::planets::tables::STARMAP_ARRAY,
        ));
        let mut visit =
            crate::planets::solar_system::SolarSystem::new(&world, crate::engine::StarId(0));
        visit.sys.sun.radius = 8192;
        visit.sys.planets[2].image.origin = POINT { x: -1, y: 3 };
        let observation = visit.snapshot((320, 200));
        visit.sys.sun.radius = 4096;
        visit.sys.planets[2].image.origin.x = 37;
        let point = outer_location(&observation, 2);
        assert_eq!((point.x, point.y), (-17846, -13165));
    }

    #[test]
    fn center_and_cardinals_keep_the_original_truncation() {
        let center = SolarCoordinates::new((320, 200)).to_display(POINT { x: 0, y: 0 }, 8192);
        assert_eq!((center.x, center.y), (121, 93));
        let negative =
            SolarCoordinates::new((320, 200)).to_display(POINT { x: -8192, y: -1 }, 8192);
        assert_eq!((negative.x, negative.y), (65, 93));
        let corner = SolarCoordinates::new((320, 200)).to_location(POINT { x: 0, y: 0 }, 8192);
        assert_eq!((corner.x, corner.y), (-17700, -13604));
    }

    #[test]
    fn exact_scale_steps_round_trip_at_every_zoom() {
        for screen in [(320, 200), (640, 360), (960, 600)] {
            let geometry = SolarCoordinates::new(screen);
            let (width, height) = geometry.dimensions();
            let scale = ((width >> 1) - 8) >> 1;
            for radius in [1024, 2048, 4096, 8192] {
                for x in -3..=3 {
                    for y in -3..=3 {
                        let point = POINT {
                            x: (x * radius) as i16,
                            y: (y * radius) as i16,
                        };
                        let display = geometry.to_display(point, radius as i16);
                        assert_eq!(
                            (i32::from(display.x), i32::from(display.y)),
                            ((width >> 1) + x * scale, (height >> 1) + y * scale)
                        );
                        let restored = geometry.to_location(display, radius as i16);
                        assert_eq!((restored.x, restored.y), (point.x, point.y));
                    }
                }
            }
        }
    }
}
