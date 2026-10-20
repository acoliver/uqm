use super::*;
use crate::comm::locdata::{CPoint, CRect};
use crate::graphics::cmap::Color;
use crate::planets::consts::GAS_GIANT_ALGO;

// This fake proves dependency calls only, not rendering or resource effects.
// Real RAM frames and context effects are checked in linked parity.
#[derive(Default)]
struct Recording {
    calls: Vec<&'static str>,
    batches: Vec<Vec<(i16, i16, Color)>>,
}

impl TopographyGfx for Recording {
    type Frame = u8;
    fn set_frame(&mut self, _: u8) -> u8 {
        self.calls.push("frame");
        3
    }
    fn set_origin(&mut self, _: CPoint) -> CPoint {
        self.calls.push("origin");
        CPoint { x: 7, y: -9 }
    }
    fn clip(&mut self) -> CRect {
        self.calls.push("clip");
        CRect::default()
    }
    fn set_clip(&mut self, _: Option<CRect>) {
        self.calls.push("set_clip");
    }
    fn batch(&mut self, points: &[TopographyPoint; 64], first: usize) {
        self.calls.push("batch");
        self.batches.push(
            points[first..]
                .iter()
                .map(|p| (p.point.x, p.point.y, p.color))
                .collect(),
        );
    }
}

#[test]
fn batches_reverse_across_rows_and_keep_every_tail() {
    let xlat = [10; 256];
    let palette = TopographyPalette {
        translation: &xlat,
        colors: &[10, 99, 63, 64, 255],
        planet_type: 0,
        base_elevation: 0,
    };
    for count in [0, 1, 63, 64, 65, 127, 128, 129] {
        let mut gfx = Recording::default();
        render_topography(&mut gfx, 9, &vec![0; count], 1, count as i32, Some(palette)).unwrap();
        let points: Vec<_> = gfx.batches.iter().flatten().collect();
        let expected: Vec<_> = (0..count)
            .collect::<Vec<_>>()
            .chunks(64)
            .flat_map(|chunk| chunk.iter().rev().copied())
            .collect();
        assert_eq!(
            points.iter().map(|p| p.1 as usize).collect::<Vec<_>>(),
            expected
        );
        assert!(points
            .iter()
            .all(|p| p.0 == 0 && p.2 == Color::new(255, 8, 255)));
        assert_eq!(&gfx.calls[..4], ["frame", "origin", "clip", "set_clip"]);
        assert_eq!(
            &gfx.calls[gfx.calls.len() - 3..],
            ["set_clip", "origin", "frame"]
        );
    }
}

#[test]
fn bad_inputs_fail_before_effects_including_late_palette_error() {
    let xlat = std::array::from_fn(|i| i as u8);
    let palette = TopographyPalette {
        translation: &xlat,
        colors: &[0, 255, 0, 0, 0],
        planet_type: GAS_GIANT_ALGO,
        base_elevation: 0,
    };
    for (width, height, depth, colors, error) in [
        (
            32768,
            1,
            vec![],
            palette.colors,
            TopographyError::Dimensions,
        ),
        (
            1,
            32768,
            vec![],
            palette.colors,
            TopographyError::Dimensions,
        ),
        (2, 1, vec![0], palette.colors, TopographyError::DepthLength),
        (1, 1, vec![0], &[][..], TopographyError::PaletteHeader),
        (
            1,
            1,
            vec![0],
            &[1, 1, 0, 0, 0][..],
            TopographyError::PaletteIndex,
        ),
        (
            65,
            1,
            [vec![0; 64], vec![1]].concat(),
            palette.colors,
            TopographyError::PaletteIndex,
        ),
    ] {
        let mut gfx = Recording::default();
        let before = depth.clone();
        assert_eq!(
            render_topography(
                &mut gfx,
                9,
                &depth,
                width,
                height,
                Some(TopographyPalette { colors, ..palette })
            ),
            Err(error)
        );
        assert!(gfx.calls.is_empty());
        assert_eq!(depth, before);
    }
}

#[test]
fn missing_translation_does_not_inspect_inputs_or_change_origin_clip() {
    let mut gfx = Recording::default();
    render_topography(&mut gfx, 9, &[], i32::MAX, i32::MAX, None).unwrap();
    assert_eq!(gfx.calls, ["frame", "frame"]);
}

#[test]
fn signed_size_narrows_before_clamp_and_gas_ignores_base() {
    let xlat = std::array::from_fn(|i| i as u8);
    let palette = TopographyPalette {
        translation: &xlat,
        colors: &[],
        planet_type: 0,
        base_elevation: i16::MAX,
    };
    assert_eq!(palette.elevation(1), 0);
    assert_eq!(
        TopographyPalette {
            base_elevation: i16::MIN,
            ..palette
        }
        .elevation(-1),
        255
    );
    assert_eq!(
        TopographyPalette {
            planet_type: GAS_GIANT_ALGO | 0xf3,
            ..palette
        }
        .elevation(-1),
        255
    );
    assert_eq!(
        TopographyPalette {
            base_elevation: 128,
            ..palette
        }
        .elevation(-128),
        0
    );
    assert_eq!(
        TopographyPalette {
            base_elevation: 128,
            ..palette
        }
        .elevation(127),
        255
    );
}
