//! Runtime bridge helpers for the battle model.
//!
//! Rust-owned operations are imported from their implementation modules. Opaque
//! resource handles are converted at the model boundary, not redeclared as
//! foreign functions.
//!
//! @plan PLAN-20260320-BATTLEPT2.P06
//! @requirement REQ-FFI-SAFETY, REQ-CALLBACK-SAFETY, REQ-BUILD-COEXISTENCE

// ---------------------------------------------------------------------------
// Extern "C" declarations — C functions Rust will call
// ---------------------------------------------------------------------------

pub use crate::graphics::context_abi::SetContext;

pub use crate::graphics::gfx_common_abi::ScreenTransition;

pub use crate::graphics::drawable_abi::GetFrameHot;

pub use crate::graphics::drawable_abi::GetFrameRect;

pub use crate::graphics::tfb_draw_abi::TFB_DrawScreen_SetMipmap;

#[cfg(not(test))]
pub use crate::ui::settings::PlayMusic;

pub use crate::sound::heart_ffi::PLRPlaying;

#[cfg(not(test))]
pub use crate::mainloop::input_abi::live::CurrentInputToBattleInput;

#[cfg(not(test))]
pub use crate::mainloop::input_abi::live::DoInput;

pub use crate::graphics::drawable_abi::DestroyDrawable;

#[cfg(not(test))]
pub use crate::navigation::globals::inHQSpace;

pub use crate::sound::heart_ffi::DestroyMusic;
pub use crate::threading::pacing::linked::sleep_thread as SleepThread;
