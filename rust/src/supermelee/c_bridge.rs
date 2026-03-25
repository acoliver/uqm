// SuperMelee C Bridge — imported C signatures used by setup/handoff boundary
// @plan PLAN-20260314-SUPERMELEE.P11
//
// These are C functions we need to call from Rust for integration.
// They remain as extern declarations until they're needed at runtime.

use std::os::raw::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// Activity constants (from globdata.h)
// ---------------------------------------------------------------------------

pub const SUPER_MELEE: u8 = 1;

// ---------------------------------------------------------------------------
// Sound / Music (from libs/sound/sound.h, sounds.h)
// ---------------------------------------------------------------------------

extern "C" {
    pub fn StopMusic();
    pub fn StopSound();
}

// ---------------------------------------------------------------------------
// Global state (from globdata.h)
// ---------------------------------------------------------------------------

extern "C" {
    /// PlayerControl array — `BYTE PlayerControl[NUM_PLAYERS]`
    pub static mut PlayerControl: [u8; 2];
}

// ---------------------------------------------------------------------------
// UIO directory for melee files
// ---------------------------------------------------------------------------

extern "C" {
    /// The melee save directory handle
    pub static mut meleeDir: *mut c_void;
    /// The config directory handle
    pub static mut configDir: *mut c_void;
}
