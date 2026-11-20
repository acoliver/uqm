//! The SDL event pump, ported from `TFB_ProcessEvents` in sdl_common.c
//! (GPL-2.0-or-later).
use super::super::present::{self, Redraw};
use super::super::state;
use sdl2::event::{Event, WindowEvent};

/// Dispatch pending SDL events: input to the input port, window and display
/// events to the presentation diagnostics; a quit request is remembered and
/// an exposed window is redrawn.
pub fn process_events() {
    if crate::host::is_replay() {
        return;
    }
    for event in crate::host::platform().poll_input() {
        if matches!(event, Event::Window { .. } | Event::Display { .. }) {
            if let Some(sample) = super::display_sample(&event) {
                present::record(sample);
            }
        }
        crate::input_port::process_input_event(&event);
        match event {
            Event::Quit { .. } => state::with(|g| g.quit_posted = true),
            Event::Window {
                win_event: WindowEvent::Exposed,
                ..
            } => present::swap_buffers(Redraw::Expose),
            _ => {}
        }
    }
}
