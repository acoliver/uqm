//! Move-only ownership at the legacy FRAME boundary. Rust graphics registry
//! handles are not C frames and must never be cast into this interface.

use crate::c_bindings::global_state::FRAME_DESC;

/// A captured, detached C drawable. Dropping an untransferred definition
/// releases it; `LoadPlanet` takes ownership on the interactive path.
pub struct CapturedSurface {
    frame: *mut FRAME_DESC,
}

impl CapturedSurface {
    /// Load and detach a resource through the same loader used by C Sol.
    /// A missing external resource has a null frame, as in the original.
    ///
    /// # Safety
    /// Game thread, with the resource and graphics systems initialised.
    pub unsafe fn load(resource: &std::ffi::CStr) -> Self {
        Self {
            frame: super::CaptureDrawable((super::LoadGraphicInstance(resource.as_ptr())).cast())
                .cast(),
        }
    }

    /// Allocate a C-compatible pixmap, not a Rust registry handle.
    ///
    /// # Safety
    /// Game thread, with the graphics pixel format initialised.
    pub unsafe fn pixmap(width: i16, height: i16, frames: u16) -> Self {
        Self {
            frame: super::CaptureDrawable(super::CreateDrawable(2, width, height, frames)).cast(),
        }
    }

    /// A separately owned rescaled drawable. The source stays owned here.
    ///
    /// # Safety
    /// The drawable's graphics backend must be initialised on this thread.
    pub unsafe fn rescaled(&self, width: i32, height: i32) -> Self {
        Self {
            frame: super::CaptureDrawable(super::RescaleFrame(self.frame.cast(), width, height))
                .cast(),
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
        let count = super::GetFrameCount(self.frame.cast());
        let matches = (0..count).all(|index| {
            let frame = super::SetAbsFrameIndex(self.frame.cast(), index).cast::<FRAME_DESC>();
            ((*frame).Bounds.width, (*frame).Bounds.height) == (width, height)
        });
        if matches {
            return Ok(self);
        }
        let target = Self::pixmap(0, 0, count);
        if target.frame.is_null() {
            return Err("allocating normalized surface definition");
        }
        for index in 0..count {
            let source = super::SetAbsFrameIndex(self.frame.cast(), index).cast::<FRAME_DESC>();
            let scaled = if ((*source).Bounds.width, (*source).Bounds.height) == (width, height) {
                None
            } else {
                Some(Self::scale_definition_frame(source, width, height)?)
            };
            let frame = scaled.as_ref().map_or(source, |surface| surface.frame);
            let dest = super::SetAbsFrameIndex(target.frame.cast(), index).cast::<FRAME_DESC>();
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
        source: *mut FRAME_DESC,
        width: i16,
        height: i16,
    ) -> Result<Self, &'static str> {
        // NormalImg is the first field of tfb_image (tfb_draw.h), the same
        // prefix the existing PlanetSide mask adapter reads. It is an SDL
        // surface, not a Rust graphics-registry handle.
        let canvas = (*source).image.cast::<*mut sdl2::sys::SDL_Surface>().read();
        let palette = (*(*canvas).format).palette;
        if palette.is_null() {
            let scaled = Self {
                frame: super::CaptureDrawable(super::RescaleFrame(
                    source.cast(),
                    i32::from(width),
                    i32::from(height),
                ))
                .cast(),
            };
            if scaled.frame.is_null() {
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
        if result.frame.is_null() {
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
        (*result.frame).Type = (*source).Type;
        (*result.frame).Bounds.width = width;
        (*result.frame).Bounds.height = height;
        (*result.frame).HotSpot.x = (i32::from((*source).HotSpot.x) * i32::from(width)
            / i32::from((*source).Bounds.width)) as i16;
        (*result.frame).HotSpot.y = (i32::from((*source).HotSpot.y) * i32::from(height)
            / i32::from((*source).Bounds.height)) as i16;
        (*result.frame).image = super::TFB_DrawImage_New(surface.raw().cast());
        // The captured image now owns the SDL allocation, released by Drop or
        // the planet cleanup boundary. No Rust Surface owner may also free it.
        std::mem::forget(surface);
        Ok(result)
    }

    pub fn as_frame(&self) -> *mut FRAME_DESC {
        self.frame
    }

    /// Transfer ownership to the planet/surface code. After generation, either
    /// the original is `TopoFrame`, or C has destroyed it after rescaling and
    /// retained the new topography drawable. `FreePlanet` destroys TopoFrame.
    pub fn into_frame(mut self) -> *mut FRAME_DESC {
        std::mem::replace(&mut self.frame, std::ptr::null_mut())
    }
}

impl Drop for CapturedSurface {
    fn drop(&mut self) {
        // SAFETY: a non-null slot owns this detached drawable exactly once.
        // ReleaseDrawable returns its parent; it does not itself free memory.
        unsafe {
            super::DestroyDrawable(super::ReleaseDrawable(self.frame.cast()));
        }
    }
}

/// Transfer a definition only when `LoadPlanet` will consume it. The existing
/// DEBUG traversal seam returns before generation and must leave it owned here.
///
/// # Safety
/// Game thread, in a live orbit, with no outstanding system/RNG references.
pub unsafe fn load_planet_surface(surface: Option<CapturedSurface>) {
    #[cfg(feature = "debug-process")]
    if std::ptr::addr_of!(super::DISABLE_INTERACTIVITY).read() != 0 {
        super::LoadPlanet(
            surface
                .as_ref()
                .map_or(std::ptr::null_mut(), |s| s.as_frame().cast()),
        );
        return;
    }
    super::LoadPlanet(surface.map_or(std::ptr::null_mut(), |s| s.into_frame().cast()));
}
