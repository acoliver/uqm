//! Subsystem teardown, in reverse dependency order.

/// Tear down every subsystem startup brought up.
///
/// Graphics goes before the callback queue and the thread system because its
/// draw queue and colour maps may still hold pending work, and the directory
/// mounts go before the thread system because nothing reads them after the game
/// loop. Called by `app::run` after the game loop returns and every worker has
/// stopped.
pub fn teardown_subsystems() {
    crate::input::sdl_input::uninit_input();
    crate::sound::audio_init::uninit_audio();
    crate::graphics::present::uninit_graphics();
    crate::engine::host::dispatch().clear();
    crate::io::uninstall();
    crate::threading::uninit_game_thread();
    crate::memory::rust_mem_uninit();
}
