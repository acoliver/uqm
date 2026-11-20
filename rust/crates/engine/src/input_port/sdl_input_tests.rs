use super::*;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
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

fn key_event(down: bool, sym: i32, keymod: Mod, repeat: bool) -> Event {
    let keycode = Keycode::from_i32(sym);
    if down {
        Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode,
            scancode: None,
            keymod,
            repeat,
        }
    } else {
        Event::KeyUp {
            timestamp: 0,
            window_id: 0,
            keycode,
            scancode: None,
            keymod,
            repeat,
        }
    }
}

fn mouse(down: bool) -> Event {
    let (timestamp, window_id, which, mouse_btn, clicks, x, y) =
        (0, 0, 0, MouseButton::Left, 1, 0, 0);
    if down {
        Event::MouseButtonDown {
            timestamp,
            window_id,
            which,
            mouse_btn,
            clicks,
            x,
            y,
        }
    } else {
        Event::MouseButtonUp {
            timestamp,
            window_id,
            which,
            mouse_btn,
            clicks,
            x,
            y,
        }
    }
}

fn text(text: &str) -> Event {
    Event::TextInput {
        timestamp: 0,
        window_id: 0,
        text: text.to_owned(),
    }
}

/// A driver with fresh state whose controls are bound as the resources say.
struct Driver {
    controls: InputControls,
}

impl Driver {
    fn new() -> Self {
        *INPUT.lock() = DriverState::new();
        let controls = InputControls::default();
        install_controls(&controls);
        Self { controls }
    }

    fn menu(&self, index: usize) -> i32 {
        self.controls.immediate().menu[index]
    }

    fn bind_menu(&self, sym: i32, index: usize) {
        self.controls
            .vcontrol(|vc| vc.add_key_binding(sym, Target::Menu(index)));
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        uninit_input();
        *INPUT.lock() = DriverState::new();
    }
}

#[test]
#[serial]
fn character_modes_defer_sdl_changes_to_events_and_clear_input() {
    let driver = Driver::new();
    driver.bind_menu(32, 0);
    process_input_event(&key_event(true, 32, Mod::empty(), false));
    assert_eq!(driver.menu(0), 0x101);
    enter_character_mode();
    assert_eq!(driver.menu(0), 0, "entering releases the controls");
    assert!(INPUT.lock().character_mode);
    assert!(!INPUT.lock().applied_character_mode);
    process_input_event(&text("é水"));
    assert!(INPUT.lock().applied_character_mode);
    assert_eq!(next_character(), u32::from('é'));
    assert_eq!(next_character(), u32::from('水'));
    assert_eq!(next_character(), 0);
    assert_eq!(last_character(), u32::from('水'));
    exit_character_mode();
    assert_eq!(last_character(), 0);
    assert!(!INPUT.lock().character_mode);
    assert!(INPUT.lock().applied_character_mode);
    process_input_event(&mouse(false));
    assert!(!INPUT.lock().applied_character_mode);
}

#[test]
#[serial]
fn numlock_keypad_characters_bypass_controls_only_in_character_mode() {
    let driver = Driver::new();
    let key = sdl2_keys::SDLK_KP_1;
    driver.bind_menu(key, 0);
    let down = key_event(true, key, Mod::NUMMOD, false);
    enter_character_mode();
    process_input_event(&down);
    assert_eq!(driver.menu(0), 0);
    exit_character_mode();
    process_input_event(&down);
    assert_eq!(driver.menu(0) & 0xff, 1);
    reset_controls();
    enter_character_mode();
    process_input_event(&key_event(true, key, Mod::empty(), false));
    assert_eq!(driver.menu(0) & 0xff, 1, "without NumLock it is a control");
    reset_controls();
    let enter = key_event(true, sdl2_keys::SDLK_KP_ENTER, Mod::NUMMOD, false);
    assert!(!is_numpad_character(&enter, true));
    assert!(is_numpad_character(&down, true));
    assert!(!is_numpad_character(&down, false));
}

#[test]
#[serial]
fn input_gate_mouse_buttons_frame_start_and_reset() {
    process_input_event(&mouse(true));
    assert!(!mouse_button_down(), "no controls, no input");
    let driver = Driver::new();
    process_input_event(&mouse(true));
    assert!(mouse_button_down());
    process_input_event(&mouse(false));
    assert!(!mouse_button_down());
    driver.bind_menu(32, 0);
    process_input_event(&key_event(true, 32, Mod::empty(), false));
    assert_eq!(driver.menu(0), 0x101);
    process_input_event(&key_event(true, 32, Mod::empty(), true));
    assert_eq!(driver.menu(0), 0x101, "a repeat is not another press");
    begin_input_frame();
    assert_eq!(driver.menu(0), 1);
    INPUT.lock().characters.text_input(b"x\0");
    reset_controls();
    assert_eq!(driver.menu(0), 0);
    assert_eq!(next_character(), 0);
    assert_eq!(last_character(), 0);
}

#[test]
#[serial]
fn joystick_events_drive_bindings_and_only_deliberate_moves_become_gestures() {
    let driver = Driver::new();
    driver.controls.vcontrol(|vc| {
        vc.init_joystick(0, "Pad".into(), 1, 2, 1).unwrap();
        vc.bind(Gesture::JoyButton { port: 0, button: 1 }, Target::Menu(3));
        vc.bind(
            Gesture::JoyAxis {
                port: 0,
                axis: 0,
                polarity: -1,
            },
            Target::Menu(4),
        );
    });
    let button = |down| {
        if down {
            Event::JoyButtonDown {
                timestamp: 0,
                which: 0,
                button_idx: 1,
            }
        } else {
            Event::JoyButtonUp {
                timestamp: 0,
                which: 0,
                button_idx: 1,
            }
        }
    };
    let axis = |value| Event::JoyAxisMotion {
        timestamp: 0,
        which: 0,
        axis_idx: 0,
        value,
    };
    process_input_event(&button(true));
    assert_eq!(driver.menu(3), 1);
    assert_eq!(
        driver.controls.vcontrol(|vc| vc.last_gesture()),
        Some(Gesture::JoyButton { port: 0, button: 1 })
    );
    process_input_event(&button(false));
    assert_eq!(driver.menu(3), 0);
    driver.controls.vcontrol(|vc| vc.clear_gesture());
    process_input_event(&axis(-12000));
    assert_eq!(driver.menu(4), 1, "past the dead zone it drives the word");
    assert_eq!(driver.controls.vcontrol(|vc| vc.last_gesture()), None);
    process_input_event(&axis(-20000));
    assert_eq!(
        driver.controls.vcontrol(|vc| vc.last_gesture()),
        Some(Gesture::JoyAxis {
            port: 0,
            axis: 0,
            polarity: -1
        })
    );
}

fn put(key: &str, value: &str) {
    crate::resource::settings::Settings.put_string(key, value);
}

#[test]
#[serial]
fn resource_controls_register_menu_alternates_templates_and_removal() {
    crate::resource::uninit();
    crate::resource::init();
    put("menu.pause.1", "key Space");
    put("menu.pause.2", "key Return");
    put("menu.pause.4", "key Escape");
    put(
        "keys.1.name",
        "a name longer than twenty nine bytes is truncated",
    );
    put("keys.1.up.1", "key Up");
    put("keys.1.up.2", "key w");
    let driver = Driver::new();
    assert_eq!(
        DriverState::control_index(1, 6, 1),
        Some(((FLIGHT_KEYS + 6) * ALTERNATES) + 1)
    );
    for (template, control, alternate) in [(-1, 0, 0), (6, 0, 0), (0, 7, 0), (0, 0, 2)] {
        assert_eq!(
            DriverState::control_index(template, control, alternate),
            None
        );
    }
    let names = input_templates();
    assert_eq!(names[0].to_bytes(), b"a name longer than twenty nin");
    assert!(names[1].is_empty());
    process_input_event(&key_event(true, 32, Mod::empty(), false));
    assert_eq!(driver.menu(0) & 0xff, 1);
    process_input_event(&key_event(true, 27, Mod::empty(), false));
    assert_eq!(
        driver.menu(0) & 0xff,
        1,
        "the alternate after a gap is not loaded"
    );
    process_input_event(&key_event(true, sdl2_keys::SDLK_UP, Mod::empty(), false));
    assert_eq!(driver.controls.immediate().key[0][0] & 0xff, 1);
    assert_eq!(control_name(0, 0, 1), "w");
    assert_eq!(control_name(5, 0, 0), "");
    remove_input_state(0, 0, 1);
    assert!(!crate::resource::has_key("keys.1.up.2"));
    assert_eq!(control_name(0, 0, 1), "");
    driver.controls.vcontrol(|vc| vc.reset_states());
    process_input_event(&key_event(true, i32::from(b'w'), Mod::empty(), false));
    assert_eq!(driver.controls.immediate().key[0][0], 0, "no longer bound");
    drop(driver);
    crate::resource::uninit();
}

#[test]
#[serial]
fn rebinding_waits_for_a_gesture_then_binds_and_saves_it() {
    crate::resource::uninit();
    crate::resource::init();
    put("keys.1.up.1", "key Up");
    let driver = Driver::new();
    let mut polls = 0;
    rebind_input_state(0, 0, 0, || {
        polls += 1;
        if polls == 3 {
            process_input_event(&key_event(true, 32, Mod::empty(), false));
        }
    });
    assert_eq!(polls, 3);
    assert_eq!(control_name(0, 0, 0), "Space");
    let key = flight_key(0, "up", 0);
    assert_eq!(
        crate::resource::string(key.to_str().unwrap()).as_deref(),
        Some("key Space")
    );
    process_input_event(&key_event(true, 32, Mod::empty(), false));
    assert_eq!(driver.controls.immediate().key[0][0] & 0xff, 1);
    process_input_event(&key_event(true, sdl2_keys::SDLK_UP, Mod::empty(), false));
    assert_eq!(
        driver.controls.immediate().key[0][0] & 0xff,
        1,
        "the old key no longer drives it"
    );
    drop(driver);
    crate::resource::uninit();
}

#[test]
#[serial]
fn interrogation_formats_each_gesture_kind() {
    let _driver = Driver::new();
    {
        let mut state = INPUT.lock();
        state.configured[0] = Some(Gesture::JoyButton { port: 2, button: 3 });
        state.configured[1] = Some(Gesture::JoyAxis {
            port: 1,
            axis: 2,
            polarity: -1,
        });
        state.configured[2] = Some(Gesture::JoyHat {
            port: 0,
            hat: 1,
            dir: 8,
        });
    }
    for (control, alternate, expected) in [
        (0, 0, "[J2 B4]"),
        (0, 1, "[J1 A2 -]"),
        (1, 0, "[J0 H1 8]"),
        (1, 1, ""),
    ] {
        assert_eq!(control_name(0, control, alternate), expected);
    }
}
