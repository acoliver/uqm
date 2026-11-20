//! Presenting the screens on the display, ported from sdl_common.c and
//! sdl2_common.c (GPL-2.0-or-later).
use super::canvas::Canvas;
use super::dcq::{DrawCommand, Screen};
use super::geometry::Rect;
use super::presentation_diagnostics::PresentationSample;
use super::presented_frame::PresentedFrame;
use super::state::{self, VideoMode, GFXFLAGS_FULLSCREEN};
use crate::host::{Display, Layer, PresentFrame};

/// Watches the frames the display shows (automation and replay capture).
pub trait PresentObserver {
    /// Whether to skip presenting (an observer that has finished).
    fn suppress_present(&mut self) -> bool;
    /// At the presentation boundary: the presentation count and the samples
    /// since the last one; a returned generation reads the frame back.
    fn capture(&mut self, count: u64, samples: &[PresentationSample]) -> Option<u64>;
    /// After the frame is shown, with its readback if one was captured.
    fn presented(&mut self, frame: Option<PresentedFrame>);
}

/// The display and the observer of what it shows.
pub(crate) struct Presentation {
    pub display: Box<dyn Display>,
    pub observer: Option<Box<dyn PresentObserver>>,
}

/// C: the `TFB_REDRAW_*` values passed to `TFB_SwapBuffers`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Redraw {
    No = 0,
    Fading = 1,
    Expose = 2,
    Yes = 3,
}

/// C: `TFB_SwapBuffers`: compose the main screen, a running transition, the
/// fade and the system box, and show them when anything changed.
pub fn swap_buffers(redraw: Redraw) {
    let mut observer = state::with(|g| {
        g.display
            .as_mut()
            .and_then(|presentation| presentation.observer.take())
    });
    if observer
        .as_mut()
        .is_some_and(|observer| observer.suppress_present())
    {
        restore_observer(observer);
        return;
    }
    let fade = state::fade_amount();
    let presented = state::with(|g| {
        let transition = g.transition_amount;
        let swap = &mut g.swap;
        let mut force = redraw as i32;
        if redraw == Redraw::No
            && !g.bbox.valid
            && fade == 255
            && transition == 255
            && swap.last_fade == 255
            && swap.last_transition == 255
        {
            return None;
        }
        if redraw == Redraw::No
            && (fade != 255
                || transition != 255
                || swap.last_fade != 255
                || swap.last_transition != 255)
        {
            force = Redraw::Fading as i32;
        }
        swap.last_fade = fade;
        swap.last_transition = transition;
        let system_box = swap.system_box;
        let Some(presentation) = g.display.as_mut() else {
            return Some(None);
        };
        let layers = layers(
            &g.screens,
            (transition, g.transition_clip),
            fade,
            system_box,
        );
        let frame = PresentFrame {
            force_redraw: force,
            transition,
            fade,
            layers,
        };
        Some(presentation.display.present(&frame, &mut |count, samples| {
            observer
                .as_mut()
                .and_then(|observer| observer.capture(count, samples))
        }))
    });
    if let (Some(frame), Some(observer)) = (presented, observer.as_mut()) {
        observer.presented(frame);
    }
    restore_observer(observer);
}

/// The layers one presentation composes: the main screen, the transition
/// screen while a transition runs, the fade colour, then the system box.
fn layers(
    screens: &[Canvas],
    (transition, transition_clip): (i32, Rect),
    fade: i32,
    system_box: Option<Rect>,
) -> Vec<Layer<'_>> {
    let main = &screens[Screen::Main.index()];
    let mut layers = vec![Layer::Screen {
        screen: Screen::Main as i32,
        canvas: main,
        alpha: 255,
        rect: None,
    }];
    if transition != 255 {
        layers.push(Layer::Screen {
            screen: Screen::Transition as i32,
            canvas: &screens[Screen::Transition.index()],
            alpha: (255 - transition) as u8,
            rect: Some(transition_clip),
        });
    }
    if fade != 255 {
        layers.push(Layer::Color {
            rgba: if fade < 255 {
                [0, 0, 0, (255 - fade) as u8]
            } else {
                [255, 255, 255, (fade - 255) as u8]
            },
        });
    }
    if let Some(rect) = system_box {
        layers.push(Layer::Screen {
            screen: Screen::Main as i32,
            canvas: main,
            alpha: 255,
            rect: Some(rect),
        });
    }
    layers
}

fn restore_observer(observer: Option<Box<dyn PresentObserver>>) {
    if observer.is_some() {
        state::with(|g| {
            if let Some(presentation) = g.display.as_mut() {
                presentation.observer = observer;
            }
        });
    }
}

/// Record a diagnostic sample with the display, if there is one.
pub(crate) fn record(sample: PresentationSample) {
    state::with(|g| {
        if let Some(presentation) = g.display.as_mut() {
            presentation.display.record(sample);
        }
    });
}

/// Whether a display is open.
pub fn has_display() -> bool {
    state::with(|g| g.display.is_some())
}

/// Show a movie frame on the display at window resolution; false (with
/// the reason logged) when there is no display or it failed.
pub fn present_video(pixels: &[u32], width: u32, height: u32) -> bool {
    state::with(|g| {
        let Some(presentation) = g.display.as_mut() else {
            crate::bridge_log::rust_bridge_log_msg(
                "RUST_VIDEO: rust_present_video_to_window no graphics state",
            );
            return false;
        };
        match presentation.display.present_video(pixels, width, height) {
            Ok(()) => true,
            Err(error) => {
                crate::bridge_log::rust_bridge_log_msg(&format!("RUST_VIDEO: {error}"));
                false
            }
        }
    })
}

/// The display window's SDL id and client bounds; `None` without a
/// display.
#[cfg(feature = "debug-process")]
pub fn native_window(show: bool) -> Option<(u32, (i32, i32, u32, u32))> {
    state::with(|g| {
        g.display
            .as_mut()
            .map(|presentation| presentation.display.native_window(show))
    })
}

/// C: `SetSystemRect` / `ClearSystemRect`: an area of the main screen drawn
/// over the fade.
pub fn set_system_rect(rect: Option<Rect>) {
    state::with(|g| g.swap.system_box = rect);
}

/// Options for opening the display.
#[derive(Default)]
pub struct GraphicsOptions {
    /// A borderless window at this desktop position (automation).
    pub native_position: Option<(i32, i32)>,
    pub observer: Option<Box<dyn PresentObserver>>,
}

/// C: `TFB_InitGraphics`: open the window and create the screens. A display
/// that cannot be opened ends the process, as in C.
pub fn init_graphics(driver: i32, flags: i32, options: GraphicsOptions, width: i32, height: i32) {
    let display = match super::sdl::SdlDisplay::open(flags, options.native_position, width, height)
    {
        Ok(display) => display,
        Err(error) => {
            log::error!("Graphics initialization failed: {error}");
            std::process::exit(1);
        }
    };
    state::init(
        VideoMode {
            driver,
            flags,
            width,
            height,
        },
        Some(Presentation {
            display: Box::new(display),
            observer: options.observer,
        }),
    );
}

/// C: `TFB_UninitGraphics`.
pub fn uninit_graphics() {
    drop(state::uninit());
}

/// C: `TFB_ReInitGraphics`: apply a new mode to the open display.
fn reinit_graphics(mode: VideoMode) -> Result<(), String> {
    state::with(|g| {
        g.video.flags = mode.flags;
        let presentation = g
            .display
            .as_mut()
            .unwrap_or_else(|| panic!("video reinitialized without a display"));
        presentation
            .display
            .reconfigure(mode.flags, mode.width, mode.height)?;
        g.video = VideoMode { driver: 1, ..mode };
        Ok(())
    })
}

/// The queued `REINITVIDEO` command: switch modes, falling back to the
/// previous one, then redraw.
pub(crate) fn reinit_video(mode: VideoMode) {
    let previous = state::video_mode();
    if reinit_graphics(mode).is_err() {
        log::error!("Could not provide requested mode: reverting to last known driver.");
        if reinit_graphics(previous).is_err() {
            log::error!("Video reinitialization failed");
            std::process::exit(1);
        }
    }
    swap_buffers(Redraw::Yes);
}

/// Queue a switch between fullscreen and windowed.
pub fn toggle_fullscreen() {
    state::with(|g| {
        let mode = VideoMode {
            flags: g.video.flags ^ GFXFLAGS_FULLSCREEN,
            ..g.video
        };
        g.enqueue(DrawCommand::ReinitVideo(mode));
    });
}

/// C: `TFB_SetGamma`: the renderer has no gamma control.
pub fn set_gamma(_gamma: f32) -> bool {
    false
}

/// C: `TFB_SupportsHardwareScaling`.
pub fn supports_hardware_scaling() -> bool {
    true
}

/// A frame through the display, with what composition it carried; used by
/// tests of the swap rules.
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Default)]
    struct Shown {
        frames: Vec<(i32, i32, i32, usize)>,
    }

    struct Recorder(Rc<RefCell<Shown>>);

    impl Display for Recorder {
        fn present(
            &mut self,
            frame: &PresentFrame<'_>,
            _: &mut dyn FnMut(u64, &[PresentationSample]) -> Option<u64>,
        ) -> Option<PresentedFrame> {
            self.0.borrow_mut().frames.push((
                frame.force_redraw,
                frame.transition,
                frame.fade,
                frame.layers.len(),
            ));
            None
        }

        fn record(&mut self, _: PresentationSample) {}

        fn present_video(&mut self, _: &[u32], _: u32, _: u32) -> Result<(), String> {
            Ok(())
        }

        #[cfg(feature = "debug-process")]
        fn native_window(&mut self, _: bool) -> (u32, (i32, i32, u32, u32)) {
            (0, (0, 0, 0, 0))
        }

        fn reconfigure(&mut self, _: i32, _: i32, _: i32) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn swaps_skip_unchanged_frames_and_compose_the_system_box() {
        let shown = Rc::new(RefCell::new(Shown::default()));
        state::init(
            VideoMode::default(),
            Some(Presentation {
                display: Box::new(Recorder(shown.clone())),
                observer: None,
            }),
        );
        swap_buffers(Redraw::No);
        assert!(shown.borrow().frames.is_empty());
        swap_buffers(Redraw::Expose);
        set_system_rect(Some(Rect::new(1, 2, 3, 4)));
        swap_buffers(Redraw::Yes);
        assert_eq!(shown.borrow().frames, [(2, 255, 255, 1), (3, 255, 255, 2)]);
        set_system_rect(None);
        drop(state::uninit());
    }
}
