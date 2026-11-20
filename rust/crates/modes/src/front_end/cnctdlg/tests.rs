use uqm_engine::graphics::geometry::Point;
#[test]
#[serial_test::serial]
fn editor_keeps_initial_49_bytes_but_inserts_only_up_to_48_utf8_bytes() {
    use uqm_engine::{battle::Battle, observe::Unobserved, Game};
    let display = Display::new();
    for count in [46, 47, 49] {
        let original = "a".repeat(count);
        let expected = if count == 46 {
            format!("{original}é")
        } else {
            original.clone()
        };
        let mut session = NetplaySession::default();
        session.options_mut().peer[0].host = original;
        let mut state = ConnectDialogState::new(&mut session, 0, display.resources());
        let mut game = Game::new();
        let mut battle = Battle::new();
        let driver = std::rc::Rc::new(Driver {
            input: InputSession::default(),
            keys: std::cell::RefCell::new(
                [
                    KEY_MENU_DOWN as usize,
                    KEY_MENU_SELECT as usize,
                    usize::MAX,
                    KEY_MENU_SELECT as usize,
                    KEY_MENU_DOWN as usize,
                    KEY_MENU_SELECT as usize,
                ]
                .into_iter()
                .collect(),
            ),
            sleeps: std::cell::Cell::new(0),
        });
        let _characters =
            uqm_engine::test_support::native_session::NativeInput::bind(&driver.input);
        game.kernel.install_input_runtime(driver.clone());
        let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || 0);
        let mut observer = Unobserved;
        let mut io = Io {
            observer: &mut observer,
        };
        cx.run_input(&mut io, true, |cx, io| {
            do_melee_connect_dialog(&mut state, cx, io, &driver.input)
        });
        drop(state);
        assert_eq!(session.options.peer[0].host, expected);
    }
}

use super::*;

#[test]
fn connect_and_listen_update_only_the_selected_peer_and_listen_keeps_its_host() {
    for side in 0..2 {
        for server in [false, true] {
            let mut session = NetplaySession::default();
            let other = session.options.peer[1 - side].clone();
            let mut values = DialogValues::new(&session, side);
            values.text[0] = TextValue::new("remote.example");
            values.text[1] = TextValue::new("32100");
            values.delay = 9;
            apply_connection_options(&mut session, side, &values, server);
            let peer = &session.options.peer[side];
            assert_eq!(
                peer.host,
                if server {
                    "localhost"
                } else {
                    "remote.example"
                }
            );
            assert_eq!(peer.port, "32100");
            assert_eq!(peer.server, server);
            assert_eq!(session.options.input_delay, 9);
            assert_eq!(session.options.peer[1 - side].host, other.host);
            assert_eq!(session.options.peer[1 - side].port, other.port);
            assert_eq!(session.options.peer[1 - side].server, other.server);
        }
    }
}

#[test]
fn focus_wraps_in_widget_order_and_cancel_does_not_apply_edits() {
    let session = NetplaySession::default();
    let mut values = DialogValues::new(&session, 0);
    assert_eq!(values.focus, 0);
    assert_eq!(values.event(Event::Select), Action::Commit(true));
    values.event(Event::Up);
    assert_eq!(values.focus, 5);
    assert_eq!(values.event(Event::Select), Action::Cancel);
    values.event(Event::Down);
    for (focus, action) in [
        (0, Action::Commit(true)),
        (1, Action::Edit(0)),
        (2, Action::Commit(false)),
        (3, Action::None),
        (4, Action::Edit(1)),
        (5, Action::Cancel),
    ] {
        assert_eq!(values.focus, focus);
        assert_eq!(values.event(Event::Select), action);
        values.event(Event::Down);
    }
    values.text[0] = TextValue::new("not committed");
    assert_eq!(values.event(Event::Cancel), Action::Cancel);
    assert_eq!(values.focus, 5);
    assert_eq!(session.options.peer[0].host, "localhost");
    assert_eq!(session.options.input_delay, 2);
}

#[test]
fn text_bounds_keep_complete_utf8_and_edit_cancel_restores_the_value() {
    for text in ["a".repeat(100), "é".repeat(40), "abc🦀".repeat(20)] {
        let mut value = TextValue::new(&text);
        assert!(value.text().to_bytes().len() <= 49);
        assert!(text.starts_with(value.text().to_str().unwrap()));
        assert!(value.bytes.contains(&0));
        let saved = value.clone();
        value.finish_edit(c"changed", false);
        assert_eq!(value.text(), saved.text());
        value.finish_edit(c"hôte", true);
        assert_eq!(value.text(), c"hôte");
    }
    assert_eq!(TextValue::new(&"a".repeat(49)).text().to_bytes().len(), 49);
    assert_eq!(TextValue::new("a\0ignored").text(), c"a");
}

#[test]
fn delay_keys_saturate_and_ignore_keys_on_other_widgets() {
    let session = NetplaySession::default();
    let mut values = DialogValues::new(&session, 0);
    values.event(Event::Right);
    assert_eq!(values.delay, 2);
    values.focus = 3;
    for _ in 0..20 {
        values.event(Event::Left);
    }
    assert_eq!(values.delay, 0);
    for _ in 0..20 {
        values.event(Event::Right);
    }
    assert_eq!(values.delay, 9);
    assert_eq!(values.event(Event::Delete), Action::None);
}

#[test]
fn sequential_dialogs_start_with_their_own_values_and_focus() {
    let mut first = NetplaySession::default();
    first.options_mut().peer[0].host = "first".into();
    let mut values = DialogValues::new(&first, 0);
    values.focus = 3;
    values.event(Event::Left);
    apply_connection_options(&mut first, 0, &values, false);
    let second = NetplaySession::default();
    let second_values = DialogValues::new(&second, 1);
    assert_eq!(second_values.focus, 0);
    assert_eq!(second_values.delay, 2);
    assert_eq!(second_values.text[0].text(), c"localhost");
    let again = DialogValues::new(&first, 0);
    assert_eq!(again.focus, 0);
    assert_eq!(again.delay, 1);
    assert_eq!(again.text[0].text(), c"first");
}

#[test]
fn input_priority_keeps_the_first_pressed_menu_key() {
    let input = InputSession::default();
    let mut pulse = uqm_engine::input::frames::InputState::default();
    pulse.menu[KEY_MENU_UP as usize] = 1;
    pulse.menu[KEY_MENU_SELECT as usize] = 1;
    pulse.menu[KEY_MENU_CANCEL as usize] = 1;
    input.advance(pulse, pulse, 0);
    assert_eq!(Event::from_input(&input), Some(Event::Up));
}

struct Display {
    context: uqm_engine::graphics::context::LoadedContext,
    screen: uqm_engine::graphics::drawable::LoadedDrawable,
    player: Option<LoadedFont>,
    button: Option<LoadedFont>,
    _graphics: uqm_engine::test_support::native_session::Graphics,
}

impl Display {
    fn new() -> Self {
        let files = uqm_engine::test_support::native_session::FixtureFiles::new();
        files.write_font(32..=127);
        files.write("fixture.rmp", "fixture.font = FONTRES:font\n");
        let graphics = uqm_engine::test_support::native_session::Graphics::start();
        let (player, button) = {
            let mount = uqm_engine::test_support::native_session::ContentMount::new(files.path());
            mount.init_resources("fixture.rmp");
            (
                LoadedFont::load(c"fixture.font").unwrap(),
                LoadedFont::load(c"fixture.font").unwrap(),
            )
        };
        let context = uqm_engine::graphics::context::LoadedContext::create(c"connection fixture");
        let screen =
            uqm_engine::graphics::drawable::LoadedDrawable::create(8, 320, 240, 1).unwrap();
        context.lease().set_frame(screen.lease().context_frame(0));
        Self {
            context,
            screen,
            player: Some(player),
            button: Some(button),
            _graphics: graphics,
        }
    }

    fn resources(&self) -> DialogResources {
        DialogResources {
            context: self.context.lease(),
            player_font: self.player.as_ref().unwrap().lease(),
            button_font: self.button.as_ref().unwrap().lease(),
            labels: std::array::from_fn(|_| c"A".to_owned()),
            width: 320,
            height: 240,
        }
    }
}

#[test]
#[serial_test::serial]
fn renderer_draws_box_focus_slider_and_edit_cursor_pixels_after_font_release() {
    let mut display = Display::new();
    let resources = display.resources();
    let session = NetplaySession::default();
    let mut values = DialogValues::new(&session, 0);
    values.text[0] = TextValue::new("A");
    drop(display.player.take());
    drop(display.button.take());
    resources.draw(&values);
    resources.context.flush();
    let measured =
        resources
            .button_font
            .measure(c"A", Point { x: 160, y: 83 }, TextAlign::Center, 1);
    assert!(measured.visible);
    assert_eq!(
        (
            measured.rect.corner.x,
            measured.rect.corner.y,
            measured.rect.extent.width,
            measured.rect.extent.height
        ),
        (160, 82, 1, 2)
    );
    let frame = display.screen.lease().context_frame(0);
    let rgb = |x, y| {
        let c = frame.pixel(Point { x, y });
        (c.r, c.g, c.b)
    };
    assert_eq!(rgb(30, 65), (165, 165, 165));
    assert_eq!(rgb(28, 63), (66, 66, 66));
    assert_eq!(rgb(160, 82), (255, 255, 0));
    assert_eq!(rgb(136, 127), (49, 49, 49));
    assert_eq!(rgb(156, 124), (49, 49, 49));
    values.focus = 1;
    values.text[0].editing = true;
    values.text[0].cursor = 1;
    resources.draw(&values);
    assert_eq!(rgb(131, 100), (255, 255, 255));
    assert_eq!(rgb(129, 101), (255, 255, 0));
    drop(resources);
}

struct Driver {
    input: InputSession,
    keys: std::cell::RefCell<std::collections::VecDeque<usize>>,
    sleeps: std::cell::Cell<usize>,
}

impl uqm_engine::input::InputRuntime for Driver {
    fn input(&self) -> &InputSession {
        &self.input
    }
    fn observer(&self) -> Box<dyn uqm_engine::observe::Observer> {
        Box::new(uqm_engine::observe::Unobserved)
    }
    fn yield_poll(&self, _: &mut Cx<'_>) {
        panic!("unexpected yield");
    }
    fn sleep(&self, _: &mut Cx<'_>, _: u32) {
        panic!("unexpected sleep");
    }
    fn sleep_until(&self, _: &mut Cx<'_>, _: u32) {
        self.sleeps.set(self.sleeps.get() + 1);
    }
    fn flush(&self) {
        self.input.clear_menu_state();
    }
    fn confirm_exit(&self, _: &mut Cx<'_>) -> bool {
        panic!("unexpected confirmation");
    }
    fn update(&self, _: &mut Cx<'_>) {
        panic!("unexpected update");
    }
    fn battle_input(&self, _: &uqm_engine::battle::input::InputContext) -> u8 {
        panic!("unexpected battle input");
    }
    fn run(
        &self,
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        reset: bool,
        frame: &mut dyn uqm_engine::input::Frame,
    ) {
        assert!(reset);
        loop {
            let key = self
                .keys
                .borrow_mut()
                .pop_front()
                .expect("input script exhausted");
            self.input.clear_menu_state();
            let mut pulse = uqm_engine::input::frames::InputState::default();
            if key == usize::MAX {
                uqm_engine::test_support::native_session::text_input("é");
            } else {
                pulse.menu[key] = 1;
            }
            self.input.advance(pulse, pulse, 0);
            if !frame.frame(cx, io) {
                break;
            }
        }
    }
}

#[test]
#[serial_test::serial]
fn dialog_frames_commit_cancel_and_abort_with_independent_sessions() {
    use uqm_engine::{battle::Battle, observe::Unobserved, Game};
    let display = Display::new();
    for (side, keys, abort, confirmed, server, sleeps) in [
        (0, vec![KEY_MENU_SELECT], false, true, true, 1),
        (
            1,
            vec![KEY_MENU_DOWN, KEY_MENU_DOWN, KEY_MENU_SELECT],
            false,
            true,
            false,
            3,
        ),
        (0, vec![KEY_MENU_CANCEL], false, false, true, 1),
        (1, vec![KEY_MENU_DOWN], true, true, true, 1),
    ] {
        let mut session = NetplaySession::default();
        let mut state = ConnectDialogState::new(&mut session, side, display.resources());
        let mut game = Game::new();
        let mut battle = Battle::new();
        let driver = std::rc::Rc::new(Driver {
            input: InputSession::default(),
            keys: std::cell::RefCell::new(keys.into_iter().map(|key| key as usize).collect()),
            sleeps: std::cell::Cell::new(0),
        });
        game.kernel.install_input_runtime(driver.clone());
        if abort {
            game.world.set_activity(0x4000);
        }
        let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || 0);
        let mut observer = Unobserved;
        let mut io = Io {
            observer: &mut observer,
        };
        cx.run_input(&mut io, true, |cx, io| {
            do_melee_connect_dialog(&mut state, cx, io, &driver.input)
        });
        assert_eq!(state.confirmed, confirmed);
        assert_eq!(driver.sleeps.get(), sleeps);
        assert_eq!(driver.input.repeat_config(), (70, 420, 70, false));
        drop(state);
        assert_eq!(session.options.peer[side].server, server);
    }
}

#[test]
#[serial_test::serial]
fn safe_editor_applies_utf8_deletions_and_rolls_back_cancelled_edits() {
    use uqm_engine::input_port::frames::{KEY_MENU_EDIT_CANCEL, KEY_MENU_HOME};
    use uqm_engine::{battle::Battle, observe::Unobserved, Game};
    let display = Display::new();
    for (end, expected) in [(KEY_MENU_SELECT, "é"), (KEY_MENU_EDIT_CANCEL, "é漢")] {
        let mut session = NetplaySession::default();
        session.options_mut().peer[1].host = "é漢".into();
        let mut state = ConnectDialogState::new(&mut session, 1, display.resources());
        let mut game = Game::new();
        let mut battle = Battle::new();
        let driver = std::rc::Rc::new(Driver {
            input: InputSession::default(),
            keys: std::cell::RefCell::new(
                [
                    KEY_MENU_DOWN,
                    KEY_MENU_SELECT,
                    KEY_MENU_HOME,
                    KEY_MENU_RIGHT,
                    KEY_MENU_DELETE,
                    end,
                    KEY_MENU_DOWN,
                    KEY_MENU_SELECT,
                ]
                .into_iter()
                .map(|key| key as usize)
                .collect(),
            ),
            sleeps: std::cell::Cell::new(0),
        });
        game.kernel.install_input_runtime(driver.clone());
        let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || 0);
        let mut observer = Unobserved;
        let mut io = Io {
            observer: &mut observer,
        };
        cx.run_input(&mut io, true, |cx, io| {
            do_melee_connect_dialog(&mut state, cx, io, &driver.input)
        });
        assert!(state.confirmed);
        assert!(!state.values.text[0].editing);
        assert_eq!(state.values.text[0].text().to_str().unwrap(), expected);
        drop(state);
        assert_eq!(session.options.peer[1].host, expected);
        assert!(!session.options.peer[1].server);
        assert_eq!(session.options.peer[0].host, "localhost");
    }
}
