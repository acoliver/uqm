//! Planet surface definitions owned from loading through orbit generation.
use super::canvas::scale::rescale_nearest;
use super::canvas::Canvas;
use super::drawable::{FrameId, LoadedDrawable, WANT_PIXMAP};
use super::geometry::{Extent, HotSpot};
use super::image::Image;
use super::state;

/// A loaded surface definition; a missing external resource leaves it
/// empty, as in the original.
#[derive(Debug)]
pub struct CapturedSurface {
    drawable: Option<LoadedDrawable>,
}

impl CapturedSurface {
    /// Load a graphic resource.
    pub fn load(content: &crate::resource::Content, resource: &std::ffi::CStr) -> Self {
        Self {
            drawable: LoadedDrawable::load(content, resource).ok(),
        }
    }

    /// An owned pixmap.
    pub fn pixmap(width: i16, height: i16, frames: u16) -> Self {
        Self {
            drawable: LoadedDrawable::create(WANT_PIXMAP, width, height, frames),
        }
    }

    /// A separately owned rescaled drawable; the source stays owned here.
    pub fn rescaled(&self, width: i32, height: i32) -> Self {
        Self {
            drawable: self
                .drawable
                .as_ref()
                .and_then(|drawable| drawable.rescaled(0, width, height)),
        }
    }

    /// Resize every definition frame before surface generation, keeping
    /// indexed palettes.
    ///
    /// # Panics
    /// Panics if the frames are held elsewhere: normalization moves their
    /// images into a new drawable.
    pub fn into_definition(self, width: i16, height: i16) -> Result<Self, &'static str> {
        let Some(source) = &self.drawable else {
            return Ok(self);
        };
        assert!(
            source.is_exclusive(),
            "surface definition has retained frame readers"
        );
        let first = source.first_frame();
        let count = first.count();
        let size = Extent::new(width, height);
        let frames: Vec<FrameId> = (0..count).map(|index| first.at(index)).collect();
        if frames
            .iter()
            .all(|frame| frame.rect().is_some_and(|rect| rect.extent == size))
        {
            return Ok(self);
        }
        let target = Self::pixmap(0, 0, count);
        let target_first = target.first_frame();
        for (index, &frame) in frames.iter().enumerate() {
            let dest = target_first.at(index as u16);
            if frame.rect().is_some_and(|rect| rect.extent == size) {
                // Matching frames keep their indexed elevation pixels.
                state::with(|g| {
                    let moved = std::mem::take(g.frame_mut(frame));
                    *g.frame_mut(dest) = moved;
                });
            } else {
                let scaled = self.scale_definition_frame(frame, width, height)?;
                state::with(|g| {
                    let moved = std::mem::take(g.frame_mut(scaled.first_frame()));
                    *g.frame_mut(dest) = moved;
                });
            }
        }
        Ok(target)
    }

    /// One frame scaled to the definition size: true-colour frames through
    /// `RescaleFrame`, indexed ones onto an indexed canvas with the same
    /// palette.
    fn scale_definition_frame(
        &self,
        frame: FrameId,
        width: i16,
        height: i16,
    ) -> Result<Self, &'static str> {
        let indexed = state::with(|g| g.frame_image(frame).canvas().is_indexed());
        if !indexed {
            let scaled = Self {
                drawable: self.drawable.as_ref().and_then(|drawable| {
                    drawable.rescaled(
                        usize::from(frame.index()),
                        i32::from(width),
                        i32::from(height),
                    )
                }),
            };
            if scaled.first_frame().is_null() {
                return Err("rescaling surface definition");
            }
            return Ok(scaled);
        }
        let result = Self::pixmap(0, 0, 1);
        state::with(|g| {
            let source = g.frame(frame).clone();
            let canvas = g.frame_image(frame).canvas();
            let mut scaled = Canvas::new(
                super::canvas::PixelFormat::Indexed,
                i32::from(width),
                i32::from(height),
            );
            scaled.set_palette(
                canvas
                    .palette()
                    .unwrap_or_else(|| unreachable!("an indexed canvas has a palette")),
            );
            let (mut extent, mut hot) = (Extent::zero(), HotSpot::default());
            rescale_nearest(
                canvas,
                &mut scaled,
                -1,
                HotSpot::default(),
                &mut extent,
                &mut hot,
            );
            let image = g.images.insert(Image::new(scaled));
            let target = g.frame_mut(result.first_frame());
            target.kind = source.kind;
            target.bounds = Extent::new(width, height);
            target.hot_spot = HotSpot::new(
                (i32::from(source.hot_spot.x) * i32::from(width) / i32::from(source.bounds.width))
                    as i16,
                (i32::from(source.hot_spot.y) * i32::from(height) / i32::from(source.bounds.height))
                    as i16,
            );
            target.image = Some(image);
        });
        Ok(result)
    }

    /// The first frame, or the null frame for a missing resource.
    pub fn first_frame(&self) -> FrameId {
        self.drawable
            .as_ref()
            .map_or(FrameId::NULL, LoadedDrawable::first_frame)
    }

    /// Transfer release authority into the per-orbit image state.
    pub fn into_drawable(self) -> Option<LoadedDrawable> {
        self.drawable
    }
}
