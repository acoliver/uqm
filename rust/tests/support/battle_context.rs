//! The serialized battle fixture: one owned game and battle record per test body.
//!
//! `with` builds a fresh `Cx` over the fixture's owners for each operation, and
//! `kernel` lends the fixture kernel to one record access. Both borrow the fixture
//! dynamically, so a record borrow held across an engine call panics instead of
//! aliasing the engine's own borrow.
#![allow(dead_code)]

use std::cell::{RefCell, RefMut};
use std::sync::Mutex;
use uqm_engine::{battle::record::Kernel, battle::Battle, battle::Cx, Game};

static SERIAL: Mutex<()> = Mutex::new(());

type Fixture = RefCell<Option<Box<(Game, Battle)>>>;

thread_local! {
    // Leaked once per test thread so record borrows can name a 'static owner;
    // each test body still installs and drops its own game.
    static FIXTURE: &'static Fixture = Box::leak(Box::new(RefCell::new(None)));
}

fn fixture() -> &'static Fixture {
    FIXTURE.with(|fixture| *fixture)
}

struct Installed;
impl Drop for Installed {
    fn drop(&mut self) {
        fixture().borrow_mut().take();
    }
}

fn game() -> Game {
    let mut game = Game::new();
    uqm_modes::battle::services::install(&mut game.kernel);
    game.world.set_catalog(std::borrow::Cow::Borrowed(
        &uqm_modes::planets::tables::STARMAP_ARRAY,
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
pub fn run<R>(body: impl FnOnce(&uqm_engine::input::InputSession) -> R) -> R {
    let _guard = SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let game = game();
    let runtime = game.kernel.input_runtime();
    let previous = fixture().replace(Some(Box::new((game, Battle::new()))));
    assert!(previous.is_none(), "battle fixtures do not nest");
    let _installed = Installed;
    body(runtime.input())
}

#[track_caller]
fn lend() -> RefMut<'static, (Game, Battle)> {
    let Ok(fixture) = fixture().try_borrow_mut() else {
        panic!("fixture context is already lent");
    };
    RefMut::map(fixture, |fixture| {
        &mut **fixture
            .as_mut()
            .unwrap_or_else(|| panic!("fixture operation outside battle_context::run"))
    })
}

/// Lend a context over the running fixture's owners to one operation.
#[track_caller]
pub fn with<R>(operation: impl FnOnce(&mut Cx<'_>) -> R) -> R {
    let mut fixture = lend();
    let (game, battle) = &mut *fixture;
    operation(&mut Cx::new(
        battle,
        &mut game.world,
        &mut game.kernel,
        uqm_engine::math::random,
    ))
}

/// Lend the fixture kernel until the returned borrow drops.
#[track_caller]
pub fn kernel() -> RefMut<'static, Kernel> {
    RefMut::map(lend(), |(game, _)| &mut game.kernel)
}

pub fn dispose(id: uqm_engine::battle::ElementId) {
    with(|cx| cx.dispose(id));
}

pub fn unlink(id: uqm_engine::battle::ElementId) {
    with(|cx| uqm_modes::battle::sounds::engine::unlink(cx, id));
}
