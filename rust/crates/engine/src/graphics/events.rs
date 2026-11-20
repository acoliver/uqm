//! The event pump, ported from `TFB_ProcessEvents` in sdl_common.c
//! (GPL-2.0-or-later).
use super::present::{self, Redraw};
use super::presentation_diagnostics::PresentationSample;
use super::state;
use crate::host::HostEvent;

/// Dispatch pending platform events: input to the input port, window and
/// display events to the presentation diagnostics; a quit request is
/// remembered and an exposed window is redrawn.
pub fn process_events() {
    if crate::host::is_replay() {
        return;
    }
    for event in crate::host::platform().poll_input() {
        if let HostEvent::Window { detail, .. } = &event {
            present::record(PresentationSample::Display {
                event: detail.clone(),
            });
        }
        crate::input_port::process_input_event(&event);
        match event {
            HostEvent::Quit => state::with(|g| g.quit_posted = true),
            HostEvent::Window { exposed: true, .. } => present::swap_buffers(Redraw::Expose),
            _ => {}
        }
    }
}
