//! The kernel's display resources: the screen, the drawing contexts and the
//! frames the status display draws from.

use std::ffi::CStr;

use crate::graphics::context::{ContextId, LoadedContext};
use crate::graphics::drawable::{FrameId, LoadedDrawable};
use crate::graphics::geometry::{Extent, Point, Rect};
use crate::graphics::state::{SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::graphics::{menu, palette_cycle::PaletteCycle};
use crate::kernel::Kernel;
use crate::math::battle_types::STATUS_WIDTH;

/// The status display's distance from the screen edge.
const SAFE_X: i32 = 0;
const SAFE_Y: i32 = 0;

fn create_context(name: &'static CStr) -> Option<ContextId> {
    Some(LoadedContext::create(name).into_id())
}

/// Release a context the kernel owns by id.
fn destroy_context(context: &mut Option<ContextId>) {
    drop(context.take().map(LoadedContext::from_id));
}

/// Release a drawable the kernel owns by its first frame.
fn destroy_frame(frame: &mut FrameId) {
    drop(LoadedDrawable::from_frame(std::mem::take(frame)));
}

fn load_frame(resource: &CStr) -> FrameId {
    LoadedDrawable::load(resource).map_or(FrameId::NULL, LoadedDrawable::into_frame)
}

impl Kernel {
    /// Start the video player and create the screen context and display.
    pub fn open_screen(&mut self) -> bool {
        // SAFETY: game thread; graphics and the resource system are initialized.
        unsafe { crate::video::control::InitVideoPlayer(1) };
        self.screen_context = create_context(c"ScreenContext");
        let (width, height) = crate::graphics::state::screen_dimensions();
        (self.screen_width, self.screen_height) = (width as i16, height as i16);
        self.screen = LoadedDrawable::display(3).into_frame();
        menu::select(self.screen_context);
        menu::set_frame(self.screen);
        menu::set_origin(Point { x: 0, y: 0 });
        true
    }

    /// Release the screen display and context and stop the video player.
    pub fn close_screen(&mut self) {
        destroy_frame(&mut self.screen);
        destroy_context(&mut self.screen_context);
        // SAFETY: game thread, after the game loop has stopped using the screen.
        unsafe { crate::video::control::UninitVideoPlayer() };
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
        let Some(map) = PaletteCycle::load(c"colortable.main") else {
            return false;
        };
        map.apply();
        true
    }

    /// Create the status, space and off-screen contexts; the status context is
    /// clipped to the status display.
    pub fn create_contexts(&mut self) -> bool {
        self.status_context = create_context(c"StatusContext");
        menu::select(self.status_context);
        menu::set_frame(self.screen);
        menu::set_clip(Rect {
            corner: Point {
                x: (SCREEN_WIDTH - STATUS_WIDTH - SAFE_X) as i16,
                y: SAFE_Y as i16,
            },
            extent: Extent {
                width: STATUS_WIDTH as i16,
                height: (SCREEN_HEIGHT - 2 * SAFE_Y) as i16,
            },
        });
        self.space_context = create_context(c"SpaceContext");
        self.off_screen_context = create_context(c"OffScreenContext");
        true
    }

    pub fn destroy_contexts(&mut self) {
        destroy_context(&mut self.off_screen_context);
        destroy_context(&mut self.space_context);
        destroy_context(&mut self.status_context);
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
        destroy_frame(&mut self.status_frame);
        destroy_frame(&mut self.activity_frame);
    }
}
