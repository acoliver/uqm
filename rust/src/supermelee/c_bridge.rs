// SuperMelee C Bridge — imported C signatures used by setup/handoff boundary
// @plan PLAN-20260314-SUPERMELEE.P11
//
// These are C functions we need to call from Rust for integration.
// They remain as extern declarations until they're needed at runtime.

// ---------------------------------------------------------------------------
// Activity constants (from globdata.h)
// ---------------------------------------------------------------------------

pub const SUPER_MELEE: u8 = 1;

// ---------------------------------------------------------------------------
// Sound / Music (from libs/sound/sound.h, sounds.h)
// ---------------------------------------------------------------------------

#[cfg(not(test))]
pub use crate::ui::settings::stop_music;
#[cfg(test)]
extern "C" {
    pub fn stop_music();
}
pub use crate::sound::heart_ffi::stop_sound as StopSound;

// ---------------------------------------------------------------------------
// Global state (from globdata.h)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// UIO directory for melee files
// ---------------------------------------------------------------------------

#[cfg(not(test))]
pub use crate::mainloop::options_abi::{configDir, meleeDir};
