//! Conversion formats for serialized RAM-drawable rendering without a video driver.

use super::{ffi, sdl::lifecycle_abi};

pub struct OffscreenFormats {
    color: sdl2::surface::Surface<'static>,
    screen: sdl2::surface::Surface<'static>,
    owner: std::sync::Arc<std::thread::ThreadId>,
}

impl OffscreenFormats {
    pub fn new() -> Result<Self, String> {
        let owner = ffi::native_allocations()
            .owner
            .as_ref()
            .unwrap_or_else(|| panic!("graphics thread admission"))
            .clone();
        let color = sdl2::surface::Surface::new(1, 1, sdl2::pixels::PixelFormatEnum::RGBA8888)?;
        let screen = sdl2::surface::Surface::new(1, 1, sdl2::pixels::PixelFormatEnum::RGBX8888)?;
        Self::publish(&color, &screen, true);
        Ok(Self {
            color,
            screen,
            owner,
        })
    }

    fn publish(
        color: &sdl2::surface::Surface<'_>,
        screen: &sdl2::surface::Surface<'_>,
        publish: bool,
    ) {
        let surfaces = (color.raw().cast(), screen.raw().cast());
        let empty = (std::ptr::null_mut(), std::ptr::null_mut());
        let (expected, replacement) = if publish {
            (empty, surfaces)
        } else {
            (surfaces, empty)
        };
        // SAFETY: admission serializes publication; the caller owns both surfaces throughout it.
        unsafe {
            let current = (lifecycle_abi::format_conv_surf, lifecycle_abi::SDL_Screen);
            assert_eq!(
                current, expected,
                "offscreen format publication changed owners"
            );
            lifecycle_abi::format_conv_surf = replacement.0;
            lifecycle_abi::SDL_Screen = replacement.1;
        }
    }
}

impl Drop for OffscreenFormats {
    fn drop(&mut self) {
        assert_eq!(*self.owner, std::thread::current().id());
        Self::publish(&self.color, &self.screen, false);
    }
}
