//! Direct Rust startup and teardown providers.

#[cfg(any(not(test), feature = "linked_c_archive"))]
pub use crate::callback::ffi::{
    Alarm_uninit as uqm_alarm_uninit, Callback_uninit as uqm_callback_uninit,
};
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub use crate::game_init::kernel_abi::uninitIO;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub use crate::graphics::cmap_abi::UninitColorMaps;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub use crate::graphics::dcqueue_abi::TFB_PurgeDanglingGraphics;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub use crate::graphics::sdl::lifecycle_abi::TFB_UninitGraphics;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub use crate::input::sdl_input::TFB_UninitInput;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub use crate::mainloop::options_abi::unprepareAllDirs as uqm_unprepare_all_dirs;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub use crate::mainloop::startup::{free_addons as uqm_free_options_addons, initialize};
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub use crate::sound::audio_init::unInitAudio;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub use crate::threading::legacy_abi::UnInitThreadSystem;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub use crate::threading::task_abi::CleanupTaskSystem;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub use crate::time::counter_abi::UnInitTimeSystem;

#[cfg(all(test, not(feature = "linked_c_archive")))]
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
#[cfg(all(test, not(feature = "linked_c_archive")))]
pub use isolated::*;
