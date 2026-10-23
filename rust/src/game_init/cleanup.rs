//! Kernel resource teardown, ported from cleanup.c (GPL-2.0-or-later).
#![allow(non_snake_case)]

use crate::graphics::{context_abi::DestroyContext, font_abi::DestroyFont};
use crate::ships::native::abi::*;
use std::ptr;

extern "C" {
    fn UninitPlayerInput();
    fn UninitSpace();
    fn FreeIPData();
}

/// # Safety
/// Run on the game thread after the game loop stops, before graphics teardown.
#[no_mangle]
pub unsafe extern "C" fn FreeKernel() {
    UninitPlayerInput();
    crate::resource::ffi_bridge::UninitResourceSystem();
    DestroyDrawable(ReleaseDrawable(Screen));
    Screen = ptr::null_mut();
    DestroyContext(ScreenContext.cast());
    ScreenContext = ptr::null_mut();
    crate::video::control::UninitVideoPlayer();
    crate::sound::heart_ffi::UninitSound();
}

unsafe fn uninit_contexts() {
    UninitQueue(ptr::addr_of_mut!(disp_q));
    DestroyContext(OffScreenContext.cast());
    DestroyContext(SpaceContext.cast());
    DestroyContext(StatusContext.cast());
}

unsafe fn uninit_kernel() {
    UninitSpace();
    DestroySound(crate::strings::string_table::ReleaseStringTable(MenuSounds.cast()).cast());
    DestroyFont(MicroFont.cast());
    DestroyStringTable(ReleaseStringTable(GameStrings));
    DestroyDrawable(ReleaseDrawable(StatusFrame));
    DestroyDrawable(ReleaseDrawable(ActivityFrame));
    DestroyFont(TinyFont.cast());
    DestroyFont(StarConFont.cast());
    let races = ptr::addr_of_mut!(race_q).cast::<QUEUE>();
    UninitQueue(races);
    UninitQueue(races.add(1));
    ActivityFrame = ptr::null_mut();
}

/// # Safety
/// Game thread only; all game resource owners must still be initialized.
#[no_mangle]
pub unsafe extern "C" fn FreeGameData() {
    crate::navigation::globals::FreeSC2Data();
    crate::planet_side::orbit_music::free_lander_data();
    FreeIPData();
    crate::navigation::hyper_data::FreeHyperData();
}

/// # Safety
/// Game thread only, after the active game has stopped using its resources.
#[no_mangle]
pub unsafe extern "C" fn UninitGameKernel() {
    if !ActivityFrame.is_null() {
        FreeGameData();
        uninit_kernel();
        uninit_contexts();
    }
}
