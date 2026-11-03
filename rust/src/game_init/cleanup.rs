//! Kernel resource teardown, ported from cleanup.c (GPL-2.0-or-later).
#![allow(non_snake_case)]

use crate::graphics::{context_abi::DestroyContext, font_abi::DestroyFont};
use crate::ships::native::abi::*;
use std::ptr;

#[cfg(not(test))]
use crate::game_init::kernel_abi::UninitPlayerInput;
#[cfg(test)]
extern "C" {
    fn UninitPlayerInput();
}

use crate::game_init::space_resources::live::UninitSpace;

use crate::planets::solarsys::resources::FreeIPData;

/// # Safety
/// Run on the game thread after the game loop stops, before graphics teardown.
pub unsafe fn FreeKernel() {
    use crate::engine::Kernel;
    UninitPlayerInput();
    crate::resource::ffi_bridge::UninitResourceSystem();
    DestroyDrawable(ReleaseDrawable((*Kernel::current()).screen));
    (*Kernel::current()).screen = ptr::null_mut();
    DestroyContext((*Kernel::current()).screen_context.cast());
    (*Kernel::current()).screen_context = ptr::null_mut();
    crate::video::control::UninitVideoPlayer();
    crate::sound::heart_ffi::uninit_sound();
}

unsafe fn uninit_contexts() {
    use crate::engine::Kernel;
    crate::engine::battle::access::lifecycle::uninit_elements(&mut *Kernel::current());
    DestroyContext((*Kernel::current()).off_screen_context.cast());
    DestroyContext((*Kernel::current()).space_context.cast());
    DestroyContext((*Kernel::current()).status_context.cast());
}

unsafe fn uninit_kernel() {
    use crate::engine::Kernel;
    UninitSpace();
    DestroySound(ReleaseStringTable((*Kernel::current()).menu_sounds.cast()).cast());
    DestroyFont((*Kernel::current()).micro_font.cast());
    DestroyStringTable(ReleaseStringTable((*Kernel::current()).game_strings));
    DestroyDrawable(ReleaseDrawable((*Kernel::current()).status_frame));
    DestroyDrawable(ReleaseDrawable((*Kernel::current()).activity_frame));
    DestroyFont((*Kernel::current()).tiny_font.cast());
    DestroyFont((*Kernel::current()).star_con_font.cast());
    crate::battle::context_access::lifecycle::uninit_ships(&mut *Kernel::current());
    (*Kernel::current()).activity_frame = ptr::null_mut();
}

/// # Safety
/// Game thread only; all game resource owners must still be initialized.
pub unsafe fn FreeGameData() {
    crate::navigation::globals::FreeSC2Data();
    crate::planet_side::orbit_music::free_lander_data();
    FreeIPData();
    crate::navigation::hyper_data::FreeHyperData();
}

/// # Safety
/// Game thread only, after the active game has stopped using its resources.
pub unsafe fn UninitGameKernel() {
    use crate::engine::Kernel;
    if !(*Kernel::current()).activity_frame.is_null() {
        FreeGameData();
        uninit_kernel();
        uninit_contexts();
    }
}
