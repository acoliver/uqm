//! SDL2 input driver and resource-backed control configuration.

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr;

use parking_lot::Mutex;
use sdl2::sys;

use super::c_api::*;
use super::ffi::{self, GestureUnion, VCONTROL_GESTURE, VCONTROL_NONE};
use super::keynames::key_name;
use super::vcontrol::Gesture;
#[cfg(not(test))]
use crate::mainloop::dir_prep::{configDir as CONFIG_DIR, contentDir as CONTENT_DIR};
#[cfg(test)]
static mut CONFIG_DIR: *mut c_void = ptr::null_mut();
#[cfg(test)]
static mut CONTENT_DIR: *mut c_void = ptr::null_mut();
use crate::resource::ffi_bridge::{
    res_GetString, res_HasKey, res_IsString, res_PutString, res_Remove, LoadResourceIndex,
    SaveResourceIndex,
};

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

#[cfg(not(test))]
extern "C" {
    #[link_name = "input_templates"]
    static mut INPUT_TEMPLATES: [[c_char; 30]; 6];
    fn TaskSwitch();
}

#[cfg(test)]
static mut INPUT_TEMPLATES: [[c_char; 30]; 6] = [[0; 30]; 6];

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

struct InputState {
    menu: usize,
    num_menu: usize,
    flight: usize,
    num_templates: usize,
    num_flight: usize,
    controls: Vec<VCONTROL_GESTURE>,
    initialized: bool,
    character_mode: bool,
    applied_character_mode: bool,
    characters: CharacterBuffer,
}

impl InputState {
    const fn new() -> Self {
        Self {
            menu: 0,
            num_menu: 0,
            flight: 0,
            num_templates: 0,
            num_flight: 0,
            controls: Vec::new(),
            initialized: false,
            character_mode: false,
            applied_character_mode: false,
            characters: CharacterBuffer::new(),
        }
    }

    fn control_index(&self, template: c_int, control: c_int, alternate: c_int) -> Option<usize> {
        if template < 0
            || control < 0
            || alternate < 0
            || template as usize >= self.num_templates
            || control as usize >= self.num_flight
            || alternate as usize >= ALTERNATES
        {
            return None;
        }
        Some(
            (template as usize * self.num_flight + control as usize) * ALTERNATES
                + alternate as usize,
        )
    }

    fn target(&self, template: usize, control: usize) -> *mut c_int {
        (self.flight as *mut c_int).wrapping_add(template * self.num_flight + control)
    }

    unsafe fn register_controls(&mut self) {
        for (index, name) in MENU_NAMES.iter().take(self.num_menu).enumerate() {
            let mut alternate = 1;
            loop {
                let key = resource_key(format!("menu.{name}.{alternate}"));
                if res_IsString(key.as_ptr()) == 0 {
                    break;
                }
                let mut gesture = empty_gesture();
                vcontrol_parse_gesture(&mut gesture, res_GetString(key.as_ptr()));
                vcontrol_add_gesture_binding(&mut gesture, (self.menu as *mut c_int).add(index));
                alternate += 1;
            }
        }
    }

    unsafe fn register_flight(&mut self) {
        for template in 0..self.num_templates {
            let key = resource_key(format!("keys.{}.name", template + 1));
            let dest = ptr::addr_of_mut!(INPUT_TEMPLATES)
                .cast::<c_char>()
                .add(template * 30);
            if res_IsString(key.as_ptr()) != 0 {
                libc::strncpy(dest, res_GetString(key.as_ptr()), 29);
                *dest.add(29) = 0;
            } else {
                *dest = 0;
            }
            for (control, name) in FLIGHT_NAMES.iter().take(self.num_flight).enumerate() {
                for alternate in 0..ALTERNATES {
                    let key = flight_key(template, name, alternate);
                    let index = (template * self.num_flight + control) * ALTERNATES + alternate;
                    if res_IsString(key.as_ptr()) == 0 {
                        self.controls[index].gesture_type = VCONTROL_NONE;
                        continue;
                    }
                    vcontrol_parse_gesture(&mut self.controls[index], res_GetString(key.as_ptr()));
                    let target = self.target(template, control);
                    vcontrol_add_gesture_binding(&mut self.controls[index], target);
                }
            }
        }
    }
}

static INPUT: Mutex<InputState> = Mutex::new(InputState::new());

fn empty_gesture() -> VCONTROL_GESTURE {
    VCONTROL_GESTURE {
        gesture_type: VCONTROL_NONE,
        gesture: GestureUnion { data: [0; 3] },
    }
}

fn resource_key(mut key: String) -> CString {
    key.truncate(38);
    CString::new(key).expect("control resource keys contain no NUL")
}

fn flight_key(template: usize, name: &str, alternate: usize) -> CString {
    resource_key(format!("keys.{}.{name}.{}", template + 1, alternate + 1))
}

fn fatal(message: &str) -> ! {
    crate::log_abi::log_message(0, message);
    std::process::exit(1)
}

/// # Safety
/// The vectors must hold the stated element counts and outlive all bindings.
pub unsafe extern "C" fn tfb_set_input_vectors(
    menu: *mut c_int,
    num_menu: c_int,
    flight: *mut c_int,
    num_templates: c_int,
    num_flight: c_int,
) {
    if num_menu < 0 || num_templates < 0 || num_flight < 0 {
        fatal("TFB_SetInputVectors(): invalid vector size");
    }
    let mut state = INPUT.lock();
    state.menu = menu as usize;
    state.num_menu = num_menu as usize;
    state.flight = flight as usize;
    state.num_templates = num_templates as usize;
    state.num_flight = num_flight as usize;
}

/// # Safety
/// Input vectors and the resource directories must have been initialized.
#[no_mangle]
pub unsafe extern "C" fn TFB_InitInput(_driver: c_int, _flags: c_int) -> c_int {
    if sys::SDL_InitSubSystem(sys::SDL_INIT_JOYSTICK) == -1 {
        fatal(&format!(
            "Couldn't initialize joystick subsystem: {}",
            CStr::from_ptr(sys::SDL_GetError()).to_string_lossy()
        ));
    }
    crate::log_abi::log_message(
        4,
        &format!("{} joysticks were found.", sys::SDL_NumJoysticks()),
    );
    let joysticks = sys::SDL_NumJoysticks();
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
        sys::SDL_JoystickEventState(sys::SDL_ENABLE as c_int);
    }
    let mut state = INPUT.lock();
    state.character_mode = false;
    ffi::rust_v_control_init();
    if state.menu == 0 || state.flight == 0 {
        fatal("initKeyConfig(): invalid input vectors");
    }
    let count = state.num_templates * state.num_flight * ALTERNATES;
    state.controls = (0..count).map(|_| empty_gesture()).collect();
    LoadResourceIndex(CONTENT_DIR, c"menu.key".as_ptr(), c"menu.".as_ptr());
    LoadResourceIndex(CONFIG_DIR, c"override.cfg".as_ptr(), c"menu.".as_ptr());
    state.register_controls();
    LoadResourceIndex(CONFIG_DIR, c"flight.cfg".as_ptr(), c"keys.".as_ptr());
    if res_HasKey(c"keys.1.name".as_ptr()) == 0 {
        LoadResourceIndex(CONTENT_DIR, c"uqm.key".as_ptr(), c"keys.".as_ptr());
    }
    state.register_flight();
    ffi::rust_vcontrol_reset_input();
    state.initialized = true;
    0
}

#[no_mangle]
pub extern "C" fn TFB_UninitInput() {
    let mut state = INPUT.lock();
    unsafe { ffi::rust_v_control_uninit() };
    state.controls = Vec::new();
}

#[no_mangle]
pub extern "C" fn EnterCharacterMode() {
    let mut state = INPUT.lock();
    state.characters.clear();
    state.character_mode = true;
    unsafe { ffi::rust_vcontrol_reset_input() };
}

#[no_mangle]
pub extern "C" fn ExitCharacterMode() {
    let mut state = INPUT.lock();
    unsafe { ffi::rust_vcontrol_reset_input() };
    state.character_mode = false;
    state.characters.clear();
}

#[no_mangle]
pub extern "C" fn GetNextCharacter() -> u32 {
    INPUT.lock().characters.next()
}

#[no_mangle]
pub extern "C" fn GetLastCharacter() -> u32 {
    INPUT.lock().characters.last
}

#[no_mangle]
pub static mut MouseButtonDown: c_int = 0;

unsafe fn is_numpad_character(event: &sys::SDL_Event, character_mode: bool) -> bool {
    character_mode
        && matches!(event.type_, 0x300 | 0x301)
        && event.key.keysym.mod_ & sys::SDL_Keymod::KMOD_NUM as u16 != 0
        && (super::keynames::sdl2_keys::SDLK_KP_1..=super::keynames::sdl2_keys::SDLK_KP_PERIOD)
            .contains(&event.key.keysym.sym)
}

/// # Safety
/// `event` must point to a valid SDL2 event, with valid SDL text input bytes.
pub unsafe extern "C" fn process_input_event(event: *const sys::SDL_Event) {
    let mut state = INPUT.lock();
    if !state.initialized {
        return;
    }
    let event = &*event;
    match event.type_ {
        0x401 => ptr::write_volatile(ptr::addr_of_mut!(MouseButtonDown), 1),
        0x402 => ptr::write_volatile(ptr::addr_of_mut!(MouseButtonDown), 0),
        _ => {}
    }
    if state.character_mode && !state.applied_character_mode {
        state.applied_character_mode = true;
        sys::SDL_StartTextInput();
    }
    if !state.character_mode && state.applied_character_mode {
        state.applied_character_mode = false;
        sys::SDL_StopTextInput();
    }
    if !is_numpad_character(event, state.character_mode) {
        vcontrol_handle_event((event as *const sys::SDL_Event).cast());
    }
    if event.type_ == 0x303 {
        let text = &event.text.text;
        let bytes = std::slice::from_raw_parts(text.as_ptr().cast::<u8>(), text.len());
        state.characters.text_input(bytes);
    }
}

#[no_mangle]
pub extern "C" fn TFB_ResetControls() {
    let mut state = INPUT.lock();
    unsafe { ffi::rust_vcontrol_reset_input() };
    state.characters.clear();
}

unsafe fn write_buffer(buffer: *mut c_char, maxlen: c_int, text: &str) {
    if maxlen > 0 {
        let len = text.len().min(maxlen as usize - 1);
        ptr::copy_nonoverlapping(text.as_ptr(), buffer.cast(), len);
        *buffer.add(len) = 0;
    }
}

/// # Safety
/// `buffer` must be writable for `maxlen` bytes (at least one for invalid indices).
#[no_mangle]
pub unsafe extern "C" fn InterrogateInputState(
    template: c_int,
    control: c_int,
    alternate: c_int,
    buffer: *mut c_char,
    maxlen: c_int,
) {
    let state = INPUT.lock();
    let Some(index) = state.control_index(template, control, alternate) else {
        crate::log_abi::log_message(3, "InterrogateInputState(): invalid control index");
        *buffer = 0;
        return;
    };
    let text = match state.controls[index].to_gesture() {
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
        None => {
            *buffer = 0;
            return;
        }
    };
    write_buffer(buffer, maxlen, &text);
}

#[no_mangle]
pub extern "C" fn RemoveInputState(template: c_int, control: c_int, alternate: c_int) {
    let mut state = INPUT.lock();
    let Some(index) = state.control_index(template, control, alternate) else {
        crate::log_abi::log_message(3, "RemoveInputState(): invalid control index");
        return;
    };
    let target = state.target(template as usize, control as usize);
    unsafe {
        vcontrol_remove_gesture_binding(&mut state.controls[index], target);
        state.controls[index].gesture_type = VCONTROL_NONE;
        let key = flight_key(
            template as usize,
            FLIGHT_NAMES[control as usize],
            alternate as usize,
        );
        res_Remove(key.as_ptr());
    }
}

#[no_mangle]
pub extern "C" fn RebindInputState(template: c_int, control: c_int, alternate: c_int) {
    let Some(index) = INPUT.lock().control_index(template, control, alternate) else {
        crate::log_abi::log_message(3, "RebindInputState(): invalid control index");
        return;
    };
    RemoveInputState(template, control, alternate);
    unsafe {
        ffi::rust_vcontrol_clear_gesture();
        let mut gesture = empty_gesture();
        while vcontrol_get_last_gesture(&mut gesture) == 0 {
            // TaskSwitch pumps SDL events in the single-threaded game runtime.
            #[cfg(not(test))]
            TaskSwitch();
            #[cfg(test)]
            crate::threading::task_switch();
        }
        let mut state = INPUT.lock();
        vcontrol_add_gesture_binding(
            &mut gesture,
            state.target(template as usize, control as usize),
        );
        state.controls[index] = gesture;
        let key = flight_key(
            template as usize,
            FLIGHT_NAMES[control as usize],
            alternate as usize,
        );
        let mut value = [0 as c_char; 40];
        vcontrol_dump_gesture(value.as_mut_ptr(), 39, &mut state.controls[index]);
        res_PutString(key.as_ptr(), value.as_ptr());
    }
}

/// # Safety
/// `path` must be a live UIO directory and `filename` a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn SaveKeyConfiguration(path: *mut c_void, filename: *const c_char) {
    SaveResourceIndex(path, filename, c"keys.".as_ptr(), 1);
}

#[no_mangle]
pub extern "C" fn BeginInputFrame() {
    unsafe { ffi::rust_vcontrol_begin_frame() };
}

#[cfg(test)]
#[path = "sdl_input_tests.rs"]
mod tests;
