//! Direct Rust startup and teardown providers.

#[cfg(not(test))]
pub use crate::callback::ffi::{
    Alarm_uninit as uqm_alarm_uninit, Callback_uninit as uqm_callback_uninit,
};
#[cfg(not(test))]
pub use crate::game_init::kernel_abi::uninitIO;
#[cfg(not(test))]
pub use crate::graphics::cmap_abi::UninitColorMaps;
#[cfg(not(test))]
pub use crate::graphics::dcqueue_abi::TFB_PurgeDanglingGraphics;
#[cfg(not(test))]
pub use crate::graphics::sdl::lifecycle_abi::TFB_UninitGraphics;
#[cfg(not(test))]
pub use crate::input::sdl_input::TFB_UninitInput;
#[cfg(not(test))]
pub use crate::mainloop::options_abi::unprepareAllDirs as uqm_unprepare_all_dirs;
#[cfg(not(test))]
pub use crate::mainloop::startup::{free_addons as uqm_free_options_addons, initialize};
#[cfg(not(test))]
pub use crate::sound::audio_init::unInitAudio;
#[cfg(not(test))]
pub use crate::threading::legacy_abi::UnInitThreadSystem;
#[cfg(not(test))]
pub use crate::threading::task_abi::CleanupTaskSystem;
#[cfg(not(test))]
pub use crate::time::counter_abi::UnInitTimeSystem;

#[cfg(test)]
pub use isolated::*;

#[cfg(test)]
mod isolated {
    extern "C" {
        pub fn initialize(argc: i32, argv: *mut *mut std::ffi::c_char) -> i32;
        pub fn TFB_UninitInput();
        pub fn unInitAudio();
        pub fn TFB_PurgeDanglingGraphics();
        pub fn UninitColorMaps();
        pub fn TFB_UninitGraphics();
        pub fn uqm_callback_uninit();
        pub fn uqm_alarm_uninit();
        pub fn CleanupTaskSystem();
        pub fn UnInitTimeSystem();
        pub fn uqm_unprepare_all_dirs();
        pub fn uninitIO();
        pub fn UnInitThreadSystem();
        pub fn uqm_free_options_addons();
    }
}
