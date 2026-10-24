use super::*;
use crate::resource::ffi_bridge::{InitResourceSystem, UninitResourceSystem};
use serial_test::serial;

#[test]
fn character_buffer_decodes_the_original_sdl2_sequences() {
    let mut buffer = CharacterBuffer::new();
    buffer.text_input("Aé水😀B\0".as_bytes());
    assert_eq!(buffer.last, u32::from('B'));
    for expected in [
        u32::from('A'),
        u32::from('é'),
        u32::from('水'),
        0xfffd,
        u32::from('B'),
        0,
    ] {
        assert_eq!(buffer.next(), expected);
    }
    buffer.text_input("😀é水Z\0".as_bytes());
    assert_eq!(buffer.next(), 0xfffd);
    assert_eq!(buffer.next(), u32::from('Z'));
    assert_eq!(buffer.next(), 0);
}

#[test]
fn character_buffer_keeps_255_chars_drops_overflow_and_wraps() {
    let mut buffer = CharacterBuffer::new();
    for _ in 0..255 {
        buffer.text_input(b"a\0");
    }
    buffer.text_input(b"b\0");
    assert_eq!(buffer.last, u32::from('a'));
    assert_eq!(buffer.next(), u32::from('a'));
    buffer.text_input(b"c\0");
    assert_eq!(buffer.last, u32::from('c'));
    for _ in 0..254 {
        assert_eq!(buffer.next(), u32::from('a'));
    }
    assert_eq!(buffer.next(), u32::from('c'));
    assert_eq!(buffer.next(), 0);
    buffer.clear();
    assert_eq!((buffer.head, buffer.tail, buffer.last), (0, 0, 0));
}

fn event(kind: u32) -> sys::SDL_Event {
    let mut event: sys::SDL_Event = unsafe { std::mem::zeroed() };
    event.type_ = kind;
    event
}

struct Driver;

impl Driver {
    fn new() -> Self {
        ffi::rust_v_control_uninit();
        ffi::rust_v_control_init();

        *INPUT.lock() = InputState::new();
        INPUT.lock().initialized = true;
        Self
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        TFB_UninitInput();
        *INPUT.lock() = InputState::new();
    }
}

#[test]
#[serial]
fn character_modes_defer_sdl_changes_to_events_and_clear_input() {
    let _driver = Driver::new();
    let mut target = 0;
    unsafe {
        ffi::rust_vcontrol_add_key_binding(32, &mut target);
        ffi::rust_vcontrol_process_key_down(32);
    }
    EnterCharacterMode();
    assert_eq!(target, 0);
    assert!(INPUT.lock().character_mode);
    assert!(!INPUT.lock().applied_character_mode);
    let mut text = event(0x303);
    for (dest, byte) in unsafe { &mut text.text.text }
        .iter_mut()
        .zip("é水\0".as_bytes())
    {
        *dest = *byte as c_char;
    }
    unsafe { process_input_event(&text) };
    assert!(INPUT.lock().applied_character_mode);
    assert_eq!(GetNextCharacter(), u32::from('é'));
    assert_eq!(GetNextCharacter(), u32::from('水'));
    assert_eq!(GetNextCharacter(), 0);
    assert_eq!(GetLastCharacter(), u32::from('水'));
    ExitCharacterMode();
    assert_eq!(GetLastCharacter(), 0);
    assert!(!INPUT.lock().character_mode);
    assert!(INPUT.lock().applied_character_mode);
    unsafe { process_input_event(&event(0x400)) };
    assert!(!INPUT.lock().applied_character_mode);
}

#[test]
#[serial]
fn numlock_keypad_characters_bypass_controls_only_in_character_mode() {
    let _driver = Driver::new();
    let key = super::super::keynames::sdl2_keys::SDLK_KP_1;
    let mut target = 0;
    unsafe { ffi::rust_vcontrol_add_key_binding(key, &mut target) };
    let mut key_down = event(0x300);
    key_down.key.keysym.sym = key;
    key_down.key.keysym.mod_ = sys::SDL_Keymod::KMOD_NUM as u16;
    EnterCharacterMode();
    unsafe { process_input_event(&key_down) };
    assert_eq!(target, 0);
    ExitCharacterMode();
    unsafe { process_input_event(&key_down) };
    assert_eq!(target & 0xff, 1);
    TFB_ResetControls();
    EnterCharacterMode();
    key_down.key.keysym.mod_ = 0;
    unsafe { process_input_event(&key_down) };
    assert_eq!(target & 0xff, 1);
    TFB_ResetControls();
    key_down.key.keysym.sym = super::super::keynames::sdl2_keys::SDLK_KP_ENTER;
    key_down.key.keysym.mod_ = sys::SDL_Keymod::KMOD_NUM as u16;
    assert!(!unsafe { is_numpad_character(&key_down, true) });
}

#[test]
#[serial]
fn input_gate_mouse_buttons_frame_start_and_reset_match_c() {
    let _driver = Driver::new();
    unsafe {
        ptr::write_volatile(ptr::addr_of_mut!(MouseButtonDown), 0);
        INPUT.lock().initialized = false;
        process_input_event(&event(0x401));
        assert_eq!(ptr::read_volatile(ptr::addr_of!(MouseButtonDown)), 0);
        INPUT.lock().initialized = true;
        process_input_event(&event(0x401));
        assert_eq!(ptr::read_volatile(ptr::addr_of!(MouseButtonDown)), 1);
        process_input_event(&event(0x402));
        assert_eq!(ptr::read_volatile(ptr::addr_of!(MouseButtonDown)), 0);
    }
    let mut target = 0;
    unsafe {
        ffi::rust_vcontrol_add_key_binding(32, &mut target);
        ffi::rust_vcontrol_process_key_down(32);
    }
    assert_eq!(target, 0x101);
    BeginInputFrame();
    assert_eq!(target, 1);
    INPUT.lock().characters.text_input(b"x\0");
    TFB_ResetControls();
    assert_eq!(target, 0);
    assert_eq!(GetNextCharacter(), 0);
    assert_eq!(GetLastCharacter(), 0);
}

unsafe fn put(key: &str, value: &str) {
    res_PutString(
        CString::new(key).unwrap().as_ptr(),
        CString::new(value).unwrap().as_ptr(),
    );
}

#[test]
#[serial]
fn resource_controls_register_menu_alternates_templates_and_removal() {
    let _driver = Driver::new();
    unsafe {
        UninitResourceSystem();
        InitResourceSystem();
        put("menu.pause.1", "key Space");
        put("menu.pause.2", "key Return");
        put("menu.pause.4", "key Escape");
        put(
            "keys.1.name",
            "a name longer than twenty nine bytes is truncated",
        );
        put("keys.1.up.1", "key Up");
        put("keys.1.up.2", "key w");
        let mut menu = [0; 24];
        let mut flight = [0; 14];
        tfb_set_input_vectors(menu.as_mut_ptr(), 24, flight.as_mut_ptr(), 2, 7);
        {
            let mut state = INPUT.lock();
            state.controls = (0..28).map(|_| empty_gesture()).collect();
            state.register_controls();
            state.register_flight();
            assert_eq!(state.controls[0].gesture_type, 1);
            assert_eq!(state.controls[1].gesture_type, 1);
            assert_eq!(state.controls[2].gesture_type, 0);
            assert_eq!(state.control_index(1, 6, 1), Some(27));
            for indices in [(-1, 0, 0), (2, 0, 0), (0, 7, 0), (0, 0, 2)] {
                assert!(state
                    .control_index(indices.0, indices.1, indices.2)
                    .is_none());
            }
        }
        let name = CStr::from_ptr(ptr::addr_of!(INPUT_TEMPLATES).cast::<c_char>());
        assert_eq!(name.to_bytes(), b"a name longer than twenty nin");
        assert_eq!(*ptr::addr_of!(INPUT_TEMPLATES).cast::<c_char>().add(30), 0);
        ffi::rust_vcontrol_process_key_down(32);
        assert_eq!(menu[0] & 0xff, 1);
        ffi::rust_vcontrol_process_key_down(27);
        assert_eq!(menu[0] & 0xff, 1, "alternate after a gap is not loaded");
        ffi::rust_vcontrol_process_key_down(super::super::keynames::sdl2_keys::SDLK_UP);
        assert_eq!(flight[0] & 0xff, 1);
        let mut buffer = [0 as c_char; 40];
        InterrogateInputState(0, 0, 1, buffer.as_mut_ptr(), 40);
        assert_eq!(CStr::from_ptr(buffer.as_ptr()), c"w");
        InterrogateInputState(2, 0, 0, buffer.as_mut_ptr(), 40);
        assert_eq!(buffer[0], 0);
        RemoveInputState(0, 0, 1);
        assert_eq!(res_HasKey(c"keys.1.up.2".as_ptr()), 0);
        ffi::rust_vcontrol_reset_input();
        ffi::rust_vcontrol_process_key_down(i32::from(b'w'));
        assert_eq!(flight[0], 0);
        TFB_UninitInput();
        UninitResourceSystem();
    }
}

#[test]
#[serial]
fn interrogation_formats_each_joystick_gesture_and_truncates() {
    let _driver = Driver::new();
    {
        let mut state = INPUT.lock();
        state.num_templates = 1;
        state.num_flight = 2;
        state.controls = [
            Gesture::JoyButton { port: 2, button: 3 },
            Gesture::JoyAxis {
                port: 1,
                axis: 2,
                polarity: -1,
            },
            Gesture::JoyHat {
                port: 0,
                hat: 1,
                dir: 8,
            },
        ]
        .iter()
        .map(VCONTROL_GESTURE::from_gesture)
        .collect();
        state.controls.push(empty_gesture());
    }
    for (control, alternate, expected) in [
        (0, 0, "[J2 B4]"),
        (0, 1, "[J1 A2 -]"),
        (1, 0, "[J0 H1 8]"),
        (1, 1, ""),
    ] {
        let mut buffer = [0 as c_char; 20];
        unsafe {
            InterrogateInputState(0, control, alternate, buffer.as_mut_ptr(), 20);
            assert_eq!(CStr::from_ptr(buffer.as_ptr()).to_str().unwrap(), expected);
        }
    }
    let mut buffer = [0x55 as c_char; 5];
    unsafe {
        InterrogateInputState(0, 0, 0, buffer.as_mut_ptr(), 4);
        assert_eq!(CStr::from_ptr(buffer.as_ptr()), c"[J2");
    }
    assert_eq!(buffer[4], 0x55);
}
