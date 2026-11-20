//! The display port: compose and show one frame of the screen canvases.
//!
//! A display belongs to the game thread that draws (an SDL window cannot
//! move between threads), so it is separate from the process-wide platform
//! services.
use crate::graphics::{
    canvas::Canvas, geometry::Rect, presentation_diagnostics::PresentationSample,
    presented_frame::PresentedFrame,
};

/// One layer of a presented frame, drawn in order.
pub enum Layer<'a> {
    /// A screen canvas at `alpha`, whole or clipped to `rect`.
    Screen {
        screen: i32,
        canvas: &'a Canvas,
        alpha: u8,
        rect: Option<Rect>,
    },
    /// A fade colour over the whole frame.
    Color { rgba: [u8; 4] },
}

/// A frame to show: the swap state it was composed from and its layers.
pub struct PresentFrame<'a> {
    pub force_redraw: i32,
    pub transition: i32,
    pub fade: i32,
    pub layers: Vec<Layer<'a>>,
}

/// The display a game frame is presented on.
pub trait Display {
    /// Compose and show `frame`. Before showing it, `capture` receives the
    /// presentation count and the samples since the last present; when it
    /// returns a capture generation, the composed pixels are read back and
    /// returned with that generation.
    fn present(
        &mut self,
        frame: &PresentFrame<'_>,
        capture: &mut dyn FnMut(u64, &[PresentationSample]) -> Option<u64>,
    ) -> Option<PresentedFrame>;

    /// Record a diagnostic sample for the next present (window and display
    /// events, reconfiguration).
    fn record(&mut self, sample: PresentationSample);

    /// Show one decoded movie frame scaled to fit the window, outside the
    /// screen canvases (the 3DO movies at full window resolution). Pixels
    /// are RGBA words with red in the low byte.
    fn present_video(&mut self, pixels: &[u32], width: u32, height: u32) -> Result<(), String>;

    /// The window's SDL id and client bounds `(x, y, width, height)` for
    /// native-window proof; `show` raises the window first.
    #[cfg(feature = "debug-process")]
    fn native_window(&mut self, show: bool) -> (u32, (i32, i32, u32, u32));

    /// Apply a new video mode (fullscreen flag and window size).
    fn reconfigure(&mut self, flags: i32, width: i32, height: i32) -> Result<(), String>;
}
