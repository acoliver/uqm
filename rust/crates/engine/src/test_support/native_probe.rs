//! Safe probes over process-global native graphics state.
//!
//! Every fixture that calls these is `#[serial]` and owns the display, queue and
//! frames it probes, which is the invariant each wrapped call needs.
//!
//! Frames and contexts arrive as the engine's raw handle types, taken from its own
//! safe lease and drawable APIs, never fabricated by a test.
#![allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "handles come from live engine leases; the dependents that call these forbid unsafe"
)]

use crate::graphics::{
    context_abi, dcqueue_abi, drawable_abi, gfx_common_abi,
    native_abi::{Color, DrawMode, CONTEXT, FONT, FRAME, POINT},
};

/// Run every queued draw command.
pub fn flush_graphics() {
    // SAFETY: the serialized fixture owns the initialized graphics queue.
    unsafe { dcqueue_abi::TFB_FlushGraphics() };
}

/// Initialized draw-command queue, released on drop.
pub struct DrawQueue(());

impl DrawQueue {
    pub fn init() -> Self {
        // SAFETY: the serialized fixture owns the queue until this guard drops.
        unsafe { dcqueue_abi::Init_DrawCommandQueue() };
        Self(())
    }
}

impl Drop for DrawQueue {
    fn drop(&mut self) {
        // SAFETY: the guard owns the queue it initialized.
        unsafe { dcqueue_abi::Uninit_DrawCommandQueue() };
    }
}

pub fn batch_graphics() {
    // SAFETY: the serialized fixture owns an initialized queue.
    unsafe { gfx_common_abi::BatchGraphics() };
}

pub fn unbatch_graphics() {
    // SAFETY: the serialized fixture owns an initialized queue.
    unsafe { gfx_common_abi::UnbatchGraphics() };
}

pub fn batch_depth() -> i32 {
    // SAFETY: the serialized fixture owns an initialized queue.
    unsafe { gfx_common_abi::GetBatchDepth() }
}

/// Read a `width` by `height` RAM frame as colors. `None` when the frame is absent.
pub fn read_pixels(frame: FRAME, width: i32, height: i32) -> Option<Vec<Color>> {
    let mut pixels = vec![Color::default(); width as usize * height as usize];
    // SAFETY: the fixture retains the non-screen frame and the buffer covers width * height colors.
    let read =
        unsafe { drawable_abi::ReadFramePixelColors(frame, pixels.as_mut_ptr(), width, height) };
    (read != 0).then_some(pixels)
}

/// Overwrite a `width` by `height` RAM frame. `false` when the frame is absent.
pub fn write_pixels(frame: FRAME, pixels: &[Color], width: i32, height: i32) -> bool {
    assert_eq!(pixels.len(), width as usize * height as usize);
    // SAFETY: the fixture retains the non-screen frame and the buffer covers width * height colors.
    unsafe { drawable_abi::WriteFramePixelColors(frame, pixels.as_ptr(), width, height) != 0 }
}

pub fn frame_pixel(frame: FRAME, at: POINT) -> Color {
    // SAFETY: the fixture retains the frame and `at` lies inside it.
    unsafe { drawable_abi::GetFramePixel(frame, at) }
}

/// The currently selected context, or null.
pub fn selected_context() -> CONTEXT {
    // SAFETY: context selection is serialized by the fixture.
    unsafe { context_abi::_pCurContext }
}

/// The font a context currently holds.
pub fn context_font(context: CONTEXT) -> FONT {
    // SAFETY: the fixture retains the registered context.
    unsafe { (*context).Font }
}

/// Draw mode of the selected context.
pub fn selected_draw_mode() -> DrawMode {
    // SAFETY: the fixture has selected a retained context.
    unsafe { context_abi::GetContextDrawMode() }
}

/// Set the graphic scale and return the previous one.
pub fn set_graphic_scale(scale: i32) -> i32 {
    // SAFETY: the serialized fixture owns native drawing state.
    unsafe { gfx_common_abi::set_graphic_scale(scale) }
}

pub fn graphic_scale() -> i32 {
    // SAFETY: the serialized fixture owns native drawing state.
    unsafe { gfx_common_abi::get_graphic_scale() }
}

/// A screen-sized display drawable and its first frame, released on drop.
pub struct Display {
    drawable: *mut crate::graphics::native_abi::drawable_desc,
    frame: FRAME,
}

impl Display {
    pub fn create() -> Self {
        let (mut width, mut height) = (0, 0);
        // SAFETY: the isolated fixture owns the initialized graphics thread.
        let drawable = unsafe { drawable_abi::CreateDisplay(0, &mut width, &mut height) };
        assert!(!drawable.is_null());
        // SAFETY: the live display has one registered frame retained until drop.
        let frame = unsafe { crate::graphics::pixmap_abi::CaptureDrawable(drawable) };
        Self { drawable, frame }
    }

    pub fn frame(&self) -> FRAME {
        self.frame
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        // SAFETY: drawing has drained and this display uniquely owns its allocation.
        unsafe { drawable_abi::DestroyDrawable(self.drawable) };
    }
}

/// The frame at `index` within the same drawable as `frame`.
pub fn frame_at(frame: FRAME, index: u16) -> FRAME {
    // SAFETY: the fixture owns the drawable and `index` is one of its frames.
    unsafe { crate::graphics::pixmap_abi::SetAbsFrameIndex(frame, index) }
}

pub fn set_frame_hot_spot(frame: FRAME, hot: crate::graphics::native_abi::HOT_SPOT) {
    // SAFETY: the fixture retains the frame.
    unsafe { drawable_abi::SetFrameHot(frame, hot) };
}

/// Release the capture reference a drawable's first frame holds. Returns the
/// drawable's address, which identifies the allocation across conversations.
pub fn release_drawable(frame: FRAME) -> usize {
    // SAFETY: the fixture retains the drawable the frame belongs to.
    unsafe { crate::graphics::pixmap_abi::ReleaseDrawable(frame) as usize }
}

/// Software surfaces installed as the graphics format-conversion and screen
/// globals, with the previous globals restored on drop.
pub struct SdlSurfaces {
    previous: (
        *mut crate::graphics::ffi::SDL_Surface,
        *mut crate::graphics::ffi::SDL_Surface,
    ),
    _format: sdl2::surface::Surface<'static>,
    _screen: sdl2::surface::Surface<'static>,
}

impl SdlSurfaces {
    pub fn install(width: u32, height: u32) -> Result<Self, String> {
        use crate::graphics::sdl::lifecycle_abi;
        use sdl2::pixels::PixelFormatEnum;
        let format = sdl2::surface::Surface::new(width, height, PixelFormatEnum::RGBA32)?;
        let screen = sdl2::surface::Surface::new(width, height, PixelFormatEnum::RGB888)?;
        // SAFETY: the serialized fixture owns both surfaces and restores the borrowed globals before they drop.
        let previous = unsafe {
            let previous = (lifecycle_abi::format_conv_surf, lifecycle_abi::SDL_Screen);
            lifecycle_abi::format_conv_surf = format.raw().cast();
            lifecycle_abi::SDL_Screen = screen.raw().cast();
            previous
        };
        Ok(Self {
            previous,
            _format: format,
            _screen: screen,
        })
    }
}

impl Drop for SdlSurfaces {
    fn drop(&mut self) {
        use crate::graphics::sdl::lifecycle_abi;
        // SAFETY: the surfaces this guard installed are still alive until its fields drop.
        unsafe {
            lifecycle_abi::format_conv_surf = self.previous.0;
            lifecycle_abi::SDL_Screen = self.previous.1;
        }
    }
}

/// Install a packed color-map table (a two-byte range header, then 256 RGB entries per map).
pub fn set_color_map(table: &[u8]) {
    assert!(table.len() >= 2);
    let maps = usize::from(table[1] - table[0]) + 1;
    assert_eq!(table.len(), 2 + maps * 256 * 3);
    // SAFETY: the table is complete for the maps its header declares.
    unsafe { crate::graphics::cmap_abi::SetColorMap(table.as_ptr()) };
}
