//! Sound option globals shared with the C option parsing in `uqm.c` and the
//! Rust bridge setters.

use std::ffi::{c_float, c_int};

/// Music volume last chosen in the setup menu (`NORMAL_VOLUME` initially).
#[no_mangle]
pub static mut musicVolume: c_int = 160;
#[no_mangle]
pub static mut musicVolumeScale: c_float = 0.0;
#[no_mangle]
pub static mut sfxVolumeScale: c_float = 0.0;
#[no_mangle]
pub static mut speechVolumeScale: c_float = 0.0;

/// Sound driver and quality selected on the command line.
#[no_mangle]
pub static mut snddriver: c_int = 0;
#[no_mangle]
pub static mut soundflags: c_int = 0;
