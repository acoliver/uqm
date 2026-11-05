//! Main-thread lifecycle callbacks, ported from starcon.c (GPL-2.0-or-later).

use crate::engine::Kernel;
use crate::engine::StoryKey;
use crate::engine::World;
use crate::engine::WorldData;
use crate::ships::native::abi::*;
use std::ptr;

#[no_mangle]
pub static mut MainExited: i32 = 0;

unsafe extern "C" fn on_battle_frame() {
    crate::time::clock_abi::live::GameClockTick();
    World::current().write(|world| {
        let opening = crate::navigation::get(world, StoryKey::ARILOU_SPACE) == 1;
        let counter = crate::navigation::get(world, StoryKey::ARILOU_SPACE_COUNTER);
        crate::navigation::set(
            world,
            StoryKey::ARILOU_SPACE_COUNTER,
            if opening {
                counter + u8::from(counter < 9)
            } else {
                counter - u8::from(counter > 0)
            },
        );
    });
    if WorldData::current().read(|owner| owner.game.current_activity)
        & (CHECK_ABORT | CHECK_LOAD) as u16
        == 0
    {
        crate::navigation::hyper_radar::SeedUniverse();
    }
    crate::ui::sis::DrawAutoPilotMessage(0);
}

unsafe extern "C" fn background_init_kernel(timeout: u32) {
    for species in SPECIES_ID_ARILOU_ID..=SPECIES_ID_LAST_MELEE_ID {
        crate::threading::legacy_abi::TaskSwitch();
        // SAFETY: the splash callback enters the published game owner between task switches.
        unsafe {
            crate::app::bridge::enter(|cx| {
                crate::ships::master_native::engine::load_entry(cx.world, cx.kernel, species)
            });
        }
    }
    crate::threading::legacy_abi::TaskSwitch();
    Kernel::current().write(|kernel| {
        crate::game_init::kernel_abi::InitGameKernel(
            kernel,
            World::current().read(|world| world.activity()),
        )
    });
    while crate::time::counter_abi::GetTimeCounter() <= timeout
        && WorldData::current().read(|owner| owner.game.current_activity) & CHECK_ABORT as u16 == 0
    {
        super::input_abi::UpdateInputState();
        if crate::time::source::is_replay() {
            crate::threading::pacing::linked::sleep_thread_until(timeout.saturating_add(1));
        } else {
            crate::threading::legacy_abi::TaskSwitch();
        }
    }
}

/// # Safety
/// Called by the game loop on the SDL owner thread during kernel initialization.
pub unsafe fn uqm_splash_with_bg_init_kernel(
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
) {
    extern "C" fn initialize(timeout: u32) {
        unsafe {
            background_init_kernel(timeout);
        }
    }
    crate::front_end::fmv::runtime::splash_screen(cx, io, Some(initialize));
}

/// # Safety
/// The game kernel and battle state must be initialized on the SDL owner thread.
pub unsafe fn uqm_battle_with_frame_callback() {
    Battle(Some(on_battle_frame));
}

/// # Safety
/// Called on the game thread while live state and input are initialized.
pub unsafe extern "C" fn signal_stop_main_thread() {
    ptr::addr_of_mut!(GamePaused).write_volatile(0);
    (**WorldData::current()).game.current_activity |= CHECK_ABORT as u16;
    crate::threading::legacy_abi::TaskSwitch();
}

/// # Safety
/// Called by the game thread after immediate input has been sampled.
pub unsafe extern "C" fn process_utility_keys() {
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
            crate::front_end::uqmdebug::debug_key_pressed();
        }
        DEBUG_KEY_STATE = pressed;
    }
}
