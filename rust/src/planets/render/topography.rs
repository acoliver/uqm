//! Colored surface batches from `plangen.c::RenderTopography`.

use crate::comm::locdata::{CPoint, CRect};
use crate::graphics::cmap::Color;
use crate::planets::consts::{GAS_GIANT_ALGO, PLAN_ALGO_MASK};

pub const NUM_BATCH_POINTS: usize = 64;

#[derive(Clone, Copy, Default)]
pub struct TopographyPoint {
    pub point: CPoint,
    pub color: Color,
}

/// Borrowed resource bytes. The two palette header bytes precede RGB triples;
/// the first is the translation offset, not a color or an alpha component.
#[derive(Clone, Copy)]
pub struct TopographyPalette<'a> {
    pub translation: &'a [u8; 256],
    pub colors: &'a [u8],
    pub planet_type: u8,
    pub base_elevation: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopographyError {
    PlanetIndex,
    TranslationLength,
    Dimensions,
    DepthLength,
    PaletteHeader,
    PaletteIndex,
}

/// Existing context operations; drawing is infallible, as in C's void DrawBatch.
/// The frame is borrowed, never captured, destroyed or transferred here.
pub trait TopographyGfx {
    type Frame: Copy;
    fn set_frame(&mut self, frame: Self::Frame) -> Self::Frame;
    fn set_origin(&mut self, origin: CPoint) -> CPoint;
    fn clip(&mut self) -> CRect;
    fn set_clip(&mut self, clip: Option<CRect>);
    /// Slots `first..64` form the ascending linked list submitted to DrawBatch.
    fn batch(&mut self, points: &[TopographyPoint; NUM_BATCH_POINTS], first: usize);
}

impl TopographyPalette<'_> {
    fn elevation(self, depth: i8) -> usize {
        if self.planet_type & PLAN_ALGO_MASK == GAS_GIANT_ALGO {
            usize::from(depth as u8)
        } else {
            // SIZE is signed 16-bit: compound assignment narrows before clamping.
            i16::from(depth)
                .wrapping_add(self.base_elevation)
                .clamp(0, 255) as usize
        }
    }

    fn offset(self, depth: i8) -> Result<usize, TopographyError> {
        let index = self.translation[self.elevation(depth)]
            .checked_sub(self.colors[0])
            .ok_or(TopographyError::PaletteIndex)?;
        let offset = 2 + usize::from(index) * 3;
        if offset + 3 > self.colors.len() {
            return Err(TopographyError::PaletteIndex);
        }
        Ok(offset)
    }

    fn color(self, depth: i8) -> Color {
        let index = usize::from(self.translation[self.elevation(depth)] - self.colors[0]);
        let rgb = &self.colors[2 + index * 3..][..3];
        let channel = |v: u8| {
            let v = u16::from(v >> 1);
            ((v << 3) | (v >> 2)) as u8
        };
        Color::new(channel(rgb[0]), channel(rgb[1]), channel(rgb[2]))
    }
}

/// Render tightly packed signed elevations, reversing each 64-point batch.
/// There is no input wrapping or implicit pitch; frame clipping belongs to the
/// renderer. Origin and effective clip are saved after selecting the target,
/// then restored before restoring the old frame. Draw mode and foreground color
/// remain untouched. Missing translation still selects and restores the frame.
///
/// Positive dimensions must fit COORD. Nonpositive dimensions produce no points,
/// as in C. Invalid storage/resource input returns before any context mutation.
/// No graphics/resource failure is swallowed, and no RNG is consumed.
pub fn render_topography<G: TopographyGfx>(
    gfx: &mut G,
    destination: G::Frame,
    depth: &[i8],
    width: i32,
    height: i32,
    palette: Option<TopographyPalette<'_>>,
) -> Result<(), TopographyError> {
    let Some(palette) = palette else {
        let old = gfx.set_frame(destination);
        gfx.set_frame(old);
        return Ok(());
    };
    if width > i32::from(i16::MAX) || height > i32::from(i16::MAX) {
        return Err(TopographyError::Dimensions);
    }
    let count = width.max(0) as usize * height.max(0) as usize;
    if depth.len() < count {
        return Err(TopographyError::DepthLength);
    }
    if palette.colors.len() < 2 {
        return Err(TopographyError::PaletteHeader);
    }
    for &cell in &depth[..count] {
        palette.offset(cell)?;
    }
    let old_frame = gfx.set_frame(destination);
    let old_origin = gfx.set_origin(CPoint::default());
    let clip = gfx.clip();
    gfx.set_clip(None);
    let mut points = [TopographyPoint::default(); NUM_BATCH_POINTS];
    let mut first = NUM_BATCH_POINTS;
    for (index, &cell) in depth[..count].iter().enumerate() {
        first -= 1;
        points[first] = TopographyPoint {
            point: CPoint {
                x: (index % width as usize) as i16,
                y: (index / width as usize) as i16,
            },
            color: palette.color(cell),
        };
        if first == 0 {
            gfx.batch(&points, first);
            first = NUM_BATCH_POINTS;
        }
    }
    if first < NUM_BATCH_POINTS {
        gfx.batch(&points, first);
    }
    gfx.set_clip(Some(clip));
    gfx.set_origin(old_origin);
    gfx.set_frame(old_frame);
    Ok(())
}

#[cfg(test)]
#[path = "topography_tests.rs"]
mod tests;
