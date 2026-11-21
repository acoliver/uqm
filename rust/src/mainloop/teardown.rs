//! Subsystem teardown, in reverse dependency order.

/// Tear down every subsystem startup brought up.
///
/// Graphics goes before the callback queue and the thread system because its
/// draw queue and colour maps may still hold pending work, and the directory
/// mounts go before the thread system because nothing reads them after the game
/// loop. Called by `app::run` after the game loop returns and every worker has
/// stopped.
pub fn teardown_subsystems() {
    uqm_engine::input_port::sdl_input::uninit_input();
    uqm_engine::sound::audio_init::uninit_audio();
    uqm_engine::graphics::present::uninit_graphics();
    uqm_engine::host::dispatch().clear();
    uqm_engine::io::uninstall();
    uqm_engine::threading::uninit_game_thread();
}
