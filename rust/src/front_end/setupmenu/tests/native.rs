#![cfg(test)]
use super::*;
use crate::graphics::{
    context_abi::LoadedContext,
    drawable_abi::LoadedDrawable,
    native_abi::POINT,
    native_font::LoadedFont,
    sdl::lifecycle_abi::{GraphicsOptions, TFB_InitGraphics, TFB_UninitGraphics},
};
use crate::io::uio_bridge as uio;
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    ptr,
    rc::Rc,
};

struct Fixture {
    context: Option<LoadedContext>,
    screen: Option<LoadedDrawable>,
    font: Option<LoadedFont>,
    input: InputSession,
    directory: *mut uio::uio_DirHandle,
    repository: *mut uio::uio_Repository,
    previous_content: *mut uqm_engine::io::ffi::uio_DirHandle,
    previous_config: *mut uqm_engine::io::ffi::uio_DirHandle,
    files: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tmp")
            .canonicalize()
            .unwrap();
        let files = tempfile::tempdir_in(root).unwrap();
        std::fs::create_dir(files.path().join("font")).unwrap();
        for code in 32..=127 {
            image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 255, 255, 255]))
                .save(files.path().join(format!("font/{code:04x}.png")))
                .unwrap();
        }
        std::fs::write(
            files.path().join("fixture.rmp"),
            "fixture.font = FONTRES:font\nkeys.1.name = STRING:one\nkeys.1.up.1 = STRING:key Up\n",
        )
        .unwrap();
        let path = CString::new(files.path().to_str().unwrap()).unwrap();
        std::env::set_var("SDL_VIDEODRIVER", "dummy");
        std::env::set_var("SDL_AUDIODRIVER", "dummy");
        let input = InputSession::default();
        // SAFETY: each serial fixture retains its mount, vectors and display until teardown.
        let (repository, directory, previous_content, previous_config, font) = unsafe {
            assert_eq!(
                TFB_InitGraphics(
                    1,
                    0,
                    ptr::from_ref(&GraphicsOptions::default()).cast(),
                    320,
                    240
                ),
                0
            );
            uio::uio_init();
            let repository = uio::uio_openRepository(0);
            assert!(!uio::uio_mountDir(
                repository,
                c"/".as_ptr(),
                1,
                ptr::null_mut(),
                ptr::null(),
                path.as_ptr(),
                ptr::null_mut(),
                6,
                ptr::null_mut()
            )
            .is_null());
            let directory = uio::uio_openDir(repository, c"/".as_ptr(), 0);
            let old = (
                crate::mainloop::dir_prep::contentDir,
                crate::mainloop::dir_prep::configDir,
            );
            crate::mainloop::dir_prep::contentDir = directory.cast();
            crate::mainloop::dir_prep::configDir = directory.cast();
            crate::resource::ffi_bridge::InitResourceSystem();
            crate::resource::ffi_bridge::LoadResourceIndex(
                directory.cast(),
                c"fixture.rmp".as_ptr(),
                ptr::null(),
            );
            let state = input.immediate_ptr();
            use crate::engine::input::frames::{FLIGHT_KEYS, MENU_KEYS, TEMPLATES};
            sdl_input::tfb_set_input_vectors(
                ptr::addr_of_mut!((*state).menu).cast(),
                MENU_KEYS as i32,
                ptr::addr_of_mut!((*state).key).cast(),
                TEMPLATES as i32,
                FLIGHT_KEYS as i32,
            );
            assert_eq!(sdl_input::TFB_InitInput(0, 0), 0);
            (
                repository,
                directory,
                old.0,
                old.1,
                LoadedFont::load(c"fixture.font").unwrap(),
            )
        };
        let context = LoadedContext::create(c"setup fixture");
        let screen = LoadedDrawable::create(8, 320, 240, 1).unwrap();
        context.lease().set_frame(screen.lease().context_frame(0));
        Self {
            context: Some(context),
            screen: Some(screen),
            font: Some(font),
            input,
            directory,
            repository,
            previous_content,
            previous_config,
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
    fn configuration(&self) -> SetupConfiguration {
        SetupConfiguration {
            input: self.input.clone(),
            runtime: RuntimeSettings::capture(&self.context.as_ref().unwrap().lease()),
            settings: Settings,
            names: templates::input_templates(),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        sdl_input::TFB_UninitInput();
        // SAFETY: callbacks have returned and no vectors or directory leases escape the fixture.
        unsafe {
            sdl_input::tfb_set_input_vectors(ptr::null_mut(), 0, ptr::null_mut(), 0, 0);
            crate::mainloop::dir_prep::contentDir = self.previous_content;
            crate::mainloop::dir_prep::configDir = self.previous_config;
            uio::uio_closeDir(self.directory);
            uio::uio_closeRepository(self.repository);
            uio::uio_unInit();
        }
        drop(self.font.take());
        drop(self.context.take());
        drop(self.screen.take());
        // SAFETY: all retained display resources have been released on the fixture's thread.
        unsafe { TFB_UninitGraphics() };
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
impl crate::engine::input::InputRuntime for Driver {
    fn input(&self) -> &InputSession {
        &self.input
    }
    fn observer(
        &self,
    ) -> Box<dyn crate::engine::observe::Observer<Option<crate::engine::observe::CampaignView>>>
    {
        Box::new(crate::engine::observe::Unobserved)
    }
    fn sleep(&self, _: &mut Cx<'_>, _: u32) {
        panic!("unexpected sleep")
    }
    fn sleep_until(&self, _: &mut Cx<'_>, _: u32) {
        self.sleeps.set(self.sleeps.get() + 1)
    }
    fn yield_poll(&self, _: &mut Cx<'_>) {
        // SAFETY: a valid SDL key-down reaches the fixture's initialized native provider.
        unsafe {
            let mut event: sdl2_sys::SDL_Event = std::mem::zeroed();
            event.type_ = 0x300;
            event.key.state = 1;
            event.key.keysym.sym = i32::from(b'w');
            event.key.keysym.scancode = sdl2_sys::SDL_Scancode::SDL_SCANCODE_W;
            sdl_input::process_input_event(&event);
        }
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
    fn battle_input(&self, _: &crate::engine::battle::input::InputContext) -> u8 {
        panic!("unexpected battle")
    }
    fn run(
        &self,
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        reset: bool,
        frame: &mut dyn crate::engine::input::Frame,
    ) {
        assert!(reset);
        loop {
            let key = self
                .keys
                .borrow_mut()
                .pop_front()
                .expect("setup script exhausted");
            self.input.clear_menu_state();
            let mut pulse = crate::engine::input::frames::InputState::default();
            if key == usize::MAX {
                // SAFETY: the initialized provider receives a terminated SDL UTF-8 event.
                unsafe {
                    let mut event: sdl2_sys::SDL_Event = std::mem::zeroed();
                    event.type_ = 0x303;
                    event.text.text[0] = 0xc3u8 as std::ffi::c_char;
                    event.text.text[1] = 0xa9u8 as std::ffi::c_char;
                    sdl_input::process_input_event(&event);
                }
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
    let driver = Rc::new(Driver {
        input: fixture.input.clone(),
        keys: RefCell::new([10].into()),
        sleeps: Cell::new(0),
    });
    let mut game = crate::engine::Game::new();
    game.kernel.install_input_runtime(driver.clone());
    let mut observer = crate::engine::observe::Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    let configuration = fixture.configuration();
    let resources = fixture.resources();
    // SAFETY: this fixture exclusively publishes its game during the synchronous menu loop.
    unsafe {
        crate::app::bridge::with_game(&mut game, || {
            crate::app::bridge::enter(|cx| setup_menu(cx, &mut io, configuration, resources))
        })
    };
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
    let mut session = SetupSession::new(fixture.configuration(), fixture.resources());
    drop(fixture.font.take());
    let mut game = crate::engine::Game::new();
    game.kernel.install_input_runtime(Rc::new(Driver {
        input: fixture.input.clone(),
        keys: RefCell::default(),
        sleeps: Cell::new(0),
    }));
    let mut observer = crate::engine::observe::Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    // SAFETY: all retained graphics/input resources belong to this fixture's synchronous game.
    unsafe {
        crate::app::bridge::with_game(&mut game, || {
            crate::app::bridge::enter(|cx| {
                session.populate_keys();
                assert_eq!(session.controls[0][0], c"Up");
                session.act(Action::Rebind(0), cx, &mut io);
                assert_eq!(session.controls[0][0], c"w");
                assert_eq!(Settings.string("keys.1.up.1").as_deref(), Some("key w"));
                session.act(Action::Clear(0), cx, &mut io);
                assert_eq!(session.controls[0][0], c"");
                assert_eq!(Settings.string("keys.1.up.1"), None);
                for page in 0..8 {
                    session.menu.enter(page, true);
                    session.draw();
                }
            })
        })
    };
    session.menu.enter(0, true);
    session.draw();
    session.resources.context.flush();
    let frame = fixture.screen.as_ref().unwrap().lease().context_frame(0);
    let title = frame.pixel(POINT { x: 160, y: 9 });
    assert_eq!((title.r, title.g, title.b), (255, 255, 255));
    let active = frame.pixel(POINT { x: 160, y: 67 });
    assert_eq!((active.r, active.g, active.b), (255, 255, 0));
    let _retained = fixture.configuration().runtime.directory.clone();
    drop(_retained);
}

#[test]
#[serial_test::serial]
fn template_editor_accepts_utf8_preserves_cancel_and_bounds_the_native_name() {
    use crate::engine::input::frames::{KEY_MENU_EDIT_CANCEL, KEY_MENU_SELECT};
    let fixture = Fixture::new();
    let mut session = SetupSession::new(fixture.configuration(), fixture.resources());
    session.populate_keys();
    session.menu.enter(7, true);
    let driver = Rc::new(Driver {
        input: fixture.input.clone(),
        keys: RefCell::new([usize::MAX, KEY_MENU_SELECT as usize].into()),
        sleeps: Cell::new(0),
    });
    let mut game = crate::engine::Game::new();
    game.kernel.install_input_runtime(driver.clone());
    let mut observer = crate::engine::observe::Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    // SAFETY: this fixture owns the game publication, native vectors and editor resources.
    unsafe {
        crate::app::bridge::with_game(&mut game, || {
            crate::app::bridge::enter(|cx| {
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
            })
        })
    };
    assert_eq!(driver.sleeps.get(), 3);
    assert!(!session.entry.editing);
}

#[test]
#[serial_test::serial]
fn scripted_audio_changes_apply_on_return_and_abort_clears_the_activity_bit() {
    for abort in [false, true] {
        let fixture = Fixture::new();
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
        let mut game = crate::engine::Game::new();
        game.kernel.install_input_runtime(driver.clone());
        if abort {
            game.world.set_activity(0x4000);
        }
        let mut observer = crate::engine::observe::Unobserved;
        let mut io = Io {
            observer: &mut observer,
        };
        let configuration = fixture.configuration();
        let resources = fixture.resources();
        // SAFETY: each serial fixture exclusively owns this game's native input loop.
        unsafe {
            crate::app::bridge::with_game(&mut game, || {
                crate::app::bridge::enter(|cx| setup_menu(cx, &mut io, configuration, resources))
            })
        };
        assert_eq!(
            Settings.integer("config.musicvol"),
            Some(if abort { 25 } else { 30 })
        );
        assert_eq!(game.world.activity() & 0x4000, 0);
        assert!(driver.keys.borrow().is_empty());
    }
}
