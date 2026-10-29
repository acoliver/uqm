//! Runtime bridge helpers for the battle model.
//!
//! Rust-owned operations are imported from their implementation modules. Opaque
//! resource handles are converted at the model boundary, not redeclared as
//! foreign functions.
//!
//! @plan PLAN-20260320-BATTLEPT2.P06
//! @requirement REQ-FFI-SAFETY, REQ-CALLBACK-SAFETY, REQ-BUILD-COEXISTENCE

pub use crate::math::seed_random as TFB_SeedRandom;
use std::os::raw::{c_int, c_void};

use super::element::Element;

pub use crate::c_bindings::global_state::DrawBatch;

// ---------------------------------------------------------------------------
// Opaque C types
// ---------------------------------------------------------------------------

/// Opaque C CONTEXT handle
pub type Context = *mut c_void;
/// Opaque C FRAME handle
pub type Frame = *mut c_void;
/// Opaque C DRAWABLE handle
pub type Drawable = *mut c_void;
/// Opaque C SOUND handle
pub type Sound = *mut c_void;
/// Opaque C MUSIC_REF handle
pub type MusicRef = *mut c_void;
/// Opaque C STARSHIP pointer
pub type StarShipPtr = *mut c_void;
/// C PRIM_LINKS type (u32 with pred/succ packed)
pub type PrimLinks = u32;

// ---------------------------------------------------------------------------
// Extern "C" declarations — C functions Rust will call
// ---------------------------------------------------------------------------

#[cfg(not(test))]
pub use crate::game_init::macro_abi::get_current_activity;
#[cfg(test)]
extern "C" {
    pub fn get_current_activity() -> u16;
}

pub use crate::threading::legacy_abi::TaskSwitch;
#[cfg(not(test))]
pub use crate::ui::settings::StopMusic;
#[cfg(test)]
extern "C" {
    pub fn StopMusic();
}
pub use crate::sound::heart_ffi::stop_sound as StopSound;
pub use crate::time::counter_abi::GetTimeCounter;

pub use crate::graphics::context_abi::SetContext;

pub use crate::graphics::frame_abi::ClearDrawable;

pub use crate::graphics::gfx_common_abi::ScreenTransition;

pub use crate::graphics::geometry_ffi::DrawablesIntersect;

#[cfg(not(test))]
pub use crate::game_init::macro_abi::InitIntersectStartPoint;

#[cfg(not(test))]
pub use crate::game_init::macro_abi::InitIntersectEndPoint;

pub use crate::graphics::pixmap_abi::SetEquFrameIndex;

pub use crate::graphics::pixmap_abi::SetAbsFrameIndex;

pub use crate::graphics::pixmap_abi::IncFrameIndex;

pub use crate::graphics::drawable_abi::GetFrameHot;

pub use crate::graphics::drawable_abi::GetFrameRect;

pub use crate::graphics::pixmap_abi::GetFrameCount;

pub use crate::graphics::tfb_draw_abi::TFB_DrawScreen_SetMipmap;

#[cfg(not(test))]
pub use crate::battle::sounds::engine::ProcessSound;

#[cfg(not(test))]
pub use crate::battle::sounds::FlushSounds;

#[cfg(not(test))]
pub use crate::battle::sounds::UpdateSoundPositions;

#[cfg(not(test))]
pub use crate::battle::sounds::engine::RemoveSoundsForObject;

#[cfg(not(test))]
pub use crate::ui::settings::PlayMusic;

pub use crate::sound::heart_ffi::PLRPlaying;

#[cfg(not(test))]
pub use crate::battle::tactrans_native::engine::StopDitty;

#[cfg(not(test))]
pub use crate::mainloop::input_abi::live::CurrentInputToBattleInput;

#[cfg(not(test))]
pub use crate::mainloop::input_abi::live::DoInput;

pub use crate::graphics::pixmap_abi::CaptureDrawable;

pub use crate::graphics::pixmap_abi::ReleaseDrawable;

pub use crate::graphics::drawable_abi::DestroyDrawable;

#[cfg(not(test))]
pub use crate::game_init::macro_abi::LoadMusic;

#[cfg(not(test))]
pub use crate::game_init::macro_abi::GetElementStarShip;

#[cfg(not(test))]
pub use crate::navigation::globals::inHQSpace;

#[cfg(not(test))]
pub use crate::battle::world_elements::engine::do_damage;

pub use crate::battle::collision::collide;

pub use crate::battle::gravity::CalculateGravity;

#[cfg(not(test))]
pub use crate::navigation::hyperspace::live::MoveSIS;

#[cfg(not(test))]
pub use crate::battle::galaxy::engine::MoveGalaxy;

/// Begin batched graphics mode
pub use crate::graphics::gfx_common_abi::BatchGraphics;
/// Set the graphics scale factor (0 = reset)
pub use crate::graphics::gfx_common_abi::SetGraphicScale;
/// End batched graphics mode
pub use crate::graphics::gfx_common_abi::UnbatchGraphics;
pub use crate::sound::heart_ffi::DestroyMusic;
pub use crate::threading::pacing::linked::sleep_thread as SleepThread;
pub use crate::threading::pacing::linked::sleep_thread_until as SleepThreadUntil;

// ---------------------------------------------------------------------------
// Safe Rust wrappers
// ---------------------------------------------------------------------------

/// Safe wrapper: elastic collision.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn elastic_collide_c(a: *mut Element, b: *mut Element) {
    if !a.is_null() && !b.is_null() {
        collide(a, b);
    }
}

#[cfg(not(test))]
/// Safe wrapper: process and flush sounds.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn flush_all_sounds() {
    FlushSounds();
}

#[cfg(not(test))]
/// Safe wrapper: update stereo sound positions.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn update_sound_positions() {
    UpdateSoundPositions();
}

/// Safe wrapper: set graphics scale.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn set_graphic_scale(scale: i32) {
    SetGraphicScale(scale as c_int);
}

/// Safe wrapper: clear current drawable.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn clear_drawable() {
    ClearDrawable();
}

#[cfg(not(test))]
/// Safe wrapper: check if in HyperSpace/QuasiSpace.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn is_hq_space() -> bool {
    inHQSpace() != 0
}

/// Safe wrapper: get current activity flags.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn current_activity() -> u16 {
    crate::mainloop::ffi::get_current_activity().0
}

pub use crate::math::random;

pub use crate::time::counter_abi::GetTimeCounter as time_counter;

/// Safe wrapper: task switch / yield.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn task_switch() {
    TaskSwitch();
}

/// Safe wrapper: sleep thread for ticks.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn sleep_thread(ticks: u32) {
    SleepThread(ticks);
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// C's `END_OF_LIST` sentinel for prim links
pub const END_OF_LIST: u16 = 0xFFFF;

/// C's `NUM_PRIMS` — max primitive types for InsertPrim check
pub const NUM_PRIMS: u8 = 6;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_end_of_list_sentinel() {
        assert_eq!(END_OF_LIST, 0xFFFF);
    }

    #[test]
    fn test_num_prims_value() {
        const { assert!(NUM_PRIMS > 0) };
        const { assert!(NUM_PRIMS <= 8) };
    }

    #[test]
    fn test_opaque_type_sizes() {
        // All opaque types are pointer-sized
        assert_eq!(
            std::mem::size_of::<Context>(),
            std::mem::size_of::<*mut c_void>()
        );
        assert_eq!(
            std::mem::size_of::<Frame>(),
            std::mem::size_of::<*mut c_void>()
        );
    }
}
