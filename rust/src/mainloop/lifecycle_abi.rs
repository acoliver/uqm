//! Main-thread lifecycle callbacks, ported from starcon.c (GPL-2.0-or-later).
#![allow(non_snake_case)]

use crate::navigation::globals::GlobData;
use crate::ships::native::abi::*;
use std::{ffi::c_void, ptr};

#[no_mangle]
pub static mut MainExited: i32 = 0;

unsafe extern "C" fn on_battle_frame() {
    crate::time::clock_abi::live::GameClockTick();
    let opening = crate::navigation::get("ARILOU_SPACE") == 1;
    let counter = crate::navigation::get("ARILOU_SPACE_COUNTER");
    crate::navigation::set(
        "ARILOU_SPACE_COUNTER",
        if opening {
            counter + u8::from(counter < 9)
        } else {
            counter - u8::from(counter > 0)
        },
    );
    if GlobData.Game_state.CurrentActivity & (CHECK_ABORT | CHECK_LOAD) as u16 == 0 {
        crate::navigation::hyper_radar::SeedUniverse();
    }
    crate::ui::sis::DrawAutoPilotMessage(0);
}

unsafe extern "C" fn background_init_kernel(timeout: u32) {
    crate::ships::master_native::LoadMasterShipList(Some(crate::threading::legacy_abi::TaskSwitch));
    crate::threading::legacy_abi::TaskSwitch();
    crate::game_init::kernel_abi::InitGameKernel();
    while crate::time::counter_abi::GetTimeCounter() <= timeout
        && GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 == 0
    {
        super::input_abi::UpdateInputState();
        crate::threading::legacy_abi::TaskSwitch();
    }
}

/// # Safety
/// Called by the game loop on the SDL owner thread during kernel initialization.
pub unsafe fn uqm_splash_with_bg_init_kernel() {
    extern "C" fn initialize(timeout: u32) {
        unsafe {
            background_init_kernel(timeout);
        }
    }
    super::restart_menu::c_extern::SplashScreen(Some(initialize));
}

/// # Safety
/// The game kernel and battle state must be initialized on the SDL owner thread.
pub unsafe fn uqm_battle_with_frame_callback() {
    Battle(Some(on_battle_frame));
}

/// # Safety
/// Called on the game thread while live state and input are initialized.
pub unsafe extern "C" fn SignalStopMainThread() {
    ptr::addr_of_mut!(GamePaused).write_volatile(0);
    GlobData.Game_state.CurrentActivity |= CHECK_ABORT as u16;
    crate::threading::legacy_abi::TaskSwitch();
}

/// # Safety
/// Called by the game thread after immediate input has been sampled.
pub unsafe extern "C" fn ProcessUtilityKeys() {
    let state = ptr::addr_of!(ImmediateInputState).read_volatile();
    if state.menu[KEY_ABORT as usize] != 0 {
        crate::log_abi::log_showBox(false, false);
        std::process::exit(0);
    }
    if state.menu[KEY_FULLSCREEN as usize] != 0 {
        let flags = GfxFlags ^ TFB_GFXFLAGS_FULLSCREEN as i32;
        FlushInput();
        crate::graphics::tfb_draw_abi::TFB_DrawScreen_ReinitVideo(
            GraphicsDriver,
            flags,
            ScreenWidthActual,
            ScreenHeightActual,
        );
    }
    #[cfg(feature = "debug-process")]
    {
        static mut DEBUG_KEY_STATE: i32 = 0;
        let pressed = state.menu[KEY_DEBUG as usize];
        if pressed != 0 && DEBUG_KEY_STATE == 0 {
            crate::front_end::uqmdebug::debugKeyPressed();
        }
        DEBUG_KEY_STATE = pressed;
    }
}

pub extern "C" fn Starcon2Main(_arg: *mut c_void) -> i32 {
    super::game_loop::rust_game_loop()
}
