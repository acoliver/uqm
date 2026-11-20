//! SDL2 input driver and resource-backed control configuration.
//!
//! This is the SDL boundary of input: it polls SDL, feeds the events to the
//! registered `InputControls` and keeps the configured gesture of every
//! control so the setup menu can show, clear and rebind them.

use std::ffi::{CStr, CString};

use parking_lot::Mutex;
use sdl2::event::Event;
use sdl2::keyboard::Mod;
use sdl2::sys;

use super::controls::InputControls;
use super::frames::{Target, FLIGHT_KEYS, MENU_KEYS, TEMPLATES};
use super::gesture::Gesture;
use super::keynames::{key_name, sdl2_keys};
use super::templates::bounded_template_name;
use crate::io::Dir;

const ALTERNATES: usize = 2;
const MENU_NAMES: &[&str] = &[
    "pause",
    "exit",
    "abort",
    "debug",
    "fullscreen",
    "up",
    "down",
    "left",
    "right",
    "select",
    "cancel",
    "special",
    "pageup",
    "pagedown",
    "home",
    "end",
    "zoomin",
    "zoomout",
    "delete",
    "backspace",
    "editcancel",
    "search",
    "next",
];
const FLIGHT_NAMES: &[&str] = &["up", "down", "left", "right", "weapon", "special", "escape"];

/// The longest gesture text a resource value holds.
const GESTURE_TEXT_MAX: usize = 38;

pub fn input_templates() -> [CString; 6] {
    INPUT
        .lock()
        .names
        .clone()
        .map(|name| CString::new(name).unwrap_or_else(|_| unreachable!("template NUL")))
}

pub fn rename_template(index: usize, name: &CStr) -> CString {
    let name = bounded_template_name(name);
    INPUT.lock().names[index] = name.to_string_lossy().into_owned();
    name
}

struct CharacterBuffer {
    data: [u32; 256],
    head: usize,
    tail: usize,
    last: u32,
}

impl CharacterBuffer {
    const fn new() -> Self {
        Self {
            data: [0; 256],
            head: 0,
            tail: 0,
            last: 0,
        }
    }

    fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.last = 0;
    }

    fn next(&mut self) -> u32 {
        if self.head == self.tail {
            return 0;
        }
        let result = self.data[self.head];
        self.head = (self.head + 1) & 255;
        result
    }

    fn text_input(&mut self, text: &[u8]) {
        let mut i = 0;
        while i < text.len() && text[i] != 0 {
            let mut ch = u32::from(text[i]);
            i += 1;
            if (0xc0..0xe0).contains(&ch) {
                ch = ((ch & 0x1f) << 6) | u32::from(text[i] & 0x3f);
                i += 1;
            } else if (0xe0..0xf0).contains(&ch) {
                ch = ((ch & 0x0f) << 6) | u32::from(text[i] & 0x3f);
                i += 1;
                ch = (ch << 6) | u32::from(text[i] & 0x3f);
                i += 1;
            } else if ch >= 0xf0 {
                ch = 0xfffd;
                // The original driver skips every following high byte, including
                // any adjacent non-ASCII character, after an out-of-BMP sequence.
                while i < text.len() && text[i] > 0x7f {
                    i += 1;
                }
            }
            if ch == 0 {
                return;
            }
            let tail = (self.tail + 1) & 255;
            if tail != self.head {
                self.data[self.tail] = ch;
                self.tail = tail;
                self.last = ch;
            }
        }
    }
}

struct DriverState {
    /// The controls the SDL events drive; set between init and uninit.
    controls: Option<InputControls>,
    /// The gesture configured for each template, control and alternate.
    configured: Vec<Option<Gesture>>,
    names: [String; 6],
    mouse_button_down: bool,
    character_mode: bool,
    applied_character_mode: bool,
    characters: CharacterBuffer,
}

static INPUT: Mutex<DriverState> = Mutex::new(DriverState::new());

fn flight_target(template: usize, control: usize) -> Target {
    Target::Flight {
        template,
        key: control,
    }
}

fn resource_key(mut key: String) -> CString {
    key.truncate(GESTURE_TEXT_MAX);
    CString::new(key).unwrap_or_else(|_| panic!("control resource keys contain no NUL"))
}

fn flight_key(template: usize, name: &str, alternate: usize) -> CString {
    resource_key(format!("keys.{}.{name}.{}", template + 1, alternate + 1))
}

/// The gesture text of a resource string, if the resource exists.
fn resource_gesture(key: &CStr) -> Option<Option<Gesture>> {
    let spec = crate::resource::string(key.to_str().ok()?)?;
    Some(Gesture::parse(&spec))
}

impl DriverState {
    const fn new() -> Self {
        Self {
            controls: None,
            configured: Vec::new(),
            names: [const { String::new() }; 6],
            mouse_button_down: false,
            character_mode: false,
            applied_character_mode: false,
            characters: CharacterBuffer::new(),
        }
    }

    fn control_index(template: i32, control: i32, alternate: i32) -> Option<usize> {
        let in_range =
            |value: i32, limit: usize| usize::try_from(value).ok().filter(|v| *v < limit);
        Some(
            (in_range(template, TEMPLATES)? * FLIGHT_KEYS + in_range(control, FLIGHT_KEYS)?)
                * ALTERNATES
                + in_range(alternate, ALTERNATES)?,
        )
    }

    fn register_menu_controls(&self, controls: &InputControls) {
        for (index, name) in MENU_NAMES.iter().take(MENU_KEYS).enumerate() {
            for alternate in 1.. {
                let key = resource_key(format!("menu.{name}.{alternate}"));
                let Some(gesture) = resource_gesture(&key) else {
                    break;
                };
                if let Some(gesture) = gesture {
                    controls.vcontrol(|vc| vc.bind(gesture, Target::Menu(index)));
                }
            }
        }
    }

    fn register_flight_controls(&mut self, controls: &InputControls) {
        for template in 0..TEMPLATES {
            let key = resource_key(format!("keys.{}.name", template + 1));
            let name = resource_gesture_name(&key);
            self.names[template] = bounded_template_name(&name).to_string_lossy().into_owned();
            for (control, name) in FLIGHT_NAMES.iter().take(FLIGHT_KEYS).enumerate() {
                for alternate in 0..ALTERNATES {
                    let key = flight_key(template, name, alternate);
                    let index = (template * FLIGHT_KEYS + control) * ALTERNATES + alternate;
                    let Some(gesture) = resource_gesture(&key) else {
                        self.configured[index] = None;
                        continue;
                    };
                    self.configured[index] = gesture;
                    if let Some(gesture) = gesture {
                        controls.vcontrol(|vc| vc.bind(gesture, flight_target(template, control)));
                    }
                }
            }
        }
    }
}

/// The value of a string resource, empty when it is absent.
fn resource_gesture_name(key: &CStr) -> CString {
    key.to_str()
        .ok()
        .and_then(crate::resource::string)
        .map_or_else(CString::default, |name| {
            CString::new(name).unwrap_or_else(|_| panic!("resource string contains NUL"))
        })
}

/// Why the input driver could not start.
#[derive(Debug)]
pub struct InputInitError(pub String);

impl std::fmt::Display for InputInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for InputInitError {}

/// Start the driver: bring up SDL's joystick subsystem, load the configured
/// gestures and bind them to `controls`.
pub fn init_input(controls: &InputControls) -> Result<(), InputInitError> {
    init_joysticks()?;
    install_controls(controls);
    Ok(())
}

fn install_controls(controls: &InputControls) {
    let mut state = INPUT.lock();
    state.character_mode = false;
    controls.vcontrol(|vc| vc.init());
    state.configured = vec![None; TEMPLATES * FLIGHT_KEYS * ALTERNATES];
    let content = crate::io::content_dir();
    let config = crate::io::config_dir();
    load_index(content.as_ref(), c"menu.key", c"menu.");
    load_index(config.as_ref(), c"override.cfg", c"menu.");
    state.register_menu_controls(controls);
    load_index(config.as_ref(), c"flight.cfg", c"keys.");
    if !crate::resource::has_key("keys.1.name") {
        load_index(content.as_ref(), c"uqm.key", c"keys.");
    }
    state.register_flight_controls(controls);
    controls.vcontrol(|vc| vc.reset_states());
    state.controls = Some(controls.clone());
}

/// Send the SDL events and control resets to `controls` without loading the
/// configured bindings, for hosts that bind controls themselves.
pub fn attach_controls(controls: &InputControls) {
    INPUT.lock().controls = Some(controls.clone());
}

fn init_joysticks() -> Result<(), InputInitError> {
    // SAFETY: SDL's joystick calls are made on the thread that owns SDL, after
    // the video subsystem start-up; the names SDL returns are terminated strings.
    unsafe {
        if sys::SDL_InitSubSystem(sys::SDL_INIT_JOYSTICK) == -1 {
            return Err(InputInitError(format!(
                "Couldn't initialize joystick subsystem: {}",
                CStr::from_ptr(sys::SDL_GetError()).to_string_lossy()
            )));
        }
        let joysticks = sys::SDL_NumJoysticks();
        crate::log_abi::log_message(4, &format!("{joysticks} joysticks were found."));
        if joysticks > 0 {
            crate::log_abi::log_message(4, "The names of the joysticks are:");
            for index in 0..joysticks {
                crate::log_abi::log_message(
                    4,
                    &format!(
                        "    {}",
                        CStr::from_ptr(sys::SDL_JoystickNameForIndex(index)).to_string_lossy()
                    ),
                );
            }
            sys::SDL_JoystickEventState(sys::SDL_ENABLE as i32);
        }
    }
    Ok(())
}

/// Load a resource index from `dir`; an absent directory loads nothing.
fn load_index(dir: Option<&Dir>, file: &CStr, prefix: &CStr) {
    if let Some(dir) = dir {
        crate::resource::load_index(dir, file, Some(prefix));
    }
}

pub fn uninit_input() {
    let mut state = INPUT.lock();
    if let Some(controls) = state.controls.take() {
        controls.vcontrol(|vc| vc.uninit());
    }
    state.configured = Vec::new();
}

/// Forget the template names the configuration file defined, so the
/// resources loaded next provide them.
pub fn forget_template_names() {
    for template in 1..=TEMPLATES {
        crate::resource::remove(&format!("config.keys.{template}.name"));
    }
}

pub fn enter_character_mode() {
    let mut state = INPUT.lock();
    state.characters.clear();
    state.character_mode = true;
    reset_bound_controls(&state);
}

pub fn exit_character_mode() {
    let mut state = INPUT.lock();
    reset_bound_controls(&state);
    state.character_mode = false;
    state.characters.clear();
}

pub fn next_character() -> u32 {
    INPUT.lock().characters.next()
}

pub fn last_character() -> u32 {
    INPUT.lock().characters.last
}

pub fn mouse_button_down() -> bool {
    INPUT.lock().mouse_button_down
}

fn reset_bound_controls(state: &DriverState) {
    if let Some(controls) = &state.controls {
        controls.vcontrol(|vc| vc.reset_states());
    }
}

/// Release every control and drop the typed characters.
pub fn reset_controls() {
    let mut state = INPUT.lock();
    reset_bound_controls(&state);
    state.characters.clear();
}

/// Start a frame: the start bits of the last frame's presses are cleared.
pub fn begin_input_frame() {
    if let Some(controls) = &INPUT.lock().controls {
        controls.vcontrol(|vc| vc.begin_frame());
    }
}

/// Drain SDL's event queue.
pub fn poll_sdl_events() -> Vec<Event> {
    let mut events = Vec::new();
    let mut raw = std::mem::MaybeUninit::<sys::SDL_Event>::uninit();
    // SAFETY: SDL_PollEvent initializes the whole event whenever it returns 1.
    while unsafe { sys::SDL_PollEvent(raw.as_mut_ptr()) } > 0 {
        events.push(Event::from_ll(unsafe { raw.assume_init() }));
    }
    events
}

/// Keypad digits typed with NumLock on are characters, not controls, while
/// text is being entered.
fn is_numpad_character(event: &Event, character_mode: bool) -> bool {
    match event {
        Event::KeyDown {
            keycode: Some(key),
            keymod,
            ..
        }
        | Event::KeyUp {
            keycode: Some(key),
            keymod,
            ..
        } => {
            character_mode
                && keymod.contains(Mod::NUMMOD)
                && (sdl2_keys::SDLK_KP_1..=sdl2_keys::SDLK_KP_PERIOD).contains(&key.into_i32())
        }
        _ => false,
    }
}

fn sync_text_input(state: &mut DriverState) {
    if state.character_mode && !state.applied_character_mode {
        state.applied_character_mode = true;
        // SAFETY: SDL text input toggling takes no arguments.
        unsafe { sys::SDL_StartTextInput() };
    }
    if !state.character_mode && state.applied_character_mode {
        state.applied_character_mode = false;
        // SAFETY: as above.
        unsafe { sys::SDL_StopTextInput() };
    }
}

/// Deliver one SDL event to the registered controls.
pub fn process_input_event(event: &Event) {
    let mut state = INPUT.lock();
    let Some(controls) = state.controls.clone() else {
        return;
    };
    match event {
        Event::MouseButtonDown { .. } => state.mouse_button_down = true,
        Event::MouseButtonUp { .. } => state.mouse_button_down = false,
        _ => {}
    }
    sync_text_input(&mut state);
    if !is_numpad_character(event, state.character_mode) {
        controls.vcontrol(|vc| apply_event(vc, event));
    }
    if let Event::TextInput { text, .. } = event {
        state.characters.text_input(text.as_bytes());
    }
}

/// Apply one event to the bindings and remember the gestures worth rebinding.
fn apply_event(vc: &mut super::vcontrol::VControl, event: &Event) {
    match *event {
        Event::KeyDown {
            keycode: Some(key),
            repeat: false,
            ..
        } => {
            let sym = key.into_i32();
            vc.handle_key_down(sym);
            vc.set_last_gesture(Gesture::Key(sym));
        }
        Event::KeyUp {
            keycode: Some(key), ..
        } => vc.handle_key_up(key.into_i32()),
        Event::JoyAxisMotion {
            which,
            axis_idx,
            value,
            ..
        } => {
            vc.handle_joy_axis(which, i32::from(axis_idx), value);
            // Only a deliberate push is a gesture.
            if !(-15000..=15000).contains(&value) {
                vc.set_last_gesture(Gesture::JoyAxis {
                    port: which,
                    axis: i32::from(axis_idx),
                    polarity: if value < 0 { -1 } else { 1 },
                });
            }
        }
        Event::JoyHatMotion {
            which,
            hat_idx,
            state,
            ..
        } => {
            vc.handle_joy_hat(which, i32::from(hat_idx), state.to_raw());
            vc.set_last_gesture(Gesture::JoyHat {
                port: which,
                hat: i32::from(hat_idx),
                dir: state.to_raw(),
            });
        }
        Event::JoyButtonDown {
            which, button_idx, ..
        } => {
            vc.handle_joy_button(which, i32::from(button_idx), true);
            vc.set_last_gesture(Gesture::JoyButton {
                port: which,
                button: i32::from(button_idx),
            });
        }
        Event::JoyButtonUp {
            which, button_idx, ..
        } => vc.handle_joy_button(which, i32::from(button_idx), false),
        _ => {}
    }
}

/// The text the setup menu shows for a configured control.
pub fn control_name(template: i32, control: i32, alternate: i32) -> String {
    let state = INPUT.lock();
    let Some(index) = DriverState::control_index(template, control, alternate) else {
        crate::log_abi::log_message(3, "InterrogateInputState(): invalid control index");
        return String::new();
    };
    match state.configured.get(index).copied().flatten() {
        Some(Gesture::Key(key)) => key_name(key).to_owned(),
        Some(Gesture::JoyButton { port, button }) => format!("[J{} B{}]", port as i32, button + 1),
        Some(Gesture::JoyAxis {
            port,
            axis,
            polarity,
        }) => format!(
            "[J{} A{} {}]",
            port as i32,
            axis,
            if polarity > 0 { '+' } else { '-' }
        ),
        Some(Gesture::JoyHat { port, hat, dir }) => format!("[J{} H{} {}]", port as i32, hat, dir),
        None => String::new(),
    }
}

/// Unbind a flight control alternate and forget its configured resource.
pub fn remove_input_state(template: i32, control: i32, alternate: i32) {
    let mut state = INPUT.lock();
    let Some(index) = DriverState::control_index(template, control, alternate) else {
        crate::log_abi::log_message(3, "RemoveInputState(): invalid control index");
        return;
    };
    let (template, control, alternate) = (template as usize, control as usize, alternate as usize);
    if let (Some(controls), Some(gesture)) = (&state.controls, state.configured[index]) {
        controls.vcontrol(|vc| vc.unbind(gesture, flight_target(template, control)));
    }
    state.configured[index] = None;
    crate::resource::remove(
        &flight_key(template, FLIGHT_NAMES[control], alternate).to_string_lossy(),
    );
}

/// Wait for the player to make a gesture and bind it to a flight control
/// alternate. `poll` runs one frame of the caller's loop between checks.
pub fn rebind_input_state(template: i32, control: i32, alternate: i32, mut poll: impl FnMut()) {
    let Some(index) = DriverState::control_index(template, control, alternate) else {
        crate::log_abi::log_message(3, "RebindInputState(): invalid control index");
        return;
    };
    remove_input_state(template, control, alternate);
    let Some(controls) = INPUT.lock().controls.clone() else {
        return;
    };
    controls.vcontrol(|vc| vc.clear_gesture());
    let gesture = loop {
        if let Some(gesture) = controls.vcontrol(|vc| vc.last_gesture()) {
            break gesture;
        }
        poll();
    };
    let (template, control, alternate) = (template as usize, control as usize, alternate as usize);
    controls.vcontrol(|vc| vc.bind(gesture, flight_target(template, control)));
    INPUT.lock().configured[index] = Some(gesture);
    let key = flight_key(template, FLIGHT_NAMES[control], alternate);
    crate::resource::settings::Settings.put_string(&key.to_string_lossy(), &gesture.to_string());
}

#[cfg(test)]
#[path = "sdl_input_tests.rs"]
mod tests;
