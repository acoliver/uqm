#![cfg(test)]
use super::*;
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
};
use uqm_engine::graphics::{
    context::LoadedContext, drawable::LoadedDrawable, font::LoadedFont, geometry::Point,
};

struct Fixture {
    context: Option<LoadedContext>,
    screen: Option<LoadedDrawable>,
    font: Option<LoadedFont>,
    input: InputSession,
    _native_input: uqm_engine::test_support::native_session::NativeInput,
    _mount: uqm_engine::test_support::native_session::ContentMount,
    _graphics: uqm_engine::test_support::native_session::Graphics,
    files: uqm_engine::test_support::native_session::FixtureFiles,
}
impl Fixture {
    fn new() -> Self {
        let files = uqm_engine::test_support::native_session::FixtureFiles::new();
        files.write_font(32..=127);
        files.write(
            "fixture.rmp",
            "fixture.font = FONTRES:font\nkeys.1.name = STRING:one\nkeys.1.up.1 = STRING:key Up\n",
        );
        let input = InputSession::default();
        let graphics = uqm_engine::test_support::native_session::Graphics::start();
        let mount = uqm_engine::test_support::native_session::ContentMount::new(files.path())
            .with_config(files.path());
        mount.init_resources("fixture.rmp");
        let native_input = uqm_engine::test_support::native_session::NativeInput::bind(
            &input,
            &mut mount.bindings.borrow_mut(),
            Some(mount.content_directory()),
            mount.config_directory().as_ref(),
        );
        let font = LoadedFont::load(c"fixture.font").unwrap();
        let context = LoadedContext::create(c"setup fixture");
        let screen = LoadedDrawable::create(8, 320, 240, 1).unwrap();
        context.lease().set_frame(screen.lease().context_frame(0));
        Self {
            context: Some(context),
            screen: Some(screen),
            font: Some(font),
            input,
            _native_input: native_input,
            _mount: mount,
            _graphics: graphics,
            files,
        }
    }
    fn resources(&self) -> SetupResources {
        use super::super::strings::SetupStrings;
        let rows = table_rows();
        let table = uqm_engine::strings::owned_table::LoadedTextTable::fixture(
            rows.into_iter()
                .map(|row| CString::new(row).unwrap().into_bytes_with_nul())
                .collect(),
        );
        SetupResources {
            context: self.context.as_ref().unwrap().lease(),
            font: self.font.as_ref().unwrap().lease(),
            background: LoadedDrawable::create(8, 320, 240, 4).unwrap(),
            strings: SetupStrings::parse(&table),
        }
    }
    fn _mount_config(&self) -> uqm_engine::io::Dir {
        self._mount.config_directory().expect("fixture config")
    }
    fn configuration<'a>(
        &self,
        settings: &'a mut Settings,
        bindings: &'a mut Bindings,
    ) -> SetupConfiguration<'a> {
        SetupConfiguration {
            input: self.input.clone(),
            runtime: RuntimeSettings::capture(&self.context.as_ref().unwrap().lease()),
            settings,
            bindings,
            directory: self._mount_config(),
            names: templates::input_templates(),
        }
    }
}
fn table_rows() -> Vec<String> {
    let names = |count| {
        std::iter::repeat_n("A", count)
            .collect::<Vec<_>>()
            .join("\n")
    };
    let mut rows = vec!["A".into(), names(8), names(24)];
    for count in [
        7, 2, 4, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 2, 3, 3, 2, 6, 6, 6, 2, 2, 2,
    ] {
        rows.push(names(count));
        for _ in 0..count {
            rows.push(names(3));
        }
    }
    rows.push(names(4));
    for _ in 0..4 {
        rows.push(names(3));
    }
    rows.push(names(10));
    for _ in 0..10 {
        rows.push(names(3));
    }
    rows.push(names(4));
    for _ in 0..4 {
        rows.push(names(2));
    }
    rows.extend([names(1), names(1), names(7)]);
    rows
}

struct Driver {
    input: InputSession,
    keys: RefCell<VecDeque<usize>>,
    sleeps: Cell<usize>,
}
impl uqm_engine::input::InputRuntime for Driver {
    fn input(&self) -> &InputSession {
        &self.input
    }
    fn observer(&self) -> Box<dyn uqm_engine::observe::Observer> {
        Box::new(uqm_engine::observe::Unobserved)
    }
    fn sleep(&self, _: &mut Cx<'_>, _: u32) {
        panic!("unexpected sleep")
    }
    fn sleep_until(&self, _: &mut Cx<'_>, _: u32) {
        self.sleeps.set(self.sleeps.get() + 1)
    }
    fn yield_poll(&self, _: &mut Cx<'_>) {
        uqm_engine::test_support::native_session::key_down(b'w');
    }
    fn flush(&self) {
        self.input.clear_menu_state()
    }
    fn confirm_exit(&self, _: &mut Cx<'_>) -> bool {
        panic!("unexpected confirmation")
    }
    fn update(&self, _: &mut Cx<'_>) {
        panic!("unexpected update")
    }
    fn battle_input(&self, _: &uqm_engine::battle::input::InputContext) -> u8 {
        panic!("unexpected battle")
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
                .expect("setup script exhausted");
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
fn root_cancel_applies_and_persists_on_the_actual_input_loop() {
    let fixture = Fixture::new();
    let mut settings = Settings::default();
    let mut bindings = std::mem::take(&mut *fixture._mount.bindings.borrow_mut());
    let driver = Rc::new(Driver {
        input: fixture.input.clone(),
        keys: RefCell::new([10].into()),
        sleeps: Cell::new(0),
    });
    let mut game = uqm_engine::Game::new();
    game.kernel.install_input_runtime(driver.clone());
    let mut observer = uqm_engine::observe::Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    let configuration = fixture.configuration(&mut settings, &mut bindings);
    let resources = fixture.resources();
    game.with_context(|cx| setup_menu(cx, &mut io, configuration, resources));
    assert!(driver.keys.borrow().is_empty());
    assert_eq!(driver.sleeps.get(), 1);
    let saved = std::fs::read_to_string(fixture.files.path().join("uqm.cfg")).unwrap();
    assert!(saved.contains("gamma = INT32:"));
    let keys = std::fs::read_to_string(fixture.files.path().join("flight.cfg")).unwrap();
    assert!(keys.contains("1.name = STRING:one"));
    assert!(keys.contains("1.up.1 = STRING:key Up"));
    assert_eq!(game.world.activity() & 0x4000, 0);
}

#[test]
#[serial_test::serial]
fn retained_renderer_draws_all_pages_and_native_binding_changes_refresh_owned_values() {
    let mut fixture = Fixture::new();
    let mut settings = Settings::default();
    let mut bindings = std::mem::take(&mut *fixture._mount.bindings.borrow_mut());
    let mut session = SetupSession::new(
        fixture.configuration(&mut settings, &mut bindings),
        fixture.resources(),
    );
    drop(fixture.font.take());
    let mut game = uqm_engine::Game::new();
    game.kernel.install_input_runtime(Rc::new(Driver {
        input: fixture.input.clone(),
        keys: RefCell::default(),
        sleeps: Cell::new(0),
    }));
    let mut observer = uqm_engine::observe::Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    game.with_context(|cx| {
        session.populate_keys();
        assert_eq!(session.controls[0][0], c"Up");
        session.act(Action::Rebind(0), cx, &mut io);
        assert_eq!(session.controls[0][0], c"w");
        assert_eq!(
            session
                .configuration
                .bindings
                .string("keys.1.up.1")
                .as_deref(),
            Some("key w")
        );
        session.act(Action::Clear(0), cx, &mut io);
        assert_eq!(session.controls[0][0], c"");
        assert_eq!(session.configuration.bindings.string("keys.1.up.1"), None);
        for page in 0..8 {
            session.menu.enter(page, true);
            session.draw();
        }
    });
    session.menu.enter(0, true);
    session.draw();
    session.resources.context.flush();
    let frame = fixture.screen.as_ref().unwrap().lease().context_frame(0);
    let title = frame.pixel(Point { x: 160, y: 9 });
    assert_eq!((title.r, title.g, title.b), (255, 255, 255));
    let active = frame.pixel(Point { x: 160, y: 67 });
    assert_eq!((active.r, active.g, active.b), (255, 255, 0));
    let _retained = session.configuration.directory.clone();
    drop(_retained);
}

#[test]
#[serial_test::serial]
fn template_editor_accepts_utf8_preserves_cancel_and_bounds_the_native_name() {
    use uqm_engine::input::frames::{KEY_MENU_EDIT_CANCEL, KEY_MENU_SELECT};
    let fixture = Fixture::new();
    let mut settings = Settings::default();
    let mut bindings = std::mem::take(&mut *fixture._mount.bindings.borrow_mut());
    let mut session = SetupSession::new(
        fixture.configuration(&mut settings, &mut bindings),
        fixture.resources(),
    );
    session.populate_keys();
    session.menu.enter(7, true);
    let driver = Rc::new(Driver {
        input: fixture.input.clone(),
        keys: RefCell::new([usize::MAX, KEY_MENU_SELECT as usize].into()),
        sleeps: Cell::new(0),
    });
    let mut game = uqm_engine::Game::new();
    game.kernel.install_input_runtime(driver.clone());
    let mut observer = uqm_engine::observe::Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    game.with_context(|cx| {
        session.edit(cx, &mut io);
        assert_eq!(session.entry.text, c"oneé");
        assert_eq!(session.names[0], c"oneé");
        *driver.keys.borrow_mut() = [usize::MAX, KEY_MENU_EDIT_CANCEL as usize].into();
        session.edit(cx, &mut io);
        assert_eq!(session.entry.text, c"oneé");
        assert_eq!(templates::input_templates()[0], c"oneé");
        session.entry.text = CString::new("a".repeat(28)).unwrap();
        *driver.keys.borrow_mut() = [usize::MAX, KEY_MENU_SELECT as usize].into();
        session.edit(cx, &mut io);
        assert_eq!(session.entry.text.to_bytes().len(), 30);
        assert_eq!(session.names[0].to_bytes(), "a".repeat(28).as_bytes());
        assert_eq!(session.names[0].to_str().unwrap(), "a".repeat(28));
    });
    assert_eq!(driver.sleeps.get(), 3);
    assert!(!session.entry.editing);
}

#[test]
#[serial_test::serial]
fn scripted_audio_changes_apply_on_return_and_abort_clears_the_activity_bit() {
    for abort in [false, true] {
        let fixture = Fixture::new();
        let mut settings = Settings::default();
        let mut bindings = std::mem::take(&mut *fixture._mount.bindings.borrow_mut());
        RuntimeSettings::apply(
            &fixture.context.as_ref().unwrap().lease(),
            None,
            Some([0.25, 0.5, 0.75]),
        );
        let keys = if abort {
            vec![0]
        } else {
            vec![6, 6, 9, 8, 10, 10]
        };
        let driver = Rc::new(Driver {
            input: fixture.input.clone(),
            keys: RefCell::new(keys.into()),
            sleeps: Cell::new(0),
        });
        let mut game = uqm_engine::Game::new();
        game.kernel.install_input_runtime(driver.clone());
        if abort {
            game.world.set_activity(0x4000);
        }
        let mut observer = uqm_engine::observe::Unobserved;
        let mut io = Io {
            observer: &mut observer,
        };
        let configuration = fixture.configuration(&mut settings, &mut bindings);
        let resources = fixture.resources();
        game.with_context(|cx| setup_menu(cx, &mut io, configuration, resources));
        assert_eq!(
            settings.integer("config.musicvol"),
            Some(if abort { 25 } else { 30 })
        );
        assert_eq!(game.world.activity() & 0x4000, 0);
        assert!(driver.keys.borrow().is_empty());
    }
}
