//! The kernel's display resources: the screen, the drawing contexts and the
//! frames the status display draws from.
//!
//! Each call creates or releases resources through the graphics layer's
//! pointer API, which is `unsafe` because it requires the game thread. The
//! kernel is only ever driven from that thread, so these methods are safe to
//! call and the graphics owner turns their bodies safe when its API is.

use std::ffi::CStr;

use crate::graphics::gfx_common_abi::{ScreenHeight, ScreenWidth};
use crate::graphics::native_abi::{self as gfx, CONTEXT};
use crate::graphics::{cmap_abi, context_abi, drawable_abi, pixmap_abi};
use crate::kernel::Kernel;
use crate::math::battle_types::STATUS_WIDTH;
use crate::strings::sresins::LoadStringTableInstance;
use crate::strings::string_table::{CaptureStringTable, DestroyStringTable, ReleaseStringTable};

/// The status display's distance from the screen edge.
const SAFE_X: i32 = 0;
const SAFE_Y: i32 = 0;

fn create_context(name: &CStr) -> CONTEXT {
    // SAFETY: game thread; the name is a terminated string.
    unsafe {
        #[cfg(feature = "debug-process")]
        {
            context_abi::CreateContextAux(name.as_ptr()).cast()
        }
        #[cfg(not(feature = "debug-process"))]
        {
            let _ = name;
            context_abi::CreateContextAux().cast()
        }
    }
}

fn load_frame(resource: &CStr) -> gfx::FRAME {
    // SAFETY: game thread; the resource system is initialized and the key is terminated.
    unsafe {
        pixmap_abi::CaptureDrawable(
            crate::graphics::resgfx::LoadGraphicInstance(resource.as_ptr()).cast(),
        )
        .cast()
    }
}

impl Kernel {
    /// Start the video player and create the screen context and display.
    pub fn open_screen(&mut self) -> bool {
        // SAFETY: game thread; graphics and the resource system are initialized.
        unsafe { crate::video::control::InitVideoPlayer(1) };
        self.screen_context = create_context(c"ScreenContext");
        if self.screen_context.is_null() {
            return false;
        }
        // SAFETY: game thread; the dimensions are fields of this kernel and the
        // display keeps writing them only while this kernel is live.
        unsafe {
            self.screen = pixmap_abi::CaptureDrawable(drawable_abi::CreateDisplay(
                3,
                std::ptr::addr_of_mut!(self.screen_width),
                std::ptr::addr_of_mut!(self.screen_height),
            ))
            .cast();
            if self.screen.is_null() {
                return false;
            }
            context_abi::SetContext(self.screen_context.cast());
            drawable_abi::SetContextFGFrame(self.screen.cast());
            context_abi::SetContextOrigin(gfx::POINT { x: 0, y: 0 });
        }
        true
    }

    /// Release the screen display and context and stop the video player.
    pub fn close_screen(&mut self) {
        // SAFETY: game thread, after the game loop has stopped using the screen.
        unsafe {
            drawable_abi::DestroyDrawable(pixmap_abi::ReleaseDrawable(self.screen));
            self.screen = std::ptr::null_mut();
            context_abi::DestroyContext(self.screen_context.cast());
            self.screen_context = std::ptr::null_mut();
            crate::video::control::UninitVideoPlayer();
        }
    }

    pub fn init_resources(&mut self) -> bool {
        // SAFETY: game thread, once, before any resource is loaded.
        self.h_res_index = unsafe { crate::resource::ffi_bridge::InitResourceSystem() };
        !self.h_res_index.is_null()
    }

    pub fn uninit_resources(&mut self) {
        // SAFETY: game thread, after every resource owner has released its resources.
        unsafe { crate::resource::ffi_bridge::UninitResourceSystem() };
    }

    /// Make `colortable.main` the active colour map.
    pub fn load_main_colormap(&mut self) -> bool {
        // SAFETY: game thread; the resource system is initialized, the key is
        // terminated, and the table is released after the map is installed.
        unsafe {
            let map = CaptureStringTable(LoadStringTableInstance(c"colortable.main".as_ptr()));
            if map.is_null() {
                return false;
            }
            cmap_abi::SetColorMap(cmap_abi::GetColorMapAddress(map.cast()));
            DestroyStringTable(ReleaseStringTable(map));
        }
        true
    }

    /// Create the status, space and off-screen contexts; the status context is
    /// clipped to the status display.
    pub fn create_contexts(&mut self) -> bool {
        self.status_context = create_context(c"StatusContext");
        if self.status_context.is_null() {
            return false;
        }
        // SAFETY: game thread; the contexts and the screen were created by this kernel.
        unsafe {
            context_abi::SetContext(self.status_context.cast());
            drawable_abi::SetContextFGFrame(self.screen.cast());
            let mut rect = gfx::RECT {
                corner: gfx::POINT {
                    x: (ScreenWidth - STATUS_WIDTH - SAFE_X) as i16,
                    y: SAFE_Y as i16,
                },
                extent: gfx::EXTENT {
                    width: STATUS_WIDTH as i16,
                    height: (ScreenHeight - 2 * SAFE_Y) as i16,
                },
            };
            context_abi::SetContextClipRect(&mut rect);
        }
        self.space_context = create_context(c"SpaceContext");
        if self.space_context.is_null() {
            return false;
        }
        self.off_screen_context = create_context(c"OffScreenContext");
        !self.off_screen_context.is_null()
    }

    pub fn destroy_contexts(&mut self) {
        // SAFETY: game thread, after every element and mode has stopped drawing.
        unsafe {
            context_abi::DestroyContext(self.off_screen_context.cast());
            context_abi::DestroyContext(self.space_context.cast());
            context_abi::DestroyContext(self.status_context.cast());
        }
    }

    /// Load the activity and status frames.
    pub fn load_status_frames(&mut self) -> bool {
        self.activity_frame = load_frame(c"graphics.activity");
        if self.activity_frame.is_null() {
            return false;
        }
        self.status_frame = load_frame(c"graphics.status");
        !self.status_frame.is_null()
    }

    pub fn release_status_frames(&mut self) {
        // SAFETY: game thread, after the status display has stopped drawing.
        unsafe {
            drawable_abi::DestroyDrawable(pixmap_abi::ReleaseDrawable(self.status_frame));
            drawable_abi::DestroyDrawable(pixmap_abi::ReleaseDrawable(self.activity_frame));
        }
        self.status_frame = std::ptr::null_mut();
        self.activity_frame = std::ptr::null_mut();
    }
}
