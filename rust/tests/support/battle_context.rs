use std::sync::Mutex;

static CONTEXT: Mutex<()> = Mutex::new(());

pub fn run<R>(body: impl FnOnce() -> R) -> R {
    let _guard = CONTEXT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    // SAFETY: the test mutex serializes all use of this binary's game authorities.
    unsafe { uqm_rust::app::bridge::with_battle(body) }
}
