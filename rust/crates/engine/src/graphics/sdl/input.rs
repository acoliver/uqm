//! SDL's event queue, text-input toggle and joysticks, in the engine's terms.
use std::ffi::CStr;

use crate::host::HostEvent;
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Mod;
use sdl2::sys;

/// Drain SDL's event queue.
pub fn poll() -> Vec<HostEvent> {
    let mut events = Vec::new();
    let mut raw = std::mem::MaybeUninit::<sys::SDL_Event>::uninit();
    // SAFETY: SDL_PollEvent initializes the whole event whenever it returns 1.
    while unsafe { sys::SDL_PollEvent(raw.as_mut_ptr()) } > 0 {
        // SAFETY: as above.
        if let Some(event) = translate(&Event::from_ll(unsafe { raw.assume_init() })) {
            events.push(event);
        }
    }
    events
}

/// The engine's view of one SDL event; `None` for events it ignores.
pub fn translate(event: &Event) -> Option<HostEvent> {
    Some(match event {
        Event::KeyDown {
            keycode: Some(key),
            keymod,
            repeat,
            ..
        } => HostEvent::KeyDown {
            key: key.into_i32(),
            numlock: keymod.contains(Mod::NUMMOD),
            repeat: *repeat,
        },
        Event::KeyUp {
            keycode: Some(key),
            keymod,
            ..
        } => HostEvent::KeyUp {
            key: key.into_i32(),
            numlock: keymod.contains(Mod::NUMMOD),
        },
        Event::TextInput { text, .. } => HostEvent::TextInput(text.clone()),
        Event::MouseButtonDown { .. } => HostEvent::MouseButton { down: true },
        Event::MouseButtonUp { .. } => HostEvent::MouseButton { down: false },
        Event::JoyAxisMotion {
            which,
            axis_idx,
            value,
            ..
        } => HostEvent::JoyAxis {
            port: *which,
            axis: i32::from(*axis_idx),
            value: *value,
        },
        Event::JoyHatMotion {
            which,
            hat_idx,
            state,
            ..
        } => HostEvent::JoyHat {
            port: *which,
            hat: i32::from(*hat_idx),
            state: state.to_raw(),
        },
        Event::JoyButtonDown {
            which, button_idx, ..
        } => HostEvent::JoyButton {
            port: *which,
            button: i32::from(*button_idx),
            down: true,
        },
        Event::JoyButtonUp {
            which, button_idx, ..
        } => HostEvent::JoyButton {
            port: *which,
            button: i32::from(*button_idx),
            down: false,
        },
        Event::Quit { .. } => HostEvent::Quit,
        Event::Window { win_event, .. } => HostEvent::Window {
            detail: format!("{event:?}"),
            exposed: matches!(win_event, WindowEvent::Exposed),
        },
        Event::Display { .. } => HostEvent::Window {
            detail: format!("{event:?}"),
            exposed: false,
        },
        _ => return None,
    })
}

pub fn set_text_input(on: bool) {
    // SAFETY: SDL text input toggling takes no arguments.
    unsafe {
        if on {
            sys::SDL_StartTextInput();
        } else {
            sys::SDL_StopTextInput();
        }
    }
}

/// Start SDL's joystick subsystem and report the joysticks' names.
pub fn init_joysticks() -> Result<Vec<String>, String> {
    // SAFETY: SDL's joystick calls are made on the thread that owns SDL, after
    // the video subsystem start-up; the names SDL returns are terminated strings.
    unsafe {
        if sys::SDL_InitSubSystem(sys::SDL_INIT_JOYSTICK) == -1 {
            return Err(CStr::from_ptr(sys::SDL_GetError())
                .to_string_lossy()
                .into_owned());
        }
        let names = (0..sys::SDL_NumJoysticks())
            .map(|index| {
                CStr::from_ptr(sys::SDL_JoystickNameForIndex(index))
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>();
        if !names.is_empty() {
            sys::SDL_JoystickEventState(sys::SDL_ENABLE as i32);
        }
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdl2::keyboard::Keycode;

    #[test]
    fn keys_carry_numlock_and_window_events_carry_their_description() {
        let down = Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: Keycode::from_i32(32),
            scancode: None,
            keymod: Mod::NUMMOD,
            repeat: true,
        };
        assert_eq!(
            translate(&down),
            Some(HostEvent::KeyDown {
                key: 32,
                numlock: true,
                repeat: true
            })
        );
        let resized = Event::Window {
            timestamp: 7,
            window_id: 1,
            win_event: WindowEvent::Resized(1280, 960),
        };
        let Some(HostEvent::Window { detail, exposed }) = translate(&resized) else {
            panic!("window event");
        };
        assert!(detail.contains("Resized(1280, 960)") && detail.contains("timestamp: 7"));
        assert!(!exposed);
        assert_eq!(
            translate(&Event::Quit { timestamp: 8 }),
            Some(HostEvent::Quit)
        );
        assert_eq!(translate(&Event::AppTerminating { timestamp: 9 }), None);
    }
}
