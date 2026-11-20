//! SDL start-up and the process exit hook.

use sdl2::sys;

/// The SDL library version in use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

/// Start SDL's video subsystem and report the library version. SDL is torn
/// down by the OS at process exit.
pub fn init_video() -> Result<Version, String> {
    // SAFETY: SDL_Init takes flags only; start-up runs once on the main thread.
    if unsafe { sys::SDL_Init(sys::SDL_INIT_VIDEO) } != 0 {
        return Err("Could not initialize SDL".into());
    }
    let mut version = sys::SDL_version {
        major: 0,
        minor: 0,
        patch: 0,
    };
    // SAFETY: SDL_GetVersion writes the whole struct it is given.
    unsafe { sys::SDL_GetVersion(&mut version) };
    Ok(Version {
        major: version.major,
        minor: version.minor,
        patch: version.patch,
    })
}

/// The handlers `at_exit` registered, run in reverse order at exit.
static EXIT_HANDLERS: std::sync::Mutex<Vec<fn()>> = std::sync::Mutex::new(Vec::new());

extern "C" fn run_exit_handlers() {
    let handlers = std::mem::take(
        &mut *EXIT_HANDLERS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
    );
    for handler in handlers.into_iter().rev() {
        handler();
    }
}

/// Run `handler` when the process exits normally, including through
/// `std::process::exit`.
pub fn at_exit(handler: fn()) {
    static HOOK: std::sync::Once = std::sync::Once::new();
    HOOK.call_once(|| {
        // SAFETY: the hook is a plain function that lives for the whole
        // process. atexit fails only when the process's handler table is
        // full, which this game's single registration cannot cause.
        let status = unsafe { libc::atexit(run_exit_handlers) };
        assert_eq!(status, 0, "could not register an exit handler");
    });
    EXIT_HANDLERS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .push(handler);
}
