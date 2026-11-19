//! The serialized battle fixture: one owned game and battle record per test body.
//!
//! `with` builds a fresh `Cx` over the fixture's owners for each operation, so no
//! context is published and no borrow outlives the closure that received it.
#![allow(dead_code)]

use std::cell::RefCell;
use std::sync::Mutex;
use uqm_rust::engine::{battle::Battle, battle::Cx, Game};

static SERIAL: Mutex<()> = Mutex::new(());

thread_local! {
    static FIXTURE: RefCell<Option<Box<(Game, Battle)>>> = const { RefCell::new(None) };
}

struct Installed;
impl Drop for Installed {
    fn drop(&mut self) {
        FIXTURE.with(|fixture| fixture.borrow_mut().take());
    }
}

fn game() -> Game {
    let mut game = Game::new();
    uqm_rust::battle::services::install(&mut game.kernel);
    game.world.set_catalog(std::borrow::Cow::Borrowed(
        &uqm_rust::planets::tables::STARMAP_ARRAY,
    ));
    let configuration = game.kernel.configuration.clone();
    game.kernel.install_input_runtime(std::rc::Rc::new(
        uqm_rust::mainloop::input_abi::NativeInputRuntime::new(
            None,
            Default::default(),
            configuration,
        ),
    ));
    game
}

/// Run a test body against a fresh fixture game, serialized with every other body.
pub fn run<R>(body: impl FnOnce(&uqm_rust::engine::input::InputSession) -> R) -> R {
    let _guard = SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let game = game();
    let runtime = game.kernel.input_runtime();
    FIXTURE.with(|fixture| {
        let previous = fixture.replace(Some(Box::new((game, Battle::new()))));
        assert!(previous.is_none(), "battle fixtures do not nest");
    });
    let _installed = Installed;
    body(runtime.input())
}

/// Lend a context over the running fixture's owners to one operation.
#[track_caller]
pub fn with<R>(operation: impl FnOnce(&mut Cx<'_>) -> R) -> R {
    FIXTURE.with(|fixture| {
        let mut fixture = fixture
            .try_borrow_mut()
            .unwrap_or_else(|_| panic!("fixture context is already lent"));
        let (game, battle) = &mut **fixture
            .as_mut()
            .unwrap_or_else(|| panic!("fixture operation outside battle_context::run"));
        operation(&mut Cx::new(
            battle,
            &mut game.world,
            &mut game.kernel,
            uqm_rust::math::random,
        ))
    })
}

pub unsafe fn dispose(handle: *mut std::ffi::c_void) {
    with(|cx| {
        if let Some(id) = cx.elements().id(std::ptr::NonNull::new(handle)) {
            cx.dispose(id);
        }
    });
}

pub unsafe fn unlink(handle: *mut std::ffi::c_void) {
    with(|cx| {
        let id = cx
            .elements()
            .id(std::ptr::NonNull::new(handle))
            .unwrap_or_else(|| panic!("fixture element"));
        uqm_rust::battle::sounds::engine::unlink(cx, id);
    });
}
