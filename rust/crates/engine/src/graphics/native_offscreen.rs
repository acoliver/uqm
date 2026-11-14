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
        // SAFETY: registry admission serializes graphics access; these surfaces remain owned here.
        unsafe {
            assert!(
                lifecycle_abi::format_conv_surf.is_null() && lifecycle_abi::SDL_Screen.is_null(),
                "offscreen formats cannot replace an active display"
            );
            lifecycle_abi::format_conv_surf = color.raw().cast();
            lifecycle_abi::SDL_Screen = screen.raw().cast();
        }
        Ok(Self {
            color,
            screen,
            owner,
        })
    }
}

impl Drop for OffscreenFormats {
    fn drop(&mut self) {
        assert_eq!(*self.owner, std::thread::current().id());
        // SAFETY: this owner is the only publisher, and unpublishes before its SDL surfaces drop.
        unsafe {
            let format = lifecycle_abi::format_conv_surf;
            assert_eq!(format, self.color.raw().cast());
            let screen = lifecycle_abi::SDL_Screen;
            assert_eq!(screen, self.screen.raw().cast());
            lifecycle_abi::format_conv_surf = std::ptr::null_mut();
            lifecycle_abi::SDL_Screen = std::ptr::null_mut();
        }
    }
}
