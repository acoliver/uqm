use std::sync::Mutex;
use uqm_rust::battle::ContextOps;

static CONTEXT: Mutex<()> = Mutex::new(());

pub fn run<R>(body: impl FnOnce(&uqm_rust::engine::input::InputSession) -> R) -> R {
    let _guard = CONTEXT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    // SAFETY: the test mutex serializes all use of this binary's game authorities.
    unsafe {
        let mut game = uqm_rust::engine::Game::new();
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
        let runtime = game.kernel.input_runtime();
        uqm_rust::app::bridge::with_game(&mut game, || body(runtime.input()))
    }
}

#[allow(dead_code)]
pub unsafe fn dispose(handle: *mut std::ffi::c_void) {
    // SAFETY: callers own the serialized fixture and pass its live optional allocation.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            if let Some(id) = cx.elements().id(std::ptr::NonNull::new(handle)) {
                cx.dispose(id);
            }
        });
    }
}
#[allow(dead_code)]
pub unsafe fn unlink(handle: *mut std::ffi::c_void) {
    // SAFETY: callers own the serialized fixture and pass a currently linked allocation.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            let id = cx
                .elements()
                .id(std::ptr::NonNull::new(handle))
                .unwrap_or_else(|| panic!("fixture element"));
            cx.unlink(id);
        });
    }
}
