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
    let kernel = crate::engine::Kernel::current();
    UninitPlayerInput();
    crate::resource::ffi_bridge::UninitResourceSystem();
    DestroyDrawable(ReleaseDrawable((*kernel).screen));
    (*kernel).screen = ptr::null_mut();
    DestroyContext((*kernel).screen_context.cast());
    (*kernel).screen_context = ptr::null_mut();
    crate::video::control::UninitVideoPlayer();
    crate::sound::heart_ffi::uninit_sound();
}

unsafe fn uninit_contexts() {
    let kernel = crate::engine::Kernel::current();
    crate::engine::battle::access::lifecycle::uninit_elements(
        &mut *crate::engine::Kernel::current(),
    );
    DestroyContext((*kernel).off_screen_context.cast());
    DestroyContext((*kernel).space_context.cast());
    DestroyContext((*kernel).status_context.cast());
}

unsafe fn uninit_kernel() {
    let kernel = crate::engine::Kernel::current();
    UninitSpace();
    DestroySound(ReleaseStringTable((*kernel).menu_sounds.cast()).cast());
    DestroyFont((*kernel).micro_font.cast());
    DestroyStringTable(ReleaseStringTable((*kernel).game_strings));
    DestroyDrawable(ReleaseDrawable((*kernel).status_frame));
    DestroyDrawable(ReleaseDrawable((*kernel).activity_frame));
    DestroyFont((*kernel).tiny_font.cast());
    DestroyFont((*kernel).star_con_font.cast());
    crate::battle::context_access::lifecycle::uninit_ships(&mut *crate::engine::Kernel::current());
    (*kernel).activity_frame = ptr::null_mut();
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
    let kernel = crate::engine::Kernel::current();
    if !(*kernel).activity_frame.is_null() {
        FreeGameData();
        uninit_kernel();
        uninit_contexts();
    }
}
