//! Native surface definitions owned from acquisition through orbit generation.

use crate::engine::graphics::drawable_abi::LoadedDrawable;
use crate::graphics::native_abi::FRAME_DESC;

/// A detached native definition, retaining absence for a missing external resource.
pub struct CapturedSurface {
    drawable: Option<LoadedDrawable>,
}

impl CapturedSurface {
    /// Load and detach a native graphic resource.
    /// A missing external resource has a null frame, as in the original.
    ///
    /// # Safety
    /// Game thread, with the resource and graphics systems initialised.
    pub unsafe fn load(resource: &std::ffi::CStr) -> Self {
        Self {
            drawable: LoadedDrawable::load(resource),
        }
    }

    /// Allocate an owned native pixmap.
    ///
    /// # Safety
    /// Game thread, with the graphics pixel format initialised.
    pub unsafe fn pixmap(width: i16, height: i16, frames: u16) -> Self {
        Self {
            drawable: LoadedDrawable::create(2, width, height, frames),
        }
    }

    /// A separately owned rescaled drawable. The source stays owned here.
    ///
    /// # Safety
    /// The drawable's graphics backend must be initialised on this thread.
    pub unsafe fn rescaled(&self, width: i32, height: i32) -> Self {
        Self {
            drawable: self
                .drawable
                .as_ref()
                .and_then(|drawable| drawable.rescaled(0, width, height)),
        }
    }

    /// Normalize every definition frame while its ownership is still in Rust.
    /// Legacy surface generation retains frame zero but does not release a
    /// separately rescaled elevation frame. Passing matching extents avoids
    /// that unowned temporary, including for external replacement Earth art.
    ///
    /// # Safety
    /// Game thread, with a live graphics backend and exclusive drawable ownership.
    pub unsafe fn into_definition(self, width: i16, height: i16) -> Result<Self, &'static str> {
        let count = super::GetFrameCount(self.as_frame().cast());
        let matches = (0..count).all(|index| {
            let frame = super::SetAbsFrameIndex(self.as_frame().cast(), index).cast::<FRAME_DESC>();
            ((*frame).Bounds.width, (*frame).Bounds.height) == (width, height)
        });
        if matches {
            return Ok(self);
        }
        let target = Self::pixmap(0, 0, count);
        if target.as_frame().is_null() {
            return Err("allocating normalized surface definition");
        }
        for index in 0..count {
            let source =
                super::SetAbsFrameIndex(self.as_frame().cast(), index).cast::<FRAME_DESC>();
            let scaled = if ((*source).Bounds.width, (*source).Bounds.height) == (width, height) {
                None
            } else {
                Some(self.scale_definition_frame(index, width, height)?)
            };
            let frame = scaled.as_ref().map_or(source, |surface| surface.as_frame());
            let dest =
                super::SetAbsFrameIndex(target.as_frame().cast(), index).cast::<FRAME_DESC>();
            // Move the image, keeping the destination's own parent and index.
            // Matching frames keep their indexed elevation pixels unchanged.
            (*dest).Type = (*frame).Type;
            (*dest).HotSpot = (*frame).HotSpot;
            (*dest).Bounds = (*frame).Bounds;
            (*dest).image = std::mem::replace(&mut (*frame).image, std::ptr::null_mut());
        }
        Ok(target)
    }

    unsafe fn scale_definition_frame(
        &self,
        index: u16,
        width: i16,
        height: i16,
    ) -> Result<Self, &'static str> {
        let source = super::SetAbsFrameIndex(self.as_frame().cast(), index).cast::<FRAME_DESC>();
        // NormalImg is the first field of tfb_image (tfb_draw.h), the same
        // prefix the existing PlanetSide mask adapter reads. It is an SDL
        // surface, not a Rust graphics-registry handle.
        let canvas = (*source).image.cast::<*mut sdl2::sys::SDL_Surface>().read();
        let palette = (*(*canvas).format).palette;
        if palette.is_null() {
            let scaled = Self {
                drawable: self.drawable.as_ref().and_then(|drawable| {
                    drawable.rescaled(usize::from(index), i32::from(width), i32::from(height))
                }),
            };
            if scaled.as_frame().is_null() {
                return Err("rescaling surface definition");
            }
            return Ok(scaled);
        }
        // RescaleFrame allocates a truecolor target. Its nearest scaler only
        // handles equal pixel depths, so indexed elevation needs an indexed
        // target and must retain its palette/index semantics.
        let surface = sdl2::surface::Surface::new(
            width as u32,
            height as u32,
            sdl2::pixels::PixelFormatEnum::Index8,
        )
        .map_err(|_| "allocating indexed surface definition")?;
        if sdl2::sys::SDL_SetPaletteColors(
            (*(*surface.raw()).format).palette,
            (*palette).colors,
            0,
            (*palette).ncolors,
        ) != 0
        {
            return Err("copying surface definition palette");
        }
        let result = Self::pixmap(0, 0, 1);
        if result.as_frame().is_null() {
            return Err("allocating indexed definition frame");
        }
        super::TFB_DrawCanvas_Rescale_Nearest(
            canvas.cast(),
            surface.raw().cast(),
            -1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        (*result.as_frame()).Type = (*source).Type;
        (*result.as_frame()).Bounds.width = width;
        (*result.as_frame()).Bounds.height = height;
        (*result.as_frame()).HotSpot.x = (i32::from((*source).HotSpot.x) * i32::from(width)
            / i32::from((*source).Bounds.width)) as i16;
        (*result.as_frame()).HotSpot.y = (i32::from((*source).HotSpot.y) * i32::from(height)
            / i32::from((*source).Bounds.height)) as i16;
        (*result.as_frame()).image = super::TFB_DrawImage_New(surface.raw().cast());
        // The captured image now owns the SDL allocation, released by Drop or
        // the planet cleanup boundary. No Rust Surface owner may also free it.
        std::mem::forget(surface);
        Ok(result)
    }

    pub fn as_frame(&self) -> *mut FRAME_DESC {
        self.drawable
            .as_ref()
            .and_then(|drawable| drawable.lease().frame(0))
            .unwrap_or(std::ptr::null_mut())
    }

    /// Transfer release authority into the per-orbit image state.
    pub fn into_drawable(self) -> Option<LoadedDrawable> {
        self.drawable
    }
}
